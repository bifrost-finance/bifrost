// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use std::{cell::RefCell, sync::Arc};

use crate::{
	collator_polkadot::FullClient,
	eth::{spawn_frontier_tasks, EthConfiguration},
};
use bifrost_polkadot_runtime::{constants::time::SLOT_DURATION, TransactionConverter};
use cumulus_client_parachain_inherent::{MockValidationDataInherentDataProvider, MockXcmConfig};
use cumulus_primitives_core::{relay_chain::Hash, ParaId};
use fc_storage::StorageOverrideHandler;
use jsonrpsee::core::async_trait;
use sc_client_api::Backend;
use sc_network::NetworkBackend;
use sc_service::{Configuration, TaskManager};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_blockchain::HeaderBackend;
use sp_core::Encode;
pub type Block = bifrost_primitives::Block;
pub type RuntimeApi = bifrost_polkadot_runtime::RuntimeApi;
pub type FullBackend = sc_service::TFullBackend<Block>;
pub type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

thread_local!(static TIMESTAMP: RefCell<u64> = const { RefCell::new(0) });

/// Provide a mock duration starting at 0 in millisecond for timestamp inherent.
/// Each call will increment timestamp by slot_duration making Aura think time has passed.
struct MockTimestampInherentDataProvider;

#[async_trait]
impl sp_inherents::InherentDataProvider for MockTimestampInherentDataProvider {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut sp_inherents::InherentData,
	) -> Result<(), sp_inherents::Error> {
		TIMESTAMP.with(|x| {
			*x.borrow_mut() += SLOT_DURATION;
			inherent_data.put_data(sp_timestamp::INHERENT_IDENTIFIER, &*x.borrow())
		})
	}

	async fn try_handle_error(
		&self,
		_identifier: &sp_inherents::InherentIdentifier,
		_error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		// The pallet never reports error.
		None
	}
}

/// Builds a new development service. This service uses manual seal, and mocks
/// the parachain inherent.
/// Before calling this function, you must set OnTimestampSet in runtime to be ().
pub async fn start_node<Net>(
	parachain_config: Configuration,
	eth_config: EthConfiguration,
	para_id: ParaId,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)>
where
	Net: NetworkBackend<bifrost_primitives::Block, Hash>,
{
	let params = crate::collator_polkadot::new_partial(&parachain_config, true)?;
	let (
		_block_import,
		mut telemetry,
		_telemetry_worker_handle,
		frontier_backend,
		filter_pool,
		fee_history_cache,
	) = params.other;

	let client = params.client.clone();
	let backend = params.backend.clone();
	let mut task_manager = params.task_manager;

	let transaction_pool = params.transaction_pool.clone();
	let net_config =
		sc_network::config::FullNetworkConfiguration::<_, _, Net>::new(&parachain_config.network);
	let metrics = Net::register_notification_metrics(
		parachain_config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
	);

	let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &parachain_config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue: params.import_queue,
			block_announce_validator_builder: None,
			warp_sync_params: None,
			block_relay: None,
			metrics,
		})?;

	let prometheus_registry = parachain_config.prometheus_registry().cloned();

	if parachain_config.offchain_worker.enabled {
		use futures::FutureExt;

		let backend_ofc = backend.clone();
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-work",
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				keystore: Some(params.keystore_container.keystore()),
				offchain_db: backend_ofc.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: Arc::new(network.clone()),
				is_validator: parachain_config.role.is_authority(),
				enable_http_requests: false,
				custom_extensions: move |_| vec![],
			})
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
	}

	let select_chain = params
		.select_chain
		.expect("In `dev` mode, `new_partial` will return some `select_chain`; qed");

	let proposer_factory = sc_basic_authorship::ProposerFactory::new(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool.clone(),
		None,
		None,
	);

	// Channel for the rpc handler to communicate with the authorship task.
	let (command_sink, commands_stream) = futures::channel::mpsc::channel(1024);
	let client_set_aside_for_cidp = client.clone();

	// Create channels for mocked XCM messages.
	let (_downward_xcm_sender, downward_xcm_receiver) = flume::bounded::<Vec<u8>>(100);
	let (_hrmp_xcm_sender, hrmp_xcm_receiver) = flume::bounded::<(ParaId, Vec<u8>)>(100);

	let authorship_future =
		sc_consensus_manual_seal::run_manual_seal(sc_consensus_manual_seal::ManualSealParams {
			block_import: client.clone(),
			env: proposer_factory,
			client: client.clone(),
			pool: transaction_pool.clone(),
			commands_stream,
			select_chain,
			consensus_data_provider: None,
			create_inherent_data_providers: move |block, ()| {
				let maybe_current_para_block = client_set_aside_for_cidp.number(block);
				let maybe_current_para_head = client_set_aside_for_cidp.expect_header(block);
				let downward_xcm_receiver = downward_xcm_receiver.clone();
				let hrmp_xcm_receiver = hrmp_xcm_receiver.clone();

				let client_for_xcm = client_set_aside_for_cidp.clone();
				async move {
					let time = sp_timestamp::InherentDataProvider::from_system_time();

					let current_para_block = maybe_current_para_block?
						.ok_or(sp_blockchain::Error::UnknownBlock(block.to_string()))?;

					let current_para_block_head =
						Some(polkadot_primitives::HeadData(maybe_current_para_head?.encode()));

					let mocked_parachain = MockValidationDataInherentDataProvider {
						current_para_block,
						current_para_block_head,
						para_id,
						relay_offset: 1000,
						relay_blocks_per_para_block: 2,
						// TODO: Recheck
						para_blocks_per_relay_epoch: 10,
						relay_randomness_config: (),
						xcm_config: MockXcmConfig::new(&*client_for_xcm, block, Default::default()),
						raw_downward_messages: downward_xcm_receiver.drain().collect(),
						raw_horizontal_messages: hrmp_xcm_receiver.drain().collect(),
						additional_key_values: None,
					};

					Ok((time, mocked_parachain))
				}
			},
		});
	// we spawn the future on a background thread managed by service.
	task_manager.spawn_essential_handle().spawn_blocking(
		"manual-seal",
		Some("block-authoring"),
		authorship_future,
	);

	let storage_override = Arc::new(StorageOverrideHandler::new(client.clone()));
	let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
		task_manager.spawn_handle(),
		storage_override.clone(),
		eth_config.eth_log_block_cache,
		eth_config.eth_statuses_cache,
		prometheus_registry.clone(),
	));

	// Sinks for pubsub notifications.
	// Everytime a new subscription is created, a new mpsc channel is added to the sink pool.
	// The MappingSyncWorker sends through the channel on block import and the subscription emits a
	// notification to the subscriber on receiving a message through this channel.
	// This way we avoid race conditions when using native substrate block import notification
	// stream.
	let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
		fc_mapping_sync::EthereumBlockNotification<Block>,
	> = Default::default();
	let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

	let rpc_builder = {
		let client = client.clone();
		let is_authority = parachain_config.role.is_authority();
		let transaction_pool = transaction_pool.clone();
		let network = network.clone();
		let sync_service = sync_service.clone();
		let frontier_backend = frontier_backend.clone();
		let fee_history_cache = fee_history_cache.clone();
		let filter_pool = filter_pool.clone();
		let storage_override = storage_override.clone();
		let pubsub_notification_sinks = pubsub_notification_sinks.clone();

		Box::new(move |deny_unsafe, subscription_task_executor| {
			let deps = crate::rpc::FullDepsPolkadot {
				client: client.clone(),
				pool: transaction_pool.clone(),
				deny_unsafe,
				command_sink: Some(command_sink.clone()),
			};
			let module = crate::rpc::create_full_polkadot(deps)?;

			let eth_deps = crate::rpc::EthDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				graph: transaction_pool.pool().clone(),
				converter: Some(TransactionConverter),
				is_authority,
				enable_dev_signer: eth_config.enable_dev_signer,
				network: network.clone(),
				sync_service: sync_service.clone(),
				frontier_backend: frontier_backend.clone(),
				storage_override: storage_override.clone(),
				block_data_cache: block_data_cache.clone(),
				filter_pool: filter_pool.clone(),
				max_past_logs: eth_config.max_past_logs,
				fee_history_cache: fee_history_cache.clone(),
				fee_history_cache_limit: eth_config.fee_history_limit,
				execute_gas_limit_multiplier: eth_config.execute_gas_limit_multiplier,
			};

			crate::rpc::create_eth(
				module,
				eth_deps,
				subscription_task_executor,
				pubsub_notification_sinks.clone(),
			)
			.map_err(Into::into)
		})
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		rpc_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.keystore(),
		backend: backend.clone(),
		network: network.clone(),
		sync_service: sync_service.clone(),
		system_rpc_tx,
		tx_handler_controller,
		telemetry: telemetry.as_mut(),
	})?;
	spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend,
		frontier_backend,
		filter_pool,
		storage_override,
		fee_history_cache,
		eth_config.fee_history_limit,
		sync_service.clone(),
		pubsub_notification_sinks,
	);

	start_network.start_network();

	Ok((task_manager, client))
}

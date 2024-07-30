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

use crate::{
	collator_polkadot::FullClient,
	eth::{spawn_frontier_tasks, EthConfiguration},
};
use bifrost_polkadot_runtime::TransactionConverter;
use cumulus_client_parachain_inherent::{MockValidationDataInherentDataProvider, MockXcmConfig};
use cumulus_primitives_core::{relay_chain::Hash, ParaId};
use fc_storage::StorageOverrideHandler;
use sc_client_api::Backend;
use sc_network::NetworkBackend;
use sc_service::{Configuration, TaskManager};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_core::Encode;
use std::sync::Arc;

pub type Block = bifrost_primitives::Block;
pub type RuntimeApi = bifrost_polkadot_runtime::RuntimeApi;

pub type FullBackend = sc_service::TFullBackend<Block>;
pub type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

/// Builds a new development service. This service uses manual seal, and mocks
/// the parachain inherent.
/// Before calling this function, you must set OnTimestampSet in runtime to be ().
#[sc_tracing::logging::prefix_logs_with("Dev mode")]
pub async fn start_node<Net>(
	config: Configuration,
	_: Configuration,
	eth_config: EthConfiguration,
	_: cumulus_client_cli::CollatorOptions,
	para_id: ParaId,
	_: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)>
where
	Net: NetworkBackend<Block, Hash>,
{
	let params = crate::collator_polkadot::new_partial(&config, true)?;
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

	let prometheus_registry = config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let net_config =
		sc_network::config::FullNetworkConfiguration::<_, _, Net>::new(&config.network);

	let metrics = Net::register_notification_metrics(
		config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
	);

	let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
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

	if config.offchain_worker.enabled {
		use futures::FutureExt;

		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-work",
			sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
				runtime_api_provider: client.clone(),
				keystore: Some(params.keystore_container.keystore()),
				offchain_db: backend.offchain_storage(),
				transaction_pool: Some(OffchainTransactionPoolFactory::new(
					transaction_pool.clone(),
				)),
				network_provider: Arc::new(network.clone()),
				is_validator: config.role.is_authority(),
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
	let (_command_sink, commands_stream) = futures::channel::mpsc::channel(1024);

	let _pool = transaction_pool.pool().clone();

	let client_for_cidp = client.clone();

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
			create_inherent_data_providers: move |block, _| {
				let current_para_block = client_for_cidp
					.header(block)
					.ok()
					.flatten()
					.expect("Header lookup should succeed")
					.number;
				let current_para_block_head = client_for_cidp.header(block).expect("UnknownBlock");
				let current_para_block_head =
					Some(polkadot_primitives::HeadData(current_para_block_head.encode()));

				let downward_xcm_receiver = downward_xcm_receiver.clone();
				let hrmp_xcm_receiver = hrmp_xcm_receiver.clone();
				let client_for_xcm = client_for_cidp.clone();
				async move {
					let mocked_parachain = MockValidationDataInherentDataProvider {
						current_para_block,
						current_para_block_head,
						para_id,
						relay_offset: 1000,
						relay_blocks_per_para_block: 2,
						para_blocks_per_relay_epoch: 0,
						relay_randomness_config: (),
						xcm_config: MockXcmConfig::new(&*client_for_xcm, block, Default::default()),
						raw_downward_messages: downward_xcm_receiver.drain().collect(),
						raw_horizontal_messages: hrmp_xcm_receiver.drain().collect(),
						additional_key_values: None,
					};

					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					Ok((timestamp, mocked_parachain))
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
		let is_authority = config.role.is_authority();
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
				command_sink: None,
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
		config,
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

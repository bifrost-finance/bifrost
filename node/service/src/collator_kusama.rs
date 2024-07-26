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

//! Service implementation. Specialized wrapper over substrate service.
use std::{sync::Arc, time::Duration};

#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
pub use bifrost_kusama_runtime;
use bifrost_kusama_runtime::RuntimeApi;
use cumulus_client_cli::CollatorOptions;
use cumulus_client_collator::service::CollatorService;
use cumulus_client_consensus_aura::collators::basic::{
	self as basic_aura, Params as BasicAuraParams,
};

use cumulus_client_consensus_common::ParachainBlockImport as TParachainBlockImport;
use cumulus_client_consensus_proposer::Proposer;

use crate::eth::EthConfiguration;
use bifrost_primitives::Block;
use cumulus_client_service::{
	build_network, build_relay_chain_interface, prepare_node_config, start_relay_chain_tasks,
	BuildNetworkParams, CollatorSybilResistance, DARecoveryProfile, StartRelayChainTasksParams,
};
use cumulus_primitives_core::{relay_chain::Hash, ParaId};
use cumulus_relay_chain_interface::{OverseerHandle, RelayChainInterface};
use polkadot_primitives::CollatorPair;
use sc_client_api::backend::Backend;
use sc_consensus::{ImportQueue, LongestChain};
use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::{service::traits::NetworkBackend, NetworkBlock};
use sc_network_sync::SyncingService;
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_keystore::KeystorePtr;
use substrate_prometheus_endpoint::Registry;

#[cfg(not(feature = "runtime-benchmarks"))]
type HostFunctions = cumulus_client_service::ParachainHostFunctions;

#[cfg(feature = "runtime-benchmarks")]
type HostFunctions = (
	cumulus_client_service::ParachainHostFunctions,
	frame_benchmarking::benchmarking::HostFunctions,
);
pub type FullBackend = TFullBackend<Block>;
pub type FullClient = TFullClient<Block, RuntimeApi, WasmExecutor<HostFunctions>>;
pub type MaybeFullSelectChain = Option<LongestChain<FullBackend, Block>>;
type ParachainBlockImport = TParachainBlockImport<Block, Arc<FullClient>, FullBackend>;

pub fn new_partial(
	config: &Configuration,
	dev: bool,
) -> Result<
	PartialComponents<
		FullClient,
		FullBackend,
		MaybeFullSelectChain,
		sc_consensus::import_queue::BasicQueue<Block>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(ParachainBlockImport, Option<Telemetry>, Option<TelemetryWorkerHandle>),
	>,
	sc_service::Error,
> {
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let heap_pages = config
		.default_heap_pages
		.map_or(DEFAULT_HEAP_ALLOC_STRATEGY, |h| HeapAllocStrategy::Static { extra_pages: h as _ });

	let executor = sc_executor::WasmExecutor::<HostFunctions>::builder()
		.with_execution_method(config.wasm_method)
		.with_max_runtime_instances(config.max_runtime_instances)
		.with_runtime_cache_size(config.runtime_cache_size)
		.with_onchain_heap_alloc_strategy(heap_pages)
		.with_offchain_heap_alloc_strategy(heap_pages)
		.build();

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts_record_import::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
			true,
		)?;
	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let registry = config.prometheus_registry();

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		registry,
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let select_chain = if dev { Some(LongestChain::new(backend.clone())) } else { None };

	let block_import = ParachainBlockImport::new(client.clone(), backend.clone());

	let import_queue = if dev {
		sc_consensus_manual_seal::import_queue(
			Box::new(client.clone()),
			&task_manager.spawn_essential_handle(),
			registry,
		)
	} else {
		build_import_queue(
			client.clone(),
			block_import.clone(),
			config,
			telemetry.as_ref().map(|telemetry| telemetry.handle()),
			&task_manager,
		)?
	};

	Ok(PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain,
		other: (block_import, telemetry, telemetry_worker_handle),
	})
}

/// Build the import queue for the parachain runtime.
fn build_import_queue(
	client: Arc<FullClient>,
	block_import: ParachainBlockImport,
	config: &Configuration,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
) -> Result<sc_consensus::DefaultImportQueue<Block>, sc_service::Error> {
	let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

	cumulus_client_consensus_aura::import_queue::<
		sp_consensus_aura::sr25519::AuthorityPair,
		_,
		_,
		_,
		_,
		_,
	>(cumulus_client_consensus_aura::ImportQueueParams {
		block_import,
		client,
		create_inherent_data_providers: move |_, _| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

			let slot =
				sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);

			Ok((slot, timestamp))
		},
		registry: config.prometheus_registry(),
		spawner: &task_manager.spawn_essential_handle(),
		telemetry,
	})
	.map_err(Into::into)
}

fn start_consensus(
	client: Arc<FullClient>,
	block_import: ParachainBlockImport,
	prometheus_registry: Option<&Registry>,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
	relay_chain_interface: Arc<dyn RelayChainInterface>,
	transaction_pool: Arc<sc_transaction_pool::FullPool<Block, FullClient>>,
	sync_oracle: Arc<SyncingService<Block>>,
	keystore: KeystorePtr,
	relay_chain_slot_duration: Duration,
	para_id: ParaId,
	collator_key: CollatorPair,
	overseer_handle: OverseerHandle,
	announce_block: Arc<dyn Fn(Hash, Option<Vec<u8>>) + Send + Sync>,
) -> Result<(), sc_service::Error> {
	let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool,
		prometheus_registry,
		telemetry.clone(),
	);

	let proposer = Proposer::new(proposer_factory);

	let collator_service = CollatorService::new(
		client.clone(),
		Arc::new(task_manager.spawn_handle()),
		announce_block,
		client.clone(),
	);

	let params = BasicAuraParams {
		create_inherent_data_providers: move |_, ()| async move { Ok(()) },
		block_import,
		para_client: client,
		relay_client: relay_chain_interface,
		sync_oracle,
		keystore,
		collator_key,
		para_id,
		overseer_handle,
		relay_chain_slot_duration,
		proposer,
		collator_service,
		// Very limited proposal time.
		authoring_duration: Duration::from_millis(500),
		collation_request_receiver: None,
	};

	let fut =
		basic_aura::run::<Block, sp_consensus_aura::sr25519::AuthorityPair, _, _, _, _, _, _, _>(
			params,
		);
	task_manager.spawn_essential_handle().spawn("aura", None, fut);

	Ok(())
}

/// Start a node with the given parachain `Configuration` and relay chain
/// `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the
/// runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain🌈")]
async fn start_node_impl<Net>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	_eth_config: EthConfiguration,
	collator_options: CollatorOptions,
	sybil_resistance_level: CollatorSybilResistance,
	para_id: ParaId,
	hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)>
where
	Net: NetworkBackend<Block, Hash>,
{
	let parachain_config = prepare_node_config(parachain_config);

	let params = new_partial(&parachain_config, false)?;
	let (block_import, mut telemetry, telemetry_worker_handle) = params.other;

	let client = params.client.clone();
	let backend = params.backend.clone();
	let mut task_manager = params.task_manager;

	let (relay_chain_interface, collator_key) = build_relay_chain_interface(
		polkadot_config,
		&parachain_config,
		telemetry_worker_handle,
		&mut task_manager,
		collator_options.clone(),
		hwbench.clone(),
	)
	.await
	.map_err(|e| sc_service::Error::Application(Box::new(e) as Box<_>))?;

	let validator = parachain_config.role.is_authority();
	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let import_queue_service = params.import_queue.service();
	let net_config =
		sc_network::config::FullNetworkConfiguration::<_, _, Net>::new(&parachain_config.network);
	let (network, system_rpc_tx, tx_handler_controller, start_network, sync_service) =
		build_network(BuildNetworkParams {
			parachain_config: &parachain_config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			para_id,
			spawn_handle: task_manager.spawn_handle(),
			relay_chain_interface: relay_chain_interface.clone(),
			import_queue: params.import_queue,
			sybil_resistance_level,
		})
		.await?;

	if parachain_config.offchain_worker.enabled {
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
				is_validator: parachain_config.role.is_authority(),
				enable_http_requests: false,
				custom_extensions: move |_| vec![],
			})
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
	}

	let rpc_builder = {
		let client = client.clone();
		let transaction_pool = transaction_pool.clone();

		Box::new(move |deny_unsafe, _| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				deny_unsafe,
			};
			crate::rpc::create_full(deps).map_err(Into::into)
		})
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		rpc_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.keystore(),
		backend,
		network: network.clone(),
		sync_service: sync_service.clone(),
		system_rpc_tx,
		tx_handler_controller,
		telemetry: telemetry.as_mut(),
	})?;

	if let Some(hwbench) = hwbench {
		sc_sysinfo::print_hwbench(&hwbench);

		if let Some(ref mut telemetry) = telemetry {
			let telemetry_handle = telemetry.handle();
			task_manager.spawn_handle().spawn(
				"telemetry_hwbench",
				None,
				sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
			);
		}
	}

	let announce_block = {
		let sync_service = sync_service.clone();
		Arc::new(move |hash, data| sync_service.announce_block(hash, data))
	};

	let relay_chain_slot_duration = Duration::from_secs(6);

	let overseer_handle = relay_chain_interface
		.overseer_handle()
		.map_err(|e| sc_service::Error::Application(Box::new(e)))?;

	start_relay_chain_tasks(StartRelayChainTasksParams {
		client: client.clone(),
		announce_block: announce_block.clone(),
		para_id,
		relay_chain_interface: relay_chain_interface.clone(),
		task_manager: &mut task_manager,
		da_recovery_profile: if validator {
			DARecoveryProfile::Collator
		} else {
			DARecoveryProfile::FullNode
		},
		import_queue: import_queue_service,
		relay_chain_slot_duration,
		recovery_handle: Box::new(overseer_handle.clone()),
		sync_service: sync_service.clone(),
	})?;

	if validator {
		start_consensus(
			client.clone(),
			block_import,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			relay_chain_interface.clone(),
			transaction_pool,
			sync_service.clone(),
			params.keystore_container.keystore(),
			relay_chain_slot_duration,
			para_id,
			collator_key.expect("Command line arguments do not allow this. qed"),
			overseer_handle,
			announce_block,
		)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

/// Start a normal parachain node.
pub async fn start_node<Net: NetworkBackend<Block, Hash>>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	eth_config: EthConfiguration,
	collator_options: CollatorOptions,
	para_id: ParaId,
	hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)> {
	start_node_impl::<Net>(
		parachain_config,
		polkadot_config,
		eth_config,
		collator_options,
		CollatorSybilResistance::Resistant,
		para_id,
		hwbench,
	)
	.await
}

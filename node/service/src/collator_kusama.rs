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
use std::{
	collections::BTreeMap,
	path::Path,
	sync::{Arc, Mutex},
	time::Duration,
};

#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
pub use bifrost_kusama_runtime;
use bifrost_kusama_runtime::{RuntimeApi, TransactionConverter};
use cumulus_client_cli::CollatorOptions;
use cumulus_client_collator::service::CollatorService;
use cumulus_client_consensus_aura::collators::basic::{
	self as basic_aura, Params as BasicAuraParams,
};

use cumulus_client_consensus_common::ParachainBlockImport as TParachainBlockImport;
use cumulus_client_consensus_proposer::Proposer;
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;

use bifrost_primitives::Block;
use cumulus_client_service::{
	build_network, build_relay_chain_interface, prepare_node_config, start_relay_chain_tasks,
	BuildNetworkParams, CollatorSybilResistance, DARecoveryProfile, StartRelayChainTasksParams,
};
use cumulus_primitives_core::{relay_chain::Hash, ParaId};
use cumulus_relay_chain_interface::{OverseerHandle, RelayChainInterface};
use polkadot_cli::IdentifyVariant;
use polkadot_primitives::{CollatorPair, PersistedValidationData};
use sc_client_api::backend::Backend;
use sc_consensus::{ImportQueue, LongestChain};
use sc_executor::{HeapAllocStrategy, WasmExecutor, DEFAULT_HEAP_ALLOC_STRATEGY};
use sc_network::NetworkBlock;
use sc_network_sync::SyncingService;
use sc_service::{Configuration, PartialComponents, TFullBackend, TFullClient, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_core::U256;
use sp_keystore::KeystorePtr;
use substrate_prometheus_endpoint::Registry;

use crate::{
	dev,
	eth::{db_config_dir, spawn_frontier_tasks, BackendType, EthConfiguration, FrontierBackend},
};

#[cfg(not(feature = "runtime-benchmarks"))]
type HostFunctions = sp_io::SubstrateHostFunctions;

#[cfg(feature = "runtime-benchmarks")]
type HostFunctions =
	(sp_io::SubstrateHostFunctions, frame_benchmarking::benchmarking::HostFunctions);

#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
pub struct BifrostExecutor;
#[cfg(any(feature = "with-bifrost-kusama-runtime", feature = "with-bifrost-runtime"))]
impl sc_executor::NativeExecutionDispatch for BifrostExecutor {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		bifrost_kusama_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		bifrost_kusama_runtime::native_version()
	}
}

pub type FullBackend = TFullBackend<Block>;
pub type FullClient = TFullClient<Block, RuntimeApi, WasmExecutor<HostFunctions>>;
pub type MaybeFullSelectChain = Option<LongestChain<FullBackend, Block>>;
type ParachainBlockImport = TParachainBlockImport<Block, Arc<FullClient>, FullBackend>;

pub fn new_partial(
	config: &Configuration,
	eth_config: &EthConfiguration,
	dev: bool,
) -> Result<
	PartialComponents<
		FullClient,
		FullBackend,
		MaybeFullSelectChain,
		sc_consensus::import_queue::BasicQueue<Block>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(ParachainBlockImport, Option<Telemetry>, Option<TelemetryWorkerHandle>, FrontierBackend),
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
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
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

	let overrides = fc_storage::overrides_handle(client.clone());
	let frontier_backend = match eth_config.frontier_backend_type {
		BackendType::KeyValue => FrontierBackend::KeyValue(fc_db::kv::Backend::open(
			Arc::clone(&client),
			&config.database,
			&db_config_dir(config),
		)?),
		BackendType::Sql => {
			let db_path = db_config_dir(config).join("sql");
			std::fs::create_dir_all(&db_path).expect("failed creating sql db directory");
			let backend = futures::executor::block_on(fc_db::sql::Backend::new(
				fc_db::sql::BackendConfig::Sqlite(fc_db::sql::SqliteBackendConfig {
					path: Path::new("sqlite:///")
						.join(db_path)
						.join("frontier.db3")
						.to_str()
						.unwrap(),
					create_if_missing: true,
					thread_count: eth_config.frontier_sql_backend_thread_count,
					cache_size: eth_config.frontier_sql_backend_cache_size,
				}),
				eth_config.frontier_sql_backend_pool_size,
				std::num::NonZeroU32::new(eth_config.frontier_sql_backend_num_ops_timeout),
				overrides.clone(),
			))
			.unwrap_or_else(|err| panic!("failed creating sql backend: {:?}", err));
			FrontierBackend::Sql(backend)
		},
	};

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
		other: (block_import, telemetry, telemetry_worker_handle, frontier_backend),
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
	let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

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
		slot_duration,
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
async fn start_node_impl(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	eth_config: EthConfiguration,
	collator_options: CollatorOptions,
	sybil_resistance_level: CollatorSybilResistance,
	para_id: ParaId,
	hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)> {
	let parachain_config = prepare_node_config(parachain_config);

	let params = new_partial(&parachain_config, &eth_config, false)?;
	let (block_import, mut telemetry, telemetry_worker_handle, frontier_backend) = params.other;

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
	let net_config = sc_network::config::FullNetworkConfiguration::new(&parachain_config.network);
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
				network_provider: network.clone(),
				is_validator: parachain_config.role.is_authority(),
				enable_http_requests: false,
				custom_extensions: move |_| vec![],
			})
			.run(client.clone(), task_manager.spawn_handle())
			.boxed(),
		);
	}

	let overrides = crate::rpc::overrides_handle(client.clone());
	let _block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
		task_manager.spawn_handle(),
		overrides.clone(),
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

	let filter_pool = Some(Arc::new(Mutex::new(BTreeMap::new())));
	let fee_history_cache = Arc::new(Mutex::new(BTreeMap::new()));
	// let fee_history_cache_limit = parachain_config.fee_history_limit;
	let fee_history_cache_limit = 2048;

	let rpc_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let network = network.clone();
		let sync_service = sync_service.clone();

		let is_authority = parachain_config.role.is_authority();
		let enable_dev_signer = eth_config.enable_dev_signer;
		let max_past_logs = eth_config.max_past_logs;
		let execute_gas_limit_multiplier = eth_config.execute_gas_limit_multiplier;
		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let pubsub_notification_sinks = pubsub_notification_sinks.clone();
		let overrides = overrides.clone();
		let fee_history_cache = fee_history_cache.clone();
		let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
			task_manager.spawn_handle(),
			overrides.clone(),
			eth_config.eth_log_block_cache,
			eth_config.eth_statuses_cache,
			prometheus_registry.clone(),
		));

		let slot_duration = sc_consensus_aura::slot_duration(&*client)?;
		let target_gas_price = eth_config.target_gas_price;
		let pending_create_inherent_data_providers = move |_, ()| async move {
			let current = sp_timestamp::InherentDataProvider::from_system_time();
			let next_slot = current.timestamp().as_millis() + slot_duration.as_millis();
			let timestamp = sp_timestamp::InherentDataProvider::new(next_slot.into());
			let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
				*timestamp,
				slot_duration,
			);
			let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
			// Create a dummy parachain inherent data provider which is required to pass
			// the checks by the para chain system. We use dummy values because in the 'pending
			// context' neither do we have access to the real values nor do we need them.
			let (relay_parent_storage_root, relay_chain_state) =
				RelayStateSproofBuilder::default().into_state_root_and_proof();
			let vfp = PersistedValidationData {
				// This is a hack to make
				// `cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases` happy. Relay
				// parent number can't be bigger than u32::MAX.
				relay_parent_number: u32::MAX,
				relay_parent_storage_root,
				..Default::default()
			};
			let parachain_inherent_data = ParachainInherentData {
				validation_data: vfp,
				relay_chain_state,
				downward_messages: Default::default(),
				horizontal_messages: Default::default(),
			};
			Ok((slot, timestamp, dynamic_fee, parachain_inherent_data))
		};

		Box::new(move |deny_unsafe, subscription_task_executor| {
			let eth_deps = crate::rpc::EthDeps {
				client: client.clone(),
				pool: pool.clone(),
				graph: pool.pool().clone(),
				converter: Some(TransactionConverter),
				is_authority,
				enable_dev_signer,
				network: network.clone(),
				sync: sync_service.clone(),
				frontier_backend: match frontier_backend.clone() {
					fc_db::Backend::KeyValue(b) => Arc::new(b),
					fc_db::Backend::Sql(b) => Arc::new(b),
				},
				overrides: overrides.clone(),
				block_data_cache: block_data_cache.clone(),
				filter_pool: filter_pool.clone(),
				max_past_logs,
				fee_history_cache: fee_history_cache.clone(),
				fee_history_cache_limit,
				execute_gas_limit_multiplier,
				forced_parent_hashes: None,
				pending_create_inherent_data_providers,
			};
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
				command_sink: None,
				eth: eth_deps,
			};
			crate::rpc::create_full(
				deps,
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
		overrides,
		fee_history_cache,
		fee_history_cache_limit,
		sync_service.clone(),
		pubsub_notification_sinks,
	)
	.await;

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
pub async fn start_node(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	eth_config: EthConfiguration,
	collator_options: CollatorOptions,
	para_id: ParaId,
	hwbench: Option<sc_sysinfo::HwBench>,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient>)> {
	if parachain_config.chain_spec.is_dev() {
		dev::start_node(parachain_config, eth_config).await
	} else {
		start_node_impl(
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
}

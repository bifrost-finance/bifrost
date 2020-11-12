// Copyright 2019-2020 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

#![warn(unused_extern_crates)]

//! Service implementation. Specialized wrapper over substrate service.

pub mod chain_spec;
mod client;

use std::sync::Arc;
use sc_consensus_babe;
use sc_finality_grandpa::{self as grandpa, FinalityProofProvider as GrandpaFinalityProofProvider};
use node_primitives::Block;
pub use sc_service::{
	Role, PruningMode, TransactionPoolOptions, RuntimeGenesis,
	TFullClient, TLightClient, TFullBackend, TLightBackend, TFullCallExecutor, TLightCallExecutor,
	ChainSpec, RpcHandlers, TaskManager,
	config::{Configuration}, error::{Error as ServiceError},
};
use sp_inherents::InherentDataProviders;
use sc_network::{Event, NetworkService};
use sp_runtime::traits::{Block as BlockT, BlakeTwo256};
use futures::prelude::*;
use sc_client_api::{ExecutorProvider, RemoteBackend};
use sc_executor::native_executor_instance;

use sp_trie::PrefixedMemoryDB;
pub use self::client::{AbstractClient, Client, ClientHandle, ExecuteWithClient, RuntimeApiCollection};
pub use sp_api::{ApiRef, Core as CoreApi, ConstructRuntimeApi, ProvideRuntimeApi, StateBackend};
pub use sc_executor::NativeExecutionDispatch;
pub use chain_spec::{AsgardChainSpec, BifrostChainSpec, RococoChainSpec};

pub use asgard_runtime;
pub use bifrost_runtime;
pub use rococo_runtime;

native_executor_instance!(
	pub AsgardExecutor,
	asgard_runtime::api::dispatch,
	asgard_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

native_executor_instance!(
	pub BifrostExecutor,
	bifrost_runtime::api::dispatch,
	bifrost_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

native_executor_instance!(
	pub RococoExecutor,
	rococo_runtime::api::dispatch,
	rococo_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

/// Can be called for a `Configuration` to check if it is a configuration for the `Bifrost` network.
pub trait IdentifyVariant {
	/// Returns if this is a configuration for the `Asgard` network.
	fn is_asgard(&self) -> bool;

	/// Returns if this is a configuration for the `Bifrost` network.
	fn is_bifrost(&self) -> bool;

	/// Returns if this is a configuration for the `Rococo` network.
	fn is_rococo(&self) -> bool;
}

impl IdentifyVariant for Box<dyn ChainSpec> {
	fn is_asgard(&self) -> bool {
		self.id().starts_with("asgard") || self.id().starts_with("asg")
	}
	fn is_bifrost(&self) -> bool {
		self.id().starts_with("bifrost") || self.id().starts_with("bnc")
	}
	fn is_rococo(&self) -> bool {
		self.id().starts_with("rococo") || self.id().starts_with("rco")
	}
}

type FullClient<RuntimeApi, Executor> = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport<RuntimeApi, Executor> = grandpa::GrandpaBlockImport<
	FullBackend, Block, FullClient<RuntimeApi, Executor>, FullSelectChain
>;

// type LightClient = sc_service::TLightClient<Block, RuntimeApi, Executor>;
type LightBackend = sc_service::TLightBackendWithHash<Block, sp_runtime::traits::BlakeTwo256>;

type LightClient<RuntimeApi, Executor> =
	sc_service::TLightClientWithBackend<Block, RuntimeApi, Executor, LightBackend>;

pub fn new_partial<RuntimeApi, Executor>(
	config: &Configuration
) -> Result<sc_service::PartialComponents<
	FullClient<RuntimeApi, Executor>, FullBackend, FullSelectChain,
	sp_consensus::DefaultImportQueue<Block, FullClient<RuntimeApi, Executor>>,
	sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
	(
		impl Fn(
			node_rpc::DenyUnsafe,
			sc_rpc::SubscriptionTaskExecutor,
		) -> node_rpc::IoHandler,
		(
			sc_consensus_babe::BabeBlockImport<Block, FullClient<RuntimeApi, Executor>, FullGrandpaBlockImport<RuntimeApi, Executor>>,
			grandpa::LinkHalf<Block, FullClient<RuntimeApi, Executor>, FullSelectChain>,
			sc_consensus_babe::BabeLink<Block>,
		),
		(
			grandpa::SharedVoterState,
			Arc<GrandpaFinalityProofProvider<FullBackend, Block>>,
		),
	)
>, ServiceError>
	where
		RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
		RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
		Executor: NativeExecutionDispatch + 'static,
{
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
	let client = Arc::new(client);

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
	);

	let (grandpa_block_import, grandpa_link) = grandpa::block_import(
		client.clone(), &(client.clone() as Arc<_>), select_chain.clone(),
	)?;
	let justification_import = grandpa_block_import.clone();

	let (block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::Config::get_or_compute(&*client)?,
		grandpa_block_import,
		client.clone(),
	)?;

	let inherent_data_providers = sp_inherents::InherentDataProviders::new();

	let import_queue = sc_consensus_babe::import_queue(
		babe_link.clone(),
		block_import.clone(),
		Some(Box::new(justification_import)),
		None,
		client.clone(),
		select_chain.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
		sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
	)?;

	let import_setup = (block_import, grandpa_link, babe_link);

	let (rpc_extensions_builder, rpc_setup) = {
		let (_, grandpa_link, babe_link) = &import_setup;

		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let shared_voter_state = grandpa::SharedVoterState::empty();
		let finality_proof_provider =
			GrandpaFinalityProofProvider::new_for_service(backend.clone(), client.clone());

		let rpc_setup = (shared_voter_state.clone(), finality_proof_provider.clone());

		let babe_config = babe_link.config().clone();
		let shared_epoch_changes = babe_link.epoch_changes().clone();

		let client = client.clone();
		let pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let keystore = keystore_container.sync_keystore();
		let chain_spec = config.chain_spec.cloned_box();

		let rpc_extensions_builder = move |deny_unsafe, subscription_executor| {
			let deps = node_rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				select_chain: select_chain.clone(),
				chain_spec: chain_spec.cloned_box(),
				deny_unsafe,
				babe: node_rpc::BabeDeps {
					babe_config: babe_config.clone(),
					shared_epoch_changes: shared_epoch_changes.clone(),
					keystore: keystore.clone(),
				},
				grandpa: node_rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor,
					finality_provider: finality_proof_provider.clone(),
				},
			};

			node_rpc::create_full(deps)
		};

		(rpc_extensions_builder, rpc_setup)
	};

	Ok(sc_service::PartialComponents {
		client, backend, task_manager, keystore_container,
		select_chain, import_queue, transaction_pool, inherent_data_providers,
		other: (rpc_extensions_builder, import_setup, rpc_setup)
	})
}

pub struct NewFullBase<RuntimeApi, Executor> {
	pub task_manager: TaskManager,
	pub inherent_data_providers: InherentDataProviders,
	pub client: Arc<FullClient<RuntimeApi, Executor>>,
	pub network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
	pub network_status_sinks: sc_service::NetworkStatusSinks<Block>,
}

/// Creates a full service from the configuration.
pub fn new_full_base<RuntimeApi, Executor>(
	config: Configuration
) -> Result<NewFullBase<RuntimeApi, Executor>, ServiceError>
	where
		RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
		RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
		Executor: NativeExecutionDispatch + 'static,
{
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		inherent_data_providers,
		other: (rpc_extensions_builder, import_setup, rpc_setup),
	} = new_partial::<RuntimeApi, Executor>(&config)?;

	let (shared_voter_state, finality_proof_provider) = rpc_setup;

	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: None,
			finality_proof_request_builder: None,
			finality_proof_provider: Some(finality_proof_provider.clone()),
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config, backend.clone(), task_manager.spawn_handle(), client.clone(), network.clone(),
		);
	}

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	let telemetry_connection_sinks = sc_service::TelemetryConnectionSinks::default();

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config,
		backend: backend.clone(),
		client: client.clone(),
		keystore: keystore_container.sync_keystore(),
		network: network.clone(),
		rpc_extensions_builder: Box::new(rpc_extensions_builder),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		on_demand: None,
		remote_blockchain: None,
		telemetry_connection_sinks: telemetry_connection_sinks.clone(),
		network_status_sinks: network_status_sinks.clone(),
		system_rpc_tx,
	})?;

	let (block_import, grandpa_link, babe_link) = import_setup;

	if let sc_service::config::Role::Authority { .. } = &role {
		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
		);

		let can_author_with =
			sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.sync_keystore(),
			client: client.clone(),
			select_chain,
			env: proposer,
			block_import,
			sync_oracle: network.clone(),
			inherent_data_providers: inherent_data_providers.clone(),
			force_authoring,
			babe_link,
			can_author_with,
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;
		task_manager.spawn_essential_handle().spawn_blocking("babe-proposer", babe);
	}

	// Spawn authority discovery module.
	if role.is_authority() {
		let authority_discovery_role = sc_authority_discovery::Role::PublishAndDiscover(
			keystore_container.keystore(),
		);
		let dht_event_stream = network.event_stream("authority-discovery")
			.filter_map(|e| async move { match e {
				Event::Dht(e) => Some(e),
				_ => None,
			}});
		let (authority_discovery_worker, _service) = sc_authority_discovery::new_worker_and_service(
			client.clone(),
			network.clone(),
			Box::pin(dht_event_stream),
			authority_discovery_role,
			prometheus_registry.clone(),
		);

		task_manager.spawn_handle().spawn("authority-discovery-worker", authority_discovery_worker.run());
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore = if role.is_authority() {
		Some(keystore_container.sync_keystore())
	} else {
		None
	};

	let config = grandpa::Config {
		// FIXME #1578 make this available through chainspec
		gossip_duration: std::time::Duration::from_millis(333),
		justification_period: 512,
		name: Some(name),
		observer_enabled: false,
		keystore,
		is_authority: role.is_network_authority(),
	};

	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_config = grandpa::GrandpaParams {
			config,
			link: grandpa_link,
			network: network.clone(),
			telemetry_on_connect: Some(telemetry_connection_sinks.on_connect_stream()),
			voting_rule: grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state,
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			grandpa::run_grandpa_voter(grandpa_config)?
		);
	} else {
		grandpa::setup_disabled_grandpa(network.clone())?;
	}

	network_starter.start_network();
	Ok(NewFullBase {
		task_manager,
		inherent_data_providers,
		client,
		network,
		network_status_sinks,
	})
}

/// Builds a new service for a full client.
pub fn new_full<RuntimeApi, Executor>(
	config: Configuration
) -> Result<TaskManager, ServiceError>
	where
		RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
		RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
		Executor: NativeExecutionDispatch + 'static,
{
	new_full_base::<RuntimeApi, Executor>(config).map(|NewFullBase { task_manager, .. }| {
		task_manager
	})
}

pub struct NewLightBase<RuntimeApi, Executor> {
	pub task_manager: TaskManager,
	pub rpc_handlers: RpcHandlers,
	pub client: Arc<LightClient<RuntimeApi, Executor>>,
	pub network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
}

pub fn new_light_base<RuntimeApi, Executor>(
	config: Configuration
) -> Result<NewLightBase<RuntimeApi, Executor>, ServiceError>
	where
		RuntimeApi: ConstructRuntimeApi<Block, LightClient<RuntimeApi, Executor>> + Send + Sync + 'static,
		RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<LightBackend, Block>>,
		Executor: NativeExecutionDispatch + 'static,
{
	let (client, backend, keystore_container, mut task_manager, on_demand) =
		sc_service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
		config.transaction_pool.clone(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
		on_demand.clone(),
	));

	let grandpa_block_import = grandpa::light_block_import(
		client.clone(), backend.clone(), &(client.clone() as Arc<_>),
		Arc::new(on_demand.checker().clone()),
	)?;

	let finality_proof_import = grandpa_block_import.clone();
	let finality_proof_request_builder =
		finality_proof_import.create_finality_proof_request_builder();

	let (babe_block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::Config::get_or_compute(&*client)?,
		grandpa_block_import,
		client.clone(),
	)?;

	let inherent_data_providers = sp_inherents::InherentDataProviders::new();

	let import_queue = sc_consensus_babe::import_queue(
		babe_link,
		babe_block_import,
		None,
		Some(Box::new(finality_proof_import)),
		client.clone(),
		select_chain.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
		sp_consensus::NeverCanAuthor,
	)?;

	let finality_proof_provider =
		GrandpaFinalityProofProvider::new_for_service(backend.clone(), client.clone());

	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: Some(on_demand.clone()),
			block_announce_validator_builder: None,
			finality_proof_request_builder: Some(finality_proof_request_builder),
			finality_proof_provider: Some(finality_proof_provider),
		})?;
	network_starter.start_network();

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config, backend.clone(), task_manager.spawn_handle(), client.clone(), network.clone(),
		);
	}

	let light_deps = node_rpc::LightDeps {
		remote_blockchain: backend.remote_blockchain(),
		fetcher: on_demand.clone(),
		client: client.clone(),
		pool: transaction_pool.clone(),
	};

	let rpc_extensions = node_rpc::create_light(light_deps);

	let rpc_handlers =
		sc_service::spawn_tasks(sc_service::SpawnTasksParams {
			on_demand: Some(on_demand),
			remote_blockchain: Some(backend.remote_blockchain()),
			rpc_extensions_builder: Box::new(sc_service::NoopRpcExtensionBuilder(rpc_extensions)),
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			keystore: keystore_container.sync_keystore(),
			config, backend, network_status_sinks, system_rpc_tx,
			network: network.clone(),
			telemetry_connection_sinks: sc_service::TelemetryConnectionSinks::default(),
			task_manager: &mut task_manager,
		})?;

	Ok(NewLightBase {
		task_manager,
		rpc_handlers,
		client,
		network,
	})
}

/// Builds a new service for a light client.
pub fn new_light<RuntimeApi, Executor>(
	config: Configuration
) -> Result<TaskManager, ServiceError>
	where
		RuntimeApi: ConstructRuntimeApi<Block, LightClient<RuntimeApi, Executor>> + Send + Sync + 'static,
		RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<LightBackend, Block>>,
		Executor: NativeExecutionDispatch + 'static,
{
	new_light_base::<RuntimeApi, Executor>(config).map(|NewLightBase { task_manager, .. }| {
		task_manager
	})
}

/// Builds a new object suitable for chain operations.
pub fn new_chain_ops(mut config: &mut Configuration) -> Result<
	(
		Arc<Client>,
		Arc<FullBackend>,
		sp_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		TaskManager,
	),
	ServiceError
>
{
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	if config.chain_spec.is_asgard() {
		let sc_service::PartialComponents { client, backend, import_queue, task_manager, .. }
			= new_partial::<asgard_runtime::RuntimeApi, AsgardExecutor>(config)?;
		Ok((Arc::new(Client::Asgard(client)), backend, import_queue, task_manager))
	} else if config.chain_spec.is_bifrost() {
		let sc_service::PartialComponents { client, backend, import_queue, task_manager, .. }
			= new_partial::<bifrost_runtime::RuntimeApi, BifrostExecutor>(config)?;
		Ok((Arc::new(Client::Bifrost(client)), backend, import_queue, task_manager))
	} else if config.chain_spec.is_rococo() {
		let sc_service::PartialComponents { client, backend, import_queue, task_manager, .. }
			= new_partial::<rococo_runtime::RuntimeApi, RococoExecutor>(config)?;
		Ok((Arc::new(Client::Rococo(client)), backend, import_queue, task_manager))
	} else {
		let sc_service::PartialComponents { client, backend, import_queue, task_manager, .. }
			= new_partial::<bifrost_runtime::RuntimeApi, BifrostExecutor>(config)?;
		Ok((Arc::new(Client::Bifrost(client)), backend, import_queue, task_manager))
	}
}

pub fn build_light(config: Configuration) -> Result<TaskManager, ServiceError> {
	if config.chain_spec.is_asgard() {
		new_light_base::<asgard_runtime::RuntimeApi, AsgardExecutor>(
			config
		).map(|full| full.task_manager)
	} else if config.chain_spec.is_bifrost() {
		new_light_base::<bifrost_runtime::RuntimeApi, BifrostExecutor>(
			config
		).map(|full| full.task_manager)
	} else if config.chain_spec.is_rococo() {
		new_light_base::<rococo_runtime::RuntimeApi, RococoExecutor>(
			config
		).map(|full| full.task_manager)
	} else {
		new_light_base::<bifrost_runtime::RuntimeApi, BifrostExecutor>(
			config
		).map(|full| full.task_manager)
	}
}

pub fn build_full(config: Configuration) -> Result<TaskManager, ServiceError> {
	if config.chain_spec.is_asgard() {
		new_full_base::<asgard_runtime::RuntimeApi, AsgardExecutor>(
			config
		).map(|full| full.task_manager)
	} else if config.chain_spec.is_bifrost() {
		new_full_base::<bifrost_runtime::RuntimeApi, BifrostExecutor>(
			config
		).map(|full| full.task_manager)
	} else if config.chain_spec.is_rococo() {
		new_full_base::<rococo_runtime::RuntimeApi, RococoExecutor>(
			config
		).map(|full| full.task_manager)
	} else {
		new_full_base::<bifrost_runtime::RuntimeApi, BifrostExecutor>(
			config
		).map(|full| full.task_manager)
	}
}

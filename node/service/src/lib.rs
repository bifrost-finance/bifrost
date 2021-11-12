// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

#![warn(unused_extern_crates)]

//! Service implementation. Specialized wrapper over substrate service.
use std::sync::Arc;

use cumulus_client_consensus_aura::{build_aura_consensus, BuildAuraConsensusParams};
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_client_network::build_block_announce_validator;
use cumulus_client_service::{
	prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use sc_client_api::ExecutorProvider;
use sc_consensus::LongestChain;
use sc_consensus_aura::SlotProportion;

pub mod chain_spec;
#[cfg(feature = "with-asgard-runtime")]
pub mod dev;
#[cfg(feature = "with-asgard-polkadot-runtime")]
pub use asgard_polkadot_runtime;
#[cfg(feature = "with-asgard-runtime")]
pub use asgard_runtime;
#[cfg(feature = "with-bifrost-polkadot-runtime")]
pub use bifrost_polkadot_runtime;
#[cfg(feature = "with-bifrost-runtime")]
pub use bifrost_runtime;
use node_rpc as rpc;
mod client;
pub use client::RuntimeApiCollection;
use node_primitives::{Block, Hash};
use sc_executor::NativeElseWasmExecutor;
use sc_network::NetworkService;
use sc_service::{
	error::Error as ServiceError, Configuration, PartialComponents, Role, TFullBackend, TaskManager,
};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_consensus::SlotData;
use sp_consensus_aura::sr25519::{AuthorityId as AuraId, AuthorityPair as AuraPair};
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use sp_trie::PrefixedMemoryDB;
use substrate_prometheus_endpoint::Registry;

use crate::client::Client;

#[cfg(feature = "with-asgard-runtime")]
pub struct AsgardExecutor;
#[cfg(feature = "with-asgard-runtime")]
impl sc_executor::NativeExecutionDispatch for AsgardExecutor {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		asgard_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		asgard_runtime::native_version()
	}
}

#[cfg(feature = "with-asgard-polkadot-runtime")]
pub struct AsgardPolkadotExecutor;
#[cfg(feature = "with-asgard-polkadot-runtime")]
impl sc_executor::NativeExecutionDispatch for AsgardPolkadotExecutor {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		asgard_polkadot_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		asgard_polkadot_runtime::native_version()
	}
}

#[cfg(feature = "with-bifrost-runtime")]
pub struct BifrostExecutor;
#[cfg(feature = "with-bifrost-runtime")]
impl sc_executor::NativeExecutionDispatch for BifrostExecutor {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		bifrost_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		bifrost_runtime::native_version()
	}
}

#[cfg(feature = "with-bifrost-polkadot-runtime")]
pub struct BifrostPolkadotExecutor;
#[cfg(feature = "with-bifrost-polkadot-runtime")]
impl sc_executor::NativeExecutionDispatch for BifrostPolkadotExecutor {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		bifrost_polkadot_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		bifrost_polkadot_runtime::native_version()
	}
}

pub type FullBackend = TFullBackend<Block>;

pub type FullClient<RuntimeApi, ExecutorDispatch> =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;

pub type MaybeFullSelectChain = Option<LongestChain<FullBackend, Block>>;

/// Can be called for a `Configuration` to check if it is a configuration for the `Bifrost` network.
pub trait IdentifyVariant {
	/// Returns if this is a configuration for the `Asgard` network.
	fn is_asgard(&self) -> bool;

	/// Returns if this is a configuration for the `AsgardPolkadot` network.
	fn is_asgard_polkadot(&self) -> bool;

	/// Returns if this is a configuration for the `Bifrost` network.
	fn is_bifrost(&self) -> bool;

	/// Returns if this is a configuration for the `BifrostPolkadot` network.
	fn is_bifrost_polkadot(&self) -> bool;

	/// Returns if this is a configuration for the `Dev` network.
	fn is_dev(&self) -> bool;
}

impl IdentifyVariant for Box<dyn sc_service::ChainSpec> {
	fn is_asgard(&self) -> bool {
		self.id().starts_with("asgard") && !self.id().starts_with("asgard_polkadot")
	}

	fn is_asgard_polkadot(&self) -> bool {
		self.id().starts_with("asgard_polkadot")
	}

	fn is_bifrost(&self) -> bool {
		self.id().starts_with("bifrost") && !self.id().starts_with("bifrost_polkadot")
	}

	fn is_bifrost_polkadot(&self) -> bool {
		self.id().starts_with("bifrost_polkadot")
	}

	fn is_dev(&self) -> bool {
		self.id().starts_with("dev")
	}
}

pub const BIFROST_RUNTIME_NOT_AVAILABLE: &str =
	"Bifrost runtime is not available. Please compile the node with `--features with-bifrost-runtime` to enable it.";
pub const ASGARD_RUNTIME_NOT_AVAILABLE: &str =
	"Asgard runtime is not available. Please compile the node with `--features with-asgard-runtime` to enable it.";
pub const ASGARD_POLKADOT_RUNTIME_NOT_AVAILABLE: &str =
	"Asgard-polkadot runtime is not available. Please compile the node with `--features with-asgard-polkadot-runtime` to enable it.";
pub const BIFROST_POLKADOT_RUNTIME_NOT_AVAILABLE: &str =
"Bifrost-polkadot runtime is not available. Please compile the node with `--features with-bifrost-polkadot-runtime` to enable it.";
pub const UNKNOWN_RUNTIME: &str = "Unknown runtime";

pub fn new_partial<RuntimeApi, Executor>(
	config: &Configuration,
	dev: bool,
) -> Result<
	PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		MaybeFullSelectChain,
		sc_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>,
		(Option<Telemetry>, Option<TelemetryWorkerHandle>),
	>,
	sc_service::Error,
>
where
	RuntimeApi:
		ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
{
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

	let executor = NativeElseWasmExecutor::<Executor>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", worker.run());
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

	let import_queue = if dev {
		sc_consensus_manual_seal::import_queue(
			Box::new(client.clone()),
			&task_manager.spawn_essential_handle(),
			registry,
		)
	} else {
		let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

		cumulus_client_consensus_aura::import_queue::<AuraPair, _, _, _, _, _, _>(
			cumulus_client_consensus_aura::ImportQueueParams {
				block_import: client.clone(),
				client: client.clone(),
				create_inherent_data_providers: move |_, _| async move {
					let time = sp_timestamp::InherentDataProvider::from_system_time();

					let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
						*time,
						slot_duration.slot_duration(),
					);

					Ok((time, slot))
				},
				registry,
				can_author_with: sp_consensus::CanAuthorWithNativeVersion::new(
					client.executor().clone(),
				),
				spawner: &task_manager.spawn_essential_handle(),
				telemetry: telemetry.as_ref().map(|telemetry| telemetry.handle()),
			},
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
		other: (telemetry, telemetry_worker_handle),
	})
}

/// Start a node with the given parachain `Configuration` and relay chain
/// `Configuration`.
///
/// This is the actual implementation that is abstract over the executor and the
/// runtime api.
#[sc_tracing::logging::prefix_logs_with("Parachain🌈")]
async fn start_node_impl<RB, RuntimeApi, Executor, BIC>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	id: ParaId,
	_rpc_ext_builder: RB,
	build_consensus: BIC,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
	RB: Fn(
			Arc<FullClient<RuntimeApi, Executor>>,
		) -> Result<jsonrpc_core::IoHandler<sc_rpc::Metadata>, sc_service::Error>
		+ Send
		+ 'static,
	RuntimeApi:
		ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
	BIC: FnOnce(
		Arc<FullClient<RuntimeApi, Executor>>,
		Option<&Registry>,
		Option<TelemetryHandle>,
		&TaskManager,
		&polkadot_service::NewFull<polkadot_service::Client>,
		Arc<sc_transaction_pool::FullPool<Block, FullClient<RuntimeApi, Executor>>>,
		Arc<NetworkService<Block, Hash>>,
		SyncCryptoStorePtr,
		bool,
	) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
	if matches!(parachain_config.role, Role::Light) {
		return Err("Light client not supported!".into());
	}

	let parachain_config = prepare_node_config(parachain_config);

	let params = new_partial(&parachain_config, false)?;
	let (mut telemetry, telemetry_worker_handle) = params.other;

	let relay_chain_full_node =
		cumulus_client_service::build_polkadot_full_node(polkadot_config, telemetry_worker_handle)
			.map_err(|e| match e {
				polkadot_service::Error::Sub(x) => x,
				s => format!("{}", s).into(),
			})?;

	let client = params.client.clone();
	let backend = params.backend.clone();
	let block_announce_validator = build_block_announce_validator(
		relay_chain_full_node.client.clone(),
		id,
		Box::new(relay_chain_full_node.network.clone()),
		relay_chain_full_node.backend.clone(),
	);

	let force_authoring = parachain_config.force_authoring;
	let validator = parachain_config.role.is_authority();
	let prometheus_registry = parachain_config.prometheus_registry().cloned();
	let transaction_pool = params.transaction_pool.clone();
	let mut task_manager = params.task_manager;
	let import_queue = cumulus_client_service::SharedImportQueue::new(params.import_queue);
	let (network, system_rpc_tx, start_network) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &parachain_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue: import_queue.clone(),
			on_demand: None,
			block_announce_validator_builder: Some(Box::new(|_| block_announce_validator)),
			warp_sync: None,
		})?;

	let is_bifrost = parachain_config.chain_spec.is_bifrost();
	let is_asgard = parachain_config.chain_spec.is_asgard();
	let is_asgard_polkadot = parachain_config.chain_spec.is_asgard_polkadot();

	let rpc_extensions_builder = {
		let client = client.clone();
		let transaction_pool = transaction_pool.clone();

		Box::new(move |deny_unsafe, _| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				deny_unsafe,
			};
			let rpc = if is_bifrost {
				crate::rpc::create_bifrost_rpc(deps)
			} else if is_asgard {
				crate::rpc::create_asgard_rpc(deps)
			} else if is_asgard_polkadot {
				crate::rpc::create_asgard_polkadot_rpc(deps)
			} else {
				crate::rpc::create_bifrost_polkadot_rpc(deps)
			};
			Ok(rpc)
		})
	};

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		on_demand: None,
		remote_blockchain: None,
		rpc_extensions_builder,
		client: client.clone(),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		config: parachain_config,
		keystore: params.keystore_container.sync_keystore(),
		backend: backend.clone(),
		network: network.clone(),
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;

	let announce_block = {
		let network = network.clone();
		Arc::new(move |hash, data| network.announce_block(hash, data))
	};

	if validator {
		let parachain_consensus = build_consensus(
			client.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			&relay_chain_full_node,
			transaction_pool,
			network,
			params.keystore_container.sync_keystore(),
			force_authoring,
		)?;

		let spawner = task_manager.spawn_handle();

		let params = StartCollatorParams {
			para_id: id,
			block_status: client.clone(),
			announce_block,
			client: client.clone(),
			task_manager: &mut task_manager,
			relay_chain_full_node,
			spawner,
			parachain_consensus,
			import_queue,
		};

		start_collator(params).await?;
	} else {
		let params = StartFullNodeParams {
			client: client.clone(),
			announce_block,
			task_manager: &mut task_manager,
			para_id: id,
			relay_chain_full_node,
		};

		start_full_node(params)?;
	}

	start_network.start_network();

	Ok((task_manager, client))
}

/// Start a normal parachain node.
pub async fn start_node<RuntimeApi, Executor>(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	id: ParaId,
) -> sc_service::error::Result<(TaskManager, Arc<FullClient<RuntimeApi, Executor>>)>
where
	RuntimeApi:
		ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>> + Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = sc_client_api::StateBackendFor<FullBackend, Block>>,
	RuntimeApi::RuntimeApi: sp_consensus_aura::AuraApi<Block, AuraId>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
{
	start_node_impl(
		parachain_config,
		polkadot_config,
		id,
		|_| Ok(Default::default()),
		|client,
		 prometheus_registry,
		 telemetry,
		 task_manager,
		 relay_chain_node,
		 transaction_pool,
		 sync_oracle,
		 keystore,
		 force_authoring| {
			let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

			let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
				task_manager.spawn_handle(),
				client.clone(),
				transaction_pool,
				prometheus_registry,
				telemetry.clone(),
			);

			let relay_chain_backend = relay_chain_node.backend.clone();
			let relay_chain_client = relay_chain_node.client.clone();
			Ok(build_aura_consensus::<AuraPair, _, _, _, _, _, _, _, _, _>(
				BuildAuraConsensusParams {
					proposer_factory,
					create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
						let parachain_inherent =
							cumulus_primitives_parachain_inherent::ParachainInherentData::create_at_with_client(
								relay_parent,
								&relay_chain_client,
								&*relay_chain_backend,
								&validation_data,
								id,
							);
						async move {
							let time = sp_timestamp::InherentDataProvider::from_system_time();

							let slot = sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_duration(
								*time,
								slot_duration.slot_duration(),
							);

							let parachain_inherent = parachain_inherent.ok_or_else(|| {
								Box::<dyn std::error::Error + Send + Sync>::from(
									"Failed to create parachain inherent",
								)
							})?;
							Ok((time, slot, parachain_inherent))
						}
					},
					block_import: client.clone(),
					relay_chain_client: relay_chain_node.client.clone(),
					relay_chain_backend: relay_chain_node.backend.clone(),
					para_client: client,
					backoff_authoring_blocks: Option::<()>::None,
					sync_oracle,
					keystore,
					force_authoring,
					slot_duration,
					// We got around 500ms for proposing
					block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
					// And a maximum of 750ms if slots are skipped
					max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
					telemetry,
				},
			))
		},
	)
	.await
}

/// Builds a new object suitable for chain operations.
pub fn new_chain_ops(
	mut config: &mut Configuration,
) -> Result<
	(
		Arc<Client>,
		Arc<FullBackend>,
		sc_consensus::import_queue::BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		TaskManager,
	),
	ServiceError,
> {
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	if config.chain_spec.is_asgard() {
		#[cfg(feature = "with-asgard-runtime")]
		{
			let PartialComponents { client, backend, import_queue, task_manager, .. } =
				new_partial::<asgard_runtime::RuntimeApi, AsgardExecutor>(config, false)?;
			Ok((Arc::new(Client::Asgard(client)), backend, import_queue, task_manager))
		}
		#[cfg(not(feature = "with-asgard-runtime"))]
		Err(ASGARD_RUNTIME_NOT_AVAILABLE.into())
	} else if config.chain_spec.is_bifrost() {
		#[cfg(feature = "with-bifrost-runtime")]
		{
			let PartialComponents { client, backend, import_queue, task_manager, .. } =
				new_partial::<bifrost_runtime::RuntimeApi, BifrostExecutor>(config, false)?;
			Ok((Arc::new(Client::Bifrost(client)), backend, import_queue, task_manager))
		}
		#[cfg(not(feature = "with-bifrost-runtime"))]
		Err(BIFROST_RUNTIME_NOT_AVAILABLE.into())
	} else if config.chain_spec.is_bifrost_polkadot() {
		#[cfg(feature = "with-bifrost-polkadot-runtime")]
		{
			let PartialComponents { client, backend, import_queue, task_manager, .. } =
				new_partial::<bifrost_polkadot_runtime::RuntimeApi, BifrostPolkadotExecutor>(
					config, false,
				)?;
			Ok((Arc::new(Client::BifrostPolkadot(client)), backend, import_queue, task_manager))
		}
		#[cfg(not(feature = "with-bifrost-polkadot-runtime"))]
		Err(BIFROST_POLKADOT_RUNTIME_NOT_AVAILABLE.into())
	} else if config.chain_spec.is_asgard_polkadot() {
		#[cfg(feature = "with-asgard-polkadot-runtime")]
		{
			let PartialComponents { client, backend, import_queue, task_manager, .. } =
				new_partial::<asgard_polkadot_runtime::RuntimeApi, AsgardPolkadotExecutor>(
					config, false,
				)?;
			Ok((Arc::new(Client::AsgardPolkadot(client)), backend, import_queue, task_manager))
		}
		#[cfg(not(feature = "with-asgard-polkadot-runtime"))]
		Err(ASGARD_POLKADOT_RUNTIME_NOT_AVAILABLE.into())
	} else {
		Err(UNKNOWN_RUNTIME.into())
	}
}

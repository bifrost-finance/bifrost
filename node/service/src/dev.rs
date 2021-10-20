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

use std::sync::Arc;

use cumulus_primitives_parachain_inherent::MockValidationDataInherentDataProvider;
pub use dev_runtime;
use futures::StreamExt;
use jsonrpc_core::IoHandler;
use node_rpc::{self, RpcExtension};
use sc_consensus::LongestChain;
use sc_executor::NativeElseWasmExecutor;
use sc_rpc::Metadata;
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, TaskManager};
use sc_telemetry::TelemetryWorker;

#[cfg(feature = "with-dev-runtime")]
pub struct DevRuntimeExecutor;
#[cfg(feature = "with-dev-runtime")]
impl sc_executor::NativeExecutionDispatch for DevRuntimeExecutor {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		dev_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		dev_runtime::native_version()
	}
}

pub type Block = dev_runtime::Block;
pub type Executor = DevRuntimeExecutor;
pub type RuntimeApi = dev_runtime::RuntimeApi;
pub type FullClient = sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>;
pub type FullBackend = sc_service::TFullBackend<Block>;
pub type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

pub fn default_mock_parachain_inherent_data_provider() -> MockValidationDataInherentDataProvider {
	MockValidationDataInherentDataProvider {
		current_para_block: 0,
		relay_offset: 1000,
		relay_blocks_per_para_block: 2,
	}
}

/// Builds the PartialComponents for a parachain or development service
///
/// Use this function if you don't actually need the full service, but just the partial in order to
/// be able to perform chain operations.
#[allow(clippy::type_complexity)]
pub fn new_partial(
	config: &Configuration,
) -> Result<
	PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block, FullClient>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(),
	>,
	ServiceError,
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

	let executor = NativeElseWasmExecutor::<Executor>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>(
			&config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;

	let client = Arc::new(client);

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let select_chain = LongestChain::new(backend.clone());

	// Depending whether we are
	let import_queue = sc_consensus_manual_seal::import_queue(
		Box::new(client.clone()),
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
	);

	Ok(PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain,
		other: (),
	})
}

/// Builds a new development service. This service uses manual seal, and mocks
/// the parachain inherent.
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (),
	} = new_partial(&config)?;

	let (network, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: None,
			warp_sync: None,
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let prometheus_registry = config.prometheus_registry().cloned();
	let role = config.role.clone();

	if role.is_authority() {
		let proposer_factory = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			None,
		);

		let pool = transaction_pool.pool().clone();
		let commands_stream = pool.validated_pool().import_notification_stream().map(|_| {
			sc_consensus_manual_seal::rpc::EngineCommand::SealNewBlock {
				create_empty: false,
				finalize: true,
				parent_hash: None,
				sender: None,
			}
		});

		let authorship_future =
			sc_consensus_manual_seal::run_manual_seal(sc_consensus_manual_seal::ManualSealParams {
				block_import: client.clone(),
				env: proposer_factory,
				client: client.clone(),
				pool: transaction_pool.clone(),
				commands_stream,
				select_chain,
				consensus_data_provider: None,
				create_inherent_data_providers: |_, _| async {
					Ok((
						sp_timestamp::InherentDataProvider::from_system_time(),
						default_mock_parachain_inherent_data_provider(),
					))
				},
			});
		// we spawn the future on a background thread managed by service.
		task_manager
			.spawn_essential_handle()
			.spawn_blocking("instant-seal", authorship_future);
	}

	let rpc_extensions_builder = {
		Box::new(move |_deny_unsafe, _| -> Result<IoHandler<Metadata>, _> {
			return Ok(RpcExtension::default());
		})
	};

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network,
		client,
		keystore: keystore_container.sync_keystore(),
		task_manager: &mut task_manager,
		transaction_pool,
		rpc_extensions_builder,
		on_demand: None,
		remote_blockchain: None,
		backend,
		system_rpc_tx,
		config,
		telemetry: None,
	})?;

	log::info!("Development Service Ready");

	network_starter.start_network();
	Ok(task_manager)
}

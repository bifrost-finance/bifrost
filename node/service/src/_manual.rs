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

use asgard_runtime::Header;
use polkadot_service::{FullBackend, LongestChain};
use sc_client_api::ExecutorProvider;
pub use sc_consensus_aura::{ImportQueueParams, StartAuraParams};
use sc_consensus_manual_seal::InstantSealParams;
use sc_consensus_slots::SlotProportion;
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::TransactionFor;
use sp_consensus::{import_queue::BasicQueue, SlotData};
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;
use sp_inherents::{CreateInherentDataProviders, InherentDataProvider};
use sp_runtime::generic;

use crate::AsgardExecutor;

type Block = generic::Block<Header, sp_runtime::OpaqueExtrinsic>;
type FullSelectChain = LongestChain<FullBackend, Block>;
type FullClientDev = sc_service::TFullClient<Block, asgard_runtime::RuntimeApi, AsgardExecutor>;

#[allow(clippy::type_complexity)]
pub fn new_partial(
	config: &Configuration,
) -> Result<
	PartialComponents<
		FullClientDev,
		FullBackend,
		FullSelectChain,
		BasicQueue<Block, TransactionFor<FullClientDev, Block>>,
		sc_transaction_pool::FullPool<Block, FullClientDev>,
		(),
	>,
	ServiceError,
> {
	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, asgard_runtime::RuntimeApi, AsgardExecutor>(
			&config, None,
		)?;
	let client = Arc::new(client);

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let import_queue = sc_consensus_manual_seal::import_queue(
		Box::new(client.clone()),
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
	);

	Ok(PartialComponents {
		client,
		backend,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain,
		other: (),
	})
}

pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		..
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
		})?;

	let keystore = keystore_container.sync_keystore();
	if config.offchain_worker.enabled {
		// Initialize seed for signing transaction using off-chain workers. This is a convenience
		// so learners can see the transactions submitted simply running the node.
		// Typically these keys should be inserted with RPC calls to `author_insertKey`.
		#[cfg(feature = "ocw")]
		{
			sp_keystore::SyncCryptoStore::sr25519_generate_new(
				&*keystore,
				runtime::ocw_demo::KEY_TYPE,
				Some("//Alice"),
			)
			.expect("Creating key with account Alice should succeed.");
		}

		sc_service::build_offchain_workers(
			&config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let is_authority = config.role.is_authority();
	let prometheus_registry = config.prometheus_registry().cloned();

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		network,
		client: client.clone(),
		keystore,
		task_manager: &mut task_manager,
		transaction_pool: transaction_pool.clone(),
		rpc_extensions_builder: Box::new(|_, _| ()),
		on_demand: None,
		remote_blockchain: None,
		backend,
		system_rpc_tx,
		config,
		telemetry: None,
	})?;

	if is_authority {
		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			None,
		);

		let authorship_future = sc_consensus_manual_seal::run_instant_seal(InstantSealParams {
			block_import: client.clone(),
			env: proposer,
			client,
			pool: transaction_pool.pool().clone(),
			select_chain,
			consensus_data_provider: None,
			create_inherent_data_providers: None,
		});

		task_manager
			.spawn_essential_handle()
			.spawn_blocking("instant-seal", authorship_future);
	};

	network_starter.start_network();
	Ok(task_manager)
}

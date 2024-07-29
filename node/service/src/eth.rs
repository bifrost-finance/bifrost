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

use std::{marker::PhantomData, path::PathBuf, sync::Arc, time::Duration};

use crate::collator_polkadot::{FullBackend, FullClient};
use bifrost_polkadot_runtime::opaque::Block;
use cumulus_client_consensus_common::ParachainBlockImportMarker;
use cumulus_primitives_core::BlockT;
use fc_consensus::Error;
pub use fc_consensus::FrontierBlockImport;
pub use fc_db::kv::Backend as FrontierBackend;
use fc_mapping_sync::{kv::MappingSyncWorker, SyncStrategy};
use fc_rpc::EthTask;
pub use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fc_storage::StorageOverride;
use fp_consensus::ensure_log;
use fp_rpc::EthereumRuntimeRPCApi;
use futures::{future, prelude::*};
use polkadot_service::HeaderT;
use sc_client_api::{AuxStore, BlockOf, BlockchainEvents};
use sc_consensus::{
	BlockCheckParams, BlockImport as BlockImportT, BlockImportParams, ImportResult,
};
use sc_network_sync::SyncingService;
use sc_service::{Configuration, TaskManager};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::Error as ConsensusError;

pub fn db_config_dir(config: &Configuration) -> PathBuf {
	config.base_path.config_dir(config.chain_spec.id())
}

/// The ethereum-compatibility configuration used to run a node.
#[derive(Clone, Debug, clap::Parser)]
pub struct EthConfiguration {
	/// Maximum number of logs in a query.
	#[arg(long, default_value = "10000")]
	pub max_past_logs: u32,

	/// Maximum fee history cache size.
	#[arg(long, default_value = "2048")]
	pub fee_history_limit: u64,

	#[arg(long)]
	pub enable_dev_signer: bool,

	/// The dynamic-fee pallet target gas price set by block author
	#[arg(long, default_value = "1")]
	pub target_gas_price: u64,

	/// Maximum allowed gas limit will be `block.gas_limit * execute_gas_limit_multiplier`
	/// when using eth_call/eth_estimateGas.
	#[arg(long, default_value = "10")]
	pub execute_gas_limit_multiplier: u64,

	/// Size in bytes of the LRU cache for block data.
	#[arg(long, default_value = "50")]
	pub eth_log_block_cache: usize,

	/// Size in bytes of the LRU cache for transactions statuses data.
	#[arg(long, default_value = "50")]
	pub eth_statuses_cache: usize,
}

type BlockNumberOf<B> = <<B as BlockT>::Header as HeaderT>::Number;

pub struct BlockImport<B: BlockT, I: BlockImportT<B>, C> {
	inner: I,
	client: Arc<C>,
	backend: Arc<fc_db::kv::Backend<B, C>>,
	evm_since: BlockNumberOf<B>,
	_marker: PhantomData<B>,
}

impl<Block: BlockT, I: Clone + BlockImportT<Block>, C> Clone for BlockImport<Block, I, C> {
	fn clone(&self) -> Self {
		BlockImport {
			inner: self.inner.clone(),
			client: self.client.clone(),
			backend: self.backend.clone(),
			evm_since: self.evm_since,
			_marker: PhantomData,
		}
	}
}

impl<B, I, C> BlockImport<B, I, C>
where
	B: BlockT,
	I: BlockImportT<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + BlockOf,
	C::Api: EthereumRuntimeRPCApi<B>,
	C::Api: BlockBuilderApi<B>,
{
	pub fn new(
		inner: I,
		client: Arc<C>,
		backend: Arc<fc_db::kv::Backend<B, C>>,
		evm_since: BlockNumberOf<B>,
	) -> Self {
		Self { inner, client, backend, evm_since, _marker: PhantomData }
	}
}

#[async_trait::async_trait]
impl<B, I, C> BlockImportT<B> for BlockImport<B, I, C>
where
	B: BlockT,
	<B::Header as HeaderT>::Number: PartialOrd,
	I: BlockImportT<B> + Send + Sync,
	I::Error: Into<ConsensusError>,
	C: ProvideRuntimeApi<B> + Send + Sync + HeaderBackend<B> + AuxStore + BlockOf,
	C::Api: EthereumRuntimeRPCApi<B>,
	C::Api: BlockBuilderApi<B>,
{
	type Error = ConsensusError;

	async fn check_block(
		&mut self,
		block: BlockCheckParams<B>,
	) -> Result<ImportResult, Self::Error> {
		self.inner.check_block(block).await.map_err(Into::into)
	}

	async fn import_block(
		&mut self,
		block: BlockImportParams<B>,
	) -> Result<ImportResult, Self::Error> {
		if *block.header.number() >= self.evm_since {
			ensure_log(block.header.digest()).map_err(Error::from)?;
		}
		self.inner.import_block(block).await.map_err(Into::into)
	}
}

impl<B: BlockT, I: BlockImportT<B>, C> ParachainBlockImportMarker for BlockImport<B, I, C> {}

pub fn spawn_frontier_tasks(
	task_manager: &TaskManager,
	client: Arc<FullClient>,
	backend: Arc<FullBackend>,
	frontier_backend: Arc<FrontierBackend<Block, FullClient>>,
	filter_pool: FilterPool,
	storage_overrides: Arc<dyn StorageOverride<Block>>,
	fee_history_cache: FeeHistoryCache,
	fee_history_cache_limit: FeeHistoryCacheLimit,
	sync: Arc<SyncingService<Block>>,
	pubsub_notification_sinks: Arc<
		fc_mapping_sync::EthereumBlockNotificationSinks<
			fc_mapping_sync::EthereumBlockNotification<Block>,
		>,
	>,
) {
	task_manager.spawn_essential_handle().spawn(
		"frontier-mapping-sync-worker",
		None,
		MappingSyncWorker::new(
			client.import_notification_stream(),
			Duration::new(6, 0),
			client.clone(),
			backend,
			storage_overrides.clone(),
			frontier_backend,
			3,
			0,
			SyncStrategy::Parachain,
			sync,
			pubsub_notification_sinks,
		)
		.for_each(|()| future::ready(())),
	);

	// Spawn Frontier EthFilterApi maintenance task.
	// Each filter is allowed to stay in the pool for 100 blocks.
	const FILTER_RETAIN_THRESHOLD: u64 = 100;
	task_manager.spawn_essential_handle().spawn(
		"frontier-filter-pool",
		None,
		EthTask::filter_pool_task(client.clone(), filter_pool, FILTER_RETAIN_THRESHOLD),
	);

	// Spawn Frontier FeeHistory cache maintenance task.
	task_manager.spawn_essential_handle().spawn(
		"frontier-fee-history",
		None,
		EthTask::fee_history_task(
			client,
			storage_overrides,
			fee_history_cache,
			fee_history_cache_limit,
		),
	);
}

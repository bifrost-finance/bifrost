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

use std::sync::Arc;

use jsonrpsee::RpcModule;
// Substrate
use bifrost_polkadot_runtime::opaque::Block;
use cumulus_primitives_core::PersistedValidationData;
use cumulus_primitives_parachain_inherent::ParachainInherentData;
use cumulus_test_relay_sproof_builder::RelayStateSproofBuilder;
use fc_db::kv::Backend as FrontierBackend;
pub use fc_rpc::{EthBlockDataCacheTask, StorageOverride};
pub use fc_rpc_core::types::{FeeHistoryCache, FeeHistoryCacheLimit, FilterPool};
use fp_rpc::{ConvertTransaction, ConvertTransactionRuntimeApi, EthereumRuntimeRPCApi};
use sc_client_api::{
	backend::{Backend, StorageProvider},
	client::BlockchainEvents,
	StateBackend,
};
use sc_network::service::traits::NetworkService;
use sc_network_sync::SyncingService;
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool::{ChainApi, Pool};
use sc_transaction_pool_api::TransactionPool;
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};

pub struct BifrostEthConfig<C, BE>(std::marker::PhantomData<(C, BE)>);

impl<C, BE> fc_rpc::EthConfig<Block, C> for BifrostEthConfig<C, BE>
where
	C: sc_client_api::StorageProvider<Block, BE> + Sync + Send + 'static,
	BE: Backend<Block> + 'static,
{
	type EstimateGasAdapter = ();
	type RuntimeStorageOverride =
		fc_rpc::frontier_backend_client::SystemAccountId20StorageOverride<Block, C, BE>;
}

/// Extra dependencies for Ethereum compatibility.
pub struct EthDeps<C, P, A: ChainApi, CT> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Graph pool instance.
	pub graph: Arc<Pool<A>>,
	/// Ethereum transaction converter.
	pub converter: Option<CT>,
	/// The Node authority flag
	pub is_authority: bool,
	/// Whether to enable dev signer
	pub enable_dev_signer: bool,
	/// Network service
	pub network: Arc<dyn NetworkService>,
	/// Chain syncing service
	pub sync_service: Arc<SyncingService<Block>>,
	/// Frontier Backend.
	pub frontier_backend: Arc<FrontierBackend<Block, C>>,
	/// Ethereum data access overrides.
	pub storage_override: Arc<dyn StorageOverride<Block>>,
	/// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
	/// EthFilterApi pool.
	pub filter_pool: FilterPool,
	/// Maximum number of logs in a query.
	pub max_past_logs: u32,
	/// Fee history cache.
	pub fee_history_cache: FeeHistoryCache,
	/// Maximum fee history cache size.
	pub fee_history_cache_limit: FeeHistoryCacheLimit,
	/// Maximum allowed gas limit will be ` block.gas_limit * execute_gas_limit_multiplier` when
	/// using eth_call/eth_estimateGas.
	pub execute_gas_limit_multiplier: u64,
}

/// Instantiate Ethereum-compatible RPC extensions.
pub fn create_eth<C, BE, P, A, CT>(
	mut io: RpcModule<()>,
	deps: EthDeps<C, P, A, CT>,
	subscription_task_executor: SubscriptionTaskExecutor,
	pubsub_notification_sinks: Arc<
		fc_mapping_sync::EthereumBlockNotificationSinks<
			fc_mapping_sync::EthereumBlockNotification<Block>,
		>,
	>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>,
	C::Api:
		BlockBuilderApi<Block> + EthereumRuntimeRPCApi<Block> + ConvertTransactionRuntimeApi<Block>,
	C: BlockchainEvents<Block> + 'static,
	C: HeaderBackend<Block>
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ StorageProvider<Block, BE>,
	C: CallApiAt<Block>,
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	P: TransactionPool<Block = Block> + 'static,
	A: ChainApi<Block = Block> + 'static,
	CT: ConvertTransaction<<Block as BlockT>::Extrinsic> + Send + Sync + 'static,
{
	use fc_rpc::{
		Debug, DebugApiServer, Eth, EthApiServer, EthDevSigner, EthFilter, EthFilterApiServer,
		EthPubSub, EthPubSubApiServer, EthSigner, Net, NetApiServer, TxPool, TxPoolApiServer, Web3,
		Web3ApiServer,
	};

	let EthDeps {
		client,
		pool,
		graph,
		converter,
		is_authority,
		enable_dev_signer,
		network,
		sync_service,
		frontier_backend,
		storage_override,
		block_data_cache,
		filter_pool,
		max_past_logs,
		fee_history_cache,
		fee_history_cache_limit,
		execute_gas_limit_multiplier,
	} = deps;

	let mut signers = Vec::new();
	if enable_dev_signer {
		signers.push(Box::new(EthDevSigner::new()) as Box<dyn EthSigner>);
	}

	let pending_create_inherent_data_providers = move |_, _| async move {
		let timestamp = sp_timestamp::InherentDataProvider::from_system_time();
		// Create a dummy parachain inherent data provider which is required to pass
		// the checks by the para chain system. We use dummy values because in the 'pending context'
		// neither do we have access to the real values nor do we need them.
		let (relay_parent_storage_root, relay_chain_state) =
			RelayStateSproofBuilder::default().into_state_root_and_proof();
		let vfp = PersistedValidationData {
			// This is a hack to make
			// `cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases` happy. Relay parent
			// number can't be bigger than u32::MAX.
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
		Ok((timestamp, parachain_inherent_data))
	};

	io.merge(
		Eth::<_, _, _, _, _, _, _, BifrostEthConfig<_, _>>::new(
			client.clone(),
			pool.clone(),
			graph.clone(),
			converter,
			sync_service.clone(),
			signers,
			storage_override.clone(),
			frontier_backend.clone(),
			is_authority,
			block_data_cache.clone(),
			fee_history_cache,
			fee_history_cache_limit,
			execute_gas_limit_multiplier,
			None,
			pending_create_inherent_data_providers,
			None,
		)
		.replace_config::<BifrostEthConfig<C, BE>>()
		.into_rpc(),
	)?;

	io.merge(
		EthFilter::new(
			client.clone(),
			frontier_backend.clone(),
			graph.clone(),
			filter_pool,
			500_usize, // max stored filters
			max_past_logs,
			block_data_cache.clone(),
		)
		.into_rpc(),
	)?;

	io.merge(
		EthPubSub::new(
			pool,
			client.clone(),
			sync_service,
			subscription_task_executor,
			storage_override.clone(),
			pubsub_notification_sinks,
		)
		.into_rpc(),
	)?;

	io.merge(
		Net::new(
			client.clone(),
			network,
			// Whether to format the `peer_count` response as Hex (default) or not.
			true,
		)
		.into_rpc(),
	)?;

	io.merge(Web3::new(client.clone()).into_rpc())?;

	io.merge(
		Debug::new(client.clone(), frontier_backend, storage_override, block_data_cache).into_rpc(),
	)?;

	io.merge(TxPool::new(client, graph).into_rpc())?;

	Ok(io)
}

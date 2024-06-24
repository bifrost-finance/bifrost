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

//! A collection of node-specific RPC methods.
//!
//! Since `substrate` core functionality makes no assumptions
//! about the modules used inside the runtime, so do
//! RPC methods defined in `sc-rpc` crate.
//! It means that `client/rpc` can't have any methods that
//! need some strong assumptions about the particular runtime.
//!
//! The RPCs available in this crate however can make some assumptions
//! about how the runtime is constructed and what FRAME pallets
//! are part of it. Therefore all node-runtime-specific RPCs can
//! be placed here or imported from corresponding FRAME RPC definitions.

#![warn(missing_docs)]

use std::sync::Arc;

use bifrost_farming_rpc::{FarmingRpc, FarmingRpcApiServer};
use bifrost_farming_rpc_runtime_api::FarmingRuntimeApi;
use bifrost_flexible_fee_rpc::{FeeRpcApiServer, FlexibleFeeRpc};
use bifrost_flexible_fee_rpc_runtime_api::FlexibleFeeRuntimeApi as FeeRuntimeApi;
use bifrost_kusama_runtime::{opaque::Block as BlockK, Hash};
use bifrost_primitives::{AccountId, Balance, Block, CurrencyId, Nonce, ParaId, PoolId};
use bifrost_salp_rpc::{SalpRpc, SalpRpcApiServer};
use bifrost_salp_rpc_runtime_api::SalpRuntimeApi;
use bifrost_stable_pool_rpc::{StablePoolRpc, StablePoolRpcApiServer};
use bifrost_stable_pool_rpc_runtime_api::StablePoolRuntimeApi;
use bifrost_ve_minting_rpc::{VeMintingRpc, VeMintingRpcApiServer};
use bifrost_ve_minting_rpc_runtime_api::VeMintingRuntimeApi;
use futures::channel::mpsc;
use lend_market_rpc::{LendMarket, LendMarketApiServer};
use lend_market_rpc_runtime_api::LendMarketApi;
use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
use sc_client_api::{
	backend::{Backend, StorageProvider},
	client::BlockchainEvents,
	AuxStore, UsageProvider,
};
use sc_consensus_manual_seal::rpc::{EngineCommand, ManualSeal, ManualSealApiServer};
use sc_rpc::SubscriptionTaskExecutor;
use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool::ChainApi;
use sc_transaction_pool_api::TransactionPool;
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::traits::{Block as BlockT, BlockIdTo};
use substrate_frame_rpc_system::{System, SystemApiServer};
use zenlink_protocol::AssetId;
use zenlink_protocol_rpc::{ZenlinkProtocol, ZenlinkProtocolApiServer};
use zenlink_protocol_runtime_api::ZenlinkProtocolApi as ZenlinkProtocolRuntimeApi;
use zenlink_stable_amm_rpc::{StableAmm, StableAmmApiServer};

mod eth;
pub use self::eth::{create_eth, overrides_handle, EthDeps};

/// Full client dependencies.
pub struct FullDeps<C, P, A: ChainApi, CT, CIDP> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// Ethereum-compatibility specific dependencies.
	pub eth: EthDeps<BlockK, C, P, A, CT, CIDP>,
	/// Manual seal command sink
	pub command_sink: Option<mpsc::Sender<EngineCommand<Hash>>>,
}

pub struct DefaultEthConfig<C, BE>(std::marker::PhantomData<(C, BE)>);

impl<C, BE> fc_rpc::EthConfig<BlockK, C> for DefaultEthConfig<C, BE>
where
	C: StorageProvider<BlockK, BE> + Sync + Send + 'static,
	BE: Backend<BlockK> + 'static,
{
	type EstimateGasAdapter = ();
	type RuntimeStorageOverride =
		fc_rpc::frontier_backend_client::SystemAccountId20StorageOverride<BlockK, C, BE>;
}

/// Full client dependencies.
pub struct FullDepsPolkadot<C, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
}

/// A IO handler that uses all Full RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// RPC of bifrost-kusama runtime.
pub fn create_full<C, P, BE, A, CT, CIDP>(
	deps: FullDeps<C, P, A, CT, CIDP>,
	subscription_task_executor: SubscriptionTaskExecutor,
	pubsub_notification_sinks: Arc<
		fc_mapping_sync::EthereumBlockNotificationSinks<
			fc_mapping_sync::EthereumBlockNotification<BlockK>,
		>,
	>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
	C: CallApiAt<BlockK>,
	C: ProvideRuntimeApi<BlockK>
		+ HeaderBackend<BlockK>
		+ HeaderMetadata<BlockK, Error = BlockChainError>
		+ Send
		+ Sync
		+ 'static,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<BlockK, Balance>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<BlockK, AccountId, Nonce>,
	C::Api: sp_consensus_aura::AuraApi<BlockK, AuraId>,
	C::Api: FarmingRuntimeApi<BlockK, AccountId, PoolId, CurrencyId>,
	C::Api: FeeRuntimeApi<BlockK, AccountId>,
	C::Api: SalpRuntimeApi<BlockK, ParaId, AccountId>,
	C::Api: StablePoolRuntimeApi<BlockK>,
	C::Api: LendMarketApi<BlockK, AccountId, Balance>,
	C::Api: ZenlinkProtocolRuntimeApi<BlockK, AccountId, AssetId>,
	C::Api: zenlink_stable_amm_runtime_api::StableAmmApi<
		BlockK,
		CurrencyId,
		Balance,
		AccountId,
		PoolId,
	>,
	C::Api: BlockBuilder<BlockK>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<BlockK>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<BlockK>,
	C: HeaderBackend<BlockK> + HeaderMetadata<BlockK, Error = BlockChainError> + 'static,
	C: BlockchainEvents<BlockK> + AuxStore + UsageProvider<BlockK> + StorageProvider<BlockK, BE>,
	BE: Backend<BlockK> + 'static,
	P: TransactionPool<Block = BlockK> + 'static,
	A: ChainApi<Block = BlockK> + 'static,
	CIDP: CreateInherentDataProviders<BlockK, ()> + Send + 'static,
	CT: fp_rpc::ConvertTransaction<<Block as BlockT>::Extrinsic> + Send + Sync + 'static,
{
	let mut module = RpcExtension::new(());
	let FullDeps { client, pool, deny_unsafe, eth, command_sink } = deps;

	module.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

	module.merge(FarmingRpc::new(client.clone()).into_rpc())?;
	module.merge(FlexibleFeeRpc::new(client.clone()).into_rpc())?;
	module.merge(SalpRpc::new(client.clone()).into_rpc())?;
	module.merge(ZenlinkProtocol::new(client.clone()).into_rpc())?;
	module.merge(StableAmm::new(client.clone()).into_rpc())?;
	module.merge(StablePoolRpc::new(client.clone()).into_rpc())?;
	module.merge(LendMarket::new(client).into_rpc())?;

	if let Some(command_sink) = command_sink {
		module.merge(
			// We provide the rpc handler with the sending end of the channel to allow the rpc
			// send EngineCommands to the background block authorship task.
			ManualSeal::new(command_sink).into_rpc(),
		)?;
	}

	// Ethereum compatibility RPCs
	let module = create_eth::<_, _, _, _, _, _, _, DefaultEthConfig<C, BE>>(
		module,
		eth,
		subscription_task_executor,
		pubsub_notification_sinks,
	)?;

	Ok(module)
}

/// RPC of bifrost-polkadot runtime.
pub fn create_full_polkadot<C, P>(
	deps: FullDepsPolkadot<C, P>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Send
		+ Sync
		+ 'static
		+ BlockIdTo<Block>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: FarmingRuntimeApi<Block, AccountId, PoolId, CurrencyId>,
	C::Api: FeeRuntimeApi<Block, AccountId>,
	C::Api: SalpRuntimeApi<Block, ParaId, AccountId>,
	C::Api: VeMintingRuntimeApi<Block, AccountId>,
	C::Api: LendMarketApi<Block, AccountId, Balance>,
	C::Api: ZenlinkProtocolRuntimeApi<Block, AccountId, AssetId>,
	C::Api: StablePoolRuntimeApi<Block>,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool + Sync + Send + 'static,
{
	let mut module = RpcExtension::new(());
	let FullDepsPolkadot { client, pool, deny_unsafe } = deps;

	module.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

	module.merge(FarmingRpc::new(client.clone()).into_rpc())?;
	module.merge(FlexibleFeeRpc::new(client.clone()).into_rpc())?;
	module.merge(SalpRpc::new(client.clone()).into_rpc())?;
	module.merge(VeMintingRpc::new(client.clone()).into_rpc())?;
	module.merge(ZenlinkProtocol::new(client.clone()).into_rpc())?;
	module.merge(StablePoolRpc::new(client.clone()).into_rpc())?;
	module.merge(LendMarket::new(client).into_rpc())?;

	Ok(module)
}

// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

use bifrost_farming_rpc_api::{FarmingRpc, FarmingRpcApiServer};
use bifrost_farming_rpc_runtime_api::FarmingRuntimeApi;
use bifrost_flexible_fee_rpc::{FeeRpcApiServer, FlexibleFeeRpc};
use bifrost_flexible_fee_rpc_runtime_api::FlexibleFeeRuntimeApi as FeeRuntimeApi;
use bifrost_liquidity_mining_rpc_api::{LiquidityMiningRpc, LiquidityMiningRpcApiServer};
use bifrost_liquidity_mining_rpc_runtime_api::LiquidityMiningRuntimeApi;
use bifrost_salp_rpc_api::{SalpRpc, SalpRpcApiServer};
use bifrost_salp_rpc_runtime_api::SalpRuntimeApi;
use node_primitives::{AccountId, Balance, Block, Nonce, ParaId, PoolId};
use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use substrate_frame_rpc_system::{System, SystemApiServer};
use zenlink_protocol_rpc::{ZenlinkProtocol, ZenlinkProtocolApiServer};
use zenlink_protocol_runtime_api::ZenlinkProtocolApi as ZenlinkProtocolRuntimeApi;

/// Full client dependencies.
pub struct FullDeps<C, P> {
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
pub fn create_full<C, P>(
	deps: FullDeps<C, P>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Send
		+ Sync
		+ 'static,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: FarmingRuntimeApi<Block, AccountId, PoolId>,
	C::Api: FeeRuntimeApi<Block, AccountId>,
	C::Api: SalpRuntimeApi<Block, ParaId, AccountId>,
	C::Api: LiquidityMiningRuntimeApi<Block, AccountId, PoolId>,
	C::Api: ZenlinkProtocolRuntimeApi<Block, AccountId>,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool + Sync + Send + 'static,
{
	let mut module = RpcExtension::new(());
	let FullDeps { client, pool, deny_unsafe } = deps;

	module.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

	module.merge(FarmingRpc::new(client.clone()).into_rpc())?;
	module.merge(FlexibleFeeRpc::new(client.clone()).into_rpc())?;
	module.merge(SalpRpc::new(client.clone()).into_rpc())?;
	module.merge(LiquidityMiningRpc::new(client.clone()).into_rpc())?;
	module.merge(ZenlinkProtocol::new(client).into_rpc())?;

	Ok(module)
}

/// RPC of bifrost-polkadot runtime.
pub fn create_full_polkadot<C, P>(
	deps: FullDeps<C, P>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Send
		+ Sync
		+ 'static,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: FarmingRuntimeApi<Block, AccountId, PoolId>,
	C::Api: FeeRuntimeApi<Block, AccountId>,
	C::Api: ZenlinkProtocolRuntimeApi<Block, AccountId>,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool + Sync + Send + 'static,
{
	let mut module = RpcExtension::new(());
	let FullDeps { client, pool, deny_unsafe } = deps;

	module.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

	module.merge(FarmingRpc::new(client.clone()).into_rpc())?;
	module.merge(FlexibleFeeRpc::new(client.clone()).into_rpc())?;
	module.merge(ZenlinkProtocol::new(client).into_rpc())?;

	Ok(module)
}

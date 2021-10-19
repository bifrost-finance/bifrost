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

use std::{marker::PhantomData, sync::Arc};

pub use bifrost_liquidity_mining_rpc_runtime_api::{
	self as runtime_api, LiquidityMiningRuntimeApi,
};
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use node_primitives::{Balance, CurrencyId};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{generic::BlockId, traits::Block as BlockT, SaturatedConversion};

#[rpc]
pub trait LiquidityMiningRpcApi<BlockHash, AccountId, PoolId> {
	/// rpc method for getting current rewards
	#[rpc(name = "liquidityMining_getRewards")]
	fn get_rewards(
		&self,
		who: AccountId,
		pid: PoolId,
		pallet_instance: u32,
		at: Option<BlockHash>,
	) -> JsonRpcResult<Vec<(CurrencyId, NumberOrHex)>>;
}

#[derive(Clone, Debug)]
pub struct LiquidityMiningRpcWrapper<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, Block> LiquidityMiningRpcWrapper<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

impl<C, Block, AccountId, PoolId> LiquidityMiningRpcApi<<Block as BlockT>::Hash, AccountId, PoolId>
	for LiquidityMiningRpcWrapper<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: LiquidityMiningRuntimeApi<Block, AccountId, PoolId>,
	AccountId: Codec,
	PoolId: Codec,
{
	fn get_rewards(
		&self,
		who: AccountId,
		pid: PoolId,
		pallet_instance: u32,
		at: Option<<Block as BlockT>::Hash>,
	) -> JsonRpcResult<Vec<(CurrencyId, NumberOrHex)>> {
		let lm_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let rs: Result<Vec<(CurrencyId, Balance)>, _> =
			lm_rpc_api.get_rewards(&at, who, pid, pallet_instance);

		match rs {
			Ok(rewards) => Ok(rewards
				.into_iter()
				.map(|(token, amount)| (token, NumberOrHex::Number(amount.saturated_into())))
				.collect()),
			Err(e) => Err(RpcError {
				code: ErrorCode::InternalError,
				message: "Failed to get lm rewards.".to_owned(),
				data: Some(format!("{:?}", e).into()),
			}),
		}
	}
}

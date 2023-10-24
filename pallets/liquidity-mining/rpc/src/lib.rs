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

use std::{marker::PhantomData, sync::Arc};

pub use bifrost_liquidity_mining_rpc_runtime_api::{
	self as runtime_api, LiquidityMiningRuntimeApi,
};
use codec::Codec;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorCode, ErrorObject},
};
use bifrost_primitives::{Balance, CurrencyId};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

#[rpc(client, server)]
pub trait LiquidityMiningRpcApi<BlockHash, AccountId, PoolId> {
	/// rpc method for getting current rewards
	#[method(name = "liquidityMining_getRewards")]
	fn get_rewards(
		&self,
		who: AccountId,
		pid: PoolId,
		pallet_instance: u32,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<(CurrencyId, NumberOrHex)>>;
}

#[derive(Clone, Debug)]
pub struct LiquidityMiningRpc<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, Block> LiquidityMiningRpc<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

#[async_trait]
impl<C, Block, AccountId, PoolId>
	LiquidityMiningRpcApiServer<<Block as BlockT>::Hash, AccountId, PoolId>
	for LiquidityMiningRpc<C, Block>
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
	) -> RpcResult<Vec<(CurrencyId, NumberOrHex)>> {
		let lm_rpc_api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		let rs: Result<Vec<(CurrencyId, Balance)>, _> =
			lm_rpc_api.get_rewards(at, who, pid, pallet_instance);

		match rs {
			Ok(rewards) => Ok(rewards
				.into_iter()
				.map(|(token, amount)| (token, NumberOrHex::Hex(amount.into())))
				.collect()),
			Err(e) => Err(CallError::Custom(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get lm rewards.",
				Some(format!("{:?}", e)),
			))),
		}
		.map_err(|e| jsonrpsee::core::Error::Call(e))
	}
}

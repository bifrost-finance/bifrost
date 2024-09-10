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

pub use bifrost_farming_rpc_runtime_api::{self as runtime_api, FarmingRuntimeApi};
use bifrost_primitives::Balance;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::error::{ErrorCode, ErrorObject},
};
use parity_scale_codec::Codec;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::traits::Block as BlockT;

#[rpc(client, server)]
pub trait FarmingRpcApi<BlockHash, AccountId, PoolId, CurrencyId> {
	/// rpc method for getting farming rewards
	#[method(name = "farming_getFarmingRewards")]
	fn get_farming_rewards(
		&self,
		who: AccountId,
		pid: PoolId,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<(CurrencyId, NumberOrHex)>>;

	/// rpc method for getting gauge rewards
	#[method(name = "farming_getGaugeRewards")]
	fn get_gauge_rewards(
		&self,
		who: AccountId,
		pid: PoolId,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<(CurrencyId, NumberOrHex)>>;
}

#[derive(Clone, Debug)]
pub struct FarmingRpc<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, Block> FarmingRpc<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

#[async_trait]
impl<C, Block, AccountId, PoolId, CurrencyId>
	FarmingRpcApiServer<<Block as BlockT>::Hash, AccountId, PoolId, CurrencyId>
	for FarmingRpc<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: FarmingRuntimeApi<Block, AccountId, PoolId, CurrencyId>,
	AccountId: Codec,
	PoolId: Codec,
	CurrencyId: Codec,
{
	fn get_farming_rewards(
		&self,
		who: AccountId,
		pid: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<(CurrencyId, NumberOrHex)>> {
		let lm_rpc_api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		let rs: Result<Vec<(CurrencyId, Balance)>, _> =
			lm_rpc_api.get_farming_rewards(at, who, pid);

		match rs {
			Ok(rewards) => Ok(rewards
				.into_iter()
				.map(|(token, amount)| (token, NumberOrHex::Hex(amount.into())))
				.collect()),
			Err(e) => Err(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get farming rewards.",
				Some(format!("{:?}", e)),
			)),
		}
	}

	fn get_gauge_rewards(
		&self,
		who: AccountId,
		pid: PoolId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<(CurrencyId, NumberOrHex)>> {
		let lm_rpc_api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		let rs: Result<Vec<(CurrencyId, Balance)>, _> = lm_rpc_api.get_gauge_rewards(at, who, pid);

		match rs {
			Ok(rewards) => Ok(rewards
				.into_iter()
				.map(|(token, amount)| (token, NumberOrHex::Hex(amount.into())))
				.collect()),
			Err(e) => Err(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get gauge rewards.",
				Some(format!("{:?}", e)),
			)),
		}
	}
}

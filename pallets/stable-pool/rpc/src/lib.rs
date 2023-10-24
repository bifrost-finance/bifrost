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

use bifrost_primitives::Balance;
pub use bifrost_stable_pool_rpc_runtime_api::{self as runtime_api, StablePoolRuntimeApi};
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorCode, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::traits::Block as BlockT;

#[rpc(client, server)]
pub trait StablePoolRpcApi<BlockHash> {
	/// rpc method for getting stable_pool swap output amount
	#[method(name = "stable_pool_getSwapOutputAmount")]
	fn get_swap_output_amount(
		&self,
		pool_id: u32,
		currency_id_in: u32,
		currency_id_out: u32,
		amount: Balance,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;

	#[method(name = "stable_pool_addLiquidityAmount")]
	fn add_liquidity_amount(
		&self,
		pool_id: u32,
		amounts: Vec<Balance>,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;
}

#[derive(Clone, Debug)]
pub struct StablePoolRpc<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, Block> StablePoolRpc<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

#[async_trait]
impl<C, Block> StablePoolRpcApiServer<<Block as BlockT>::Hash> for StablePoolRpc<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: StablePoolRuntimeApi<Block>,
{
	fn get_swap_output_amount(
		&self,
		pool_id: u32,
		currency_id_in: u32,
		currency_id_out: u32,
		amount: Balance,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let lm_rpc_api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		let rs: Result<Balance, _> =
			lm_rpc_api.get_swap_output(at, pool_id, currency_id_in, currency_id_out, amount);

		match rs {
			Ok(amount) => Ok(NumberOrHex::Hex(amount.into())),
			Err(e) => Err(CallError::Custom(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get stable_pool swap output amount.",
				Some(format!("{:?}", e)),
			))),
		}
		.map_err(|e| jsonrpsee::core::Error::Call(e))
	}

	fn add_liquidity_amount(
		&self,
		pool_id: u32,
		amounts: Vec<Balance>,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let lm_rpc_api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		let rs: Result<Balance, _> = lm_rpc_api.add_liquidity_amount(at, pool_id, amounts);

		match rs {
			Ok(amount) => Ok(NumberOrHex::Hex(amount.into())),
			Err(e) => Err(CallError::Custom(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get stable_pool add liquidity amount.",
				Some(format!("{:?}", e)),
			))),
		}
		.map_err(|e| jsonrpsee::core::Error::Call(e))
	}
}

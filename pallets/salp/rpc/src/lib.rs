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

use std::{marker::PhantomData, sync::Arc};

pub use bifrost_salp_rpc_runtime_api::{self as runtime_api, SalpRuntimeApi};
use codec::Codec;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorCode, ErrorObject},
};
use node_primitives::{Balance, RpcContributionStatus};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{generic::BlockId, sp_std::convert::TryInto, traits::Block as BlockT};

#[derive(Clone, Debug)]
pub struct SalpRpc<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, Block> SalpRpc<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

fn convert_rpc_params(value: Balance) -> RpcResult<NumberOrHex> {
	value
		.try_into()
		.map_err(|e| {
			CallError::Custom(ErrorObject::owned(
				ErrorCode::InvalidParams.code(),
				format!("{} doesn't fit in NumberOrHex representation", value),
				Some(format!("{:?}", e)),
			))
		})
		.map_err(|e| jsonrpsee::core::Error::Call(e))
}

#[rpc(client, server)]
pub trait SalpRpcApi<BlockHash, ParaId, AccountId> {
	/// rpc method for getting current contribution
	#[method(name = "salp_getContribution")]
	fn get_contribution(
		&self,
		index: ParaId,
		who: AccountId,
		at: Option<BlockHash>,
	) -> RpcResult<(NumberOrHex, RpcContributionStatus)>;

	#[method(name = "salp_getLiteContribution")]
	fn get_lite_contribution(
		&self,
		index: ParaId,
		who: AccountId,
		at: Option<BlockHash>,
	) -> RpcResult<(NumberOrHex, RpcContributionStatus)>;
}

#[async_trait]
impl<C, Block, ParaId, AccountId> SalpRpcApiServer<<Block as BlockT>::Hash, ParaId, AccountId>
	for SalpRpc<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: SalpRuntimeApi<Block, ParaId, AccountId>,
	ParaId: Codec,
	AccountId: Codec,
{
	fn get_contribution(
		&self,
		index: ParaId,
		account: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<(NumberOrHex, RpcContributionStatus)> {
		let salp_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let rs = salp_rpc_api.get_contribution(&at, index, account);

		match rs {
			Ok((val, status)) => match convert_rpc_params(val) {
				Ok(value) => Ok((value, status)),
				Err(e) => Err(e),
			},
			Err(e) => Err(CallError::Custom(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get salp contribution.",
				Some(format!("{:?}", e)),
			)))
			.map_err(|e| jsonrpsee::core::Error::Call(e)),
		}
	}

	fn get_lite_contribution(
		&self,
		index: ParaId,
		account: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<(NumberOrHex, RpcContributionStatus)> {
		let salp_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let rs = salp_rpc_api.get_lite_contribution(&at, index, account);

		match rs {
			Ok((val, status)) => match convert_rpc_params(val) {
				Ok(value) => Ok((value, status)),
				Err(e) => Err(e),
			},
			Err(e) => Err(CallError::Custom(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get salp contribution.",
				Some(format!("{:?}", e)),
			)))
			.map_err(|e| jsonrpsee::core::Error::Call(e)),
		}
	}
}

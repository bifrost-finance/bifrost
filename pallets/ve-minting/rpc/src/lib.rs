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

pub use bifrost_ve_minting_rpc_runtime_api::{self as runtime_api, VeMintingRuntimeApi};
use codec::Codec;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorCode, ErrorObject},
};
use node_primitives::{Balance, CurrencyId};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

#[rpc(client, server)]
pub trait VeMintingRpcApi<BlockHash, AccountId> {
	/// rpc method for getting user balance
	#[method(name = "ve_minting_balanceOf")]
	fn balance_of(
		&self,
		who: AccountId,
		t: Option<Timestamp>,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;

	#[method(name = "ve_minting_total_supply")]
	fn total_supply(&self, t: Timestamp, at: Option<BlockHash>) -> RpcResult<NumberOrHex>;

	#[method(name = "ve_minting_findBlockEpoch")]
	fn find_block_epoch(
		&self,
		block: BlockNumber,
		max_epoch: U256,
		at: Option<BlockHash>,
	) -> RpcResult<NumberOrHex>;
}

#[derive(Clone, Debug)]
pub struct VeMintingRpc<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, Block> VeMintingRpc<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

#[async_trait]
impl<C, Block, AccountId> VeMintingRpcApiServer<<Block as BlockT>::Hash, AccountId>
	for VeMintingRpc<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: VeMintingRuntimeApi<Block, AccountId, PoolId>,
	AccountId: Codec,
{
	fn balance_of(
		&self,
		who: AccountId,
		t: Option<Timestamp>,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let lm_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let rs: Result<Balance, _> = lm_rpc_api.balance_of(&at, who, t);

		match rs {
			Ok(rewards) =>
				Ok(rewards.into_iter().map(|amount| NumberOrHex::Hex(amount.into())).collect()),
			Err(e) => Err(CallError::Custom(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get balance_of.",
				Some(format!("{:?}", e)),
			))),
		}
		.map_err(|e| jsonrpsee::core::Error::Call(e))
	}

	fn total_supply(
		&self,
		t: Timestamp,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let lm_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let rs: Result<Balance, _> = lm_rpc_api.total_supply(&at, t);

		match rs {
			Ok(rewards) =>
				Ok(rewards.into_iter().map(|amount| NumberOrHex::Hex(amount.into())).collect()),
			Err(e) => Err(CallError::Custom(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get total_supply.",
				Some(format!("{:?}", e)),
			))),
		}
		.map_err(|e| jsonrpsee::core::Error::Call(e))
	}

	fn find_block_epoch(
		&self,
		block: BlockNumber,
		max_epoch: U256,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let lm_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let rs: Result<U256, _> = lm_rpc_api.find_block_epoch(&at, block, max_epoch);

		match rs {
			Ok(rewards) =>
				Ok(rewards.into_iter().map(|amount| NumberOrHex::Hex(amount.into())).collect()),
			Err(e) => Err(CallError::Custom(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get find_block_epoch.",
				Some(format!("{:?}", e)),
			))),
		}
		.map_err(|e| jsonrpsee::core::Error::Call(e))
	}
}

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

pub use bb_bnc_rpc_runtime_api::{self as runtime_api, BbBNCRuntimeApi};
use bifrost_primitives::Balance;
use jsonrpsee::{
	core::{async_trait, RpcResult},
	proc_macros::rpc,
	types::error::{ErrorCode, ErrorObject},
};
use parity_scale_codec::Codec;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::U256;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, BlockIdTo},
	SaturatedConversion,
};

#[rpc(client, server)]
pub trait BbBNCRpcApi<BlockHash, AccountId> {
	/// rpc method for getting user balance
	#[method(name = "bb_bnc_balanceOf")]
	fn balance_of(&self, who: AccountId, at: Option<BlockHash>) -> RpcResult<NumberOrHex>;

	/// RPC method to get total supply
	#[method(name = "bb_bnc_totalSupply")]
	fn total_supply(&self, at: Option<BlockHash>) -> RpcResult<NumberOrHex>;

	/// RPC method to find block epoch
	#[method(name = "bb_bnc_findBlockEpoch")]
	fn find_block_epoch(&self, max_epoch: U256, at: Option<BlockHash>) -> RpcResult<NumberOrHex>;
}

#[derive(Clone, Debug)]
pub struct BbBNCRpc<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, Block> BbBNCRpc<C, Block>
where
	Block: BlockT,
	C: BlockIdTo<Block>,
{
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

#[async_trait]
impl<C, Block, AccountId> BbBNCRpcApiServer<<Block as BlockT>::Hash, AccountId>
	for BbBNCRpc<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block> + BlockIdTo<Block>,
	C::Api: BbBNCRuntimeApi<Block, AccountId>,
	AccountId: Codec,
	// CallError: From<<C as BlockIdTo<Block>>::Error>,
{
	/// Get the balance of the user at a specific block hash
	fn balance_of(
		&self,
		who: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let lm_rpc_api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		let block_number = self
			.client
			.to_number(&BlockId::Hash(at))
			.map(|num| match num {
				Some(inner_num) => Some(inner_num.saturated_into::<u32>()),
				None => None,
			})
			.map_err(|e| {
				ErrorObject::owned(
					ErrorCode::InternalError.code(),
					"Failed to get balance_of.",
					Some(format!("{:?}", e)),
				)
			})?;

		let rs: Result<Balance, _> = lm_rpc_api.balance_of(at, who, block_number);

		match rs {
			Ok(balane) => Ok(NumberOrHex::Hex(balane.into())),
			Err(e) => Err(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get balance_of.",
				Some(format!("{:?}", e)),
			)),
		}
	}

	/// Get the total supply at a specific block hash
	fn total_supply(&self, at: Option<<Block as BlockT>::Hash>) -> RpcResult<NumberOrHex> {
		let lm_rpc_api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		let block_number = self
			.client
			.to_number(&BlockId::Hash(at))
			.map(|num| match num {
				Some(inner_num) => Some(inner_num.saturated_into::<u32>()),
				None => None,
			})
			.map_err(|e| {
				ErrorObject::owned(
					ErrorCode::InternalError.code(),
					"Failed to get total_supply.",
					Some(format!("{:?}", e)),
				)
			})?;
		let rs: Result<Balance, _> =
			lm_rpc_api.total_supply(at, block_number.expect("no block found"));

		match rs {
			Ok(supply) => Ok(NumberOrHex::Hex(supply.into())),
			Err(e) => Err(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get total_supply.",
				Some(format!("{:?}", e)),
			)),
		}
	}

	/// Find the block epoch given a max epoch and block hash
	fn find_block_epoch(
		&self,
		max_epoch: U256,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<NumberOrHex> {
		let lm_rpc_api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);
		let block_number = self
			.client
			.to_number(&BlockId::Hash(at))
			.map(|num| match num {
				Some(inner_num) => Some(inner_num.saturated_into::<u32>()),
				None => None,
			})
			.map_err(|e| {
				ErrorObject::owned(
					ErrorCode::InternalError.code(),
					"Failed to get find_block_epoch.",
					Some(format!("{:?}", e)),
				)
			})?;
		let rs: Result<U256, _> =
			lm_rpc_api.find_block_epoch(at, block_number.expect("no block found"), max_epoch);

		match rs {
			Ok(epoch) => Ok(NumberOrHex::Hex(epoch.into())),
			Err(e) => Err(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get find_block_epoch.",
				Some(format!("{:?}", e)),
			)),
		}
	}
}

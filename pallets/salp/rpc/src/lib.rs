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

pub use bifrost_salp_rpc_runtime_api::{self as runtime_api, SalpRuntimeApi};
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use node_primitives::RpcContributionStatus;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{generic::BlockId, traits::Block as BlockT, SaturatedConversion};

pub use self::gen_client::Client as SalpClient;

#[derive(Clone, Debug)]
pub struct SalpRpcWrapper<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, Block> SalpRpcWrapper<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

#[rpc]
pub trait SalpRpcApi<BlockHash, ParaId, AccountId> {
	/// rpc method for getting current contribution
	#[rpc(name = "salp_getContribution")]
	fn get_contribution(
		&self,
		index: ParaId,
		who: AccountId,
		at: Option<BlockHash>,
	) -> JsonRpcResult<(NumberOrHex, RpcContributionStatus)>;
}

impl<C, Block, ParaId, AccountId> SalpRpcApi<<Block as BlockT>::Hash, ParaId, AccountId>
	for SalpRpcWrapper<C, Block>
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
	) -> JsonRpcResult<(NumberOrHex, RpcContributionStatus)> {
		let salp_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let rs = salp_rpc_api.get_contribution(&at, index, account);

		match rs {
			Ok((val, status)) => Ok((NumberOrHex::Number(val.saturated_into()), status)),
			Err(e) => Err(RpcError {
				code: ErrorCode::InternalError,
				message: "Failed to get salp contribution.".to_owned(),
				data: Some(format!("{:?}", e).into()),
			}),
		}
	}
}

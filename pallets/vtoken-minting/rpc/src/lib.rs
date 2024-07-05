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

pub use bifrost_vtoken_minting_rpc_runtime_api::VtokenMintingRuntimeApi;
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
use sp_runtime::traits::Block as BlockT;

#[rpc(client, server)]
pub trait VtokenMintingRpcApi<CurrencyId, BlockHash> {
	/// rpc method for getting vtoken exchange rate
	#[method(name = "vtoken_minting_getExchangeRate")]
	fn get_exchange_rate(
		&self,
		asset_id: Option<CurrencyId>,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<(CurrencyId, NumberOrHex)>>;
}

#[derive(Clone, Debug)]
pub struct VtokenMintingRpc<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, Block> VtokenMintingRpc<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

#[async_trait]
impl<C, Block, CurrencyId> VtokenMintingRpcApiServer<CurrencyId, <Block as BlockT>::Hash>
	for VtokenMintingRpc<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: VtokenMintingRuntimeApi<Block, CurrencyId>,
	CurrencyId: Codec,
{
	fn get_exchange_rate(
		&self,
		token_id: Option<CurrencyId>,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<(CurrencyId, NumberOrHex)>> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		let rs: Result<Vec<(CurrencyId, U256)>, _> = api.get_exchange_rate(at, token_id);

		match rs {
			Ok(data) => Ok(data
				.into_iter()
				.map(|(token, rate)| (token, NumberOrHex::Hex(rate.into())))
				.collect()),
			Err(e) => Err(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get find_block_epoch.",
				Some(format!("{:?}", e)),
			)),
		}
	}
}

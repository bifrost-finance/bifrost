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
use sp_runtime::traits::Block as BlockT;

#[rpc(client, server)]
pub trait VtokenMintingRpcApi<CurrencyId, Balance, BlockHash> {
	/// rpc method for getting vtoken exchange rate
	#[method(name = "vtoken_minting_get_v_currency_amount_by_currency_amount")]
	fn get_v_currency_amount_by_currency_amount(
		&self,
		currency_id: CurrencyId,
		v_currency_id: CurrencyId,
		currency_amount: Balance,
		at: Option<BlockHash>,
	) -> RpcResult<Balance>;

	/// rpc method for getting vtoken exchange rate
	#[method(name = "vtoken_minting_get_currency_amount_by_v_currency_amount")]
	fn get_currency_amount_by_v_currency_amount(
		&self,
		currency_id: CurrencyId,
		v_currency_id: CurrencyId,
		v_currency_amount: Balance,
		at: Option<BlockHash>,
	) -> RpcResult<Balance>;
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
impl<C, Block, CurrencyId, Balance>
	VtokenMintingRpcApiServer<CurrencyId, Balance, <Block as BlockT>::Hash>
	for VtokenMintingRpc<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: VtokenMintingRuntimeApi<Block, CurrencyId, Balance>,
	CurrencyId: Codec,
	Balance: Codec,
{
	fn get_v_currency_amount_by_currency_amount(
		&self,
		currency_id: CurrencyId,
		v_currency_id: CurrencyId,
		currency_amount: Balance,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Balance> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		let rs: Result<Balance, _> = api.get_v_currency_amount_by_currency_amount(
			at,
			currency_id,
			v_currency_id,
			currency_amount,
		);

		match rs {
			Ok(data) => Ok(data),
			Err(e) => Err(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get find_block_epoch.",
				Some(format!("{:?}", e)),
			)),
		}
	}

	fn get_currency_amount_by_v_currency_amount(
		&self,
		currency_id: CurrencyId,
		v_currency_id: CurrencyId,
		v_currency_amount: Balance,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Balance> {
		let api = self.client.runtime_api();
		let at = at.unwrap_or_else(|| self.client.info().best_hash);

		let rs: Result<Balance, _> = api.get_currency_amount_by_v_currency_amount(
			at,
			currency_id,
			v_currency_id,
			v_currency_amount,
		);

		match rs {
			Ok(data) => Ok(data),
			Err(e) => Err(ErrorObject::owned(
				ErrorCode::InternalError.code(),
				"Failed to get find_block_epoch.",
				Some(format!("{:?}", e)),
			)),
		}
	}
}

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

use codec::{Codec, Decode};
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use node_primitives::{Balance, CurrencyId};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::Bytes;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, Zero},
	SaturatedConversion,
};

#[derive(Clone, Debug)]
pub struct BancorStruct<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>,
}

impl<C, Block> BancorStruct<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: PhantomData }
	}
}

#[rpc]
pub trait BancorRpcApi<BlockHash, CurrencyId, Balance> {
	#[rpc(name = "bancor_getBancorTokenAmountOut")]
	fn get_bancor_token_amount_out(
		&self,
		token_id: CurrencyId,
		vstoken_amount: Balance,
		at: Option<BlockHash>,
	) -> JsonRpcResult<Balance>;

	#[rpc(name = "bancor_getBancorVstokenAmountOut")]
	fn get_bancor_vstoken_amount_out(
		&self,
		token_id: CurrencyId,
		token_amount: Balance,
		at: Option<BlockHash>,
	) -> JsonRpcResult<Balance>;

	#[rpc(name = "bancor_getInstantVstokenPrice")]
	fn get_instant_vstoken_price(
		&self,
		currency_id: CurrencyId,
		at: Option<BlockHash>,
	) -> JsonRpcResult<Balance>;

	#[rpc(name = "bancor_getInstantTokenPrice")]
	fn get_instant_token_price(
		&self,
		currency_id: CurrencyId,
		at: Option<BlockHash>,
	) -> JsonRpcResult<Balance>;
}

/// Error type of this RPC api.
pub enum Error {
	/// The transaction was not decodable.
	DecodeError,
	/// The call to runtime failed.
	RuntimeError,
}

impl From<Error> for i64 {
	fn from(e: Error) -> i64 {
		match e {
			Error::RuntimeError => 1,
			Error::DecodeError => 2,
		}
	}
}

impl<C, Block, CurrencyId, Balance> BancorRpcApi<<Block as BlockT>::Hash, CurrencyId, Balance>
	for BancorStruct<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: BancorRuntimeApi<Block, CurrencyId, Balance>,
	CurrencyId: Codec,
	Balance: Codec + std::fmt::Display + std::ops::Add<Output = Balance> + sp_runtime::traits::Zero,
{
	fn get_bancor_token_amount_out(
		&self,
		token_id: CurrencyId,
		vstoken_amount: Balance,
		at: Option<BlockHash>,
	) -> JsonRpcResult<Balance> {
		let api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_bancor_token_amount_out(&at, token_id, vstoken_amount)
			.map_err(|e| RpcError {
				code: ErrorCode::InternalError,
				message: "Failed to get bancor token amount out.".to_owned(),
				data: Some(format!("{:?}", e).into()),
			})
	}

	fn get_bancor_vstoken_amount_out(
		&self,
		token_id: CurrencyId,
		token_amount: Balance,
		at: Option<BlockHash>,
	) -> JsonRpcResult<Balance> {
		let api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_bancor_vstoken_amount_out(&at, token_id, token_amount)
			.map_err(|e| RpcError {
				code: ErrorCode::InternalError,
				message: "Failed to get bancor vstoken amount out.".to_owned(),
				data: Some(format!("{:?}", e).into()),
			})
	}

	fn get_instant_vstoken_price(
		&self,
		currency_id: CurrencyId,
		at: Option<BlockHash>,
	) -> JsonRpcResult<Balance> {
		let api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_instant_vstoken_price(&at, currency_id).map_err(|e| RpcError {
			code: ErrorCode::InternalError,
			message: "Failed to get bancor instant vstoken price.".to_owned(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn get_instant_token_price(
		&self,
		currency_id: CurrencyId,
		at: Option<BlockHash>,
	) -> JsonRpcResult<Balance> {
		let api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_instant_token_price(&at, currency_id).map_err(|e| RpcError {
			code: ErrorCode::InternalError,
			message: "Failed to get bancor instant token price.".to_owned(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}

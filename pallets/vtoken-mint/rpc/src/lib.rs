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

use codec::Codec;
use jsonrpc_derive::rpc;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use std::sync::Arc;
use std::marker::PhantomData;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
pub use self::gen_client::Client as VtokenMintClient;
pub use vtoken_mint_rpc_runtime_api::{self as runtime_api, VtokenMintPriceApi as VtokenMintRateRuntimeApi};

#[derive(Clone, Debug)]
pub struct VtokenMint<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>
}

impl<C, Block> VtokenMint<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: PhantomData
		}
	}
}

#[rpc]
pub trait VtokenMintPriceApi<BlockHash, AssetId, VtokenMintPrice> {
	/// rpc method for getting current vtoken mint rate
	#[rpc(name = "vtokenmint_getVtokenMintRate")]
	fn get_vtoken_mint_rate(&self, asset_id: AssetId, at: Option<BlockHash>) -> JsonRpcResult<VtokenMintPrice>;
}

impl<C, Block, AssetId, VtokenMintPrice> VtokenMintPriceApi<<Block as BlockT>::Hash, AssetId, VtokenMintPrice>
for VtokenMint<C, Block>
	where
		Block: BlockT,
		C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
		C::Api: VtokenMintRateRuntimeApi<Block, AssetId, VtokenMintPrice>,
		AssetId: Codec,
		VtokenMintPrice: Codec,
{
	fn get_vtoken_mint_rate(&self, asset_id: AssetId, at: Option<<Block as BlockT>::Hash>) -> JsonRpcResult<VtokenMintPrice> {
		let vtoken_mint_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		vtoken_mint_rpc_api.get_vtoken_mint_rate(&at, asset_id).map_err(|e| RpcError {
			code: ErrorCode::InternalError,
			message: "Failed to get current vtoken mint rate.".to_owned(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}

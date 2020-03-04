// Copyright 2019-2020 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

use codec::Codec;
use jsonrpc_derive::rpc;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use std::sync::Arc;
use std::marker::PhantomData;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
pub use self::gen_client::Client as ExchangeClient;
pub use exchange_rpc_runtime_api::{self as runtime_api, ExchangeRateApi as ExchangeRateRuntimeApi};

#[derive(Clone, Debug)]
pub struct Exchange<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>
}

impl<C, Block> Exchange<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: PhantomData
		}
	}
}

#[rpc]
pub trait ExchangeRateApi<BlockHash, AssetId, ExchangeRate> {
	/// rpc method for getting current exchange rate
	#[rpc(name = "exchange_getExchange")]
	fn get_exchange_rate(&self, vtoken_id: AssetId, at: Option<BlockHash>) -> JsonRpcResult<ExchangeRate>;
}

impl<C, Block, AssetId, ExchangeRate> ExchangeRateApi<<Block as BlockT>::Hash, AssetId, ExchangeRate>
for Exchange<C, Block>
	where
		Block: BlockT,
		C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
		C::Api: ExchangeRateRuntimeApi<Block, AssetId, ExchangeRate>,
		AssetId: Codec,
		ExchangeRate: Codec,
{
	fn get_exchange_rate(&self, vtoken_id: AssetId, at: Option<<Block as BlockT>::Hash>) -> JsonRpcResult<ExchangeRate> {
		let exchange_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		exchange_rpc_api.get_exchange_rate(&at, vtoken_id).map_err(|e| RpcError {
			code: ErrorCode::InternalError,
			message: "Failed to get current exchange rate.".to_owned(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}

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
pub use self::gen_client::Client as AssetsClient;
pub use assets_rpc_runtime_api::{self as runtime_api, AssetsApi as AssetsRuntimeApi};

#[derive(Clone, Debug)]
pub struct Assets<C, Block> {
	client: Arc<C>,
	_marker: PhantomData<Block>
}

impl<C, Block> Assets<C, Block> {
	pub fn new(client: Arc<C>) -> Self {
		Self {
			client,
			_marker: PhantomData
		}
	}
}

#[rpc]
pub trait AssetsApi<BlockHash, TokenSymbol, AccountId, Balance> {
	/// rpc method get balances by account id
	/// useage: curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "assets_getBalances", "params": [0, "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"]}' http://localhost:9933/
	#[rpc(name = "assets_getBalances")]
	fn asset_balances(
		&self,
		token_symbol: TokenSymbol,
		who: AccountId,
		at: Option<BlockHash>
	) -> JsonRpcResult<u64>;

	/// rpc method get tokens by account id
	/// useage: curl -H "Content-Type: application/json" -d '{"id":1, "jsonrpc":"2.0", "method": "assets_getTokens", "params": ["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"]}' http://localhost:9933/
	#[rpc(name = "assets_getTokens")]
	fn asset_tokens(
		&self,
		who: AccountId,
		at: Option<BlockHash>
	) -> JsonRpcResult<Vec<TokenSymbol>>;
}

impl<C, Block, TokenSymbol, AccountId, Balance> AssetsApi<<Block as BlockT>::Hash, TokenSymbol, AccountId, Balance>
	for Assets<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: AssetsRuntimeApi<Block, TokenSymbol, AccountId, Balance>,
	AccountId: Codec,
	TokenSymbol: Codec,
	Balance: Codec,
{
	fn asset_balances(&self, token_symbol: TokenSymbol, who: AccountId, at: Option<<Block as BlockT>::Hash>) -> JsonRpcResult<u64> {
		let asset_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		asset_rpc_api.asset_balances(&at, token_symbol, who).map_err(|e| RpcError {
			code: ErrorCode::InternalError,
			message: "Failed to get balance for you requested asset id.".to_owned(),
			data: Some(format!("{:?}", e).into()),
		})
	}

	fn asset_tokens(&self, who: AccountId, at: Option<<Block as BlockT>::Hash>) -> JsonRpcResult<Vec<TokenSymbol>> {
		let asset_rpc_api = self.client.runtime_api();
		let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		asset_rpc_api.asset_tokens(&at, who).map_err(|e| RpcError {
			code: ErrorCode::InternalError,
			message: "Failed to get balance for you requested account.".to_owned(),
			data: Some(format!("{:?}", e).into()),
		})
	}
}

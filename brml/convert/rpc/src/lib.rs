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

pub use self::gen_client::Client as ConvertClient;
use codec::Codec;
pub use convert_rpc_runtime_api::{self as runtime_api, ConvertPriceApi as ConvertRateRuntimeApi};
use jsonrpc_core::{Error as RpcError, ErrorCode, Result as JsonRpcResult};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Convert<C, Block> {
    client: Arc<C>,
    _marker: PhantomData<Block>,
}

impl<C, Block> Convert<C, Block> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: PhantomData,
        }
    }
}

#[rpc]
pub trait ConvertPriceApi<BlockHash, TokenSymbol, ConvertPrice> {
    /// rpc method for getting current convert rate
    #[rpc(name = "convert_getConvert")]
    fn get_convert_rate(
        &self,
        token_symbol: TokenSymbol,
        at: Option<BlockHash>,
    ) -> JsonRpcResult<ConvertPrice>;
}

impl<C, Block, TokenSymbol, ConvertPrice>
    ConvertPriceApi<<Block as BlockT>::Hash, TokenSymbol, ConvertPrice> for Convert<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: ConvertRateRuntimeApi<Block, TokenSymbol, ConvertPrice>,
    TokenSymbol: Codec,
    ConvertPrice: Codec,
{
    fn get_convert_rate(
        &self,
        token_symbol: TokenSymbol,
        at: Option<<Block as BlockT>::Hash>,
    ) -> JsonRpcResult<ConvertPrice> {
        let convert_rpc_api = self.client.runtime_api();
        let at = BlockId::<Block>::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        convert_rpc_api
            .get_convert_rate(&at, token_symbol)
            .map_err(|e| RpcError {
                code: ErrorCode::InternalError,
                message: "Failed to get current convert rate.".to_owned(),
                data: Some(format!("{:?}", e).into()),
            })
    }
}

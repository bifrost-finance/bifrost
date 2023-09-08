// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;

pub use pallet_lend_market_rpc_runtime_api::LoansApi as LoansRuntimeApi;

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorCode, ErrorObject},
};
use node_primitives::{CurrencyId, Liquidity, Rate, Ratio, Shortfall};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{generic::BlockId, traits::Block as BlockT, FixedU128};

#[rpc(client, server)]
pub trait LoansApi<BlockHash, AccountId, Balance>
where
    Balance: Codec + Copy + TryFrom<NumberOrHex>,
{
    #[method(name = "lend_market_getCollateralLiquidity")]
    fn get_account_liquidity(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> RpcResult<(Liquidity, Shortfall, Liquidity, Shortfall)>;
    #[method(name = "lend_market_getMarketStatus")]
    fn get_market_status(
        &self,
        asset_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> RpcResult<(Rate, Rate, Rate, Ratio, NumberOrHex, NumberOrHex, FixedU128)>;
    #[method(name = "lend_market_getLiquidationThresholdLiquidity")]
    fn get_liquidation_threshold_liquidity(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> RpcResult<(Liquidity, Shortfall, Liquidity, Shortfall)>;
}

/// A struct that implements the [`LoansApi`].
pub struct Loans<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Loans<C, B> {
    /// Create new `Loans` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

pub enum Error {
    RuntimeError,
    AccountLiquidityError,
    MarketStatusError,
}

impl From<Error> for i32 {
    fn from(e: Error) -> i32 {
        match e {
            Error::RuntimeError => 1,
            Error::AccountLiquidityError => 2,
            Error::MarketStatusError => 3,
        }
    }
}

#[async_trait]
impl<C, Block, AccountId, Balance> LoansApiServer<<Block as BlockT>::Hash, AccountId, Balance>
    for Loans<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static,
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block>,
    C::Api: LoansRuntimeApi<Block, AccountId, Balance>,
    AccountId: Codec,
    Balance: Codec + Copy + TryFrom<NumberOrHex> + Into<NumberOrHex> + std::fmt::Display,
{
    fn get_account_liquidity(
        &self,
        account: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<(Liquidity, Shortfall, Liquidity, Shortfall)> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or(
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash,
        ));
        api.get_account_liquidity(&at, account)
            .map_err(runtime_error_into_rpc_error)?
            .map_err(account_liquidity_error_into_rpc_error)
    }

    fn get_market_status(
        &self,
        asset_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<(Rate, Rate, Rate, Ratio, NumberOrHex, NumberOrHex, FixedU128)> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or(
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash,
        ));
        let (
            borrow_rate,
            supply_rate,
            exchange_rate,
            util,
            total_borrows,
            total_reserves,
            borrow_index,
        ) = api
            .get_market_status(&at, asset_id)
            .map_err(runtime_error_into_rpc_error)?
            .map_err(market_status_error_into_rpc_error)?;
        Ok((
            borrow_rate,
            supply_rate,
            exchange_rate,
            util,
            try_into_rpc_balance(total_borrows)?,
            try_into_rpc_balance(total_reserves)?,
            borrow_index,
        ))
    }

    fn get_liquidation_threshold_liquidity(
        &self,
        account: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<(Liquidity, Shortfall, Liquidity, Shortfall)> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or(
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash,
        ));
        api.get_liquidation_threshold_liquidity(&at, account)
            .map_err(runtime_error_into_rpc_error)?
            .map_err(account_liquidity_error_into_rpc_error)
    }
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_error(err: impl std::fmt::Debug) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        Error::RuntimeError.into(),
        "Runtime trapped",
        Some(format!("{:?}", err)),
    )))
}

/// Converts an account liquidity error into an RPC error.
fn account_liquidity_error_into_rpc_error(err: impl std::fmt::Debug) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        Error::AccountLiquidityError.into(),
        "Not able to get account liquidity",
        Some(format!("{:?}", err)),
    )))
}

/// Converts an market status error into an RPC error.
fn market_status_error_into_rpc_error(err: impl std::fmt::Debug) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        Error::MarketStatusError.into(),
        "Not able to get market status",
        Some(format!("{:?}", err)),
    )))
}

fn try_into_rpc_balance<T: std::fmt::Display + Copy + TryInto<NumberOrHex>>(
    value: T,
) -> RpcResult<NumberOrHex> {
    value.try_into().map_err(|_| {
        JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
            ErrorCode::InvalidParams.code(),
            format!("{} doesn't fit in NumberOrHex representation", value),
            None::<()>,
        )))
    })
}

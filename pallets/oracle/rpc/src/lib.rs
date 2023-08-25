//! RPC interface for the Oracle.

use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error as JsonRpseeError, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorCode, ErrorObject},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
    DispatchError,
};
use std::sync::Arc;

pub use oracle_rpc_runtime_api::{BalanceWrapper, OracleApi as OracleRuntimeApi};

#[rpc(client, server)]
pub trait OracleApi<BlockHash, Balance, CurrencyId>
where
    Balance: Codec + MaybeDisplay + MaybeFromStr,
    CurrencyId: Codec,
{
    #[method(name = "oracle_wrappedToCollateral")]
    fn wrapped_to_collateral(
        &self,
        amount: BalanceWrapper<Balance>,
        currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    #[method(name = "oracle_collateralToWrapped")]
    fn collateral_to_wrapped(
        &self,
        amount: BalanceWrapper<Balance>,
        currency_id: CurrencyId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;
}

fn internal_err<T: ToString>(message: T) -> JsonRpseeError {
    JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
        ErrorCode::InternalError.code(),
        message.to_string(),
        None::<()>,
    )))
}

/// A struct that implements the [`OracleApi`].
pub struct Oracle<C, B> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<B>,
}

impl<C, B> Oracle<C, B> {
    /// Create new `Oracle` with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Oracle {
            client,
            _marker: Default::default(),
        }
    }
}

fn handle_response<T, E: std::fmt::Debug>(result: Result<Result<T, DispatchError>, E>) -> RpcResult<T> {
    result
        .map_err(|err| internal_err(format!("Runtime error: {:?}", err)))?
        .map_err(|err| internal_err(format!("Execution error: {:?}", err)))
}

#[async_trait]
impl<C, Block, Balance, CurrencyId> OracleApiServer<<Block as BlockT>::Hash, Balance, CurrencyId> for Oracle<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: OracleRuntimeApi<Block, Balance, CurrencyId>,
    Balance: Codec + MaybeDisplay + MaybeFromStr,
    CurrencyId: Codec,
{
    fn wrapped_to_collateral(
        &self,
        amount: BalanceWrapper<Balance>,
        currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        handle_response(api.wrapped_to_collateral(at, amount, currency_id))
    }

    fn collateral_to_wrapped(
        &self,
        amount: BalanceWrapper<Balance>,
        currency_id: CurrencyId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = at.unwrap_or_else(|| self.client.info().best_hash);

        handle_response(api.collateral_to_wrapped(at, amount, currency_id))
    }
}

//! Runtime API definition for the Oracle.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::dispatch::DispatchError;

pub use primitives::BalanceWrapper;

sp_api::decl_runtime_apis! {
    pub trait OracleApi<Balance, CurrencyId> where
        Balance: Codec,
        CurrencyId: Codec,
    {
        fn wrapped_to_collateral(
            amount: BalanceWrapper<Balance>,
            currency_id: CurrencyId,
        ) -> Result<BalanceWrapper<Balance>, DispatchError>;

        fn collateral_to_wrapped(
            amount: BalanceWrapper<Balance>,
            currency_id: CurrencyId,
        ) -> Result<BalanceWrapper<Balance>, DispatchError>;
    }
}

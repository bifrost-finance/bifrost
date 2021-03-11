// Copyright 2019-2021 Liebi Technologies.
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

// The swap pool algorithm implements Balancer protocol
// For more details, refer to https://balancer.finance/whitepaper/

#![cfg_attr(not(feature = "std"), no_std)]

use core::convert::Into;
use frame_support::{
    pallet_prelude::*,
    traits::{
        Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, ReservableCurrency,
        WithdrawReasons,
    },
    weights::Weight,
};
use sp_runtime::{
    traits::{AtLeast32Bit, CheckedSub, DispatchInfoOf, PostDispatchInfoOf, Saturating, Zero},
    transaction_validity::TransactionValidityError,
};

use frame_support::dispatch::DispatchResultWithPostInfo;
use frame_system::pallet_prelude::*;
pub use pallet::*;
use sp_arithmetic::traits::SaturatedConversion;
use sp_std::{vec, vec::Vec};

use node_primitives::CurrencyId;
use orml_traits::MultiCurrency;
use pallet_transaction_payment::OnChargeTransaction;
use zenlink_protocol::{AssetId, DEXOperations, TokenBalance};

mod default_weight;
mod mock;
mod tests;

type CurrencyIdOf<T> = <<T as Config>::CurrenciesHandler as MultiCurrency<
    <T as frame_system::Config>::AccountId,
>>::CurrencyId;

pub trait WeightInfo {
    fn set_user_fee_charge_order() -> Weight;
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    // pub trait Config: frame_system::Config + zenlink_protocol::Config {
    pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
        // /// The arithmetic type of asset identifier.
        // type CurrencyId: Parameter
        //     + Member
        //     + Copy
        //     + MaybeSerializeDeserialize
        //     + Ord
        //     + CurrencyIdExt
        //     + From<u8>;
        /// The units in which we record balances.
        type Balance: Member
            + Parameter
            + AtLeast32Bit
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Into<u128>
            + From<PalletBalanceOf<Self>>;
        /// Weight information for the extrinsics in this module.
        type WeightInfo: WeightInfo;
        /// Handler for both NativeCurrency and MultiCurrency
        type CurrenciesHandler: MultiCurrency<
            Self::AccountId,
            CurrencyId = CurrencyId,
            Balance = Self::Balance,
        >;
        /// The currency type in which fees will be paid.
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
        /// Handler for the unbalanced decrease
        type OnUnbalanced: OnUnbalanced<NegativeImbalanceOf<Self>>;
        /// Zenlink DEX operations handler
        type ZenlinkDEX: DEXOperations<Self::AccountId>;

        #[pallet::constant]
        type NativeCurrencyId: Get<CurrencyId>;
    }

    pub type PalletBalanceOf<T> =
        <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
    pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::NegativeImbalance;
    pub type PositiveImbalanceOf<T> = <<T as Config>::Currency as Currency<
        <T as frame_system::Config>::AccountId,
    >>::PositiveImbalance;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::type_value]
    pub fn DefaultFeeChargeOrder<T: Config>() -> Vec<CurrencyIdOf<T>> {
        let mut fee_charge_order_vec = Vec::new();
        // Should frequently update the number of Currency Ids. Right now there's only 12.
        for i in 0..12 {
            fee_charge_order_vec.push(CurrencyId::from(i as u8));
        }

        fee_charge_order_vec
    }

    #[pallet::storage]
    #[pallet::getter(fn user_fee_charge_order_list)]
    pub type UserFeeChargeOrderList<T: Config> =
        StorageMap<_, Twox64Concat, T::AccountId, Vec<CurrencyId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn default_fee_charge_order_list)]
    pub type DefaultFeeChargeOrderList<T: Config> =
        StorageValue<_, Vec<CurrencyId>, ValueQuery, DefaultFeeChargeOrder<T>>;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(<T as Config>::WeightInfo::set_user_fee_charge_order())]
        /// Set user fee charge assets order
        pub fn set_user_fee_charge_order(
            origin: OriginFor<T>,
            asset_order_list_vec: Option<Vec<CurrencyId>>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            if let Some(mut asset_order_list) = asset_order_list_vec {
                asset_order_list.insert(0, T::NativeCurrencyId::get());
                asset_order_list.dedup();
                UserFeeChargeOrderList::<T>::insert(&who, asset_order_list);
            } else {
                UserFeeChargeOrderList::<T>::remove(&who);
            }

            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Get user fee charge assets order
    fn inner_get_user_fee_charge_order_list(account_id: &T::AccountId) -> Vec<CurrencyId> {
        let mut charge_order_list = UserFeeChargeOrderList::<T>::get(&account_id);
        if charge_order_list.is_empty() {
            charge_order_list = DefaultFeeChargeOrderList::<T>::get();
        }
        charge_order_list
    }

    /// Make sure there are enough BNC to be deducted if the user has assets in other form of tokens rather than BNC.
    fn ensure_can_charge_fee(who: &T::AccountId, fee: PalletBalanceOf<T>, reason: WithdrawReasons) {
        // get the user defined fee charge order list.
        let user_fee_charge_order_list = Self::inner_get_user_fee_charge_order_list(who);

        // charge the fee by the order of the above order list.
        // first to check whether the user has the asset. If no, pass it. If yes, try to make transaction in the DEX in exchange for BNC
        for currency_id in user_fee_charge_order_list {
            println!("currency_id: {:?}", currency_id);
            let native_asset_id: AssetId = AssetId::from(T::NativeCurrencyId::get());
            // If it is mainnet currency
            if currency_id == T::NativeCurrencyId::get() {
                // check native balance if is enough
                let native_is_enough = <<T as Config>::Currency as Currency<
                    <T as frame_system::Config>::AccountId,
                >>::free_balance(who)
                .checked_sub(&fee)
                .map_or(false, |new_free_balance| {
                    <<T as Config>::Currency as Currency<
                                                <T as frame_system::Config>::AccountId,
                                            >>::ensure_can_withdraw(
                                                who, fee, reason, new_free_balance
                                            )
                                            .is_ok()
                });

                if native_is_enough {
                    println!(
                        "native_is_enough: {:?}",
                        <<T as Config>::Currency as Currency<
                            <T as frame_system::Config>::AccountId,
                        >>::free_balance(who)
                    );
                    // native balance is enough, break iteration
                    break;
                }
            } else {
                // If it is other assets
                let asset_balance = T::CurrenciesHandler::total_balance(currency_id, who);
                let asset_id: AssetId = AssetId::from(currency_id);

                // // mock
                // if asset_balance >= fee.into() {
                //     // decrease tokens of other currency while increase BNC tokens by the same amount
                //     T::CurrenciesHandler::asset_redeem(currency_id, who, fee.into());
                //     T::Currency::deposit_into_existing(who, fee.into())
                //         .unwrap_or_else(|_| PositiveImbalanceOf::<T>::zero());
                //     break;
                // }

                let path = vec![asset_id, native_asset_id];

                // let amount_out = TryInto::<TokenBalance>::try_into(fee);
                let amount_out: TokenBalance = fee.saturated_into();
                println!("amount_out: {:?}", amount_out);
                // let amount_in_max = TryInto::<TokenBalance>::try_into(asset_balance);
                let amount_in_max: TokenBalance = asset_balance.saturated_into();
                println!("amount_in_max: {:?}", amount_in_max);

                if T::ZenlinkDEX::inner_swap_tokens_for_exact_tokens(
                    who,
                    amount_out,
                    amount_in_max,
                    &path,
                    who,
                )
                .is_ok()
                {
                    // successfully swap, break iteration
                    break;
                }
            }
        }
    }

    /// This function is for runtime-api to call
    pub fn cal_fee_token_and_amount(
        who: &T::AccountId,
        fee: PalletBalanceOf<T>,
    ) -> Result<(CurrencyId, T::Balance), DispatchError> {
        let mut fee_token_id_out: CurrencyIdOf<T> = T::NativeCurrencyId::get();
        let mut fee_token_amount_out: T::Balance = T::Balance::from(0 as u32);

        // get the user defined fee charge order list.
        let user_fee_charge_order_list = Self::inner_get_user_fee_charge_order_list(who);
        let amount_out: TokenBalance = fee.saturated_into();
        let native_asset_id: AssetId = AssetId::from(T::NativeCurrencyId::get());

        // charge the fee by the order of the above order list.
        // first to check whether the user has the asset. If no, pass it. If yes, try to make transaction in the DEX in exchange for BNC
        for currency_id in user_fee_charge_order_list {
            // If it is mainnet currency
            if currency_id == T::NativeCurrencyId::get() {
                // check native balance if is enough
                let native_balance = <<T as Config>::Currency as Currency<
                    <T as frame_system::Config>::AccountId,
                >>::free_balance(who);

                if native_balance >= fee.into() {
                    fee_token_amount_out = fee.into();
                    break;
                }
            } else {
                // If it is other assets
                let asset_balance = T::CurrenciesHandler::total_balance(currency_id, who);
                let token_asset_id: AssetId = AssetId::from(currency_id);
                let path = vec![native_asset_id.clone(), token_asset_id];

                let amount_vec = T::ZenlinkDEX::get_amount_in_by_path(amount_out, &path)?;
                let amount_in = amount_vec[0];
                let amount_in_balance = amount_in.saturated_into();

                if asset_balance >= amount_in_balance {
                    fee_token_id_out = currency_id;
                    fee_token_amount_out = amount_in_balance;
                    break;
                }
            }
        }
        Ok((fee_token_id_out, fee_token_amount_out))
    }
}

/// Default implementation for a Currency and an OnUnbalanced handler.
impl<T> OnChargeTransaction<T> for Pallet<T>
where
    T: Config,
    T::TransactionByteFee: Get<PalletBalanceOf<T>>,
    T::Currency: Currency<<T as frame_system::Config>::AccountId>,
    PositiveImbalanceOf<T>: Imbalance<PalletBalanceOf<T>, Opposite = NegativeImbalanceOf<T>>,
    NegativeImbalanceOf<T>: Imbalance<PalletBalanceOf<T>, Opposite = PositiveImbalanceOf<T>>,
{
    type LiquidityInfo = Option<NegativeImbalanceOf<T>>;
    type Balance = PalletBalanceOf<T>;

    /// Withdraw the predicted fee from the transaction origin.
    ///
    /// Note: The `fee` already includes the `tip`.
    fn withdraw_fee(
        who: &T::AccountId,
        _call: &T::Call,
        _info: &DispatchInfoOf<T::Call>,
        fee: Self::Balance,
        tip: Self::Balance,
    ) -> Result<Self::LiquidityInfo, TransactionValidityError> {
        if fee.is_zero() {
            return Ok(None);
        }

        let withdraw_reason = if tip.is_zero() {
            WithdrawReasons::TRANSACTION_PAYMENT
        } else {
            WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
        };
        // Make sure there are enough BNC to be deducted if the user has assets in other form of tokens rather than BNC.
        Self::ensure_can_charge_fee(who, fee, withdraw_reason);

        match T::Currency::withdraw(who, fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
            Ok(imbalance) => Ok(Some(imbalance)),
            Err(_) => Err(InvalidTransaction::Payment.into()),
        }
    }

    /// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
    /// Since the predicted fee might have been too high, parts of the fee may
    /// be refunded.
    ///
    /// Note: The `fee` already includes the `tip`.
    fn correct_and_deposit_fee(
        who: &T::AccountId,
        _dispatch_info: &DispatchInfoOf<T::Call>,
        _post_info: &PostDispatchInfoOf<T::Call>,
        corrected_fee: Self::Balance,
        tip: Self::Balance,
        already_withdrawn: Self::LiquidityInfo,
    ) -> Result<(), TransactionValidityError> {
        if let Some(paid) = already_withdrawn {
            // Calculate how much refund we should return
            let refund_amount = paid.peek().saturating_sub(corrected_fee);
            // refund to the the account that paid the fees. If this fails, the
            // account might have dropped below the existential balance. In
            // that case we don't refund anything.
            let refund_imbalance = T::Currency::deposit_into_existing(&who, refund_amount)
                .unwrap_or_else(|_| PositiveImbalanceOf::<T>::zero());
            // merge the imbalance caused by paying the fees and refunding parts of it again.
            let adjusted_paid = paid
                .offset(refund_imbalance)
                .map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
            // Call someone else to handle the imbalance (fee and tip separately)
            let imbalances = adjusted_paid.split(tip);
            T::OnUnbalanced::on_unbalanceds(
                Some(imbalances.0).into_iter().chain(Some(imbalances.1)),
            );
        }
        Ok(())
    }
}

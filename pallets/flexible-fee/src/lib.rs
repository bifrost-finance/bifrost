// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

#![cfg_attr(not(feature = "std"), no_std)]

use core::convert::Into;
use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	traits::{
		Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, ReservableCurrency,
		WithdrawReasons,
	},
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, ExtraFeeName, TryConvertFrom};
use orml_traits::MultiCurrency;
pub use pallet::*;
use pallet_transaction_payment::OnChargeTransaction;
use sp_arithmetic::traits::SaturatedConversion;
use sp_runtime::{
	traits::{DispatchInfoOf, PostDispatchInfoOf, Saturating, Zero},
	transaction_validity::TransactionValidityError,
	BoundedVec,
};
use sp_std::{vec, vec::Vec};
pub use weights::WeightInfo;
use zenlink_protocol::{AssetBalance, AssetId, ExportZenlink};

use crate::misc_fees::{FeeDeductor, FeeGetter};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
pub mod misc_fees;
mod mock;
mod tests;
pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_transaction_payment::Config {
		/// Event
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
		/// Handler for both NativeCurrency and MultiCurrency
		type MultiCurrency: MultiCurrency<
			Self::AccountId,
			CurrencyId = CurrencyId,
			Balance = PalletBalanceOf<Self>,
		>;
		/// The currency type in which fees will be paid.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Handler for the unbalanced decrease
		type OnUnbalanced: OnUnbalanced<NegativeImbalanceOf<Self>>;
		type DexOperator: ExportZenlink<Self::AccountId, AssetId>;
		/// Filter if this transaction needs to be deducted extra fee besides basic transaction fee,
		/// and get the name of the fee
		type ExtraFeeMatcher: FeeGetter<CallOf<Self>>;
		/// In charge of deducting extra fees
		type MiscFeeHandler: FeeDeductor<
			Self::AccountId,
			CurrencyIdOf<Self>,
			PalletBalanceOf<Self>,
			CallOf<Self>,
		>;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyIdOf<Self>>;

		#[pallet::constant]
		type AlternativeFeeCurrencyId: Get<CurrencyIdOf<Self>>;

		/// Alternative Fee currency exchange rate: ?x Fee currency: ?y Native currency
		#[pallet::constant]
		type AltFeeCurrencyExchangeRate: Get<(u32, u32)>;

		#[pallet::constant]
		type MaxFeeCurrencyOrderListLen: Get<u32>;

		type ParachainId: Get<ParaId>;

		/// The only origin that can set universal fee currency order list
		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub type CurrencyIdOf<T> =
		<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::CurrencyId;
	pub type PalletBalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
	pub type NegativeImbalanceOf<T> =
		<<T as Config>::Currency as Currency<AccountIdOf<T>>>::NegativeImbalance;
	pub type PositiveImbalanceOf<T> =
		<<T as Config>::Currency as Currency<AccountIdOf<T>>>::PositiveImbalance;

	pub type CallOf<T> = <T as frame_system::Config>::RuntimeCall;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FlexibleFeeExchanged(CurrencyIdOf<T>, PalletBalanceOf<T>), // token and amount
		FixedRateFeeExchanged(CurrencyIdOf<T>, PalletBalanceOf<T>),
		ExtraFeeDeducted(ExtraFeeName, CurrencyIdOf<T>, PalletBalanceOf<T>),
	}

	/// The current storage version, we set to 2 our new version(after migrate stroage from vec t
	/// boundedVec).
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	/// Universal fee currency order list for all users
	#[pallet::storage]
	#[pallet::getter(fn get_universal_fee_currency_order_list)]
	pub type UniversalFeeCurrencyOrderList<T: Config> =
		StorageValue<_, BoundedVec<CurrencyIdOf<T>, T::MaxFeeCurrencyOrderListLen>, ValueQuery>;

	/// User default fee currency, if set, will be used as the first fee currency, and then use the
	/// universal fee currency order list
	#[pallet::storage]
	#[pallet::getter(fn get_user_default_fee_currency)]
	pub type UserDefaultFeeCurrency<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, CurrencyIdOf<T>, OptionQuery>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		Overflow,
		ConversionError,
		WrongListLength,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set user default fee currency
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::set_user_default_fee_currency())]
		pub fn set_user_default_fee_currency(
			origin: OriginFor<T>,
			maybe_fee_currency: Option<CurrencyIdOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if let Some(fee_currency) = maybe_fee_currency {
				UserDefaultFeeCurrency::<T>::insert(&who, fee_currency);
			} else {
				UserDefaultFeeCurrency::<T>::remove(&who);
			}

			Ok(())
		}

		/// Set universal fee currency order list
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::set_universal_fee_currency_order_list())]
		pub fn set_universal_fee_currency_order_list(
			origin: OriginFor<T>,
			default_list: BoundedVec<CurrencyIdOf<T>, T::MaxFeeCurrencyOrderListLen>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			ensure!(default_list.len() > 0 as usize, Error::<T>::WrongListLength);

			UniversalFeeCurrencyOrderList::<T>::set(default_list);

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Get user fee charge assets order
	fn inner_get_user_fee_charge_order_list(account_id: &T::AccountId) -> Vec<CurrencyIdOf<T>> {
		let mut order_list: Vec<CurrencyIdOf<T>> = Vec::new();
		// Get user default fee currency
		if let Some(user_default_fee_currency) = UserDefaultFeeCurrency::<T>::get(&account_id) {
			order_list.push(user_default_fee_currency);
		};

		// Get universal fee currency order list
		let mut universal_fee_currency_order_list: Vec<CurrencyIdOf<T>> =
			UniversalFeeCurrencyOrderList::<T>::get().into_iter().collect();

		// Concat user default fee currency and universal fee currency order list
		order_list.append(&mut universal_fee_currency_order_list);

		order_list
	}

	/// Make sure there are enough BNC to be deducted if the user has assets in other form of tokens
	/// rather than BNC.
	fn ensure_can_charge_fee(
		who: &T::AccountId,
		fee: PalletBalanceOf<T>,
		_reason: WithdrawReasons,
	) -> Result<PalletBalanceOf<T>, DispatchError> {
		let result_option = Self::find_out_fee_currency_and_amount(who, fee)
			.map_err(|_| DispatchError::Other("Fee calculation Error."))?;

		if let Some((currency_id, amount_in, amount_out)) = result_option {
			if currency_id != T::NativeCurrencyId::get() {
				let native_asset_id: AssetId = AssetId::try_convert_from(
					T::NativeCurrencyId::get(),
					T::ParachainId::get().into(),
				)
				.map_err(|_| DispatchError::Other("Conversion Error."))?;
				let asset_id: AssetId =
					AssetId::try_convert_from(currency_id, T::ParachainId::get().into())
						.map_err(|_| DispatchError::Other("Conversion Error."))?;
				let path = vec![asset_id, native_asset_id];

				T::DexOperator::inner_swap_assets_for_exact_assets(
					who,
					amount_out.saturated_into(),
					amount_in.saturated_into(),
					&path,
					who,
				)?;

				Self::deposit_event(Event::FlexibleFeeExchanged(
					currency_id,
					PalletBalanceOf::<T>::saturated_from(amount_in),
				));
			}
		}

		Ok(fee)
	}

	/// This function is for runtime-api to call
	pub fn cal_fee_token_and_amount(
		who: &T::AccountId,
		fee: PalletBalanceOf<T>,
	) -> Result<(CurrencyIdOf<T>, PalletBalanceOf<T>), DispatchError> {
		let result_option = Self::find_out_fee_currency_and_amount(who, fee)
			.map_err(|_| DispatchError::Other("Fee calculation Error."))?;

		let (currency_id, amount_in, _amount_out) =
			result_option.ok_or(DispatchError::Other("Not enough balance for fee."))?;

		Ok((currency_id, amount_in))
	}
}

/// Default implementation for a Currency and an OnUnbalanced handler.
impl<T> OnChargeTransaction<T> for Pallet<T>
where
	T: Config,
	T::Currency: Currency<<T as frame_system::Config>::AccountId>,
	PositiveImbalanceOf<T>: Imbalance<PalletBalanceOf<T>, Opposite = NegativeImbalanceOf<T>>,
	NegativeImbalanceOf<T>: Imbalance<PalletBalanceOf<T>, Opposite = PositiveImbalanceOf<T>>,
{
	type Balance = PalletBalanceOf<T>;
	type LiquidityInfo = Option<NegativeImbalanceOf<T>>;

	/// Withdraw the predicted fee from the transaction origin.
	///
	/// Note: The `fee` already includes the `tip`.
	fn withdraw_fee(
		who: &T::AccountId,
		call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
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

		// Make sure there are enough BNC to be deducted if the user has assets in other form of
		// tokens rather than BNC.
		Self::ensure_can_charge_fee(who, fee, withdraw_reason)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

		let rs = match T::Currency::withdraw(
			who,
			fee,
			withdraw_reason,
			ExistenceRequirement::AllowDeath,
		) {
			Ok(imbalance) => Ok(Some(imbalance)),
			Err(_msg) => Err(InvalidTransaction::Payment.into()),
		};

		// See if the this RuntimeCall needs to pay extra fee
		let (fee_name, if_extra_fee) = T::ExtraFeeMatcher::get_fee_info(&call);
		if if_extra_fee {
			// We define 77 as the error of extra fee deduction failure.
			let (extra_fee_currency, extra_fee_amount) =
				T::MiscFeeHandler::deduct_fee(who, &T::TreasuryAccount::get(), call).map_err(
					|_| TransactionValidityError::Invalid(InvalidTransaction::Custom(77u8)),
				)?;
			Self::deposit_event(Event::ExtraFeeDeducted(
				fee_name,
				extra_fee_currency,
				extra_fee_amount,
			));
		}

		rs
	}

	/// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
	/// Since the predicted fee might have been too high, parts of the fee may
	/// be refunded.
	///
	/// Note: The `fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &T::AccountId,
		_dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		_post_info: &PostDispatchInfoOf<T::RuntimeCall>,
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
			let refund_imbalance = T::Currency::deposit_into_existing(who, refund_amount)
				.unwrap_or_else(|_| PositiveImbalanceOf::<T>::zero());
			// merge the imbalance caused by paying the fees and refunding parts of it again.
			let adjusted_paid = paid
				.offset(refund_imbalance)
				.same()
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
			// RuntimeCall someone else to handle the imbalance (fee and tip separately)
			let imbalances = adjusted_paid.split(tip);
			T::OnUnbalanced::on_unbalanceds(
				Some(imbalances.0).into_iter().chain(Some(imbalances.1)),
			);
		}
		Ok(())
	}
}

impl<T: Config> Pallet<T> {
	fn find_out_fee_currency_and_amount(
		who: &T::AccountId,
		fee: PalletBalanceOf<T>,
	) -> Result<Option<(CurrencyIdOf<T>, PalletBalanceOf<T>, PalletBalanceOf<T>)>, Error<T>> {
		// get the user defined fee charge order list.
		let user_fee_charge_order_list = Self::inner_get_user_fee_charge_order_list(who);

		// charge the fee by the order of the above order list.
		// first to check whether the user has the asset. If no, pass it. If yes, try to make
		// transaction in the DEX in exchange for BNC
		for currency_id in user_fee_charge_order_list {
			// If it is mainnet currency
			if currency_id == T::NativeCurrencyId::get() {
				// check native balance if is enough
				if T::MultiCurrency::ensure_can_withdraw(currency_id, who, fee).is_ok() {
					// currency, amount_in, amount_out
					return Ok(Some((currency_id, fee, fee)));
				}
			} else {
				// If it is other assets, go to exchange fee amount.
				let native_asset_id: AssetId = AssetId::try_convert_from(
					T::NativeCurrencyId::get(),
					T::ParachainId::get().into(),
				)
				.map_err(|_| Error::<T>::ConversionError)?;

				let amount_out: AssetBalance = fee.saturated_into();

				let asset_id: AssetId =
					AssetId::try_convert_from(currency_id, T::ParachainId::get().into())
						.map_err(|_| Error::<T>::ConversionError)?;

				let path = vec![asset_id, native_asset_id];
				// see if path exists, if not, continue.
				// query for amount in
				if let Ok(amounts) = T::DexOperator::get_amount_in_by_path(amount_out, &path) {
					// make sure the user has enough free token balance that can be charged.
					let amount_in = PalletBalanceOf::<T>::saturated_from(amounts[0]);
					if T::MultiCurrency::ensure_can_withdraw(currency_id, who, amount_in).is_ok() {
						// currency, amount_in, amount_out
						return Ok(Some((
							currency_id,
							amount_in,
							PalletBalanceOf::<T>::saturated_from(amount_out),
						)));
					}
				}
			}
		}
		Ok(None)
	}
}

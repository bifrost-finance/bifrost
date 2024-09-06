// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use frame_support::{ensure, pallet_prelude::*, sp_runtime::traits::AccountIdConversion, PalletId};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;

use bifrost_primitives::{
	currency::{VBNC, VBNC_P},
	CurrencyId,
};
pub use pallet::*;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod mock;
mod tests;
pub mod weights;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub(crate) type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Currecny operation handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
		/// VBNC-convert Pallet Id
		type VBNCConvertPalletId: Get<PalletId>;
		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
	}
	#[pallet::error]
	/// Error types for the VBNC convert pallet.
	pub enum Error<T> {
		/// The account does not have enough balance to complete the operation.
		NotEnoughBalance,
		/// The resulting balance is less than the existential deposit, which would lead to the
		/// account being reaped.
		LessThanExistentialDeposit,
		/// The specified currency is not supported for conversion to VBNC-P.
		CurrencyNotSupport,
	}

	#[pallet::event]
	/// Event types emitted by the VBNC convert pallet.
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Emitted when VBNC-P has been successfully converted and transferred to the user.
		VBNCPConverted {
			/// The account that received the converted VBNC-P.
			to: AccountIdOf<T>,
			/// The amount of VBNC-P converted.
			value: BalanceOf<T>,
		},
		/// Emitted when VBNC-P has been successfully charged from a user account.
		VbncPCharged {
			/// The account from which VBNC-P was charged.
			who: AccountIdOf<T>,
			/// The amount of VBNC-P charged.
			value: BalanceOf<T>,
		},
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Converts the specified `value` of a supported currency to VBNC-P.
		///
		/// # Parameters
		/// - `origin`: The origin of the transaction, which must be a signed account.
		/// - `currency`: The currency to be converted into VBNC-P.
		/// - `value`: The amount of the specified currency to be converted.
		///
		/// # Errors
		/// - `Error::<T>::CurrencyNotSupport`: If the provided currency is not supported for
		///   conversion.
		/// - `Error::<T>::NotEnoughBalance`: If the user does not have sufficient balance of the
		///   specified currency.
		/// - `Error::<T>::LessThanExistentialDeposit`: If the converted amount of VBNC-P is less
		///   than the minimum required balance.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::convert_to_vbnc_p())]
		pub fn convert_to_vbnc_p(
			origin: OriginFor<T>,
			currency: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_currency(&currency)?;

			// check the user balance of currency
			T::MultiCurrency::ensure_can_withdraw(currency, &who, value)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;

			// the VBNC and VBNC-P exchange ratio is one to one
			let vbnc_p_amount = value;
			Self::ensure_pool_balance_enough(VBNC_P, vbnc_p_amount)?;

			let existential_deposit = T::MultiCurrency::minimum_balance(VBNC_P);
			ensure!(vbnc_p_amount >= existential_deposit, Error::<T>::LessThanExistentialDeposit);

			// transfer vBNC-p from pool to user
			let vbnc_pool_account = Self::vbnc_p_pool_account();
			T::MultiCurrency::transfer(VBNC_P, &vbnc_pool_account, &who, vbnc_p_amount)?;

			// burn currency
			T::MultiCurrency::withdraw(currency, &who, value)?;

			// deposit event
			Self::deposit_event(Event::VBNCPConverted { to: who, value: vbnc_p_amount });

			Ok(())
		}

		/// Charges the specified `amount` of VBNC-P from the user's account.
		///
		/// # Parameters
		/// - `origin`: The origin of the transaction, which must be a signed account.
		/// - `amount`: The amount of VBNC-P to charge from the user's account.
		///
		/// # Errors
		/// - `Error::<T>::NotEnoughBalance`: If the user does not have sufficient VBNC-P to charge.
		/// - `Error::<T>::LessThanExistentialDeposit`: If the amount to be charged is less than the
		///   existential deposit.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::charge_vbnc_p())]
		pub fn charge_vbnc_p(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// check the user balance of currency
			T::MultiCurrency::ensure_can_withdraw(VBNC_P, &who, amount)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;

			let vbnc_pool_account = Self::vbnc_p_pool_account();
			T::MultiCurrency::transfer(VBNC_P, &who, &vbnc_pool_account, amount)?;

			// deposit event
			Self::deposit_event(Event::VbncPCharged { who, value: amount });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns the account ID of VBNCConvertPalletId.
		/// This account is used to hold VBNC-P and perform conversions.
		pub fn vbnc_p_pool_account() -> AccountIdOf<T> {
			T::VBNCConvertPalletId::get().into_account_truncating()
		}

		/// Ensures that the provided currency is supported for conversion.
		/// Currently, only VBNC is supported.
		fn ensure_currency(currency: &CurrencyIdOf<T>) -> Result<(), DispatchError> {
			// Ensure that the currency is VBNC, otherwise return an error.
			ensure!(*currency == VBNC, Error::<T>::CurrencyNotSupport);
			Ok(())
		}

		/// Ensures that the VBNC-P pool has enough balance to complete the transaction.
		/// If the pool has insufficient balance, an error is returned.
		fn ensure_pool_balance_enough(
			currency: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> Result<(), DispatchError> {
			let pool_account = Self::vbnc_p_pool_account();

			// Check if the pool account can withdraw the specified amount of currency.
			T::MultiCurrency::ensure_can_withdraw(currency, &pool_account, value)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;

			Ok(())
		}
	}
}

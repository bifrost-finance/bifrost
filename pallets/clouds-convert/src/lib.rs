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

use alloc::vec;
use bb_bnc::BbBNCInterface;
use bifrost_primitives::{
	currency::{CLOUD, VBNC},
	CurrencyId,
};
use frame_support::{
	ensure,
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, UniqueSaturatedFrom},
		SaturatedConversion,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
use sp_core::U256;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod mock;
mod tests;
pub mod weights;

pub use pallet::*;

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

		/// Clouds Pallet Id
		type CloudsPalletId: Get<PalletId>;

		// bbBNC interface
		type BbBNC: BbBNCInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
			BlockNumberFor<Self>,
		>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// locked blocks for veBNC converted from clouds
		#[pallet::constant]
		type LockedBlocks: Get<BlockNumberFor<Self>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		CalculationOverflow,
		LessThanExpected,
		LessThanExistentialDeposit,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		CloudsConverted { clouds: BalanceOf<T>, vebnc: BalanceOf<T> },

		VbncCharged { vbnc: BalanceOf<T> },
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::clouds_to_vebnc())]
		pub fn clouds_to_vebnc(
			origin: OriginFor<T>,
			value: BalanceOf<T>,
			expected_min_vebnc: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// check the user balance of clouds
			T::MultiCurrency::ensure_can_withdraw(CLOUD, &who, value)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;

			let can_get_vbnc = Self::calculate_can_get_vbnc(value)?;
			ensure!(can_get_vbnc >= expected_min_vebnc, Error::<T>::LessThanExpected);
			// ensure can_get_vbnc greater than existential deposit
			let existential_deposit = T::MultiCurrency::minimum_balance(VBNC);
			ensure!(can_get_vbnc >= existential_deposit, Error::<T>::LessThanExistentialDeposit);

			// burn clouds
			T::MultiCurrency::withdraw(CLOUD, &who, value)?;

			// transfer vBNC from pool to user
			let vbnc_pool_account = Self::clouds_pool_account();
			T::MultiCurrency::transfer(VBNC, &vbnc_pool_account, &who, can_get_vbnc)?;

			// mint veBNC for user
			T::BbBNC::create_lock_inner(&who, can_get_vbnc, T::LockedBlocks::get())?;

			// deposit event
			Self::deposit_event(Event::CloudsConverted { clouds: value, vebnc: can_get_vbnc });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::charge_vbnc())]
		pub fn charge_vbnc(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Transfer vBNC from user to clouds pool
			let vbnc_pool_account = Self::clouds_pool_account();
			T::MultiCurrency::transfer(VBNC, &who, &vbnc_pool_account, amount)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;

			// deposit event
			Self::deposit_event(Event::VbncCharged { vbnc: amount });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn calculate_can_get_vbnc(clouds: BalanceOf<T>) -> Result<BalanceOf<T>, Error<T>> {
			// get the vBNC balance of clouds pool
			let vbnc_pool_account = Self::clouds_pool_account();
			let vbnc_balance = T::MultiCurrency::free_balance(VBNC, &vbnc_pool_account);

			// get the total supply of clouds
			let total_supply = T::MultiCurrency::total_issuance(CLOUD);

			let can_get_amount = U256::from(vbnc_balance.saturated_into::<u128>())
				.saturating_mul(clouds.saturated_into::<u128>().into())
				.checked_div(total_supply.saturated_into::<u128>().into())
				// first turn into u128，then use unique_saturated_into BalanceOf<T>
				.map(|x| x.saturated_into::<u128>())
				.map(|x| BalanceOf::<T>::unique_saturated_from(x))
				.ok_or(Error::<T>::CalculationOverflow)?;

			Ok(can_get_amount)
		}

		pub fn clouds_pool_account() -> AccountIdOf<T> {
			T::CloudsPalletId::get().into_account_truncating()
		}
	}
}

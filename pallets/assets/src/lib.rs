// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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
#![allow(clippy::unused_unit)]

// pub use crate::imbalances::{NegativeImbalance, PositiveImbalance};

use frame_support::{ensure, pallet_prelude::*, transactional};
use frame_system::{pallet_prelude::*};
use orml_traits::{
	currency::TransferAll,
	MultiReservableCurrency, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency,
};
use sp_runtime::traits::StaticLookup;

mod mock;
mod tests;

pub use pallet::*;

type BalanceOf<T> = <
	<T as Config>::MultiCurrency as MultiCurrency<<T as frame_system::Config>::AccountId>
>::Balance;
type CurrencyIdOf<T> = <
	<T as Config>::MultiCurrency as MultiCurrency<<T as frame_system::Config>::AccountId>
>::CurrencyId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: TransferAll<Self::AccountId>
			+ MultiCurrencyExtended<Self::AccountId>
			+ MultiLockableCurrency<Self::AccountId>
			+ MultiReservableCurrency<Self::AccountId>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The balance is too low
		BalanceTooLow,
		/// This operation will cause balance to overflow
		BalanceOverflow,
		/// Destroy balance too much
		BurnTooMuch,
	}

	#[pallet::event]
	#[pallet::metadata(BalanceOf<T> = "Balance", CurrencyIdOf<T> = "CurrencyId")]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Token issue success, \[currency_id, dest, amount\]
		Issued(T::AccountId, CurrencyIdOf<T>, BalanceOf<T>),
		/// Token burn success, \[currency_id, dest, amount\]
		Burned(T::AccountId, CurrencyIdOf<T>, BalanceOf<T>),

	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Issue some balance to an account.
		///
		/// The dispatch origin for this call must be `Root` by the
		/// transactor.
		#[pallet::weight(1000)]
		#[transactional]
		pub fn issue(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyIdOf<T>,
			#[pallet::compact] amount: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let dest = T::Lookup::lookup(dest)?;
			T::MultiCurrency::deposit(currency_id, &dest, amount)?;

			Self::deposit_event(Event::Issued(dest, currency_id, amount));
			Ok(().into())
		}

		/// Destroy some balance from an account.
		///
		/// The dispatch origin for this call must be `Root` by the
		/// transactor.
		#[pallet::weight(1000)]
		#[transactional]
		pub fn burn(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			currency_id: CurrencyIdOf<T>,
			#[pallet::compact] amount: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let dest = T::Lookup::lookup(dest)?;

			let balance = T::MultiCurrency::free_balance(currency_id, &dest);
			ensure!(balance > amount, Error::<T>::BurnTooMuch);

			T::MultiCurrency::withdraw(currency_id, &dest, amount)?;

			Self::deposit_event(Event::Burned(dest, currency_id, amount));
			Ok(().into())
		}
	}

	pub trait WeightInfo {
		fn burn() -> Weight;
		fn issue() -> Weight;
	}

	impl WeightInfo for () {
		fn burn() -> Weight { Default::default() }
		fn issue() -> Weight { Default::default() }
	}
}

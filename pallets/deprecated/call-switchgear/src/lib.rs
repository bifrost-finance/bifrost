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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use bifrost_primitives::CurrencyId;
use frame_support::{
	pallet_prelude::*,
	traits::{CallMetadata, Contains, GetCallMetadata, PalletInfoAccess},
};
use frame_system::pallet_prelude::*;
use sp_runtime::DispatchResult;
use sp_std::prelude::*;

mod mock;
mod tests;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The origin which may set filter.
		type UpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// can not switch off
		CannotSwitchOff,
		/// Invalid character
		InvalidCharacter,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Switch off transaction . \[pallet_name, function_name\]
		TransactionSwitchedoff(Vec<u8>, Vec<u8>),
		/// Switch on transaction . \[pallet_name, function_name\]
		TransactionSwitchedOn(Vec<u8>, Vec<u8>),
		TransferAccountDisabled(CurrencyId),
		TransferAccountEnabled(CurrencyId),
	}

	/// Controls whether or not all of the pallets are banned.
	#[pallet::storage]
	#[pallet::getter(fn get_overall_indicator)]
	pub(crate) type OverallToggle<T: Config> = StorageValue<_, bool, ValueQuery, DefaultStatus>;

	// Defult release amount is 30 KSM
	#[pallet::type_value]
	pub fn DefaultStatus() -> bool {
		false
	}

	#[pallet::storage]
	#[pallet::getter(fn get_switchoff_transactions)]
	pub type SwitchedOffTransactions<T: Config> =
		StorageMap<_, Twox64Concat, (Vec<u8>, Vec<u8>), (), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_disabled_tranfer_accounts)]
	pub type DisabledTransfers<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, (), OptionQuery>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::switchoff_transaction())]
		pub fn switchoff_transaction(
			origin: OriginFor<T>,
			pallet_name: Vec<u8>,
			function_name: Vec<u8>,
		) -> DispatchResult {
			T::UpdateOrigin::ensure_origin(origin)?;

			let pallet_name_string =
				sp_std::str::from_utf8(&pallet_name).map_err(|_| Error::<T>::InvalidCharacter)?;
			ensure!(
				pallet_name_string != <Self as PalletInfoAccess>::name(),
				Error::<T>::CannotSwitchOff
			);

			// If "all" received, ban all of the pallets. Otherwise, only the passed-in pallet.
			if pallet_name_string.to_lowercase() == "all" {
				OverallToggle::<T>::put(true);
			} else {
				SwitchedOffTransactions::<T>::mutate_exists(
					(pallet_name.clone(), function_name.clone()),
					|item| {
						if item.is_none() {
							*item = Some(());
						}
					},
				);
			}

			Self::deposit_event(Event::TransactionSwitchedoff(pallet_name, function_name));

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::switchon_transaction())]
		pub fn switchon_transaction(
			origin: OriginFor<T>,
			pallet_name: Vec<u8>,
			function_name: Vec<u8>,
		) -> DispatchResult {
			T::UpdateOrigin::ensure_origin(origin)?;

			let pallet_name_string =
				sp_std::str::from_utf8(&pallet_name).map_err(|_| Error::<T>::InvalidCharacter)?;

			if pallet_name_string.to_lowercase() == "all" {
				OverallToggle::<T>::put(false);
			} else {
				SwitchedOffTransactions::<T>::take((pallet_name.clone(), &function_name.clone()));
			}

			Self::deposit_event(Event::TransactionSwitchedOn(pallet_name, function_name));

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::disable_transfers())]
		pub fn disable_transfers(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResult {
			T::UpdateOrigin::ensure_origin(origin)?;

			DisabledTransfers::<T>::mutate_exists(currency_id, |item| {
				if item.is_none() {
					*item = Some(());
					Self::deposit_event(Event::TransferAccountDisabled(currency_id));
				}
			});
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::enable_transfers())]
		pub fn enable_transfers(origin: OriginFor<T>, currency_id: CurrencyId) -> DispatchResult {
			T::UpdateOrigin::ensure_origin(origin)?;

			if DisabledTransfers::<T>::take(currency_id).is_some() {
				Self::deposit_event(Event::TransferAccountEnabled(currency_id));
			};
			Ok(())
		}
	}
}

pub struct SwitchOffTransactionFilter<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> Contains<T::RuntimeCall> for SwitchOffTransactionFilter<T>
where
	<T as frame_system::Config>::RuntimeCall: GetCallMetadata,
{
	fn contains(call: &T::RuntimeCall) -> bool {
		let CallMetadata { function_name, pallet_name } = call.get_call_metadata();
		SwitchedOffTransactions::<T>::contains_key((
			pallet_name.as_bytes(),
			function_name.as_bytes(),
		))
	}
}

pub struct DisableTransfersFilter<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> Contains<CurrencyId> for DisableTransfersFilter<T> {
	fn contains(currency_id: &CurrencyId) -> bool {
		DisabledTransfers::<T>::contains_key(currency_id)
	}
}

pub struct OverallToggleFilter<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> OverallToggleFilter<T> {
	pub fn get_overall_toggle_status() -> bool {
		OverallToggle::<T>::get()
	}
}

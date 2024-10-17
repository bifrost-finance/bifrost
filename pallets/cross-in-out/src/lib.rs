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
use bifrost_primitives::CurrencyId;
use frame_support::{ensure, pallet_prelude::*, PalletId};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
use sp_std::boxed::Box;
pub use weights::WeightInfo;
#[allow(deprecated)]
use xcm::v2::MultiLocation;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod migrations;
mod mock;
mod tests;
pub mod weights;

pub use pallet::*;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(deprecated)]
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Currecny operation handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Entrance account Pallet Id
		type EntrancePalletId: Get<PalletId>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type MaxLengthLimit: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Indicates that the balance is not sufficient for the requested operation.
		NotEnoughBalance,
		/// Indicates that the specified item does not exist.
		NotExist,
		/// Indicates that the operation is not allowed for the current context.
		NotAllowed,
		/// Indicates that the currency does not support crossing in and out.
		CurrencyNotSupportCrossInAndOut,
		/// Indicates that there is no mapping for the specified multilocation.
		NoMultilocationMapping,
		/// Indicates that the item already exists.
		AlreadyExist,
		/// Indicates that there is no minimum crossing amount set for the operation.
		NoCrossingMinimumSet,
		/// Indicates that the specified amount is lower than the required minimum.
		AmountLowerThanMinimum,
		/// Indicates that the list has reached its maximum capacity.
		ListOverflow,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a currency is successfully crossed out from a location.
		CrossedOut {
			currency_id: CurrencyId,
			crosser: AccountIdOf<T>,
			location: MultiLocation,
			amount: BalanceOf<T>,
		},
		/// Event emitted when a currency is deregistered.
		CurrencyDeregistered { currency_id: CurrencyId },
		/// Event emitted when a linked account is successfully registered.
		LinkedAccountRegistered {
			currency_id: CurrencyId,
			who: AccountIdOf<T>,
			foreign_location: MultiLocation,
		},
		/// Event emitted when an account is added to the register list.
		AddedToRegisterList { account: AccountIdOf<T>, currency_id: CurrencyId },
		/// Event emitted when an account is removed from the register list.
		RemovedFromRegisterList { account: AccountIdOf<T>, currency_id: CurrencyId },
		/// Event emitted when the crossing minimum amounts are set for a currency.
		CrossingMinimumAmountSet {
			currency_id: CurrencyId,
			cross_in_minimum: BalanceOf<T>,
			cross_out_minimum: BalanceOf<T>,
		},
	}

	/// The current storage version, we set to 2 our new version(after migrate stroage from vec t
	/// boundedVec).
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

	/// To store currencies that support indirect cross-in and cross-out.
	#[pallet::storage]
	pub type CrossCurrencyRegistry<T> = StorageMap<_, Blake2_128Concat, CurrencyId, ()>;

	/// Accounts in the whitelist can issue the corresponding Currency.
	#[pallet::storage]
	pub type IssueWhiteList<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BoundedVec<AccountIdOf<T>, T::MaxLengthLimit>>;

	/// Accounts in the whitelist can register the mapping between a multilocation and an accountId.
	#[pallet::storage]
	pub type RegisterWhiteList<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BoundedVec<AccountIdOf<T>, T::MaxLengthLimit>>;

	/// Mapping a Bifrost account to a multilocation of a outer chain
	#[pallet::storage]
	pub type AccountToOuterMultilocation<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		AccountIdOf<T>,
		MultiLocation,
		OptionQuery,
	>;

	/// Mapping a multilocation of a outer chain to a Bifrost account
	#[pallet::storage]
	pub type OuterMultilocationToAccount<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		AccountIdOf<T>,
		OptionQuery,
	>;

	/// minimum crossin and crossout amount【crossinMinimum, crossoutMinimum】
	#[pallet::storage]
	pub type CrossingMinimumAmount<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (BalanceOf<T>, BalanceOf<T>)>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Destroy some balance from an account and issue cross-out event.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::cross_out())]
		pub fn cross_out(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let crosser = ensure_signed(origin)?;

			ensure!(
				CrossCurrencyRegistry::<T>::contains_key(currency_id),
				Error::<T>::CurrencyNotSupportCrossInAndOut
			);

			let crossing_minimum_amount = CrossingMinimumAmount::<T>::get(currency_id)
				.ok_or(Error::<T>::NoCrossingMinimumSet)?;
			ensure!(amount >= crossing_minimum_amount.1, Error::<T>::AmountLowerThanMinimum);

			let balance = T::MultiCurrency::free_balance(currency_id, &crosser);
			ensure!(balance >= amount, Error::<T>::NotEnoughBalance);

			let location = AccountToOuterMultilocation::<T>::get(currency_id, &crosser)
				.ok_or(Error::<T>::NoMultilocationMapping)?;

			T::MultiCurrency::withdraw(currency_id, &crosser, amount)?;

			Self::deposit_event(Event::CrossedOut { currency_id, crosser, location, amount });
			Ok(())
		}

		// Register the mapping relationship of Bifrost account and account from other chains
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::register_linked_account())]
		pub fn register_linked_account(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: AccountIdOf<T>,
			foreign_location: Box<MultiLocation>,
		) -> DispatchResult {
			let registerer = ensure_signed(origin)?;

			let register_whitelist =
				RegisterWhiteList::<T>::get(currency_id).ok_or(Error::<T>::NotAllowed)?;
			ensure!(register_whitelist.contains(&registerer), Error::<T>::NotAllowed);

			ensure!(
				CrossCurrencyRegistry::<T>::contains_key(currency_id),
				Error::<T>::CurrencyNotSupportCrossInAndOut
			);

			ensure!(
				!AccountToOuterMultilocation::<T>::contains_key(&currency_id, who.clone()),
				Error::<T>::AlreadyExist
			);

			AccountToOuterMultilocation::<T>::insert(
				currency_id,
				who.clone(),
				foreign_location.clone(),
			);
			OuterMultilocationToAccount::<T>::insert(
				currency_id,
				foreign_location.clone(),
				who.clone(),
			);

			Pallet::<T>::deposit_event(Event::LinkedAccountRegistered {
				currency_id,
				who,
				foreign_location: *foreign_location,
			});

			Ok(())
		}

		// Change originally registered linked outer chain multilocation
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::change_outer_linked_account())]
		pub fn change_outer_linked_account(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			foreign_location: Box<MultiLocation>,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			ensure!(
				CrossCurrencyRegistry::<T>::contains_key(currency_id),
				Error::<T>::CurrencyNotSupportCrossInAndOut
			);

			let original_location =
				AccountToOuterMultilocation::<T>::get(currency_id, account.clone())
					.ok_or(Error::<T>::NotExist)?;
			ensure!(original_location != *foreign_location.clone(), Error::<T>::AlreadyExist);

			AccountToOuterMultilocation::<T>::insert(
				currency_id,
				account.clone(),
				foreign_location.clone(),
			);
			OuterMultilocationToAccount::<T>::insert(
				currency_id,
				foreign_location.clone(),
				account.clone(),
			);

			Pallet::<T>::deposit_event(Event::LinkedAccountRegistered {
				currency_id,
				who: account,
				foreign_location: *foreign_location,
			});

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::deregister_currency_for_cross_in_out())]
		pub fn deregister_currency_for_cross_in_out(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if CrossCurrencyRegistry::<T>::take(currency_id).is_some() {
				Self::deposit_event(Event::CurrencyDeregistered { currency_id });
			};

			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::add_to_register_whitelist())]
		pub fn add_to_register_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if RegisterWhiteList::<T>::get(currency_id) == None {
				RegisterWhiteList::<T>::insert(currency_id, BoundedVec::default());
			}

			RegisterWhiteList::<T>::mutate(
				currency_id,
				|register_whitelist| -> Result<(), Error<T>> {
					match register_whitelist {
						Some(register_list) if !register_list.contains(&account) => {
							register_list
								.try_push(account.clone())
								.map_err(|_| Error::<T>::ListOverflow)?;
							Self::deposit_event(Event::AddedToRegisterList {
								account,
								currency_id,
							});
							Ok(())
						},
						_ => Err(Error::<T>::NotAllowed),
					}
				},
			)?;

			Ok(())
		}

		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::remove_from_register_whitelist())]
		pub fn remove_from_register_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			RegisterWhiteList::<T>::mutate(
				currency_id,
				|register_whitelist| -> Result<(), Error<T>> {
					match register_whitelist {
						Some(register_list) if register_list.contains(&account) => {
							register_list.retain(|x| x.clone() != account);
							Self::deposit_event(Event::RemovedFromRegisterList {
								account,
								currency_id,
							});
							Ok(())
						},
						_ => Err(Error::<T>::NotExist),
					}
				},
			)?;

			Ok(())
		}

		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::set_crossing_minimum_amount())]
		pub fn set_crossing_minimum_amount(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			cross_in_minimum: BalanceOf<T>,
			cross_out_minimum: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			CrossingMinimumAmount::<T>::insert(currency_id, (cross_in_minimum, cross_out_minimum));

			Self::deposit_event(Event::CrossingMinimumAmountSet {
				currency_id,
				cross_in_minimum,
				cross_out_minimum,
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn register_currency_for_cross_in_out(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			CrossCurrencyRegistry::<T>::mutate_exists(currency_id, |registration| {
				if registration.is_none() {
					*registration = Some(());
				}
			});

			Ok(())
		}
	}
}

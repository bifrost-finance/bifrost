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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(deprecated)] // TODO: clear transaction

// pub use crate::imbalances::{NegativeImbalance, PositiveImbalance};
extern crate alloc;

use alloc::vec::Vec;

use frame_support::{ensure, pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use node_primitives::CurrencyId;
use orml_traits::MultiCurrency;
pub use weights::WeightInfo;
use xcm::latest::MultiLocation;

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

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Currecny operation handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::Origin>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The balance is not enough
		NotEnoughBalance,
		/// The account doesn't exist in the whitelist.
		NotExist,
		/// The origin is not allowed to perform the operation.
		NotAllowed,
		CurrencyNotSupportCrossOut,
		NoMultilocationMapping,
		NoAccountIdMapping,
		AlreadyExist,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		CrossedOut {
			currency_id: CurrencyId,
			crosser: AccountIdOf<T>,
			location: MultiLocation,
			amount: BalanceOf<T>,
		},
		CrossedIn {
			currency_id: CurrencyId,
			dest: AccountIdOf<T>,
			location: MultiLocation,
			amount: BalanceOf<T>,
			remark: Option<Vec<u8>>,
		},
		CurrencyRegistered {
			currency_id: CurrencyId,
			operation: Option<()>,
		},
		AddedToIssueList {
			account: AccountIdOf<T>,
			currency_id: CurrencyId,
		},
		RemovedFromIssueList {
			account: AccountIdOf<T>,
			currency_id: CurrencyId,
		},
		LinkedAccountRegistered {
			currency_id: CurrencyId,
			who: AccountIdOf<T>,
			foreign_location: MultiLocation,
		},
		AddedToRegisterList {
			account: AccountIdOf<T>,
			currency_id: CurrencyId,
		},
		RemovedFromRegisterList {
			account: AccountIdOf<T>,
			currency_id: CurrencyId,
		},
	}

	/// To store currencies that support indirect cross-in and cross-out.
	#[pallet::storage]
	#[pallet::getter(fn get_cross_currency_registry)]
	pub type CrossCurrencyRegistry<T> = StorageMap<_, Blake2_128Concat, CurrencyId, ()>;

	/// Accounts in the whitelist can issue the corresponding Currency.
	#[pallet::storage]
	#[pallet::getter(fn get_issue_whitelist)]
	pub type IssueWhiteList<T> = StorageMap<_, Blake2_128Concat, CurrencyId, Vec<AccountIdOf<T>>>;

	/// Accounts in the whitelist can register the mapping between a multilocation and an accountId.
	#[pallet::storage]
	#[pallet::getter(fn get_register_whitelist)]
	pub type RegisterWhiteList<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, Vec<AccountIdOf<T>>>;

	/// Mapping a Bifrost account to a multilocation of a outer chain
	#[pallet::storage]
	#[pallet::getter(fn account_to_outer_multilocation)]
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
	#[pallet::getter(fn outer_multilocation_to_account)]
	pub type OuterMultilocationToAccount<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		AccountIdOf<T>,
		OptionQuery,
	>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::cross_in())]
		#[transactional]
		pub fn cross_in(
			origin: OriginFor<T>,
			location: Box<MultiLocation>,
			currency_id: CurrencyId,
			#[pallet::compact] amount: BalanceOf<T>,
			remark: Option<Vec<u8>>,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			let issue_whitelist =
				Self::get_issue_whitelist(currency_id).ok_or(Error::<T>::NotAllowed)?;
			ensure!(issue_whitelist.contains(&issuer), Error::<T>::NotAllowed);

			let dest = Self::outer_multilocation_to_account(currency_id, location.clone())
				.ok_or(Error::<T>::NoAccountIdMapping)?;

			T::MultiCurrency::deposit(currency_id, &dest, amount)?;

			Self::deposit_event(Event::CrossedIn {
				dest,
				currency_id,
				location: *location,
				amount,
				remark,
			});
			Ok(())
		}

		/// Destroy some balance from an account and issue cross-out event.
		#[pallet::weight(T::WeightInfo::cross_out())]
		#[transactional]
		pub fn cross_out(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let crosser = ensure_signed(origin)?;

			ensure!(
				CrossCurrencyRegistry::<T>::contains_key(currency_id),
				Error::<T>::CurrencyNotSupportCrossOut
			);

			let balance = T::MultiCurrency::free_balance(currency_id, &crosser);
			ensure!(balance >= amount, Error::<T>::NotEnoughBalance);

			let location = AccountToOuterMultilocation::<T>::get(currency_id, &crosser)
				.ok_or(Error::<T>::NoMultilocationMapping)?;

			T::MultiCurrency::withdraw(currency_id, &crosser, amount)?;

			Self::deposit_event(Event::CrossedOut { currency_id, crosser, location, amount });
			Ok(())
		}

		// Register the mapping relationship of Bifrost account and account from other chains
		#[pallet::weight(T::WeightInfo::register_linked_account())]
		pub fn register_linked_account(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: AccountIdOf<T>,
			foreign_location: Box<MultiLocation>,
		) -> DispatchResult {
			let registeror = ensure_signed(origin)?;

			let register_whitelist =
				Self::get_register_whitelist(currency_id).ok_or(Error::<T>::NotAllowed)?;
			ensure!(register_whitelist.contains(&registeror), Error::<T>::NotAllowed);

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

		#[pallet::weight(T::WeightInfo::register_currency_for_cross_in_out())]
		#[transactional]
		pub fn register_currency_for_cross_in_out(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			operation: Option<()>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			CrossCurrencyRegistry::<T>::mutate_exists(currency_id, |registration| {
				*registration = operation;
			});

			Self::deposit_event(Event::CurrencyRegistered { currency_id, operation });

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::add_to_issue_whitelist())]
		#[transactional]
		pub fn add_to_issue_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let empty_vec: Vec<AccountIdOf<T>> = Vec::new();
			if Self::get_issue_whitelist(currency_id) == None {
				IssueWhiteList::<T>::insert(currency_id, empty_vec);
			}

			IssueWhiteList::<T>::mutate(currency_id, |issue_whitelist| -> Result<(), Error<T>> {
				match issue_whitelist {
					Some(issue_list) if !issue_list.contains(&account) => {
						issue_list.push(account.clone());
						Self::deposit_event(Event::AddedToIssueList { account, currency_id });
						Ok(())
					},
					_ => Err(Error::<T>::NotAllowed),
				}
			})?;

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::remove_from_issue_whitelist())]
		#[transactional]
		pub fn remove_from_issue_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			IssueWhiteList::<T>::mutate(currency_id, |issue_whitelist| -> Result<(), Error<T>> {
				match issue_whitelist {
					Some(issue_list) if issue_list.contains(&account) => {
						issue_list.retain(|x| x.clone() != account);
						Self::deposit_event(Event::RemovedFromIssueList { account, currency_id });
						Ok(())
					},
					_ => Err(Error::<T>::NotExist),
				}
			})?;

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::add_to_register_whitelist())]
		#[transactional]
		pub fn add_to_register_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let empty_vec: Vec<AccountIdOf<T>> = Vec::new();
			if Self::get_register_whitelist(currency_id) == None {
				RegisterWhiteList::<T>::insert(currency_id, empty_vec);
			}

			RegisterWhiteList::<T>::mutate(
				currency_id,
				|register_whitelist| -> Result<(), Error<T>> {
					match register_whitelist {
						Some(register_list) if !register_list.contains(&account) => {
							register_list.push(account.clone());
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

		#[pallet::weight(T::WeightInfo::remove_from_register_whitelist())]
		#[transactional]
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
	}
}

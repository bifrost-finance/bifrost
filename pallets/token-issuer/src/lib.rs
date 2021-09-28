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
extern crate alloc;

use alloc::vec::Vec;

use frame_support::{ensure, pallet_prelude::*, transactional};
use frame_system::{pallet_prelude::*, WeightInfo};
use orml_traits::{
	currency::TransferAll, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency,
	MultiReservableCurrency,
};

mod mock;
mod tests;

pub use pallet::*;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

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
	}

	#[pallet::event]
	#[pallet::metadata(BalanceOf<T> = "Balance", CurrencyIdOf<T> = "CurrencyId")]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Successful added a new account to the issue whitelist. \[account, currency_id]\
		AddedToIssueList(T::AccountId, CurrencyIdOf<T>),
		/// Successful remove an account from the issue whitelist. \[account, currency_id]\
		RemovedFromIssueList(T::AccountId, CurrencyIdOf<T>),
		/// Successful added a new account to the transfer whitelist. \[account, currency_id]\
		AddedToTransferList(T::AccountId, CurrencyIdOf<T>),
		/// Successful remove an account from the transfer whitelist. \[account, currency_id]\
		RemovedFromTransferList(T::AccountId, CurrencyIdOf<T>),
		/// Token issue success, \[currency_id, dest, amount\]
		Issued(T::AccountId, CurrencyIdOf<T>, BalanceOf<T>),
		/// Token transferred success, \[origin, dest, currency_id, amount\]
		Transferred(T::AccountId, T::AccountId, CurrencyIdOf<T>, BalanceOf<T>),
	}

	/// Accounts in the whitelist can issue the corresponding Currency.
	#[pallet::storage]
	#[pallet::getter(fn get_issue_whitelist)]
	pub type IssueWhiteList<T> =
		StorageMap<_, Blake2_128Concat, CurrencyIdOf<T>, Vec<AccountIdOf<T>>>;

	/// Accounts in the whitelist can transfer the corresponding Currency.
	#[pallet::storage]
	#[pallet::getter(fn get_transfer_whitelist)]
	pub type TransferWhiteList<T> =
		StorageMap<_, Blake2_128Concat, CurrencyIdOf<T>, Vec<AccountIdOf<T>>>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000)]
		#[transactional]
		pub fn add_to_issue_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
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
						Self::deposit_event(Event::AddedToIssueList(account, currency_id));
						Ok(())
					},
					_ => Err(Error::<T>::NotAllowed),
				}
			})?;

			Ok(())
		}

		#[pallet::weight(1000)]
		#[transactional]
		pub fn remove_from_issue_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			IssueWhiteList::<T>::mutate(currency_id, |issue_whitelist| -> Result<(), Error<T>> {
				match issue_whitelist {
					Some(issue_list) if issue_list.contains(&account) => {
						issue_list.retain(|x| x.clone() != account);
						Self::deposit_event(Event::RemovedFromIssueList(account, currency_id));
						Ok(())
					},
					_ => Err(Error::<T>::NotExist),
				}
			})?;

			Ok(())
		}

		#[pallet::weight(1000)]
		#[transactional]
		pub fn add_to_transfer_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let empty_vec: Vec<AccountIdOf<T>> = Vec::new();
			if Self::get_transfer_whitelist(currency_id) == None {
				TransferWhiteList::<T>::insert(currency_id, empty_vec);
			}

			TransferWhiteList::<T>::mutate(
				currency_id,
				|transfer_whitelist| -> Result<(), Error<T>> {
					match transfer_whitelist {
						Some(transfer_list) if !transfer_list.contains(&account) => {
							transfer_list.push(account.clone());
							Self::deposit_event(Event::AddedToTransferList(account, currency_id));
							Ok(())
						},
						_ => Err(Error::<T>::NotAllowed),
					}
				},
			)?;

			Ok(())
		}

		#[pallet::weight(1000)]
		#[transactional]
		pub fn remove_from_transfer_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			account: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			TransferWhiteList::<T>::mutate(
				currency_id,
				|transfer_whitelist| -> Result<(), Error<T>> {
					match transfer_whitelist {
						Some(transfer_list) if transfer_list.contains(&account) => {
							transfer_list.retain(|x| x.clone() != account);
							Self::deposit_event(Event::RemovedFromTransferList(
								account,
								currency_id,
							));
							Ok(())
						},
						_ => Err(Error::<T>::NotExist),
					}
				},
			)?;

			Ok(())
		}

		#[pallet::weight(1000)]
		#[transactional]
		pub fn issue(
			origin: OriginFor<T>,
			dest: AccountIdOf<T>,
			currency_id: CurrencyIdOf<T>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			let issue_whitelist =
				Self::get_issue_whitelist(currency_id).ok_or(Error::<T>::NotAllowed)?;
			ensure!(issue_whitelist.contains(&issuer), Error::<T>::NotAllowed);

			T::MultiCurrency::deposit(currency_id, &dest, amount)?;

			Self::deposit_event(Event::Issued(dest, currency_id, amount));
			Ok(())
		}

		/// Destroy some balance from an account.
		///
		/// The dispatch origin for this call must be `Root` by the
		/// transactor.
		#[pallet::weight(1000)]
		#[transactional]
		pub fn transfer(
			origin: OriginFor<T>,
			dest: AccountIdOf<T>,
			currency_id: CurrencyIdOf<T>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let issuer = ensure_signed(origin)?;

			let transfer_whitelist =
				Self::get_transfer_whitelist(currency_id).ok_or(Error::<T>::NotAllowed)?;
			ensure!(transfer_whitelist.contains(&issuer), Error::<T>::NotAllowed);

			let balance = T::MultiCurrency::free_balance(currency_id, &issuer);
			ensure!(balance >= amount, Error::<T>::NotEnoughBalance);

			T::MultiCurrency::transfer(currency_id, &issuer, &dest, amount)?;

			Self::deposit_event(Event::Transferred(issuer, dest, currency_id, amount));
			Ok(())
		}
	}
}

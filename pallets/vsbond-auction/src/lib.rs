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

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use orml_traits::{
	MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency,
};
use sp_std::collections::btree_set::BTreeSet;

mod mock;
mod tests;

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct OrderInfo<T: Config> {
	owner: AccountIdOf<T>,
	currency_sold: CurrencyIdOf<T>,
	amount_sold: BalanceOf<T>,
	currency_expected: CurrencyIdOf<T>,
	amount_expected: BalanceOf<T>,
	order_id: OrderId,
	order_state: OrderState,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq)]
pub enum OrderState {
	InTrade,
	Revoked,
	Clinchd,
}

pub type OrderId = u64;

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type MaximumOrderInTrade: Get<u32>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>>
			+ MultiCurrencyExtended<AccountIdOf<Self>>
			+ MultiLockableCurrency<AccountIdOf<Self>>
			+ MultiReservableCurrency<AccountIdOf<Self>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughCurrencySold,
		NotEnoughCurrencyExpected,
		NotFindOrderInfo,
		ForbidCreateOrderWithSameCurrency,
		ForbidRevokeOrderNotInTrade,
		ForbidRevokeOrderWithoutOwnership,
		ForbidClinchOrderNotInTrade,
		ForbidClinchOrderWithinOwnership,
		ExceedMaximumOrderInTrade,
		Unexpected,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The order has been created.
		///
		/// [order_id, order_owner, currency_sold, amount_sold, currency_expected, amount_expected]
		OrderCreated(
			OrderId,
			AccountIdOf<T>,
			CurrencyIdOf<T>,
			BalanceOf<T>,
			CurrencyIdOf<T>,
			BalanceOf<T>,
		),
		/// The order has been revoked.
		///
		/// [order_id_revoked, order_owner]
		OrderRevoked(OrderId, AccountIdOf<T>),
		/// The order has been clinched.
		///
		/// [order_id_clinched, order_owner, order_buyer]
		OrderClinchd(OrderId, AccountIdOf<T>, AccountIdOf<T>),
	}

	#[pallet::storage]
	pub type NextOrderId<T: Config> = StorageValue<_, OrderId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn in_trade_order_ids)]
	pub type InTradeOrderIds<T: Config> =
		StorageMap<_, Twox64Concat, AccountIdOf<T>, BTreeSet<OrderId>>;

	#[pallet::storage]
	#[pallet::getter(fn revoked_order_ids)]
	pub type RevokedOrderIds<T: Config> =
		StorageMap<_, Twox64Concat, AccountIdOf<T>, BTreeSet<OrderId>>;

	#[pallet::storage]
	#[pallet::getter(fn clinchd_order_ids)]
	pub type ClinchdOrderIds<T: Config> =
		StorageMap<_, Twox64Concat, AccountIdOf<T>, BTreeSet<OrderId>>;

	#[pallet::storage]
	#[pallet::getter(fn order)]
	pub type TotalOrders<T: Config> = StorageMap<_, Twox64Concat, OrderId, OrderInfo<T>>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_order(
			origin: OriginFor<T>,
			currency_sold: CurrencyIdOf<T>,
			#[pallet::compact] amount_sold: BalanceOf<T>,
			currency_expected: CurrencyIdOf<T>,
			#[pallet::compact] amount_expected: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let owner = ensure_signed(origin)?;

			// TODO: Check `currency_sold` is `CurrencyId::VSBond(..)`

			// Check assets
			T::MultiCurrency::ensure_can_withdraw(currency_sold, &owner, amount_sold)
				.map_err(|_| Error::<T>::NotEnoughCurrencySold)?;

			ensure!(
				currency_sold != currency_expected,
				Error::<T>::ForbidCreateOrderWithSameCurrency
			);

			let pending_order_count = {
				if let Some(sets) = Self::in_trade_order_ids(&owner) {
					sets.len() as u32
				} else {
					0
				}
			};
			ensure!(
				pending_order_count < T::MaximumOrderInTrade::get(),
				Error::<T>::ExceedMaximumOrderInTrade,
			);

			// Create order
			let order_id = Self::next_order_id();
			let order_info = OrderInfo::<T> {
				owner: owner.clone(),
				currency_sold,
				amount_sold,
				currency_expected,
				amount_expected,
				order_id,
				order_state: OrderState::InTrade,
			};

			// Lock the balance of currency_sold
			let lock_iden = order_id.to_be_bytes();
			T::MultiCurrency::set_lock(lock_iden, currency_sold, &owner, amount_sold)?;

			TotalOrders::<T>::insert(order_id, order_info);

			if !InTradeOrderIds::<T>::contains_key(&owner) {
				InTradeOrderIds::<T>::insert(owner.clone(), BTreeSet::<OrderId>::new());
			}
			Self::in_trade_order_ids(&owner)
				.ok_or(Error::<T>::Unexpected)?
				.insert(order_id);

			Self::deposit_event(Event::OrderCreated(
				order_id,
				owner,
				currency_sold,
				amount_sold,
				currency_expected,
				amount_expected,
			));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn revoke_order(
			origin: OriginFor<T>,
			#[pallet::compact] order_id: OrderId,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let from = ensure_signed(origin)?;

			// Check order
			let order_info = Self::order(order_id).ok_or(Error::<T>::NotFindOrderInfo)?;

			// Check order state
			ensure!(
				order_info.order_state == OrderState::InTrade,
				Error::<T>::ForbidRevokeOrderNotInTrade
			);

			// Check order owner
			ensure!(
				order_info.owner == from,
				Error::<T>::ForbidRevokeOrderWithoutOwnership
			);

			// Unlock the balance of currency_sold
			let lock_iden = order_info.order_id.to_be_bytes();
			T::MultiCurrency::remove_lock(lock_iden, order_info.currency_sold, &from)?;

			// Revoke order
			TotalOrders::<T>::insert(
				order_id,
				OrderInfo {
					order_state: OrderState::Revoked,
					..order_info
				},
			);

			// Move order_id from `InTrade` to `Revoked`.
			Self::in_trade_order_ids(&from)
				.ok_or(Error::<T>::Unexpected)?
				.remove(&order_id);
			if !RevokedOrderIds::<T>::contains_key(&from) {
				RevokedOrderIds::<T>::insert(from.clone(), BTreeSet::<OrderId>::new());
			}
			Self::revoked_order_ids(&from)
				.ok_or(Error::<T>::Unexpected)?
				.insert(order_id);

			Self::deposit_event(Event::OrderRevoked(order_id, from));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn clinch_order(
			origin: OriginFor<T>,
			#[pallet::compact] order_id: OrderId,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let buyer = ensure_signed(origin)?;

			// Check order
			let order_info = Self::order(order_id).ok_or(Error::<T>::NotFindOrderInfo)?;

			// Check order state
			ensure!(
				order_info.order_state == OrderState::InTrade,
				Error::<T>::ForbidClinchOrderNotInTrade
			);

			// Check order owner
			ensure!(
				order_info.owner != buyer,
				Error::<T>::ForbidClinchOrderWithinOwnership
			);

			// Check the balance of currency
			T::MultiCurrency::ensure_can_withdraw(
				order_info.currency_expected,
				&buyer,
				order_info.amount_expected,
			)
			.map_err(|_| Error::<T>::NotEnoughCurrencyExpected)?;

			// Unlock the balance of currency_sold
			let lock_iden = order_info.order_id.to_be_bytes();
			T::MultiCurrency::remove_lock(lock_iden, order_info.currency_sold, &order_info.owner)?;

			// Exchange assets
			T::MultiCurrency::transfer(
				order_info.currency_sold,
				&order_info.owner,
				&buyer,
				order_info.amount_sold,
			)?;
			T::MultiCurrency::transfer(
				order_info.currency_expected,
				&buyer,
				&order_info.owner,
				order_info.amount_expected,
			)?;

			let owner = order_info.owner.clone();
			// Clinch order
			TotalOrders::<T>::insert(
				order_id,
				OrderInfo {
					order_state: OrderState::Clinchd,
					..order_info
				},
			);

			// Move order_id from `InTrade` to `Clinchd`.
			Self::in_trade_order_ids(&owner)
				.ok_or(Error::<T>::Unexpected)?
				.remove(&order_id);
			if !ClinchdOrderIds::<T>::contains_key(&owner) {
				ClinchdOrderIds::<T>::insert(owner.clone(), BTreeSet::<OrderId>::new());
			}
			Self::clinchd_order_ids(&owner)
				.ok_or(Error::<T>::Unexpected)?
				.insert(order_id);

			Self::deposit_event(Event::<T>::OrderClinchd(order_id, owner, buyer));

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn next_order_id() -> OrderId {
		let next_order_id = NextOrderId::<T>::get();
		NextOrderId::<T>::mutate(|current| *current + 1);
		next_order_id
	}
}

// TODO: Maybe impl Auction trait for vsbond-auction

#[allow(type_alias_bounds)]
type AccountIdOf<T: Config> = <T as frame_system::Config>::AccountId;
#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;
#[allow(type_alias_bounds)]
type CurrencyIdOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::CurrencyId;

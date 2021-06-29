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

use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::{CheckedMul, Saturating, Zero},
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, LeasePeriod};
use orml_traits::{
	MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency,
};
use sp_std::{cmp::min, collections::btree_set::BTreeSet};

mod mock;
mod tests;

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct OrderInfo<T: Config> {
	/// The owner of the order
	owner: AccountIdOf<T>,
	/// The vsbond type of the order to sell
	vsbond: CurrencyId,
	/// The quantity of vsbond to sell
	supply: BalanceOf<T>,
	/// The quantity of vsbond has not be sold
	remain: BalanceOf<T>,
	unit_price: BalanceOf<T>,
	order_id: OrderId,
	order_state: OrderState,
}

impl<T: Config> core::fmt::Debug for OrderInfo<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_tuple("")
			.field(&self.owner)
			.field(&self.vsbond)
			.field(&self.supply)
			.field(&self.unit_price)
			.field(&self.order_id)
			.field(&self.order_state)
			.finish()
	}
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Debug)]
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
	pub trait Config: frame_system::Config<BlockNumber = LeasePeriod> {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The currency type that buyer to pay
		#[pallet::constant]
		type InvoicingCurrency: Get<CurrencyId>;

		/// The amount of orders in-trade that user can hold
		#[pallet::constant]
		type MaximumOrderInTrade: Get<u32>;

		/// The sale quantity needs to be greater than `MinimumSupply` to create an order
		#[pallet::constant]
		type MinimumSupply: Get<BalanceOf<Self>>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiCurrencyExtended<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughSupply,
		NotFindOrderInfo,
		ForbidRevokeOrderNotInTrade,
		ForbidRevokeOrderWithoutOwnership,
		ForbidClinchOrderNotInTrade,
		ForbidClinchOrderWithinOwnership,
		ExceedMaximumOrderInTrade,
		Overflow,
		Unexpected,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The order has been created.
		///
		/// [order_id, order_info]
		OrderCreated(OrderId, OrderInfo<T>),
		/// The order has been revoked.
		///
		/// [order_id_revoked, order_owner]
		OrderRevoked(OrderId, AccountIdOf<T>),
		/// The order has been clinched.
		///
		/// [order_id_clinched, order_owner, order_buyer, quantity]
		OrderClinchd(OrderId, AccountIdOf<T>, AccountIdOf<T>, BalanceOf<T>),
	}

	#[pallet::storage]
	#[pallet::getter(fn order_id)]
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
	#[pallet::getter(fn order_info)]
	pub type TotalOrderInfos<T: Config> = StorageMap<_, Twox64Concat, OrderId, OrderInfo<T>>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_order(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] first_slot: LeasePeriodOf<T>,
			#[pallet::compact] last_slot: LeasePeriodOf<T>,
			#[pallet::compact] supply: BalanceOf<T>,
			#[pallet::compact] unit_price: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let owner = ensure_signed(origin)?;

			// Check supply
			ensure!(supply > T::MinimumSupply::get(), Error::<T>::NotEnoughSupply);

			// Construct vsbond
			let vsbond =
				CurrencyId::VSBond(*T::InvoicingCurrency::get(), index, first_slot, last_slot);

			// Check the balance of vsbond
			T::MultiCurrency::ensure_can_withdraw(vsbond, &owner, supply)?;

			let order_in_trade_amount = {
				if let Some(sets) = Self::in_trade_order_ids(&owner) {
					sets.len() as u32
				} else {
					0
				}
			};
			ensure!(
				order_in_trade_amount < T::MaximumOrderInTrade::get(),
				Error::<T>::ExceedMaximumOrderInTrade,
			);

			// Create OrderInfo
			let order_id = Self::next_order_id();
			let order_info = OrderInfo::<T> {
				owner: owner.clone(),
				vsbond,
				supply,
				remain: supply,
				unit_price,
				order_id,
				order_state: OrderState::InTrade,
			};

			// Lock the balance of vsbond_type
			let lock_iden = order_id.to_be_bytes();
			T::MultiCurrency::set_lock(lock_iden, vsbond, &owner, supply)?;

			// Insert OrderInfo to Storage
			TotalOrderInfos::<T>::insert(order_id, order_info.clone());

			// Add order_id to the order_ids in-trade of account
			if !InTradeOrderIds::<T>::contains_key(&owner) {
				InTradeOrderIds::<T>::insert(owner.clone(), BTreeSet::<OrderId>::new());
			}
			InTradeOrderIds::<T>::try_mutate(owner.clone(), |list| match list {
				Some(list) => {
					list.insert(order_id);
					Ok(())
				}
				None => Err(Error::<T>::Unexpected),
			})?;

			Self::deposit_event(Event::OrderCreated(order_id, order_info));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn revoke_order(
			origin: OriginFor<T>,
			#[pallet::compact] order_id: OrderId,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let from = ensure_signed(origin)?;

			// Check OrderInfo
			let order_info = Self::order_info(order_id).ok_or(Error::<T>::NotFindOrderInfo)?;

			// Check OrderState
			ensure!(
				order_info.order_state == OrderState::InTrade,
				Error::<T>::ForbidRevokeOrderNotInTrade
			);

			// Check OrderOwner
			ensure!(order_info.owner == from, Error::<T>::ForbidRevokeOrderWithoutOwnership);

			// Unlock the vsbond
			let lock_iden = order_info.order_id.to_be_bytes();
			T::MultiCurrency::remove_lock(lock_iden, order_info.vsbond, &from)?;

			// Revoke order
			TotalOrderInfos::<T>::insert(
				order_id,
				OrderInfo { order_state: OrderState::Revoked, ..order_info },
			);

			// Move order_id from `InTrade` to `Revoked`.
			InTradeOrderIds::<T>::try_mutate(from.clone(), |list| match list {
				Some(list) => {
					list.remove(&order_id);
					Ok(())
				}
				None => Err(Error::<T>::Unexpected),
			})?;
			if !RevokedOrderIds::<T>::contains_key(&from) {
				RevokedOrderIds::<T>::insert(from.clone(), BTreeSet::<OrderId>::new());
			}
			RevokedOrderIds::<T>::try_mutate(from.clone(), |list| match list {
				Some(list) => {
					list.insert(order_id);
					Ok(())
				}
				None => Err(Error::<T>::Unexpected),
			})?;

			Self::deposit_event(Event::OrderRevoked(order_id, from));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn clinch_order(
			origin: OriginFor<T>,
			#[pallet::compact] order_id: OrderId,
		) -> DispatchResultWithPostInfo {
			let order_info = Self::order_info(order_id).ok_or(Error::<T>::NotFindOrderInfo)?;

			// Check OrderState
			ensure!(
				order_info.order_state == OrderState::InTrade,
				Error::<T>::ForbidClinchOrderNotInTrade
			);

			Self::partial_clinch_order(origin, order_id, order_info.remain)?;

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn partial_clinch_order(
			origin: OriginFor<T>,
			#[pallet::compact] order_id: OrderId,
			#[pallet::compact] quantity: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check Zero
			if quantity.is_zero() {
				return Ok(().into());
			}

			// Check origin
			let buyer = ensure_signed(origin)?;

			// Check OrderInfo
			let order_info = Self::order_info(order_id).ok_or(Error::<T>::NotFindOrderInfo)?;

			// Check OrderState
			ensure!(
				order_info.order_state == OrderState::InTrade,
				Error::<T>::ForbidClinchOrderNotInTrade
			);

			// Check OrderOwner
			ensure!(order_info.owner != buyer, Error::<T>::ForbidClinchOrderWithinOwnership);

			// Calculate the real quantity to clinch
			let quantity_clinchd = min(order_info.remain, quantity);
			// Calculate the total price that buyer need to pay
			let total_price = quantity_clinchd
				.checked_mul(&order_info.unit_price)
				.ok_or(Error::<T>::Overflow)?;

			// Get the new OrderInfo
			let new_order_info = if quantity_clinchd == order_info.remain {
				OrderInfo { remain: Zero::zero(), order_state: OrderState::Clinchd, ..order_info }
			} else {
				OrderInfo {
					remain: order_info.remain.saturating_sub(quantity_clinchd),
					..order_info
				}
			};

			// Decrease the locked amount of vsbond
			let lock_iden = order_id.to_be_bytes();
			T::MultiCurrency::remove_lock(lock_iden, new_order_info.vsbond, &new_order_info.owner)?;
			T::MultiCurrency::set_lock(
				lock_iden,
				new_order_info.vsbond,
				&new_order_info.owner,
				new_order_info.remain,
			)?;

			// TODO: Maybe fail if double lock?
			// Exchange: Transfer vsbond from owner to buyer
			T::MultiCurrency::transfer(
				new_order_info.vsbond,
				&new_order_info.owner,
				&buyer,
				quantity_clinchd,
			)?;
			// Exchange: Transfer token from buyer to owner
			T::MultiCurrency::transfer(
				T::InvoicingCurrency::get(),
				&buyer,
				&new_order_info.owner,
				total_price,
			)?;

			// Move order_id from InTrade to Clinchd if meets condition
			if new_order_info.order_state == OrderState::Clinchd {
				InTradeOrderIds::<T>::try_mutate(
					new_order_info.owner.clone(),
					|list| match list {
						Some(list) => {
							list.remove(&order_id);
							Ok(())
						}
						None => Err(Error::<T>::Unexpected),
					},
				)?;
				if !ClinchdOrderIds::<T>::contains_key(&new_order_info.owner) {
					ClinchdOrderIds::<T>::insert(
						new_order_info.owner.clone(),
						BTreeSet::<OrderId>::new(),
					);
				}
				ClinchdOrderIds::<T>::try_mutate(
					new_order_info.owner.clone(),
					|list| match list {
						Some(list) => {
							list.insert(order_id);
							Ok(())
						}
						None => Err(Error::<T>::Unexpected),
					},
				)?;
			}
			// Change the OrderInfo in Storage
			TotalOrderInfos::<T>::insert(order_id, new_order_info.clone());

			Self::deposit_event(Event::<T>::OrderClinchd(
				order_id,
				new_order_info.owner,
				buyer,
				quantity_clinchd,
			));

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn next_order_id() -> OrderId {
		let next_order_id = Self::order_id();
		NextOrderId::<T>::mutate(|current| *current += 1);
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
type LeasePeriodOf<T: Config> = <T as frame_system::Config>::BlockNumber;
type ParaId = u32;

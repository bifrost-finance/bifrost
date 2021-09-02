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

//! The pallet supports the trading functions of `vsbond`.
//!
//! Users can create sell orders by `create_order`;
//! Or buy the sell orders by `clinch_order`, `partial_clinch_order`.
//!
//! NOTE: Pallet does not support users creating buy orders by now.

use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::{SaturatedConversion, Saturating, Zero},
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, LeasePeriod, TokenInfo, TokenSymbol};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
pub use pallet::*;
use sp_std::{cmp::min, collections::btree_set::BTreeSet, convert::TryFrom};
use substrate_fixed::{traits::FromFixed, types::U64F64};
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

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
	/// Total price of the order
	total_price: BalanceOf<T>,
	/// The unique id of the order
	order_id: OrderId,
	order_state: OrderState,
}

impl<T: Config> OrderInfo<T> {
	pub fn unit_price(&self) -> U64F64 {
		let supply: u128 = self.supply.saturated_into();
		let total_price: u128 = self.total_price.saturated_into();

		U64F64::from_num(total_price) / supply
	}
}

impl<T: Config> core::fmt::Debug for OrderInfo<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_tuple("")
			.field(&self.owner)
			.field(&self.vsbond)
			.field(&self.supply)
			.field(&self.unit_price())
			.field(&self.order_id)
			.field(&self.order_state)
			.finish()
	}
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Debug)]
enum OrderState {
	InTrade,
	Revoked,
	Clinchd,
}

type OrderId = u64;
type ParaId = u32;

#[allow(type_alias_bounds)]
type AccountIdOf<T: Config> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[allow(type_alias_bounds)]
type LeasePeriodOf<T: Config> = <T as frame_system::Config>::BlockNumber;

#[frame_support::pallet]
pub mod pallet {
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
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// Set default weight.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughSupply,
		NotFindOrderInfo,
		NotEnoughBalanceToUnreserve,
		NotEnoughBalanceToReserve,
		CantPayThePrice,
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
	pub(crate) type NextOrderId<T: Config> = StorageValue<_, OrderId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn in_trade_order_ids)]
	pub(crate) type InTradeOrderIds<T: Config> =
		StorageMap<_, Twox64Concat, AccountIdOf<T>, BTreeSet<OrderId>>;

	#[pallet::storage]
	#[pallet::getter(fn revoked_order_ids)]
	pub(crate) type RevokedOrderIds<T: Config> =
		StorageMap<_, Twox64Concat, AccountIdOf<T>, BTreeSet<OrderId>>;

	#[pallet::storage]
	#[pallet::getter(fn clinchd_order_ids)]
	pub(crate) type ClinchdOrderIds<T: Config> =
		StorageMap<_, Twox64Concat, AccountIdOf<T>, BTreeSet<OrderId>>;

	#[pallet::storage]
	#[pallet::getter(fn order_info)]
	pub(crate) type TotalOrderInfos<T: Config> = StorageMap<_, Twox64Concat, OrderId, OrderInfo<T>>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a sell order to sell `vsbond`.
		#[pallet::weight(1_000)]
		pub fn create_order(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			#[pallet::compact] first_slot: LeasePeriodOf<T>,
			#[pallet::compact] last_slot: LeasePeriodOf<T>,
			#[pallet::compact] supply: BalanceOf<T>,
			#[pallet::compact] total_price: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let owner = ensure_signed(origin)?;

			// Check supply
			ensure!(supply > T::MinimumSupply::get(), Error::<T>::NotEnoughSupply);

			let currency_id_u64: u64 = T::InvoicingCurrency::get().currency_id();
			let tokensymbo_bit = (currency_id_u64 & 0x0000_0000_0000_00ff) as u8;
			let currency_tokensymbol =
				TokenSymbol::try_from(tokensymbo_bit).map_err(|_| Error::<T>::Unexpected)?;

			// Construct vsbond
			let vsbond = CurrencyId::VSBond(currency_tokensymbol, index, first_slot, last_slot);

			// Check the balance of vsbond
			ensure!(
				T::MultiCurrency::can_reserve(vsbond, &owner, supply),
				Error::<T>::NotEnoughBalanceToReserve
			);

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
				total_price,
				order_id,
				order_state: OrderState::InTrade,
			};

			// Reserve the balance of vsbond_type
			T::MultiCurrency::reserve(vsbond, &owner, supply)?;

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
				},
				None => Err(Error::<T>::Unexpected),
			})?;

			Self::deposit_event(Event::OrderCreated(order_id, order_info));

			Ok(().into())
		}

		/// Revoke a sell order in trade by the order creator.
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

			// Unreserve the vsbond
			let reserved_balance =
				T::MultiCurrency::reserved_balance(order_info.vsbond, &order_info.owner);
			ensure!(reserved_balance >= order_info.remain, Error::<T>::NotEnoughBalanceToUnreserve);
			T::MultiCurrency::unreserve(order_info.vsbond, &order_info.owner, order_info.remain);

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
				},
				None => Err(Error::<T>::Unexpected),
			})?;
			if !RevokedOrderIds::<T>::contains_key(&from) {
				RevokedOrderIds::<T>::insert(from.clone(), BTreeSet::<OrderId>::new());
			}
			RevokedOrderIds::<T>::try_mutate(from.clone(), |list| match list {
				Some(list) => {
					list.insert(order_id);
					Ok(())
				},
				None => Err(Error::<T>::Unexpected),
			})?;

			Self::deposit_event(Event::OrderRevoked(order_id, from));

			Ok(().into())
		}

		/// Users(non-order-creator) buy the remaining `vsbond` of a sell order.
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

		/// Users(non-order-creator) buys some of the remaining `vsbond` of a sell order.
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
			let price_to_pay = Self::price_to_pay(quantity_clinchd, order_info.unit_price());

			// Check the balance of buyer
			T::MultiCurrency::ensure_can_withdraw(
				T::InvoicingCurrency::get(),
				&buyer,
				price_to_pay,
			)
			.map_err(|_| Error::<T>::CantPayThePrice)?;

			// Get the new OrderInfo
			let new_order_info = if quantity_clinchd == order_info.remain {
				OrderInfo { remain: Zero::zero(), order_state: OrderState::Clinchd, ..order_info }
			} else {
				OrderInfo {
					remain: order_info.remain.saturating_sub(quantity_clinchd),
					..order_info
				}
			};

			// Unreserve the balance of vsbond to transfer
			let reserved_balance =
				T::MultiCurrency::reserved_balance(new_order_info.vsbond, &new_order_info.owner);
			ensure!(reserved_balance >= quantity_clinchd, Error::<T>::NotEnoughBalanceToUnreserve);
			T::MultiCurrency::unreserve(
				new_order_info.vsbond,
				&new_order_info.owner,
				quantity_clinchd,
			);

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
				price_to_pay,
			)?;

			// Move order_id from InTrade to Clinchd if meets condition
			if new_order_info.order_state == OrderState::Clinchd {
				InTradeOrderIds::<T>::try_mutate(
					new_order_info.owner.clone(),
					|list| match list {
						Some(list) => {
							list.remove(&order_id);
							Ok(())
						},
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
						},
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

	impl<T: Config> Pallet<T> {
		pub(crate) fn next_order_id() -> OrderId {
			let next_order_id = Self::order_id();
			NextOrderId::<T>::mutate(|current| *current += 1);
			next_order_id
		}

		/// Get the price(round up) needed to pay.
		pub(crate) fn price_to_pay(quantity: BalanceOf<T>, unit_price: U64F64) -> BalanceOf<T> {
			let quantity: u128 = quantity.saturated_into();
			let total_price = u128::from_fixed((unit_price * quantity).ceil());

			BalanceOf::<T>::saturated_from(total_price)
		}
	}
}

// TODO: Maybe impl Auction trait for vsbond-auction

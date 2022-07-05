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

//! The pallet supports the trading functions of `vsbond`.
//!
//! Users can create sell orders by `create_order`;
//! Or buy the sell orders by `clinch_order`, `partial_clinch_order`.
//!
//! NOTE: Pallet does not support users creating buy orders by now.

use core::fmt::Debug;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{
			AccountIdConversion, AtLeast32BitUnsigned, SaturatedConversion, Saturating, Zero,
		},
		FixedPointNumber, FixedU128,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, LeasePeriod, ParaId, TokenSymbol};
use orml_traits::{MultiCurrency, MultiReservableCurrency};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_arithmetic::{
	per_things::Permill,
	traits::{CheckedAdd, CheckedSub},
};
use sp_std::cmp::min;
pub use weights::WeightInfo;

pub mod migration;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo)]
pub struct OrderInfo<AccountIdOf, BalanceOf>
where
	AccountIdOf: Debug,
	BalanceOf: Debug + AtLeast32BitUnsigned,
{
	/// The owner of the order
	owner: AccountIdOf,
	/// The vsbond type of the order to sell
	vsbond: CurrencyId,
	/// The quantity of vsbond to sell or buy
	amount: BalanceOf,
	/// The quantity of vsbond has not be sold or took
	remain: BalanceOf,
	/// Total price of the order
	total_price: BalanceOf,
	/// Helper to calculate the remain to unreserve.
	/// Useful for buy order, it is the amount that has not been spent yet.
	remain_price: BalanceOf,
	/// The unique id of the order
	order_id: OrderId,
	order_type: OrderType,
}

impl<AccountIdOf, BalanceOf> OrderInfo<AccountIdOf, BalanceOf>
where
	AccountIdOf: Debug,
	BalanceOf: Debug + AtLeast32BitUnsigned + Copy,
{
	pub fn unit_price(&self) -> FixedU128 {
		let amount: u128 = self.amount.saturated_into();
		let total_price: u128 = self.total_price.saturated_into();

		match amount {
			0 => 0.into(),
			_ => FixedU128::from((total_price, amount)),
		}
	}
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Debug, TypeInfo)]
pub enum OrderType {
	Sell,
	Buy,
}

type OrderId = u64;

#[allow(type_alias_bounds)]
type AccountIdOf<T: Config> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config<I>, I: 'static = ()> =
	<<T as Config<I>>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[allow(type_alias_bounds)]
type LeasePeriodOf<T: Config> = <T as frame_system::Config>::BlockNumber;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config<BlockNumber = LeasePeriod> {
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		/// The currency type that buyer to pay
		#[pallet::constant]
		type InvoicingCurrency: Get<CurrencyId>;

		/// The amount of orders in-trade that user can hold
		#[pallet::constant]
		type MaximumOrderInTrade: Get<u32>;

		/// The sale or buy quantity needs to be greater than `MinimumSupply` to create an order
		#[pallet::constant]
		type MinimumAmount: Get<BalanceOf<Self, I>>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// Set default weight.
		type WeightInfo: WeightInfo;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The account that transaction fees go into
		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		/// The only origin that can modify transaction fee rate
		type ControlOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		NotEnoughAmount,
		NotFindOrderInfo,
		NotEnoughBalanceToCreateOrder,
		DontHaveEnoughToPay,
		ForbidRevokeOrderNotInTrade,
		ForbidRevokeOrderWithoutOwnership,
		ForbidClinchOrderNotInTrade,
		ForbidClinchOrderWithinOwnership,
		ExceedMaximumOrderInTrade,
		InvalidVsbond,
		Unexpected,
		InvalidRateInput,
		Overflow,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// The order has been created.
		///
		/// [order_id, order_type, order_creator, vsbond_type, vsbond_amount, total_price]
		OrderCreated(
			OrderId,
			OrderType,
			AccountIdOf<T>,
			CurrencyId,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
		),
		/// The order has been revoked.
		///
		/// [order_id, order_type, order_creator, vsbond_type, vsbond_amount, vsbond_remain,
		/// total_price]
		OrderRevoked(
			OrderId,
			OrderType,
			AccountIdOf<T>,
			CurrencyId,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
		),
		/// The order has been clinched.
		///
		/// [order_id, order_type, order_creator, order_opponent, vsbond_type,
		/// vsbond_amount_clinched, vsbond_amount, vsbond_remain, total_price]
		OrderClinchd(
			OrderId,
			OrderType,
			AccountIdOf<T>,
			AccountIdOf<T>,
			CurrencyId,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
		),
		/// Transaction fee rate has been reset.
		///
		/// [buy_fee_rate, sell_fee_rate]
		TransactionFeeRateSet(Permill, Permill),
	}

	#[pallet::storage]
	#[pallet::getter(fn order_id)]
	pub(crate) type NextOrderId<T: Config<I>, I: 'static = ()> =
		StorageValue<_, OrderId, ValueQuery>;

	// Just store order ids that be in-trade.
	#[pallet::storage]
	#[pallet::getter(fn user_order_ids)]
	pub(crate) type UserOrderIds<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		OrderType,
		BoundedVec<OrderId, T::MaximumOrderInTrade>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn order_info)]
	pub type TotalOrderInfos<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Blake2_128Concat, OrderId, OrderInfo<AccountIdOf<T>, BalanceOf<T, I>>>;

	/// transaction fee rate[sellFee, buyFee]
	#[pallet::storage]
	#[pallet::getter(fn get_transaction_fee_rate)]
	pub type TransactionFee<T: Config<I>, I: 'static = ()> =
		StorageValue<_, (Permill, Permill), ValueQuery, DefaultPrice>;

	// Defult rate for sell and buy transaction fees is 0
	#[pallet::type_value]
	pub fn DefaultPrice() -> (Permill, Permill) {
		(Permill::zero(), Permill::zero())
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Create a sell order or buy order to sell `vsbond`.
		#[transactional]
		#[pallet::weight(T::WeightInfo::create_order())]
		pub fn create_order(
			origin: OriginFor<T>,
			#[pallet::compact] index: ParaId,
			token_symbol: TokenSymbol,
			#[pallet::compact] first_slot: LeasePeriodOf<T>,
			#[pallet::compact] last_slot: LeasePeriodOf<T>,
			#[pallet::compact] amount: BalanceOf<T, I>,
			#[pallet::compact] total_price: BalanceOf<T, I>,
			order_type: OrderType,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let owner = ensure_signed(origin)?;

			// Check amount
			ensure!(amount > T::MinimumAmount::get(), Error::<T, I>::NotEnoughAmount);

			// Check the token_symbol
			ensure!(
				token_symbol == TokenSymbol::KSM || token_symbol == TokenSymbol::DOT,
				Error::<T, I>::InvalidVsbond
			);

			// Construct vsbond
			let (_, vsbond) = CurrencyId::vsAssets(token_symbol, index, first_slot, last_slot);

			// Check the balance
			let (token_to_transfer, amount_to_transfer) = match order_type {
				OrderType::Buy => (T::InvoicingCurrency::get(), total_price),
				OrderType::Sell => (vsbond, amount),
			};

			// Calculate the transaction fee
			let maker_fee_rate = Self::get_transaction_fee_rate().0;
			let maker_fee = maker_fee_rate.mul_floor(total_price);

			match order_type {
				OrderType::Buy => {
					let amt_to_transfer = amount_to_transfer
						.checked_add(&maker_fee)
						.ok_or(Error::<T, I>::Overflow)?;
					T::MultiCurrency::ensure_can_withdraw(
						token_to_transfer,
						&owner,
						amt_to_transfer,
					)
					.map_err(|_| Error::<T, I>::NotEnoughBalanceToCreateOrder)?;
				},
				OrderType::Sell => {
					T::MultiCurrency::ensure_can_withdraw(
						token_to_transfer,
						&owner,
						amount_to_transfer,
					)
					.map_err(|_| Error::<T, I>::NotEnoughBalanceToCreateOrder)?;

					T::MultiCurrency::ensure_can_withdraw(
						T::InvoicingCurrency::get(),
						&owner,
						maker_fee,
					)
					.map_err(|_| Error::<T, I>::NotEnoughBalanceToCreateOrder)?;
				},
			}

			let order_ids_len = Self::user_order_ids(&owner, order_type).len();
			ensure!(
				order_ids_len < T::MaximumOrderInTrade::get() as usize,
				Error::<T, I>::ExceedMaximumOrderInTrade,
			);

			// Create OrderInfo
			let order_id = Self::next_order_id();
			let order_info = OrderInfo::<AccountIdOf<T>, BalanceOf<T, I>> {
				owner: owner.clone(),
				vsbond,
				amount,
				remain: amount,
				total_price,
				remain_price: total_price,
				order_id,
				order_type,
			};

			let module_account: AccountIdOf<T> = T::PalletId::get().into_account();

			// Transfer the amount to vsbond-acution module account.
			T::MultiCurrency::transfer(
				token_to_transfer,
				&owner,
				&module_account,
				amount_to_transfer,
			)?;

			// Charge fee
			T::MultiCurrency::transfer(
				T::InvoicingCurrency::get(),
				&owner,
				&T::TreasuryAccount::get(),
				maker_fee,
			)?;

			// Insert OrderInfo to Storage
			TotalOrderInfos::<T, I>::insert(order_id, order_info);
			UserOrderIds::<T, I>::try_append(owner.clone(), order_type, order_id)
				.map_err(|_| Error::<T, I>::Unexpected)
				.map_err(|_| Error::<T, I>::Unexpected)?;

			Self::deposit_event(Event::OrderCreated(
				order_id,
				order_type,
				owner,
				vsbond,
				amount,
				total_price,
			));

			Ok(().into())
		}

		/// Revoke a sell or buy order in trade by the order creator.
		#[transactional]
		#[pallet::weight(T::WeightInfo::revoke_order())]
		pub fn revoke_order(
			origin: OriginFor<T>,
			#[pallet::compact] order_id: OrderId,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let from = ensure_signed(origin)?;

			// Check OrderInfo
			let order_info = Self::order_info(order_id).ok_or(Error::<T, I>::NotFindOrderInfo)?;

			// Check OrderOwner
			ensure!(order_info.owner == from, Error::<T, I>::ForbidRevokeOrderWithoutOwnership);

			Self::do_order_revoke(order_id)?;

			Ok(().into())
		}

		/// Revoke a sell or buy order in trade by the order creator.
		#[transactional]
		#[pallet::weight(T::WeightInfo::revoke_order())]
		pub fn force_revoke(
			origin: OriginFor<T>,
			#[pallet::compact] order_id: OrderId,
		) -> DispatchResultWithPostInfo {
			// Check origin
			T::ControlOrigin::ensure_origin(origin)?;

			Self::do_order_revoke(order_id)?;

			Ok(().into())
		}

		/// Users(non-order-creator) buy the remaining `vsbond` of a sell order.
		#[transactional]
		#[pallet::weight(T::WeightInfo::clinch_order())]
		pub fn clinch_order(
			origin: OriginFor<T>,
			#[pallet::compact] order_id: OrderId,
		) -> DispatchResultWithPostInfo {
			let order_info = Self::order_info(order_id).ok_or(Error::<T, I>::NotFindOrderInfo)?;

			Self::partial_clinch_order(origin, order_id, order_info.remain)?;

			Ok(().into())
		}

		/// Users(non-order-creator) buys some of the remaining `vsbond` of a sell or buy order.
		#[transactional]
		#[pallet::weight(T::WeightInfo::partial_clinch_order())]
		pub fn partial_clinch_order(
			origin: OriginFor<T>,
			#[pallet::compact] order_id: OrderId,
			#[pallet::compact] quantity: BalanceOf<T, I>,
		) -> DispatchResultWithPostInfo {
			// Check Zero
			if quantity.is_zero() {
				return Ok(().into());
			}

			// Check origin
			let order_taker = ensure_signed(origin)?;

			// Check OrderInfo
			let order_info = Self::order_info(order_id).ok_or(Error::<T, I>::NotFindOrderInfo)?;

			// Check OrderOwner
			ensure!(
				order_info.owner != order_taker,
				Error::<T, I>::ForbidClinchOrderWithinOwnership
			);

			// Calculate the real quantity to clinch
			let quantity_clinchd = min(order_info.remain, quantity);
			// Calculate the total price that buyer need to pay
			let price_to_pay = Self::price_to_pay(quantity_clinchd, order_info.unit_price())?;

			let (token_to_get, amount_to_get, token_to_pay, amount_to_pay) = match order_info
				.order_type
			{
				OrderType::Buy =>
					(T::InvoicingCurrency::get(), price_to_pay, order_info.vsbond, quantity_clinchd),
				OrderType::Sell =>
					(order_info.vsbond, quantity_clinchd, T::InvoicingCurrency::get(), price_to_pay),
			};

			// Calculate the transaction fee
			let taker_fee_rate = Self::get_transaction_fee_rate().1;
			let taker_fee = taker_fee_rate.mul_floor(price_to_pay);

			// Check the balance of order taker
			match order_info.order_type {
				OrderType::Buy => {
					// transaction amount
					T::MultiCurrency::ensure_can_withdraw(
						token_to_pay,
						&order_taker,
						amount_to_pay,
					)
					.map_err(|_| Error::<T, I>::DontHaveEnoughToPay)?;

					// fee
					T::MultiCurrency::ensure_can_withdraw(
						T::InvoicingCurrency::get(),
						&order_taker,
						taker_fee,
					)
					.map_err(|_| Error::<T, I>::DontHaveEnoughToPay)?;
				},
				OrderType::Sell => {
					// transaction amount + fee
					let amt_to_pay =
						amount_to_pay.checked_add(&taker_fee).ok_or(Error::<T, I>::Overflow)?;
					T::MultiCurrency::ensure_can_withdraw(token_to_pay, &order_taker, amt_to_pay)
						.map_err(|_| Error::<T, I>::DontHaveEnoughToPay)?;
				},
			};

			// Get the new OrderInfo
			let remain_order = order_info
				.remain
				.checked_sub(&quantity_clinchd)
				.ok_or(Error::<T, I>::Overflow)?;
			let remain_price = order_info
				.remain_price
				.checked_sub(&price_to_pay)
				.ok_or(Error::<T, I>::Overflow)?;

			let new_order_info = OrderInfo { remain: remain_order, remain_price, ..order_info };

			let module_account: AccountIdOf<T> = T::PalletId::get().into_account();

			let mut account_to_send = new_order_info.owner.clone();
			let ed = T::MultiCurrency::minimum_balance(token_to_pay);

			// deal with account exisitence error we might encounter
			if amount_to_pay < ed {
				let receiver_balance =
					T::MultiCurrency::total_balance(token_to_pay, &new_order_info.owner);

				let receiver_balance_after =
					receiver_balance.checked_add(&amount_to_pay).ok_or(Error::<T, I>::Overflow)?;
				if receiver_balance_after < ed {
					account_to_send = T::TreasuryAccount::get();
				}
			}

			// Exchange: Transfer corresponding token amount to the order maker from order taker
			T::MultiCurrency::transfer(
				token_to_pay,
				&order_taker,
				&account_to_send,
				amount_to_pay,
			)?;

			// Charge fee
			T::MultiCurrency::transfer(
				T::InvoicingCurrency::get(),
				&order_taker,
				&T::TreasuryAccount::get(),
				taker_fee,
			)?;

			let mut account_to_send = order_taker.clone();
			let ed = T::MultiCurrency::minimum_balance(token_to_get);

			// deal with account exisitence error we might encounter
			if amount_to_get < ed {
				let receiver_balance = T::MultiCurrency::total_balance(token_to_get, &order_taker);

				let receiver_balance_after =
					receiver_balance.checked_add(&amount_to_get).ok_or(Error::<T, I>::Overflow)?;
				if receiver_balance_after < ed {
					account_to_send = T::TreasuryAccount::get();
				}
			}

			// Transfer corresponding token amount to the order taker from the module account
			T::MultiCurrency::transfer(
				token_to_get,
				&module_account,
				&account_to_send,
				amount_to_get,
			)?;

			// Change the OrderInfo in Storage
			// The seller sells out what he want to sell in the case of sell-order type.
			// Or the buyer get all the tokens he wants to buy in the case of buy-order type,
			// but the buyer might still have some unspent fund due to small number round-up.
			if new_order_info.remain == Zero::zero() {
				TotalOrderInfos::<T, I>::remove(order_id);
				Self::try_to_remove_order_id(
					new_order_info.owner.clone(),
					order_info.order_type,
					order_id,
				);

				if new_order_info.order_type == OrderType::Buy {
					T::MultiCurrency::transfer(
						token_to_get,
						&module_account,
						&new_order_info.owner,
						new_order_info.remain_price,
					)?;
				}
			} else {
				TotalOrderInfos::<T, I>::insert(order_id, new_order_info.clone());
			}

			Self::deposit_event(Event::<T, I>::OrderClinchd(
				order_id,
				new_order_info.order_type,
				new_order_info.owner,
				order_taker,
				new_order_info.vsbond,
				quantity_clinchd,
				new_order_info.amount,
				new_order_info.remain,
				new_order_info.total_price,
			));

			Ok(().into())
		}

		// edit token release start and end block
		// input number used as perthousand rate, so it should be less or equal than 1000.
		#[transactional]
		#[pallet::weight(T::WeightInfo::set_buy_and_sell_transaction_fee_rate())]
		pub fn set_buy_and_sell_transaction_fee_rate(
			origin: OriginFor<T>,
			buy_rate: u32,
			sell_rate: u32,
		) -> DispatchResult {
			// Check origin
			T::ControlOrigin::ensure_origin(origin)?;

			// number input should be less than 10_000, since it is used as x * 1/10_000
			ensure!(buy_rate <= 10_000u32, Error::<T, I>::InvalidRateInput);
			ensure!(sell_rate <= 10_000u32, Error::<T, I>::InvalidRateInput);

			let b_rate = buy_rate.checked_mul(100).ok_or(Error::<T, I>::Overflow)?;
			let s_rate = sell_rate.checked_mul(100).ok_or(Error::<T, I>::Overflow)?;

			let buy_fee_rate = Permill::from_parts(b_rate);
			let sell_fee_rate = Permill::from_parts(s_rate);

			TransactionFee::<T, I>::mutate(|fee| *fee = (buy_fee_rate, sell_fee_rate));

			Self::deposit_event(Event::TransactionFeeRateSet(buy_fee_rate, sell_fee_rate));

			Ok(())
		}
	}

	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		pub(crate) fn next_order_id() -> OrderId {
			let next_order_id = Self::order_id();
			NextOrderId::<T, I>::mutate(|current| *current += 1);
			next_order_id
		}

		pub(crate) fn try_to_remove_order_id(
			account: AccountIdOf<T>,
			order_type: OrderType,
			order_id: OrderId,
		) {
			UserOrderIds::<T, I>::mutate(account, order_type, |order_ids| {
				if let Some(position) = order_ids.iter().position(|&r| r == order_id) {
					order_ids.remove(position);
				}
			});
		}

		/// Get the price(round up) needed to pay.
		pub(crate) fn price_to_pay(
			quantity: BalanceOf<T, I>,
			unit_price: FixedU128,
		) -> Result<BalanceOf<T, I>, Error<T, I>> {
			let quantity: u128 = quantity.saturated_into();
			let total_price =
				unit_price.checked_mul_int(quantity).ok_or(Error::<T, I>::Overflow)?;

			Ok(BalanceOf::<T, I>::saturated_from(total_price))
		}

		pub(crate) fn do_order_revoke(order_id: OrderId) -> DispatchResultWithPostInfo {
			// Check OrderInfo
			let order_info = Self::order_info(order_id).ok_or(Error::<T, I>::NotFindOrderInfo)?;

			let (token_to_return, amount_to_return) = match order_info.order_type {
				OrderType::Buy => (T::InvoicingCurrency::get(), order_info.remain_price),
				OrderType::Sell => (order_info.vsbond, order_info.remain),
			};

			let mut account_to_return = order_info.owner.clone();
			let ed = T::MultiCurrency::minimum_balance(token_to_return);

			// deal with account exisitence error we might encounter
			if amount_to_return < ed {
				let receiver_balance =
					T::MultiCurrency::total_balance(token_to_return, &order_info.owner);

				let receiver_balance_after = receiver_balance
					.checked_add(&amount_to_return)
					.ok_or(Error::<T, I>::Overflow)?;

				if receiver_balance_after < ed {
					account_to_return = T::TreasuryAccount::get();
				}
			}

			// To transfer back the unused amount
			let module_account: AccountIdOf<T> = T::PalletId::get().into_account();
			T::MultiCurrency::transfer(
				token_to_return,
				&module_account,
				&account_to_return,
				amount_to_return,
			)?;

			// Revoke order
			TotalOrderInfos::<T, I>::remove(order_id);
			Self::try_to_remove_order_id(order_info.owner.clone(), order_info.order_type, order_id);

			Self::deposit_event(Event::OrderRevoked(
				order_id,
				order_info.order_type,
				order_info.owner,
				order_info.vsbond,
				order_info.amount,
				order_info.remain,
				order_info.total_price,
			));

			Ok(().into())
		}
	}
}

// TODO: Maybe impl Auction trait for vsbond-auction

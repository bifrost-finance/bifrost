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
// #![cfg_attr(not(feature = "std"), no_std)]

use super::*;
use crate::{Pallet, TotalOrderInfos};

pub fn migrate_orders<T: Config<I>, I: 'static>() -> Weight {
	// get current orders in list
	let order_iter = TotalOrderInfos::<T, I>::iter();
	let mut ok_count = 0;
	let mut err_count = 0;

	for (order_id, order_info) in order_iter {
		let owner = order_info.owner;
		let order_type = order_info.order_type;
		let vsbond = order_info.vsbond;
		let token = T::InvoicingCurrency::get();

		let (token_to_transfer, amount_to_transfer) = match order_type {
			OrderType::Buy => (token, order_info.remain_price),
			OrderType::Sell => (vsbond, order_info.remain),
		};

		let total = T::MultiCurrency::total_balance(token_to_transfer, &owner);
		let free = T::MultiCurrency::free_balance(token_to_transfer, &owner);
		let reserved = total - free;

		let module_account: AccountIdOf<T> = T::PalletId::get().into_account_truncating();

		if reserved >= amount_to_transfer {
			ok_count += 1;
			// unreserved and then transfer
			T::MultiCurrency::unreserve(token_to_transfer, &owner, amount_to_transfer);
			let _ = T::MultiCurrency::transfer(
				token_to_transfer,
				&owner,
				&module_account,
				amount_to_transfer,
			);
		} else if total >= amount_to_transfer {
			ok_count += 1;
			// make free all the reserved balance and then transfer
			T::MultiCurrency::unreserve(token_to_transfer, &owner, reserved);
			let _ = T::MultiCurrency::transfer(
				token_to_transfer,
				&owner,
				&module_account,
				amount_to_transfer,
			);
		} else {
			err_count += 1;
			TotalOrderInfos::<T, I>::remove(order_id);
			Pallet::<T, I>::try_to_remove_order_id(owner, order_type, order_id);
			log::info!(
				"Order {:?} is removed, transfer amount is: {:?}, account balance is: {:?}",
				order_id,
				amount_to_transfer,
				total
			);
		}
	}

	// one storage read + two account balance changes
	let ok_weight =
		ok_count.saturating_mul(T::DbWeight::get().reads(1) + T::DbWeight::get().writes(2));
	let err_weight =
		err_count.saturating_mul(T::DbWeight::get().reads(1) + T::WeightInfo::revoke_order());

	ok_weight + err_weight
}

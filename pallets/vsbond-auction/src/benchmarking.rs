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

#![cfg(feature = "runtime-benchmarks")]

use bifrost_primitives::TokenSymbol;
use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, v1::BenchmarkError, whitelisted_caller,
};
use frame_support::{sp_runtime::traits::UniqueSaturatedFrom, traits::UnfilteredDispatchable};
use frame_system::RawOrigin;

use super::*;
#[allow(unused_imports)]
use crate::Pallet as VSBondAuction;

benchmarks! {
	create_order {
		let caller: T::AccountId = whitelisted_caller();
		let index: ParaId = 3000;
		let first_slot = 13u32;
		let last_slot = 20u32;
		let supply = BalanceOf::<T>::unique_saturated_from(10u128);
		let total_price = BalanceOf::<T>::unique_saturated_from(30u128);
		let order_type = OrderType::Sell;
	}: _(RawOrigin::Signed(caller), index, TokenSymbol::KSM, first_slot, last_slot, supply, total_price, order_type)

	revoke_order {
		let caller: T::AccountId = whitelisted_caller();
		let index: ParaId = 3000;
		let first_slot = 13u32;
		let last_slot = 20u32;
		let supply = BalanceOf::<T>::unique_saturated_from(10u128);
		let total_price = BalanceOf::<T>::unique_saturated_from(30u128);
		let order_type = OrderType::Sell;

		VSBondAuction::<T>::create_order(<T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(caller.clone())), index, TokenSymbol::KSM, first_slot, last_slot, supply, total_price, order_type)?;
	}: _(RawOrigin::Signed(caller),0u64)

	clinch_order {
		let caller: T::AccountId = whitelisted_caller();
		let index: ParaId = 3000;
		let first_slot = 13u32;
		let last_slot = 20u32;
		let supply = BalanceOf::<T>::unique_saturated_from(10u128);
		let total_price = BalanceOf::<T>::unique_saturated_from(30u128);
		let order_owner = account("bechmarking_account_1", 0, 0);
		let order_type = OrderType::Sell;

		VSBondAuction::<T>::create_order(<T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(order_owner)), index, TokenSymbol::KSM, first_slot, last_slot, supply, total_price, order_type)?;
	}: _(RawOrigin::Signed(caller),0u64)

	partial_clinch_order {
		let caller: T::AccountId = whitelisted_caller();
		let index: ParaId = 3000;
		let first_slot = 13u32;
		let last_slot = 20u32;
		let supply = BalanceOf::<T>::unique_saturated_from(10u128);
		let total_price = BalanceOf::<T>::unique_saturated_from(30u128);
		let order_owner = account("bechmarking_account_1", 0, 0);
		let order_type = OrderType::Sell;

		VSBondAuction::<T>::create_order(<T as frame_system::Config>::RuntimeOrigin::from(RawOrigin::Signed(order_owner)), index, TokenSymbol::KSM, first_slot, last_slot, supply, total_price, order_type)?;
	}: _(RawOrigin::Signed(caller),0u64, BalanceOf::<T>::unique_saturated_from(5u128))

	set_buy_and_sell_transaction_fee_rate {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let buy_rate = 1000u32;
		let sell_rate = 1000u32;
		let call = Call::<T>::set_buy_and_sell_transaction_fee_rate { buy_rate, sell_rate };
  }: {call.dispatch_bypass_filter(origin)?}

}

impl_benchmark_test_suite!(VSBondAuction, crate::mock::new_test_ext(), crate::mock::Test);

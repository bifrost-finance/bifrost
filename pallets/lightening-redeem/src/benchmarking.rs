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

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::{dispatch::UnfilteredDispatchable, sp_runtime::traits::UniqueSaturatedFrom};
use frame_system::RawOrigin;

use super::*;
#[allow(unused_imports)]
use crate::Pallet as LighteningRedeem;
use crate::{PoolAmount, StartEndReleaseBlock};

benchmarks! {
	add_ksm_to_pool {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u32 as u128);
	}: _(RawOrigin::Signed(caller), token_amount)

	exchange_for_ksm {
		let caller: T::AccountId = whitelisted_caller();
		// add 1000 ksm to the pool
		let amount = BalanceOf::<T>::unique_saturated_from(1_000u128);
		LighteningRedeem::<T>::add_ksm_to_pool(RawOrigin::Signed(caller.clone()).into(), BalanceOf::<T>::unique_saturated_from(amount))?;

		PoolAmount::<T>::mutate(|amt| *amt += amount);

		let exchange_amount: u128 = 900;
		let token_amount = BalanceOf::<T>::unique_saturated_from(exchange_amount);
	}: _(RawOrigin::Signed(caller), token_amount)

	edit_exchange_price {
		let origin = T::ControlOrigin::successful_origin();
		let price = BalanceOf::<T>::unique_saturated_from(50u128);
		let call = Call::<T>::edit_exchange_price { price };
	}: {call.dispatch_bypass_filter(origin)?}

	edit_release_per_day {
		let origin = T::ControlOrigin::successful_origin();
		let amount_per_day = BalanceOf::<T>::unique_saturated_from(50u128);
		let call = Call::<T>::edit_release_per_day { amount_per_day };
	}: {call.dispatch_bypass_filter(origin)?}

	edit_release_start_and_end_block {
		let origin = T::ControlOrigin::successful_origin();
		let start = BlockNumberFor::<T>::from(50u32);
		let end = BlockNumberFor::<T>::from(100u32);
		let call = Call::<T>::edit_release_start_and_end_block { start, end };
	}: {call.dispatch_bypass_filter(origin)?}

	on_initialize {
		let caller: T::AccountId = whitelisted_caller();
		let amount: u128 = 1_000;
		LighteningRedeem::<T>::add_ksm_to_pool(RawOrigin::Signed(caller.clone()).into(), BalanceOf::<T>::unique_saturated_from(amount))?;
		StartEndReleaseBlock::<T>::mutate(|interval| *interval = (T::BlockNumber::from(0u32), T::BlockNumber::from(100u32)));

		let block_num = T::BlockNumber::from(10u32);
	}:{LighteningRedeem::<T>::on_initialize(block_num);}
}

impl_benchmark_test_suite!(
	LighteningRedeem,
	crate::mock::ExtBuilder::default()
		.one_hundred_precision_for_each_currency_type_for_whitelist_account()
		.build(),
	crate::mock::Test
);

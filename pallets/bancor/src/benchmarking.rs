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
use frame_system::RawOrigin;
use node_primitives::{CurrencyId, TokenSymbol};
use sp_runtime::traits::UniqueSaturatedFrom;

use super::*;
use crate::BancorPools;
#[allow(unused_imports)]
use crate::Pallet as Bancor;

benchmarks! {
	add_token_to_pool {
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(10u32 as u128);
	}: _(RawOrigin::Signed(caller), currency_id, token_amount)

	exchange_for_token {
		let caller: T::AccountId = whitelisted_caller();
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let vstoken_amount = BalanceOf::<T>::unique_saturated_from(1u128);
		let token_out_min =  BalanceOf::<T>::unique_saturated_from(0u128);

		// add token to the pool
		BancorPools::<T>::mutate(currency_id, |pool_info_option|{
			let pool_info = pool_info_option.as_mut().unwrap();

			pool_info.token_ceiling =
				pool_info.token_ceiling.saturating_add(BalanceOf::<T>::unique_saturated_from(10u128));
		});

	}: _(RawOrigin::Signed(caller), currency_id, vstoken_amount, token_out_min)

	exchange_for_vstoken {
		let caller: T::AccountId = whitelisted_caller();
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let token_amount = BalanceOf::<T>::unique_saturated_from(100u128);
		let vstoken_out_min =  BalanceOf::<T>::unique_saturated_from(0u128);

		BancorPools::<T>::mutate(currency_id, |pool| {
			let pool_info = pool.as_mut().unwrap();
			pool_info.token_pool = pool_info.token_pool.saturating_add(token_amount);
			pool_info.vstoken_pool = pool_info.vstoken_pool.saturating_add(token_amount);
		});
	}: _(RawOrigin::Signed(caller), currency_id, token_amount, vstoken_out_min)

	on_initialize {
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(7200 as u128);

		BancorPools::<T>::mutate(currency_id, |pool| {
			let pool_info = pool.as_mut().unwrap();
			pool_info.token_pool = pool_info.token_pool.saturating_add(token_amount);
		});

		let block_num = T::BlockNumber::from(100u32);
	}:{Bancor::<T>::on_initialize(block_num);}

}

impl_benchmark_test_suite!(
	Bancor,
	crate::mock::ExtBuilder::default()
		.one_hundred_precision_for_each_currency_type_for_whitelist_account()
		.build(),
	crate::mock::Test
);

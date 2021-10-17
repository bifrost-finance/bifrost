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

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use node_primitives::{CurrencyId, TokenSymbol};
use sp_runtime::traits::UniqueSaturatedFrom;

use super::*;
#[allow(unused_imports)]
use crate::Pallet as VtokenMint;

benchmarks! {
	set_token_staking_lock_period {
		let token_id = CurrencyId::Token(TokenSymbol::KSM);
		let locking_blocks = T::BlockNumber::from(100u32);
	}: _(RawOrigin::Root, token_id, locking_blocks)

	set_vtoken_pool {
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let new_token_pool = BalanceOf::<T>::unique_saturated_from(10u32 as u128);
		let new_vtoken_pool = BalanceOf::<T>::unique_saturated_from(20u32 as u128);
	}: _(RawOrigin::Root, currency_id, new_token_pool, new_vtoken_pool)

	mint {
		VtokenMint::<T>::expand_mint_pool(CurrencyId::Token(TokenSymbol::KSM), BalanceOf::<T>::unique_saturated_from(100u32 as u128))?;
		VtokenMint::<T>::expand_mint_pool(CurrencyId::VToken(TokenSymbol::KSM), BalanceOf::<T>::unique_saturated_from(200u32 as u128))?;

		let caller: T::AccountId = whitelisted_caller();
		let vtoken_id = CurrencyId::VToken(TokenSymbol::KSM);
		let token_amount = BalanceOf::<T>::unique_saturated_from(10u32 as u128);
	}: _(RawOrigin::Signed(caller), vtoken_id, token_amount)

	redeem {
		VtokenMint::<T>::expand_mint_pool(CurrencyId::Token(TokenSymbol::KSM), BalanceOf::<T>::unique_saturated_from(100u32 as u128))?;
		VtokenMint::<T>::expand_mint_pool(CurrencyId::VToken(TokenSymbol::KSM), BalanceOf::<T>::unique_saturated_from(200u32 as u128))?;

		let caller: T::AccountId = whitelisted_caller();
		let token_id = CurrencyId::Token(TokenSymbol::KSM);
		let vtoken_amount = BalanceOf::<T>::unique_saturated_from(10u32 as u128);
	}: _(RawOrigin::Signed(caller), token_id, vtoken_amount)

	on_initialize {
		let block_num = T::BlockNumber::from(100u32);
	}:{VtokenMint::<T>::on_initialize(block_num);}
}

impl_benchmark_test_suite!(
	VtokenMint,
	crate::mock::ExtBuilder::default()
		.one_hundred_precision_for_each_currency_type_for_whitelist_account()
		.build(),
	crate::mock::Runtime
);

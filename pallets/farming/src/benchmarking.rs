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
#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, vec, whitelisted_caller};
use frame_support::{assert_ok, sp_runtime::traits::UniqueSaturatedFrom};
use frame_system::{Pallet as System, RawOrigin};
use node_primitives::{CurrencyId, TokenSymbol};

use crate::{Pallet as Farming, *};

benchmarks! {
	on_initialize {}:{Farming::<T>::on_initialize(T::BlockNumber::from(10u32));}

	create_farming_pool {
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let tokens_proportion = vec![(KSM, Perbill::from_percent(100))];
		let basic_rewards = vec![(KSM, token_amount)];
		let gauge_basic_rewards = vec![(KSM, token_amount)];
	}: _(RawOrigin::Root,
	tokens_proportion.clone(),
	basic_rewards.clone(),
	Some((KSM, BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
	BalanceOf::<T>::unique_saturated_from(0u128),
	BlockNumberFor::<T>::from(0u32),
	BlockNumberFor::<T>::from(7u32),
	BlockNumberFor::<T>::from(6u32),
	5)

	deposit {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let tokens_proportion = vec![(KSM, Perbill::from_percent(100))];
		let basic_rewards = vec![(KSM, token_amount)];
		let gauge_basic_rewards = vec![(KSM, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((KSM, BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(KSM,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards));
	}: _(RawOrigin::Signed(caller.clone()), 0, token_amount, None)

	withdraw {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let tokens_proportion = vec![(KSM, Perbill::from_percent(100))];
		let basic_rewards = vec![(KSM, token_amount)];
		let gauge_basic_rewards = vec![(KSM, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((KSM, BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(KSM,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards));
		assert_ok!(Farming::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, token_amount, None));
	}: _(RawOrigin::Signed(caller.clone()), 0, None)

	claim {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let tokens_proportion = vec![(KSM, Perbill::from_percent(100))];
		let basic_rewards = vec![(KSM, token_amount)];
		let gauge_basic_rewards = vec![(KSM, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((KSM, BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(KSM,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards));
		assert_ok!(Farming::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, token_amount, None));
		System::<T>::set_block_number(System::<T>::block_number() + BlockNumberFor::<T>::from(10u32));
		Farming::<T>::on_initialize(BlockNumberFor::<T>::from(0u32));
	}: _(RawOrigin::Signed(caller.clone()), 0)

	gauge_withdraw {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let tokens_proportion = vec![(KSM, Perbill::from_percent(100))];
		let basic_rewards = vec![(KSM, token_amount)];
		let gauge_basic_rewards = vec![(KSM, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((KSM, BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(KSM,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards));
		assert_ok!(Farming::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, token_amount, Some((BalanceOf::<T>::unique_saturated_from(100u128), BlockNumberFor::<T>::from(100u32)))));
		// System::<T>::set_block_number(System::<T>::block_number() + BlockNumberFor::<T>::from(10u32));
	}: _(RawOrigin::Signed(caller.clone()), 0)
}

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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::{assert_ok, sp_runtime::traits::UniqueSaturatedFrom};
use frame_system::{Pallet as System, RawOrigin};
use sp_std::vec;

use crate::{Pallet as Farming, *};

benchmarks! {
	on_initialize {}:{Farming::<T>::on_initialize(BlockNumberFor::<T>::from(10u32));}
	create_farming_pool {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
	}: _(RawOrigin::Root,
	tokens_proportion.clone(),
	basic_rewards.clone(),
	Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
	BalanceOf::<T>::unique_saturated_from(0u128),
	BlockNumberFor::<T>::from(0u32),
	BlockNumberFor::<T>::from(7u32),
	BlockNumberFor::<T>::from(6u32),
	5)

	deposit {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards, false));
	}: _(RawOrigin::Signed(caller.clone()), 0, token_amount)

	withdraw {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards, false));
		assert_ok!(Farming::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, token_amount));
	}: _(RawOrigin::Signed(caller.clone()), 0, None)

	claim {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards, false));
		assert_ok!(Farming::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, token_amount));
		System::<T>::set_block_number(System::<T>::block_number() + BlockNumberFor::<T>::from(10u32));
		Farming::<T>::on_initialize(BlockNumberFor::<T>::from(0u32));
	}: _(RawOrigin::Signed(caller.clone()), 0)

	gauge_withdraw {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards, false));
		assert_ok!(Farming::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, token_amount));
		// System::<T>::set_block_number(System::<T>::block_number() + BlockNumberFor::<T>::from(10u32));
	}: _(RawOrigin::Signed(caller.clone()), 0)

	withdraw_claim {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards, false));
		assert_ok!(Farming::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, token_amount));
	}: _(RawOrigin::Signed(caller.clone()), 0)

	reset_pool {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		let pid = 0;
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards.clone())),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards, false));
		System::<T>::set_block_number(System::<T>::block_number() + BlockNumberFor::<T>::from(10u32));
		Farming::<T>::on_initialize(BlockNumberFor::<T>::from(0u32));
		assert_ok!(Farming::<T>::close_pool(RawOrigin::Root.into(), pid));
		assert_ok!(Farming::<T>::set_retire_limit(RawOrigin::Root.into(), 10));
		assert_ok!(Farming::<T>::force_retire_pool(RawOrigin::Root.into(), pid));
	}: _(RawOrigin::Root,
	pid,
	Some(basic_rewards.clone()),
	Some(BalanceOf::<T>::unique_saturated_from(0u128)),
	Some(BlockNumberFor::<T>::from(0u32)),
	Some(BlockNumberFor::<T>::from(7u32)),
	Some(BlockNumberFor::<T>::from(6u32)),
	Some(5),
	Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards))
	)

	force_retire_pool {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		let pid = 0;
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards.clone())),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards, false));
		System::<T>::set_block_number(System::<T>::block_number() + BlockNumberFor::<T>::from(10u32));
		Farming::<T>::on_initialize(BlockNumberFor::<T>::from(0u32));
		assert_ok!(Farming::<T>::close_pool(RawOrigin::Root.into(), pid));
		assert_ok!(Farming::<T>::set_retire_limit(RawOrigin::Root.into(), 10));
	}: _(RawOrigin::Root, pid)

	kill_pool {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		let pid = 0;
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards.clone())),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
	}: _(RawOrigin::Root,pid)

	edit_pool {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards.clone())),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5));
	}: _(RawOrigin::Root,
	0,
	Some(basic_rewards.clone()),
	Some(BlockNumberFor::<T>::from(7u32)),
	Some(BlockNumberFor::<T>::from(6u32)),
	Some(gauge_basic_rewards),
	Some(5))

	close_pool {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::create_farming_pool(RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5));
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards, false));
		System::<T>::set_block_number(System::<T>::block_number() + BlockNumberFor::<T>::from(10u32));
		Farming::<T>::on_initialize(BlockNumberFor::<T>::from(0u32));
	}: _(RawOrigin::Root, 0)

	charge {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
	}: _(RawOrigin::Signed(caller.clone()), 0, charge_rewards, false)

	force_gauge_claim {
		let caller: T::AccountId = whitelisted_caller();
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u128);
		let default_currency_id = CurrencyIdOf::<T>::default();
		let tokens_proportion = vec![(default_currency_id, Perbill::from_percent(100))];
		let basic_rewards = vec![(default_currency_id, token_amount)];
		let gauge_basic_rewards = vec![(default_currency_id, token_amount)];
		assert_ok!(Farming::<T>::create_farming_pool(
			RawOrigin::Root.into(),
			tokens_proportion.clone(),
			basic_rewards.clone(),
			Some((BlockNumberFor::<T>::from(1000u32), gauge_basic_rewards)),
			BalanceOf::<T>::unique_saturated_from(0u128),
			BlockNumberFor::<T>::from(0u32),
			BlockNumberFor::<T>::from(7u32),
			BlockNumberFor::<T>::from(6u32),
			5,
		));
		let charge_rewards = vec![(default_currency_id,BalanceOf::<T>::unique_saturated_from(300000u128))];
		assert_ok!(Farming::<T>::charge(RawOrigin::Signed(caller.clone()).into(), 0, charge_rewards, false));
		assert_ok!(Farming::<T>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, token_amount));
		assert_ok!(Farming::<T>::set_retire_limit(RawOrigin::Root.into(), 10));
	}: _(RawOrigin::Root, 0)

	set_retire_limit {}: _(RawOrigin::Root, 10)

	add_boost_pool_whitelist {}: _(RawOrigin::Root, vec![0])

	set_next_round_whitelist {
		assert_ok!(Farming::<T>::add_boost_pool_whitelist(RawOrigin::Root.into(), vec![0]));
	}: _(RawOrigin::Root, vec![0])

	vote {
		let caller: T::AccountId = whitelisted_caller();
		let vote_list: Vec<(u32, Percent)> = vec![(0, Percent::from_percent(100))];
		assert_ok!(Farming::<T>::add_boost_pool_whitelist(RawOrigin::Root.into(), vec![0]));
	}: _(RawOrigin::Signed(caller.clone()), vote_list)

	start_boost_round {
		assert_ok!(Farming::<T>::add_boost_pool_whitelist(RawOrigin::Root.into(), vec![0]));
	}: _(RawOrigin::Root, BlockNumberFor::<T>::from(100000u32))

	end_boost_round {
		assert_ok!(Farming::<T>::add_boost_pool_whitelist(RawOrigin::Root.into(), vec![0]));
		assert_ok!(Farming::<T>::start_boost_round(RawOrigin::Root.into(), BlockNumberFor::<T>::from(100000u32)));
	}: _(RawOrigin::Root)

	charge_boost {
		let caller: T::AccountId = whitelisted_caller();
		let default_currency_id = CurrencyIdOf::<T>::default();
		let charge_list = vec![(default_currency_id, BalanceOf::<T>::unique_saturated_from(1_000_0000_000_000u128))];
		assert_ok!(Farming::<T>::add_boost_pool_whitelist(RawOrigin::Root.into(), vec![0]));
		assert_ok!(Farming::<T>::start_boost_round(RawOrigin::Root.into(), BlockNumberFor::<T>::from(100000u32)));
	}: _(RawOrigin::Signed(caller.clone()), charge_list)
}

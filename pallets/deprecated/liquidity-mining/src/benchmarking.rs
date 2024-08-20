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
use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelisted_caller,
};
use frame_support::{
	assert_ok,
	sp_runtime::{
		sp_std::convert::TryInto,
		traits::{UniqueSaturatedFrom, Zero},
	},
	sp_std::prelude::*,
};
use frame_system::RawOrigin;
use bifrost_primitives::Balance;

use crate::{Pallet as LM, *};

const FARMING_DEPOSIT_1: CurrencyId = CurrencyId::VSToken(TokenSymbol::KSM);
const FARMING_DEPOSIT_2: CurrencyId = CurrencyId::VSBond(TokenSymbol::KSM, 2001, 13, 20);
const REWARD_1: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
const REWARD_2: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
const SINGLE_TOKEN_DEPOSIT: CurrencyId = CurrencyId::Token(TokenSymbol::ZLK);
const MINING_TRADING_PAIR: (CurrencyId, CurrencyId) =
	(CurrencyId::Token(TokenSymbol::DOT), CurrencyId::Token(TokenSymbol::KSM));
const UNIT: Balance = 1_000_000_000_000;
const REWARD_AMOUNT: Balance = UNIT;
const MINUTES: u32 = 60 / (12 as u32);
const HOURS: u32 = MINUTES * 60;
const DAYS: u32 = HOURS * 24;

fn run_to_block<T: Config<I>, I: 'static>(n: BlockNumberFor<T>) {
	type System<T> = frame_system::Pallet<T>;

	while System::<T>::block_number() < n {
		LM::<T, I>::on_finalize(System::<T>::block_number());
		System::<T>::on_finalize(System::<T>::block_number());
		System::<T>::set_block_number(System::<T>::block_number() + 1u128.saturated_into());
		System::<T>::on_initialize(System::<T>::block_number());
		LM::<T, I>::on_initialize(System::<T>::block_number());
	}
}

benchmarks_instance_pallet! {

	charge {
		let caller: T::AccountId = whitelisted_caller();
assert_ok!(
			LM::<T, I>::lazy_migration_v2_0_0(
				RawOrigin::Signed(caller.clone()).into(),
			200)
		);
		let duration = T::MinimumDuration::get().saturating_add(1u128.saturated_into());
		let min_deposit_to_start = T::MinimumDepositOfUser::get();
		let amount: BalanceOf<T,I> = UNIT.saturated_into();

		assert_ok!(T::MultiCurrency::deposit(REWARD_1, &caller, amount));
		assert_ok!(T::MultiCurrency::deposit(REWARD_2, &caller, amount));

		assert_ok!(LM::<T,I>::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, amount),
			vec![(REWARD_2, amount)].try_into().unwrap(),
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			0u128.saturated_into(),
			Zero::zero(),
			0,
		));
	}: _(RawOrigin::Signed(caller.clone()), 0)

	deposit {
				let caller: T::AccountId = whitelisted_caller();
assert_ok!(
			LM::<T, I>::lazy_migration_v2_0_0(
				RawOrigin::Signed(caller.clone()).into(),
			200)
		);
		let duration = T::MinimumDuration::get().saturating_add(1u128.saturated_into());
		let min_deposit_to_start = T::MinimumDepositOfUser::get();
		let amount: BalanceOf<T,I> = UNIT.saturated_into();

		let investor: T::AccountId = account("lm", 0, 0);
		assert_ok!(T::MultiCurrency::deposit(REWARD_1, &investor, amount));
		assert_ok!(T::MultiCurrency::deposit(REWARD_2, &investor, amount));

		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_1, &caller, amount));
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_2, &caller, amount));

		assert_ok!(LM::<T,I>::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, amount),
			vec![(REWARD_2, amount)].try_into().unwrap(),
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			0u128.saturated_into(),
			Zero::zero(),
			0,
		));

		assert_ok!(LM::<T,I>::charge(RawOrigin::Signed(investor).into(), 0));

	}: _(RawOrigin::Signed(caller.clone()), 0, T::MinimumDepositOfUser::get())

	redeem {
		let caller: T::AccountId = whitelisted_caller();
assert_ok!(
			LM::<T, I>::lazy_migration_v2_0_0(
				RawOrigin::Signed(caller.clone()).into(),
			200)
		);
		let duration = T::MinimumDuration::get().saturating_add(1u128.saturated_into());
		let min_deposit_to_start = T::MinimumDepositOfUser::get();
		let amount: BalanceOf<T,I> = UNIT.saturated_into();

		let investor: T::AccountId = account("lm", 0, 0);
		assert_ok!(T::MultiCurrency::deposit(REWARD_1, &investor, amount));
		assert_ok!(T::MultiCurrency::deposit(REWARD_2, &investor, amount));

		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_1, &caller, amount));
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_2, &caller, amount));

		assert_ok!(LM::<T,I>::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, amount),
			vec![(REWARD_2, amount)].try_into().unwrap(),
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			0u128.saturated_into(),
			Zero::zero(),
			0,
		));

		assert_ok!(LM::<T,I>::charge(RawOrigin::Signed(investor).into(), 0));

		assert_ok!(LM::<T,I>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, amount));

		// Run to block
		run_to_block::<T,I>(duration);

	}: _(RawOrigin::Signed(caller.clone()), 0, amount)
	verify {
		let pool = LM::<T,I>::pool(0);
		let deposit_data = LM::<T,I>::user_deposit_data(0, caller.clone());
		assert!(pool.is_none());
		assert!(deposit_data.is_none());
	}

	redeem_all {
		let caller: T::AccountId = whitelisted_caller();
assert_ok!(
			LM::<T, I>::lazy_migration_v2_0_0(
				RawOrigin::Signed(caller.clone()).into(),
			200)
		);
		let duration = T::MinimumDuration::get().saturating_add(1u128.saturated_into());
		let min_deposit_to_start = T::MinimumDepositOfUser::get();
		let amount: BalanceOf<T,I> = UNIT.saturated_into();

		let investor: T::AccountId = account("lm", 0, 0);
		assert_ok!(T::MultiCurrency::deposit(REWARD_1, &investor, amount));
		assert_ok!(T::MultiCurrency::deposit(REWARD_2, &investor, amount));

		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_1, &caller, amount));
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_2, &caller, amount));

		assert_ok!(LM::<T,I>::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, amount),
			vec![(REWARD_2, amount)].try_into().unwrap(),
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			0u128.saturated_into(),
			Zero::zero(),
			0,
		));

		assert_ok!(LM::<T,I>::charge(RawOrigin::Signed(investor).into(), 0));

		assert_ok!(LM::<T,I>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, amount));

		// Run to block
		run_to_block::<T,I>(duration);

	}: _(RawOrigin::Signed(caller.clone()), 0)
	verify {
		let pool = LM::<T,I>::pool(0);
		let deposit_data = LM::<T,I>::user_deposit_data(0, caller.clone());
		assert!(pool.is_none());
		assert!(deposit_data.is_none());
	}

	volunteer_to_redeem {
		let caller: T::AccountId = whitelisted_caller();
assert_ok!(
			LM::<T, I>::lazy_migration_v2_0_0(
				RawOrigin::Signed(caller.clone()).into(),
			200)
		);
		let duration = T::MinimumDuration::get().saturating_add(1u128.saturated_into());
		let min_deposit_to_start = T::MinimumDepositOfUser::get();
		let amount: BalanceOf<T,I> = UNIT.saturated_into();

		let investor: T::AccountId = account("lm", 0, 0);
		assert_ok!(T::MultiCurrency::deposit(REWARD_1, &investor, amount));
		assert_ok!(T::MultiCurrency::deposit(REWARD_2, &investor, amount));

		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_1, &caller, amount));
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_2, &caller, amount));

		assert_ok!(LM::<T,I>::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, amount),
			vec![(REWARD_2, amount)].try_into().unwrap(),
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			0u128.saturated_into(),
			Zero::zero(),
			0,
		));

		assert_ok!(LM::<T,I>::charge(RawOrigin::Signed(investor).into(), 0));

		assert_ok!(LM::<T,I>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, amount));

		// Run to block
		run_to_block::<T,I>(duration);

		let volunteer = account("lm", 0, 1);

	}: _(RawOrigin::Signed(volunteer), 0, None)
	verify {
		let pool = LM::<T,I>::pool(0);
		let deposit_data = LM::<T,I>::user_deposit_data(0, caller.clone());
		assert!(pool.is_none());
		assert!(deposit_data.is_none());
	}

	claim {
		let caller: T::AccountId = whitelisted_caller();
assert_ok!(
			LM::<T, I>::lazy_migration_v2_0_0(
				RawOrigin::Signed(caller.clone()).into(),
			200)
		);
		let duration = T::MinimumDuration::get().saturating_add(1u128.saturated_into());
		let min_deposit_to_start = T::MinimumDepositOfUser::get();
		let amount: BalanceOf<T,I> = UNIT.saturated_into();

		let investor: T::AccountId = account("lm", 0, 0);
		assert_ok!(T::MultiCurrency::deposit(REWARD_1, &investor, amount));
		assert_ok!(T::MultiCurrency::deposit(REWARD_2, &investor, amount));

		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_1, &caller, amount));
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_2, &caller, amount));

		assert_ok!(LM::<T,I>::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, amount),
			vec![(REWARD_2, amount)].try_into().unwrap(),
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			0u128.saturated_into(),
			Zero::zero(),
			0,
		));

		assert_ok!(LM::<T,I>::charge(RawOrigin::Signed(investor).into(), 0));

		assert_ok!(LM::<T,I>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, amount));

		// Run to block
		run_to_block::<T,I>(1u128.saturated_into());

	}: _(RawOrigin::Signed(caller.clone()), 0)

	unlock {
		let caller: T::AccountId = whitelisted_caller();
assert_ok!(
			LM::<T, I>::lazy_migration_v2_0_0(
				RawOrigin::Signed(caller.clone()).into(),
			200)
		);
		let duration = T::MinimumDuration::get().saturating_add(1u128.saturated_into());
		let min_deposit_to_start = T::MinimumDepositOfUser::get();
		let amount: BalanceOf<T,I> = UNIT.saturated_into();
		let redeem_limit_time: BlockNumberFor<T> = 100u32.saturated_into();
		let unlock_limit_nums = 3;

		let investor: T::AccountId = account("lm", 0, 0);
		assert_ok!(T::MultiCurrency::deposit(REWARD_1, &investor, amount));
		assert_ok!(T::MultiCurrency::deposit(REWARD_2, &investor, amount));

		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_1, &caller, amount));
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_2, &caller, amount));

		assert_ok!(LM::<T,I>::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, amount),
			vec![(REWARD_2, amount)].try_into().unwrap(),
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			0u128.saturated_into(),
			redeem_limit_time,
			unlock_limit_nums,
		));

		assert_ok!(LM::<T,I>::charge(RawOrigin::Signed(investor).into(), 0));

		assert_ok!(LM::<T,I>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, amount));

		// Run to block
		run_to_block::<T,I>(duration);

		assert_ok!(LM::<T,I>::redeem_all(RawOrigin::Signed(caller.clone()).into(), 0));

		run_to_block::<T,I>(duration.saturating_add(redeem_limit_time));

	}: _(RawOrigin::Signed(caller.clone()), 0)
	verify {
		let pool = LM::<T,I>::pool(0);
		let deposit_data = LM::<T,I>::user_deposit_data(0, caller.clone());
		assert!(pool.is_none());
		assert!(deposit_data.is_none());
	}

	cancel_unlock{
		let caller: T::AccountId = whitelisted_caller();
assert_ok!(
			LM::<T, I>::lazy_migration_v2_0_0(
				RawOrigin::Signed(caller.clone()).into(),
			200)
		);
		let duration = T::MinimumDuration::get().saturating_add(1u128.saturated_into());
		let min_deposit_to_start = T::MinimumDepositOfUser::get();
		let amount: BalanceOf<T,I> = UNIT.saturated_into();
		let redeem_limit_time: BlockNumberFor<T> = 100u32.saturated_into();
		let unlock_limit_nums = 3;

		let investor: T::AccountId = account("lm", 0, 0);
		assert_ok!(T::MultiCurrency::deposit(REWARD_1, &investor, amount));
		assert_ok!(T::MultiCurrency::deposit(REWARD_2, &investor, amount));

		let caller: T::AccountId = whitelisted_caller();
		let double_amount = amount.saturating_mul(2u128.saturated_into());
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_1, &caller, double_amount));
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_2, &caller, double_amount));

		assert_ok!(LM::<T,I>::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, amount),
			vec![(REWARD_2, amount)].try_into().unwrap(),
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			0u128.saturated_into(),
			redeem_limit_time,
			unlock_limit_nums,
		));

		assert_ok!(LM::<T,I>::charge(RawOrigin::Signed(investor).into(), 0));

		assert_ok!(LM::<T,I>::deposit(RawOrigin::Signed(caller.clone()).into(), 0, double_amount));

		// Run to block
		run_to_block::<T,I>(1u32.saturated_into());

		assert_ok!(LM::<T,I>::redeem(RawOrigin::Signed(caller.clone()).into(), 0, amount));

	}: _(RawOrigin::Signed(caller.clone()), 0, 0)
	verify {
		let pool = LM::<T,I>::pool(0);
		let deposit_data = LM::<T,I>::user_deposit_data(0, caller.clone());
		assert!(pool.unwrap().pending_unlock_nums == 0);
		assert!(deposit_data.unwrap().pending_unlocks.len() == 0);
	}

	create_single_token_pool {
		let token = SINGLE_TOKEN_DEPOSIT;
		let main_reward = (REWARD_1.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT));
		let option_rewards = vec![(REWARD_2.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT))].try_into().unwrap();
		let duration = DAYS.into();
		let min_deposit_to_start = BalanceOf::<T,I>::unique_saturated_from(1_000 * UNIT);
		let after_block_to_start = 0u32.into();
		let redeem_limit_time = Zero::zero();
		let unlock_limit_nums = 0;
	}: _(RawOrigin::Root, token, main_reward,option_rewards,duration,min_deposit_to_start,after_block_to_start,redeem_limit_time,unlock_limit_nums)

	create_mining_pool {
		let trading_pair = MINING_TRADING_PAIR;
		let main_reward = (REWARD_1.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT));
		let option_rewards = vec![(REWARD_2.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT))].try_into().unwrap();
		let duration = DAYS.into();
		let min_deposit_to_start = BalanceOf::<T,I>::unique_saturated_from(1_000 * UNIT);
		let after_block_to_start = 0u32.into();
		let redeem_limit_time = Zero::zero();
		let unlock_limit_nums = 0;
	}: _(RawOrigin::Root, trading_pair, main_reward,option_rewards,duration,min_deposit_to_start,after_block_to_start,redeem_limit_time,unlock_limit_nums)

	create_farming_pool {
		let index = 2001;
		let first_slot = 13;
		let last_slot = 20;
		let main_reward = (REWARD_1.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT));
		let option_rewards = vec![(REWARD_2.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT))].try_into().unwrap();
		let duration = DAYS.into();
		let min_deposit_to_start = BalanceOf::<T,I>::unique_saturated_from(1_000 * UNIT);
		let after_block_to_start = 0u32.into();
		let redeem_limit_time = Zero::zero();
		let unlock_limit_nums = 0;
	}: _(RawOrigin::Root, index, first_slot,last_slot,main_reward,option_rewards,duration,min_deposit_to_start,after_block_to_start,redeem_limit_time,unlock_limit_nums)

	create_eb_farming_pool {
		let index = 2001;
		let first_slot = 13;
		let last_slot = 20;
		let main_reward = (REWARD_1.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT));
		let option_rewards = vec![(REWARD_2.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT))].try_into().unwrap();
		let duration = DAYS.into();
		let min_deposit_to_start = BalanceOf::<T,I>::unique_saturated_from(1_000 * UNIT);
		let after_block_to_start = 0u32.into();
	}: _(RawOrigin::Root, index, first_slot,last_slot,main_reward,option_rewards,duration,min_deposit_to_start,after_block_to_start)

	kill_pool {
		let trading_pair = MINING_TRADING_PAIR;
		let main_reward = (REWARD_1.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT));
		let option_rewards = vec![(REWARD_2.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT))].try_into().unwrap();
		let duration = DAYS.into();
		let min_deposit_to_start = BalanceOf::<T,I>::unique_saturated_from(1_000 * UNIT);
		let after_block_to_start = 0u32.into();
		let redeem_limit_time = Zero::zero();
		let unlock_limit_nums = 0;
		assert_ok!(LM::<T,I>::create_pool(
			trading_pair,
			main_reward,
			option_rewards,
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			after_block_to_start,
			redeem_limit_time,
			unlock_limit_nums,
		));
	}: _(RawOrigin::Root, 0)

	force_retire_pool {
		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(
			LM::<T, I>::lazy_migration_v2_0_0(
				RawOrigin::Signed(caller.clone()).into(),
			200)
		);
		let duration = T::MinimumDuration::get().saturating_add(1u128.saturated_into());
		let min_deposit_to_start = T::MinimumDepositOfUser::get();
		let amount: BalanceOf<T,I> = UNIT.saturated_into();

		let investor: T::AccountId = account("lm", 0, 0);
		assert_ok!(T::MultiCurrency::deposit(REWARD_1, &investor, amount));
		assert_ok!(T::MultiCurrency::deposit(REWARD_2, &investor, amount));

		let caller: T::AccountId = whitelisted_caller();
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_1, &caller, amount));
		assert_ok!(T::MultiCurrency::deposit(FARMING_DEPOSIT_2, &caller, amount));

		assert_ok!(LM::<T,I>::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, amount),
			vec![(REWARD_2, amount)].try_into().unwrap(),
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			0u128.saturated_into(),
			Zero::zero(),
			0,
		));

		assert_ok!(LM::<T,I>::charge(RawOrigin::Signed(investor).into(), 0));
	}: _(RawOrigin::Root, 0)

	edit_pool {
		let trading_pair = MINING_TRADING_PAIR;
		let main_reward = (REWARD_1.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT));
		let option_rewards = vec![(REWARD_2.into(), BalanceOf::<T,I>::unique_saturated_from(REWARD_AMOUNT))].try_into().unwrap();
		let duration = DAYS.into();
		let min_deposit_to_start = BalanceOf::<T,I>::unique_saturated_from(1_000 * UNIT);
		let after_block_to_start = 0u32.into();
		let redeem_limit_time = Zero::zero();
		let unlock_limit_nums = 0;
		let investor: T::AccountId =  whitelisted_caller();
		assert_ok!(LM::<T,I>::create_pool(
			trading_pair,
			main_reward,
			option_rewards,
			PoolType::Farming,
			duration,
			min_deposit_to_start,
			after_block_to_start,
			redeem_limit_time,
			unlock_limit_nums,
		));

	}: _(RawOrigin::Root, 0u32.into(),0u32.into(),0u32.into())
}

impl_benchmark_test_suite!(LM, crate::mock::new_test_ext(), crate::mock::Test);

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

#![cfg(test)]

use frame_support::{assert_err, assert_ok};

use crate::{mock::*, *};

#[test]
fn claim() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let (pid, _tokens) = init_no_gauge();
		// assert_eq!(Farming::shares_and_withdrawn_rewards(pid, ALICE), (0, tokens));
		assert_err!(Farming::claim(Origin::signed(ALICE), pid), Error::<Runtime>::InvalidPoolState);
		System::set_block_number(System::block_number() + 100);
		Farming::on_initialize(0);
		assert_ok!(Farming::claim(Origin::signed(ALICE), pid));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 2000);
		Farming::on_initialize(0);
		assert_ok!(Farming::claim(Origin::signed(ALICE), pid));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 3000);
		Farming::on_initialize(0);
		assert_ok!(Farming::close_pool(Origin::signed(ALICE), pid));
		assert_ok!(Farming::force_retire_pool(Origin::signed(ALICE), pid));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 5000);
		Farming::on_initialize(0);
		assert_err!(
			Farming::force_retire_pool(Origin::signed(ALICE), pid),
			Error::<Runtime>::InvalidPoolState
		);
	});
}

#[test]
fn deposit() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let (pid, tokens) = init_no_gauge();
		System::set_block_number(System::block_number() + 1);
		assert_ok!(Farming::deposit(Origin::signed(ALICE), pid, tokens.clone(), Some((100, 100))));
		System::set_block_number(System::block_number() + 1);
		assert_ok!(Farming::deposit(Origin::signed(ALICE), pid, 0, Some((100, 100))));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 800);
		let gauge_pool_info2 = GaugePoolInfo {
			pid,
			token: KSM,
			rewards:
				BTreeMap::<CurrencyIdOf<Runtime>, (BalanceOf<Runtime>, BalanceOf<Runtime>)>::new(),
			coefficient: Permill::from_percent(100),
			max_block: 1000,
			gauge_amount: 200,
			total_time_factor: 39900,
			gauge_last_block: System::block_number(),
			gauge_state: GaugeState::Bonded,
		};
		assert_eq!(Farming::gauge_pool_infos(0), gauge_pool_info2);
		Farming::on_initialize(0);
		Farming::on_initialize(0);
		System::set_block_number(System::block_number() + 1000);
	})
}

#[test]
fn withdraw() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let (pid, tokens) = init_no_gauge();
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 2000);
	})
}

#[test]
fn gauge() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let (pid, tokens) = init_gauge();
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 1900);
		Farming::on_initialize(0);
		System::set_block_number(System::block_number() + 1);
		Farming::on_initialize(0);
		assert_ok!(Farming::claim(Origin::signed(ALICE), pid));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 2919);
		Farming::on_initialize(0);
		System::set_block_number(System::block_number() + 10);
		assert_ok!(Farming::deposit(Origin::signed(ALICE), pid, tokens, Some((100, 100))));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 1819);
		System::set_block_number(System::block_number() + 20);
		assert_ok!(Farming::claim(Origin::signed(ALICE), pid));
		System::set_block_number(System::block_number() + 200);
		assert_ok!(Farming::claim(Origin::signed(ALICE), pid));
		// assert_err!(
		// 	Farming::claim(Origin::signed(ALICE), pid),
		// 	orml_tokens::Error::<Runtime>::BalanceTooLow
		// );
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 5482);
	})
}

fn init_gauge() -> (PoolId, BalanceOf<Runtime>) {
	let mut tokens_proportion_map = BTreeMap::<CurrencyIdOf<Runtime>, Permill>::new();
	tokens_proportion_map.entry(KSM).or_insert(Permill::from_percent(100));
	let tokens_proportion = vec![(KSM, Permill::from_percent(100))];

	// let mut tokens = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
	// tokens.entry(KSM).or_insert(1000);
	let tokens = 1000;
	let basic_rewards = vec![(KSM, 1000)];

	assert_ok!(Farming::create_farming_pool(
		Origin::signed(ALICE),
		tokens_proportion.clone(),
		basic_rewards.clone(),
		Some((KSM, Permill::from_percent(90), 1000)),
		0,
		0,
		0,
		0,
		5
	));

	let pid = 0;
	// let mut charge_rewards = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
	// let _ = charge_rewards.entry(KSM).or_insert(3000);
	let charge_rewards = vec![(KSM, 300000)];
	assert_ok!(Farming::charge(Origin::signed(BOB), pid, charge_rewards));
	// let keeper: AccountId = <Runtime as Config>::PalletId::get().into_sub_account(pid);
	// let starting_token_values: Vec<BalanceOf<Runtime>> = tokens.values().cloned().collect();
	// let pool_info = PoolInfo::reset(
	// 	keeper,
	// 	tokens_proportion_map.clone(),
	// 	basic_rewards.clone(),
	// 	PoolState::Charged,
	// 	Some(0),
	// 	0,
	// 	0,
	// 	0,
	// 	0,
	// );

	// assert_eq!(Farming::pool_infos(pid), pool_info);

	assert_ok!(Farming::deposit(Origin::signed(ALICE), pid, tokens.clone(), Some((100, 100))));
	(pid, tokens)
}

fn init_no_gauge() -> (PoolId, BalanceOf<Runtime>) {
	let mut tokens_proportion_map = BTreeMap::<CurrencyIdOf<Runtime>, Permill>::new();
	tokens_proportion_map.entry(KSM).or_insert(Permill::from_percent(100));
	let tokens_proportion = vec![(KSM, Permill::from_percent(100))];
	let tokens = 1000;
	// let mut basic_rewards = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
	// let _ = basic_rewards.entry(KSM).or_insert(1000);
	let basic_rewards = vec![(KSM, 1000)];

	assert_ok!(Farming::create_farming_pool(
		Origin::signed(ALICE),
		tokens_proportion.clone(),
		basic_rewards.clone(),
		Some((KSM, Permill::from_percent(100), 1000)),
		0,
		0,
		10,
		0,
		5
	));

	let pid = 0;
	let keeper: AccountId = <Runtime as Config>::Keeper::get().into_sub_account(pid);
	// let mut charge_rewards = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
	// let _ = charge_rewards.entry(KSM).or_insert(100000);
	let charge_rewards = vec![(KSM, 100000)];

	assert_ok!(Farming::charge(Origin::signed(BOB), pid, charge_rewards));
	assert_eq!(Tokens::free_balance(KSM, &keeper), 100000);

	// let pool_info = PoolInfo::reset(
	// 	keeper.clone(),
	// 	tokens_proportion_map.clone(),
	// 	basic_rewards.clone(),
	// 	PoolState::Charged,
	// 	Some(0),
	// 	0,
	// 	0,
	// 	0,
	// 	0,
	// );

	// assert_eq!(Farming::pool_infos(pid), pool_info);
	assert_ok!(Farming::deposit(Origin::signed(ALICE), pid, tokens.clone(), None));
	(pid, tokens)
}

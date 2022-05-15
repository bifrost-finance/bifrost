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

use frame_support::{assert_err, assert_noop, assert_ok};
pub use primitives::{VstokenConversionExchangeFee, VstokenConversionExchangeRate};
use sp_arithmetic::per_things::Percent;

use crate::{mock::*, *};

#[test]
fn claim() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let mut tokens = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
		tokens.entry(KSM).or_insert(1000);
		let mut basic_rewards = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
		let _ = basic_rewards.entry(KSM).or_insert(1000);

		assert_ok!(Farming::create_farming_pool(
			Origin::signed(ALICE),
			tokens.clone(),
			basic_rewards.clone(),
			Some(KSM),
			BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new(),
			0,
			0,
			0,
		));

		let pid = 0;
		let keeper: AccountId = <Runtime as Config>::PalletId::get().into_sub_account(pid);
		let mut charge_rewards = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
		let _ = charge_rewards.entry(KSM).or_insert(100000);
		assert_ok!(Farming::charge(Origin::signed(BOB), pid, charge_rewards));
		assert_eq!(Tokens::free_balance(KSM, &keeper), 100000);
		let starting_token_values: Vec<BalanceOf<Runtime>> = tokens.values().cloned().collect();

		let pool_info = PoolInfo::reset(
			keeper.clone(),
			tokens.clone(),
			basic_rewards.clone(),
			PoolState::Charged,
			starting_token_values,
			Some(0),
			BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new(),
			0,
			0,
			0,
		);

		assert_eq!(Farming::pool_infos(pid), pool_info);
		assert_ok!(Farming::deposit(Origin::signed(ALICE), pid, tokens.clone(), None));
		// assert_eq!(Farming::shares_and_withdrawn_rewards(pid, ALICE), (0, tokens));
		assert_err!(Farming::claim(Origin::signed(ALICE), pid), Error::<Runtime>::InvalidPoolState);
		Farming::on_initialize(0);
		assert_ok!(Farming::claim(Origin::signed(ALICE), pid));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 1000);
		Farming::on_initialize(0);
		assert_ok!(Farming::claim(Origin::signed(ALICE), pid));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 2000);
		Farming::on_initialize(0);
		assert_ok!(Farming::close_pool(Origin::signed(ALICE), pid));
		assert_ok!(Farming::force_retire_pool(Origin::signed(ALICE), pid));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 4000);
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
		let mut tokens = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
		tokens.entry(KSM).or_insert(1000);
		let mut basic_rewards = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
		let _ = basic_rewards.entry(KSM).or_insert(1000);

		assert_ok!(Farming::create_farming_pool(
			Origin::signed(ALICE),
			tokens.clone(),
			basic_rewards.clone(),
			Some(KSM),
			BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new(),
			0,
			0,
			0,
		));

		let pid = 0;
		let mut charge_rewards = BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new();
		let _ = charge_rewards.entry(KSM).or_insert(100000);
		assert_ok!(Farming::charge(Origin::signed(BOB), pid, charge_rewards));
		let keeper = <Runtime as Config>::PalletId::get().into_sub_account(pid);
		let starting_token_values: Vec<BalanceOf<Runtime>> = tokens.values().cloned().collect();
		let pool_info = PoolInfo::reset(
			keeper,
			tokens.clone(),
			basic_rewards.clone(),
			PoolState::Charged,
			starting_token_values,
			Some(0),
			BTreeMap::<CurrencyIdOf<Runtime>, BalanceOf<Runtime>>::new(),
			0,
			0,
			0,
		);

		assert_eq!(Farming::pool_infos(pid), pool_info);

		assert_ok!(Farming::deposit(Origin::signed(ALICE), pid, tokens.clone(), Some((100, 100))));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 900);
		// let current_block_number = frame_system::Pallet::<Runtime>::block_number();
		// let mut gauge_pool_info = GaugePoolInfo::new(pid, KSM, current_block_number);
		let gauge_pool_info = GaugePoolInfo {
			pid,
			token: KSM,
			gauge_amount: 100,
			total_time_factor: 10000,
			gauge_start_block: System::block_number(),
			gauge_last_block: System::block_number(),
			gauge_state: GaugeState::Bonded,
		};
		assert_eq!(Farming::gauge_pool_infos(0), gauge_pool_info);
		System::set_block_number(System::block_number() + 1);
		assert_ok!(Farming::deposit(Origin::signed(ALICE), pid, tokens.clone(), Some((100, 100))));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 800);
		let gauge_pool_info2 = GaugePoolInfo {
			pid,
			token: KSM,
			gauge_amount: 200,
			total_time_factor: 20000,
			gauge_start_block: System::block_number() - 1,
			gauge_last_block: System::block_number(),
			gauge_state: GaugeState::Bonded,
		};
		assert_eq!(Farming::gauge_pool_infos(0), gauge_pool_info2);
	})
}

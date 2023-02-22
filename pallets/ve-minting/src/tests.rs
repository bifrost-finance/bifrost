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

use crate::{mock::*, traits::VeMintingInterface, *};
use bifrost_asset_registry::AssetMetadata;
use bifrost_runtime_common::milli;
use frame_support::{assert_noop, assert_ok};
use node_primitives::TokenInfo;

#[test]
fn _checkpoint() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);
		let old_locked = LockedBalance { amount: 0, end: 0 };
		let new_locked = LockedBalance {
			amount: 10000000000000,
			end: System::block_number() + 365 * 86400 / 12,
		};

		assert_ok!(VeMinting::set_config(
			RuntimeOrigin::signed(ALICE),
			Some(0),
			Some(7 * 86400 / 12)
		));
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::_checkpoint(&BOB, old_locked, new_locked));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(7499936934420));
	});
}

#[test]
fn update_reward() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::set_config(
			RuntimeOrigin::signed(ALICE),
			Some(0),
			Some(7 * 86400 / 12)
		));

		System::set_block_number(System::block_number() + 20);
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::_create_lock(
			&BOB,
			100_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(174785436640));
		assert_ok!(VeMinting::deposit_for(&BOB, 100_000_000_000));
		assert_ok!(VeMinting::update_reward(Some(&BOB)));

		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(349578735500));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(349578735500));
	});
}

fn asset_registry() {
	let items = vec![(KSM, 10 * milli::<Runtime>(KSM)), (BNC, 10 * milli::<Runtime>(BNC))];
	for (currency_id, metadata) in items.iter().map(|(currency_id, minimal_balance)| {
		(
			currency_id,
			AssetMetadata {
				name: currency_id.name().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
				symbol: currency_id.symbol().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
				decimals: currency_id.decimals().unwrap_or_default(),
				minimal_balance: *minimal_balance,
			},
		)
	}) {
		AssetRegistry::do_register_metadata(*currency_id, &metadata).expect("Token register");
	}
}

#[test]
fn notify_reward_amount() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::set_config(
			RuntimeOrigin::signed(ALICE),
			Some(0),
			Some(7 * 86400 / 12)
		));

		System::set_block_number(System::block_number() + 20);
		System::set_block_number(System::block_number() + 20);
		assert_noop!(
			VeMinting::get_rewards(RuntimeOrigin::signed(BOB)),
			Error::<Runtime>::NoRewards
		);
		assert_ok!(VeMinting::_create_lock(
			&BOB,
			20_000_000_000,
			System::block_number() + 4 * 365 * 86400 / 12
		));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);
		assert_noop!(
			VeMinting::get_rewards(RuntimeOrigin::signed(BOB)),
			Error::<Runtime>::NoRewards
		);
		assert_ok!(VeMinting::increase_amount(RuntimeOrigin::signed(BOB), 80_000_000_000));
		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(399146883040));

		let rewards = vec![(KSM, 1_000_000_000)];
		assert_ok!(VeMinting::notify_rewards(
			RuntimeOrigin::signed(ALICE),
			ALICE,
			Some(7 * 86400 / 12),
			rewards.clone()
		));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);
		System::set_block_number(System::block_number() + 20);
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);
		assert_ok!(VeMinting::get_rewards(RuntimeOrigin::signed(BOB)));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 396819);
		System::set_block_number(System::block_number() + 7 * 86400 / 12);
		assert_ok!(VeMinting::get_rewards_inner(&BOB));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 999986398);
		assert_ok!(VeMinting::notify_rewards(
			RuntimeOrigin::signed(ALICE),
			ALICE,
			Some(7 * 86400 / 12),
			rewards
		));
		assert_ok!(VeMinting::_create_lock(
			&CHARLIE,
			100_000_000_000,
			System::block_number() + 4 * 365 * 86400 / 12
		));
		System::set_block_number(System::block_number() + 1 * 86400 / 12);
		assert_ok!(VeMinting::get_rewards_inner(&BOB));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 1071285014);
		assert_ok!(VeMinting::get_rewards_inner(&CHARLIE));
		assert_eq!(Tokens::free_balance(KSM, &CHARLIE), 71556583);
		System::set_block_number(System::block_number() + 7 * 86400 / 12);
		assert_ok!(VeMinting::get_rewards_inner(&CHARLIE));
		assert_eq!(Tokens::free_balance(KSM, &CHARLIE), 500898890);
		assert_ok!(VeMinting::get_rewards_inner(&BOB));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 1499073906);
	});
}

#[test]
fn create_lock_to_withdraw() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 7 * 86400 / 12); // a week
		assert_ok!(VeMinting::set_config(
			RuntimeOrigin::signed(ALICE),
			Some(4 * 365 * 86400 / 12),
			Some(7 * 86400 / 12)
		));

		log::debug!(
			"1System::block_number():{:?} total_supply:{:?}",
			System::block_number(),
			VeMinting::total_supply(System::block_number())
		);

		let rewards = vec![(KSM, 1000)];
		assert_ok!(VeMinting::notify_rewards(
			RuntimeOrigin::signed(ALICE),
			ALICE,
			Some(7 * 86400 / 12),
			rewards
		));
		log::debug!(
			"2System::block_number():{:?} total_supply:{:?}",
			System::block_number(),
			VeMinting::total_supply(System::block_number())
		);
		assert_noop!(
			VeMinting::increase_amount(RuntimeOrigin::signed(BOB), 50_000_000_000_000),
			Error::<Runtime>::LockNotExist
		);
		assert_noop!(
			VeMinting::increase_unlock_time(
				RuntimeOrigin::signed(BOB),
				System::block_number() + 365 * 86400 / 12
			),
			Error::<Runtime>::LockNotExist
		);
		assert_noop!(
			VeMinting::create_lock(
				RuntimeOrigin::signed(BOB),
				50_000_000_000_000,
				System::block_number() + 5 * 365 * 86400 / 12
			),
			Error::<Runtime>::Expired
		);
		assert_noop!(
			VeMinting::create_lock(
				RuntimeOrigin::signed(BOB),
				50_000_000_000_000,
				System::block_number() + 7 * 86400 / 12 - 1
			),
			Error::<Runtime>::Expired
		);
		assert_noop!(
			VeMinting::create_lock(
				RuntimeOrigin::signed(BOB),
				50_000,
				System::block_number() + 7 * 86400 / 12
			),
			Error::<Runtime>::BelowMinimumMint
		);
		assert_ok!(VeMinting::_create_lock(
			&BOB,
			50_000_000_000_000,
			System::block_number() + 365 * 86400 / 12
		));
		assert_noop!(
			VeMinting::_create_lock(
				&BOB,
				50_000_000_000_000,
				System::block_number() + 365 * 86400 / 12
			),
			Error::<Runtime>::LockExist
		);
		assert_noop!(
			VeMinting::increase_unlock_time(
				RuntimeOrigin::signed(BOB),
				System::block_number() + 5 * 365 * 86400 / 12
			),
			Error::<Runtime>::Expired
		);
		assert_eq!(VeMinting::balance_of_at(&BOB, System::block_number()), Ok(87397254003200));
		assert_eq!(VeMinting::balance_of_at(&BOB, System::block_number() - 10), Ok(0));
		assert_eq!(VeMinting::balance_of_at(&BOB, 0), Ok(0));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number() - 10)), Ok(0));
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(87397254003200));
		assert_noop!(
			VeMinting::increase_amount(RuntimeOrigin::signed(BOB), 50_000),
			Error::<Runtime>::BelowMinimumMint
		);
		assert_ok!(VeMinting::increase_amount(RuntimeOrigin::signed(BOB), 50_000_000_000_000));
		log::debug!(
			"3System::block_number():{:?} total_supply:{:?}",
			System::block_number(),
			VeMinting::total_supply(System::block_number())
		);
		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(174794515868800));
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(174794515868800));

		assert_ok!(VeMinting::withdraw(RuntimeOrigin::signed(ALICE)));
		assert_noop!(VeMinting::withdraw(RuntimeOrigin::signed(BOB)), Error::<Runtime>::Expired);
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(174794515868800));
		System::set_block_number(System::block_number() + 365 * 86400 / 12); // a year
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(100_000_000_000_000));
		assert_ok!(VeMinting::withdraw(RuntimeOrigin::signed(BOB)));
		assert_ok!(VeMinting::_withdraw(&BOB));
		log::debug!(
			"5System::block_number():{:?} total_supply:{:?}",
			System::block_number(),
			VeMinting::total_supply(System::block_number())
		);
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(0));
	});
}

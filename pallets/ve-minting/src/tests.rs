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

#![cfg(test)]

use crate::{mock::*, traits::VeMintingInterface, *};
use bifrost_asset_registry::AssetMetadata;
use bifrost_primitives::TokenInfo;
use bifrost_runtime_common::milli;
use frame_support::{assert_noop, assert_ok};

const POSITIONID0: u128 = 0;
const POOLID0: PoolId = 0;

#[test]
fn create_lock() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(VeMinting::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(VeMinting::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 4 * 365 * 86400 / 12,
		));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(9972575751740));
	});
}

#[test]
fn update_reward() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);
		assert_ok!(VeMinting::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));

		System::set_block_number(System::block_number() + 40);
		assert_ok!(VeMinting::create_lock_inner(
			&BOB,
			100_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(24928478880));
		assert_eq!(VeMinting::balance_of_position_current_block(0), Ok(24928478880));
		assert_ok!(VeMinting::deposit_for(&BOB, 0, 100_000_000_000));
		assert_ok!(VeMinting::update_reward(POOLID0, Some(&BOB), None)); // TODO

		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(49859578500));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(49859578500));
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
		assert_ok!(VeMinting::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));

		System::set_block_number(System::block_number() + 40);
		assert_ok!(VeMinting::get_rewards(RuntimeOrigin::signed(BOB))); // balance of veBNC is 0
		assert_ok!(VeMinting::create_lock_inner(
			&BOB,
			20_000_000_000,
			System::block_number() + 4 * 365 * 86400 / 12
		));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);
		// balance of veBNC is not 0
		assert_noop!(
			VeMinting::get_rewards(RuntimeOrigin::signed(BOB)),
			Error::<Runtime>::NoRewards
		);
		assert_ok!(VeMinting::increase_amount(RuntimeOrigin::signed(BOB), 0, 80_000_000_000));
		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(99715627680));

		let rewards = vec![(KSM, 1_000_000_000)];
		assert_ok!(VeMinting::notify_rewards(
			RuntimeOrigin::root(),
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
		assert_ok!(VeMinting::get_rewards_inner(POOLID0, &BOB, None));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 999986398);
		assert_ok!(VeMinting::notify_rewards(
			RuntimeOrigin::root(),
			ALICE,
			Some(7 * 86400 / 12),
			rewards
		));
		assert_ok!(VeMinting::create_lock_inner(&CHARLIE, 100_000_000_000, 4 * 365 * 86400 / 12));
		System::set_block_number(System::block_number() + 1 * 86400 / 12);
		assert_ok!(VeMinting::get_rewards_inner(POOLID0, &BOB, None));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 1071241763);
		assert_ok!(VeMinting::get_rewards_inner(POOLID0, &CHARLIE, None));
		assert_eq!(Tokens::free_balance(KSM, &CHARLIE), 71599834);
		System::set_block_number(System::block_number() + 7 * 86400 / 12);
		assert_ok!(VeMinting::get_rewards_inner(POOLID0, &CHARLIE, None));
		assert_eq!(Tokens::free_balance(KSM, &CHARLIE), 501203849);
		assert_ok!(VeMinting::get_rewards_inner(POOLID0, &BOB, None));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 1498768947);
	});
}

#[test]
fn create_lock_to_withdraw() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 7 * 86400 / 12); // a week
		assert_ok!(VeMinting::set_config(
			RuntimeOrigin::root(),
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
			RuntimeOrigin::root(),
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
			VeMinting::increase_amount(RuntimeOrigin::signed(BOB), POSITIONID0, 50_000_000_000_000),
			Error::<Runtime>::LockNotExist
		);
		assert_noop!(
			VeMinting::increase_unlock_time(
				RuntimeOrigin::signed(BOB),
				POSITIONID0,
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
				7 * 86400 / 12 - 1
			),
			Error::<Runtime>::Expired
		);
		assert_noop!(
			VeMinting::create_lock(RuntimeOrigin::signed(BOB), 50_000, 7 * 86400 / 12),
			Error::<Runtime>::BelowMinimumMint
		);
		assert_ok!(VeMinting::create_lock_inner(&BOB, 50_000_000_000_000, 365 * 86400 / 12));
		assert_noop!(
			VeMinting::increase_unlock_time(
				RuntimeOrigin::signed(BOB),
				POSITIONID0,
				System::block_number() + 5 * 365 * 86400 / 12
			),
			Error::<Runtime>::Expired
		);
		assert_eq!(VeMinting::balance_of_at(&BOB, System::block_number()), Ok(12465751334400));
		assert_eq!(VeMinting::balance_of_at(&BOB, System::block_number() - 10), Ok(0));
		assert_eq!(VeMinting::balance_of_at(&BOB, 0), Ok(0));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number() - 10)), Ok(0));
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(12465751334400));
		assert_noop!(
			VeMinting::increase_amount(RuntimeOrigin::signed(BOB), 0, 50_000),
			Error::<Runtime>::BelowMinimumMint
		);
		assert_ok!(VeMinting::increase_amount(RuntimeOrigin::signed(BOB), 0, 50_000_000_000_000));
		log::debug!(
			"3System::block_number():{:?} total_supply:{:?}",
			System::block_number(),
			VeMinting::total_supply(System::block_number())
		);
		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(24931505289600));
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(24931505289600));

		assert_noop!(
			VeMinting::withdraw(RuntimeOrigin::signed(ALICE), POSITIONID0),
			Error::<Runtime>::LockNotExist
		);
		assert_noop!(
			VeMinting::withdraw(RuntimeOrigin::signed(BOB), POSITIONID0),
			Error::<Runtime>::Expired
		);
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(24931505289600));
		System::set_block_number(System::block_number() + 2 * 365 * 86400 / 12);
		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(0));
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(0));
		assert_ok!(VeMinting::withdraw(RuntimeOrigin::signed(BOB), POSITIONID0));
		assert_ok!(VeMinting::withdraw_inner(&BOB, 0));
		log::debug!(
			"5System::block_number():{:?} total_supply:{:?}",
			System::block_number(),
			VeMinting::total_supply(System::block_number())
		);
		assert_eq!(VeMinting::balance_of(&BOB, None), Ok(0));
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(0));
	});
}

#[test]
fn overflow() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		assert_ok!(VeMinting::create_lock_inner(&BOB, 100_000_000_000_000, 77000));
		System::set_block_number(77001);
		assert_eq!(VeMinting::balance_of(&BOB, Some(77001)), Ok(0));
		assert_eq!(VeMinting::total_supply(System::block_number()), Ok(0));
	});
}

#[test]
fn deposit_markup_before_lock_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(VeMinting::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(VeMinting::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			1_000.into(),
			10_000_000_000_000.into()
		));
		assert_ok!(VeMinting::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_ok!(VeMinting::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::one()),
			Point { bias: 2518061148680, slope: 960806, block: 20, amount: 10100000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(2518061148680));
	});
}

#[test]
fn deposit_markup_after_lock_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(VeMinting::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(VeMinting::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(VeMinting::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			1_000.into(),
			10_000_000_000_000.into()
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(1)),
			Point { bias: 2493129668540, slope: 951293, block: 20, amount: 10000000000000 }
		);
		assert_ok!(VeMinting::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(2)),
			Point { bias: 2518061148680, slope: 960806, block: 20, amount: 10100000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(2518061148680));
	});
}

#[test]
fn withdraw_markup_after_lock_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(VeMinting::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(VeMinting::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(VeMinting::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			1_000.into(),
			10_000_000_000_000.into()
		));
		assert_ok!(VeMinting::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_ok!(VeMinting::withdraw_markup(RuntimeOrigin::signed(BOB), VBNC));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(2)),
			Point { bias: 2518061148680, slope: 960806, block: 20, amount: 10100000000000 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(3)),
			Point { bias: 2493129668540, slope: 951293, block: 20, amount: 10000000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(2493129668540));
	});
}

#[test]
fn redeem_unlock_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(VeMinting::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(VeMinting::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			1_000.into(),
			10_000_000_000_000.into()
		));
		assert_ok!(VeMinting::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(0));
		assert_ok!(VeMinting::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::one()),
			Point { bias: 2518061148680, slope: 960806, block: 20, amount: 10100000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(2518061148680));
		assert_ok!(VeMinting::redeem_unlock(RuntimeOrigin::signed(BOB), 0));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(0));
	});
}

#[test]
fn refresh_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(VeMinting::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(VeMinting::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			1_000.into(),
			10_000_000_000_000.into()
		));
		assert_ok!(VeMinting::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(0));
		assert_ok!(VeMinting::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(VeMinting::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			2_000.into(),
			10_000_000_000_000.into()
		));
		assert_ok!(VeMinting::refresh_inner(RuntimeOrigin::signed(BOB), VBNC));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::one()),
			Point { bias: 2518061148680, slope: 960806, block: 20, amount: 10100000000000 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(2)),
			Point { bias: 2542992628820, slope: 970319, block: 20, amount: 10200000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(2542992628820));
		assert_ok!(VeMinting::redeem_unlock(RuntimeOrigin::signed(BOB), 0));
		assert_eq!(VeMinting::balance_of(&BOB, Some(System::block_number())), Ok(0));
	});
}

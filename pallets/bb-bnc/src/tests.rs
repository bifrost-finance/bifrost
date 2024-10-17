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

use crate::{mock::*, traits::BbBNCInterface, *};
use bifrost_asset_registry::AssetMetadata;
use bifrost_primitives::TokenInfo;
use bifrost_runtime_common::milli;
use frame_support::{assert_noop, assert_ok};

const POSITIONID0: u128 = 0;
const POSITIONID1: u128 = 1;

#[test]
fn create_lock_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + (4 * 365 * 86400 - 5 * 86400) / 12,
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(1)),
			Point { bias: 9972575751740, slope: 951293, block: 20, amount: 10000000000000 }
		);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(9972575751740));
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(9972575751740));
	});
}

#[test]
fn create_multi_locks_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + (4 * 365 * 86400 - 5 * 86400) / 12,
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(1)),
			Point { bias: 9972575751740, slope: 951293, block: 20, amount: 10000000000000 }
		);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(9972575751740));
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(9972575751740));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			5_000_000_000_000,
			System::block_number() + (2 * 365 * 86400 - 5 * 86400) / 12,
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID1, U256::from(1)),
			Point { bias: 2493136560680, slope: 475646, block: 20, amount: 5000000000000 }
		);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(12465712312420));
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(12465712312420));
	});
}

#[test]
fn increase_unlock_time_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 7 * 86400 / 12);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + (3 * 365 * 86400 - 5 * 86400) / 12,
		));
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).end, 7963200);
		assert_noop!(
			BbBNC::increase_unlock_time(
				RuntimeOrigin::signed(BOB),
				POSITIONID0,
				System::block_number() + 365 * 86400 / 12
			),
			Error::<Runtime>::ArgumentsError
		);
		assert_ok!(BbBNC::increase_unlock_time(
			RuntimeOrigin::signed(BOB),
			POSITIONID0,
			(365 * 86400 - 5 * 86400) / 12
		),);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(1)),
			Point { bias: 7527391250400, slope: 951293, block: 50400, amount: 10000000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).end, 10584000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(10020539944800));
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(10020539944800));
	});
}

#[test]
fn increase_unlock_time_should_work2() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 7 * 86400 / 12);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + (3 * 365 * 86400 - 5 * 86400) / 12,
		));
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).end, 7963200);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(7527391250400));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			5_000_000_000_000,
			System::block_number() + (3 * 365 * 86400 - 5 * 86400) / 12,
		));
		assert_eq!(Locked::<Runtime>::get(POSITIONID1).end, 7963200);
		assert_noop!(
			BbBNC::increase_unlock_time(
				RuntimeOrigin::signed(BOB),
				POSITIONID0,
				System::block_number() + 365 * 86400 / 12
			),
			Error::<Runtime>::ArgumentsError
		);
		assert_noop!(
			BbBNC::increase_unlock_time(
				RuntimeOrigin::signed(BOB),
				POSITIONID1,
				System::block_number() + 365 * 86400 / 12
			),
			Error::<Runtime>::ArgumentsError
		);
		assert_ok!(BbBNC::increase_unlock_time(
			RuntimeOrigin::signed(BOB),
			POSITIONID0,
			(365 * 86400 - 5 * 86400) / 12
		));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(13784231613600));
		assert_ok!(BbBNC::increase_unlock_time(
			RuntimeOrigin::signed(BOB),
			POSITIONID1,
			(365 * 86400 - 5 * 86400) / 12
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(1)),
			Point { bias: 7527391250400, slope: 951293, block: 50400, amount: 10000000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).end, 10584000);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID1, U256::from(1)),
			Point { bias: 3763691668800, slope: 475646, block: 50400, amount: 5000000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID1).end, 10584000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(15030804650400));
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(15030804650400));
	});
}

#[test]
fn update_reward() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);
		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));

		System::set_block_number(System::block_number() + 40);
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			100_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_eq!(BbBNC::balance_of(&BOB, None), Ok(25407883680));
		assert_eq!(BbBNC::balance_of_position_current_block(0), Ok(25407883680));
		assert_ok!(BbBNC::deposit_for(&BOB, 0, 100_000_000_000));
		assert_ok!(BbBNC::update_reward(BB_BNC_SYSTEM_POOL_ID, Some(&BOB), None)); // TODO

		assert_eq!(BbBNC::balance_of(&BOB, None), Ok(50818438500));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(50818438500));
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
		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));

		System::set_block_number(System::block_number() + 40);
		assert_ok!(BbBNC::get_rewards(RuntimeOrigin::signed(BOB))); // balance of veBNC is 0
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			20_000_000_000,
			System::block_number() + (4 * 365 * 86400 - 7 * 86400) / 12,
		));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);
		// balance of veBNC is not 0
		assert_ok!(BbBNC::get_rewards(RuntimeOrigin::signed(BOB)));
		assert_ok!(BbBNC::increase_amount(RuntimeOrigin::signed(BOB), 0, 80_000_000_000));
		assert_eq!(BbBNC::balance_of(&BOB, None), Ok(99715627680));

		let rewards = vec![(KSM, 1_000_000_000)];
		assert_ok!(BbBNC::notify_rewards(
			RuntimeOrigin::root(),
			ALICE,
			Some(7 * 86400 / 12),
			rewards.clone()
		));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);
		System::set_block_number(System::block_number() + 20);
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);
		assert_ok!(BbBNC::get_rewards(RuntimeOrigin::signed(BOB)));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 396819);
		System::set_block_number(System::block_number() + 7 * 86400 / 12);
		assert_ok!(BbBNC::get_rewards_inner(BB_BNC_SYSTEM_POOL_ID, &BOB, None));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 999986398);
		assert_ok!(BbBNC::notify_rewards(
			RuntimeOrigin::root(),
			ALICE,
			Some(7 * 86400 / 12),
			rewards
		));
		assert_ok!(BbBNC::create_lock_inner(
			&CHARLIE,
			100_000_000_000,
			(4 * 365 * 86400 - 7 * 86400) / 12
		));
		System::set_block_number(System::block_number() + 1 * 86400 / 12);
		assert_ok!(BbBNC::get_rewards_inner(BB_BNC_SYSTEM_POOL_ID, &BOB, None));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 1071241763);
		assert_ok!(BbBNC::get_rewards_inner(BB_BNC_SYSTEM_POOL_ID, &CHARLIE, None));
		assert_eq!(Tokens::free_balance(KSM, &CHARLIE), 71599834);
		System::set_block_number(System::block_number() + 7 * 86400 / 12);
		assert_ok!(BbBNC::get_rewards_inner(BB_BNC_SYSTEM_POOL_ID, &CHARLIE, None));
		assert_eq!(Tokens::free_balance(KSM, &CHARLIE), 501203849);
		assert_ok!(BbBNC::get_rewards_inner(BB_BNC_SYSTEM_POOL_ID, &BOB, None));
		assert_eq!(Tokens::free_balance(KSM, &BOB), 1498768947);
	});
}

#[test]
fn create_lock_to_withdraw() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 7 * 86400 / 12); // a week
		assert_ok!(BbBNC::set_config(
			RuntimeOrigin::root(),
			Some(4 * 365 * 86400 / 12),
			Some(14 * 86400 / 12)
		));
		let rewards = vec![(KSM, 1000)];
		assert_ok!(BbBNC::notify_rewards(
			RuntimeOrigin::root(),
			ALICE,
			Some(7 * 86400 / 12),
			rewards
		));
		assert_noop!(
			BbBNC::increase_amount(RuntimeOrigin::signed(BOB), POSITIONID0, 50_000_000_000_000),
			Error::<Runtime>::LockNotExist
		);
		assert_noop!(
			BbBNC::increase_unlock_time(
				RuntimeOrigin::signed(BOB),
				POSITIONID0,
				System::block_number() + 365 * 86400 / 12
			),
			Error::<Runtime>::LockNotExist
		);
		assert_noop!(
			BbBNC::create_lock(
				RuntimeOrigin::signed(BOB),
				50_000_000_000_000,
				System::block_number() + 5 * 365 * 86400 / 12
			),
			Error::<Runtime>::ArgumentsError
		);
		assert_noop!(
			BbBNC::create_lock(RuntimeOrigin::signed(BOB), 50_000_000_000_000, 1),
			Error::<Runtime>::ArgumentsError
		);
		assert_noop!(
			BbBNC::create_lock(RuntimeOrigin::signed(BOB), 50_000, 7 * 86400 / 12),
			Error::<Runtime>::BelowMinimumMint
		);
		assert_eq!(Tokens::free_balance(VBNC, &BOB), 1000000000000000);
		assert_ok!(BbBNC::create_lock_inner(&BOB, 50_000_000_000_000, 365 * 86400 / 12));
		assert_noop!(
			BbBNC::increase_unlock_time(
				RuntimeOrigin::signed(BOB),
				POSITIONID0,
				System::block_number() + 5 * 365 * 86400 / 12
			),
			Error::<Runtime>::ArgumentsError
		);
		assert_eq!(BbBNC::balance_of_at(&BOB, System::block_number()), Ok(12705477321600));
		assert_eq!(BbBNC::balance_of_at(&BOB, System::block_number() - 10), Ok(0));
		assert_eq!(BbBNC::balance_of_at(&BOB, 0), Ok(0));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number() - 10)), Ok(0));
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(12705477321600));
		assert_noop!(
			BbBNC::increase_amount(RuntimeOrigin::signed(BOB), 0, 50_000),
			Error::<Runtime>::BelowMinimumMint
		);
		assert_ok!(BbBNC::increase_amount(RuntimeOrigin::signed(BOB), 0, 50_000_000_000_000));
		assert_eq!(BbBNC::balance_of(&BOB, None), Ok(25410957314400));
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(25410957314400));

		assert_noop!(
			BbBNC::withdraw(RuntimeOrigin::signed(ALICE), POSITIONID0),
			Error::<Runtime>::LockNotExist
		);
		assert_noop!(
			BbBNC::withdraw(RuntimeOrigin::signed(BOB), POSITIONID0),
			Error::<Runtime>::Expired
		);
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(25410957314400));
		System::set_block_number(System::block_number() + 2 * 365 * 86400 / 12);
		assert_eq!(BbBNC::balance_of(&BOB, None), Ok(0));
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(0));
		assert_ok!(BbBNC::withdraw(RuntimeOrigin::signed(BOB), POSITIONID0));
		assert_ok!(BbBNC::withdraw_inner(&BOB, 0));
		assert_ok!(BbBNC::withdraw_inner(&BOB, 1));
		assert_eq!(BbBNC::balance_of(&BOB, None), Ok(0));
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(0));
	});
}

#[test]
fn overflow() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		assert_ok!(BbBNC::create_lock_inner(&BOB, 100_000_000_000_000, 77000));
		System::set_block_number(77001);
		assert_eq!(BbBNC::balance_of(&BOB, Some(77001)), Ok(226398387663));
		assert_eq!(BbBNC::total_supply(System::block_number()), Ok(226398387663));
	});
}

#[test]
fn deposit_markup_before_lock_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			FixedU128::from_inner(100_000_000_000_000_000), // 0.1
			FixedU128::saturating_from_integer(1),
		));
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::one()),
			Point { bias: 2796030953200, slope: 1046740, block: 20, amount: 11003333333333 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(2796030953200));
	});
}

#[test]
fn deposit_markup_before_lock_should_work2() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			FixedU128::from_inner(100_000_000_000_000_000), // 0.1
			FixedU128::saturating_from_integer(1),
		));
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			15_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::one()),
			Point { bias: 2796030953200, slope: 1046740, block: 20, amount: 11003333333333 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID1, U256::one()),
			Point { bias: 4194046429800, slope: 1570110, block: 20, amount: 16504999999999 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(Locked::<Runtime>::get(POSITIONID1).amount, 15_000_000_000_000);
		assert_eq!(
			BbBNC::balance_of(&BOB, Some(System::block_number())),
			Ok(2796030953200 + 4194046429800)
		);
		assert_eq!(
			BbBNC::balance_of(&BOB, Some(System::block_number())),
			Ok(2796030953200 + 4194046429800)
		);
	});
}

#[test]
fn deposit_markup_after_lock_should_work2() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			MOVR,
			FixedU128::from_inner(500_000_000_000_000_000), // 0.5
			FixedU128::saturating_from_integer(1),
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(1)),
			Point { bias: 2541074835740, slope: 951293, block: 20, amount: 10_000_000_000_000 }
		);
		assert_eq!(
			Tokens::ensure_can_withdraw(VBNC, &BOB, 1_000_000_000_000_000 - 10_000_000_000_000)
				.is_ok(),
			true
		);
		assert_eq!(Tokens::ensure_can_withdraw(MOVR, &BOB, 10_000_000_000_000).is_ok(), true);
		assert_eq!(UserMarkupInfos::<Runtime>::get(BOB), None);
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), MOVR, 9_000_000_000_000));
		assert_eq!(
			UserMarkupInfos::<Runtime>::get(BOB),
			Some(UserMarkupInfo {
				old_markup_coefficient: FixedU128::from_inner(0),
				markup_coefficient: FixedU128::from_inner(950_000_000_000_000_000),
			})
		);
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), MOVR, 1_000_000_000_000));
		assert_eq!(
			UserMarkupInfos::<Runtime>::get(BOB),
			Some(UserMarkupInfo {
				old_markup_coefficient: FixedU128::from_inner(950_000_000_000_000_000),
				markup_coefficient: FixedU128::from_inner(1_000_000_000_000_000_000),
			})
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(2)),
			Point { bias: 4955097665960, slope: 1855022, block: 20, amount: 19500000000000 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(3)),
			Point { bias: 5082152342660, slope: 1902587, block: 20, amount: 20000000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(5082152342660));
	});
}

#[test]
fn deposit_markup_after_lock_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			FixedU128::from_inner(100_000_000_000_000_000), // 0.1
			FixedU128::saturating_from_integer(1),
		));

		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(1)),
			Point { bias: 2541074835740, slope: 951293, block: 20, amount: 10000000000000 }
		);
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(2)),
			Point { bias: 2796030953200, slope: 1046740, block: 20, amount: 11003333333333 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(2796030953200));
	});
}

#[test]
fn withdraw_markup_after_lock_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			FixedU128::from_inner(100_000_000_000_000_000), // 0.1
			FixedU128::saturating_from_integer(1),
		));
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_ok!(BbBNC::withdraw_markup(RuntimeOrigin::signed(BOB), VBNC));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(2)),
			Point { bias: 2796030953200, slope: 1046740, block: 20, amount: 11003333333333 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(3)),
			Point { bias: 2541074835740, slope: 951293, block: 20, amount: 10000000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(2541074835740));
	});
}

#[test]
fn redeem_unlock_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			VKSM,
			FixedU128::from_inner(FixedU128::DIV / 10), // 0.1
			FixedU128::saturating_from_integer(1),
		));
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), VKSM, 10_000_000_000_000));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(0));
		assert_eq!(Tokens::free_balance(VBNC, &BOB), 1000000000000000);
		assert_eq!(Tokens::ensure_can_withdraw(VBNC, &BOB, 1000000000000000).is_ok(), true);
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::one()),
			Point { bias: 5082152342660, slope: 1902587, block: 20, amount: 20000000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(5082152342660));
		assert_eq!(Tokens::free_balance(VBNC, &BOB), 1_000_000_000_000_000);
		assert_eq!(Tokens::ensure_can_withdraw(VBNC, &BOB, 1_000_000_000_000_000).is_ok(), false);
		assert_ok!(BbBNC::redeem_unlock(RuntimeOrigin::signed(BOB), 0));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(0));
		assert_eq!(Tokens::free_balance(VBNC, &BOB), 997451711199422);
		assert_eq!(Tokens::ensure_can_withdraw(VBNC, &BOB, 997451711199422).is_ok(), true);
	});
}

#[test]
fn withdraw_markup_after_lock_should_work3() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			15_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			FixedU128::from_inner(100_000_000_000_000_000), // 0.1
			FixedU128::saturating_from_integer(1),
		));
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_ok!(BbBNC::withdraw_markup(RuntimeOrigin::signed(BOB), VBNC));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(2)),
			Point { bias: 2796030953200, slope: 1046740, block: 20, amount: 11003333333333 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(3)),
			Point { bias: 2541074835740, slope: 951293, block: 20, amount: 10000000000000 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID1, U256::from(2)),
			Point { bias: 4194046429800, slope: 1570110, block: 20, amount: 16504999999999 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID1, U256::from(3)),
			Point { bias: 3811613589200, slope: 1426940, block: 20, amount: 15000000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(Locked::<Runtime>::get(POSITIONID1).amount, 15_000_000_000_000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(6352688424940));
	});
}

#[test]
fn redeem_unlock_after_360_days_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			VKSM,
			FixedU128::from_inner(FixedU128::DIV / 10), // 0.1
			FixedU128::saturating_from_integer(1),
		));
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), VKSM, 10_000_000_000_000));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(0));
		assert_eq!(Tokens::free_balance(VBNC, &BOB), 1000000000000000);
		assert_eq!(Tokens::ensure_can_withdraw(VBNC, &BOB, 1000000000000000).is_ok(), true);
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::one()),
			Point { bias: 5082152342660, slope: 1902587, block: 20, amount: 20000000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(5082152342660));
		assert_eq!(Tokens::free_balance(VBNC, &BOB), 1_000_000_000_000_000);
		assert_eq!(Tokens::ensure_can_withdraw(VBNC, &BOB, 1_000_000_000_000_000).is_ok(), false);
		System::set_block_number(System::block_number() + 360 * 86400 / 12);
		assert_ok!(BbBNC::redeem_unlock(RuntimeOrigin::signed(BOB), 0));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(0));
		assert_eq!(Tokens::free_balance(VBNC, &BOB), 999336664330082);
		assert_eq!(Tokens::ensure_can_withdraw(VBNC, &BOB, 999336664330082).is_ok(), true);
	});
}

#[test]
fn redeem_unlock_after_360_days_should_work2() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			VKSM,
			FixedU128::from_inner(FixedU128::DIV / 10), // 0.1
			FixedU128::saturating_from_integer(1),
		));
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), VKSM, 10_000_000_000_000));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(0));
		assert_eq!(Tokens::free_balance(VBNC, &BOB), 1000000000000000);
		assert_eq!(Tokens::ensure_can_withdraw(VBNC, &BOB, 1000000000000000).is_ok(), true);
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			15_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::one()),
			Point { bias: 5082152342660, slope: 1902587, block: 20, amount: 20000000000000 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID1, U256::one()),
			Point { bias: 7623229849580, slope: 2853881, block: 20, amount: 30000000000000 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(Locked::<Runtime>::get(POSITIONID1).amount, 15_000_000_000_000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(12705382192240));
		assert_eq!(Tokens::free_balance(VBNC, &BOB), 1_000_000_000_000_000);
		assert_eq!(Tokens::ensure_can_withdraw(VBNC, &BOB, 1_000_000_000_000_000).is_ok(), false);
		System::set_block_number(System::block_number() + 360 * 86400 / 12);
		assert_ok!(BbBNC::redeem_unlock(RuntimeOrigin::signed(BOB), POSITIONID0));
		assert_ok!(BbBNC::redeem_unlock(RuntimeOrigin::signed(BOB), POSITIONID1));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(0));
		assert_eq!(Tokens::free_balance(VBNC, &BOB), 998341660825205);
		assert_eq!(Tokens::ensure_can_withdraw(VBNC, &BOB, 998341660825205).is_ok(), true);
	});
}

#[test]
fn refresh_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		asset_registry();
		System::set_block_number(System::block_number() + 20);

		assert_ok!(BbBNC::set_config(RuntimeOrigin::root(), Some(0), Some(7 * 86400 / 12)));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			FixedU128::from_inner(100_000_000_000_000_000), // 0.1
			FixedU128::saturating_from_integer(1),
		));
		assert_ok!(BbBNC::deposit_markup(RuntimeOrigin::signed(BOB), VBNC, 10_000_000_000_000));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(0));
		assert_ok!(BbBNC::create_lock_inner(
			&BOB,
			10_000_000_000_000,
			System::block_number() + 365 * 86400 / 12,
		));
		assert_ok!(BbBNC::set_markup_coefficient(
			RuntimeOrigin::root(),
			VBNC,
			FixedU128::from_inner(200_000_000_000_000_000), // 0.2
			FixedU128::saturating_from_integer(1),
		));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::one()),
			Point { bias: 2796030953200, slope: 1046740, block: 20, amount: 11003333333333 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(2)),
			Point { bias: 0, slope: 0, block: 0, amount: 0 }
		);
		assert_ok!(BbBNC::refresh_inner(VBNC));
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::one()),
			Point { bias: 2796030953200, slope: 1046740, block: 20, amount: 11003333333333 }
		);
		assert_eq!(
			UserPointHistory::<Runtime>::get(POSITIONID0, U256::from(2)),
			Point { bias: 3050984399480, slope: 1142186, block: 20, amount: 12006666666666 }
		);
		assert_eq!(Locked::<Runtime>::get(POSITIONID0).amount, 10_000_000_000_000);
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(3050984399480));
		assert_ok!(BbBNC::redeem_unlock(RuntimeOrigin::signed(BOB), 0));
		assert_eq!(BbBNC::balance_of(&BOB, Some(System::block_number())), Ok(0));
	});
}

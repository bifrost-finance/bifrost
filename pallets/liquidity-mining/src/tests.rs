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

use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use frame_system::pallet_prelude::OriginFor;
use node_primitives::Balance;

use crate::{mock::*, Error, PoolId, PoolState, PoolType};

#[test]
fn create_farming_pool_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_farming_pool(
			Some(CREATOR).into(),
			2001,
			13,
			20,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000 * UNIT,
			0
		));

		let pool = LM::pool(0).unwrap();

		assert_eq!(pool.r#type, PoolType::Farming);

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let reserved = per_block * DAYS as Balance;
		let free = REWARD_AMOUNT - reserved;

		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).free, free);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, reserved);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).free, free);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, reserved);
	});
}

#[test]
fn create_mining_pool_should_work() {
	// TODO
}

#[test]
fn increase_pid_when_create_pool_should_work() {
	new_test_ext().execute_with(|| {
		const NUM: PoolId = 8;
		for pid in 0..NUM {
			assert_ok!(LM::create_pool(
				Some(CREATOR).into(),
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT / NUM),
				vec![(REWARD_2, REWARD_AMOUNT / NUM)],
				PoolType::Farming,
				DAYS,
				1_000 * UNIT,
				0
			));
			let pool = LM::pool(pid).unwrap();
			assert_eq!(pool.pool_id, pid);
		}
	});
}

#[test]
fn create_pool_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LM::create_pool(
				Origin::root(),
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)],
				PoolType::Farming,
				DAYS,
				1_000 * UNIT,
				0
			),
			DispatchError::BadOrigin,
		);

		assert_noop!(
			LM::create_pool(
				Origin::none(),
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)],
				PoolType::Farming,
				DAYS,
				1_000 * UNIT,
				0
			),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn create_pool_with_duplicate_trading_pair_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LM::create_pool(
				Some(CREATOR).into(),
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_1),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)],
				PoolType::Farming,
				DAYS,
				1_000 * UNIT,
				0
			),
			Error::<T>::InvalidTradingPair,
		);
	});
}

#[test]
fn create_pool_with_too_small_duration_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LM::create_pool(
				Some(CREATOR).into(),
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)],
				PoolType::Farming,
				MinimumDuration::get() - 1,
				1_000 * UNIT,
				0
			),
			Error::<T>::InvalidDuration,
		);
	});
}

#[test]
fn create_pool_with_wrong_condition_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LM::create_pool(
				Some(CREATOR).into(),
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)],
				PoolType::Farming,
				DAYS,
				MinimumDeposit::get() - 1,
				0
			),
			Error::<T>::InvalidDepositLimit,
		);

		assert_noop!(
			LM::create_pool(
				Some(CREATOR).into(),
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)],
				PoolType::Farming,
				DAYS,
				MaximumDepositInPool::get() + 1,
				0
			),
			Error::<T>::InvalidDepositLimit,
		);
	});
}

#[test]
fn create_pool_with_too_small_per_block_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LM::create_pool(
				Some(CREATOR).into(),
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)],
				PoolType::Farming,
				(REWARD_AMOUNT + 1) as BlockNumber,
				1_000 * UNIT,
				0
			),
			Error::<T>::InvalidRewardPerBlock,
		);
	});
}

#[test]
fn create_pool_with_duplicate_reward_should_fail() {
	new_test_ext().execute_with(|| {
		let result = LM::create_pool(
			Some(CREATOR).into(),
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_1, REWARD_AMOUNT)],
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0,
		);
		assert_noop!(result, Error::<T>::DuplicateReward,);
	});
}

#[test]
fn approve_pool_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_pool(
			Some(CREATOR).into(),
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.state, PoolState::UnderAudit);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.state, PoolState::Approved);
	});
}

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

use frame_support::{
	assert_noop, assert_ok,
	dispatch::DispatchError,
	sp_runtime::{FixedPointNumber, FixedU128},
	traits::Hooks,
};
use node_primitives::{Balance, CurrencyId, TokenSymbol};

use crate::{mock::*, Error, PoolId, PoolInfo, PoolState, PoolType, TotalPoolInfos};

fn run_to_block(n: BlockNumber) {
	while System::block_number() < n {
		LM::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		LM::on_initialize(System::block_number());
	}
}

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
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000 * UNIT,
			0
		));

		let pool = LM::pool(0).unwrap();

		assert_eq!(pool.r#type, PoolType::Mining);

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
fn create_mining_pool_with_wrong_currency_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LM::create_mining_pool(
				Some(CREATOR).into(),
				(
					CurrencyId::VSBond(RelayChainTokenSymbol::get(), 2001, 13, 20),
					CurrencyId::VSToken(RelayChainTokenSymbol::get()),
				),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)],
				DAYS,
				1_000 * UNIT,
				0,
			),
			Error::<T>::InvalidTradingPair,
		);

		assert_noop!(
			LM::create_mining_pool(
				Some(CREATOR).into(),
				(
					CurrencyId::LPToken(TokenSymbol::KSM, 1u8, TokenSymbol::DOT, 2u8),
					CurrencyId::VSToken(RelayChainTokenSymbol::get()),
				),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)],
				DAYS,
				1_000 * UNIT,
				0,
			),
			Error::<T>::InvalidTradingPair,
		);
	});
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

		assert!(LM::approved_pids().contains(&0));
	});
}

#[test]
fn approve_pool_with_wrong_origin_should_fail() {
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

		assert_noop!(LM::approve_pool(Some(TC_MEMBER_1).into(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::approve_pool(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::approve_pool(Origin::none(), 0), DispatchError::BadOrigin);
	});
}

#[test]
fn approve_pool_with_wrong_state_should_fail() {
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

		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_noop!(
			LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0),
			Error::<T>::InvalidPoolState
		);
	});
}

#[test]
fn approve_pool_exceed_maximum_should_fail() {
	new_test_ext().execute_with(|| {
		for i in 0..MaximumApproved::get() as u128 {
			assert_ok!(LM::create_pool(
				Some(CREATOR).into(),
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT / (MaximumApproved::get() + 1) as u128),
				vec![(REWARD_2, REWARD_AMOUNT / (MaximumApproved::get() + 1) as u128)],
				PoolType::Farming,
				DAYS,
				1_000 * UNIT,
				0
			));

			assert_ok!(LM::approve_pool(
				pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
				i
			));

			assert!(LM::approved_pids().contains(&i));
		}

		assert_ok!(LM::create_pool(
			Some(CREATOR).into(),
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT / (MaximumApproved::get() + 1) as u128),
			vec![(REWARD_2, REWARD_AMOUNT / (MaximumApproved::get() + 1) as u128)],
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));

		assert_noop!(
			LM::approve_pool(
				pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
				MaximumApproved::get() as u128,
			),
			Error::<T>::ExceedMaximumApproved
		);

		assert!(!LM::approved_pids().contains(&(MaximumApproved::get() as u128)));
	});
}

#[test]
fn kill_pool_should_work() {
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

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let reserved = per_block * DAYS as Balance;
		let free = REWARD_AMOUNT - reserved;

		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).free, free);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, reserved);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).free, free);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, reserved);

		assert_ok!(LM::kill_pool(Some(CREATOR).into(), 0));

		assert!(!TotalPoolInfos::<T>::contains_key(0));

		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, 0);
	});
}

#[test]
fn kill_pool_with_wrong_origin_should_fail() {
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

		assert_noop!(LM::kill_pool(Some(USER_1).into(), 0), Error::<T>::InvalidPoolOwner);
		assert_noop!(LM::kill_pool(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::kill_pool(Origin::none(), 0), DispatchError::BadOrigin);
	});
}

#[test]
fn kill_pool_with_wrong_state_should_fail() {
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

		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_noop!(LM::kill_pool(Some(CREATOR).into(), 0), Error::<T>::InvalidPoolState);
	});
}

#[test]
fn deposit_to_mining_pool_approved_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		let deposit = 1_000_000 as Balance;
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, deposit));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, deposit));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 2 * deposit);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.deposit, 2 * deposit);

		let deposit_data = LM::user_deposit_data(USER_1, 0).unwrap();
		assert_eq!(deposit_data.deposit, 2 * deposit);
	});
}

#[test]
fn deposit_to_farming_pool_approved_should_work() {
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

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		let deposit = 1_000_000 as Balance;
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, deposit));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, deposit));

		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).frozen, 2 * deposit);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).frozen, 2 * deposit);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).reserved, 0);

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.deposit, 2 * deposit);

		let deposit_data = LM::user_deposit_data(USER_1, 0).unwrap();
		assert_eq!(deposit_data.deposit, 2 * deposit);
	});
}

#[test]
fn startup_pool_meet_conditions_should_auto_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT));

		run_to_block(101);

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.state, PoolState::Ongoing);

		assert!(!LM::approved_pids().contains(&0));
	});
}

#[test]
fn deposit_to_pool_ongoing_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT));

		run_to_block(101);

		assert_ok!(LM::deposit(Some(USER_2).into(), 0, DEPOSIT_AMOUNT));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.deposit, 2 * DEPOSIT_AMOUNT);

		let deposit_data = LM::user_deposit_data(USER_2, 0).unwrap();
		assert_eq!(deposit_data.deposit, DEPOSIT_AMOUNT);
	});
}

#[test]
fn deposit_to_pool_ongoing_with_init_deposit_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, 1_000_000));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, 1_000_000));

		run_to_block(200);

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, 1_000_000));

		let per_block: Balance = REWARD_AMOUNT / DAYS as Balance;
		let pbpd = FixedU128::from((per_block, 2 * 1_000_000));
		let reward_to_user_1 =
			(pbpd * (100 * 1_000_000).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, reward_to_user_1);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, reward_to_user_1);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		let pool: PoolInfo<T> = LM::pool(0).unwrap();
		assert_eq!(pool.deposit, 3_000_000);

		for (_rtoken, reward) in pool.rewards.iter() {
			assert_eq!(reward.claimed, reward_to_user_1);
		}

		let deposit_data_1 = LM::user_deposit_data(USER_1, 0).unwrap();
		assert_eq!(deposit_data_1.deposit, 2_000_000);
		let deposit_data_2 = LM::user_deposit_data(USER_2, 0).unwrap();
		assert_eq!(deposit_data_2.deposit, 1_000_000);
	});
}

#[test]
fn double_deposit_to_pool_ongoing_in_diff_block_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, 1_000_000));

		run_to_block(200);

		assert_ok!(LM::deposit(Some(USER_2).into(), 0, 1_000_000));

		System::set_block_number(300);

		assert_ok!(LM::deposit(Some(USER_2).into(), 0, 1_000_000));

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let pbpd = FixedU128::from((per_block, 2 * 1_000_000));
		let reward_to_user_2 =
			(pbpd * (100 * 1_000_000).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, reward_to_user_2);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, reward_to_user_2);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);

		let pool: PoolInfo<T> = LM::pool(0).unwrap();
		assert_eq!(pool.deposit, 3_000_000);

		for (_rtoken, reward) in pool.rewards.iter() {
			assert_eq!(reward.claimed, reward_to_user_2);
		}

		let deposit_data_1 = LM::user_deposit_data(USER_1, 0).unwrap();
		assert_eq!(deposit_data_1.deposit, 1_000_000);
		let deposit_data_2 = LM::user_deposit_data(USER_2, 0).unwrap();
		assert_eq!(deposit_data_2.deposit, 2_000_000);
	});
}

#[test]
fn double_deposit_to_pool_ongoing_in_same_block_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, 1_000_000));

		run_to_block(200);

		assert_ok!(LM::deposit(Some(USER_2).into(), 0, 1_000_000));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, 1_000_000));

		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);

		let pool: PoolInfo<T> = LM::pool(0).unwrap();
		assert_eq!(pool.deposit, 3_000_000);

		for (_rtoken, reward) in pool.rewards.iter() {
			assert_eq!(reward.claimed, 0);
		}

		let deposit_data_1 = LM::user_deposit_data(USER_1, 0).unwrap();
		assert_eq!(deposit_data_1.deposit, 1_000_000);
		let deposit_data_2 = LM::user_deposit_data(USER_2, 0).unwrap();
		assert_eq!(deposit_data_2.deposit, 2_000_000);
	});
}

#[test]
fn deposit_with_wrong_pid_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(LM::deposit(Some(USER_1).into(), 0, 1_000_000), Error::<T>::InvalidPoolId);

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_noop!(LM::deposit(Some(USER_1).into(), 1, 1_000_000), Error::<T>::InvalidPoolId);
	});
}

#[test]
fn deposit_with_wrong_state_should_fail() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000_000,
			0
		));

		assert_noop!(LM::deposit(Some(USER_1).into(), 0, 1_000_000), Error::<T>::InvalidPoolState);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, 1_000_000));

		run_to_block(100 + DAYS);

		let result = LM::deposit(Some(USER_1).into(), 0, 1_000_000);
		assert_noop!(result, Error::<T>::InvalidPoolState);
	});
}

#[test]
fn deposit_too_little_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(LM::deposit(Some(USER_1).into(), 0, 1_000_000), Error::<T>::InvalidPoolId);

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_noop!(
			LM::deposit(Some(USER_1).into(), 0, MinimumDeposit::get() - 1),
			Error::<T>::TooLowToDeposit
		);
	});
}

#[test]
fn deposit_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(LM::deposit(Some(USER_1).into(), 0, 1_000_000), Error::<T>::InvalidPoolId);

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_noop!(LM::deposit(Origin::root(), 0, 1_000_000), DispatchError::BadOrigin);
		assert_noop!(LM::deposit(Origin::none(), 0, 1_000_000), DispatchError::BadOrigin);
	});
}

#[test]
fn deposit_exceed_the_limit_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(LM::deposit(Some(USER_1).into(), 0, 1_000_000), Error::<T>::InvalidPoolId);

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, 1_000_000));
		assert_noop!(
			LM::deposit(Some(RICHER).into(), 0, MaximumDepositInPool::get() + 1),
			Error::<T>::ExceedMaximumDeposit
		);
	});
}

#[test]
fn redeem_from_pool_ongoing_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, UNIT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, UNIT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		run_to_block(100);

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let pbpd = FixedU128::from((per_block, 2 * UNIT));
		let rewarded = (pbpd * (100 * UNIT).into()).into_inner() / FixedU128::accuracy();

		assert_ok!(LM::redeem(Some(USER_1).into(), 0));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);
		assert!(LM::user_deposit_data(USER_1, 0).is_none());

		assert_ok!(LM::redeem(Some(USER_2).into(), 0));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, MinimumDeposit::get());
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);
		assert_eq!(LM::user_deposit_data(USER_2, 0).unwrap().deposit, MinimumDeposit::get());

		assert_eq!(LM::pool(0).unwrap().deposit, MinimumDeposit::get());
	});
}

#[test]
fn redeem_from_pool_retired_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, UNIT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, UNIT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		run_to_block(DAYS);

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let pbpd = FixedU128::from((per_block, 2 * UNIT));
		let rewarded =
			(pbpd * (DAYS as Balance * UNIT).into()).into_inner() / FixedU128::accuracy();

		assert_ok!(LM::redeem(Some(USER_1).into(), 0));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);
		assert!(LM::user_deposit_data(USER_1, 0).is_none());

		assert_ok!(LM::redeem(Some(USER_2).into(), 0));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);
		assert!(LM::user_deposit_data(USER_2, 0).is_none());

		assert!(LM::pool(0).is_none());

		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).free, REWARD_AMOUNT - 2 * rewarded);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).free, REWARD_AMOUNT - 2 * rewarded);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, 0);
	});
}

#[test]
fn double_redeem_from_pool_in_diff_state_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, UNIT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, UNIT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		run_to_block(100);

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let pbpd = FixedU128::from((per_block, 2 * UNIT));
		let old_rewarded = (pbpd * (100 * UNIT).into()).into_inner() / FixedU128::accuracy();

		assert_ok!(LM::redeem(Some(USER_1).into(), 0));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, old_rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, old_rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);
		assert!(LM::user_deposit_data(USER_1, 0).is_none());

		assert_ok!(LM::redeem(Some(USER_2).into(), 0));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, MinimumDeposit::get());
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, old_rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, old_rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);
		assert_eq!(LM::user_deposit_data(USER_2, 0).unwrap().deposit, MinimumDeposit::get());

		assert_eq!(LM::pool(0).unwrap().deposit, MinimumDeposit::get());

		// USER_2 didn't remember to redeem until the seventh day
		run_to_block(7 * DAYS);
		let pbpd = FixedU128::from((per_block, MinimumDeposit::get()));
		let new_rewarded = (pbpd * ((DAYS - 100) as Balance * MinimumDeposit::get()).into())
			.into_inner() /
			FixedU128::accuracy();

		assert_ok!(LM::redeem(Some(USER_2).into(), 0));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, old_rewarded + new_rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, old_rewarded + new_rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);
		assert!(LM::user_deposit_data(USER_2, 0).is_none());

		assert!(LM::pool(0).is_none());

		assert_eq!(
			Tokens::accounts(CREATOR, REWARD_1).free,
			REWARD_AMOUNT - (2 * old_rewarded + new_rewarded)
		);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, 0);
		assert_eq!(
			Tokens::accounts(CREATOR, REWARD_2).free,
			REWARD_AMOUNT - (2 * old_rewarded + new_rewarded)
		);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, 0);
	});
}

#[test]
fn redeem_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(LM::redeem(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::redeem(Origin::none(), 0), DispatchError::BadOrigin);
	});
}

#[test]
fn redeem_with_wrong_pid_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(100);

		assert_noop!(LM::redeem(Some(USER_1).into(), 1), Error::<T>::InvalidPoolId);
	});
}

#[test]
fn redeem_with_wrong_state_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(LM::redeem(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		assert_noop!(LM::redeem(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);
	});
}

#[test]
fn redeem_without_deposit_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(LM::redeem(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		run_to_block(100);

		assert_ok!(LM::redeem(Some(USER_1).into(), 0));
		assert_noop!(LM::redeem(Some(USER_1).into(), 0), Error::<T>::NoDepositOfUser);
	});
}

#[test]
fn redeem_all_deposit_from_pool_ongoing_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(LM::redeem(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(100);

		assert_ok!(LM::redeem(Some(USER_1).into(), 0));
		let result = LM::redeem(Some(USER_1).into(), 0);
		assert_noop!(result, Error::<T>::TooLowDepositInPoolToRedeem);
	});
}

#[test]
fn claim_from_pool_ongoing_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		run_to_block(100);

		assert_ok!(LM::claim(Some(USER_1).into(), 0));

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let reserved = per_block * DAYS as Balance;
		let free = REWARD_AMOUNT - reserved;

		let pbpd = FixedU128::from((per_block, 2 * UNIT));
		let rewarded: Balance = (pbpd * (100 * UNIT).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).free, free);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, reserved - rewarded);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).free, free);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, reserved - rewarded);

		let pool: PoolInfo<T> = LM::pool(0).unwrap();
		assert_eq!(pool.rewards.get(&REWARD_1).unwrap().claimed, rewarded);
		assert_eq!(pool.rewards.get(&REWARD_2).unwrap().claimed, rewarded);
	});
}

#[test]
fn claim_from_pool_retired_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		run_to_block(DAYS);

		assert_ok!(LM::claim(Some(USER_1).into(), 0));

		let per_block = REWARD_AMOUNT / DAYS as Balance;

		let reserved = per_block * DAYS as Balance;
		let free = REWARD_AMOUNT - reserved;

		let pbpd = FixedU128::from((per_block, 2 * UNIT));
		let rewarded: Balance =
			(pbpd * (DAYS as Balance * UNIT).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).free, free);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, reserved - rewarded);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).free, free);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, reserved - rewarded);

		let pool: PoolInfo<T> = LM::pool(0).unwrap();
		assert_eq!(pool.rewards.get(&REWARD_1).unwrap().claimed, rewarded);
		assert_eq!(pool.rewards.get(&REWARD_2).unwrap().claimed, rewarded);
	});
}

#[test]
fn claim_with_wrong_pid_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		run_to_block(DAYS);

		assert_noop!(LM::claim(Some(USER_1).into(), 1), Error::<T>::InvalidPoolId);
	});
}

#[test]
fn claim_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		run_to_block(DAYS);

		assert_noop!(LM::claim(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::claim(Origin::none(), 0), DispatchError::BadOrigin);
	});
}

#[test]
fn claim_with_wrong_state_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(LM::claim(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);
		assert_noop!(LM::claim(Some(USER_2).into(), 0), Error::<T>::InvalidPoolState);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		assert_noop!(LM::claim(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);
		assert_noop!(LM::claim(Some(USER_2).into(), 0), Error::<T>::InvalidPoolState);

		run_to_block(DAYS);
	});
}

#[test]
fn claim_without_deposit_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(DAYS);

		let result = LM::claim(Some(USER_2).into(), 0);
		assert_noop!(result, Error::<T>::NoDepositOfUser);
	});
}

#[test]
fn double_claim_in_same_block_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(DAYS);

		assert_ok!(LM::claim(Some(USER_1).into(), 0));
		assert_noop!(LM::claim(Some(USER_1).into(), 0), Error::<T>::TooShortBetweenTwoClaim);
	});
}

#[test]
fn force_retire_pool_approved_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		assert_ok!(LM::force_retire_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			0
		));

		assert_noop!(LM::deposit(Some(RICHER).into(), 0, UNIT), Error::<T>::InvalidPoolState);

		assert_noop!(LM::claim(Some(USER_1).into(), 0), Error::<T>::TooShortBetweenTwoClaim);
		assert_noop!(LM::claim(Some(USER_2).into(), 0), Error::<T>::TooShortBetweenTwoClaim);
		assert_ok!(LM::redeem(Some(USER_1).into(), 0));
		assert_ok!(LM::redeem(Some(USER_2).into(), 0));

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, UNIT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, UNIT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, 0);

		assert!(LM::pool(0).is_none());
		assert!(LM::user_deposit_data(USER_1, 0).is_none());
		assert!(LM::user_deposit_data(USER_2, 0).is_none());
	});
}

#[test]
fn force_retire_pool_ongoing_should_work() {
	const PER_BLOCK: Balance = REWARD_AMOUNT / DAYS as Balance;

	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(100);

		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		run_to_block(200);

		assert_ok!(LM::force_retire_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			0
		));
		assert_noop!(LM::deposit(Some(RICHER).into(), 0, UNIT), Error::<T>::InvalidPoolState);

		assert_ok!(LM::claim(Some(USER_1).into(), 0));
		assert_ok!(LM::claim(Some(USER_2).into(), 0));
		assert_ok!(LM::redeem(Some(USER_1).into(), 0));
		assert_ok!(LM::redeem(Some(USER_2).into(), 0));

		let pbpd_1 = FixedU128::from((PER_BLOCK, UNIT));
		let reward_step_1 = (pbpd_1 * (100 * UNIT).into()).into_inner() / FixedU128::accuracy();

		let pbpd_2 = FixedU128::from((PER_BLOCK, 2 * UNIT));
		let reward_step_2 = (pbpd_2 * (100 * UNIT).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, reward_step_1 + reward_step_2);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, reward_step_1 + reward_step_2);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, reward_step_2);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, reward_step_2);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, UNIT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, UNIT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		let remain = REWARD_AMOUNT - (reward_step_1 + 2 * reward_step_2);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).free, remain);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).free, remain);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, 0);

		assert!(LM::pool(0).is_none());
		assert!(LM::user_deposit_data(USER_1, 0).is_none());
		assert!(LM::user_deposit_data(USER_2, 0).is_none());
	});
}

#[test]
fn force_retire_pool_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_noop!(LM::force_retire_pool(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::force_retire_pool(Origin::none(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::force_retire_pool(Some(CREATOR).into(), 0), DispatchError::BadOrigin);
	});
}

#[test]
fn force_retire_pool_with_wrong_pool_state_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(
			LM::force_retire_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0),
			Error::<T>::InvalidPoolState
		);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(DAYS);

		let result =
			LM::force_retire_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0);
		assert_noop!(result, Error::<T>::InvalidPoolState);
	});
}

#[test]
fn simple_integration_test() {
	new_test_ext().execute_with(|| {
		const PER_BLOCK: Balance = REWARD_AMOUNT / DAYS as Balance;

		assert_ok!(LM::create_mining_pool(
			Some(CREATOR).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::approve_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, UNIT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		run_to_block(100);

		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, UNIT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		run_to_block(200);

		assert_ok!(LM::claim(Some(USER_1).into(), 0));
		assert_ok!(LM::claim(Some(USER_2).into(), 0));

		let pbpd_1 = FixedU128::from((PER_BLOCK, UNIT));
		let reward_step_1 = (pbpd_1 * (100 * UNIT).into()).into_inner() / FixedU128::accuracy();

		let pbpd_2 = FixedU128::from((PER_BLOCK, 2 * UNIT));
		let reward_step_2 = (pbpd_2 * (100 * UNIT).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, reward_step_1 + reward_step_2);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, reward_step_1 + reward_step_2);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, reward_step_2);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, reward_step_2);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);

		run_to_block(DAYS);

		assert_ok!(LM::redeem(Some(USER_1).into(), 0));
		assert_ok!(LM::redeem(Some(USER_2).into(), 0));

		let reward_step_3 =
			(pbpd_2 * ((DAYS - 200) as Balance * UNIT).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(
			Tokens::accounts(USER_1, REWARD_1).free,
			reward_step_1 + reward_step_2 + reward_step_3
		);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(
			Tokens::accounts(USER_1, REWARD_2).free,
			reward_step_1 + reward_step_2 + reward_step_3
		);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, reward_step_2 + reward_step_3);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, reward_step_2 + reward_step_3);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, UNIT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, UNIT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		let remain = REWARD_AMOUNT - (reward_step_1 + 2 * reward_step_2 + 2 * reward_step_3);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).free, remain);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).free, remain);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(CREATOR, REWARD_2).reserved, 0);

		assert!(LM::pool(0).is_none());
		assert!(LM::user_deposit_data(USER_1, 0).is_none());
		assert!(LM::user_deposit_data(USER_2, 0).is_none());
	});
}

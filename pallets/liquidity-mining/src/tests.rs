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

use std::convert::TryInto;

use frame_support::{
	assert_noop, assert_ok,
	dispatch::DispatchError,
	sp_runtime::{FixedPointNumber, FixedU128},
	traits::Hooks,
};
use frame_system::pallet_prelude::OriginFor;
use node_primitives::{Balance, CurrencyId, TokenSymbol};
use orml_traits::MultiReservableCurrency;

use crate::{
	mock::{Test as T, *},
	Error, PoolId, PoolInfo, PoolState, PoolType, TotalPoolInfos,
};

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
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			2001,
			13,
			20,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000 * UNIT,
			0
		));

		let pool = LM::pool(0).unwrap();

		assert_eq!(pool.r#type, PoolType::Farming);
		assert!(pool.investor.is_none());

		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).reserved, 0);
	});
}

#[test]
fn create_mining_pool_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000 * UNIT,
			0
		));

		let pool = LM::pool(0).unwrap();

		assert_eq!(pool.r#type, PoolType::Mining);
		assert!(pool.investor.is_none());

		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).reserved, 0);
	});
}

#[test]
fn create_mining_pool_with_wrong_currency_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LM::create_mining_pool(
				pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
				(
					CurrencyId::VSBond(RelayChainTokenSymbol::get(), 2001, 13, 20),
					CurrencyId::VSToken(RelayChainTokenSymbol::get()),
				),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
				DAYS,
				1_000 * UNIT,
				0,
			),
			Error::<T>::InvalidTradingPair,
		);

		assert_noop!(
			LM::create_mining_pool(
				pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
				(
					CurrencyId::LPToken(TokenSymbol::KSM, 1u8, TokenSymbol::DOT, 2u8),
					CurrencyId::VSToken(RelayChainTokenSymbol::get()),
				),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
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
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT / NUM as Balance),
				vec![(REWARD_2, REWARD_AMOUNT / NUM as Balance)].try_into().unwrap(),
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
		let wrong_origins: [OriginFor<Test>; 3] =
			[Origin::root(), Origin::none(), Some(INVESTOR).into()];

		for wrong_origin in wrong_origins {
			assert_noop!(
				LM::create_mining_pool(
					wrong_origin.clone(),
					MINING_TRADING_PAIR,
					(REWARD_1, REWARD_AMOUNT),
					vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
					DAYS,
					1_000 * UNIT,
					0
				),
				DispatchError::BadOrigin,
			);

			assert_noop!(
				LM::create_farming_pool(
					wrong_origin.clone(),
					2001,
					13,
					20,
					(REWARD_1, REWARD_AMOUNT),
					vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
					DAYS,
					1_000 * UNIT,
					0
				),
				DispatchError::BadOrigin
			);

			assert_noop!(
				LM::create_eb_farming_pool(
					wrong_origin.clone(),
					2001,
					13,
					20,
					(REWARD_1, REWARD_AMOUNT),
					vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
					DAYS,
					1_000 * UNIT,
					0
				),
				DispatchError::BadOrigin
			);
		}
	});
}

#[test]
fn create_pool_with_duplicate_trading_pair_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			LM::create_pool(
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_1),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
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
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
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
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
				PoolType::Farming,
				DAYS,
				MinimumDeposit::get() - 1,
				0
			),
			Error::<T>::InvalidDepositLimit,
		);

		assert_noop!(
			LM::create_pool(
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
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
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT),
				vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
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
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_1, REWARD_AMOUNT)].try_into().unwrap(),
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0,
		);
		assert_noop!(result, Error::<T>::DuplicateReward,);
	});
}

#[test]
fn charge_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.state, PoolState::UnCharged);
		assert!(pool.investor.is_none());

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.state, PoolState::Charged);

		assert!(LM::charged_pids().contains(&0));

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let kept = per_block * DAYS as Balance;
		let left = REWARD_AMOUNT - kept;

		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).free, left);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).free, left);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_1).free, kept);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_2).free, kept);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_2).reserved, 0);
	});
}

#[test]
fn charge_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));

		assert_noop!(
			LM::charge(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0),
			DispatchError::BadOrigin
		);
		assert_noop!(LM::charge(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::charge(Origin::none(), 0), DispatchError::BadOrigin);
	});
}

#[test]
fn charge_with_wrong_state_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));

		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));
		assert_noop!(LM::charge(Some(INVESTOR).into(), 0), Error::<T>::InvalidPoolState);
	});
}

#[test]
fn charge_exceed_maximum_should_fail() {
	new_test_ext().execute_with(|| {
		for i in 0..MaximumApproved::get() {
			assert_ok!(LM::create_pool(
				(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
				(REWARD_1, REWARD_AMOUNT / (MaximumApproved::get() + 1) as u128),
				vec![(REWARD_2, REWARD_AMOUNT / (MaximumApproved::get() + 1) as u128)]
					.try_into()
					.unwrap(),
				PoolType::Farming,
				DAYS,
				1_000 * UNIT,
				0
			));

			assert_ok!(LM::charge(Some(INVESTOR).into(), i));

			assert!(LM::charged_pids().contains(&i));
		}

		assert_ok!(LM::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT / (MaximumApproved::get() + 1) as u128),
			vec![(REWARD_2, REWARD_AMOUNT / (MaximumApproved::get() + 1) as u128)]
				.try_into()
				.unwrap(),
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));

		assert_noop!(
			LM::charge(Some(INVESTOR).into(), MaximumApproved::get()),
			Error::<T>::ExceedMaximumCharged
		);

		assert!(!LM::charged_pids().contains(&(MaximumApproved::get())));
	});
}

#[test]
fn charge_without_enough_balance_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.state, PoolState::UnCharged);
		assert!(pool.investor.is_none());

		// It is unable to call Collective::execute(..) which is private;
		assert_noop!(LM::charge(Some(BEGGAR).into(), 0), orml_tokens::Error::<T>::BalanceTooLow);
	});
}

#[test]
fn kill_pool_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.state, PoolState::UnCharged);

		assert_ok!(LM::kill_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0));

		assert!(!TotalPoolInfos::<T>::contains_key(0));
	});
}

#[test]
fn kill_pool_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));

		assert_noop!(LM::kill_pool(Some(USER_1).into(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::kill_pool(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::kill_pool(Origin::none(), 0), DispatchError::BadOrigin);
	});
}

#[test]
fn kill_pool_with_wrong_state_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_pool(
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));

		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_noop!(
			LM::kill_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0),
			Error::<T>::InvalidPoolState
		);
	});
}

#[test]
fn deposit_to_mining_pool_charged_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		let deposit = 1_000_000 as Balance;
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, deposit));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, deposit));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT - 2 * deposit);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.deposit, 2 * deposit);

		assert_eq!(Tokens::accounts(pool.keeper.clone(), MINING_DEPOSIT).free, 2 * deposit);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), MINING_DEPOSIT).reserved, 0);

		let deposit_data = LM::user_deposit_data(0, USER_1).unwrap();
		assert_eq!(deposit_data.deposit, 2 * deposit);
	});
}

#[test]
fn deposit_to_farming_pool_charged_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_farming_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			2001,
			13,
			20,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		let deposit = 1_000_000 as Balance;
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, deposit));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, deposit));

		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).free, DEPOSIT_AMOUNT - 2 * deposit);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).free, DEPOSIT_AMOUNT - 2 * deposit);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).reserved, 0);

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.deposit, 2 * deposit);

		assert_eq!(Tokens::accounts(pool.keeper.clone(), FARMING_DEPOSIT_1).free, 2 * deposit);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), FARMING_DEPOSIT_1).frozen, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), FARMING_DEPOSIT_1).reserved, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), FARMING_DEPOSIT_2).free, 2 * deposit);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), FARMING_DEPOSIT_2).frozen, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), FARMING_DEPOSIT_2).reserved, 0);

		let deposit_data = LM::user_deposit_data(0, USER_1).unwrap();
		assert_eq!(deposit_data.deposit, 2 * deposit);
	});
}

#[test]
fn startup_pool_meet_conditions_should_auto_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT));

		run_to_block(101);

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.state, PoolState::Ongoing);

		assert!(!LM::charged_pids().contains(&0));
	});
}

#[test]
fn deposit_to_pool_ongoing_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT));

		run_to_block(101);

		assert_ok!(LM::deposit(Some(USER_2).into(), 0, DEPOSIT_AMOUNT));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		let pool = LM::pool(0).unwrap();
		assert_eq!(pool.deposit, 2 * DEPOSIT_AMOUNT);

		assert_eq!(Tokens::accounts(pool.keeper.clone(), MINING_DEPOSIT).free, 2 * DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), MINING_DEPOSIT).reserved, 0);

		let deposit_data = LM::user_deposit_data(0, USER_2).unwrap();
		assert_eq!(deposit_data.deposit, DEPOSIT_AMOUNT);
	});
}

#[test]
fn deposit_to_pool_ongoing_with_init_deposit_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

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

		let deposit_data_1 = LM::user_deposit_data(0, USER_1).unwrap();
		assert_eq!(deposit_data_1.deposit, 2_000_000);
		let deposit_data_2 = LM::user_deposit_data(0, USER_2).unwrap();
		assert_eq!(deposit_data_2.deposit, 1_000_000);
	});
}

#[test]
fn double_deposit_to_pool_ongoing_in_diff_block_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

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

		let deposit_data_1 = LM::user_deposit_data(0, USER_1).unwrap();
		assert_eq!(deposit_data_1.deposit, 1_000_000);
		let deposit_data_2 = LM::user_deposit_data(0, USER_2).unwrap();
		assert_eq!(deposit_data_2.deposit, 2_000_000);
	});
}

#[test]
fn double_deposit_to_pool_ongoing_in_same_block_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

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

		let deposit_data_1 = LM::user_deposit_data(0, USER_1).unwrap();
		assert_eq!(deposit_data_1.deposit, 1_000_000);
		let deposit_data_2 = LM::user_deposit_data(0, USER_2).unwrap();
		assert_eq!(deposit_data_2.deposit, 2_000_000);
	});
}

#[test]
fn deposit_with_wrong_pid_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(LM::deposit(Some(USER_1).into(), 0, 1_000_000), Error::<T>::InvalidPoolId);

		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_noop!(LM::deposit(Some(USER_1).into(), 1, 1_000_000), Error::<T>::InvalidPoolId);
	});
}

#[test]
fn deposit_with_wrong_state_should_fail() {
	new_test_ext().execute_with(|| {
		System::set_block_number(100);

		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		assert_noop!(LM::deposit(Some(USER_1).into(), 0, 1_000_000), Error::<T>::InvalidPoolState);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));
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
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

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
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_noop!(LM::deposit(Origin::root(), 0, 1_000_000), DispatchError::BadOrigin);
		assert_noop!(LM::deposit(Origin::none(), 0, 1_000_000), DispatchError::BadOrigin);
	});
}

#[test]
fn deposit_exceed_the_limit_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(LM::deposit(Some(USER_1).into(), 0, 1_000_000), Error::<T>::InvalidPoolId);

		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

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
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		let keeper = LM::pool(0).unwrap().keeper;

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, DEPOSIT_AMOUNT));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, 2 * DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		run_to_block(100);

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let pbpd = FixedU128::from((per_block, 2 * DEPOSIT_AMOUNT));
		let rewarded = (pbpd * (100 * DEPOSIT_AMOUNT).into()).into_inner() / FixedU128::accuracy();

		let redeemed = DEPOSIT_AMOUNT / 2;
		let deposit_left = DEPOSIT_AMOUNT - redeemed;

		assert_ok!(LM::redeem(Some(USER_1).into(), 0, redeemed));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, redeemed);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);
		assert_eq!(LM::user_deposit_data(0, USER_1).unwrap().deposit, deposit_left);

		assert_ok!(LM::redeem(Some(USER_2).into(), 0, redeemed));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, redeemed);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);
		assert_eq!(LM::user_deposit_data(0, USER_2).unwrap().deposit, deposit_left);

		assert_eq!(LM::pool(0).unwrap().deposit, 2 * deposit_left);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, 2 * deposit_left);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		run_to_block(100);

		let pbpd = FixedU128::from((per_block, 2 * deposit_left));
		let rewarded = (pbpd * (100 * deposit_left).into()).into_inner() / FixedU128::accuracy();

		assert_ok!(LM::redeem_all(Some(USER_1).into(), 0));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);
		assert!(LM::user_deposit_data(0, USER_1).is_none());

		assert_ok!(LM::redeem_all(Some(USER_2).into(), 0));

		assert_eq!(
			Tokens::accounts(USER_2, MINING_DEPOSIT).free,
			DEPOSIT_AMOUNT - MinimumDeposit::get()
		);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);
		assert_eq!(LM::user_deposit_data(0, USER_2).unwrap().deposit, MinimumDeposit::get());

		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, MinimumDeposit::get());
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);
	});
}

#[test]
fn redeem_from_pool_retired_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		let keeper = LM::pool(0).unwrap().keeper;

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, DEPOSIT_AMOUNT));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, 2 * DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		run_to_block(DAYS);

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let pbpd = FixedU128::from((per_block, 2 * UNIT));
		let rewarded =
			(pbpd * (DAYS as Balance * UNIT).into()).into_inner() / FixedU128::accuracy();

		let minimum: Balance = 1;
		assert_ok!(LM::redeem(Some(USER_1).into(), 0, minimum));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);
		assert!(LM::user_deposit_data(0, USER_1).is_none());

		assert_ok!(LM::redeem_all(Some(USER_2).into(), 0));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);
		assert!(LM::user_deposit_data(0, USER_2).is_none());

		assert!(LM::pool(0).is_none());
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).free, REWARD_AMOUNT - 2 * rewarded);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).free, REWARD_AMOUNT - 2 * rewarded);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).reserved, 0);
	});
}

#[test]
fn double_redeem_from_pool_in_diff_state_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, DEPOSIT_AMOUNT));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		let keeper = LM::pool(0).unwrap().keeper;

		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, 2 * DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		run_to_block(100);

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let pbpd = FixedU128::from((per_block, 2 * UNIT));
		let old_rewarded = (pbpd * (100 * UNIT).into()).into_inner() / FixedU128::accuracy();

		let redeemed = DEPOSIT_AMOUNT / 2;
		let deposit_left = DEPOSIT_AMOUNT - redeemed;

		assert_ok!(LM::redeem(Some(USER_1).into(), 0, redeemed));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, redeemed);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, old_rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, old_rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);
		assert_eq!(LM::user_deposit_data(0, USER_1).unwrap().deposit, deposit_left);

		assert_ok!(LM::redeem_all(Some(USER_2).into(), 0));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, old_rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, old_rewarded);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);
		assert!(LM::user_deposit_data(0, USER_2).is_none());

		assert_eq!(LM::pool(0).unwrap().deposit, deposit_left);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, deposit_left);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		// USER_1 didn't remember to redeem until the seventh day
		run_to_block(7 * DAYS);
		let pbpd = FixedU128::from((per_block, deposit_left));
		let new_rewarded = (pbpd * ((DAYS - 100) as Balance * deposit_left).into()).into_inner() /
			FixedU128::accuracy();

		let minimum: Balance = 1;
		assert_ok!(LM::redeem(Some(USER_1).into(), 0, minimum));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, old_rewarded + new_rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, old_rewarded + new_rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);
		assert!(LM::user_deposit_data(0, USER_1).is_none());

		assert!(LM::pool(0).is_none());
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		assert_eq!(
			Tokens::accounts(INVESTOR, REWARD_1).free,
			REWARD_AMOUNT - (2 * old_rewarded + new_rewarded)
		);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).reserved, 0);
		assert_eq!(
			Tokens::accounts(INVESTOR, REWARD_2).free,
			REWARD_AMOUNT - (2 * old_rewarded + new_rewarded)
		);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).reserved, 0);
	});
}

#[test]
fn redeem_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(LM::redeem_all(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::redeem_all(Origin::none(), 0), DispatchError::BadOrigin);
	});
}

#[test]
fn redeem_with_wrong_pid_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(100);

		assert_noop!(LM::redeem_all(Some(USER_1).into(), 1), Error::<T>::InvalidPoolId);
	});
}

#[test]
fn redeem_with_wrong_state_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(LM::redeem_all(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		assert_noop!(LM::redeem_all(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);
	});
}

#[test]
fn redeem_without_deposit_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(LM::redeem_all(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		run_to_block(100);

		assert_ok!(LM::redeem_all(Some(USER_1).into(), 0));
		assert_noop!(LM::redeem_all(Some(USER_1).into(), 0), Error::<T>::NoDepositOfUser);
	});
}

#[test]
fn redeem_all_deposit_from_pool_ongoing_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(LM::redeem_all(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(100);

		assert_ok!(LM::redeem_all(Some(USER_1).into(), 0));
		let result = LM::redeem_all(Some(USER_1).into(), 0);
		assert_noop!(result, Error::<T>::TooLowToRedeem);
	});
}

#[test]
fn redeem_some_more_than_user_can_redeem_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(100);

		assert_noop!(
			LM::redeem(Some(USER_1).into(), 0, UNIT - MinimumDeposit::get() + 1),
			Error::<T>::TooLowToRedeem
		);
	});
}

#[test]
fn volunteer_to_redeem_should_work() {
	const PER_BLOCK: Balance = REWARD_AMOUNT / DAYS as Balance;

	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		let keeper = LM::pool(0).unwrap().keeper;

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, 2 * UNIT);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		run_to_block(DAYS);

		assert_ok!(LM::volunteer_to_redeem(Some(RICHER).into(), 0, Some(USER_1)));
		assert_ok!(LM::volunteer_to_redeem(Origin::root(), 0, None));

		let pbpd = FixedU128::from((PER_BLOCK, 2 * UNIT));
		let reward_to_user =
			(pbpd * (DAYS as Balance * UNIT).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, reward_to_user);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, reward_to_user);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, REWARD_1).free, reward_to_user);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).free, reward_to_user);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, UNIT);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, UNIT);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		assert!(LM::pool(0).is_none());
		assert!(LM::user_deposit_data(0, USER_1).is_none());
		assert!(LM::user_deposit_data(0, USER_2).is_none());
	});
}

#[test]
fn volunteer_to_redeem_with_wrong_pid_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(DAYS);

		assert_noop!(LM::volunteer_to_redeem(Origin::none(), 1, None), Error::<T>::InvalidPoolId);
	});
}

#[test]
fn volunteer_to_redeem_with_wrong_state_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(
			LM::volunteer_to_redeem(Origin::none(), 0, None),
			Error::<T>::InvalidPoolState
		);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_noop!(
			LM::volunteer_to_redeem(Origin::none(), 0, None),
			Error::<T>::InvalidPoolState
		);

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(100);

		assert_noop!(
			LM::volunteer_to_redeem(Origin::none(), 0, None),
			Error::<T>::InvalidPoolState
		);
	});
}

#[test]
fn claim_from_pool_ongoing_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		run_to_block(100);

		assert_ok!(LM::claim(Some(USER_1).into(), 0));

		let per_block = REWARD_AMOUNT / DAYS as Balance;
		let kept = per_block * DAYS as Balance;

		let pbpd = FixedU128::from((per_block, 2 * UNIT));
		let rewarded: Balance = (pbpd * (100 * UNIT).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, rewarded);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		let pool: PoolInfo<T> = LM::pool(0).unwrap();

		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_1).free, kept - rewarded);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_2).free, kept - rewarded);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(pool.keeper.clone(), REWARD_2).reserved, 0);

		let pool: PoolInfo<T> = LM::pool(0).unwrap();
		assert_eq!(pool.rewards.get(&REWARD_1).unwrap().claimed, rewarded);
		assert_eq!(pool.rewards.get(&REWARD_2).unwrap().claimed, rewarded);
	});
}

#[test]
fn claim_from_pool_retired_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(DAYS);

		let result = LM::claim(Some(USER_1).into(), 0);
		assert_noop!(result, Error::<T>::InvalidPoolState);
	});
}

#[test]
fn claim_with_wrong_pid_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

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
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

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
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(LM::claim(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);
		assert_noop!(LM::claim(Some(USER_2).into(), 0), Error::<T>::InvalidPoolState);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

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
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(DAYS - 1);

		let result = LM::claim(Some(USER_2).into(), 0);
		assert_noop!(result, Error::<T>::NoDepositOfUser);
	});
}

#[test]
fn double_claim_in_same_block_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(DAYS - 1);

		assert_ok!(LM::claim(Some(USER_1).into(), 0));
		assert_noop!(LM::claim(Some(USER_1).into(), 0), Error::<T>::TooShortBetweenTwoClaim);
	});
}

#[test]
fn force_retire_pool_charged_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));
		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		assert_ok!(LM::force_retire_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			0
		));

		assert_noop!(LM::deposit(Some(RICHER).into(), 0, UNIT), Error::<T>::InvalidPoolState);

		assert_noop!(LM::claim(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);
		assert_noop!(LM::claim(Some(USER_2).into(), 0), Error::<T>::InvalidPoolState);
		assert_ok!(LM::redeem_all(Some(USER_1).into(), 0));
		assert_ok!(LM::redeem_all(Some(USER_2).into(), 0));

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

		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).reserved, 0);

		assert!(LM::pool(0).is_none());
		assert!(LM::user_deposit_data(0, USER_1).is_none());
		assert!(LM::user_deposit_data(0, USER_2).is_none());
	});
}

#[test]
fn force_retire_pool_charged_with_no_deposit_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::force_retire_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			0
		));

		assert!(LM::pool(0).is_none());
	});
}

#[test]
fn force_retire_pool_ongoing_should_work() {
	const PER_BLOCK: Balance = REWARD_AMOUNT / DAYS as Balance;

	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(100);

		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		run_to_block(200);

		assert_ok!(LM::force_retire_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			0
		));
		assert_noop!(LM::deposit(Some(RICHER).into(), 0, UNIT), Error::<T>::InvalidPoolState);

		run_to_block(DAYS - 100);

		assert_noop!(LM::claim(Some(USER_1).into(), 0), Error::<T>::InvalidPoolState);
		assert_noop!(LM::claim(Some(USER_2).into(), 0), Error::<T>::InvalidPoolState);
		assert_ok!(LM::redeem_all(Some(USER_1).into(), 0));
		assert_ok!(LM::redeem_all(Some(USER_2).into(), 0));

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
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).free, remain);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).free, remain);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).reserved, 0);

		assert!(LM::pool(0).is_none());
		assert!(LM::user_deposit_data(0, USER_1).is_none());
		assert!(LM::user_deposit_data(0, USER_2).is_none());
	});
}

#[test]
fn force_retire_pool_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_noop!(LM::force_retire_pool(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::force_retire_pool(Origin::none(), 0), DispatchError::BadOrigin);
		assert_noop!(LM::force_retire_pool(Some(INVESTOR).into(), 0), DispatchError::BadOrigin);
	});
}

#[test]
fn force_retire_pool_with_wrong_pool_state_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		assert_noop!(
			LM::force_retire_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0),
			Error::<T>::InvalidPoolState
		);

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		run_to_block(DAYS);

		let result =
			LM::force_retire_pool(pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(), 0);
		assert_noop!(result, Error::<T>::InvalidPoolState);
	});
}

#[test]
fn create_eb_farming_pool_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_eb_farming_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			2001,
			13,
			20,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000 * UNIT,
			0
		));

		let pool = LM::pool(0).unwrap();

		assert_eq!(pool.r#type, PoolType::EBFarming);

		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).free, REWARD_AMOUNT);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).reserved, 0);
	});
}

#[test]
fn deposit_to_eb_farming_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_eb_farming_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			2001,
			13,
			20,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_noop!(
			LM::deposit(Some(USER_1).into(), 0, MinimumDeposit::get()),
			Error::<T>::NotEnoughToDeposit
		);

		assert_ok!(Tokens::reserve(FARMING_DEPOSIT_1, &USER_1, DEPOSIT_AMOUNT / 2));
		assert_ok!(Tokens::reserve(FARMING_DEPOSIT_2, &USER_1, DEPOSIT_AMOUNT / 2));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT / 2));

		run_to_block(100);

		assert_ok!(Tokens::reserve(FARMING_DEPOSIT_1, &USER_1, DEPOSIT_AMOUNT / 2));
		assert_ok!(Tokens::reserve(FARMING_DEPOSIT_2, &USER_1, DEPOSIT_AMOUNT / 2));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT / 2));

		run_to_block(200);

		assert_eq!(Tokens::unreserve(FARMING_DEPOSIT_1, &USER_1, DEPOSIT_AMOUNT / 2), 0);
		assert_eq!(Tokens::unreserve(FARMING_DEPOSIT_2, &USER_1, DEPOSIT_AMOUNT / 2), 0);
		let result = LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT / 2);
		assert_noop!(result, Error::<T>::NotEnoughToDeposit);

		run_to_block(300);

		assert_ok!(Tokens::reserve(FARMING_DEPOSIT_1, &USER_1, DEPOSIT_AMOUNT / 2));
		assert_ok!(Tokens::reserve(FARMING_DEPOSIT_2, &USER_1, DEPOSIT_AMOUNT / 2));
		let result = LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT / 2);
		assert_noop!(result, Error::<T>::NotEnoughToDeposit);
	});
}

#[test]
fn redeem_from_eb_farming_should_work() {
	const PER_BLOCK: Balance = REWARD_AMOUNT / DAYS as Balance;

	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_eb_farming_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			2001,
			13,
			20,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(Tokens::reserve(FARMING_DEPOSIT_1, &USER_1, DEPOSIT_AMOUNT));
		assert_ok!(Tokens::reserve(FARMING_DEPOSIT_2, &USER_1, DEPOSIT_AMOUNT));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT));

		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).free, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).reserved, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).free, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).reserved, DEPOSIT_AMOUNT);

		run_to_block(100);

		assert_ok!(LM::redeem_all(Some(USER_1).into(), 0));

		let pbpd = FixedU128::from((PER_BLOCK, DEPOSIT_AMOUNT));
		let reward_to_user_1 =
			(pbpd * (100 * DEPOSIT_AMOUNT).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, reward_to_user_1);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, reward_to_user_1);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT - MinimumDeposit::get()));

		run_to_block(DAYS);

		assert_ok!(LM::redeem_all(Some(USER_1).into(), 0));

		let reward_to_user_2 = (pbpd * ((DAYS - 100) as Balance * DEPOSIT_AMOUNT).into())
			.into_inner() /
			FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, reward_to_user_1 + reward_to_user_2);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, reward_to_user_1 + reward_to_user_2);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		assert!(LM::pool(0).is_none());
		assert!(LM::user_deposit_data(0, USER_1).is_none());
	});
}

#[test]
fn claim_from_eb_farming_should_work() {
	const PER_BLOCK: Balance = REWARD_AMOUNT / DAYS as Balance;

	new_test_ext().execute_with(|| {
		assert_ok!(LM::create_eb_farming_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			2001,
			13,
			20,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1_000_000,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		assert_ok!(Tokens::reserve(FARMING_DEPOSIT_1, &USER_1, DEPOSIT_AMOUNT));
		assert_ok!(Tokens::reserve(FARMING_DEPOSIT_2, &USER_1, DEPOSIT_AMOUNT));
		assert_ok!(LM::deposit(Some(USER_1).into(), 0, DEPOSIT_AMOUNT));

		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).free, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_1).reserved, DEPOSIT_AMOUNT);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).free, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, FARMING_DEPOSIT_2).reserved, DEPOSIT_AMOUNT);

		run_to_block(100);

		assert_ok!(LM::claim(Some(USER_1).into(), 0));

		let pbpd = FixedU128::from((PER_BLOCK, DEPOSIT_AMOUNT));
		let reward_to_user_1 =
			(pbpd * (100 * DEPOSIT_AMOUNT).into()).into_inner() / FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, reward_to_user_1);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, reward_to_user_1);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		run_to_block(DAYS);

		let result = LM::claim(Some(USER_1).into(), 0);
		assert_noop!(result, Error::<T>::InvalidPoolState);
		assert_ok!(LM::redeem_all(Some(USER_1).into(), 0));

		let reward_to_user_2 = (pbpd * ((DAYS - 100) as Balance * DEPOSIT_AMOUNT).into())
			.into_inner() /
			FixedU128::accuracy();

		assert_eq!(Tokens::accounts(USER_1, REWARD_1).free, reward_to_user_1 + reward_to_user_2);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).free, reward_to_user_1 + reward_to_user_2);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, REWARD_2).reserved, 0);

		assert!(LM::pool(0).is_none());
		assert!(LM::user_deposit_data(0, USER_1).is_none());
	});
}

#[test]
fn simple_integration_test() {
	new_test_ext().execute_with(|| {
		const PER_BLOCK: Balance = REWARD_AMOUNT / DAYS as Balance;

		assert_ok!(LM::create_mining_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			MINING_TRADING_PAIR,
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)].try_into().unwrap(),
			DAYS,
			1 * UNIT,
			0
		));

		// It is unable to call Collective::execute(..) which is private;
		assert_ok!(LM::charge(Some(INVESTOR).into(), 0));

		let pool = LM::pool(0).unwrap();
		let keeper = pool.keeper.clone();
		let kept = PER_BLOCK * DAYS as Balance;

		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).free, REWARD_AMOUNT - kept);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).free, REWARD_AMOUNT - kept);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).reserved, 0);

		assert_eq!(Tokens::accounts(keeper.clone(), REWARD_1).free, kept);
		assert_eq!(Tokens::accounts(keeper.clone(), REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), REWARD_2).free, kept);
		assert_eq!(Tokens::accounts(keeper.clone(), REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), REWARD_2).reserved, 0);

		assert_ok!(LM::deposit(Some(USER_1).into(), 0, UNIT));

		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_1, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, UNIT);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		run_to_block(100);

		assert_ok!(LM::deposit(Some(USER_2).into(), 0, UNIT));

		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(USER_2, MINING_DEPOSIT).reserved, 0);

		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, 2 * UNIT);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

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

		assert_ok!(LM::redeem_all(Some(USER_1).into(), 0));
		assert_ok!(LM::redeem_all(Some(USER_2).into(), 0));

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

		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).free, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).frozen, 0);
		assert_eq!(Tokens::accounts(keeper.clone(), MINING_DEPOSIT).reserved, 0);

		let remain = REWARD_AMOUNT - (reward_step_1 + 2 * reward_step_2 + 2 * reward_step_3);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).free, remain);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_1).reserved, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).free, remain);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).frozen, 0);
		assert_eq!(Tokens::accounts(INVESTOR, REWARD_2).reserved, 0);

		assert!(LM::pool(0).is_none());
		assert!(LM::user_deposit_data(0, USER_1).is_none());
		assert!(LM::user_deposit_data(0, USER_2).is_none());
	});
}

#[test]
fn fuck_bug() {
	new_test_ext().execute_with(|| {
		const ALICE: AccountId = AccountId::new([0u8; 32]);
		const BOB: AccountId = AccountId::new([1u8; 32]);
		const CHARLIE: AccountId = AccountId::new([2u8; 32]);

		const INIT_AMOUNT: Balance = 1_000_000_000 * UNIT;

		const REWARD_TOKEN: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		const REWARD_AMOUNT: Balance = 10 * UNIT;

		const DEPOSIT_TOKEN_1: CurrencyId = CurrencyId::VSToken(TokenSymbol::KSM);
		const DEPOSIT_TOKEN_2: CurrencyId = CurrencyId::VSBond(TokenSymbol::KSM, 2001, 13, 20);

		assert_ok!(Tokens::set_balance(Origin::root(), ALICE, REWARD_TOKEN, INIT_AMOUNT, 0));
		assert_ok!(Tokens::set_balance(Origin::root(), BOB, DEPOSIT_TOKEN_1, 0, INIT_AMOUNT));
		assert_ok!(Tokens::set_balance(Origin::root(), BOB, DEPOSIT_TOKEN_2, 0, INIT_AMOUNT));
		assert_ok!(Tokens::set_balance(Origin::root(), CHARLIE, DEPOSIT_TOKEN_1, 0, INIT_AMOUNT));
		assert_ok!(Tokens::set_balance(Origin::root(), CHARLIE, DEPOSIT_TOKEN_2, 0, INIT_AMOUNT));

		run_to_block(134);

		assert_ok!(LM::create_eb_farming_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			2001,
			13,
			20,
			(REWARD_TOKEN, REWARD_AMOUNT),
			vec![].try_into().unwrap(),
			23,
			UNIT,
			0
		));

		run_to_block(135);

		assert_ok!(LM::charge(Some(ALICE).into(), 0));

		run_to_block(138);

		assert_ok!(LM::deposit(Some(BOB).into(), 0, 13 * UNIT));

		run_to_block(140);

		assert_ok!(LM::deposit(Some(CHARLIE).into(), 0, 187 * UNIT));

		run_to_block(179);

		assert_ok!(LM::redeem_all(Some(BOB).into(), 0));
		assert_ok!(LM::redeem_all(Some(CHARLIE).into(), 0));

		assert!(LM::pool(200).is_none());

		assert_ok!(LM::create_eb_farming_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			2001,
			13,
			20,
			(REWARD_TOKEN, REWARD_AMOUNT),
			vec![].try_into().unwrap(),
			23,
			UNIT,
			0
		));

		run_to_block(235);

		assert_ok!(LM::charge(Some(ALICE).into(), 1));

		run_to_block(250);

		assert_ok!(LM::deposit(Some(BOB).into(), 1, 23 * UNIT));

		run_to_block(265);

		assert_ok!(LM::deposit(Some(CHARLIE).into(), 1, 167 * UNIT));

		run_to_block(280);

		assert_ok!(LM::redeem_all(Some(BOB).into(), 1));
		assert_ok!(LM::redeem_all(Some(CHARLIE).into(), 1));

		assert!(LM::pool(1).is_none());
	});
}

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

use crate::mock::*;
use frame_support::{assert_noop, assert_ok, BoundedVec};
use lend_market::{AccountBorrows, BorrowSnapshot, Deposits};

fn init() {
	env_logger::try_init().unwrap_or(());
	assert_ok!(LendMarket::add_market(RuntimeOrigin::root(), DOT, market_mock(LDOT)));
	assert_ok!(LendMarket::activate_market(RuntimeOrigin::root(), DOT));
	assert_ok!(LendMarket::add_market(RuntimeOrigin::root(), VDOT, market_mock(LVDOT)));
	assert_ok!(LendMarket::activate_market(RuntimeOrigin::root(), VDOT));
	TimestampPallet::set_timestamp(6000);

	assert_ok!(StablePool::create_pool(
		RuntimeOrigin::root(),
		vec![DOT, VDOT],
		vec![1u128, 1u128],
		10000000u128,
		20000000u128,
		50000000u128,
		10000u128,
		2,
		1,
		unit(1),
	));
	assert_ok!(StablePool::edit_token_rate(
		RuntimeOrigin::root(),
		0,
		vec![(DOT, (1, 1)), (VDOT, (100_000_000, 100_000_000))]
	));
	assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
	assert_ok!(VtokenMinting::mint(Some(0).into(), DOT, unit(100_000), BoundedVec::default()));
	assert_eq!(Tokens::balance(VDOT, &0), unit(100_000));
	assert_ok!(VtokenMinting::mint(Some(1).into(), DOT, unit(10), BoundedVec::default()));
	assert_eq!(Tokens::balance(VDOT, &1), unit(10));
	let amounts = vec![unit(100), unit(100)];
	assert_ok!(StablePool::add_liquidity(RuntimeOrigin::signed(0), 0, amounts, 0));
	assert_ok!(LendMarket::mint(RuntimeOrigin::signed(0), DOT, unit(100)));
	assert_ok!(LendMarket::mint(RuntimeOrigin::signed(0), VDOT, unit(100)));
	assert_ok!(LendMarket::mint(RuntimeOrigin::signed(1), VDOT, 100_000));
}

#[test]
fn increase_leverage_should_not_work() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		init();
		assert_noop!(
			LeverageStaking::flash_loan_deposit(
				RuntimeOrigin::signed(1),
				DOT,
				FixedU128::from_inner(unit(1_000_000)),
			),
			lend_market::Error::<Test>::InvalidAmount
		);
	});
}

#[test]
fn increase_leverage_should_work() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		init();
		assert_ok!(LeverageStaking::flash_loan_deposit(
			RuntimeOrigin::signed(1),
			DOT,
			FixedU128::from_inner(unit(100_000)),
		));
		assert_eq!(
			AccountDeposits::<Test>::get(VDOT, 1),
			Deposits { voucher_balance: 5500000, is_collateral: true },
		);
		assert_eq!(
			AccountBorrows::<Test>::get(DOT, 1),
			BorrowSnapshot { principal: 10_000, borrow_index: 1.into() },
		);
		assert_ok!(LeverageStaking::flash_loan_deposit(
			RuntimeOrigin::signed(1),
			DOT,
			FixedU128::from_inner(unit(800_000)),
		));
		assert_eq!(
			AccountDeposits::<Test>::get(VDOT, 1),
			Deposits { voucher_balance: 9_000_000, is_collateral: true },
		);
		assert_eq!(
			AccountBorrows::<Test>::get(DOT, 1),
			BorrowSnapshot { principal: 80_000, borrow_index: 1.into() },
		);
		assert_ok!(LeverageStaking::flash_loan_deposit(
			RuntimeOrigin::signed(1),
			DOT,
			FixedU128::from_inner(unit(900_000)),
		));
		assert_eq!(
			AccountBorrows::<Test>::get(DOT, 1),
			BorrowSnapshot { principal: 90_000, borrow_index: 1.into() },
		);
		assert_eq!(Tokens::balance(VDOT, &1), 9999999900000);
		assert_eq!(
			AccountDeposits::<Test>::get(VDOT, 1),
			Deposits { voucher_balance: 9500000, is_collateral: true },
		);
	});
}

#[test]
fn reduce_leverage_should_work() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		init();
		assert_eq!(
			AccountDeposits::<Test>::get(VDOT, 1),
			Deposits { voucher_balance: 5_000_000, is_collateral: false },
		);
		assert_ok!(LeverageStaking::flash_loan_deposit(
			RuntimeOrigin::signed(1),
			DOT,
			FixedU128::from_inner(unit(900_000)),
		));
		assert_eq!(
			AccountBorrows::<Test>::get(DOT, 1),
			BorrowSnapshot { principal: 90_000, borrow_index: 1.into() },
		);
		assert_eq!(
			AccountDeposits::<Test>::get(VDOT, 1),
			Deposits { voucher_balance: 9500000, is_collateral: true },
		);
		assert_eq!(Tokens::balance(VDOT, &1), 9999999900000);
		assert_ok!(LeverageStaking::flash_loan_deposit(
			RuntimeOrigin::signed(1),
			DOT,
			FixedU128::from_inner(unit(800_000)),
		));
		assert_eq!(Tokens::balance(VDOT, &1), 9999999900000);
		assert_eq!(
			AccountBorrows::<Test>::get(DOT, 1),
			BorrowSnapshot { principal: 80_000, borrow_index: 1.into() },
		);
		assert_eq!(
			AccountDeposits::<Test>::get(VDOT, 1),
			Deposits { voucher_balance: 8994050, is_collateral: true },
		);
	});
}

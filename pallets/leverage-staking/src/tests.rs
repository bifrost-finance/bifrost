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

use crate::mock::*;
use frame_support::{assert_noop, assert_ok, BoundedVec};
use lend_market::{AccountBorrows, BorrowSnapshot};

fn init_lend_market() {
	assert_ok!(LendMarket::add_market(RuntimeOrigin::root(), DOT, market_mock(VKSM)));
	assert_ok!(LendMarket::activate_market(RuntimeOrigin::root(), DOT));
	assert_ok!(LendMarket::add_market(RuntimeOrigin::root(), VDOT, market_mock(VBNC)));
	assert_ok!(LendMarket::activate_market(RuntimeOrigin::root(), VDOT));
	TimestampPallet::set_timestamp(6000);
}

#[test]
fn mint_successful_equal_amounts() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		init_lend_market();
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		assert_ok!(VtokenMinting::mint(Some(3).into(), DOT, 100_000_000, BoundedVec::default()));
		assert_ok!(LendMarket::mint(RuntimeOrigin::signed(1), DOT, unit(100)));
		assert_ok!(LeverageStaking::flash_loan_deposit(
			RuntimeOrigin::signed(1),
			DOT,
			FixedU128::from_inner(unit(1_100_000)),
			100_000
		));
		assert_eq!(
			AccountBorrows::<Test>::get(DOT, 1),
			BorrowSnapshot { principal: 10_000, borrow_index: 1.into() },
		);
	});
}

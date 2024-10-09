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

use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError;

#[test]
fn revoke_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			TOKEN_SYMBOL,
			13,
			20,
			100,
			100,
			OrderType::Sell
		));
		assert_ok!(Auction::revoke_order(Some(ALICE).into(), 0));
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			TOKEN_SYMBOL,
			13,
			20,
			100,
			100,
			OrderType::Buy
		));
		assert_ok!(Auction::revoke_order(Some(ALICE).into(), 1));

		assert_eq!(NextOrderId::<Test>::get(), 2);

		let user_sell_order_ids = UserOrderIds::<Test>::get(ALICE, OrderType::Sell);
		let user_buy_order_ids = UserOrderIds::<Test>::get(ALICE, OrderType::Buy);

		assert_eq!(user_sell_order_ids.len(), 0);
		assert_eq!(user_buy_order_ids.len(), 0);

		assert!(TotalOrderInfos::<Test>::get(0).is_none());
		assert!(TotalOrderInfos::<Test>::get(1).is_none());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 100);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 100);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 0);
	});
}

#[test]
fn revoke_order_should_work_with_ed_limits() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(DAVE).into(),
			3000,
			TOKEN_SYMBOL,
			13,
			20,
			100,
			100,
			OrderType::Sell
		));

		assert_ok!(Auction::partial_clinch_order(Some(CHARLIE).into(), 0, 96));
		assert_ok!(Auction::revoke_order(Some(DAVE).into(), 0));

		let module_account: u64 =
			<Test as crate::Config>::PalletId::get().into_account_truncating();
		let treasury_account: u64 = <Test as crate::Config>::TreasuryAccount::get();

		assert_eq!(Tokens::accounts(DAVE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(CHARLIE, VSBOND).free, 96);
		assert_eq!(Tokens::accounts(module_account, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(treasury_account, VSBOND).free, 4);

		assert_eq!(Tokens::accounts(DAVE, TOKEN).free, 96);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 0);
		// We didn't config OnDust filed for orml_tokens module, so dust will not be removed.
		assert_eq!(Tokens::accounts(CHARLIE, TOKEN).free, 4);
		assert_eq!(Tokens::accounts(treasury_account, TOKEN).free, 0);
	});
}

#[test]
fn revoke_sell_order_which_be_partial_clinchd_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			TOKEN_SYMBOL,
			13,
			20,
			100,
			33,
			OrderType::Sell
		));
		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 33));
		assert_ok!(Auction::revoke_order(Some(ALICE).into(), 0));

		assert_eq!(NextOrderId::<Test>::get(), 1);

		let user_sell_order_ids = UserOrderIds::<Test>::get(ALICE, OrderType::Sell);
		assert_eq!(user_sell_order_ids.len(), 0);

		assert!(TotalOrderInfos::<Test>::get(0).is_none());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 67);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 110);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 133);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 90);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).reserved, 0);
	});
}

#[test]
fn revoke_buy_order_which_be_partial_clinchd_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			TOKEN_SYMBOL,
			13,
			20,
			100,
			33,
			OrderType::Buy
		));
		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 33));
		assert_ok!(Auction::revoke_order(Some(ALICE).into(), 0));

		assert_eq!(NextOrderId::<Test>::get(), 1);

		let user_buy_order_ids = UserOrderIds::<Test>::get(ALICE, OrderType::Buy);
		assert_eq!(user_buy_order_ids.len(), 0);

		assert!(TotalOrderInfos::<Test>::get(0).is_none());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 133);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 90);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 67);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 110);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).reserved, 0);
	});
}

#[test]
fn revoke_order_not_exist_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(Auction::revoke_order(Some(ALICE).into(), 0), Error::<Test>::NotFindOrderInfo);
	});
}

#[test]
fn revoke_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			TOKEN_SYMBOL,
			13,
			20,
			50,
			50,
			OrderType::Sell
		));
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			TOKEN_SYMBOL,
			13,
			20,
			50,
			50,
			OrderType::Buy
		));

		assert_noop!(
			Auction::revoke_order(Some(BRUCE).into(), 0),
			Error::<Test>::ForbidRevokeOrderWithoutOwnership
		);
		assert_noop!(
			Auction::revoke_order(Some(BRUCE).into(), 1),
			Error::<Test>::ForbidRevokeOrderWithoutOwnership
		);

		assert_noop!(Auction::revoke_order(RuntimeOrigin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(Auction::revoke_order(RuntimeOrigin::root(), 1), DispatchError::BadOrigin);

		assert_noop!(Auction::revoke_order(RuntimeOrigin::none(), 0), DispatchError::BadOrigin);
		assert_noop!(Auction::revoke_order(RuntimeOrigin::none(), 1), DispatchError::BadOrigin);
	});
}

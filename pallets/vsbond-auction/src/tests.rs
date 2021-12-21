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
use orml_traits::{LockIdentifier, MultiLockableCurrency};

use crate::{mock::*, *};

#[test]
fn create_sell_order_should_work() {
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

		assert_eq!(Auction::order_id(), 1);

		let user_order_ids = Auction::user_order_ids(ALICE, OrderType::Sell);
		assert!(user_order_ids.contains(&0));

		assert!(Auction::order_info(&0).is_some());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);

		let module_account: u64 = <Test as crate::Config>::PalletId::get().into_account();
		assert_eq!(Tokens::accounts(module_account, VSBOND).free, 100);
	});
}

#[test]
fn create_buy_order_should_work() {
	new_test_ext().execute_with(|| {
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

		assert_eq!(Auction::order_id(), 1);

		let user_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert!(user_order_ids.contains(&0));

		let module_account: u64 = <Test as crate::Config>::PalletId::get().into_account();

		assert!(Auction::order_info(&0).is_some());

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 100);
	});
}

#[test]
fn double_create_order_should_work() {
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

		assert_eq!(Auction::order_id(), 2);

		let user_sell_order_ids = Auction::user_order_ids(ALICE, OrderType::Sell);
		let user_buy_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert!(user_sell_order_ids.contains(&0));
		assert!(user_buy_order_ids.contains(&1));

		assert!(Auction::order_info(&0).is_some());
		assert!(Auction::order_info(&1).is_some());

		let module_account: u64 = <Test as crate::Config>::PalletId::get().into_account();

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 50);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, VSBOND).free, 50);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 50);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 50);
	});
}

#[test]
fn create_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Auction::create_order(
				Origin::root(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				100,
				100,
				OrderType::Sell
			),
			DispatchError::BadOrigin
		);
		assert_noop!(
			Auction::create_order(
				Origin::none(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				100,
				100,
				OrderType::Buy
			),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn create_order_under_minimum_amount_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Auction::create_order(
				Some(ALICE).into(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				0,
				0,
				OrderType::Sell
			),
			Error::<Test>::NotEnoughAmount
		);

		assert_noop!(
			Auction::create_order(
				Some(ALICE).into(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				0,
				0,
				OrderType::Buy
			),
			Error::<Test>::NotEnoughAmount
		);
	});
}

#[test]
fn create_order_without_enough_to_reserve_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Auction::create_order(
				Some(ALICE).into(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				1000,
				1000,
				OrderType::Sell
			),
			Error::<Test>::NotEnoughBalanceToCreateOrder,
		);

		const LOCK_ID_SELL: LockIdentifier = 0u64.to_be_bytes();
		assert_ok!(Tokens::set_lock(LOCK_ID_SELL, VSBOND, &ALICE, 50));
		assert_noop!(
			Auction::create_order(
				Some(ALICE).into(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				51,
				51,
				OrderType::Sell
			),
			Error::<Test>::NotEnoughBalanceToCreateOrder,
		);

		assert_noop!(
			Auction::create_order(
				Some(ALICE).into(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				1000,
				1000,
				OrderType::Buy
			),
			Error::<Test>::NotEnoughBalanceToCreateOrder,
		);

		const LOCK_ID_BUY: LockIdentifier = 1u64.to_be_bytes();
		assert_ok!(Tokens::set_lock(LOCK_ID_BUY, TOKEN, &ALICE, 50));
		assert_noop!(
			Auction::create_order(
				Some(ALICE).into(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				51,
				51,
				OrderType::Buy
			),
			Error::<Test>::NotEnoughBalanceToCreateOrder,
		);
	});
}

#[test]
fn create_order_exceed_maximum_order_in_trade_should_fail() {
	new_test_ext().execute_with(|| {
		for _ in 0..MaximumOrderInTrade::get() {
			assert_ok!(Auction::create_order(
				Some(ALICE).into(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				1,
				1,
				OrderType::Sell
			));
			assert_ok!(Auction::create_order(
				Some(ALICE).into(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				1,
				1,
				OrderType::Buy
			));
		}

		assert_noop!(
			Auction::create_order(
				Some(ALICE).into(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				1,
				1,
				OrderType::Sell
			),
			Error::<Test>::ExceedMaximumOrderInTrade,
		);

		assert_noop!(
			Auction::create_order(
				Some(ALICE).into(),
				3000,
				TOKEN_SYMBOL,
				13,
				20,
				1,
				1,
				OrderType::Buy
			),
			Error::<Test>::ExceedMaximumOrderInTrade,
		);
	});
}

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

		assert_eq!(Auction::order_id(), 2);

		let user_sell_order_ids = Auction::user_order_ids(ALICE, OrderType::Sell);
		let user_buy_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);

		assert_eq!(user_sell_order_ids.len(), 0);
		assert_eq!(user_buy_order_ids.len(), 0);

		assert!(Auction::order_info(0).is_none());
		assert!(Auction::order_info(1).is_none());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 100);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 100);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 0);
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

		assert_eq!(Auction::order_id(), 1);

		let user_sell_order_ids = Auction::user_order_ids(ALICE, OrderType::Sell);
		assert_eq!(user_sell_order_ids.len(), 0);

		assert!(Auction::order_info(0).is_none());

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

		assert_eq!(Auction::order_id(), 1);

		let user_buy_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert_eq!(user_buy_order_ids.len(), 0);

		assert!(Auction::order_info(0).is_none());

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

		assert_noop!(Auction::revoke_order(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(Auction::revoke_order(Origin::root(), 1), DispatchError::BadOrigin);

		assert_noop!(Auction::revoke_order(Origin::none(), 0), DispatchError::BadOrigin);
		assert_noop!(Auction::revoke_order(Origin::none(), 1), DispatchError::BadOrigin);
	});
}

#[test]
fn partial_clinch_sell_order_should_work() {
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

		assert_eq!(Auction::order_id(), 1);

		let user_sell_order_ids = Auction::user_order_ids(ALICE, OrderType::Sell);
		assert_eq!(user_sell_order_ids.len(), 1);

		let order_info = Auction::order_info(0).unwrap();
		assert_eq!(order_info.remain, 67);

		let module_account: u64 = <Test as crate::Config>::PalletId::get().into_account();

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, VSBOND).free, 67);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 110);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 0);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 133);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, VSBOND).free, 67);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 90);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 0);

		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 9999999));

		let user_sell_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert_eq!(user_sell_order_ids.len(), 0);

		assert!(Auction::order_info(0).is_none());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 132);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 200);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 68);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).reserved, 0);
	});
}

#[test]
fn partial_clinch_buy_order_should_work() {
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

		assert_eq!(Auction::order_id(), 1);

		let user_buy_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert_eq!(user_buy_order_ids.len(), 1);

		let order_info = Auction::order_info(0).unwrap();
		assert_eq!(order_info.remain, 67);

		let module_account: u64 = <Test as crate::Config>::PalletId::get().into_account();

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 133);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, VSBOND).free, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 67);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 23);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 67);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, VSBOND).free, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 110);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 23);

		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 9999999));

		let user_buy_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert_eq!(user_buy_order_ids.len(), 0);

		assert!(Auction::order_info(0).is_none());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 200);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 68);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 132);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).reserved, 0);
	});
}

#[test]
fn partial_clinch_order_not_exist_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(Auction::clinch_order(Some(BRUCE).into(), 1), Error::<Test>::NotFindOrderInfo);
		assert_noop!(
			Auction::partial_clinch_order(Some(BRUCE).into(), 1, 50),
			Error::<Test>::NotFindOrderInfo
		);
	});
}

#[test]
fn clinch_order_by_origin_illegal_should_fail() {
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
		assert_noop!(
			Auction::clinch_order(Some(ALICE).into(), 0),
			Error::<Test>::ForbidClinchOrderWithinOwnership
		);
		assert_noop!(
			Auction::partial_clinch_order(Some(ALICE).into(), 0, 50),
			Error::<Test>::ForbidClinchOrderWithinOwnership
		);

		assert_noop!(
			Auction::partial_clinch_order(Origin::root(), 0, 50),
			DispatchError::BadOrigin,
		);
		assert_noop!(
			Auction::partial_clinch_order(Origin::none(), 0, 50),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn clinch_order_without_enough_token_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			TOKEN_SYMBOL,
			13,
			20,
			100,
			200,
			OrderType::Sell
		));
		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 50));
		assert_noop!(
			Auction::clinch_order(Some(BRUCE).into(), 0),
			Error::<Test>::DontHaveEnoughToPay
		);
	});
}

#[test]
fn handle_special_vsbond_sell_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			2001,
			TokenSymbol::KSM,
			13,
			20,
			100,
			33,
			OrderType::Sell
		));
		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 33));

		assert_eq!(Auction::order_id(), 1);

		let user_sell_order_ids = Auction::user_order_ids(ALICE, OrderType::Sell);
		assert_eq!(user_sell_order_ids.len(), 1);

		let order_info = Auction::order_info(0).unwrap();
		assert_eq!(order_info.remain, 67);

		let module_account: u64 = <Test as crate::Config>::PalletId::get().into_account();

		assert_eq!(Tokens::accounts(ALICE, SPECIAL_VSBOND).free, 0);
		assert_eq!(Tokens::accounts(ALICE, SPECIAL_VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, SPECIAL_VSBOND).free, 67);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 110);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 0);

		assert_eq!(Tokens::accounts(BRUCE, SPECIAL_VSBOND).free, 133);
		assert_eq!(Tokens::accounts(BRUCE, SPECIAL_VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, SPECIAL_VSBOND).free, 67);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 90);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 0);

		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 9999999));

		let user_sell_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert_eq!(user_sell_order_ids.len(), 0);

		assert!(Auction::order_info(0).is_none());

		assert_eq!(Tokens::accounts(ALICE, SPECIAL_VSBOND).free, 0);
		assert_eq!(Tokens::accounts(ALICE, SPECIAL_VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, SPECIAL_VSBOND).free, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 132);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 0);

		assert_eq!(Tokens::accounts(BRUCE, SPECIAL_VSBOND).free, 200);
		assert_eq!(Tokens::accounts(BRUCE, SPECIAL_VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, SPECIAL_VSBOND).free, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 68);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 0);
	});
}

#[test]
fn handle_special_vsbond_buy_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			2001,
			TokenSymbol::KSM,
			13,
			20,
			100,
			33,
			OrderType::Buy
		));
		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 33));

		assert_eq!(Auction::order_id(), 1);

		let user_buy_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert_eq!(user_buy_order_ids.len(), 1);

		let order_info = Auction::order_info(0).unwrap();
		assert_eq!(order_info.remain, 67);

		let module_account: u64 = <Test as crate::Config>::PalletId::get().into_account();

		assert_eq!(Tokens::accounts(ALICE, SPECIAL_VSBOND).free, 133);
		assert_eq!(Tokens::accounts(ALICE, SPECIAL_VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, SPECIAL_VSBOND).free, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 67);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 23);

		assert_eq!(Tokens::accounts(BRUCE, SPECIAL_VSBOND).free, 67);
		assert_eq!(Tokens::accounts(BRUCE, SPECIAL_VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, SPECIAL_VSBOND).free, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 110);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 23);

		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 9999999));

		let user_buy_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert_eq!(user_buy_order_ids.len(), 0);

		assert!(Auction::order_info(0).is_none());

		assert_eq!(Tokens::accounts(ALICE, SPECIAL_VSBOND).free, 200);
		assert_eq!(Tokens::accounts(ALICE, SPECIAL_VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, SPECIAL_VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 68);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, SPECIAL_VSBOND).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, SPECIAL_VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, SPECIAL_VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 132);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).reserved, 0);
	});
}

#[test]
fn set_buy_and_sell_transaction_fee_rate_should_work() {
	new_test_ext().execute_with(|| {
		// both buy and see rate are 10%.
		assert_ok!(Auction::set_buy_and_sell_transaction_fee_rate(Some(ALICE).into(), 1000, 1000));

		assert_eq!(
			Auction::get_transaction_fee_rate(),
			(Permill::from_percent(10), Permill::from_percent(10))
		);

		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			TOKEN_SYMBOL,
			13,
			20,
			80,
			80,
			OrderType::Sell
		));

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 20);
		// 8 token fee is charged
		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 92);

		let module_account: u64 = <Test as crate::Config>::PalletId::get().into_account();
		assert_eq!(Tokens::accounts(module_account, VSBOND).free, 80);

		let treasury_account: u64 = <Test as crate::Config>::TreasuryAccount::get();
		assert_eq!(Tokens::accounts(treasury_account, TOKEN).free, 8);

		assert_ok!(Auction::create_order(
			Some(BRUCE).into(),
			3000,
			TOKEN_SYMBOL,
			13,
			20,
			80,
			80,
			OrderType::Buy
		));

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 100);
		// 8 token fee is charged + 80 total price
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 12);

		let module_account: u64 = <Test as crate::Config>::PalletId::get().into_account();
		assert_eq!(Tokens::accounts(module_account, VSBOND).free, 80);
		assert_eq!(Tokens::accounts(module_account, TOKEN).free, 80);

		let treasury_account: u64 = <Test as crate::Config>::TreasuryAccount::get();
		assert_eq!(Tokens::accounts(treasury_account, TOKEN).free, 8 + 8);
	});
}

// Test Utilities
#[test]
fn check_price_to_pay() {
	let unit_price: FixedU128 = FixedU128::from((33, 100));
	let quantities: [BalanceOf<Test>; 4] = [3, 33, 333, 3333];
	let price_to_pays: [BalanceOf<Test>; 4] = [0, 10, 109, 1099];

	for (quantity, price_to_pay) in quantities.iter().zip(price_to_pays.iter()) {
		assert_eq!(Auction::price_to_pay(*quantity, unit_price), *price_to_pay);
	}
}

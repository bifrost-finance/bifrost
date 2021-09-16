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

use frame_support::{assert_noop, assert_ok, dispatch::DispatchError, traits::BalanceStatus};
use orml_traits::{LockIdentifier, MultiLockableCurrency};

use crate::{mock::*, *};

#[test]
fn create_sell_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
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
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 100);
	});
}

#[test]
fn create_buy_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			13,
			20,
			100,
			100,
			OrderType::Buy
		));

		assert_eq!(Auction::order_id(), 1);

		let user_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert!(user_order_ids.contains(&0));

		assert!(Auction::order_info(&0).is_some());

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 100);
	});
}

#[test]
fn double_create_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			13,
			20,
			50,
			50,
			OrderType::Sell
		));
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 50, OrderType::Buy));

		assert_eq!(Auction::order_id(), 2);

		let user_sell_order_ids = Auction::user_order_ids(ALICE, OrderType::Sell);
		let user_buy_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert!(user_sell_order_ids.contains(&0));
		assert!(user_buy_order_ids.contains(&1));

		assert!(Auction::order_info(&0).is_some());
		assert!(Auction::order_info(&1).is_some());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 50);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 50);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 50);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 50);
	});
}

#[test]
fn create_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Auction::create_order(Origin::root(), 3000, 13, 20, 100, 100, OrderType::Sell),
			DispatchError::BadOrigin
		);
		assert_noop!(
			Auction::create_order(Origin::none(), 3000, 13, 20, 100, 100, OrderType::Buy),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn create_order_under_minimum_amount_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 0, 0, OrderType::Sell),
			Error::<Test>::NotEnoughAmount
		);

		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 0, 0, OrderType::Buy),
			Error::<Test>::NotEnoughAmount
		);
	});
}

#[test]
fn create_order_without_enough_to_reserve_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 1000, 1000, OrderType::Sell),
			Error::<Test>::NotEnoughBalanceToReserve,
		);

		const LOCK_ID_SELL: LockIdentifier = 0u64.to_be_bytes();
		assert_ok!(Tokens::set_lock(LOCK_ID_SELL, VSBOND, &ALICE, 50));
		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 51, 51, OrderType::Sell),
			Error::<Test>::NotEnoughBalanceToReserve,
		);

		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 1000, 1000, OrderType::Buy),
			Error::<Test>::NotEnoughBalanceToReserve,
		);

		const LOCK_ID_BUY: LockIdentifier = 1u64.to_be_bytes();
		assert_ok!(Tokens::set_lock(LOCK_ID_BUY, TOKEN, &ALICE, 50));
		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 51, 51, OrderType::Buy),
			Error::<Test>::NotEnoughBalanceToReserve,
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
				13,
				20,
				1,
				1,
				OrderType::Sell
			));
			assert_ok!(Auction::create_order(
				Some(ALICE).into(),
				3000,
				13,
				20,
				1,
				1,
				OrderType::Buy
			));
		}

		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 1, 1, OrderType::Sell),
			Error::<Test>::ExceedMaximumOrderInTrade,
		);

		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 1, 1, OrderType::Buy),
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
fn revoke_order_which_be_partial_clinchd_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			13,
			20,
			50,
			50,
			OrderType::Sell
		));
		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 25));
		assert_ok!(Auction::revoke_order(Some(ALICE).into(), 0));
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 50, OrderType::Buy));
		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 1, 25));
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
fn revoke_order_not_exist_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(Auction::revoke_order(Some(ALICE).into(), 0), Error::<Test>::NotFindOrderInfo);
	});
}

#[test]
fn revoke_order_without_enough_reserved_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			13,
			20,
			50,
			50,
			OrderType::Sell
		));
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 50, OrderType::Buy));

		assert_ok!(Tokens::repatriate_reserved(
			VSBOND,
			&ALICE,
			&BRUCE,
			25,
			BalanceStatus::Reserved
		));
		assert_ok!(Tokens::repatriate_reserved(TOKEN, &ALICE, &BRUCE, 25, BalanceStatus::Reserved));

		assert_noop!(
			Auction::revoke_order(Some(ALICE).into(), 0),
			Error::<Test>::NotEnoughBalanceToUnreserve
		);
		assert_noop!(
			Auction::revoke_order(Some(ALICE).into(), 1),
			Error::<Test>::NotEnoughBalanceToUnreserve
		);
	});
}

#[test]
fn revoke_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(
			Some(ALICE).into(),
			3000,
			13,
			20,
			50,
			50,
			OrderType::Sell
		));
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 50, OrderType::Buy));

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

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 67);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 110);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 133);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 90);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).reserved, 0);

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

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 133);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 67);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 23);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 67);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 110);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).reserved, 0);

		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 9999999));

		let user_buy_order_ids = Auction::user_order_ids(ALICE, OrderType::Buy);
		assert_eq!(user_buy_order_ids.len(), 0);

		assert!(Auction::order_info(0).is_none());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 200);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 67);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 1);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 132);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).reserved, 0);
	});
}
//
// #[test]
// fn partial_clinch_order_not_exist_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 100));
// 		assert_noop!(Auction::clinch_order(Some(BRUCE).into(), 1), Error::<Test>::NotFindOrderInfo);
// 		assert_noop!(
// 			Auction::partial_clinch_order(Some(BRUCE).into(), 1, 50),
// 			Error::<Test>::NotFindOrderInfo
// 		);
// 	});
// }
//
// #[test]
// fn clinch_order_by_origin_illegal_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 100));
// 		assert_noop!(
// 			Auction::clinch_order(Some(ALICE).into(), 0),
// 			Error::<Test>::ForbidClinchOrderWithinOwnership
// 		);
// 		assert_noop!(
// 			Auction::partial_clinch_order(Some(ALICE).into(), 0, 50),
// 			Error::<Test>::ForbidClinchOrderWithinOwnership
// 		);
//
// 		assert_noop!(
// 			Auction::partial_clinch_order(Origin::root(), 0, 50),
// 			DispatchError::BadOrigin,
// 		);
// 		assert_noop!(
// 			Auction::partial_clinch_order(Origin::none(), 0, 50),
// 			DispatchError::BadOrigin,
// 		);
// 	});
// }
//
// #[test]
// fn clinch_order_not_in_trade_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 100));
// 		assert_ok!(Auction::revoke_order(Some(ALICE).into(), 0));
// 		assert_noop!(
// 			Auction::partial_clinch_order(Some(BRUCE).into(), 0, 50),
// 			Error::<Test>::ForbidClinchOrderNotInTrade
// 		);
//
// 		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 100));
// 		assert_ok!(Auction::clinch_order(Some(BRUCE).into(), 1));
// 		assert_noop!(
// 			Auction::partial_clinch_order(Some(BRUCE).into(), 1, 50),
// 			Error::<Test>::ForbidClinchOrderNotInTrade
// 		);
// 	});
// }
//
// #[test]
// fn clinch_order_without_enough_token_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 200));
// 		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 50));
// 		assert_noop!(
// 			Auction::clinch_order(Some(BRUCE).into(), 0),
// 			Error::<Test>::DontHaveEnoughToPay
// 		);
// 	});
// }
//
// // Test Utilities
// #[test]
// fn check_price_to_pay() {
// 	let unit_price: U64F64 = 0.333f64.to_fixed();
// 	let quantities: [BalanceOf<Test>; 4] = [3, 33, 333, 3333];
// 	let price_to_pays: [BalanceOf<Test>; 4] = [1, 11, 111, 1110];
//
// 	for (quantity, price_to_pay) in quantities.iter().zip(price_to_pays.iter()) {
// 		assert_eq!(Auction::price_to_pay(*quantity, unit_price), *price_to_pay);
// 	}
// }

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

#![cfg(test)]

use frame_support::{assert_noop, assert_ok, dispatch::DispatchError, traits::BalanceStatus};
use orml_traits::{LockIdentifier, MultiLockableCurrency};

use crate::{mock::*, *};

#[test]
fn create_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 1));

		assert_eq!(Auction::order_id(), 1);

		let in_trade_order_ids = Auction::in_trade_order_ids(ALICE).unwrap();
		assert!(in_trade_order_ids.contains(&0));

		assert!(Auction::revoked_order_ids(ALICE).is_none());
		assert!(Auction::clinchd_order_ids(ALICE).is_none());

		assert!(Auction::order_info(&0).is_some());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 100);
	});
}

#[test]
fn double_create_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 1));
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 1));

		assert_eq!(Auction::order_id(), 2);

		let in_trade_order_ids = Auction::in_trade_order_ids(ALICE).unwrap();
		assert!(in_trade_order_ids.contains(&0));
		assert!(in_trade_order_ids.contains(&1));

		assert!(Auction::revoked_order_ids(ALICE).is_none());
		assert!(Auction::clinchd_order_ids(ALICE).is_none());

		assert!(Auction::order_info(&0).is_some());
		assert!(Auction::order_info(&1).is_some());

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 100);
	});
}

#[test]
fn create_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Auction::create_order(Origin::root(), 3000, 13, 20, 100, 1),
			DispatchError::BadOrigin
		);
		assert_noop!(
			Auction::create_order(Origin::none(), 3000, 13, 20, 100, 1),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn create_order_under_minimum_supply_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 0, 1),
			Error::<Test>::NotEnoughSupply
		);
	});
}

#[test]
fn create_order_without_enough_vsbond_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 1000, 1),
			Error::<Test>::NotEnoughBalanceToReserve,
		);

		const LOCK_ID: LockIdentifier = 0u64.to_be_bytes();
		assert_ok!(Tokens::set_lock(LOCK_ID, VSBOND, &ALICE, 50));
		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 51, 1),
			Error::<Test>::NotEnoughBalanceToReserve,
		);
	});
}

#[test]
fn create_order_exceed_maximum_order_in_trade_should_fail() {
	new_test_ext().execute_with(|| {
		for _ in 0 .. MaximumOrderInTrade::get() {
			assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 1, 1));
		}

		assert_noop!(
			Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 1, 1),
			Error::<Test>::ExceedMaximumOrderInTrade,
		);
	});
}

#[test]
fn revoke_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 1));
		assert_ok!(Auction::revoke_order(Some(ALICE).into(), 0));

		assert_eq!(Auction::order_id(), 1);

		let in_trade_order_ids = Auction::in_trade_order_ids(ALICE).unwrap();
		assert!(in_trade_order_ids.is_empty());

		let revoked_order_ids = Auction::revoked_order_ids(ALICE).unwrap();
		assert!(revoked_order_ids.contains(&0));

		assert!(Auction::clinchd_order_ids(ALICE).is_none());

		let order_info = Auction::order_info(0).unwrap();
		assert_eq!(order_info.order_state, OrderState::Revoked);

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 100);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);
	});
}

#[test]
fn revoke_order_which_be_partial_clinchd_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 1));
		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 25));
		assert_ok!(Auction::revoke_order(Some(ALICE).into(), 0));

		assert_eq!(Auction::order_id(), 1);

		let in_trade_order_ids = Auction::in_trade_order_ids(ALICE).unwrap();
		assert!(in_trade_order_ids.is_empty());

		let revoked_order_ids = Auction::revoked_order_ids(ALICE).unwrap();
		assert!(revoked_order_ids.contains(&0));

		assert!(Auction::clinchd_order_ids(ALICE).is_none());

		let order_info = Auction::order_info(0).unwrap();
		assert_eq!(order_info.order_state, OrderState::Revoked);
		assert_eq!(order_info.remain, 25);

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 75);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);
	});
}

#[test]
fn revoke_order_not_exist_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 1));
		assert_noop!(
			Auction::partial_clinch_order(Some(BRUCE).into(), 1, 25),
			Error::<Test>::NotFindOrderInfo
		);
	});
}

#[test]
fn revoke_order_without_enough_reserved_vsbond_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 1));
		assert_ok!(Tokens::repatriate_reserved(
			VSBOND,
			&ALICE,
			&BRUCE,
			25,
			BalanceStatus::Reserved
		));
		assert_noop!(
			Auction::revoke_order(Some(ALICE).into(), 0),
			Error::<Test>::NotEnoughBalanceToUnreserve
		);
	});
}

#[test]
fn revoke_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 1));
		assert_noop!(
			Auction::revoke_order(Some(BRUCE).into(), 0),
			Error::<Test>::ForbidRevokeOrderWithoutOwnership
		);
		assert_noop!(Auction::revoke_order(Origin::root(), 0), DispatchError::BadOrigin);
		assert_noop!(Auction::revoke_order(Origin::none(), 0), DispatchError::BadOrigin);
	});
}

#[test]
fn revoke_order_not_in_trade_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 1));
		assert_ok!(Auction::revoke_order(Some(ALICE).into(), 0));
		assert_noop!(
			Auction::revoke_order(Some(ALICE).into(), 0),
			Error::<Test>::ForbidRevokeOrderNotInTrade
		);

		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 50, 1));
		assert_ok!(Auction::clinch_order(Some(BRUCE).into(), 1));
		assert_noop!(
			Auction::revoke_order(Some(ALICE).into(), 1),
			Error::<Test>::ForbidRevokeOrderNotInTrade
		);
	});
}

#[test]
fn partial_clinch_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 1));
		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 25));

		assert_eq!(Auction::order_id(), 1);

		let in_trade_order_ids = Auction::in_trade_order_ids(ALICE).unwrap();
		assert!(in_trade_order_ids.contains(&0));

		assert!(Auction::revoked_order_ids(ALICE).is_none());
		assert!(Auction::clinchd_order_ids(ALICE).is_none());

		let order_info = Auction::order_info(0).unwrap();
		assert_eq!(order_info.remain, 75);
		assert_eq!(order_info.order_state, OrderState::InTrade);

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 75);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 125);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 125);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 75);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).reserved, 0);

		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 9999999));

		let in_trade_order_ids = Auction::in_trade_order_ids(ALICE).unwrap();
		assert!(in_trade_order_ids.is_empty());

		assert!(Auction::revoked_order_ids(ALICE).is_none());

		let clinchd_order_ids = Auction::clinchd_order_ids(ALICE).unwrap();
		assert!(clinchd_order_ids.contains(&0));

		let order_info = Auction::order_info(0).unwrap();
		assert_eq!(order_info.remain, 0);
		assert_eq!(order_info.order_state, OrderState::Clinchd);

		assert_eq!(Tokens::accounts(ALICE, VSBOND).free, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(ALICE, TOKEN).free, 200);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(ALICE, TOKEN).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, VSBOND).free, 200);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, VSBOND).reserved, 0);

		assert_eq!(Tokens::accounts(BRUCE, TOKEN).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, TOKEN).reserved, 0);
	});
}

#[test]
fn partial_clinch_order_not_exist_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 1));
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
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 1));
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
fn clinch_order_not_in_trade_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 1));
		assert_ok!(Auction::revoke_order(Some(ALICE).into(), 0));
		assert_noop!(
			Auction::partial_clinch_order(Some(BRUCE).into(), 0, 50),
			Error::<Test>::ForbidClinchOrderNotInTrade
		);

		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 1));
		assert_ok!(Auction::clinch_order(Some(BRUCE).into(), 1));
		assert_noop!(
			Auction::partial_clinch_order(Some(BRUCE).into(), 1, 50),
			Error::<Test>::ForbidClinchOrderNotInTrade
		);
	});
}

#[test]
fn clinch_order_without_enough_token_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Auction::create_order(Some(ALICE).into(), 3000, 13, 20, 100, 2));
		assert_ok!(Auction::partial_clinch_order(Some(BRUCE).into(), 0, 50));
		assert_noop!(Auction::clinch_order(Some(BRUCE).into(), 0), Error::<Test>::CantPayThePrice);
	});
}

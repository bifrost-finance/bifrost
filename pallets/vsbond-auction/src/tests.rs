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

use crate::mock::*;
use crate::*;
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use orml_traits::{MultiCurrency, MultiLockableCurrency};

#[test]
fn create_order_should_work() {
	let _ = new_test_ext().execute_with(|| -> DispatchResultWithPostInfo {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND,
			UNIT_PRICE,
		));

		// Check storage
		let in_trade_order_ids =
			VSBondAuction::in_trade_order_ids(ACCOUNT_ALICE).ok_or(Error::<Test>::Unexpected)?;
		assert!(in_trade_order_ids.contains(&0));
		assert_eq!(in_trade_order_ids.len(), 1);

		Ok(().into())
	});
}

#[test]
fn create_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			VSBondAuction::create_order(
				Origin::root(),
				PARA_ID,
				FIRST_SLOT,
				LAST_SLOT,
				BALANCE_VSBOND,
				UNIT_PRICE,
			),
			DispatchError::BadOrigin,
		);
		assert_noop!(
			VSBondAuction::create_order(
				Origin::none(),
				PARA_ID,
				FIRST_SLOT,
				LAST_SLOT,
				BALANCE_VSBOND,
				UNIT_PRICE,
			),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn create_order_without_enough_currency_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			VSBondAuction::create_order(
				Origin::signed(ACCOUNT_ALICE),
				PARA_ID,
				FIRST_SLOT,
				LAST_SLOT,
				BALANCE_VSBOND + 1,
				UNIT_PRICE,
			),
			orml_tokens::Error::<Test>::BalanceTooLow,
		);
	});
}

#[test]
fn create_order_with_enough_currency_under_lock_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(OrmlAssets::extend_lock(
			1u64.to_be_bytes(),
			VSBOND,
			&ACCOUNT_ALICE,
			BALANCE_VSBOND
		));

		assert_noop!(
			VSBondAuction::create_order(
				Origin::signed(ACCOUNT_ALICE),
				PARA_ID,
				FIRST_SLOT,
				LAST_SLOT,
				BALANCE_VSBOND,
				UNIT_PRICE,
			),
			orml_tokens::Error::<Test>::LiquidityRestrictions,
		);
	});
}

#[test]
fn create_order_should_increase_order_id() {
	new_test_ext().execute_with(|| {
		for i in 0..MaximumOrderInTrade::get() {
			assert_ok!(VSBondAuction::create_order(
				Origin::signed(ACCOUNT_ALICE),
				PARA_ID,
				FIRST_SLOT,
				LAST_SLOT,
				1,
				UNIT_PRICE,
			));

			assert_eq!(VSBondAuction::order_id(), i as u64 + 1);
		}
	});
}

#[test]
fn create_order_exceed_maximum_order_in_trade_should_fail() {
	let _ = new_test_ext().execute_with(|| -> DispatchResultWithPostInfo {
		for _ in 0..MaximumOrderInTrade::get() {
			assert_ok!(VSBondAuction::create_order(
				Origin::signed(ACCOUNT_ALICE),
				PARA_ID,
				FIRST_SLOT,
				LAST_SLOT,
				1,
				UNIT_PRICE,
			));
		}

		assert_noop!(
			VSBondAuction::create_order(
				Origin::signed(ACCOUNT_ALICE),
				PARA_ID,
				FIRST_SLOT,
				LAST_SLOT,
				1,
				UNIT_PRICE,
			),
			Error::<Test>::ExceedMaximumOrderInTrade,
		);

		Ok(().into())
	});
}

#[test]
fn create_order_should_lock_vsbond() {
	new_test_ext().execute_with(|| {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND,
			UNIT_PRICE,
		));

		assert_noop!(
			OrmlAssets::ensure_can_withdraw(VSBOND, &ACCOUNT_ALICE, BALANCE_VSBOND),
			orml_tokens::Error::<Test>::LiquidityRestrictions
		);
	});
}

#[test]
fn revoke_order_should_work() {
	let _ = new_test_ext().execute_with(|| -> DispatchResultWithPostInfo {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND,
			UNIT_PRICE,
		));

		assert_ok!(VSBondAuction::revoke_order(
			Origin::signed(ACCOUNT_ALICE),
			0,
		));

		let in_trade_order_ids =
			VSBondAuction::in_trade_order_ids(ACCOUNT_ALICE).ok_or(Error::<Test>::Unexpected)?;
		assert!(in_trade_order_ids.len() == 0);
		let revoked_order_ids =
			VSBondAuction::revoked_order_ids(ACCOUNT_ALICE).ok_or(Error::<Test>::Unexpected)?;
		assert!(revoked_order_ids.len() == 1 && revoked_order_ids.contains(&0));

		Ok(().into())
	});
}

#[test]
fn revoke_order_not_exist_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			VSBondAuction::revoke_order(Origin::signed(ACCOUNT_ALICE), 0,),
			Error::<Test>::NotFindOrderInfo,
		);
	});
}

#[test]
fn revoke_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND,
			UNIT_PRICE,
		));

		assert_noop!(
			VSBondAuction::revoke_order(Origin::root(), 0),
			DispatchError::BadOrigin,
		);
		assert_noop!(
			VSBondAuction::revoke_order(Origin::signed(ACCOUNT_BRUCE), 0),
			Error::<Test>::ForbidRevokeOrderWithoutOwnership,
		);
		assert_noop!(
			VSBondAuction::revoke_order(Origin::none(), 0),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn revoke_order_not_in_trade_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND / 2,
			UNIT_PRICE,
		));

		assert_ok!(VSBondAuction::revoke_order(
			Origin::signed(ACCOUNT_ALICE),
			0
		),);
		assert_noop!(
			VSBondAuction::revoke_order(Origin::signed(ACCOUNT_ALICE), 0),
			Error::<Test>::ForbidRevokeOrderNotInTrade,
		);

		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND / 2,
			UNIT_PRICE,
		));

		assert_ok!(VSBondAuction::clinch_order(
			Origin::signed(ACCOUNT_BRUCE),
			1,
		));
		assert_noop!(
			VSBondAuction::revoke_order(Origin::signed(ACCOUNT_ALICE), 1),
			Error::<Test>::ForbidRevokeOrderNotInTrade,
		);
	});
}

#[test]
fn revoke_order_should_unlock_vsbond() {
	new_test_ext().execute_with(|| {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND,
			UNIT_PRICE,
		));
		assert_ok!(VSBondAuction::revoke_order(
			Origin::signed(ACCOUNT_ALICE),
			0,
		));

		assert_ok!(OrmlAssets::ensure_can_withdraw(
			VSBOND,
			&ACCOUNT_ALICE,
			BALANCE_VSBOND
		),);
	});
}

#[test]
fn clinch_order_should_work() {
	let _ = new_test_ext().execute_with(|| -> DispatchResultWithPostInfo {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND,
			UNIT_PRICE,
		));

		assert_ok!(VSBondAuction::partial_clinch_order(
			Origin::signed(ACCOUNT_BRUCE),
			0,
			BALANCE_VSBOND / 2,
		));

		// Check remain
		let order_info = VSBondAuction::order_info(0).ok_or(Error::<Test>::Unexpected)?;
		assert_eq!(order_info.remain, BALANCE_VSBOND / 2);
		assert_eq!(order_info.order_state, OrderState::InTrade);

		assert_ok!(VSBondAuction::clinch_order(
			Origin::signed(ACCOUNT_BRUCE),
			0,
		));

		// Check balance
		assert_eq!(
			OrmlAssets::free_balance(TOKEN, &ACCOUNT_ALICE),
			BALANCE_TOKEN.saturating_mul(2)
		);
		assert_eq!(
			OrmlAssets::free_balance(VSBOND, &ACCOUNT_BRUCE),
			BALANCE_VSBOND.saturating_mul(2)
		);

		// Check storage
		let order_info = VSBondAuction::order_info(0).ok_or(Error::<Test>::Unexpected)?;
		assert_eq!(order_info.order_state, OrderState::Clinchd);

		let in_trade_order_ids =
			VSBondAuction::in_trade_order_ids(ACCOUNT_ALICE).ok_or(Error::<Test>::Unexpected)?;
		assert_eq!(in_trade_order_ids.len(), 0);
		let clinchd_order_ids =
			VSBondAuction::clinchd_order_ids(ACCOUNT_ALICE).ok_or(Error::<Test>::Unexpected)?;
		assert_eq!(clinchd_order_ids.len(), 1);
		assert!(clinchd_order_ids.contains(&0));

		Ok(().into())
	});
}

#[test]
fn clinch_order_not_exist_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			VSBondAuction::partial_clinch_order(
				Origin::signed(ACCOUNT_BRUCE),
				0,
				BALANCE_VSBOND / 2,
			),
			Error::<Test>::NotFindOrderInfo,
		);

		assert_noop!(
			VSBondAuction::clinch_order(Origin::signed(ACCOUNT_BRUCE), 0,),
			Error::<Test>::NotFindOrderInfo,
		);
	});
}

#[test]
fn clinck_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND,
			UNIT_PRICE,
		));

		assert_noop!(
			VSBondAuction::partial_clinch_order(Origin::root(), 0, BALANCE_VSBOND / 2),
			DispatchError::BadOrigin
		);
		assert_noop!(
			VSBondAuction::partial_clinch_order(
				Origin::signed(ACCOUNT_ALICE),
				0,
				BALANCE_VSBOND / 2
			),
			Error::<Test>::ForbidClinchOrderWithinOwnership
		);
		assert_noop!(
			VSBondAuction::partial_clinch_order(Origin::none(), 0, BALANCE_VSBOND / 2),
			DispatchError::BadOrigin
		);

		assert_noop!(
			VSBondAuction::clinch_order(Origin::root(), 0),
			DispatchError::BadOrigin
		);
		assert_noop!(
			VSBondAuction::clinch_order(Origin::signed(ACCOUNT_ALICE), 0,),
			Error::<Test>::ForbidClinchOrderWithinOwnership
		);
		assert_noop!(
			VSBondAuction::clinch_order(Origin::none(), 0),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn clinch_order_not_in_trade_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND / 2,
			UNIT_PRICE,
		));
		assert_ok!(VSBondAuction::revoke_order(
			Origin::signed(ACCOUNT_ALICE),
			0,
		));
		assert_noop!(
			VSBondAuction::clinch_order(Origin::signed(ACCOUNT_BRUCE), 0),
			Error::<Test>::ForbidClinchOrderNotInTrade
		);

		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND / 2,
			UNIT_PRICE,
		));
		assert_ok!(VSBondAuction::clinch_order(
			Origin::signed(ACCOUNT_BRUCE),
			1,
		));
		assert_noop!(
			VSBondAuction::clinch_order(Origin::signed(ACCOUNT_BRUCE), 1),
			Error::<Test>::ForbidClinchOrderNotInTrade
		);
	});
}

// TODO: Weird Err??
// #[test]
// fn clinch_order_without_enough_currency_expected_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		assert_ok!(VSBondAuction::create_order(
// 			Origin::signed(ACCOUNT_ALICE),
// 			PARA_ID,
// 			FIRST_SLOT,
// 			LAST_SLOT,
// 			BALANCE_VSBOND,
// 			UNIT_PRICE + 1,
// 		));
//
// 		assert_noop!(
// 			VSBondAuction::clinch_order(Origin::signed(ACCOUNT_BRUCE), 0),
// 			orml_tokens::Error::<Test>::BalanceTooLow,
// 		);
// 	});
// }
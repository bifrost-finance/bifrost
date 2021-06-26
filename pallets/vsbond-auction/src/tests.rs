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
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use orml_traits::MultiLockableCurrency;

// fn create_order_mock(
// 	who: AccountIdOf<Test>,
// 	supply: BalanceOf<Test>,
// 	unit_price: BalanceOf<Test>,
// ) -> DispatchResultWithInfo<OrderId> {
//
// 	let CurrencyId::VSBond(_, index, first_slot, last_slot) = FAKE_VSBOND;
//
// 	VSBondAuction::create_order(
// 		Origin::signed(who),
// 		index,
// 		first_slot,
// 		last_slot,
// 		supply,
// 		unit_price,
// 	)?;
//
// 	Pallet::<Test>::order_id()
// }

#[test]
fn create_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(ACCOUNT_ALICE),
			PARA_ID,
			FIRST_SLOT,
			LAST_SLOT,
			BALANCE_VSBOND,
			UNIT_PRICE,
		));
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
	new_test_ext().execute_with(|| {
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
			crate::Error::<Test>::ExceedMaximumOrderInTrade,
		);
	});
}

// #[test]
// fn revoke_order_should_work() {
// 	new_test_ext().execute_with(|| {
// 		let order_id = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_VSBOND,
//             OrderState::InTrade,
// 		);
// 		assert_ok!(VSBondAuction::revoke_order(
// 			Origin::signed(ACCOUNT_ALICE),
// 			order_id
// 		));
// 	});
// }
//
// #[test]
// fn revoke_order_not_exist_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		let order_id = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_VSBOND,
//             OrderState::InTrade,
// 		);
// 		let order_id_illegal = order_id + 1;
// 		assert_noop!(
// 			VSBondAuction::revoke_order(Origin::signed(ACCOUNT_ALICE), order_id_illegal),
// 			Error::<Test>::NotFindOrderInfo
// 		);
// 	});
// }
//
// #[test]
// fn revoke_order_by_origin_illegal_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		let order_id = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_VSBOND,
//             OrderState::InTrade,
// 		);
//
// 		assert_noop!(
// 			VSBondAuction::revoke_order(Origin::root(), order_id),
// 			DispatchError::BadOrigin,
// 		);
// 		assert_noop!(
// 			VSBondAuction::revoke_order(Origin::signed(ACCOUNT_BRUCE), order_id),
// 			Error::<Test>::ForbidRevokeOrderWithoutOwnership,
// 		);
// 		assert_noop!(
// 			VSBondAuction::revoke_order(Origin::none(), order_id),
// 			DispatchError::BadOrigin,
// 		);
// 	});
// }
//
// #[test]
// fn revoke_order_not_in_trade_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		let order_id_revoked = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_VSBOND,
//             OrderState::Revoked,
// 		);
// 		let order_id_clinchd = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_VSBOND,
//             OrderState::Clinchd,
// 		);
//
// 		assert_noop!(
// 			VSBondAuction::revoke_order(Origin::signed(ACCOUNT_ALICE), order_id_revoked),
// 			Error::<Test>::ForbidRevokeOrderNotInTrade,
// 		);
// 		assert_noop!(
// 			VSBondAuction::revoke_order(Origin::signed(ACCOUNT_ALICE), order_id_clinchd),
// 			Error::<Test>::ForbidRevokeOrderNotInTrade,
// 		);
// 	});
// }
//
// #[test]
// fn clinch_order_should_work() {
// 	new_test_ext().execute_with(|| {
// 		let order_id = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_VSBOND,
//             OrderState::InTrade,
// 		);
//
// 		assert_ok!(VSBondAuction::clinch_order(
// 			Origin::signed(ACCOUNT_BRUCE),
// 			order_id
// 		));
// 	});
// }
//
// #[test]
// fn clinch_order_not_exist_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		let order_id = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_VSBOND,
//             OrderState::InTrade,
// 		);
//
// 		let order_id_illegal = order_id + 1;
// 		assert_noop!(
// 			VSBondAuction::clinch_order(Origin::signed(ACCOUNT_ALICE), order_id_illegal),
// 			Error::<Test>::NotFindOrderInfo
// 		);
// 	});
// }
//
// #[test]
// fn clinck_order_by_origin_illegal_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		let order_id = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_VSBOND,
//             OrderState::InTrade,
// 		);
//
// 		assert_noop!(
// 			VSBondAuction::clinch_order(Origin::root(), order_id),
// 			DispatchError::BadOrigin
// 		);
// 		assert_noop!(
// 			VSBondAuction::clinch_order(Origin::signed(ACCOUNT_ALICE), order_id),
// 			Error::<Test>::ForbidClinchOrderWithinOwnership
// 		);
// 		assert_noop!(
// 			VSBondAuction::clinch_order(Origin::none(), order_id),
// 			DispatchError::BadOrigin
// 		);
// 	});
// }
//
// #[test]
// fn clinch_order_not_in_trade_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		let order_id_revoked = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_VSBOND,
//             OrderState::Revoked,
// 		);
// 		let order_id_clinchd = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_VSBOND,
//             OrderState::Clinchd,
// 		);
// 		assert_noop!(
// 			VSBondAuction::clinch_order(Origin::signed(ACCOUNT_BRUCE), order_id_revoked),
// 			Error::<Test>::ForbidClinchOrderNotInTrade
// 		);
// 		assert_noop!(
// 			VSBondAuction::clinch_order(Origin::signed(ACCOUNT_BRUCE), order_id_clinchd),
// 			Error::<Test>::ForbidClinchOrderNotInTrade
// 		);
// 	});
// }
//
// #[test]
// fn clinch_order_without_enough_currency_expected_should_fail() {
// 	new_test_ext().execute_with(|| {
// 		let order_id = create_order_for_test(
//             ACCOUNT_ALICE,
//             CURRENCY_OWNED_BY_ALICE,
//             BALANCE_VSBOND,
//             CURRENCY_OWNED_BY_BRUCE,
//             BALANCE_EXCEEDED,
//             OrderState::InTrade,
// 		);
//
// 		assert_noop!(
// 			VSBondAuction::clinch_order(Origin::signed(ACCOUNT_BRUCE), order_id),
// 			Error::<Test>::NotEnoughCurrencyToBuy
// 		);
// 	});
// }

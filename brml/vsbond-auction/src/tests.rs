#![cfg(test)]

use crate::mock::*;
use crate::*;
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use node_primitives::{AssetId, Balance};

#[test]
fn create_order_should_work() {
	todo!()
}

#[test]
fn create_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		let origin_signed = Origin::signed(1);
		let currency_sold: AssetId = 1;
		let amount_sold: Balance = 100;
		let currency_expected: AssetId = 2;
		let amount_expected: Balance = 100;

		assert_ok!(VSBondAuction::create_order(
			origin_signed,
			currency_sold,
			amount_sold,
			currency_expected,
			amount_expected,
		));
	});
}

#[test]
fn create_order_without_enough_currency_should_fail() {
	new_test_ext().execute_with(|| {
		let origin_signed = Origin::signed(1);
		let currency_sold: AssetId = 1;
		let amount_sold_more_than_owned: Balance = 1000;
		let currency_expected: AssetId = 2;
		let amount_expected: Balance = 100;

		assert_noop!(
			VSBondAuction::create_order(
				origin_signed,
				currency_sold,
				amount_sold_more_than_owned,
				currency_expected,
				amount_expected,
			),
			Error::<Test>::NotEnoughCurrencySold
		);
	});
}

#[test]
fn revoke_order_should_work() {}

#[test]
fn revoke_order_not_exist_should_fail() {}

#[test]
fn revoke_order_by_origin_illegal_should_fail() {}

#[test]
fn revoke_order_not_in_trade_should_fail() {}

#[test]
fn clinch_order_should_work() {}

#[test]
fn clinch_order_not_exist_should_fail() {}

#[test]
fn clinck_order_by_origin_illegal_should_fail() {}

#[test]
fn clinch_order_not_in_trade_should_fail() {}

#![cfg(test)]

use crate::mock::*;
use crate::*;
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};

fn create_order_for_test(
	owner: AccountIdOf<Test>,
	currency_sold: CurrencyIdOf<Test>,
	amount_sold: BalanceOf<Test>,
	currency_expected: CurrencyIdOf<Test>,
	amount_expected: BalanceOf<Test>,
	order_state: OrderState,
) -> OrderId {
	let order_id = Pallet::<Test>::next_order_id();
	let order_info = OrderInfo {
		owner: owner.clone(),
		currency_sold,
		amount_sold,
		currency_expected,
		amount_expected,
		order_id,
		order_state,
	};

	TotalOrders::<Test>::insert(order_id, order_info);
	SellerOrders::<Test>::mutate(owner, currency_sold, |order_ids| order_ids.push(order_id));

	order_id
}

#[test]
fn create_order_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(VSBondAuction::create_order(
			Origin::signed(1),
			1,
			100,
			2,
			100
		),);
	});
}

#[test]
fn create_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			VSBondAuction::create_order(Origin::root(), 1, 100, 2, 100,),
			DispatchError::BadOrigin,
		);
		assert_noop!(
			VSBondAuction::create_order(Origin::none(), 1, 100, 2, 100,),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn create_order_without_enough_currency_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			VSBondAuction::create_order(Origin::signed(1), 1, 1_000, 2, 100,),
			Error::<Test>::NotEnoughCurrencySold,
		);
	});
}

#[test]
fn revoke_order_should_work() {
	new_test_ext().execute_with(|| {
		let order_id = create_order_for_test(1, 1, 100, 2, 100, OrderState::InTrade);
		assert_ok!(VSBondAuction::revoke_order(Origin::signed(1), order_id));
	});
}

#[test]
fn revoke_order_not_exist_should_fail() {
	new_test_ext().execute_with(|| {
		let order_id = create_order_for_test(1, 1, 100, 2, 100, OrderState::InTrade);
		assert_noop!(
			VSBondAuction::revoke_order(Origin::signed(1), order_id + 1),
			Error::<Test>::NotFindOrderInfo
		);
	});
}

#[test]
fn revoke_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		let order_id = create_order_for_test(1, 1, 100, 2, 100, OrderState::InTrade);

		assert_noop!(
			VSBondAuction::revoke_order(Origin::root(), order_id),
			DispatchError::BadOrigin,
		);
		assert_noop!(
			VSBondAuction::revoke_order(Origin::signed(2), order_id),
			Error::<Test>::ForbidRevokeOrderWithoutOwnership,
		);
		assert_noop!(
			VSBondAuction::revoke_order(Origin::none(), order_id),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn revoke_order_not_in_trade_should_fail() {
	new_test_ext().execute_with(|| {
		let order_id_revoked = create_order_for_test(1, 1, 100, 2, 100, OrderState::Revoked);
		let order_id_clinchd = create_order_for_test(1, 1, 100, 2, 100, OrderState::Clinchd);

		assert_noop!(
			VSBondAuction::revoke_order(Origin::signed(1), order_id_revoked),
			Error::<Test>::ForbidRevokeOrderNotInTrade,
		);
		assert_noop!(
			VSBondAuction::revoke_order(Origin::signed(1), order_id_clinchd),
			Error::<Test>::ForbidRevokeOrderNotInTrade,
		);
	});
}

#[test]
fn clinch_order_should_work() {
	new_test_ext().execute_with(|| {
		let order_id = create_order_for_test(1, 1, 100, 2, 100, OrderState::InTrade);

		assert_ok!(VSBondAuction::clinch_order(Origin::signed(2), order_id));
	});
}

#[test]
fn clinch_order_not_exist_should_fail() {
	new_test_ext().execute_with(|| {
		let order_id = create_order_for_test(1, 1, 100, 2, 100, OrderState::InTrade);

		assert_noop!(
			VSBondAuction::clinch_order(Origin::signed(2), order_id + 1),
			Error::<Test>::NotFindOrderInfo
		);
	});
}

#[test]
fn clinck_order_by_origin_illegal_should_fail() {
	new_test_ext().execute_with(|| {
		let order_id = create_order_for_test(1, 1, 100, 2, 100, OrderState::InTrade);

		assert_noop!(
			VSBondAuction::clinch_order(Origin::root(), order_id),
			DispatchError::BadOrigin
		);
		assert_noop!(
			VSBondAuction::clinch_order(Origin::signed(1), order_id),
			Error::<Test>::ForbidClinchOrderWithinOwnership
		);
		assert_noop!(
			VSBondAuction::clinch_order(Origin::none(), order_id),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn clinch_order_not_in_trade_should_fail() {
	new_test_ext().execute_with(|| {
		let order_id_revoked = create_order_for_test(1, 1, 100, 2, 100, OrderState::Revoked);
		let order_id_clinchd = create_order_for_test(1, 1, 100, 2, 100, OrderState::Clinchd);

		assert_noop!(
			VSBondAuction::clinch_order(Origin::signed(2), order_id_revoked),
			Error::<Test>::ForbidClinchOrderNotInTrade
		);
		assert_noop!(
			VSBondAuction::clinch_order(Origin::signed(2), order_id_clinchd),
			Error::<Test>::ForbidClinchOrderNotInTrade
		);
	});
}

#[test]
fn clinch_order_without_enough_currency_expected_should_fail() {
	new_test_ext().execute_with(|| {
		let order_id = create_order_for_test(1, 1, 100, 2, 1_000, OrderState::InTrade);

		assert_noop!(
			VSBondAuction::clinch_order(Origin::signed(2), order_id),
			Error::<Test>::NotEnoughCurrencyExpected
		);
	});
}

use crate::{
	mock::{
		market_mock, new_test_ext, LendMarket, RuntimeOrigin, Test, ALICE, BNC, DAVE, DOT, DOT_U,
		KSM, LKSM, LUSDT, PHA, VBNC,
	},
	tests::unit,
	Error, *,
};
use frame_support::{
	assert_err, assert_noop, assert_ok,
	traits::tokens::{fungibles::Inspect, Provenance},
};
use sp_runtime::{FixedPointNumber, TokenError};

#[test]
fn trait_inspect_methods_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			KSM,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		));
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			DOT,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		));
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			BNC,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		)); // No Deposits can't not withdraw
		assert_err!(
			LendMarket::can_withdraw(VBNC, &DAVE, 100).into_result(false),
			TokenError::FundsUnavailable
		);
		assert_eq!(LendMarket::total_issuance(VBNC), 0);
		assert_eq!(LendMarket::total_issuance(LKSM), 0);

		let minimum_balance = LendMarket::minimum_balance(VBNC);
		assert_eq!(minimum_balance, 0);

		assert_eq!(LendMarket::balance(VBNC, &DAVE), 0);

		// DAVE Deposit 100 BNC
		assert_ok!(LendMarket::mint(RuntimeOrigin::signed(DAVE), BNC, unit(100)));
		assert_eq!(LendMarket::balance(VBNC, &DAVE), unit(100) * 50);

		assert_eq!(
			LendMarket::reducible_balance(VBNC, &DAVE, Preservation::Expendable, Fortitude::Polite),
			unit(100) * 50
		);
		assert_ok!(LendMarket::collateral_asset(RuntimeOrigin::signed(DAVE), BNC, true));
		// Borrow 25 BNC will reduce 25 BNC liquidity for collateral_factor is 50%
		assert_ok!(LendMarket::borrow(RuntimeOrigin::signed(DAVE), BNC, unit(25)));

		assert_eq!(
			ExchangeRate::<Test>::get(BNC)
				.saturating_mul_int(AccountDeposits::<Test>::get(BNC, DAVE).voucher_balance),
			unit(100)
		);

		// DAVE Deposit 100 BNC, Borrow 25 BNC
		// Liquidity BNC 25
		// Formula: lend tokens = liquidity / price(1) / collateral(0.5) / exchange_rate(0.02)
		assert_eq!(
			LendMarket::reducible_balance(VBNC, &DAVE, Preservation::Expendable, Fortitude::Polite),
			unit(25) * 2 * 50
		);

		// Multi-asset case, additional deposit DOT_U
		// DAVE Deposit 100 BNC, 50 DOT_U, Borrow 25 BNC
		// Liquidity BNC = 25, DOT_U = 25
		// lend tokens = dollar(25 + 25) / 1 / 0.5 / 0.02 = dollar(50) * 100
		assert_ok!(LendMarket::mint(RuntimeOrigin::signed(DAVE), DOT_U, unit(50)));
		assert_eq!(LendMarket::balance(LUSDT, &DAVE), unit(50) * 50);
		assert_eq!(
			LendMarket::reducible_balance(
				LUSDT,
				&DAVE,
				Preservation::Expendable,
				Fortitude::Polite
			),
			unit(25) * 2 * 50
		);
		// enable DOT_U collateral
		assert_ok!(LendMarket::collateral_asset(RuntimeOrigin::signed(DAVE), DOT_U, true));
		assert_eq!(
			LendMarket::reducible_balance(VBNC, &DAVE, Preservation::Expendable, Fortitude::Polite),
			unit(25 + 25) * 2 * 50
		);

		assert_ok!(LendMarket::borrow(RuntimeOrigin::signed(DAVE), BNC, unit(50)));
		assert_eq!(
			LendMarket::reducible_balance(VBNC, &DAVE, Preservation::Expendable, Fortitude::Polite),
			0
		);

		assert_eq!(LendMarket::total_issuance(VBNC), unit(100) * 50);
		assert_ok!(LendMarket::can_deposit(VBNC, &DAVE, 100, Provenance::Minted).into_result());
		assert_ok!(LendMarket::can_withdraw(VBNC, &DAVE, 1000).into_result(false));
	})
}

#[test]
fn lend_token_unique_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			KSM,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		));
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			DOT,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		));
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			BNC,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		)); // lend_token_id already exists in `UnderlyingAssetId`
		assert_noop!(
			LendMarket::add_market(RuntimeOrigin::root(), LKSM, market_mock(VBNC)),
			Error::<Test>::InvalidLendTokenId
		);

		// lend_token_id cannot as the same as the asset id in `Markets`
		assert_noop!(
			LendMarket::add_market(RuntimeOrigin::root(), LKSM, market_mock(KSM)),
			Error::<Test>::InvalidLendTokenId
		);
	})
}

#[test]
fn transfer_lend_token_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			KSM,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		));
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			DOT,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		));
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			BNC,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		)); // DAVE Deposit 100 BNC
		assert_ok!(LendMarket::mint(RuntimeOrigin::signed(DAVE), BNC, unit(100)));

		// DAVE BNC collateral: deposit = 100
		// BNC: cash - deposit = 1000 - 100 = 900
		assert_eq!(
			ExchangeRate::<Test>::get(BNC)
				.saturating_mul_int(AccountDeposits::<Test>::get(BNC, DAVE).voucher_balance),
			unit(100)
		);

		// ALICE BNC collateral: deposit = 0
		assert_eq!(
			ExchangeRate::<Test>::get(BNC)
				.saturating_mul_int(AccountDeposits::<Test>::get(BNC, ALICE).voucher_balance),
			unit(0)
		);

		// Transfer lend tokens from DAVE to ALICE
		LendMarket::transfer(VBNC, &DAVE, &ALICE, unit(50) * 50, true).unwrap();

		// DAVE BNC collateral: deposit = 50
		assert_eq!(
			ExchangeRate::<Test>::get(BNC)
				.saturating_mul_int(AccountDeposits::<Test>::get(BNC, DAVE).voucher_balance),
			unit(50)
		);
		// DAVE Redeem 51 BNC should cause InsufficientDeposit
		assert_noop!(
			LendMarket::redeem_allowed(BNC, &DAVE, unit(51) * 50),
			Error::<Test>::InsufficientDeposit
		);

		// ALICE BNC collateral: deposit = 50
		assert_eq!(
			ExchangeRate::<Test>::get(BNC)
				.saturating_mul_int(AccountDeposits::<Test>::get(BNC, ALICE).voucher_balance),
			unit(50)
		);
		// ALICE Redeem 50 BNC should be succeeded
		assert_ok!(LendMarket::redeem_allowed(BNC, &ALICE, unit(50) * 50));
	})
}

#[test]
fn transfer_lend_tokens_under_collateral_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			KSM,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		));
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			DOT,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		));
		assert_ok!(LendMarket::add_market_bond(
			RuntimeOrigin::root(),
			BNC,
			vec![DOT, BNC, KSM, DOT_U, PHA]
		)); // DAVE Deposit 100 BNC
		assert_ok!(LendMarket::mint(RuntimeOrigin::signed(DAVE), BNC, unit(100)));
		assert_ok!(LendMarket::collateral_asset(RuntimeOrigin::signed(DAVE), BNC, true));

		// Borrow 50 BNC will reduce 50 BNC liquidity for collateral_factor is 50%
		assert_ok!(LendMarket::borrow(RuntimeOrigin::signed(DAVE), BNC, unit(50)));
		// Repay 40 BNC
		assert_ok!(LendMarket::repay_borrow(RuntimeOrigin::signed(DAVE), BNC, unit(40)));

		// Transfer 20 lend tokens from DAVE to ALICE
		LendMarket::transfer(VBNC, &DAVE, &ALICE, unit(20) * 50, true).unwrap();

		// DAVE Deposit BNC = 100 - 20 = 80
		// DAVE Borrow BNC = 0 + 50 - 40 = 10
		// DAVE liquidity BNC = 80 * 0.5 - 10 = 30
		assert_eq!(
			ExchangeRate::<Test>::get(BNC)
				.saturating_mul_int(AccountDeposits::<Test>::get(BNC, DAVE).voucher_balance),
			unit(80)
		);
		// DAVE Borrow 31 BNC should cause InsufficientLiquidity
		assert_noop!(
			LendMarket::borrow(RuntimeOrigin::signed(DAVE), BNC, unit(31)),
			Error::<Test>::InsufficientLiquidity
		);
		assert_ok!(LendMarket::borrow(RuntimeOrigin::signed(DAVE), BNC, unit(30)));

		// Assert ALICE Supply BNC 20
		assert_eq!(
			ExchangeRate::<Test>::get(BNC)
				.saturating_mul_int(AccountDeposits::<Test>::get(BNC, ALICE).voucher_balance),
			unit(20)
		);
		// ALICE Redeem 20 BNC should be succeeded
		// Also means that transfer lend token succeed
		assert_ok!(LendMarket::redeem_allowed(BNC, &ALICE, unit(20) * 50,));
	})
}

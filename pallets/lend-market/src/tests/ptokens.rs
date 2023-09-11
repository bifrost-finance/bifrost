use crate::{
    mock::{
        market_mock, new_test_ext, Loans, RuntimeOrigin, Test, ALICE, DAVE, HKO, KSM, PHKO, PKSM,
        PUSDT, SDOT, USDT,
    },
    tests::unit,
    Error,
};
use frame_support::{
    assert_err, assert_noop, assert_ok,
    traits::tokens::fungibles::{Inspect, Transfer},
};
use sp_runtime::{FixedPointNumber, TokenError};

#[test]
fn trait_inspect_methods_works() {
    new_test_ext().execute_with(|| {
        // No Deposits can't not withdraw
        assert_err!(
            Loans::can_withdraw(PHKO, &DAVE, 100).into_result(),
            TokenError::NoFunds
        );
        assert_eq!(Loans::total_issuance(PHKO), 0);
        assert_eq!(Loans::total_issuance(PKSM), 0);

        let minimum_balance = Loans::minimum_balance(PHKO);
        assert_eq!(minimum_balance, 0);

        assert_eq!(Loans::balance(PHKO, &DAVE), 0);

        // DAVE Deposit 100 HKO
        assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), HKO, unit(100)));
        assert_eq!(Loans::balance(PHKO, &DAVE), unit(100) * 50);

        assert_eq!(Loans::reducible_balance(PHKO, &DAVE, true), unit(100) * 50);
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(DAVE),
            HKO,
            true
        ));
        // Borrow 25 HKO will reduce 25 HKO liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), HKO, unit(25)));

        assert_eq!(
            Loans::exchange_rate(HKO)
                .saturating_mul_int(Loans::account_deposits(HKO, DAVE).voucher_balance),
            unit(100)
        );

        // DAVE Deposit 100 HKO, Borrow 25 HKO
        // Liquidity HKO 25
        // Formula: ptokens = liquidity / price(1) / collateral(0.5) / exchange_rate(0.02)
        assert_eq!(
            Loans::reducible_balance(PHKO, &DAVE, true),
            unit(25) * 2 * 50
        );

        // Multi-asset case, additional deposit USDT
        // DAVE Deposit 100 HKO, 50 USDT, Borrow 25 HKO
        // Liquidity HKO = 25, USDT = 25
        // ptokens = dollar(25 + 25) / 1 / 0.5 / 0.02 = dollar(50) * 100
        assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), USDT, unit(50)));
        assert_eq!(Loans::balance(PUSDT, &DAVE), unit(50) * 50);
        assert_eq!(
            Loans::reducible_balance(PUSDT, &DAVE, true),
            unit(25) * 2 * 50
        );
        // enable USDT collateral
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(DAVE),
            USDT,
            true
        ));
        assert_eq!(
            Loans::reducible_balance(PHKO, &DAVE, true),
            unit(25 + 25) * 2 * 50
        );

        assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), HKO, unit(50)));
        assert_eq!(Loans::reducible_balance(PHKO, &DAVE, true), 0);

        assert_eq!(Loans::total_issuance(PHKO), unit(100) * 50);
        assert_ok!(Loans::can_deposit(PHKO, &DAVE, 100, true).into_result());
        assert_ok!(Loans::can_withdraw(PHKO, &DAVE, 1000).into_result());
    })
}

#[test]
fn ptoken_unique_works() {
    new_test_ext().execute_with(|| {
        // ptoken_id already exists in `UnderlyingAssetId`
        assert_noop!(
            Loans::add_market(RuntimeOrigin::root(), SDOT, market_mock(PHKO)),
            Error::<Test>::InvalidPtokenId
        );

        // ptoken_id cannot as the same as the asset id in `Markets`
        assert_noop!(
            Loans::add_market(RuntimeOrigin::root(), SDOT, market_mock(KSM)),
            Error::<Test>::InvalidPtokenId
        );
    })
}

#[test]
fn transfer_ptoken_works() {
    new_test_ext().execute_with(|| {
        // DAVE Deposit 100 HKO
        assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), HKO, unit(100)));

        // DAVE HKO collateral: deposit = 100
        // HKO: cash - deposit = 1000 - 100 = 900
        assert_eq!(
            Loans::exchange_rate(HKO)
                .saturating_mul_int(Loans::account_deposits(HKO, DAVE).voucher_balance),
            unit(100)
        );

        // ALICE HKO collateral: deposit = 0
        assert_eq!(
            Loans::exchange_rate(HKO)
                .saturating_mul_int(Loans::account_deposits(HKO, ALICE).voucher_balance),
            unit(0)
        );

        // Transfer ptokens from DAVE to ALICE
        Loans::transfer(PHKO, &DAVE, &ALICE, unit(50) * 50, true).unwrap();
        // Loans::transfer_ptokens(RuntimeOrigin::signed(DAVE), ALICE, HKO, dollar(50) * 50).unwrap();

        // DAVE HKO collateral: deposit = 50
        assert_eq!(
            Loans::exchange_rate(HKO)
                .saturating_mul_int(Loans::account_deposits(HKO, DAVE).voucher_balance),
            unit(50)
        );
        // DAVE Redeem 51 HKO should cause InsufficientDeposit
        assert_noop!(
            Loans::redeem_allowed(HKO, &DAVE, unit(51) * 50),
            Error::<Test>::InsufficientDeposit
        );

        // ALICE HKO collateral: deposit = 50
        assert_eq!(
            Loans::exchange_rate(HKO)
                .saturating_mul_int(Loans::account_deposits(HKO, ALICE).voucher_balance),
            unit(50)
        );
        // ALICE Redeem 50 HKO should be succeeded
        assert_ok!(Loans::redeem_allowed(HKO, &ALICE, unit(50) * 50));
    })
}

#[test]
fn transfer_ptokens_under_collateral_works() {
    new_test_ext().execute_with(|| {
        // DAVE Deposit 100 HKO
        assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), HKO, unit(100)));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(DAVE),
            HKO,
            true
        ));

        // Borrow 50 HKO will reduce 50 HKO liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), HKO, unit(50)));
        // Repay 40 HKO
        assert_ok!(Loans::repay_borrow(
            RuntimeOrigin::signed(DAVE),
            HKO,
            unit(40)
        ));

        // Transfer 20 ptokens from DAVE to ALICE
        Loans::transfer(PHKO, &DAVE, &ALICE, unit(20) * 50, true).unwrap();

        // DAVE Deposit HKO = 100 - 20 = 80
        // DAVE Borrow HKO = 0 + 50 - 40 = 10
        // DAVE liquidity HKO = 80 * 0.5 - 10 = 30
        assert_eq!(
            Loans::exchange_rate(HKO)
                .saturating_mul_int(Loans::account_deposits(HKO, DAVE).voucher_balance),
            unit(80)
        );
        // DAVE Borrow 31 HKO should cause InsufficientLiquidity
        assert_noop!(
            Loans::borrow(RuntimeOrigin::signed(DAVE), HKO, unit(31)),
            Error::<Test>::InsufficientLiquidity
        );
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), HKO, unit(30)));

        // Assert ALICE Supply HKO 20
        assert_eq!(
            Loans::exchange_rate(HKO)
                .saturating_mul_int(Loans::account_deposits(HKO, ALICE).voucher_balance),
            unit(20)
        );
        // ALICE Redeem 20 HKO should be succeeded
        // Also means that transfer ptoken succeed
        assert_ok!(Loans::redeem_allowed(HKO, &ALICE, unit(20) * 50,));
    })
}

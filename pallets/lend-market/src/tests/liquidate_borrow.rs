use crate::{
    mock::{
        new_test_ext, Assets, Loans, MockPriceFeeder, RuntimeOrigin, Test, ALICE, BOB, DOT, KSM,
        USDT,
    },
    tests::unit,
    Error, MarketState,
};
use frame_support::{assert_err, assert_noop, assert_ok};
use primitives::{tokens::CDOT_6_13, Rate, DOT_U};
use sp_runtime::FixedPointNumber;

#[test]
fn liquidate_borrow_allowed_works() {
    new_test_ext().execute_with(|| {
        // Borrower should have a positive shortfall
        let dot_market = Loans::market(DOT).unwrap();
        assert_noop!(
            Loans::liquidate_borrow_allowed(&ALICE, DOT, 100, &dot_market),
            Error::<Test>::InsufficientShortfall
        );
        initial_setup();
        alice_borrows_100_ksm();
        // Adjust KSM price to make shortfall
        MockPriceFeeder::set_price(KSM, 2.into());
        let ksm_market = Loans::market(KSM).unwrap();
        // Here the balance sheet of Alice is:
        // Collateral   Loans
        // CDOT $200    KSM $400
        // USDT $200
        assert_noop!(
            Loans::liquidate_borrow_allowed(&ALICE, KSM, unit(51), &ksm_market),
            Error::<Test>::TooMuchRepay
        );
        assert_ok!(Loans::liquidate_borrow_allowed(
            &ALICE,
            KSM,
            unit(50),
            &ksm_market
        ));
    })
}

#[test]
fn lf_liquidate_borrow_fails_due_to_lf_collateral() {
    new_test_ext().execute_with(|| {
        Loans::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![CDOT_6_13]).unwrap();

        assert_err!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(ALICE), BOB, DOT, unit(100), CDOT_6_13),
            Error::<Test>::CollateralReserved
        );
        assert_err!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(ALICE), BOB, DOT, unit(100), DOT_U),
            Error::<Test>::CollateralReserved
        );
    })
}

#[test]
fn lf_liquidate_borrow_allowed_works() {
    new_test_ext().execute_with(|| {
        Loans::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![CDOT_6_13]).unwrap();
        // Bob deposits $200 DOT
        Loans::mint(RuntimeOrigin::signed(BOB), DOT, unit(200)).unwrap();
        Loans::mint(RuntimeOrigin::signed(ALICE), USDT, unit(200)).unwrap();
        Loans::mint(RuntimeOrigin::signed(ALICE), CDOT_6_13, unit(200)).unwrap();
        Loans::collateral_asset(RuntimeOrigin::signed(ALICE), USDT, true).unwrap();
        Loans::collateral_asset(RuntimeOrigin::signed(ALICE), CDOT_6_13, true).unwrap();

        // ALICE
        // Collateral                 Borrowed
        // USDT  $100                 DOT $200
        // CDOT  $100
        Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(200)).unwrap();

        // CDOT's price is highly relative to DOT's price in real runtime. Thus we must update them
        // at the same time.
        MockPriceFeeder::set_price(DOT, 2.into());
        MockPriceFeeder::set_price(CDOT_6_13, 2.into());
        // ALICE
        // Collateral                 Borrowed
        // USDT  $100                 DOT $400
        // CDOT  $200

        let dot_market = Loans::market(DOT).unwrap();
        // The max repay amount = (400 - 200) * 50% = $100
        assert_err!(
            Loans::liquidate_borrow_allowed(&ALICE, DOT, unit(51), &dot_market),
            Error::<Test>::TooMuchRepay
        );
        assert_ok!(Loans::liquidate_borrow_allowed(
            &ALICE,
            DOT,
            unit(50),
            &dot_market
        ));

        // Remove CDOT from lf collateral
        Loans::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![]).unwrap();
        // The max repay amount = 400 * 50 = $200
        assert_ok!(Loans::liquidate_borrow_allowed(
            &ALICE,
            DOT,
            unit(100),
            &dot_market
        ));
    })
}

#[test]
fn deposit_of_borrower_must_be_collateral() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        // Adjust KSM price to make shortfall
        MockPriceFeeder::set_price(KSM, 2.into());
        let market = Loans::market(KSM).unwrap();
        assert_noop!(
            Loans::liquidate_borrow_allowed(&ALICE, KSM, unit(51), &market),
            Error::<Test>::TooMuchRepay
        );
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, 10, DOT),
            Error::<Test>::DepositsAreNotCollateral
        );
    })
}

#[test]
fn collateral_value_must_be_greater_than_liquidation_value() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        MockPriceFeeder::set_price(KSM, Rate::from_float(2000.0));
        Loans::mutate_market(KSM, |market| {
            market.liquidate_incentive = Rate::from_float(200.0);
            market.clone()
        })
        .unwrap();
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(50), USDT),
            Error::<Test>::InsufficientCollateral
        );
    })
}

#[test]
fn full_workflow_works_as_expected() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        // adjust KSM price to make ALICE generate shortfall
        MockPriceFeeder::set_price(KSM, 2.into());
        // BOB repay the KSM borrow balance and get DOT from ALICE
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            KSM,
            unit(50),
            USDT
        ));

        // KSM price = 2
        // incentive = repay KSM value * 1.1 = (50 * 2) * 1.1 = 110
        // Alice USDT: cash - deposit = 1000 - 200 = 800
        // Alice USDT collateral: deposit - incentive = 200 - 110 = 90
        // Alice KSM: cash + borrow = 1000 + 100 = 1100
        // Alice KSM borrow balance: origin borrow balance - liquidate amount = 100 - 50 = 50
        // Bob KSM: cash - deposit - repay = 1000 - 200 - 50 = 750
        // Bob DOT collateral: incentive = 110-(110/1.1*0.03)=107
        assert_eq!(Assets::balance(USDT, &ALICE), unit(800),);
        assert_eq!(
            Loans::exchange_rate(USDT)
                .saturating_mul_int(Loans::account_deposits(USDT, ALICE).voucher_balance),
            unit(90),
        );
        assert_eq!(Assets::balance(KSM, &ALICE), unit(1100),);
        assert_eq!(Loans::account_borrows(KSM, ALICE).principal, unit(50));
        assert_eq!(Assets::balance(KSM, &BOB), unit(750));
        assert_eq!(
            Loans::exchange_rate(USDT)
                .saturating_mul_int(Loans::account_deposits(USDT, BOB).voucher_balance),
            unit(107),
        );
        // 3 dollar reserved in our incentive reward account
        let incentive_reward_account = Loans::incentive_reward_account_id().unwrap();
        println!(
            "incentive reserve account:{:?}",
            incentive_reward_account.clone()
        );
        assert_eq!(
            Loans::exchange_rate(USDT).saturating_mul_int(
                Loans::account_deposits(USDT, incentive_reward_account.clone()).voucher_balance
            ),
            unit(3),
        );
        assert_eq!(Assets::balance(USDT, &ALICE), unit(800),);
        // reduce 2 dollar from incentive reserve to alice account
        assert_ok!(Loans::reduce_incentive_reserves(
            RuntimeOrigin::root(),
            ALICE,
            USDT,
            unit(2),
        ));
        // still 1 dollar left in reserve account
        assert_eq!(
            Loans::exchange_rate(USDT).saturating_mul_int(
                Loans::account_deposits(USDT, incentive_reward_account).voucher_balance
            ),
            unit(1),
        );
        // 2 dollar transfer to alice
        assert_eq!(Assets::balance(USDT, &ALICE), unit(800) + unit(2),);
    })
}

#[test]
fn liquidator_cannot_take_inactive_market_currency() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        // Adjust KSM price to make shortfall
        MockPriceFeeder::set_price(KSM, 2.into());
        assert_ok!(Loans::mutate_market(DOT, |stored_market| {
            stored_market.state = MarketState::Supervision;
            stored_market.clone()
        }));
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(50), DOT),
            Error::<Test>::MarketNotActivated
        );
    })
}

#[test]
fn liquidator_can_not_repay_more_than_the_close_factor_pct_multiplier() {
    new_test_ext().execute_with(|| {
        initial_setup();
        alice_borrows_100_ksm();
        MockPriceFeeder::set_price(KSM, 20.into());
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(51), DOT),
            Error::<Test>::TooMuchRepay
        );
    })
}

#[test]
fn liquidator_must_not_be_borrower() {
    new_test_ext().execute_with(|| {
        initial_setup();
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(ALICE), ALICE, KSM, 0, DOT),
            Error::<Test>::LiquidatorIsBorrower
        );
    })
}

fn alice_borrows_100_ksm() {
    assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(100)));
}

fn initial_setup() {
    // Bob deposits 200 KSM
    assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), KSM, unit(200)));
    // Alice deposits 200 DOT as collateral
    assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), USDT, unit(200)));
    assert_ok!(Loans::collateral_asset(
        RuntimeOrigin::signed(ALICE),
        USDT,
        true
    ));
    assert_ok!(Loans::mint(
        RuntimeOrigin::signed(ALICE),
        CDOT_6_13,
        unit(200)
    ));
    assert_ok!(Loans::collateral_asset(
        RuntimeOrigin::signed(ALICE),
        CDOT_6_13,
        true
    ));
    assert_ok!(Loans::update_liquidation_free_collateral(
        RuntimeOrigin::root(),
        vec![CDOT_6_13]
    ));
}

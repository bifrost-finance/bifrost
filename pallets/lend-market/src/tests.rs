// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod edge_cases;
mod interest_rate;
mod liquidate_borrow;
mod market;
mod ptokens;

use frame_support::{assert_err, assert_noop, assert_ok};

use primitives::tokens::CDOT_6_13;
use sp_runtime::{
    traits::{CheckedDiv, One, Saturating},
    FixedU128, Permill,
};

use crate::mock::*;

#[test]
fn init_minting_ok() {
    new_test_ext().execute_with(|| {
        assert_eq!(Assets::balance(KSM, ALICE), unit(1000));
        assert_eq!(Assets::balance(DOT, ALICE), unit(1000));
        assert_eq!(Assets::balance(USDT, ALICE), unit(1000));
        assert_eq!(Assets::balance(KSM, BOB), unit(1000));
        assert_eq!(Assets::balance(DOT, BOB), unit(1000));
    });
}

#[test]
fn init_markets_ok() {
    new_test_ext().execute_with(|| {
        assert_eq!(Loans::market(KSM).unwrap().state, MarketState::Active);
        assert_eq!(Loans::market(DOT).unwrap().state, MarketState::Active);
        assert_eq!(Loans::market(USDT).unwrap().state, MarketState::Active);
        assert_eq!(BorrowIndex::<Test>::get(HKO), Rate::one());
        assert_eq!(BorrowIndex::<Test>::get(KSM), Rate::one());
        assert_eq!(BorrowIndex::<Test>::get(DOT), Rate::one());
        assert_eq!(BorrowIndex::<Test>::get(USDT), Rate::one());

        assert_eq!(
            ExchangeRate::<Test>::get(KSM),
            Rate::saturating_from_rational(2, 100)
        );
        assert_eq!(
            ExchangeRate::<Test>::get(DOT),
            Rate::saturating_from_rational(2, 100)
        );
        assert_eq!(
            ExchangeRate::<Test>::get(USDT),
            Rate::saturating_from_rational(2, 100)
        );
    });
}

#[test]
fn loans_native_token_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(<Test as Config>::Assets::balance(HKO, &DAVE), unit(1000));
        assert_eq!(Loans::market(HKO).unwrap().state, MarketState::Active);
        assert_eq!(BorrowIndex::<Test>::get(HKO), Rate::one());
        assert_eq!(
            ExchangeRate::<Test>::get(HKO),
            Rate::saturating_from_rational(2, 100)
        );
        assert_ok!(Loans::mint(RuntimeOrigin::signed(DAVE), HKO, unit(1000)));

        // Redeem 1001 HKO should cause InsufficientDeposit
        assert_noop!(
            Loans::redeem_allowed(HKO, &DAVE, unit(50050)),
            Error::<Test>::InsufficientDeposit
        );
        // Redeem 1000 HKO is ok
        assert_ok!(Loans::redeem_allowed(HKO, &DAVE, unit(50000),));

        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(DAVE),
            HKO,
            true
        ));

        // Borrow 500 HKO will reduce 500 HKO liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(DAVE), HKO, unit(500)));
        // Repay 400 HKO
        assert_ok!(Loans::repay_borrow(
            RuntimeOrigin::signed(DAVE),
            HKO,
            unit(400)
        ));

        // HKO collateral: deposit = 1000
        // HKO borrow balance: borrow - repay = 500 - 400 = 100
        // HKO: cash - deposit + borrow - repay = 1000 - 1000 + 500 - 400 = 100
        assert_eq!(
            Loans::exchange_rate(HKO)
                .saturating_mul_int(Loans::account_deposits(HKO, DAVE).voucher_balance),
            unit(1000)
        );
        let borrow_snapshot = Loans::account_borrows(HKO, DAVE);
        assert_eq!(borrow_snapshot.principal, unit(100));
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(HKO));
        assert_eq!(<Test as Config>::Assets::balance(HKO, &DAVE), unit(100),);
    })
}

#[test]
fn mint_works() {
    new_test_ext().execute_with(|| {
        // Deposit 100 DOT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100)));

        // DOT collateral: deposit = 100
        // DOT: cash - deposit = 1000 - 100 = 900
        assert_eq!(
            Loans::exchange_rate(DOT)
                .saturating_mul_int(Loans::account_deposits(DOT, ALICE).voucher_balance),
            unit(100)
        );
        assert_eq!(<Test as Config>::Assets::balance(DOT, &ALICE), unit(900),);
        assert_eq!(
            <Test as Config>::Assets::balance(DOT, &Loans::account_id()),
            unit(100),
        );
    })
}

#[test]
fn mint_must_return_err_when_overflows_occur() {
    new_test_ext().execute_with(|| {
        Loans::force_update_market(
            RuntimeOrigin::root(),
            DOT,
            Market {
                supply_cap: u128::MAX,
                ..ACTIVE_MARKET_MOCK
            },
        )
        .unwrap();
        // MAX_DEPOSIT = u128::MAX * exchangeRate
        const OVERFLOW_DEPOSIT: u128 = u128::MAX / 50 + 1;

        // Verify token balance first
        assert_noop!(
            Loans::mint(RuntimeOrigin::signed(CHARLIE), DOT, OVERFLOW_DEPOSIT),
            ArithmeticError::Underflow
        );

        // Deposit OVERFLOW_DEPOSIT DOT for CHARLIE
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            DOT.into(),
            CHARLIE,
            OVERFLOW_DEPOSIT
        ));

        // Amount is too large, OVERFLOW_DEPOSIT / 0.0X == Overflow
        // Underflow is used here redeem could also be 0
        assert_noop!(
            Loans::mint(RuntimeOrigin::signed(CHARLIE), DOT, OVERFLOW_DEPOSIT),
            ArithmeticError::Underflow
        );

        // Exchange rate must ge greater than zero
        // ExchangeRate::<Test>::insert(DOT, Rate::zero());
        // assert_noop!(
        //     Loans::mint(RuntimeOrigin::signed(CHARLIE), DOT, 100),
        //     ArithmeticError::Underflow
        // );
    })
}

#[test]
fn redeem_allowed_works() {
    new_test_ext().execute_with(|| {
        // Prepare: Bob Deposit 200 DOT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, 200));

        // Deposit 200 KSM as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, 200));
        // Redeem 201 KSM should cause InsufficientDeposit
        assert_noop!(
            Loans::redeem_allowed(KSM, &ALICE, 10050),
            Error::<Test>::InsufficientDeposit
        );
        // Redeem 1 DOT should cause InsufficientDeposit
        assert_noop!(
            Loans::redeem_allowed(DOT, &ALICE, 50),
            Error::<Test>::InsufficientDeposit
        );
        // Redeem 200 KSM is ok
        assert_ok!(Loans::redeem_allowed(KSM, &ALICE, 10000));

        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            KSM,
            true
        ));
        // Borrow 50 DOT will reduce 100 KSM liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, 50));
        // Redeem 101 KSM should cause InsufficientLiquidity
        assert_noop!(
            Loans::redeem_allowed(KSM, &ALICE, 5050),
            Error::<Test>::InsufficientLiquidity
        );
        // Redeem 100 KSM is ok
        assert_ok!(Loans::redeem_allowed(KSM, &ALICE, 5000));
    })
}

#[test]
fn lf_redeem_allowed_works() {
    new_test_ext().execute_with(|| {
        // Set CDOT as lf collateral
        Loans::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![CDOT_6_13]).unwrap();
        Loans::mint(RuntimeOrigin::signed(ALICE), CDOT_6_13, unit(200)).unwrap();

        Loans::mint(RuntimeOrigin::signed(DAVE), USDT, unit(200)).unwrap();
        Loans::mint(RuntimeOrigin::signed(DAVE), DOT, unit(200)).unwrap();
        // Lend $200 CDOT
        Loans::collateral_asset(RuntimeOrigin::signed(ALICE), CDOT_6_13, true).unwrap();

        Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(50)).unwrap();

        // (200 - 100) * 50% >= 50
        assert_ok!(Loans::redeem_allowed(CDOT_6_13, &ALICE, unit(100)));

        // Set KSM as collateral, and borrow USDT
        Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(200)).unwrap();
        Loans::collateral_asset(RuntimeOrigin::signed(ALICE), KSM, true).unwrap();
        Loans::borrow(RuntimeOrigin::signed(ALICE), USDT, unit(100)).unwrap();

        assert_err!(
            Loans::redeem_allowed(KSM, &ALICE, unit(100)),
            Error::<Test>::InsufficientLiquidity
        );
        // But it'll success when redeem cdot
        assert_ok!(Loans::redeem_allowed(CDOT_6_13, &ALICE, unit(100)));

        // Remove CDOT from lf collateral
        Loans::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![]).unwrap();
        // Then it can be redeemed
        assert_ok!(Loans::redeem_allowed(KSM, &ALICE, unit(100)));
    })
}

#[test]
fn redeem_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(20)));

        // DOT collateral: deposit - redeem = 100 - 20 = 80
        // DOT: cash - deposit + redeem = 1000 - 100 + 20 = 920
        assert_eq!(
            Loans::exchange_rate(DOT)
                .saturating_mul_int(Loans::account_deposits(DOT, ALICE).voucher_balance),
            unit(80)
        );
        assert_eq!(<Test as Config>::Assets::balance(DOT, &ALICE), unit(920),);
    })
}

#[test]
fn redeem_fails() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(0)),
            Error::<Test>::InvalidAmount
        );
    })
}

#[test]
fn redeem_fails_when_insufficient_liquidity() {
    new_test_ext().execute_with(|| {
        // Prepare: Bob Deposit 200 DOT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, 200));

        // Deposit 200 KSM as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, 200));

        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            KSM,
            true
        ));
        // Borrow 50 DOT will reduce 100 KSM liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, 50));

        assert_noop!(
            Loans::redeem(RuntimeOrigin::signed(BOB), DOT, 151),
            Error::<Test>::InsufficientCash
        );
    })
}

#[test]
fn redeem_fails_when_would_use_reserved_balanace() {
    new_test_ext().execute_with(|| {
        // Prepare: Bob Deposit 200 DOT
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, 200));

        // Deposit 200 KSM as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, 200));

        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            KSM,
            true
        ));
        // Borrow 50 DOT will reduce 100 KSM liquidity for collateral_factor is 50%
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, 50));
        assert_ok!(Loans::add_reserves(RuntimeOrigin::root(), ALICE, DOT, 50));

        assert_noop!(
            Loans::redeem(RuntimeOrigin::signed(BOB), DOT, 151),
            Error::<Test>::InsufficientCash
        );
    })
}

#[test]
fn redeem_must_return_err_when_overflows_occur() {
    new_test_ext().execute_with(|| {
        // Amount is too large, max_value / 0.0X == Overflow
        // Underflow is used here redeem could also be 0
        assert_noop!(
            Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, u128::MAX),
            ArithmeticError::Underflow,
        );

        // Exchange rate must ge greater than zero
        // ExchangeRate::<Test>::insert(DOT, Rate::zero());
        // assert_noop!(
        //     Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, 100),
        //     ArithmeticError::Underflow
        // );
    })
}

#[test]
fn redeem_all_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        assert_ok!(Loans::redeem_all(RuntimeOrigin::signed(ALICE), DOT));

        // DOT: cash - deposit + redeem = 1000 - 100 + 100 = 1000
        // DOT collateral: deposit - redeem = 100 - 100 = 0
        assert_eq!(
            Loans::exchange_rate(DOT)
                .saturating_mul_int(Loans::account_deposits(DOT, ALICE).voucher_balance),
            0,
        );
        assert_eq!(<Test as Config>::Assets::balance(DOT, &ALICE), unit(1000),);
        assert!(!AccountDeposits::<Test>::contains_key(DOT, &ALICE))
    })
}

#[test]
fn borrow_allowed_works() {
    new_test_ext().execute_with(|| {
        // Deposit 200 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, 200));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, 200));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            KSM,
            true
        ));
        // Borrow 101 DOT should cause InsufficientLiquidity
        assert_noop!(
            Loans::borrow_allowed(DOT, &ALICE, 101),
            Error::<Test>::InsufficientLiquidity
        );
        // Borrow 100 DOT is ok
        assert_ok!(Loans::borrow_allowed(DOT, &ALICE, 100));

        // Set borrow limit to 10
        assert_ok!(Loans::force_update_market(
            RuntimeOrigin::root(),
            DOT,
            Market {
                borrow_cap: 10,
                ..ACTIVE_MARKET_MOCK
            },
        ));
        // Borrow 10 DOT is ok
        assert_ok!(Loans::borrow_allowed(DOT, &ALICE, 10));
        // Borrow 11 DOT should cause BorrowLimitExceeded
        assert_noop!(
            Loans::borrow_allowed(DOT, &ALICE, 11),
            Error::<Test>::BorrowCapacityExceeded
        );
    })
}

#[test]
fn update_liquidation_free_collateral_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::update_liquidation_free_collateral(
            RuntimeOrigin::root(),
            vec![CDOT_6_13]
        ));
        assert_eq!(Loans::liquidation_free_collaterals(), vec![CDOT_6_13]);
    })
}

#[test]
fn get_account_liquidity_works() {
    new_test_ext().execute_with(|| {
        Loans::mint(RuntimeOrigin::signed(ALICE), CDOT_6_13, unit(200)).unwrap();
        Loans::collateral_asset(RuntimeOrigin::signed(ALICE), CDOT_6_13, true).unwrap();

        let (liquidity, _, lf_liquidity, _) = Loans::get_account_liquidity(&ALICE).unwrap();

        assert_eq!(liquidity, FixedU128::from_inner(unit(100)));
        assert_eq!(lf_liquidity, FixedU128::from_inner(unit(100)));
    })
}

#[test]
fn get_account_liquidation_threshold_liquidity_works() {
    new_test_ext().execute_with(|| {
        Loans::mint(RuntimeOrigin::signed(BOB), DOT, unit(200)).unwrap();
        Loans::mint(RuntimeOrigin::signed(BOB), KSM, unit(200)).unwrap();

        Loans::mint(RuntimeOrigin::signed(ALICE), CDOT_6_13, unit(200)).unwrap();
        Loans::collateral_asset(RuntimeOrigin::signed(ALICE), CDOT_6_13, true).unwrap();

        Loans::mint(RuntimeOrigin::signed(ALICE), USDT, unit(200)).unwrap();
        Loans::collateral_asset(RuntimeOrigin::signed(ALICE), USDT, true).unwrap();

        Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(100)).unwrap();
        Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(100)).unwrap();

        let (liquidity, _, lf_liquidity, _) =
            Loans::get_account_liquidation_threshold_liquidity(&ALICE).unwrap();

        assert_eq!(liquidity, FixedU128::from_inner(unit(20)));
        assert_eq!(lf_liquidity, FixedU128::from_inner(unit(10)));

        MockPriceFeeder::set_price(KSM, 2.into());
        let (liquidity, shortfall, lf_liquidity, _) =
            Loans::get_account_liquidation_threshold_liquidity(&ALICE).unwrap();

        assert_eq!(liquidity, FixedU128::from_inner(unit(0)));
        assert_eq!(shortfall, FixedU128::from_inner(unit(80)));
        assert_eq!(lf_liquidity, FixedU128::from_inner(unit(10)));
    })
}

#[test]
fn lf_borrow_allowed_works() {
    new_test_ext().execute_with(|| {
        Loans::mint(RuntimeOrigin::signed(ALICE), CDOT_6_13, unit(200)).unwrap();
        Loans::mint(RuntimeOrigin::signed(DAVE), USDT, unit(200)).unwrap();
        Loans::mint(RuntimeOrigin::signed(DAVE), DOT, unit(200)).unwrap();
        // Lend $200 CDOT
        Loans::collateral_asset(RuntimeOrigin::signed(ALICE), CDOT_6_13, true).unwrap();

        assert_eq!(
            Loans::get_asset_value(DOT, unit(100)).unwrap(),
            Loans::get_asset_value(USDT, unit(100)).unwrap()
        );

        assert_noop!(
            Loans::borrow_allowed(USDT, &ALICE, unit(100)),
            Error::<Test>::InsufficientLiquidity
        );
        assert_ok!(Loans::borrow_allowed(DOT, &ALICE, unit(100)));
    })
}

#[test]
fn borrow_works() {
    new_test_ext().execute_with(|| {
        // Deposit 200 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(200)));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            DOT,
            true
        ));
        // Borrow 100 DOT
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(100)));

        // DOT collateral: deposit = 200
        // DOT borrow balance: borrow = 100
        // DOT: cash - deposit + borrow = 1000 - 200 + 100 = 900
        assert_eq!(
            Loans::exchange_rate(DOT)
                .saturating_mul_int(Loans::account_deposits(DOT, ALICE).voucher_balance),
            unit(200)
        );
        let borrow_snapshot = Loans::account_borrows(DOT, ALICE);
        assert_eq!(borrow_snapshot.principal, unit(100));
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(DOT));
        assert_eq!(<Test as Config>::Assets::balance(DOT, &ALICE), unit(900),);
    })
}

#[test]
fn lf_borrow_works() {
    new_test_ext().execute_with(|| {
        // Deposit 200 DOT as collateral
        Loans::mint(RuntimeOrigin::signed(ALICE), CDOT_6_13, unit(200)).unwrap();
        Loans::mint(RuntimeOrigin::signed(DAVE), DOT, unit(200)).unwrap();
        Loans::collateral_asset(RuntimeOrigin::signed(ALICE), CDOT_6_13, true).unwrap();

        // Borrow 100 DOT
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(100)));

        // CDOT collateral: deposit = 200
        // DOT borrow balance: borrow = 100
        // DOT: cash - deposit + borrow = 1000 + 100 = 1100
        assert_eq!(
            Loans::exchange_rate(CDOT_6_13)
                .saturating_mul_int(Loans::account_deposits(CDOT_6_13, ALICE).voucher_balance),
            unit(200)
        );
        let borrow_snapshot = Loans::account_borrows(DOT, ALICE);
        assert_eq!(borrow_snapshot.principal, unit(100));
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(DOT));
        assert_eq!(<Test as Config>::Assets::balance(DOT, &ALICE), unit(1100),);
    })
}

#[test]
fn repay_borrow_works() {
    new_test_ext().execute_with(|| {
        // Deposit 200 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(200)));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            DOT,
            true
        ));
        // Borrow 100 DOT
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        // Repay 30 DOT
        assert_ok!(Loans::repay_borrow(
            RuntimeOrigin::signed(ALICE),
            DOT,
            unit(30)
        ));

        // DOT collateral: deposit = 200
        // DOT borrow balance: borrow - repay = 100 - 30 = 70
        // DOT: cash - deposit + borrow - repay = 1000 - 200 + 100 - 30 = 870
        assert_eq!(
            Loans::exchange_rate(DOT)
                .saturating_mul_int(Loans::account_deposits(DOT, ALICE).voucher_balance),
            unit(200)
        );
        let borrow_snapshot = Loans::account_borrows(DOT, ALICE);
        assert_eq!(borrow_snapshot.principal, unit(70));
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(DOT));
        assert_eq!(<Test as Config>::Assets::balance(DOT, &ALICE), unit(870),);
    })
}

#[test]
fn repay_borrow_all_works() {
    new_test_ext().execute_with(|| {
        // Bob deposits 200 KSM
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), KSM, unit(200)));
        // Alice deposit 200 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(200)));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            DOT,
            true
        ));
        // Alice borrow 50 KSM
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(50)));

        // Alice repay all borrow balance
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), KSM));

        // DOT: cash - deposit +  = 1000 - 200 = 800
        // DOT collateral: deposit = 200
        // KSM: cash + borrow - repay = 1000 + 50 - 50 = 1000
        // KSM borrow balance: borrow - repay = 50 - 50 = 0
        assert_eq!(<Test as Config>::Assets::balance(DOT, &ALICE), unit(800),);
        assert_eq!(
            Loans::exchange_rate(DOT)
                .saturating_mul_int(Loans::account_deposits(DOT, ALICE).voucher_balance),
            unit(200)
        );
        let borrow_snapshot = Loans::account_borrows(KSM, ALICE);
        assert_eq!(borrow_snapshot.principal, 0);
        assert_eq!(borrow_snapshot.borrow_index, Loans::borrow_index(KSM));
    })
}

#[test]
fn collateral_asset_works() {
    new_test_ext().execute_with(|| {
        // No collateral assets
        assert_noop!(
            Loans::collateral_asset(RuntimeOrigin::signed(ALICE), DOT, true),
            Error::<Test>::NoDeposit
        );
        // Deposit 200 DOT as collateral
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, 200));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            DOT,
            true
        ));
        assert_eq!(Loans::account_deposits(DOT, ALICE).is_collateral, true);
        assert_noop!(
            Loans::collateral_asset(RuntimeOrigin::signed(ALICE), DOT, true),
            Error::<Test>::DuplicateOperation
        );
        // Borrow 100 DOT base on the collateral of 200 DOT
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, 100));
        assert_noop!(
            Loans::collateral_asset(RuntimeOrigin::signed(ALICE), DOT, false),
            Error::<Test>::InsufficientLiquidity
        );
        // Repay all the borrows
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            DOT,
            false
        ));
        assert_eq!(Loans::account_deposits(DOT, ALICE).is_collateral, false);
        assert_noop!(
            Loans::collateral_asset(RuntimeOrigin::signed(ALICE), DOT, false),
            Error::<Test>::DuplicateOperation
        );
    })
}

#[test]
fn total_collateral_value_works() {
    new_test_ext().execute_with(|| {
        // Mock the price for DOT = 1, KSM = 1
        let collateral_factor = Rate::saturating_from_rational(50, 100);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(200)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), USDT, unit(300)));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            DOT,
            true
        ));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            KSM,
            true
        ));
        assert_eq!(
            Loans::total_collateral_value(&ALICE).unwrap(),
            (collateral_factor.saturating_mul(FixedU128::from_inner(unit(100) + unit(200))))
        );
    })
}

#[test]
fn add_reserves_works() {
    new_test_ext().execute_with(|| {
        // Add 100 DOT reserves
        assert_ok!(Loans::add_reserves(
            RuntimeOrigin::root(),
            ALICE,
            DOT,
            unit(100)
        ));

        assert_eq!(Loans::total_reserves(DOT), unit(100));
        assert_eq!(
            <Test as Config>::Assets::balance(DOT, &Loans::account_id()),
            unit(100),
        );
        assert_eq!(<Test as Config>::Assets::balance(DOT, &ALICE), unit(900),);
    })
}

#[test]
fn reduce_reserves_works() {
    new_test_ext().execute_with(|| {
        // Add 100 DOT reserves
        assert_ok!(Loans::add_reserves(
            RuntimeOrigin::root(),
            ALICE,
            DOT,
            unit(100)
        ));

        // Reduce 20 DOT reserves
        assert_ok!(Loans::reduce_reserves(
            RuntimeOrigin::root(),
            ALICE,
            DOT,
            unit(20)
        ));

        assert_eq!(Loans::total_reserves(DOT), unit(80));
        assert_eq!(
            <Test as Config>::Assets::balance(DOT, &Loans::account_id()),
            unit(80),
        );
        assert_eq!(<Test as Config>::Assets::balance(DOT, &ALICE), unit(920),);
    })
}

#[test]
fn reduce_reserve_reduce_amount_must_be_less_than_total_reserves() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::add_reserves(
            RuntimeOrigin::root(),
            ALICE,
            DOT,
            unit(100)
        ));
        assert_noop!(
            Loans::reduce_reserves(RuntimeOrigin::root(), ALICE, DOT, unit(200)),
            Error::<Test>::InsufficientReserves
        );
    })
}

#[test]
fn ratio_and_rate_works() {
    new_test_ext().execute_with(|| {
        // Permill to FixedU128
        let ratio = Permill::from_percent(50);
        let rate: FixedU128 = ratio.into();
        assert_eq!(rate, FixedU128::saturating_from_rational(1, 2));

        // Permill  (one = 1_000_000)
        let permill = Permill::from_percent(50);
        assert_eq!(permill.mul_floor(100_u128), 50_u128);

        // FixedU128 (one = 1_000_000_000_000_000_000_000)
        let value1 = FixedU128::saturating_from_integer(100);
        let value2 = FixedU128::saturating_from_integer(10);
        assert_eq!(
            value1.checked_mul(&value2),
            Some(FixedU128::saturating_from_integer(1000))
        );
        assert_eq!(
            value1.checked_div(&value2),
            Some(FixedU128::saturating_from_integer(10))
        );
        assert_eq!(
            value1.saturating_mul(permill.into()),
            FixedU128::saturating_from_integer(50)
        );

        let value1 = FixedU128::saturating_from_rational(9, 10);
        let value2 = 10_u128;
        let value3 = FixedU128::saturating_from_integer(10_u128);
        assert_eq!(
            value1.reciprocal(),
            Some(FixedU128::saturating_from_rational(10, 9))
        );
        // u128 div FixedU128
        assert_eq!(
            FixedU128::saturating_from_integer(value2).checked_div(&value1),
            Some(FixedU128::saturating_from_rational(100, 9))
        );

        // FixedU128 div u128
        assert_eq!(
            value1.reciprocal().and_then(|r| r.checked_mul_int(value2)),
            Some(11)
        );
        assert_eq!(
            FixedU128::from_inner(17_777_777_777_777_777_777).checked_div_int(value2),
            Some(1)
        );
        // FixedU128 mul u128
        assert_eq!(
            FixedU128::from_inner(17_777_777_777_777_777_777).checked_mul_int(value2),
            Some(177)
        );

        // reciprocal
        assert_eq!(
            FixedU128::saturating_from_integer(value2).checked_div(&value1),
            Some(FixedU128::saturating_from_rational(100, 9))
        );
        assert_eq!(
            value1
                .reciprocal()
                .and_then(|r| r.checked_mul(&FixedU128::saturating_from_integer(value2))),
            Some(FixedU128::from_inner(11_111_111_111_111_111_110))
        );
        assert_eq!(
            FixedU128::saturating_from_integer(value2)
                .checked_mul(&value3)
                .and_then(|v| v.checked_div(&value1)),
            Some(FixedU128::saturating_from_rational(1000, 9))
        );
        assert_eq!(
            FixedU128::saturating_from_integer(value2)
                .checked_div(&value1)
                .and_then(|v| v.checked_mul(&value3)),
            Some(FixedU128::from_inner(111_111_111_111_111_111_110))
        );

        // FixedU128 div Permill
        let value1 = Permill::from_percent(30);
        let value2 = Permill::from_percent(40);
        let value3 = FixedU128::saturating_from_integer(10);
        assert_eq!(
            value3.checked_div(&value1.into()),
            Some(FixedU128::saturating_from_rational(100, 3)) // 10/0.3
        );

        // u128 div Permill
        assert_eq!(value1.saturating_reciprocal_mul_floor(5_u128), 16); // (1/0.3) * 5 = 16.66666666..
        assert_eq!(value1.saturating_reciprocal_mul_floor(5_u128), 16); // (1/0.3) * 5 = 16.66666666..
        assert_eq!(value2.saturating_reciprocal_mul_floor(5_u128), 12); // (1/0.4) * 5 = 12.5

        // Permill * u128
        let value1 = Permill::from_percent(34);
        let value2 = Permill::from_percent(36);
        let value3 = Permill::from_percent(30);
        let value4 = Permill::from_percent(20);
        assert_eq!(value1 * 10_u64, 3); // 0.34 * 10
        assert_eq!(value2 * 10_u64, 4); // 0.36 * 10
        assert_eq!(value3 * 5_u64, 1); // 0.3 * 5
        assert_eq!(value4 * 8_u64, 2); // 0.2 * 8
        assert_eq!(value4.mul_floor(8_u64), 1); // 0.2 mul_floor 8
    })
}

#[test]
fn update_exchange_rate_works() {
    new_test_ext().execute_with(|| {
        // Initialize value of exchange rate is 0.02
        assert_eq!(
            Loans::exchange_rate(DOT),
            Rate::saturating_from_rational(2, 100)
        );

        // total_supply = 0
        TotalSupply::<Test>::insert(DOT, 0);
        // assert_ok!(Loans::update_exchange_rate(DOT));
        assert_eq!(
            Loans::exchange_rate_stored(DOT).unwrap(),
            Rate::saturating_from_rational(2, 100)
        );

        // exchange_rate = total_cash + total_borrows - total_reverse / total_supply
        // total_cash = 10, total_supply = 500
        // exchange_rate = 10 + 5 - 1 / 500
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10)));
        TotalBorrows::<Test>::insert(DOT, unit(5));
        TotalReserves::<Test>::insert(DOT, unit(1));
        // assert_ok!(Loans::update_exchange_rate(DOT));
        assert_eq!(
            Loans::exchange_rate_stored(DOT).unwrap(),
            Rate::saturating_from_rational(14, 500)
        );
    })
}

#[test]
fn current_borrow_balance_works() {
    new_test_ext().execute_with(|| {
        // snapshot.principal = 0
        AccountBorrows::<Test>::insert(
            DOT,
            ALICE,
            BorrowSnapshot {
                principal: 0,
                borrow_index: Rate::one(),
            },
        );
        assert_eq!(Loans::current_borrow_balance(&ALICE, DOT).unwrap(), 0);

        // snapshot.borrow_index = 0
        AccountBorrows::<Test>::insert(
            DOT,
            ALICE,
            BorrowSnapshot {
                principal: 100,
                borrow_index: Rate::zero(),
            },
        );
        assert_eq!(Loans::current_borrow_balance(&ALICE, DOT).unwrap(), 0);

        // borrow_index = 1.2, snapshot.borrow_index = 1, snapshot.principal = 100
        BorrowIndex::<Test>::insert(DOT, Rate::saturating_from_rational(12, 10));
        AccountBorrows::<Test>::insert(
            DOT,
            ALICE,
            BorrowSnapshot {
                principal: 100,
                borrow_index: Rate::one(),
            },
        );
        assert_eq!(Loans::current_borrow_balance(&ALICE, DOT).unwrap(), 120);
    })
}

#[test]
fn calc_collateral_amount_works() {
    let exchange_rate = Rate::saturating_from_rational(3, 10);
    assert_eq!(
        Loans::calc_collateral_amount(1000, exchange_rate).unwrap(),
        3333
    );
    assert_eq!(
        Loans::calc_collateral_amount(u128::MAX, exchange_rate),
        Err(DispatchError::Arithmetic(ArithmeticError::Underflow))
    );

    // relative test: prevent_the_exchange_rate_attack
    let exchange_rate = Rate::saturating_from_rational(30000, 1);
    assert_eq!(
        Loans::calc_collateral_amount(10000, exchange_rate).unwrap(),
        0
    );
}

#[test]
fn get_price_works() {
    new_test_ext().execute_with(|| {
        MockPriceFeeder::set_price(DOT, 0.into());
        assert_noop!(Loans::get_price(DOT), Error::<Test>::PriceIsZero);

        MockPriceFeeder::set_price(DOT, 2.into());
        assert_eq!(
            Loans::get_price(DOT).unwrap(),
            Price::saturating_from_integer(2)
        );
    })
}

#[test]
fn ensure_enough_cash_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            KSM.into(),
            Loans::account_id(),
            unit(1000)
        ));
        assert_ok!(Loans::ensure_enough_cash(KSM, unit(1000)));
        TotalReserves::<Test>::insert(KSM, unit(10));
        assert_noop!(
            Loans::ensure_enough_cash(KSM, unit(1000)),
            Error::<Test>::InsufficientCash,
        );
        assert_ok!(Loans::ensure_enough_cash(KSM, unit(990)));
    })
}

#[test]
fn ensure_valid_exchange_rate_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Loans::ensure_valid_exchange_rate(FixedU128::saturating_from_rational(1, 100)),
            Error::<Test>::InvalidExchangeRate
        );
        assert_ok!(Loans::ensure_valid_exchange_rate(
            FixedU128::saturating_from_rational(2, 100)
        ));
        assert_ok!(Loans::ensure_valid_exchange_rate(
            FixedU128::saturating_from_rational(3, 100)
        ));
        assert_ok!(Loans::ensure_valid_exchange_rate(
            FixedU128::saturating_from_rational(99, 100)
        ));
        assert_noop!(
            Loans::ensure_valid_exchange_rate(Rate::one()),
            Error::<Test>::InvalidExchangeRate,
        );
        assert_noop!(
            Loans::ensure_valid_exchange_rate(Rate::saturating_from_rational(101, 100)),
            Error::<Test>::InvalidExchangeRate,
        );
    })
}

#[test]
fn withdraw_missing_reward_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(<Test as Config>::Assets::balance(HKO, &DAVE), unit(1000));

        assert_ok!(Loans::add_reward(RuntimeOrigin::signed(DAVE), unit(100)));

        assert_ok!(Loans::withdraw_missing_reward(
            RuntimeOrigin::root(),
            ALICE,
            unit(40),
        ));

        assert_eq!(<Test as Config>::Assets::balance(HKO, &DAVE), unit(900));

        assert_eq!(<Test as Config>::Assets::balance(HKO, &ALICE), unit(40));

        assert_eq!(
            <Test as Config>::Assets::balance(HKO, &Loans::reward_account_id().unwrap()),
            unit(60)
        );
    })
}

#[test]
fn update_market_reward_speed_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(Loans::reward_supply_speed(DOT), 0);
        assert_eq!(Loans::reward_borrow_speed(DOT), 0);

        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1)),
            Some(unit(2)),
        ));
        assert_eq!(Loans::reward_supply_speed(DOT), unit(1));
        assert_eq!(Loans::reward_borrow_speed(DOT), unit(2));

        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(2)),
            Some(0),
        ));
        assert_eq!(Loans::reward_supply_speed(DOT), unit(2));
        assert_eq!(Loans::reward_borrow_speed(DOT), unit(0));

        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(0),
            Some(0)
        ));
        assert_eq!(Loans::reward_supply_speed(DOT), unit(0));
        assert_eq!(Loans::reward_borrow_speed(DOT), unit(0));
    })
}

#[test]
fn reward_calculation_one_palyer_in_multi_markets_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(100)));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            DOT,
            true
        ));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            KSM,
            true
        ));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(10)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(10)));

        _run_to_block(10);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1)),
            Some(unit(2)),
        ));

        // check status
        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 10);
        assert_eq!(Loans::reward_supplier_index(DOT, ALICE), 0);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 10);
        assert_eq!(Loans::reward_borrower_index(DOT, ALICE), 0);
        // DOT supply:100   DOT supply reward: 0
        // DOT borrow:10    DOT borrow reward: 0
        // KSM supply:100   KSM supply reward: 0
        // KSM borrow:10    KSM borrow reward: 0
        assert_eq!(Loans::reward_accrued(ALICE), 0);

        _run_to_block(20);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            KSM,
            Some(unit(1)),
            Some(unit(1)),
        ));

        // check status
        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 20);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 10);
        // DOT supply:200   DOT supply reward: 10
        // DOT borrow:10    DOT borrow reward: 0
        // KSM supply:100   KSM supply reward: 0
        // KSM borrow:10    KSM borrow reward: 0
        // borrow reward not accrued
        assert_eq!(Loans::reward_accrued(ALICE), unit(10));

        _run_to_block(30);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(0),
            Some(0)
        ));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(10)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(100)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(10)));

        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 30);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 30);
        // DOT supply:100   DOT supply reward: 20
        // DOT borrow:20    DOT borrow reward: 40
        // KSM supply:200   KSM supply reward: 10
        // KSM borrow:20    KSM borrow reward: 10
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(80)), true);

        _run_to_block(40);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            KSM,
            Some(0),
            Some(0)
        ));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(10)));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), KSM, unit(100)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(10)));

        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 40);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 40);
        // DOT supply:200   DOT supply reward: 20
        // DOT borrow:30    DOT borrow reward: 40
        // KSM supply:100   KSM supply reward: 20
        // KSM borrow:30    KSM borrow reward: 20
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(100)), true,);

        _run_to_block(50);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1)),
            Some(unit(1)),
        ));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(100)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(10)));

        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 50);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 50);
        // DOT supply:100   DOT supply reward: 20
        // DOT borrow:0     DOT borrow reward: 40
        // KSM supply:200   KSM supply reward: 20
        // KSM borrow:40    KSM borrow reward: 20
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(100)), true,);

        _run_to_block(60);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            KSM,
            Some(unit(1)),
            Some(unit(1)),
        ));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), KSM, unit(100)));
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), KSM));

        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 60);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 50);
        // DOT supply:200   DOT supply reward: 30
        // DOT borrow:0     DOT borrow reward: 40
        // KSM supply:100   KSM supply reward: 20
        // KSM borrow:0     KSM borrow reward: 20
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(110)), true,);

        _run_to_block(70);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(0),
            Some(0)
        ));
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            KSM,
            Some(0),
            Some(0)
        ));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(100)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), KSM, unit(100)));

        let supply_state = Loans::reward_supply_state(DOT);
        assert_eq!(supply_state.block, 70);
        let borrow_state = Loans::reward_borrow_state(DOT);
        assert_eq!(borrow_state.block, 70);
        // DOT supply:500   DOT supply reward: 40
        // DOT borrow:0     DOT borrow reward: 40
        // KSM supply:600   KSM supply reward: 30
        // KSM borrow:0     KSM borrow reward: 20
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(130)), true);

        _run_to_block(80);
        assert_ok!(Loans::add_reward(RuntimeOrigin::signed(DAVE), unit(200)));
        assert_ok!(Loans::claim_reward(RuntimeOrigin::signed(ALICE)));
        assert_eq!(<Test as Config>::Assets::balance(HKO, &DAVE), unit(800));
        assert_eq!(
            almost_equal(<Test as Config>::Assets::balance(HKO, &ALICE), unit(130)),
            true
        );
        assert_eq!(
            almost_equal(
                <Test as Config>::Assets::balance(HKO, &Loans::reward_account_id().unwrap()),
                unit(70)
            ),
            true
        );
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1)),
            Some(0),
        ));

        // DOT supply:500   DOT supply reward: 50
        // DOT borrow:0     DOT borrow reward: 40
        // KSM supply:600   KSM supply reward: 30
        // KSM borrow:0     KSM borrow reward: 20
        _run_to_block(90);
        assert_ok!(Loans::claim_reward(RuntimeOrigin::signed(ALICE)));
        assert_eq!(
            almost_equal(<Test as Config>::Assets::balance(HKO, &ALICE), unit(140)),
            true
        );
    })
}

#[test]
fn reward_calculation_multi_player_in_one_market_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, unit(10)));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            DOT,
            true
        ));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(BOB),
            DOT,
            true
        ));

        _run_to_block(10);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1)),
            Some(unit(1)),
        ));
        // Alice supply:10     supply reward: 0
        // Alice borrow:0       borrow reward: 0
        // BOB supply:10       supply reward: 0
        // BOB borrow:0         borrow reward: 0
        assert_eq!(Loans::reward_accrued(ALICE), 0);
        assert_eq!(Loans::reward_accrued(BOB), 0);

        _run_to_block(20);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(70)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, unit(10)));
        // Alice supply:80     supply reward: 5
        // Alice borrow:0       borrow reward: 0
        // BOB supply:20       supply reward: 5
        // BOB borrow:10        borrow reward: 0
        assert_eq!(Loans::reward_accrued(ALICE), unit(5));
        assert_eq!(Loans::reward_accrued(BOB), unit(5));

        _run_to_block(30);
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(70)));
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(BOB), DOT, unit(10)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(1)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(BOB), DOT, unit(1)));
        // Alice supply:10     supply reward: 13
        // Alice borrow:1      borrow reward: 0
        // BOB supply:10       supply reward: 7
        // BOB borrow:1        borrow reward: 0
        assert_eq!(Loans::reward_accrued(ALICE), unit(13));
        assert_eq!(Loans::reward_accrued(BOB), unit(7));

        _run_to_block(40);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10)));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), DOT, unit(10)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), DOT, unit(1)));
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(BOB), DOT));
        // Alice supply:20     supply reward: 18
        // Alice borrow:2      borrow reward: 5
        // BOB supply:20       supply reward: 12
        // BOB borrow:0        borrow reward: 5
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(23)), true);
        assert_eq!(almost_equal(Loans::reward_accrued(BOB), unit(17)), true);

        _run_to_block(50);
        assert_ok!(Loans::redeem(RuntimeOrigin::signed(ALICE), DOT, unit(10)));
        assert_ok!(Loans::redeem_all(RuntimeOrigin::signed(BOB), DOT));
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(BOB), DOT));
        // Alice supply:10     supply reward: 23
        // Alice borrow:0      borrow reward: 15
        // BOB supply:0       supply reward: 17
        // BOB borrow:0        borrow reward: 5
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(38)), true);
        assert_eq!(almost_equal(Loans::reward_accrued(BOB), unit(22)), true);

        _run_to_block(60);
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(10)));
        assert_ok!(Loans::redeem_all(RuntimeOrigin::signed(BOB), DOT));
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(ALICE), DOT));
        assert_ok!(Loans::repay_borrow_all(RuntimeOrigin::signed(BOB), DOT));
        // Alice supply:10     supply reward: 33
        // Alice borrow:0      borrow reward: 15
        // BOB supply:0       supply reward: 17
        // BOB borrow:0        borrow reward: 5
        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(48)), true);
        assert_eq!(almost_equal(Loans::reward_accrued(BOB), unit(22)), true);

        _run_to_block(70);
        assert_ok!(Loans::add_reward(RuntimeOrigin::signed(DAVE), unit(200)));
        assert_ok!(Loans::claim_reward_for_market(
            RuntimeOrigin::signed(ALICE),
            DOT
        ));
        assert_ok!(Loans::claim_reward_for_market(
            RuntimeOrigin::signed(BOB),
            DOT
        ));
        assert_eq!(<Test as Config>::Assets::balance(HKO, &DAVE), unit(800));
        assert_eq!(
            almost_equal(<Test as Config>::Assets::balance(HKO, &ALICE), unit(58)),
            true
        );
        assert_eq!(
            almost_equal(<Test as Config>::Assets::balance(HKO, &BOB), unit(22)),
            true
        );
        assert_eq!(
            almost_equal(
                <Test as Config>::Assets::balance(HKO, &Loans::reward_account_id().unwrap()),
                unit(120)
            ),
            true
        );
    })
}

#[test]
fn reward_calculation_after_liquidate_borrow_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Loans::mint(RuntimeOrigin::signed(ALICE), DOT, unit(200)));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(ALICE),
            DOT,
            true
        ));
        assert_ok!(Loans::mint(RuntimeOrigin::signed(BOB), KSM, unit(500)));
        assert_ok!(Loans::collateral_asset(
            RuntimeOrigin::signed(BOB),
            KSM,
            true
        ));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(ALICE), KSM, unit(50)));
        assert_ok!(Loans::borrow(RuntimeOrigin::signed(BOB), KSM, unit(75)));

        _run_to_block(10);
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            DOT,
            Some(unit(1)),
            Some(unit(1)),
        ));
        assert_ok!(Loans::update_market_reward_speed(
            RuntimeOrigin::root(),
            KSM,
            Some(unit(1)),
            Some(unit(1)),
        ));

        _run_to_block(20);
        assert_ok!(Loans::update_reward_supply_index(DOT));
        assert_ok!(Loans::distribute_supplier_reward(DOT, &ALICE));
        assert_ok!(Loans::distribute_supplier_reward(DOT, &BOB));
        assert_ok!(Loans::update_reward_borrow_index(DOT));
        assert_ok!(Loans::distribute_borrower_reward(DOT, &ALICE));
        assert_ok!(Loans::distribute_borrower_reward(DOT, &BOB));

        assert_ok!(Loans::update_reward_supply_index(KSM));
        assert_ok!(Loans::distribute_supplier_reward(KSM, &ALICE));
        assert_ok!(Loans::distribute_supplier_reward(KSM, &BOB));
        assert_ok!(Loans::update_reward_borrow_index(KSM));
        assert_ok!(Loans::distribute_borrower_reward(KSM, &ALICE));
        assert_ok!(Loans::distribute_borrower_reward(KSM, &BOB));

        assert_eq!(almost_equal(Loans::reward_accrued(ALICE), unit(14)), true);
        assert_eq!(almost_equal(Loans::reward_accrued(BOB), unit(16)), true);

        MockPriceFeeder::set_price(KSM, 2.into());
        // since we set liquidate_threshold more than collateral_factor,with KSM price as 2 alice not shortfall yet.
        // so we can not liquidate_borrow here
        assert_noop!(
            Loans::liquidate_borrow(RuntimeOrigin::signed(BOB), ALICE, KSM, unit(25), DOT),
            Error::<Test>::InsufficientShortfall
        );
        // then we change KSM price = 3 to make alice shortfall
        // incentive = repay KSM value * 1.1 = (25 * 3) * 1.1 = 82.5
        // Alice DOT Deposit: 200 - 82.5 = 117.5
        // Alice KSM Borrow: 50 - 25 = 25
        // Bob DOT Deposit: 75 + 75*0.07 = 80.25
        // Bob KSM Deposit: 500
        // Bob KSM Borrow: 75
        // incentive_reward_account DOT Deposit: 75*0.03 = 2.25
        MockPriceFeeder::set_price(KSM, 3.into());
        assert_ok!(Loans::liquidate_borrow(
            RuntimeOrigin::signed(BOB),
            ALICE,
            KSM,
            unit(25),
            DOT
        ));

        _run_to_block(30);
        assert_ok!(Loans::update_reward_supply_index(DOT));
        assert_ok!(Loans::distribute_supplier_reward(DOT, &ALICE));
        assert_ok!(Loans::distribute_supplier_reward(DOT, &BOB));
        assert_ok!(Loans::update_reward_borrow_index(DOT));
        assert_ok!(Loans::distribute_borrower_reward(DOT, &ALICE));
        assert_ok!(Loans::distribute_borrower_reward(DOT, &BOB));

        assert_ok!(Loans::update_reward_supply_index(KSM));
        assert_ok!(Loans::distribute_supplier_reward(KSM, &ALICE));
        assert_ok!(Loans::distribute_supplier_reward(KSM, &BOB));
        assert_ok!(Loans::update_reward_borrow_index(KSM));
        assert_ok!(Loans::distribute_borrower_reward(KSM, &ALICE));
        assert_ok!(Loans::distribute_borrower_reward(KSM, &BOB));
        assert_ok!(Loans::distribute_supplier_reward(
            DOT,
            &Loans::incentive_reward_account_id().unwrap(),
        ));

        assert_eq!(
            almost_equal(Loans::reward_accrued(ALICE), milli_unit(22375)),
            true
        );
        assert_eq!(
            almost_equal(Loans::reward_accrued(BOB), micro_unit(37512500)),
            true
        );
        assert_eq!(
            almost_equal(
                Loans::reward_accrued(Loans::incentive_reward_account_id().unwrap()),
                micro_unit(112500),
            ),
            true,
        );
    })
}

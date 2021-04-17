// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

//! Tests for the module.

#![cfg(test)]

use crate::mock::*;
// use balances::Call as BalancesCall;
use frame_support::{
    assert_ok,
    traits::WithdrawReasons,
    weights::{GetDispatchInfo, Pays, PostDispatchInfo},
};
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::testing::TestXt;
use std::convert::TryInto;
use zenlink_protocol::{AssetId, DEXOperations};

// some common variables
pub const ALICE: u128 = 1;
pub const BOB: u128 = 2;
pub const CHARLIE: u128 = 3;
pub const DICK: u128 = 4;
pub const CURRENCY_ID_0: CurrencyId = CurrencyId::Token(TokenSymbol::ASG);
pub const CURRENCY_ID_1: CurrencyId = CurrencyId::Token(TokenSymbol::aUSD);
pub const CURRENCY_ID_2: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
pub const CURRENCY_ID_3: CurrencyId = CurrencyId::Token(TokenSymbol::vDOT);
pub const CURRENCY_ID_4: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);

fn basic_setup() {
    // Deposit some money in Alice, Bob and Charlie's accounts.
    // Alice
    assert_ok!(Currencies::deposit(CURRENCY_ID_0, &ALICE, 50));
    assert_ok!(Currencies::deposit(CURRENCY_ID_1, &ALICE, 200));
    assert_ok!(Currencies::deposit(CURRENCY_ID_2, &ALICE, 300));
    assert_ok!(Currencies::deposit(CURRENCY_ID_3, &ALICE, 400));
    assert_ok!(Currencies::deposit(CURRENCY_ID_4, &ALICE, 500));

    // Bob
    assert_ok!(Currencies::deposit(CURRENCY_ID_0, &BOB, 100));
    assert_ok!(Currencies::deposit(CURRENCY_ID_1, &BOB, 200));
    assert_ok!(Currencies::deposit(CURRENCY_ID_2, &BOB, 60));
    assert_ok!(Currencies::deposit(CURRENCY_ID_3, &BOB, 80));
    assert_ok!(Currencies::deposit(CURRENCY_ID_4, &BOB, 50));

    // Charlie
    assert_ok!(Currencies::deposit(CURRENCY_ID_0, &CHARLIE, 200));
    assert_ok!(Currencies::deposit(CURRENCY_ID_1, &CHARLIE, 20));
    assert_ok!(Currencies::deposit(CURRENCY_ID_2, &CHARLIE, 30));
    assert_ok!(Currencies::deposit(CURRENCY_ID_3, &CHARLIE, 40));
    assert_ok!(Currencies::deposit(CURRENCY_ID_4, &CHARLIE, 50));

    // Dick
    assert_ok!(Currencies::deposit(CURRENCY_ID_0, &DICK, 100000));
    assert_ok!(Currencies::deposit(CURRENCY_ID_1, &DICK, 100000));
    assert_ok!(Currencies::deposit(CURRENCY_ID_2, &DICK, 100000));
    assert_ok!(Currencies::deposit(CURRENCY_ID_3, &DICK, 100000));
    assert_ok!(Currencies::deposit(CURRENCY_ID_4, &DICK, 100000));

    // create DEX pair
    let asset_0_currency_id: AssetId = AssetId::from(CURRENCY_ID_0);
    let asset_1_currency_id: AssetId = AssetId::from(CURRENCY_ID_1);
    let asset_2_currency_id: AssetId = AssetId::from(CURRENCY_ID_2);

    assert_ok!(Zenlink::inner_create_pair_zenlink(
        &asset_0_currency_id,
        &asset_1_currency_id
    )); // asset 0 and 1
    let pool_0_1_account =
        Zenlink::get_pair_from_asset_id_zenlink(&asset_0_currency_id, &asset_1_currency_id)
            .unwrap()
            .account;

    // need to deposit some money into the pools
    // pool 0 1
    assert_ok!(Zenlink::inner_add_liquidity_zenlink(
        &DICK,
        &asset_0_currency_id,
        &asset_1_currency_id,
        1000,
        1000,
        1,
        1
    ));

    assert_ok!(Zenlink::inner_create_pair_zenlink(
        &asset_0_currency_id,
        &asset_2_currency_id
    )); // asset 0 and 2

    let pool_0_2_account =
        Zenlink::get_pair_from_asset_id_zenlink(&asset_0_currency_id, &asset_2_currency_id)
            .unwrap()
            .account;

    // pool 0 2
    assert_ok!(Zenlink::inner_add_liquidity_zenlink(
        &DICK,
        &asset_0_currency_id,
        &asset_2_currency_id,
        1000,
        1000,
        1,
        1
    ));
}

#[test]
fn set_user_fee_charge_order_should_work() {
    new_test_ext().execute_with(|| {
        let origin_signed_alice = Origin::signed(ALICE);
        let mut asset_order_list_vec: Vec<CurrencyId> = vec![
            CURRENCY_ID_4,
            CURRENCY_ID_3,
            CURRENCY_ID_2,
            CURRENCY_ID_1,
            CURRENCY_ID_0,
        ];
        assert_ok!(ChargeTransactionFee::set_user_fee_charge_order(
            origin_signed_alice.clone(),
            Some(asset_order_list_vec.clone())
        ));

        asset_order_list_vec.insert(0, CURRENCY_ID_0);
        assert_eq!(
            crate::UserFeeChargeOrderList::<Test>::get(ALICE),
            asset_order_list_vec
        );

        assert_ok!(ChargeTransactionFee::set_user_fee_charge_order(
            origin_signed_alice,
            None
        ));

        assert_eq!(
            crate::UserFeeChargeOrderList::<Test>::get(ALICE).is_empty(),
            true
        );
    });
}

#[test]
fn inner_get_user_fee_charge_order_list_should_work() {
    new_test_ext().execute_with(|| {
        let origin_signed_alice = Origin::signed(ALICE);
        let mut asset_order_list_vec: Vec<CurrencyId> = vec![
            CURRENCY_ID_4,
            CURRENCY_ID_3,
            CURRENCY_ID_2,
            CURRENCY_ID_1,
            CURRENCY_ID_0,
        ];

        let mut default_order_list: Vec<CurrencyId> = Vec::new();
        default_order_list.push(CurrencyId::from(0 as u8));
        default_order_list.push(CurrencyId::from(1 as u8));
        default_order_list.push(CurrencyId::from(2 as u8));
        default_order_list.push(CurrencyId::from(3 as u8));
        default_order_list.push(CurrencyId::from(6 as u8));
        default_order_list.push(CurrencyId::from(7 as u8));

        assert_eq!(
            ChargeTransactionFee::inner_get_user_fee_charge_order_list(&ALICE),
            default_order_list
        );

        let _ = ChargeTransactionFee::set_user_fee_charge_order(
            origin_signed_alice.clone(),
            Some(asset_order_list_vec.clone()),
        );

        asset_order_list_vec.insert(0, CURRENCY_ID_0);

        assert_eq!(
            ChargeTransactionFee::inner_get_user_fee_charge_order_list(&ALICE),
            asset_order_list_vec
        );
    });
}

#[test]
fn ensure_can_charge_fee_should_work() {
    new_test_ext().execute_with(|| {
        basic_setup();
        let origin_signed_bob = Origin::signed(BOB);
        let asset_order_list_vec: Vec<CurrencyId> = vec![
            CURRENCY_ID_4,
            CURRENCY_ID_3,
            CURRENCY_ID_2,
            CURRENCY_ID_1,
            CURRENCY_ID_0,
        ];
        let mut default_order_list: Vec<CurrencyId> = Vec::new();
        for i in 0..12 {
            default_order_list.push(CurrencyId::from(i as u8));
        }

        // Set bob order as [4,3,2,1]. Alice and Charlie will use the default order of [0..11]]
        let _ = ChargeTransactionFee::set_user_fee_charge_order(
            origin_signed_bob.clone(),
            Some(asset_order_list_vec.clone()),
        );

        let native_asset_id: AssetId = AssetId::from(CURRENCY_ID_0);
        let asset_id: AssetId = AssetId::from(CURRENCY_ID_1);

        let path = vec![asset_id, native_asset_id];

        let pool_0_1_account = Zenlink::get_pair_from_asset_id_zenlink(&asset_id, &native_asset_id)
            .unwrap()
            .account;

        println!("pool_0_1_account: {:?}", pool_0_1_account);

        let pool_0_1_price =
            <Test as crate::Config>::ZenlinkDEX::get_amount_in_by_path_zenlink(100, &path);
        let pool_0_1_account = <Test as crate::Config>::ZenlinkDEX::get_pair_from_asset_id_zenlink(
            &native_asset_id,
            &asset_id,
        )
        .unwrap()
        .account;
        println!("pool_0_1_price: {:?}", pool_0_1_price);
        println!(
            "crrency 0 total balance of pool_0_1: {:?}",
            Currencies::total_balance(CURRENCY_ID_0, &pool_0_1_account)
        );
        println!(
            "crrency 1 total balance of pool_0_1: {:?}",
            Currencies::total_balance(CURRENCY_ID_1, &pool_0_1_account)
        );

        ChargeTransactionFee::ensure_can_charge_fee(
            &ALICE,
            100,
            WithdrawReasons::TRANSACTION_PAYMENT,
        );

        // Alice should be deducted 100 from Asset 1 since Asset 0 doesn't have enough balance. asset1 : 200-100=100
        // asset0: 50+100 = 150
        assert_eq!(Currencies::total_balance(CURRENCY_ID_0, &ALICE), 150);
        assert_eq!(Currencies::total_balance(CURRENCY_ID_1, &ALICE), 88);

        assert_eq!(<Test as crate::Config>::Currency::free_balance(&ALICE), 150);

        // Bob
        ChargeTransactionFee::ensure_can_charge_fee(
            &BOB,
            100,
            WithdrawReasons::TRANSACTION_PAYMENT,
        );
        assert_eq!(<Test as crate::Config>::Currency::free_balance(&BOB), 200); // exitential deposit check should be more than 0 balance kept for charging 100 fee
        assert_eq!(Currencies::total_balance(CURRENCY_ID_1, &BOB), 60);
    });
}

#[test]
fn withdraw_fee_should_work() {
    new_test_ext().execute_with(|| {
        basic_setup();

        // prepare call variable
        let asset_order_list_vec: Vec<CurrencyId> = vec![
            CURRENCY_ID_0,
            CURRENCY_ID_1,
            CURRENCY_ID_2,
            CURRENCY_ID_3,
            CURRENCY_ID_4,
        ];
        let call = Call::ChargeTransactionFee(crate::Call::set_user_fee_charge_order(Some(
            asset_order_list_vec,
        )));

        // prepare info variable
        let extra = ();
        let xt = TestXt::new(call.clone(), Some((CHARLIE.try_into().unwrap(), extra)));
        let info = xt.get_dispatch_info();

        // 99 inclusion fee and a tip of 8
        assert_ok!(ChargeTransactionFee::withdraw_fee(
            &CHARLIE, &call, &info, 107, 8
        ));

        assert_eq!(
            <Test as crate::Config>::Currency::free_balance(&CHARLIE),
            93
        );
    });
}

#[test]
fn correct_and_deposit_fee_should_work() {
    new_test_ext().execute_with(|| {
        basic_setup();
        // prepare call variable
        let asset_order_list_vec: Vec<CurrencyId> = vec![
            CURRENCY_ID_0,
            CURRENCY_ID_1,
            CURRENCY_ID_2,
            CURRENCY_ID_3,
            CURRENCY_ID_4,
        ];
        let call = Call::ChargeTransactionFee(crate::Call::set_user_fee_charge_order(Some(
            asset_order_list_vec,
        )));
        // prepare info variable
        let extra = ();
        let xt = TestXt::new(call.clone(), Some((CHARLIE.try_into().unwrap(), extra)));
        let info = xt.get_dispatch_info();

        // prepare post info
        let post_info = PostDispatchInfo {
            actual_weight: Some(20),
            pays_fee: Pays::Yes,
        };

        let corrected_fee = 80;
        let tip = 8;

        let already_withdrawn =
            ChargeTransactionFee::withdraw_fee(&CHARLIE, &call, &info, 107, 8).unwrap();

        assert_eq!(
            <Test as crate::Config>::Currency::free_balance(&CHARLIE),
            93
        );

        assert_ok!(ChargeTransactionFee::correct_and_deposit_fee(
            &CHARLIE,
            &info,
            &post_info,
            corrected_fee,
            tip,
            already_withdrawn
        ));

        assert_eq!(
            <Test as crate::Config>::Currency::free_balance(&CHARLIE),
            120
        );
    });
}

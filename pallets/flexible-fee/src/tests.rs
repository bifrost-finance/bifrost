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

//! Tests for the module.

#![cfg(test)]

use std::convert::TryFrom;

// use balances::Call as BalancesCall;
use frame_support::{
	assert_noop, assert_ok,
	traits::WithdrawReasons,
	weights::{GetDispatchInfo, Pays, PostDispatchInfo},
};
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::{testing::TestXt, AccountId32};
use zenlink_protocol::AssetId;

use crate::{mock::*, BlockNumberFor, FeeDealer};

// some common variables
pub const CHARLIE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const ALICE: AccountId32 = AccountId32::new([2u8; 32]);
pub const DICK: AccountId32 = AccountId32::new([3u8; 32]);
pub const CURRENCY_ID_0: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
pub const CURRENCY_ID_1: CurrencyId = CurrencyId::Stable(TokenSymbol::KUSD);
pub const CURRENCY_ID_2: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
pub const CURRENCY_ID_3: CurrencyId = CurrencyId::VToken(TokenSymbol::DOT);
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
	let asset_0_currency_id: AssetId = AssetId::try_from(CURRENCY_ID_0).unwrap();
	let asset_1_currency_id: AssetId = AssetId::try_from(CURRENCY_ID_1).unwrap();
	let asset_2_currency_id: AssetId = AssetId::try_from(CURRENCY_ID_2).unwrap();

	let mut deadline: BlockNumberFor<Test> = <frame_system::Pallet<Test>>::block_number() +
		<Test as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		Origin::signed(DICK),
		asset_0_currency_id,
		asset_1_currency_id,
		1000,
		1000,
		1,
		1,
		deadline
	));

	// pool 0 2
	deadline = <frame_system::Pallet<Test>>::block_number() +
		<Test as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		Origin::signed(DICK),
		asset_0_currency_id,
		asset_2_currency_id,
		1000,
		1000,
		1,
		1,
		deadline
	));
}

#[test]
fn set_user_fee_charge_order_should_work() {
	new_test_ext().execute_with(|| {
		let origin_signed_alice = Origin::signed(ALICE);
		let mut asset_order_list_vec: Vec<CurrencyId> =
			vec![CURRENCY_ID_4, CURRENCY_ID_3, CURRENCY_ID_2, CURRENCY_ID_1, CURRENCY_ID_0];
		assert_ok!(FlexibleFee::set_user_fee_charge_order(
			origin_signed_alice.clone(),
			Some(asset_order_list_vec.clone())
		));

		asset_order_list_vec.insert(0, CURRENCY_ID_0);
		assert_eq!(crate::UserFeeChargeOrderList::<Test>::get(ALICE), asset_order_list_vec);

		assert_ok!(FlexibleFee::set_user_fee_charge_order(origin_signed_alice, None));

		assert_eq!(crate::UserFeeChargeOrderList::<Test>::get(ALICE).is_empty(), true);
	});
}

#[test]
fn inner_get_user_fee_charge_order_list_should_work() {
	new_test_ext().execute_with(|| {
		let origin_signed_alice = Origin::signed(ALICE);
		let mut asset_order_list_vec: Vec<CurrencyId> =
			vec![CURRENCY_ID_4, CURRENCY_ID_3, CURRENCY_ID_2, CURRENCY_ID_1, CURRENCY_ID_0];

		let mut default_order_list: Vec<CurrencyId> = Vec::new();
		default_order_list.push(CurrencyId::Native(TokenSymbol::ASG));
		default_order_list.push(CurrencyId::Token(TokenSymbol::KSM));

		assert_eq!(FlexibleFee::inner_get_user_fee_charge_order_list(&ALICE), default_order_list);

		let _ = FlexibleFee::set_user_fee_charge_order(
			origin_signed_alice.clone(),
			Some(asset_order_list_vec.clone()),
		);

		asset_order_list_vec.insert(0, CURRENCY_ID_0);

		assert_eq!(FlexibleFee::inner_get_user_fee_charge_order_list(&ALICE), asset_order_list_vec);
	});
}

// Three tests below are ignored due to some bugs of zenlink. Tests will be reopened after the bugs
// fixed.

#[test]
#[ignore]
fn ensure_can_charge_fee_should_work() {
	new_test_ext().execute_with(|| {
		basic_setup();
		let origin_signed_bob = Origin::signed(BOB);
		let asset_order_list_vec: Vec<CurrencyId> =
			vec![CURRENCY_ID_4, CURRENCY_ID_3, CURRENCY_ID_2, CURRENCY_ID_1, CURRENCY_ID_0];
		let mut default_order_list: Vec<CurrencyId> = Vec::new();

		default_order_list.push(CurrencyId::Native(TokenSymbol::ASG));
		default_order_list.push(CurrencyId::Stable(TokenSymbol::KUSD));
		default_order_list.push(CurrencyId::Token(TokenSymbol::DOT));
		default_order_list.push(CurrencyId::Token(TokenSymbol::KSM));
		default_order_list.push(CurrencyId::Token(TokenSymbol::KSM));
		default_order_list.push(CurrencyId::VToken(TokenSymbol::DOT));
		default_order_list.push(CurrencyId::VToken(TokenSymbol::KSM));
		default_order_list.push(CurrencyId::VToken(TokenSymbol::KSM));

		// Set bob order as [4,3,2,1]. Alice and Charlie will use the default order of [0..11]]
		let _ = FlexibleFee::set_user_fee_charge_order(
			origin_signed_bob.clone(),
			Some(asset_order_list_vec.clone()),
		);

		assert_ok!(<Test as crate::Config>::FeeDealer::ensure_can_charge_fee(
			&ALICE,
			100,
			WithdrawReasons::TRANSACTION_PAYMENT,
		));

		// Alice should be deducted 100 from Asset 1 since Asset 0 doesn't have enough balance.
		// asset1 : 200-100=100 asset0: 50+100 = 150
		assert_eq!(Currencies::total_balance(CURRENCY_ID_0, &ALICE), 150);
		assert_eq!(Currencies::total_balance(CURRENCY_ID_1, &ALICE), 88);

		assert_eq!(<Test as crate::Config>::Currency::free_balance(&ALICE), 150);

		// Bob
		assert_ok!(<Test as crate::Config>::FeeDealer::ensure_can_charge_fee(
			&BOB,
			100,
			WithdrawReasons::TRANSACTION_PAYMENT,
		));
		assert_eq!(<Test as crate::Config>::Currency::free_balance(&BOB), 200); // exitential deposit check should be more than 0 balance kept for charging 100 fee
		assert_eq!(Currencies::total_balance(CURRENCY_ID_1, &BOB), 60);
	});
}

#[test]
#[ignore]
fn withdraw_fee_should_work() {
	new_test_ext().execute_with(|| {
		basic_setup();

		// prepare call variable
		let asset_order_list_vec: Vec<CurrencyId> =
			vec![CURRENCY_ID_0, CURRENCY_ID_1, CURRENCY_ID_2, CURRENCY_ID_3, CURRENCY_ID_4];
		let call =
			Call::FlexibleFee(crate::Call::set_user_fee_charge_order(Some(asset_order_list_vec)));

		// prepare info variable
		let extra = ();
		let xt = TestXt::new(call.clone(), Some((0u64, extra)));
		let info = xt.get_dispatch_info();

		// 99 inclusion fee and a tip of 8
		assert_ok!(FlexibleFee::withdraw_fee(&CHARLIE, &call, &info, 107, 8));

		assert_eq!(<Test as crate::Config>::Currency::free_balance(&CHARLIE), 93);
	});
}

#[test]
#[ignore]
fn correct_and_deposit_fee_should_work() {
	new_test_ext().execute_with(|| {
		basic_setup();
		// prepare call variable
		let asset_order_list_vec: Vec<CurrencyId> =
			vec![CURRENCY_ID_0, CURRENCY_ID_1, CURRENCY_ID_2, CURRENCY_ID_3, CURRENCY_ID_4];
		let call =
			Call::FlexibleFee(crate::Call::set_user_fee_charge_order(Some(asset_order_list_vec)));
		// prepare info variable
		let extra = ();
		let xt = TestXt::new(call.clone(), Some((0u64, extra)));
		let info = xt.get_dispatch_info();

		// prepare post info
		let post_info = PostDispatchInfo { actual_weight: Some(20), pays_fee: Pays::Yes };

		let corrected_fee = 80;
		let tip = 8;

		let already_withdrawn = FlexibleFee::withdraw_fee(&CHARLIE, &call, &info, 107, 8).unwrap();

		assert_eq!(<Test as crate::Config>::Currency::free_balance(&CHARLIE), 93);

		assert_ok!(FlexibleFee::correct_and_deposit_fee(
			&CHARLIE,
			&info,
			&post_info,
			corrected_fee,
			tip,
			already_withdrawn
		));

		assert_eq!(<Test as crate::Config>::Currency::free_balance(&CHARLIE), 120);
	});
}

#[test]
// #[ignore = "This should be used with mock config type FeeDealer = FixedCurrencyFeeRate."]
fn ensure_can_charge_fee_v2_should_work() {
	new_test_ext().execute_with(|| {
		// Deposit 500 DOT and none of native token to Alice's account
		assert_ok!(Currencies::deposit(CurrencyId::Token(TokenSymbol::DOT), &ALICE, 500));

		assert_noop!(
			<Test as crate::Config>::FeeDealer::ensure_can_charge_fee(
				&ALICE,
				100,
				WithdrawReasons::TRANSACTION_PAYMENT,
			),
			crate::Error::<Test>::NotEnoughBalance
		);

		assert_ok!(Currencies::deposit(CurrencyId::Token(TokenSymbol::KSM), &ALICE, 1));

		// existential deposit for KSM is 1. So there is no enough KSM for fee charging
		assert_noop!(
			<Test as crate::Config>::FeeDealer::ensure_can_charge_fee(
				&ALICE,
				100,
				WithdrawReasons::TRANSACTION_PAYMENT,
			),
			crate::Error::<Test>::NotEnoughBalance
		);

		assert_ok!(Currencies::deposit(CurrencyId::Token(TokenSymbol::KSM), &ALICE, 1));

		let (sign, amount) = (<Test as crate::Config>::FeeDealer::ensure_can_charge_fee(
			&ALICE,
			100,
			WithdrawReasons::TRANSACTION_PAYMENT,
		))
		.unwrap();

		assert_eq!(sign, true);
		assert_eq!(amount, 1);

		assert_eq!(
			<Test as crate::Config>::MultiCurrency::free_balance(
				CurrencyId::Token(TokenSymbol::KSM),
				&ALICE
			),
			2
		);

		assert_noop!(
			<Test as crate::Config>::FeeDealer::ensure_can_charge_fee(
				&ALICE,
				800,
				WithdrawReasons::TRANSACTION_PAYMENT,
			),
			crate::Error::<Test>::NotEnoughBalance
		);

		// deposit enough native token for fee
		assert_ok!(Currencies::deposit(CurrencyId::Native(TokenSymbol::ASG), &ALICE, 1000));

		let (sign, amount) = (<Test as crate::Config>::FeeDealer::ensure_can_charge_fee(
			&ALICE,
			800,
			WithdrawReasons::TRANSACTION_PAYMENT,
		))
		.unwrap();

		assert_eq!(sign, false);
		assert_eq!(amount, 800);
	});
}

#[test]
// #[ignore = "This should be used with mock config type FeeDealer = FixedCurrencyFeeRate."]
fn withdraw_fee_should_work_v2() {
	new_test_ext().execute_with(|| {
		assert_ok!(Currencies::deposit(CurrencyId::Native(TokenSymbol::ASG), &CHARLIE, 108));
		assert_ok!(Currencies::deposit(CurrencyId::Token(TokenSymbol::KSM), &CHARLIE, 2));

		// prepare call variable
		let asset_order_list_vec: Vec<CurrencyId> =
			vec![CURRENCY_ID_0, CURRENCY_ID_1, CURRENCY_ID_2, CURRENCY_ID_3, CURRENCY_ID_4];
		let call =
			Call::FlexibleFee(crate::Call::set_user_fee_charge_order(Some(asset_order_list_vec)));

		// prepare info variable
		let extra = ();
		let xt = TestXt::new(call.clone(), Some((0u64, extra)));
		let info = xt.get_dispatch_info();

		// In the first time, we can charge transaction fee by native token.
		assert_ok!(FlexibleFee::withdraw_fee(&CHARLIE, &call, &info, 107, 8));
		assert_eq!(<Test as crate::Config>::Currency::free_balance(&CHARLIE), 1);

		// In the second time, we charge transaction fee by KSM.
		// 99 inclusion fee + a tip of 8
		assert_ok!(FlexibleFee::withdraw_fee(&CHARLIE, &call, &info, 107, 8));

		assert_eq!(<Test as crate::Config>::Currency::free_balance(&CHARLIE), 1);

		// check the treasury account for fee ksm income
		assert_eq!(
			<Test as crate::Config>::MultiCurrency::free_balance(
				CurrencyId::Token(TokenSymbol::KSM),
				&TREASURY_ACCOUNT
			),
			1
		);
	});
}

#[test]
// #[ignore = "This should be used with mock config type FeeDealer = FixedCurrencyFeeRate."]
fn correct_and_deposit_fee_should_work_v2() {
	new_test_ext().execute_with(|| {
		assert_ok!(Currencies::deposit(CurrencyId::Token(TokenSymbol::KSM), &CHARLIE, 2));
		assert_ok!(Currencies::deposit(CurrencyId::Native(TokenSymbol::ASG), &CHARLIE, 30));
		// prepare call variable
		let asset_order_list_vec: Vec<CurrencyId> =
			vec![CURRENCY_ID_0, CURRENCY_ID_1, CURRENCY_ID_2, CURRENCY_ID_3, CURRENCY_ID_4];
		let call =
			Call::FlexibleFee(crate::Call::set_user_fee_charge_order(Some(asset_order_list_vec)));
		// prepare info variable
		let extra = ();
		let xt = TestXt::new(call.clone(), Some((0u64, extra)));
		let info = xt.get_dispatch_info();

		// prepare post info
		let post_info = PostDispatchInfo { actual_weight: Some(20), pays_fee: Pays::Yes };

		let corrected_fee = 80;
		let tip = 8;

		// When the user is charged with KSM as fee, the already_withdrawn returned will be 0.
		let already_withdrawn = FlexibleFee::withdraw_fee(&CHARLIE, &call, &info, 107, 8).unwrap();

		assert_eq!(<Test as crate::Config>::Currency::free_balance(&CHARLIE), 30);

		// Since the fee withdrawl mode if allowdeath, if the account is destroyed
		// due to balance less than the existential deposit, no refund will be returned.

		// If already_withdrawn is less than corrected_fee, no refund will be executed
		assert_ok!(FlexibleFee::correct_and_deposit_fee(
			&CHARLIE,
			&info,
			&post_info,
			corrected_fee,
			tip,
			already_withdrawn
		));

		assert_eq!(
			<Test as crate::Config>::MultiCurrency::free_balance(
				CurrencyId::Native(TokenSymbol::ASG),
				&CHARLIE
			),
			30
		);

		assert_eq!(
			<Test as crate::Config>::MultiCurrency::free_balance(
				CurrencyId::Token(TokenSymbol::KSM),
				&CHARLIE
			),
			1
		);
	});
}

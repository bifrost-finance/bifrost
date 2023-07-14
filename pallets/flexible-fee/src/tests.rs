// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

use node_primitives::TryConvertFrom;
// use balances::Call as BalancesCall;
use crate::{
	mock::*, BlockNumberFor, BoundedVec, Config, DispatchError::BadOrigin, UserDefaultFeeCurrency,
};
use frame_support::{
	assert_noop, assert_ok,
	dispatch::{GetDispatchInfo, Pays, PostDispatchInfo},
	traits::WithdrawReasons,
	weights::Weight,
};
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::{testing::TestXt, AccountId32};
use zenlink_protocol::AssetId;

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
	let para_id: u32 = 2001;
	let asset_0_currency_id: AssetId = AssetId::try_convert_from(CURRENCY_ID_0, para_id).unwrap();
	let asset_1_currency_id: AssetId = AssetId::try_convert_from(CURRENCY_ID_1, para_id).unwrap();
	let asset_2_currency_id: AssetId = AssetId::try_convert_from(CURRENCY_ID_2, para_id).unwrap();
	let asset_3_currency_id: AssetId = AssetId::try_convert_from(CURRENCY_ID_3, para_id).unwrap();
	let asset_4_currency_id: AssetId = AssetId::try_convert_from(CURRENCY_ID_4, para_id).unwrap();

	assert_ok!(ZenlinkProtocol::create_pair(
		RuntimeOrigin::root(),
		asset_0_currency_id,
		asset_1_currency_id
	));

	assert_ok!(ZenlinkProtocol::create_pair(
		RuntimeOrigin::root(),
		asset_0_currency_id,
		asset_2_currency_id
	));

	assert_ok!(ZenlinkProtocol::create_pair(
		RuntimeOrigin::root(),
		asset_0_currency_id,
		asset_3_currency_id
	));

	assert_ok!(ZenlinkProtocol::create_pair(
		RuntimeOrigin::root(),
		asset_0_currency_id,
		asset_4_currency_id
	));

	let mut deadline: BlockNumberFor<Test> = <frame_system::Pallet<Test>>::block_number() +
		<Test as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		RuntimeOrigin::signed(DICK),
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
		RuntimeOrigin::signed(DICK),
		asset_0_currency_id,
		asset_2_currency_id,
		1000,
		1000,
		1,
		1,
		deadline
	));

	// pool 0 3
	deadline = <frame_system::Pallet<Test>>::block_number() +
		<Test as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		RuntimeOrigin::signed(DICK),
		asset_0_currency_id,
		asset_3_currency_id,
		1000,
		1000,
		1,
		1,
		deadline
	));

	// pool 0 4
	deadline = <frame_system::Pallet<Test>>::block_number() +
		<Test as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		RuntimeOrigin::signed(DICK),
		asset_0_currency_id,
		asset_4_currency_id,
		1000,
		1000,
		1,
		1,
		deadline
	));
}

#[test]
fn set_user_default_fee_currency_should_work() {
	new_test_ext().execute_with(|| {
		let origin_signed_alice = RuntimeOrigin::signed(ALICE);
		assert_ok!(FlexibleFee::set_user_default_fee_currency(
			origin_signed_alice.clone(),
			Some(CURRENCY_ID_0)
		));

		let alice_default_currency = UserDefaultFeeCurrency::<Test>::get(ALICE).unwrap();
		assert_eq!(alice_default_currency, CURRENCY_ID_0);

		assert_ok!(FlexibleFee::set_user_default_fee_currency(origin_signed_alice.clone(), None));
		assert_eq!(UserDefaultFeeCurrency::<Test>::get(ALICE).is_none(), true);
	});
}

#[test]
fn set_universal_fee_currency_order_list_should_work() {
	new_test_ext().execute_with(|| {
		let asset_order_list_vec: BoundedVec<
			CurrencyId,
			<Test as Config>::MaxFeeCurrencyOrderListLen,
		> = BoundedVec::try_from(vec![
			CURRENCY_ID_4,
			CURRENCY_ID_3,
			CURRENCY_ID_2,
			CURRENCY_ID_1,
			CURRENCY_ID_0,
		])
		.unwrap();
		assert_noop!(
			FlexibleFee::set_universal_fee_currency_order_list(
				RuntimeOrigin::signed(CHARLIE),
				asset_order_list_vec.clone()
			),
			BadOrigin
		);

		assert_ok!(FlexibleFee::set_universal_fee_currency_order_list(
			RuntimeOrigin::root(),
			asset_order_list_vec.clone()
		));

		assert_eq!(crate::UniversalFeeCurrencyOrderList::<Test>::get(), asset_order_list_vec);
	});
}

#[test]
fn inner_get_user_fee_charge_order_list_should_work() {
	new_test_ext().execute_with(|| {
		let asset_order_list_bounded_vec: BoundedVec<
			CurrencyId,
			<Test as Config>::MaxFeeCurrencyOrderListLen,
		> = BoundedVec::try_from(vec![
			CURRENCY_ID_4,
			CURRENCY_ID_3,
			CURRENCY_ID_2,
			CURRENCY_ID_1,
			CURRENCY_ID_0,
		])
		.unwrap();

		assert_ok!(FlexibleFee::set_universal_fee_currency_order_list(
			RuntimeOrigin::root(),
			asset_order_list_bounded_vec.clone(),
		));

		let mut asset_order_list_vec = asset_order_list_bounded_vec.into_iter().collect::<Vec<_>>();
		assert_eq!(
			FlexibleFee::inner_get_user_fee_charge_order_list(&ALICE),
			asset_order_list_vec.clone()
		);

		assert_ok!(FlexibleFee::set_user_default_fee_currency(
			RuntimeOrigin::signed(ALICE),
			Some(CURRENCY_ID_0),
		));

		let mut new_list = Vec::new();
		new_list.push(CURRENCY_ID_0);
		new_list.append(&mut asset_order_list_vec);

		assert_eq!(FlexibleFee::inner_get_user_fee_charge_order_list(&ALICE), new_list);
	});
}

// Three tests below are ignored due to some bugs of zenlink. Tests will be reopened after the bugs
// fixed.

#[test]
fn ensure_can_charge_fee_should_work() {
	new_test_ext().execute_with(|| {
		basic_setup();
		let origin_signed_bob = RuntimeOrigin::signed(BOB);
		let asset_order_list_vec: BoundedVec<
			CurrencyId,
			<Test as Config>::MaxFeeCurrencyOrderListLen,
		> = BoundedVec::try_from(vec![
			CURRENCY_ID_4,
			CURRENCY_ID_3,
			CURRENCY_ID_2,
			CURRENCY_ID_1,
			CURRENCY_ID_0,
		])
		.unwrap();

		assert_ok!(FlexibleFee::set_universal_fee_currency_order_list(
			RuntimeOrigin::root(),
			asset_order_list_vec.clone()
		));

		// Set bob order as [4,3,2,1]. Alice and Charlie will use the default order of [0..11]]
		let _ = FlexibleFee::set_user_default_fee_currency(
			origin_signed_bob.clone(),
			Some(CURRENCY_ID_0),
		);

		let _ = FlexibleFee::set_user_default_fee_currency(
			RuntimeOrigin::signed(ALICE),
			Some(CURRENCY_ID_1),
		);

		let _ = FlexibleFee::set_user_default_fee_currency(
			RuntimeOrigin::signed(CHARLIE),
			Some(CURRENCY_ID_2),
		);

		assert_ok!(FlexibleFee::ensure_can_charge_fee(
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
		assert_ok!(FlexibleFee::ensure_can_charge_fee(
			&BOB,
			100,
			WithdrawReasons::TRANSACTION_PAYMENT,
		));
		assert_eq!(<Test as crate::Config>::Currency::free_balance(&BOB), 100); // no exitential deposit requirement. 100 is enough
		assert_eq!(Currencies::total_balance(CURRENCY_ID_1, &BOB), 200);
	});
}

#[test]
fn withdraw_fee_should_work() {
	new_test_ext().execute_with(|| {
		basic_setup();

		let call = RuntimeCall::FlexibleFee(crate::Call::set_user_default_fee_currency {
			maybe_fee_currency: Some(CURRENCY_ID_0),
		});

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
fn correct_and_deposit_fee_should_work() {
	new_test_ext().execute_with(|| {
		basic_setup();
		let call = RuntimeCall::FlexibleFee(crate::Call::set_user_default_fee_currency {
			maybe_fee_currency: Some(CURRENCY_ID_0),
		});
		// prepare info variable
		let extra = ();
		let xt = TestXt::new(call.clone(), Some((0u64, extra)));
		let info = xt.get_dispatch_info();

		// prepare post info
		let post_info = PostDispatchInfo {
			actual_weight: Some(Weight::from_parts(20, 0)),
			pays_fee: Pays::Yes,
		};

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
fn deduct_salp_fee_should_work() {
	new_test_ext().execute_with(|| {
		// deposit some money for Charlie
		assert_ok!(Currencies::deposit(CURRENCY_ID_0, &CHARLIE, 200)); // Native token
		assert_ok!(Currencies::deposit(CURRENCY_ID_4, &CHARLIE, 200_000_000)); // Token KSM

		// prepare call variable
		let para_id = 2001;
		let value = 1_000_000_000_000;

		let call = RuntimeCall::Salp(bifrost_salp::Call::contribute { index: para_id, value });

		// prepare info variable
		let extra = ();
		let xt = TestXt::new(call.clone(), Some((0u64, extra)));
		let info = xt.get_dispatch_info();

		// 99 inclusion fee and a tip of 8
		assert_ok!(FlexibleFee::withdraw_fee(&CHARLIE, &call, &info, 107, 8));

		assert_eq!(<Test as crate::Config>::Currency::free_balance(&CHARLIE), 93);
		// fee is: 133780717
		assert_eq!(
			<Test as crate::Config>::MultiCurrency::free_balance(CURRENCY_ID_4, &CHARLIE),
			100000000
		);
		// treasury account has the fee
		assert_eq!(
			<Test as crate::Config>::MultiCurrency::free_balance(
				CURRENCY_ID_4,
				&<Test as crate::Config>::TreasuryAccount::get()
			),
			100000000
		);
	});
}

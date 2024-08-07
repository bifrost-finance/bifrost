// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

use bifrost_primitives::TryConvertFrom;
// use balances::Call as BalancesCall;
use crate::{
	mock::*, BlockNumberFor, BoundedVec, Config, DispatchError::BadOrigin, UserDefaultFeeCurrency,
};
use bifrost_primitives::{CurrencyId, TokenSymbol};
use frame_support::{
	assert_noop, assert_ok,
	dispatch::{GetDispatchInfo, Pays, PostDispatchInfo},
	traits::WithdrawReasons,
	weights::Weight,
};
use orml_traits::MultiCurrency;
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::{testing::TestXt, AccountId32};
use zenlink_protocol::AssetId;

// some common variables
pub const CHARLIE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);
pub const ALICE: AccountId32 = AccountId32::new([2u8; 32]);
pub const DICK: AccountId32 = AccountId32::new([3u8; 32]);
pub const CURRENCY_ID_0: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
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
		asset_1_currency_id,
		ALICE
	));

	assert_ok!(ZenlinkProtocol::create_pair(
		RuntimeOrigin::root(),
		asset_0_currency_id,
		asset_2_currency_id,
		ALICE
	));

	assert_ok!(ZenlinkProtocol::create_pair(
		RuntimeOrigin::root(),
		asset_0_currency_id,
		asset_3_currency_id,
		ALICE
	));

	assert_ok!(ZenlinkProtocol::create_pair(
		RuntimeOrigin::root(),
		asset_0_currency_id,
		asset_4_currency_id,
		ALICE
	));

	let mut deadline: BlockNumberFor<Test> =
		<frame_system::Pallet<Test>>::block_number() + BlockNumberFor::<Test>::from(100u32);
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
	deadline = <frame_system::Pallet<Test>>::block_number() + BlockNumberFor::<Test>::from(100u32);
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
	deadline = <frame_system::Pallet<Test>>::block_number() + BlockNumberFor::<Test>::from(100u32);
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
	deadline = <frame_system::Pallet<Test>>::block_number() + BlockNumberFor::<Test>::from(100u32);
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

		// Alice's default fee currency is Asset 1
		let _ = FlexibleFee::set_user_default_fee_currency(
			RuntimeOrigin::signed(ALICE),
			Some(CURRENCY_ID_1),
		);

		// Bob's default fee currency is Asset 0
		let _ = FlexibleFee::set_user_default_fee_currency(
			origin_signed_bob.clone(),
			Some(CURRENCY_ID_0),
		);

		// Alice originally should have 50 Asset 0 and 200 Asset 1
		// Now that 50 < 100, so Alice should be deducted some amount from Asset 1 and get 100 Asset
		// 0
		assert_ok!(FlexibleFee::ensure_can_charge_fee(
			&ALICE,
			100,
			WithdrawReasons::TRANSACTION_PAYMENT,
		));

		// Alice should be deducted 100 from Asset 1 since Asset 0 doesn't have enough balance.
		// asset1 : 200-112=88 asset0: 50+100 = 150
		assert_eq!(Currencies::total_balance(CURRENCY_ID_0, &ALICE), 150);
		assert_eq!(Currencies::total_balance(CURRENCY_ID_1, &ALICE), 88);

		// Currency 0 is the native currency.
		assert_eq!(<Test as crate::Config>::Currency::free_balance(&ALICE), 150);

		// Bob originally should have 100 Asset 0 and 200 Asset 1
		assert_ok!(FlexibleFee::ensure_can_charge_fee(
			&BOB,
			100,
			WithdrawReasons::TRANSACTION_PAYMENT,
		));
		assert_eq!(<Test as crate::Config>::Currency::free_balance(&BOB), 100); // no exitential deposit requirement. 100 is enough
																		  // Bob should be deducted 100 from Asset 0 since Asset 0 has enough balance.
																		  // Currency 1 should not be affected.
		assert_eq!(Currencies::total_balance(CURRENCY_ID_1, &BOB), 200);
	});
}

#[test]
fn find_out_fee_currency_and_amount_should_work() {
	new_test_ext().execute_with(|| {
		basic_setup();

		// set universal fee currency order list
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

		// charlie originally has 200 currency 0(Native currency)
		let (fee_token, amount_in, amount_out) =
			FlexibleFee::find_out_fee_currency_and_amount(&CHARLIE, 88).unwrap().unwrap();
		assert_eq!(fee_token, CURRENCY_ID_0);
		assert_eq!(amount_in, 88);
		assert_eq!(amount_out, 88);

		// alice originally should have 50 Asset 0. Should use Currency 4 to pay fee.
		let (fee_token, amount_in, amount_out) =
			FlexibleFee::find_out_fee_currency_and_amount(&ALICE, 88).unwrap().unwrap();
		assert_eq!(fee_token, CURRENCY_ID_4);
		assert_eq!(amount_in, 97);
		assert_eq!(amount_out, 88);
	});
}

#[test]
fn get_extrinsic_and_extra_fee_total_should_work() {
	new_test_ext().execute_with(|| {
		basic_setup();

		let native_asset_id = FlexibleFee::get_currency_asset_id(CURRENCY_ID_0).unwrap();
		let asset_id = FlexibleFee::get_currency_asset_id(CURRENCY_ID_4).unwrap();

		// call with no extra fee
		let path_vec = vec![native_asset_id, native_asset_id];
		let (total_fee, extra_bnc_fee, fee_value, path) =
			FlexibleFee::get_extrinsic_and_extra_fee_total(&BALANCE_TRANSFER_CALL, 88).unwrap();
		assert_eq!(total_fee, 88);
		assert_eq!(extra_bnc_fee, 0);
		assert_eq!(fee_value, 0);
		assert_eq!(path, path_vec);

		//  salp contribuite call with extra fee
		let path_vec = vec![native_asset_id, asset_id];
		let (total_fee, extra_bnc_fee, fee_value, path) =
			FlexibleFee::get_extrinsic_and_extra_fee_total(&SALP_CONTRIBUTE_CALL, 88).unwrap();
		assert_eq!(total_fee, 200);
		assert_eq!(extra_bnc_fee, 112);
		assert_eq!(fee_value, 100);
		assert_eq!(path, path_vec);

		// vtoken-voting vote call with extra fee
		let path_vec = vec![native_asset_id, asset_id];
		let (total_fee, extra_bnc_fee, fee_value, path) =
			FlexibleFee::get_extrinsic_and_extra_fee_total(&VTOKENVOTING_VOTE_CALL, 88).unwrap();
		assert_eq!(total_fee, 200);
		assert_eq!(extra_bnc_fee, 112);
		assert_eq!(fee_value, 100);
		assert_eq!(path, path_vec);
	});
}

#[test]
fn cal_fee_token_and_amount_should_work() {
	new_test_ext().execute_with(|| {
		basic_setup();

		// set universal fee currency order list
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

		// use default asset_order_list_vec
		let (currency_id, amount_in) =
			FlexibleFee::cal_fee_token_and_amount(&ALICE, 20, &BALANCE_TRANSFER_CALL).unwrap();
		assert_eq!(currency_id, CURRENCY_ID_4);
		assert_eq!(amount_in, 21);

		// set alice's default fee currency to be CURRENCY_ID_0
		assert_ok!(FlexibleFee::set_user_default_fee_currency(
			RuntimeOrigin::signed(ALICE),
			Some(CURRENCY_ID_0),
		));

		// alice has enough balance of CURRENCY_ID_0, so should use CURRENCY_ID_0 to pay fee
		let (currency_id, amount_in) =
			FlexibleFee::cal_fee_token_and_amount(&ALICE, 20, &BALANCE_TRANSFER_CALL).unwrap();
		assert_eq!(currency_id, CURRENCY_ID_0);
		assert_eq!(amount_in, 20);

		// alice originally only have 50 CURRENCY_ID_0. Should use Currency 4 to pay fee.
		let (currency_id, amount_in) =
			FlexibleFee::cal_fee_token_and_amount(&ALICE, 88, &SALP_CONTRIBUTE_CALL).unwrap();
		assert_eq!(currency_id, CURRENCY_ID_4);
		assert_eq!(amount_in, 251);
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
		basic_setup();

		// prepare info variable
		let extra = ();
		let xt = TestXt::new(SALP_CONTRIBUTE_CALL.clone(), Some((0u64, extra)));
		let info = xt.get_dispatch_info();

		// 80 inclusion fee and a tip of 8
		assert_ok!(FlexibleFee::withdraw_fee(&CHARLIE, &SALP_CONTRIBUTE_CALL, &info, 80, 8));

		// originally Charlie has 200 currency 0(Native currency)
		// 200 - 88 = 112. extra fee cost 104. 112 - 104 = 8
		assert_eq!(<Test as crate::Config>::Currency::free_balance(&CHARLIE), 8);

		// Other currencies should not be affected
		assert_eq!(
			<Test as crate::Config>::MultiCurrency::free_balance(CURRENCY_ID_1, &CHARLIE),
			20
		);
		assert_eq!(
			<Test as crate::Config>::MultiCurrency::free_balance(CURRENCY_ID_2, &CHARLIE),
			30
		);
		assert_eq!(
			<Test as crate::Config>::MultiCurrency::free_balance(CURRENCY_ID_3, &CHARLIE),
			40
		);
		assert_eq!(
			<Test as crate::Config>::MultiCurrency::free_balance(CURRENCY_ID_4, &CHARLIE),
			50
		);
	});
}

#[test]
fn get_currency_asset_id_should_work() {
	new_test_ext().execute_with(|| {
		// BNC
		let asset_id = FlexibleFee::get_currency_asset_id(CURRENCY_ID_0).unwrap();
		let bnc_asset_id = AssetId { chain_id: 2001, asset_type: 0, asset_index: 0 };
		assert_eq!(asset_id, bnc_asset_id);

		// KSM
		let asset_id = FlexibleFee::get_currency_asset_id(CURRENCY_ID_4).unwrap();
		let ksm_asset_id = AssetId { chain_id: 2001, asset_type: 2, asset_index: 516 };
		assert_eq!(asset_id, ksm_asset_id);
	});
}

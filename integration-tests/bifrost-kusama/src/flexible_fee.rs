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

use crate::{kusama_integration_tests::*, kusama_test_net::*};
use bifrost_flexible_fee::UserFeeChargeOrderList;
use bifrost_kusama_runtime::{Call, FlexibleFee, ZenlinkProtocol};
use bifrost_runtime_common::milli;
use frame_support::{
	assert_ok,
	weights::{GetDispatchInfo, Pays, PostDispatchInfo},
};
use frame_system::pallet_prelude::BlockNumberFor;
use node_primitives::TryConvertFrom;
use pallet_transaction_payment::OnChargeTransaction;
use sp_runtime::testing::TestXt;
use xcm_emulator::TestExt;
use zenlink_protocol::AssetId;

// some common variables
pub const CHARLIE: AccountId = AccountId::new([0u8; 32]);
pub const BOB: AccountId = AccountId::new([1u8; 32]);
pub const ALICE: AccountId = AccountId::new([2u8; 32]);
pub const DICK: AccountId = AccountId::new([3u8; 32]);
pub const CURRENCY_ID_0: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
pub const CURRENCY_ID_1: CurrencyId = CurrencyId::Stable(TokenSymbol::KUSD);
pub const CURRENCY_ID_2: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
pub const CURRENCY_ID_3: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
pub const CURRENCY_ID_4: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);

fn basic_setup() {
	// Deposit some money in Alice, Bob and Charlie's accounts.
	// Alice
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		ALICE.into(),
		CURRENCY_ID_0,
		10 * milli(CURRENCY_ID_0) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		ALICE.into(),
		CURRENCY_ID_1,
		10 * dollar(CURRENCY_ID_1) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		ALICE.into(),
		CURRENCY_ID_2,
		10 * dollar(CURRENCY_ID_2) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		ALICE.into(),
		CURRENCY_ID_3,
		100 * dollar(CURRENCY_ID_3) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		ALICE.into(),
		CURRENCY_ID_4,
		10 * dollar(CURRENCY_ID_4) as i128
	));

	// Bob
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		BOB.into(),
		CURRENCY_ID_0,
		10 * dollar(CURRENCY_ID_0) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		BOB.into(),
		CURRENCY_ID_1,
		10 * dollar(CURRENCY_ID_1) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		BOB.into(),
		CURRENCY_ID_2,
		10 * dollar(CURRENCY_ID_2) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		BOB.into(),
		CURRENCY_ID_3,
		100 * dollar(CURRENCY_ID_3) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		BOB.into(),
		CURRENCY_ID_4,
		10 * dollar(CURRENCY_ID_4) as i128
	));

	// Charlie
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		CHARLIE.into(),
		CURRENCY_ID_0,
		10 * dollar(CURRENCY_ID_0) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		CHARLIE.into(),
		CURRENCY_ID_1,
		10 * dollar(CURRENCY_ID_1) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		CHARLIE.into(),
		CURRENCY_ID_2,
		10 * dollar(CURRENCY_ID_2) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		CHARLIE.into(),
		CURRENCY_ID_3,
		100 * dollar(CURRENCY_ID_3) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		CHARLIE.into(),
		CURRENCY_ID_4,
		10 * dollar(CURRENCY_ID_4) as i128
	));

	// Dick
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		DICK.into(),
		CURRENCY_ID_0,
		10 * dollar(CURRENCY_ID_0) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		DICK.into(),
		CURRENCY_ID_1,
		10 * dollar(CURRENCY_ID_1) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		DICK.into(),
		CURRENCY_ID_2,
		10 * dollar(CURRENCY_ID_2) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		DICK.into(),
		CURRENCY_ID_3,
		100 * dollar(CURRENCY_ID_3) as i128
	));
	assert_ok!(Currencies::update_balance(
		Origin::root(),
		DICK.into(),
		CURRENCY_ID_4,
		10 * dollar(CURRENCY_ID_4) as i128
	));

	// create DEX pair
	let parachain_id: u32 = 2001;
	let asset_0_currency_id: AssetId =
		AssetId::try_convert_from(CURRENCY_ID_0, parachain_id).unwrap();
	let asset_1_currency_id: AssetId =
		AssetId::try_convert_from(CURRENCY_ID_1, parachain_id).unwrap();
	let asset_2_currency_id: AssetId =
		AssetId::try_convert_from(CURRENCY_ID_2, parachain_id).unwrap();
	let asset_3_currency_id: AssetId =
		AssetId::try_convert_from(CURRENCY_ID_3, parachain_id).unwrap();
	let asset_4_currency_id: AssetId =
		AssetId::try_convert_from(CURRENCY_ID_4, parachain_id).unwrap();

	assert_ok!(ZenlinkProtocol::create_pair(
		Origin::root(),
		asset_0_currency_id,
		asset_1_currency_id
	));

	assert_ok!(ZenlinkProtocol::create_pair(
		Origin::root(),
		asset_0_currency_id,
		asset_2_currency_id
	));

	assert_ok!(ZenlinkProtocol::create_pair(
		Origin::root(),
		asset_0_currency_id,
		asset_3_currency_id
	));

	assert_ok!(ZenlinkProtocol::create_pair(
		Origin::root(),
		asset_0_currency_id,
		asset_4_currency_id
	));

	let mut deadline: BlockNumberFor<Runtime> = <frame_system::Pallet<Runtime>>::block_number() +
		<Runtime as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		Origin::signed(DICK),
		asset_0_currency_id,
		asset_1_currency_id,
		1 * dollar(CURRENCY_ID_0),
		1 * dollar(CURRENCY_ID_1),
		1,
		1,
		deadline
	));

	// pool 0 2
	deadline = <frame_system::Pallet<Runtime>>::block_number() +
		<Runtime as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		Origin::signed(DICK),
		asset_0_currency_id,
		asset_2_currency_id,
		1 * dollar(CURRENCY_ID_0),
		1 * dollar(CURRENCY_ID_2),
		1,
		1,
		deadline
	));

	// pool 0 3
	deadline = <frame_system::Pallet<Runtime>>::block_number() +
		<Runtime as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		Origin::signed(DICK),
		asset_0_currency_id,
		asset_3_currency_id,
		1 * dollar(CURRENCY_ID_0),
		1 * dollar(CURRENCY_ID_3),
		1,
		1,
		deadline
	));

	// pool 0 4
	deadline = <frame_system::Pallet<Runtime>>::block_number() +
		<Runtime as frame_system::Config>::BlockNumber::from(100u32);
	assert_ok!(ZenlinkProtocol::add_liquidity(
		Origin::signed(DICK),
		asset_0_currency_id,
		asset_4_currency_id,
		1 * dollar(CURRENCY_ID_0),
		1 * dollar(CURRENCY_ID_4),
		1,
		1,
		deadline
	));
}

#[test]
fn set_user_fee_charge_order_should_work() {
	Bifrost::execute_with(|| {
		let origin_signed_alice = Origin::signed(ALICE);
		let mut asset_order_list_vec: Vec<CurrencyId> =
			vec![CURRENCY_ID_4, CURRENCY_ID_3, CURRENCY_ID_2, CURRENCY_ID_1, CURRENCY_ID_0];
		assert_ok!(FlexibleFee::set_user_fee_charge_order(
			origin_signed_alice.clone(),
			Some(asset_order_list_vec.clone())
		));

		asset_order_list_vec.insert(0, CURRENCY_ID_0);
		assert_eq!(UserFeeChargeOrderList::<Runtime>::get(ALICE), Some(asset_order_list_vec));

		assert_ok!(FlexibleFee::set_user_fee_charge_order(origin_signed_alice, None));

		assert_eq!(UserFeeChargeOrderList::<Runtime>::get(ALICE).is_none(), true);
	});
}

#[test]
fn withdraw_fee_should_work() {
	Bifrost::execute_with(|| {
		basic_setup();

		// prepare call variable
		let asset_order_list_vec: Vec<CurrencyId> =
			vec![CURRENCY_ID_0, CURRENCY_ID_1, CURRENCY_ID_2, CURRENCY_ID_3, CURRENCY_ID_4];
		let call = Call::FlexibleFee(bifrost_flexible_fee::Call::set_user_fee_charge_order {
			asset_order_list_vec: Some(asset_order_list_vec),
		});

		// prepare info variable
		let extra = ();
		let xt = TestXt::new(call.clone(), Some((0u64, extra)));
		let info = xt.get_dispatch_info();

		// 99 inclusion fee and a tip of 8
		assert_ok!(FlexibleFee::withdraw_fee(&CHARLIE, &call, &info, 107, 8));

		assert_eq!(
			<Runtime as bifrost_flexible_fee::Config>::Currency::free_balance(&CHARLIE),
			10 * dollar(CURRENCY_ID_0) - 107
		);
	});
}

#[test]
fn correct_and_deposit_fee_should_work() {
	Bifrost::execute_with(|| {
		basic_setup();
		// prepare call variable
		let asset_order_list_vec: Vec<CurrencyId> =
			vec![CURRENCY_ID_0, CURRENCY_ID_1, CURRENCY_ID_2, CURRENCY_ID_3, CURRENCY_ID_4];
		let call = Call::FlexibleFee(bifrost_flexible_fee::Call::set_user_fee_charge_order {
			asset_order_list_vec: Some(asset_order_list_vec),
		});
		// prepare info variable
		let extra = ();
		let xt = TestXt::new(call.clone(), Some((0u64, extra)));
		let info = xt.get_dispatch_info();

		// prepare post info
		let post_info = PostDispatchInfo { actual_weight: Some(20), pays_fee: Pays::Yes };

		let corrected_fee = 80;
		let tip = 8;

		let already_withdrawn =
			FlexibleFee::withdraw_fee(&CHARLIE, &call, &info, corrected_fee, tip).unwrap();

		assert_eq!(
			<Runtime as bifrost_flexible_fee::Config>::Currency::free_balance(&CHARLIE),
			10 * dollar(CURRENCY_ID_0) - 80
		);

		assert_ok!(FlexibleFee::correct_and_deposit_fee(
			&CHARLIE,
			&info,
			&post_info,
			corrected_fee,
			tip,
			already_withdrawn
		));

		assert_eq!(
			<Runtime as bifrost_flexible_fee::Config>::Currency::free_balance(&CHARLIE),
			10 * dollar(CURRENCY_ID_0) - 80
		);
	});
}

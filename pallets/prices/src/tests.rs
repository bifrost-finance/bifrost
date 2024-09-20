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

//! Unit tests for the prices pallet.

use super::*;
use bifrost_asset_registry::AssetMetadata;
use bifrost_primitives::{BNC, MANTA, VKSM};
use frame_support::{assert_noop, assert_ok};
use mock::{RuntimeEvent, *};
use sp_runtime::{traits::BadOrigin, FixedPointNumber};

#[test]
fn get_price_from_oracle() {
	new_test_ext().execute_with(|| {
		// currency exist
		assert_eq!(
			Prices::get_price(&DOT),
			Some((Price::from_inner(10_000_000_000 * PRICE_ONE), 0))
		);

		// currency not exist
		assert_eq!(Prices::get_price(&VKSM), None);
	});
}

#[test]
fn set_price_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			Prices::get_price(&DOT),
			Some((Price::from_inner(10_000_000_000 * PRICE_ONE), 0))
		);
		// set DOT price
		assert_ok!(Prices::set_price(
			RuntimeOrigin::signed(ALICE),
			DOT,
			Price::saturating_from_integer(99)
		));
		assert_eq!(
			Prices::get_price(&DOT),
			Some((Price::from_inner(9_900_000_000 * PRICE_ONE), 0))
		);
		assert_ok!(Prices::set_price(
			RuntimeOrigin::signed(ALICE),
			KSM,
			Price::saturating_from_integer(1)
		));
		assert_eq!(Prices::get_emergency_price(&KSM), Some((1_000_000.into(), 0)));
	});
}

#[test]
fn reset_price_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			Prices::get_price(&DOT),
			Some((Price::from_inner(10_000_000_000 * PRICE_ONE), 0))
		);
		// set DOT price
		EmergencyPrice::<Test>::insert(DOT, Price::saturating_from_integer(99));
		assert_eq!(
			Prices::get_price(&DOT),
			Some((Price::from_inner(9_900_000_000 * PRICE_ONE), 0))
		);

		// reset DOT price
		EmergencyPrice::<Test>::remove(DOT);
		assert_eq!(
			Prices::get_price(&DOT),
			Some((Price::from_inner(10_000_000_000 * PRICE_ONE), 0))
		);
	});
}

#[test]
fn set_price_call_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// set emergency price from 100 to 90
		assert_eq!(
			Prices::get_price(&DOT),
			Some((FixedU128::from_inner(10_000_000_000 * PRICE_ONE), 0))
		);
		assert_noop!(
			Prices::set_price(
				RuntimeOrigin::signed(CHARLIE),
				DOT,
				Price::saturating_from_integer(100),
			),
			BadOrigin
		);
		assert_ok!(Prices::set_price(
			RuntimeOrigin::signed(ALICE),
			DOT,
			Price::saturating_from_integer(90),
		));
		assert_eq!(
			Prices::get_price(&DOT),
			Some((FixedU128::from_inner(9_000_000_000 * PRICE_ONE), 0))
		);

		// check the event
		let set_price_event =
			RuntimeEvent::Prices(crate::Event::SetPrice(DOT, Price::saturating_from_integer(90)));
		assert!(System::events().iter().any(|record| record.event == set_price_event));
		assert_eq!(
			Prices::set_price(
				RuntimeOrigin::signed(ALICE),
				DOT,
				Price::saturating_from_integer(90),
			),
			Ok(().into())
		);
	});
}

#[test]
fn reset_price_call_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		// set emergency price from 100 to 90
		assert_eq!(
			Prices::get_price(&DOT),
			Some((FixedU128::from_inner(10_000_000_000 * PRICE_ONE), 0))
		);
		assert_ok!(Prices::set_price(
			RuntimeOrigin::signed(ALICE),
			DOT,
			Price::saturating_from_integer(90),
		));
		assert_eq!(
			Prices::get_price(&DOT),
			Some((FixedU128::from_inner(9_000_000_000 * PRICE_ONE), 0))
		);

		// try reset price
		assert_noop!(Prices::reset_price(RuntimeOrigin::signed(CHARLIE), DOT), BadOrigin);
		assert_ok!(Prices::reset_price(RuntimeOrigin::signed(ALICE), DOT));

		// price need to be 100 after reset_price
		assert_eq!(
			Prices::get_price(&DOT),
			Some((FixedU128::from_inner(10_000_000_000 * PRICE_ONE), 0))
		);

		// check the event
		let reset_price_event = RuntimeEvent::Prices(crate::Event::ResetPrice(DOT));
		assert!(System::events().iter().any(|record| record.event == reset_price_event));
		assert_eq!(Prices::reset_price(RuntimeOrigin::signed(ALICE), DOT), Ok(().into()));
	});
}

#[test]
fn get_token_price_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			Prices::get_price(&DOT),
			Some((Price::from_inner(10_000_000_000 * PRICE_ONE), 0))
		);

		assert_eq!(
			Prices::get_price(&FIL),
			Some((Price::from_inner(6666666666_666666660000000000), 0))
		);
	});
}

#[test]
fn get_foreign_token_price_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(
			Prices::get_price(&DOT),
			Some((Price::from_inner(10_000_000_000 * PRICE_ONE), 0))
		);

		assert_eq!(
			Prices::get_price(&FIL),
			Some((Price::from_inner(6666666666_666666660000000000), 0))
		);

		assert_eq!(Prices::get_price(&FIL), Prices::get_price(&VFIL));
	});
}

#[test]
fn fixed_u128() {
	new_test_ext().execute_with(|| {
		let bnc_decimal = 10u128.pow(12);
		let bnc_amount = 100 * 10u128.pow(12);
		let bnc_price = FixedU128::from_inner(200_000_000_000_000_000);
		// 100 * 0.2 = 20 U
		let dot_decimal = 10u128.pow(10);
		let dot_amount = 5 * 10u128.pow(10);
		let dot_price = FixedU128::from(4);

		let bnc_total_value =
			bnc_price / FixedU128::from_inner(bnc_decimal) * FixedU128::from_inner(bnc_amount);
		let dot_total_value =
			dot_price / FixedU128::from_inner(dot_decimal) * FixedU128::from_inner(dot_amount);
		let dot_amount_fixed_u128 =
			dot_total_value * FixedU128::from_inner(dot_decimal) / dot_price;
		assert_eq!(bnc_total_value, dot_total_value);
		println!("{:?}", bnc_total_value);
		println!("{:?}", dot_amount_fixed_u128);
		assert_eq!(dot_amount, dot_amount_fixed_u128.into_inner());
	})
}

#[test]
fn get_oracle_amount_by_currency_and_amount_in() {
	new_test_ext().execute_with(|| {
		assert_ok!(AssetRegistry::do_register_metadata(
			MANTA,
			&AssetMetadata {
				name: b"Manta".to_vec(),
				symbol: b"Manta".to_vec(),
				decimals: 18,
				minimal_balance: 1_000_000_000_000_000u128,
			}
		));
		// 100 * 0.2 = 20u
		let bnc_amount = 100 * 10u128.pow(12);
		// 0.2 DOT
		assert_eq!(
			Some((
				2_000_000_000,
				Price::from_inner(200_000_000_000_000_000),
				Price::saturating_from_integer(100)
			)),
			Prices::get_oracle_amount_by_currency_and_amount_in(&BNC, bnc_amount, &DOT)
		);
		// 0.04 KSM
		assert_eq!(
			Some((
				40_000_000_000,
				Price::from_inner(200_000_000_000_000_000),
				Price::saturating_from_integer(500)
			)),
			Prices::get_oracle_amount_by_currency_and_amount_in(&BNC, bnc_amount, &KSM)
		);
		// 33.33333333333333333333
		assert_eq!(
			Some((
				33_333_333_333_333_333_333,
				Price::from_inner(200_000_000_000_000_000),
				Price::from_inner(600_000_000_000_000_000)
			)),
			Prices::get_oracle_amount_by_currency_and_amount_in(&BNC, bnc_amount, &MANTA)
		);

		// 0.01 * 0.2 = 0.002 U
		let bnc_amount = 10u128.pow(10);
		// 0.00002 DOT * 100 =  0.002 U
		assert_eq!(
			Some((
				200_000,
				Price::from_inner(200_000_000_000_000_000),
				Price::saturating_from_integer(100)
			)),
			Prices::get_oracle_amount_by_currency_and_amount_in(&BNC, bnc_amount, &DOT)
		);
		// 0.000004 KSM * 500 = 0.002 U
		assert_eq!(
			Some((
				4_000_000,
				Price::from_inner(200_000_000_000_000_000),
				Price::saturating_from_integer(500)
			)),
			Prices::get_oracle_amount_by_currency_and_amount_in(&BNC, bnc_amount, &KSM)
		);
		// 0.003333333333333333333333 MANTA
		assert_eq!(
			Some((
				3_333_333_333_333_333,
				Price::from_inner(200_000_000_000_000_000),
				Price::from_inner(600_000_000_000_000_000)
			)),
			Prices::get_oracle_amount_by_currency_and_amount_in(&BNC, bnc_amount, &MANTA)
		);
	});
}

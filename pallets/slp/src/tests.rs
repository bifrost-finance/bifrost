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

#![cfg(test)]

use frame_support::assert_ok;
use mock::{Event, *};
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;

use super::*;
use crate::KSM;

#[test]
fn set_xcm_dest_weight_and_fee_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// Insert a new record.
		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::signed(ALICE),
			KSM,
			XcmOperation::Bond,
			Some((5_000_000_000, 5_000_000_000))
		));

		assert_eq!(
			XcmDestWeightAndFee::<Runtime>::get(KSM, XcmOperation::Bond),
			Some((5_000_000_000, 5_000_000_000))
		);

		// Delete a record.
		assert_ok!(Slp::set_xcm_dest_weight_and_fee(
			Origin::signed(ALICE),
			KSM,
			XcmOperation::Bond,
			None
		));

		assert_eq!(XcmDestWeightAndFee::<Runtime>::get(KSM, XcmOperation::Bond), None);
	});
}

#[test]
fn construct_xcm_and_send_as_subaccount_should_work() {
	let para_chain_account: AccountId =
		hex_literal::hex!["70617261d1070000000000000000000000000000000000000000000000000000"]
			.into();

	let sub_account_id = SubAccountIndexMultiLocationConvertor::derivative_account_id(
		para_chain_account.clone(),
		0u16,
	);

	// parachain_account 2001: 5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E
	// hex: 70617261d1070000000000000000000000000000000000000000000000000000
	println!("para_string: {:?}", para_chain_account);
	// sub_account index:0(sub_account_id.to_string()))
	// 5E78xTBiaN3nAGYtcNnqTJQJqYAkSDGggKqaDfpNsKyPpbcb
	// hex: 5a53736d8e96f1c007cf0d630acf5209b20611617af23ce924c8e25328eb5d28
	println!("sub_string: {:?}", sub_account_id);
}

#[test]
fn set_fee_source_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let alice_32 = Pallet::<Runtime>::account_id_to_account_32(ALICE).unwrap();
		let alice_location = Pallet::<Runtime>::account_32_to_local_location(alice_32).unwrap();

		// Insert a new record.
		assert_ok!(Slp::set_fee_source(
			Origin::signed(ALICE),
			KSM,
			Some((alice_location.clone(), 1_000_000_000_000))
		));
		assert_eq!(FeeSources::<Runtime>::get(KSM), Some((alice_location, 1_000_000_000_000)));
	});
}

// test native token fee supplement. Non-native will be tested in the integration tests.
#[test]
fn supplement_fee_reserve_works() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		// set fee source
		let alice_32 = Pallet::<Runtime>::account_id_to_account_32(ALICE).unwrap();
		let alice_location = Pallet::<Runtime>::account_32_to_local_location(alice_32).unwrap();
		assert_ok!(Slp::set_fee_source(
			Origin::signed(ALICE),
			BNC,
			Some((alice_location.clone(), 10))
		));

		// supplement fee
		let bob_32 = Pallet::<Runtime>::account_id_to_account_32(BOB).unwrap();
		let bob_location = Pallet::<Runtime>::account_32_to_local_location(bob_32).unwrap();
		assert_eq!(Balances::free_balance(&ALICE), 100);
		assert_eq!(Balances::free_balance(&BOB), 0);

		assert_ok!(Slp::supplement_fee_reserve(Origin::signed(ALICE), BNC, bob_location.clone()));

		assert_eq!(Balances::free_balance(&ALICE), 90);
		assert_eq!(Balances::free_balance(&BOB), 10);
	});
}

#[test]
fn remove_delegator_works() {
	ExtBuilder::default().build().execute_with(|| {
		// 5E78xTBiaN3nAGYtcNnqTJQJqYAkSDGggKqaDfpNsKyPpbcb
		let subaccount_0: AccountId =
			hex_literal::hex!["5a53736d8e96f1c007cf0d630acf5209b20611617af23ce924c8e25328eb5d28"]
				.into();
		let subaccount_0_32: [u8; 32] =
			Slp::account_id_to_account_32(subaccount_0.clone()).unwrap();
		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		DelegatorsIndex2Multilocation::<Runtime>::insert(KSM, 0, subaccount_0_location.clone());
		DelegatorsMultilocation2Index::<Runtime>::insert(KSM, subaccount_0_location.clone(), 0);

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 100_000_000_000,
			bond_extra_minimum: 0,
			unbond_minimum: 0,
			rebond_minimum: 0,
			unbond_record_maximum: 32,
			validators_back_maximum: 36,
			delegator_active_staking_maximum: 200_000_000_000_000,
		};

		// Set minimums and maximums
		MinimumsAndMaximums::<Runtime>::insert(KSM, mins_and_maxs);

		let sb_ledger = SubstrateLedger {
			account: subaccount_0_location.clone(),
			total: 0,
			active: 0,
			unlocking: vec![],
		};
		let ledger = Ledger::Substrate(sb_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(KSM, subaccount_0_location.clone(), ledger);

		assert_ok!(Slp::remove_delegator(
			Origin::signed(ALICE),
			KSM,
			subaccount_0_location.clone()
		));

		assert_eq!(DelegatorsIndex2Multilocation::<Runtime>::get(KSM, 0), None);
		assert_eq!(
			DelegatorsMultilocation2Index::<Runtime>::get(KSM, subaccount_0_location.clone()),
			None
		);
		assert_eq!(DelegatorLedgers::<Runtime>::get(KSM, subaccount_0_location), None);
	});
}

/// ****************************************
// Below is the VtokenMinting api testing *
/// ****************************************

#[test]
fn decrease_token_pool_works() {
	ExtBuilder::default().build().execute_with(|| {
		// Set token pool as 100.
		bifrost_vtoken_minting::TokenPool::<Runtime>::insert(KSM, 100);

		// Decrease token pool by 10.
		assert_ok!(Slp::decrease_token_pool(Origin::signed(ALICE), KSM, 10));

		// Check the value after decreasing
		assert_eq!(VtokenMinting::token_pool(KSM), 90);
	});
}

#[test]
fn update_ongoing_time_unit_works() {
	ExtBuilder::default().build().execute_with(|| {
		// Set the era to be 8.
		bifrost_vtoken_minting::OngoingTimeUnit::<Runtime>::insert(KSM, TimeUnit::Era(8));

		// Update the era to be 9.
		assert_ok!(Slp::update_ongoing_time_unit(Origin::signed(ALICE), KSM, TimeUnit::Era(9)));

		// Check the value after updating.
		assert_eq!(VtokenMinting::ongoing_time_unit(KSM), Some(TimeUnit::Era(9)));
	});
}

#[test]
fn refund_currency_due_unbond_works() {
	ExtBuilder::default().build().execute_with(|| {
		// Preparations
		// get entrance and exit accounts
		let (entrance_acc, exit_acc) = VtokenMinting::get_entrance_and_exit_accounts();
		// Set exit account balance to be 50.
		assert_ok!(Tokens::set_balance(Origin::root(), exit_acc.clone(), KSM, 50, 0));

		// set current era to be 100.
		bifrost_vtoken_minting::OngoingTimeUnit::<Runtime>::insert(KSM, TimeUnit::Era(100));

		// Set TokenUnlockLedger records.
		let record_bob = (BOB, 10, TimeUnit::Era(90));
		bifrost_vtoken_minting::TokenUnlockLedger::<Runtime>::insert(KSM, 0, record_bob);

		let record_charlie = (CHARLIE, 28, TimeUnit::Era(100));
		bifrost_vtoken_minting::TokenUnlockLedger::<Runtime>::insert(KSM, 1, record_charlie);

		let record_dave = (DAVE, 30, TimeUnit::Era(100));
		bifrost_vtoken_minting::TokenUnlockLedger::<Runtime>::insert(KSM, 2, record_dave);

		let record_eddie_1 = (EDDIE, 7, TimeUnit::Era(110));
		bifrost_vtoken_minting::TokenUnlockLedger::<Runtime>::insert(KSM, 3, record_eddie_1);

		let record_eddie_2 = (EDDIE, 6, TimeUnit::Era(110));
		bifrost_vtoken_minting::TokenUnlockLedger::<Runtime>::insert(KSM, 4, record_eddie_2);

		// insert TimeUnitUnlockLedger records
		let bounded_vec_90 = BoundedVec::try_from(vec![0]).unwrap();
		let time_record_90 = (10, bounded_vec_90, KSM);
		bifrost_vtoken_minting::TimeUnitUnlockLedger::<Runtime>::insert(
			TimeUnit::Era(90),
			KSM,
			time_record_90.clone(),
		);

		let bounded_vec_100 = BoundedVec::try_from(vec![1, 2]).unwrap();
		let time_record_100 = (58, bounded_vec_100, KSM);
		bifrost_vtoken_minting::TimeUnitUnlockLedger::<Runtime>::insert(
			TimeUnit::Era(100),
			KSM,
			time_record_100,
		);

		let bounded_vec_110 = BoundedVec::try_from(vec![3, 4]).unwrap();
		let time_record_110 = (13, bounded_vec_110, KSM);
		bifrost_vtoken_minting::TimeUnitUnlockLedger::<Runtime>::insert(
			TimeUnit::Era(110),
			KSM,
			time_record_110.clone(),
		);

		// insert UserUnlockLedger records.
		let bounded_vec_bob = BoundedVec::try_from(vec![0]).unwrap();
		bifrost_vtoken_minting::UserUnlockLedger::<Runtime>::insert(
			BOB,
			KSM,
			(10, bounded_vec_bob.clone()),
		);

		let bounded_vec_charlie = BoundedVec::try_from(vec![1]).unwrap();
		bifrost_vtoken_minting::UserUnlockLedger::<Runtime>::insert(
			CHARLIE,
			KSM,
			(28, bounded_vec_charlie.clone()),
		);

		let bounded_vec_dave = BoundedVec::try_from(vec![2]).unwrap();
		bifrost_vtoken_minting::UserUnlockLedger::<Runtime>::insert(
			DAVE,
			KSM,
			(30, bounded_vec_dave.clone()),
		);

		let bounded_vec_eddie = BoundedVec::try_from(vec![3, 4]).unwrap();
		bifrost_vtoken_minting::UserUnlockLedger::<Runtime>::insert(
			EDDIE,
			KSM,
			(13, bounded_vec_eddie.clone()),
		);

		// check account balances before refund
		assert_eq!(Tokens::free_balance(KSM, &exit_acc), 50);
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);
		assert_eq!(Tokens::free_balance(KSM, &CHARLIE), 0);
		assert_eq!(Tokens::free_balance(KSM, &DAVE), 0);
		assert_eq!(Tokens::free_balance(KSM, &EDDIE), 0);

		// Refund user
		assert_ok!(Slp::refund_currency_due_unbond(Origin::signed(ALICE), KSM));

		// Check account balances after refund
		assert_eq!(Tokens::free_balance(KSM, &exit_acc), 0);
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);
		assert_eq!(Tokens::free_balance(KSM, &CHARLIE), 28);
		assert_eq!(Tokens::free_balance(KSM, &DAVE), 22);
		assert_eq!(Tokens::free_balance(KSM, &EDDIE), 0);

		// Check storage
		// Unlocking records for era 90
		assert_eq!(
			bifrost_vtoken_minting::TimeUnitUnlockLedger::<Runtime>::get(TimeUnit::Era(90), KSM,),
			Some(time_record_90)
		);
		assert_eq!(
			bifrost_vtoken_minting::UserUnlockLedger::<Runtime>::get(BOB, KSM,),
			Some((10, bounded_vec_bob.clone()))
		);

		// Unlocking records for era 100
		let bounded_vec_100_new = BoundedVec::try_from(vec![2]).unwrap();
		let time_record_100_new = (8, bounded_vec_100_new, KSM);
		let record_dave_new = (DAVE, 8, TimeUnit::Era(100));
		assert_eq!(
			bifrost_vtoken_minting::TokenUnlockLedger::<Runtime>::get(KSM, 2),
			Some(record_dave_new.clone())
		);

		assert_eq!(
			bifrost_vtoken_minting::TokenUnlockLedger::<Runtime>::get(KSM, 2),
			Some(record_dave_new)
		);

		assert_eq!(
			bifrost_vtoken_minting::TimeUnitUnlockLedger::<Runtime>::get(TimeUnit::Era(100), KSM,),
			Some(time_record_100_new)
		);

		assert_eq!(bifrost_vtoken_minting::UserUnlockLedger::<Runtime>::get(CHARLIE, KSM,), None);
		assert_eq!(
			bifrost_vtoken_minting::UserUnlockLedger::<Runtime>::get(DAVE, KSM,),
			Some((8, bounded_vec_dave.clone()))
		);

		// Unlocking records for era 110
		assert_eq!(
			bifrost_vtoken_minting::TimeUnitUnlockLedger::<Runtime>::get(TimeUnit::Era(110), KSM,),
			Some(time_record_110)
		);

		assert_eq!(
			bifrost_vtoken_minting::UserUnlockLedger::<Runtime>::get(EDDIE, KSM,),
			Some((13, bounded_vec_eddie.clone()))
		);

		// Set some more balance to exit account.
		assert_ok!(Tokens::set_balance(Origin::root(), exit_acc.clone(), KSM, 30, 0));

		// set era to 110
		bifrost_vtoken_minting::OngoingTimeUnit::<Runtime>::insert(KSM, TimeUnit::Era(110));

		// Refund user
		assert_ok!(Slp::refund_currency_due_unbond(Origin::signed(ALICE), KSM));

		// Check storages
		assert_eq!(
			bifrost_vtoken_minting::TimeUnitUnlockLedger::<Runtime>::get(TimeUnit::Era(110), KSM,),
			None
		);

		assert_eq!(bifrost_vtoken_minting::TokenUnlockLedger::<Runtime>::get(KSM, 3), None);
		assert_eq!(bifrost_vtoken_minting::TokenUnlockLedger::<Runtime>::get(KSM, 4), None);

		assert_eq!(bifrost_vtoken_minting::UserUnlockLedger::<Runtime>::get(EDDIE, KSM,), None);

		// check account balances
		assert_eq!(Tokens::free_balance(KSM, &exit_acc), 0);
		assert_eq!(Tokens::free_balance(KSM, &entrance_acc), 17);
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);
		assert_eq!(Tokens::free_balance(KSM, &CHARLIE), 28);
		assert_eq!(Tokens::free_balance(KSM, &DAVE), 22);
		assert_eq!(Tokens::free_balance(KSM, &EDDIE), 13);
	});
}

#[test]
fn increase_token_to_add_works() {}

#[test]
fn decrease_token_to_add_works() {}

#[test]
fn increase_token_to_deduct_works() {}

#[test]
fn decrease_token_to_deduct_works() {}

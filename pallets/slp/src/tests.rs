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
fn decrease_token_pool_works() {}

#[test]
fn update_ongoing_time_unit_works() {}

#[test]
fn refund_currency_due_unbond_works() {}

#[test]
fn move_fund_from_exit_to_entrance_account_works() {}

#[test]
fn increase_token_to_add_works() {}

#[test]
fn decrease_token_to_add_works() {}

#[test]
fn increase_token_to_deduct_works() {}

#[test]
fn decrease_token_to_deduct_works() {}

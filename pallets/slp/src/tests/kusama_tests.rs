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

#![cfg(test)]

use crate::{mocks::mock_kusama::*, *};
use bifrost_primitives::currency::{KSM, VKSM};
use bifrost_vtoken_minting::{OngoingTimeUnit, TokenPool};
use frame_support::{assert_ok, PalletId};
use orml_traits::MultiCurrency;
use sp_runtime::traits::AccountIdConversion;
use xcm::v3::prelude::*;

const SUBACCOUNT_0_32: [u8; 32] =
	hex_literal::hex!["5a53736d8e96f1c007cf0d630acf5209b20611617af23ce924c8e25328eb5d28"];
const SUBACCOUNT_0_LOCATION: MultiLocation =
	MultiLocation { parents: 1, interior: X1(AccountId32 { network: None, id: SUBACCOUNT_0_32 }) };

#[test]
fn construct_xcm_and_send_as_subaccount_should_work() {
	let para_chain_account_right: AccountId =
		hex_literal::hex!["70617261d1070000000000000000000000000000000000000000000000000000"]
			.into();
	let para_chain_account: AccountId = ParaId::from(2001).into_account_truncating();
	assert_eq!(para_chain_account_right, para_chain_account);

	let sub_account_id_right: AccountId =
		hex_literal::hex!["5a53736d8e96f1c007cf0d630acf5209b20611617af23ce924c8e25328eb5d28"]
			.into();
	let sub_account_id = SubAccountIndexMultiLocationConvertor::derivative_account_id(
		para_chain_account.clone(),
		0u16,
	);
	assert_eq!(sub_account_id_right, sub_account_id);

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
			RuntimeOrigin::signed(ALICE),
			KSM,
			Some((alice_location, 1_000_000_000_000))
		));
		assert_eq!(FeeSources::<Runtime>::get(KSM), Some((alice_location, 1_000_000_000_000)));
	});
}

#[test]
fn remove_delegator_works() {
	ExtBuilder::default().build().execute_with(|| {
		let para_chain_account: AccountId = ParaId::from(2001).into_account_truncating();
		let subaccount_0: AccountId = Utility::derivative_account_id(para_chain_account, 0);
		let subaccount_0_32: [u8; 32] = Slp::account_id_to_account_32(subaccount_0).unwrap();
		let subaccount_0_location: MultiLocation =
			Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

		DelegatorsIndex2Multilocation::<Runtime>::insert(KSM, 0, subaccount_0_location);
		DelegatorsMultilocation2Index::<Runtime>::insert(KSM, subaccount_0_location, 0);

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 100_000_000_000,
			bond_extra_minimum: 0,
			unbond_minimum: 0,
			rebond_minimum: 0,
			unbond_record_maximum: 32,
			validators_back_maximum: 36,
			delegator_active_staking_maximum: 200_000_000_000_000,
			validators_reward_maximum: 0,
			delegation_amount_minimum: 0,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		MinimumsAndMaximums::<Runtime>::insert(KSM, mins_and_maxs);

		let sb_ledger = SubstrateLedger {
			account: subaccount_0_location,
			total: 0,
			active: 0,
			unlocking: vec![],
		};
		let ledger = Ledger::Substrate(sb_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(KSM, subaccount_0_location, ledger);

		assert_ok!(Slp::remove_delegator(
			RuntimeOrigin::signed(ALICE),
			KSM,
			Box::new(subaccount_0_location)
		));

		assert_eq!(DelegatorsIndex2Multilocation::<Runtime>::get(KSM, 0), None);
		assert_eq!(DelegatorsMultilocation2Index::<Runtime>::get(KSM, subaccount_0_location), None);
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
		assert_ok!(Slp::decrease_token_pool(RuntimeOrigin::signed(ALICE), KSM, 10));

		// Check the value after decreasing
		assert_eq!(TokenPool::<Runtime>::get(KSM), 90);
	});
}

#[test]
fn update_ongoing_time_unit_works() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		// Set the era to be 8.
		bifrost_vtoken_minting::OngoingTimeUnit::<Runtime>::insert(KSM, TimeUnit::Era(8));
		assert_ok!(Slp::set_ongoing_time_unit_update_interval(
			RuntimeOrigin::signed(ALICE),
			KSM,
			Some(600)
		));

		System::set_block_number(650);

		// Update the era to be 9.
		assert_ok!(Slp::update_ongoing_time_unit(
			RuntimeOrigin::signed(ALICE),
			KSM,
			TimeUnit::Era(9)
		));

		// Check the value after updating.
		assert_eq!(OngoingTimeUnit::<Runtime>::get(KSM), Some(TimeUnit::Era(9)));
	});
}

#[test]
fn charge_host_fee_and_tune_vtoken_exchange_rate_works() {
	let para_chain_account: AccountId = ParaId::from(2001).into_account_truncating();
	let subaccount_0: AccountId = Utility::derivative_account_id(para_chain_account, 0);
	let subaccount_0_32: [u8; 32] = Slp::account_id_to_account_32(subaccount_0).unwrap();
	let subaccount_0_location: MultiLocation =
		Slp::account_32_to_parent_location(subaccount_0_32).unwrap();

	ExtBuilder::default().build().execute_with(|| {
		let treasury_id: AccountId = PalletId(*b"bf/trsry").into_account_truncating();
		let treasury_32: [u8; 32] = treasury_id.clone().into();

		bifrost_vtoken_minting::OngoingTimeUnit::<Runtime>::insert(KSM, TimeUnit::Era(1));

		DelegatorsIndex2Multilocation::<Runtime>::insert(KSM, 0, subaccount_0_location);
		DelegatorsMultilocation2Index::<Runtime>::insert(KSM, subaccount_0_location, 0);

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 100_000_000_000,
			bond_extra_minimum: 0,
			unbond_minimum: 0,
			rebond_minimum: 0,
			unbond_record_maximum: 32,
			validators_back_maximum: 36,
			delegator_active_staking_maximum: 200_000_000_000_000,
			validators_reward_maximum: 0,
			delegation_amount_minimum: 0,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		MinimumsAndMaximums::<Runtime>::insert(KSM, mins_and_maxs);

		let sb_ledger = SubstrateLedger {
			account: subaccount_0_location,
			total: 0,
			active: 0,
			unlocking: vec![],
		};
		let ledger = Ledger::Substrate(sb_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(KSM, subaccount_0_location, ledger);

		// Set the hosting fee to be 20%, and the beneficiary to be bifrost treasury account.
		let pct = Permill::from_percent(20);
		let treasury_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: treasury_32 }),
		};

		assert_ok!(Slp::set_hosting_fees(
			RuntimeOrigin::signed(ALICE),
			KSM,
			Some((pct, treasury_location))
		));

		let pct_100 = Permill::from_percent(100);
		assert_ok!(Slp::set_currency_tune_exchange_rate_limit(
			RuntimeOrigin::signed(ALICE),
			KSM,
			Some((1, pct_100))
		));

		// First set base vtoken exchange rate. Should be 1:1.
		assert_ok!(Currencies::deposit(VKSM, &ALICE, 100));
		assert_ok!(Slp::increase_token_pool(RuntimeOrigin::signed(ALICE), KSM, 100));

		// call the charge_host_fee_and_tune_vtoken_exchange_rate
		assert_ok!(Slp::charge_host_fee_and_tune_vtoken_exchange_rate(
			RuntimeOrigin::signed(ALICE),
			KSM,
			100,
			Some(subaccount_0_location)
		));

		// Tokenpool should have been added 100.
		let new_token_pool_amount = <Runtime as Config>::VtokenMinting::get_token_pool(KSM);
		assert_eq!(new_token_pool_amount, 200);

		let tune_record = DelegatorLatestTuneRecord::<Runtime>::get(KSM, &subaccount_0_location);
		assert_eq!(tune_record, Some(TimeUnit::Era(1)));

		// Treasury account has been issued a fee of 20 vksm which equals to the value of 20 ksm
		// before new exchange rate tuned.
		let treasury_vksm = Currencies::free_balance(VKSM, &treasury_id);
		assert_eq!(treasury_vksm, 20);
	});
}

#[test]
fn set_hosting_fees_works() {
	ExtBuilder::default().build().execute_with(|| {
		let treasury_32: [u8; 32] = PalletId(*b"bf/trsry").into_account_truncating();

		// Set the hosting fee to be 20%, and the beneficiary to be bifrost treasury account.
		let pct = Permill::from_percent(20);
		let treasury_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: treasury_32 }),
		};

		assert_ok!(Slp::set_hosting_fees(
			RuntimeOrigin::signed(ALICE),
			KSM,
			Some((pct, treasury_location))
		));

		// let (fee, location) = Slp::get_hosting_fee(KSM).unwrap();
		let (fee, location) = HostingFees::<Runtime>::get(KSM).unwrap();
		assert_eq!(fee, pct);
		assert_eq!(location, treasury_location);
	});
}

// test for DOT
#[test]
fn bond_works() {
	ExtBuilder::default().build().execute_with(|| {
		register_subaccount_index_0();

		let bob_32 = Pallet::<Runtime>::account_id_to_account_32(BOB).unwrap();
		let bob_location = Pallet::<Runtime>::account_32_to_local_location(bob_32).unwrap();

		// set fee source to Bob
		assert_ok!(Slp::set_fee_source(
			RuntimeOrigin::signed(ALICE),
			DOT,
			Some((bob_location, 1_000_000_000_000))
		));

		// deposit some DOT to Bob, for transfer fee burning
		assert_ok!(Currencies::deposit(DOT, &BOB, 2_000_000_000_000));

		// Bond 1 ksm for sub-account index 0
		assert_ok!(Slp::bond(
			RuntimeOrigin::signed(ALICE),
			DOT,
			Box::new(SUBACCOUNT_0_LOCATION),
			1_000_000_000_000,
			None,
			None
		));
	});
}

// Preparation: register sub-account index 0.
fn register_subaccount_index_0() {
	// Set OngoingTimeUnitUpdateInterval as 1/3 Era(1800 blocks per Era, 12 seconds per
	// block)
	assert_ok!(Slp::set_ongoing_time_unit_update_interval(
		RuntimeOrigin::signed(ALICE),
		DOT,
		Some(600)
	));

	System::set_block_number(600);

	// Initialize ongoing timeunit as 0.
	assert_ok!(Slp::update_ongoing_time_unit(RuntimeOrigin::signed(ALICE), DOT, TimeUnit::Era(0)));

	// Initialize currency delays.
	let delay =
		Delays { unlock_delay: TimeUnit::Era(10), leave_delegators_delay: Default::default() };
	assert_ok!(Slp::set_currency_delays(RuntimeOrigin::signed(ALICE), DOT, Some(delay)));

	let mins_and_maxs = MinimumsMaximums {
		delegator_bonded_minimum: 100_000_000_000,
		bond_extra_minimum: 0,
		unbond_minimum: 0,
		rebond_minimum: 0,
		unbond_record_maximum: 32,
		validators_back_maximum: 36,
		delegator_active_staking_maximum: 200_000_000_000_000,
		validators_reward_maximum: 0,
		delegation_amount_minimum: 0,
		delegators_maximum: 100,
		validators_maximum: 300,
	};

	// Set minimums and maximums
	assert_ok!(Slp::set_minimums_and_maximums(
		RuntimeOrigin::signed(ALICE),
		DOT,
		Some(mins_and_maxs)
	));

	// First to setup index-multilocation relationship of subaccount_0
	assert_ok!(Slp::add_delegator(
		RuntimeOrigin::signed(ALICE),
		DOT,
		0u16,
		Box::new(SUBACCOUNT_0_LOCATION),
	));

	// Register Operation weight and fee
	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		DOT,
		XcmOperationType::TransferTo,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		DOT,
		XcmOperationType::Bond,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		DOT,
		XcmOperationType::BondExtra,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		DOT,
		XcmOperationType::Unbond,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		DOT,
		XcmOperationType::Rebond,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		DOT,
		XcmOperationType::Delegate,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		DOT,
		XcmOperationType::Payout,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		DOT,
		XcmOperationType::Liquidize,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		DOT,
		XcmOperationType::Chill,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		DOT,
		XcmOperationType::TransferBack,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));
}

#[test]
fn test_construct_xcm() {
	ExtBuilder::default().build().execute_with(|| {
		register_subaccount_index_0();

		// construct_xcm_as_subaccount_with_query_id
		let _weight = Weight::from_parts(20000000000, 20000000000);
		let (_, _, _, _messsage) =
			crate::Pallet::<Runtime>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Bond,
				sp_std::vec![],
				&SUBACCOUNT_0_LOCATION,
				DOT,
				None,
			)
			.unwrap();

		let weight = Weight::from_parts(100, 100);
		let (_, _, _, _messsage) =
			crate::Pallet::<Runtime>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Bond,
				sp_std::vec![],
				&SUBACCOUNT_0_LOCATION,
				DOT,
				Some((weight, 100u32.into())),
			)
			.unwrap();

		// construct_xcm_and_send_as_subaccount_without_query_id
		let fee = crate::Pallet::<Runtime>::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperationType::Bond,
			sp_std::vec![],
			&SUBACCOUNT_0_LOCATION,
			DOT,
			None,
		)
		.unwrap();

		assert_eq!(fee, BalanceOf::<Runtime>::from(10000000000u128));

		let fee = crate::Pallet::<Runtime>::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperationType::Bond,
			sp_std::vec![],
			&SUBACCOUNT_0_LOCATION,
			DOT,
			Some((Weight::from_parts(100, 100), 100u32.into())),
		)
		.unwrap();
		assert_eq!(fee, BalanceOf::<Runtime>::from(100u32));
	});
}

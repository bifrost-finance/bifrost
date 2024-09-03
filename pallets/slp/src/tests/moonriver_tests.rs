// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) None later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT None WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg(test)]

use crate::{
	mocks::mock::*,
	primitives::{
		OneToManyDelegationAction, OneToManyDelegatorStatus, OneToManyLedger,
		OneToManyScheduledRequest, ParachainStakingLedgerUpdateEntry,
		ParachainStakingLedgerUpdateOperation,
	},
	Junction::Parachain,
	Junctions::X2,
	*,
};
use bifrost_primitives::{currency::VMOVR, Balance};
use frame_support::{assert_noop, assert_ok, PalletId};
use parity_scale_codec::alloc::collections::BTreeMap;
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::traits::AccountIdConversion;

const VALIDATOR_0_ACCOUNT_ID_20: [u8; 20] =
	hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"];
const VALIDATOR_0_LOCATION: MultiLocation = MultiLocation {
	parents: 1,
	interior: X2(
		Parachain(2023),
		Junction::AccountKey20 { network: None, key: VALIDATOR_0_ACCOUNT_ID_20 },
	),
};

const VALIDATOR_1_ACCOUNT_ID_20: [u8; 20] =
	hex_literal::hex!["f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"];
const VALIDATOR_1_LOCATION: MultiLocation = MultiLocation {
	parents: 1,
	interior: X2(
		Parachain(2023),
		Junction::AccountKey20 { network: None, key: VALIDATOR_1_ACCOUNT_ID_20 },
	),
};

#[test]
fn initialize_moonriver_delegator() {
	ExtBuilder::default().build().execute_with(|| {
		let bifrost_parachain_account_id_20_right: [u8; 20] =
			hex_literal::hex!["7369626cd1070000000000000000000000000000"].into();
		let bifrost_parachain_account_id_20: [u8; 20] =
			Sibling::from(2001).into_account_truncating();
		assert_eq!(bifrost_parachain_account_id_20_right, bifrost_parachain_account_id_20);

		// subaccount_id_0: 0x863c1faef3c3b8f8735ecb7f8ed18996356dd3de
		let subaccount_id_0_right: [u8; 20] =
			hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();
		let subaccount_id_0 = Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0);
		assert_eq!(subaccount_id_0_right.as_slice(), subaccount_id_0.0);

		// subaccount_id_1: 0x3afe20b0c85801b74e65586fe7070df827172574
		let subaccount_id_1_right: [u8; 20] =
			hex_literal::hex!["3afe20b0c85801b74e65586fe7070df827172574"].into();
		let subaccount_id_1 = Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 1);
		assert_eq!(subaccount_id_1_right.as_slice(), subaccount_id_1.0);

		let subaccount0_location = MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(2023),
				Junction::AccountKey20 { network: None, key: subaccount_id_0.0 },
			),
		};

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 100_000_000_000,
			bond_extra_minimum: 100_000_000_000,
			unbond_minimum: 100_000_000_000,
			rebond_minimum: 100_000_000_000,
			unbond_record_maximum: 1,
			validators_back_maximum: 100,
			delegator_active_staking_maximum: 200_000_000_000_000_000_000,
			validators_reward_maximum: 300,
			delegation_amount_minimum: 500_000_000,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		assert_ok!(Slp::set_minimums_and_maximums(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Some(mins_and_maxs)
		));

		assert_ok!(Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), MOVR, None));
		assert_eq!(DelegatorNextIndex::<Runtime>::get(MOVR), 1);
		assert_eq!(
			DelegatorsIndex2Multilocation::<Runtime>::get(MOVR, 0),
			Some(subaccount0_location)
		);
		assert_eq!(
			DelegatorsMultilocation2Index::<Runtime>::get(MOVR, subaccount0_location),
			Some(0)
		);
	});
}

fn moonriver_setup() {
	let treasury_account_id_32: [u8; 32] = PalletId(*b"bf/trsry").into_account_truncating();
	let treasury_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: treasury_account_id_32 }),
	};

	// set operate_origins
	assert_ok!(Slp::set_operate_origin(RuntimeOrigin::signed(ALICE), MOVR, Some(ALICE)));

	// Set OngoingTimeUnitUpdateInterval as 1/3 round(600 blocks per round, 12 seconds per block)
	assert_ok!(Slp::set_ongoing_time_unit_update_interval(
		RuntimeOrigin::signed(ALICE),
		MOVR,
		Some(200)
	));

	System::set_block_number(300);

	// Initialize ongoing timeunit as 1.
	assert_ok!(Slp::update_ongoing_time_unit(
		RuntimeOrigin::signed(ALICE),
		MOVR,
		TimeUnit::Round(1)
	));

	// Initialize currency delays.
	let delay =
		Delays { unlock_delay: TimeUnit::Round(24), leave_delegators_delay: TimeUnit::Round(24) };
	assert_ok!(Slp::set_currency_delays(RuntimeOrigin::signed(ALICE), MOVR, Some(delay)));

	let mins_and_maxs = MinimumsMaximums {
		delegator_bonded_minimum: 100_000_000_000,
		bond_extra_minimum: 100_000_000_000,
		unbond_minimum: 100_000_000_000,
		rebond_minimum: 100_000_000_000,
		unbond_record_maximum: 1,
		validators_back_maximum: 100,
		delegator_active_staking_maximum: 200_000_000_000_000_000_000,
		validators_reward_maximum: 300,
		delegation_amount_minimum: 500_000_000,
		delegators_maximum: 100,
		validators_maximum: 300,
	};

	// Set minimums and maximums
	assert_ok!(Slp::set_minimums_and_maximums(
		RuntimeOrigin::signed(ALICE),
		MOVR,
		Some(mins_and_maxs)
	));

	// First to setup index-multilocation relationship of subaccount_0
	assert_ok!(Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), MOVR, None));

	// update some MOVR balance to treasury account
	assert_ok!(Tokens::set_balance(
		RuntimeOrigin::root(),
		treasury_account_id_32.into(),
		MOVR,
		1_000_000_000_000_000_000,
		0
	));

	// Set fee source
	assert_ok!(Slp::set_fee_source(
		RuntimeOrigin::signed(ALICE),
		MOVR,
		Some((treasury_location, 1_000_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::Bond,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::BondExtra,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::Unbond,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::Chill,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::Rebond,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::Undelegate,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::CancelLeave,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::Liquidize,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::ExecuteLeave,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::TransferBack,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::XtokensTransferBack,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		MOVR,
		XcmOperationType::TransferTo,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	// Set delegator ledger
	assert_ok!(Slp::add_validator(
		RuntimeOrigin::signed(ALICE),
		MOVR,
		Box::new(VALIDATOR_0_LOCATION),
	));

	// initialize delegator
}

#[test]
fn moonriver_bond_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location),
				5_000_000_000_000_000_000,
				Some(VALIDATOR_0_LOCATION),
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_bond_extra_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 5_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger);

		assert_noop!(
			Slp::bond_extra(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location),
				Some(VALIDATOR_0_LOCATION),
				5_000_000_000_000_000_000,
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_unbond_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 8_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 8_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger);

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location),
				Some(VALIDATOR_0_LOCATION),
				2_000_000_000_000_000_000,
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_rebond_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 8_000_000_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: VALIDATOR_0_LOCATION,
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(VALIDATOR_0_LOCATION, (TimeUnit::Round(24), 2_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 8_000_000_000_000_000_000,
			less_total: 2_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger);

		assert_noop!(
			Slp::rebond(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location),
				Some(VALIDATOR_0_LOCATION),
				None,
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_undelegate_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 5_000_000_000_000_000_000);
		delegation_set.insert(VALIDATOR_1_LOCATION, 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger);

		assert_noop!(
			Slp::undelegate(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location),
				vec![VALIDATOR_0_LOCATION],
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_liquidize_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 10_000_000_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: VALIDATOR_0_LOCATION,
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000_000_000),
		};

		let mut request_list = Vec::new();
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(VALIDATOR_0_LOCATION, (TimeUnit::Round(24), 2_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000_000_000,
			less_total: 2_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger);

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location),
				None,
				Some(VALIDATOR_0_LOCATION),
				None,
				None
			),
			Error::<Runtime>::RequestNotDue
		);

		System::set_block_number(500);

		assert_ok!(Slp::update_ongoing_time_unit(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			TimeUnit::Round(24)
		));

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location),
				None,
				Some(VALIDATOR_0_LOCATION),
				None,
				None
			),
			Error::<Runtime>::XcmFailure
		);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 10_000_000_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: VALIDATOR_0_LOCATION,
			when_executable: TimeUnit::Round(50),
			action: OneToManyDelegationAction::Revoke(10_000_000_000_000_000_000),
		};

		let mut request_list = Vec::new();
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(VALIDATOR_0_LOCATION, (TimeUnit::Round(50), 10_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000_000_000,
			less_total: 10_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Leaving(TimeUnit::Round(48)),
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger);

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location),
				None,
				Some(VALIDATOR_0_LOCATION),
				None,
				None
			),
			Error::<Runtime>::LeavingNotDue
		);

		System::set_block_number(700);

		assert_ok!(Slp::update_ongoing_time_unit(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			TimeUnit::Round(48)
		));

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location),
				None,
				Some(VALIDATOR_0_LOCATION),
				None,
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_bond_and_bond_extra_confirm_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();
	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		// set empty ledger
		let empty_delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		let old_request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		let old_ledger = OneToManyLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			total: Zero::zero(),
			less_total: Zero::zero(),
			delegations: empty_delegation_set,
			requests: vec![],
			request_briefs: old_request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};
		let movr_ledger = Ledger::<BalanceOf<Runtime>>::ParachainStaking(old_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, movr_ledger);

		// Bond confirm
		// setup updateEntry
		let query_id = 0;
		let update_entry = LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location,
			validator_id: Some(VALIDATOR_0_LOCATION),
			update_operation: ParachainStakingLedgerUpdateOperation::Bond,
			amount: 5_000_000_000_000_000_000,
			unlock_time: None,
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry, 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 5_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location), Some(ledger));

		// BondExtra confirm
		let query_id = 1;
		let update_entry_1 =
			LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
				currency_id: MOVR,
				delegator_id: subaccount_0_location,
				validator_id: Some(VALIDATOR_0_LOCATION),
				update_operation: ParachainStakingLedgerUpdateOperation::Bond,
				amount: 5_000_000_000_000_000_000,
				unlock_time: None,
			});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_1.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_1, 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 10_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location), Some(ledger));
	});
}

#[test]
fn moonriver_unbond_confirm_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 10_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger.clone());

		assert_eq!(DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location), Some(ledger));

		// Unbond confirm
		let query_id = 2;
		let update_entry_2 =
			LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
				currency_id: MOVR,
				delegator_id: subaccount_0_location,
				validator_id: Some(VALIDATOR_0_LOCATION),
				update_operation: ParachainStakingLedgerUpdateOperation::BondLess,
				amount: 2_000_000_000_000_000_000,
				unlock_time: Some(TimeUnit::Round(24)),
			});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_2.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_2, 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 10_000_000_000_000_000_000);
		let request = OneToManyScheduledRequest {
			validator: VALIDATOR_0_LOCATION,
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(VALIDATOR_0_LOCATION, (TimeUnit::Round(24), 2_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000_000_000,
			less_total: 2_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location), Some(ledger));

		// Unbond confirm
		let query_id = 3;
		let update_entry_3 =
			LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
				currency_id: MOVR,
				delegator_id: subaccount_0_location,
				validator_id: Some(VALIDATOR_0_LOCATION),
				update_operation: ParachainStakingLedgerUpdateOperation::ExecuteRequest,
				amount: 0,
				unlock_time: Some(TimeUnit::Round(0)),
			});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_3.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_3.clone(), 1000))
		);

		assert_noop!(
			Slp::confirm_delegator_ledger_query_response(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				query_id
			),
			Error::<Runtime>::RequestNotDue
		);

		assert_ok!(Slp::fail_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			query_id
		),);

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		System::set_block_number(500);

		// Not working because time is not right.
		assert_ok!(Slp::update_ongoing_time_unit(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			TimeUnit::Round(24)
		));

		let query_id = 4;
		let update_entry_4 =
			LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
				currency_id: MOVR,
				delegator_id: subaccount_0_location,
				validator_id: Some(VALIDATOR_0_LOCATION),
				update_operation: ParachainStakingLedgerUpdateOperation::ExecuteRequest,
				amount: 0,
				unlock_time: Some(TimeUnit::Round(24)),
			});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_4.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_4.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 8_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 8_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location), Some(ledger));
	});
}

#[test]
fn moonriver_unbond_all_confirm_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		// unbond_all confirm
		// schedule leave
		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 8_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 8_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Leaving(TimeUnit::Round(48)),
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger.clone());

		assert_eq!(DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location), Some(ledger));

		let query_id = 5;
		let update_entry_5 =
			LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
				currency_id: MOVR,
				delegator_id: subaccount_0_location,
				validator_id: None,
				update_operation: ParachainStakingLedgerUpdateOperation::ExecuteLeave,
				amount: 0,
				unlock_time: Some(TimeUnit::Round(24)),
			});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_5.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_5.clone(), 1000))
		);

		assert_noop!(
			Slp::confirm_delegator_ledger_query_response(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				query_id
			),
			Error::<Runtime>::LeavingNotDue
		);

		assert_ok!(Slp::fail_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			query_id
		),);

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		System::set_block_number(500);

		// Not working because time is not right.
		assert_ok!(Slp::update_ongoing_time_unit(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			TimeUnit::Round(48)
		));

		let query_id = 6;
		let update_entry_6 =
			LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
				currency_id: MOVR,
				delegator_id: subaccount_0_location,
				validator_id: Some(VALIDATOR_0_LOCATION),
				update_operation: ParachainStakingLedgerUpdateOperation::ExecuteLeave,
				amount: 0,
				unlock_time: Some(TimeUnit::Round(48)),
			});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_6.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_6.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let empty_delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		let new_ledger = OneToManyLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			total: Zero::zero(),
			less_total: Zero::zero(),
			delegations: empty_delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};
		let movr_ledger = Ledger::<BalanceOf<Runtime>>::ParachainStaking(new_ledger);

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location),
			Some(movr_ledger)
		);
	});
}

#[test]
fn moonriver_rebond_confirm_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		// confirm rebond
		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 10_000_000_000_000_000_000);
		let request = OneToManyScheduledRequest {
			validator: VALIDATOR_0_LOCATION,
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(VALIDATOR_0_LOCATION, (TimeUnit::Round(24), 2_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000_000_000,
			less_total: 2_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger.clone());

		assert_eq!(DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location), Some(ledger));

		let query_id = 7;
		let update_entry_7 =
			LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
				currency_id: MOVR,
				delegator_id: subaccount_0_location,
				validator_id: Some(VALIDATOR_0_LOCATION),
				update_operation: ParachainStakingLedgerUpdateOperation::CancelRequest,
				amount: 0,
				unlock_time: Some(TimeUnit::Round(48)),
			});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_7.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_7.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 10_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location), Some(ledger));
	});
}

#[test]
fn moonriver_undelegate_confirm_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		// undelegate confirm
		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 5_000_000_000_000_000_000);
		delegation_set.insert(VALIDATOR_1_LOCATION, 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger);

		let query_id = 8;
		let update_entry_8 =
			LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
				currency_id: MOVR,
				delegator_id: subaccount_0_location,
				validator_id: Some(VALIDATOR_0_LOCATION),
				update_operation: ParachainStakingLedgerUpdateOperation::Revoke,
				amount: 0,
				unlock_time: Some(TimeUnit::Round(24)),
			});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_8.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_8.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 5_000_000_000_000_000_000);
		delegation_set.insert(VALIDATOR_1_LOCATION, 5_000_000_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: VALIDATOR_0_LOCATION,
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::<Balance>::Revoke(5_000_000_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(VALIDATOR_0_LOCATION, (TimeUnit::Round(24), 5_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000_000_000,
			less_total: 5_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location), Some(ledger));

		// execute revoke confirm
		let query_id = 9;
		let update_entry_9 =
			LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
				currency_id: MOVR,
				delegator_id: subaccount_0_location,
				validator_id: Some(VALIDATOR_0_LOCATION),
				update_operation: ParachainStakingLedgerUpdateOperation::ExecuteRequest,
				amount: 0,
				unlock_time: Some(TimeUnit::Round(21)),
			});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_9.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_9.clone(), 1000))
		);

		assert_noop!(
			Slp::confirm_delegator_ledger_query_response(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				query_id
			),
			Error::<Runtime>::RequestNotDue
		);

		let query_id = 10;
		let update_entry_10 =
			LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
				currency_id: MOVR,
				delegator_id: subaccount_0_location,
				validator_id: Some(VALIDATOR_0_LOCATION),
				update_operation: ParachainStakingLedgerUpdateOperation::ExecuteRequest,
				amount: 0,
				unlock_time: Some(TimeUnit::Round(24)),
			});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_10.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_10.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_1_LOCATION, 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 5_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location), Some(ledger));
	});
}

#[test]
fn moonriver_transfer_back_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		let exit_account_id_32: [u8; 32] = PalletId(*b"bf/vtout").into_account_truncating();

		let exit_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: None, id: exit_account_id_32 }),
		};

		assert_noop!(
			Slp::transfer_back(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location),
				Box::new(exit_account_location),
				5_000_000_000_000_000_000,
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_transfer_to_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		let entrance_account_id_32: [u8; 32] = PalletId(*b"bf/vtkin").into_account_truncating();

		let entrance_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: None, id: entrance_account_id_32 }),
		};

		assert_noop!(
			Slp::transfer_to(
				RuntimeOrigin::signed(ALICE),
				MOVR,
				Box::new(entrance_account_location),
				Box::new(subaccount_0_location),
				5_000_000_000_000_000_000,
			),
			Error::<Runtime>::TransferToError
		);
	});
}

#[test]
fn charge_host_fee_and_tune_vtoken_exchange_rate_works() {
	let bifrost_parachain_account_id_20: [u8; 20] = Sibling::from(2001).into_account_truncating();

	let subaccount_0_account_id_20: [u8; 20] =
		Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: None, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		let treasury_id: AccountId = PalletId(*b"bf/trsry").into_account_truncating();
		let treasury_32: [u8; 32] = treasury_id.clone().into();

		// moonriver_setup();

		bifrost_vtoken_minting::OngoingTimeUnit::<Runtime>::insert(MOVR, TimeUnit::Round(1));

		DelegatorsIndex2Multilocation::<Runtime>::insert(MOVR, 0, subaccount_0_location);
		DelegatorsMultilocation2Index::<Runtime>::insert(MOVR, subaccount_0_location, 0);

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 5_000_000_000_000_000_000,
			bond_extra_minimum: 0,
			unbond_minimum: 0,
			rebond_minimum: 0,
			unbond_record_maximum: 32,
			validators_back_maximum: 100,
			delegator_active_staking_maximum: 200_000_000_000_000_000_000,
			validators_reward_maximum: 300,
			delegation_amount_minimum: 5_000_000_000_000_000_000,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		MinimumsAndMaximums::<Runtime>::insert(MOVR, mins_and_maxs);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(VALIDATOR_0_LOCATION, 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 5_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location, ledger);

		// Set the hosting fee to be 20%, and the beneficiary to be bifrost treasury account.
		let pct = Permill::from_percent(20);
		let treasury_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: treasury_32 }),
		};

		assert_ok!(Slp::set_hosting_fees(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Some((pct, treasury_location))
		));

		let pct_100 = Permill::from_percent(100);
		assert_ok!(Slp::set_currency_tune_exchange_rate_limit(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Some((1, pct_100))
		));

		// First set base vtoken exchange rate. Should be 1:1.
		assert_ok!(Currencies::deposit(VMOVR, &ALICE, 100));
		assert_ok!(Slp::increase_token_pool(RuntimeOrigin::signed(ALICE), MOVR, 100));

		// call the charge_host_fee_and_tune_vtoken_exchange_rate
		assert_ok!(Slp::charge_host_fee_and_tune_vtoken_exchange_rate(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			100,
			Some(subaccount_0_location)
		));

		// Tokenpool should have been added 100.
		let new_token_pool_amount = <Runtime as Config>::VtokenMinting::get_token_pool(MOVR);
		assert_eq!(new_token_pool_amount, 200);

		// let tune_record = DelegatorLatestTuneRecord::<Runtime>::get(MOVR,
		// &subaccount_0_location); assert_eq!(tune_record, (1, Some(TimeUnit::Round(1))));

		let tune_record = CurrencyLatestTuneRecord::<Runtime>::get(MOVR);
		assert_eq!(tune_record, Some((TimeUnit::Round(1), 1)));

		// Treasury account has been issued a fee of 20 vksm which equals to the value of 20 ksm
		// before new exchange rate tuned.
		let treasury_vmovr = Currencies::free_balance(VMOVR, &treasury_id);
		assert_eq!(treasury_vmovr, 20);
	});
}

#[test]
fn add_validator_and_remove_validator_works() {
	ExtBuilder::default().build().execute_with(|| {
		let mut valis = vec![];

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 100_000_000_000,
			bond_extra_minimum: 100_000_000_000,
			unbond_minimum: 100_000_000_000,
			rebond_minimum: 100_000_000_000,
			unbond_record_maximum: 1,
			validators_back_maximum: 100,
			delegator_active_staking_maximum: 200_000_000_000_000_000_000,
			validators_reward_maximum: 300,
			delegation_amount_minimum: 500_000_000,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		assert_ok!(Slp::set_minimums_and_maximums(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Some(mins_and_maxs)
		));

		// Set delegator ledger
		assert_ok!(Slp::add_validator(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Box::new(VALIDATOR_0_LOCATION),
		));

		// The storage is reordered by hash. So we need to adjust the push order here.
		valis.push(VALIDATOR_0_LOCATION);

		let bounded_valis = BoundedVec::try_from(valis).unwrap();

		assert_eq!(Validators::<Runtime>::get(MOVR), Some(bounded_valis));

		assert_ok!(Slp::remove_validator(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Box::new(VALIDATOR_0_LOCATION),
		));

		let empty_bounded_vec = BoundedVec::default();
		assert_eq!(Validators::<Runtime>::get(MOVR), Some(empty_bounded_vec));
	});
}

#[test]
fn reset_validators_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		moonriver_setup();

		let validator_list_empty = vec![];
		let validator_list_input =
			vec![VALIDATOR_0_LOCATION, VALIDATOR_0_LOCATION, VALIDATOR_1_LOCATION];
		let validator_list_output =
			BoundedVec::try_from(vec![VALIDATOR_0_LOCATION, VALIDATOR_1_LOCATION]).unwrap();

		// validator list is empty
		assert_noop!(
			Slp::reset_validators(RuntimeOrigin::signed(ALICE), MOVR, validator_list_empty),
			Error::<Runtime>::ValidatorNotProvided
		);

		assert_ok!(Slp::reset_validators(RuntimeOrigin::signed(ALICE), MOVR, validator_list_input));

		assert_eq!(Validators::<Runtime>::get(MOVR), Some(validator_list_output));
	});
}

#[test]
fn set_validator_boost_list_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		moonriver_setup();

		let validator_list_empty = vec![];
		let validator_list_input_1 = vec![VALIDATOR_0_LOCATION];
		let validator_list_input_2 =
			vec![VALIDATOR_0_LOCATION, VALIDATOR_0_LOCATION, VALIDATOR_1_LOCATION];

		let validator_list_output_1 =
			BoundedVec::try_from(vec![(VALIDATOR_0_LOCATION, SIX_MONTHS as u64 + 300)]).unwrap();
		let validator_list_output_2 = BoundedVec::try_from(vec![
			(VALIDATOR_0_LOCATION, SIX_MONTHS as u64 + 400),
			(VALIDATOR_1_LOCATION, SIX_MONTHS as u64 + 400),
		])
		.unwrap();

		// validator list is empty
		assert_noop!(
			Slp::set_validator_boost_list(RuntimeOrigin::signed(ALICE), MOVR, validator_list_empty),
			Error::<Runtime>::ValidatorNotProvided
		);

		assert_ok!(Slp::set_validator_boost_list(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			validator_list_input_1
		));

		let bounded_validator_list_output_1 =
			BoundedVec::try_from(validator_list_output_1).unwrap();
		assert_eq!(ValidatorBoostList::<Runtime>::get(MOVR), Some(bounded_validator_list_output_1));
		let bounded_validator_0 = BoundedVec::try_from(vec![VALIDATOR_0_LOCATION]).unwrap();
		assert_eq!(Validators::<Runtime>::get(MOVR), Some(bounded_validator_0));

		System::set_block_number(400);

		assert_ok!(Slp::set_validator_boost_list(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			validator_list_input_2
		));

		let bounded_validator_list_output_2 =
			BoundedVec::try_from(validator_list_output_2).unwrap();
		assert_eq!(ValidatorBoostList::<Runtime>::get(MOVR), Some(bounded_validator_list_output_2));
		let bounded_validator_0_1 =
			BoundedVec::try_from(vec![VALIDATOR_0_LOCATION, VALIDATOR_1_LOCATION]).unwrap();
		assert_eq!(Validators::<Runtime>::get(MOVR), Some(bounded_validator_0_1),);
	});
}

#[test]
fn add_to_validator_boost_list_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		moonriver_setup();

		let validator_list_output_1 =
			BoundedVec::try_from(vec![(VALIDATOR_0_LOCATION, SIX_MONTHS as u64 + 300)]).unwrap();
		let validator_list_output_2 = BoundedVec::try_from(vec![(
			VALIDATOR_0_LOCATION,
			SIX_MONTHS as u64 + 300 + SIX_MONTHS as u64,
		)])
		.unwrap();
		let validator_list_output_3 = BoundedVec::try_from(vec![
			(VALIDATOR_0_LOCATION, SIX_MONTHS as u64 + 300 + SIX_MONTHS as u64),
			(VALIDATOR_1_LOCATION, SIX_MONTHS as u64 + 400),
		])
		.unwrap();

		assert_ok!(Slp::add_to_validator_boost_list(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Box::new(VALIDATOR_0_LOCATION)
		));

		assert_eq!(ValidatorBoostList::<Runtime>::get(MOVR), Some(validator_list_output_1));

		let bounded_validator_0 = BoundedVec::try_from(vec![VALIDATOR_0_LOCATION]).unwrap();
		assert_eq!(Validators::<Runtime>::get(MOVR), Some(bounded_validator_0.clone()));

		System::set_block_number(400);

		assert_ok!(Slp::add_to_validator_boost_list(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Box::new(VALIDATOR_0_LOCATION)
		));

		assert_eq!(Validators::<Runtime>::get(MOVR), Some(bounded_validator_0));

		assert_eq!(ValidatorBoostList::<Runtime>::get(MOVR), Some(validator_list_output_2));

		assert_ok!(Slp::add_to_validator_boost_list(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Box::new(VALIDATOR_1_LOCATION)
		));

		assert_eq!(ValidatorBoostList::<Runtime>::get(MOVR), Some(validator_list_output_3));
		let bounded_validator_0_1 =
			BoundedVec::try_from(vec![VALIDATOR_0_LOCATION, VALIDATOR_1_LOCATION]).unwrap();
		assert_eq!(Validators::<Runtime>::get(MOVR), Some(bounded_validator_0_1),);
	});
}

#[test]
fn remove_from_validator_boost_list_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		moonriver_setup();

		let validator_list_output =
			BoundedVec::try_from(vec![(VALIDATOR_0_LOCATION, SIX_MONTHS as u64 + 300)]).unwrap();

		assert_ok!(Slp::add_to_validator_boost_list(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Box::new(VALIDATOR_0_LOCATION)
		));

		assert_eq!(ValidatorBoostList::<Runtime>::get(MOVR), Some(validator_list_output.clone()));

		assert_ok!(Slp::remove_from_validator_boot_list(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Box::new(VALIDATOR_1_LOCATION)
		));

		assert_eq!(ValidatorBoostList::<Runtime>::get(MOVR), Some(validator_list_output));

		assert_ok!(Slp::remove_from_validator_boot_list(
			RuntimeOrigin::signed(ALICE),
			MOVR,
			Box::new(VALIDATOR_0_LOCATION)
		));

		assert_eq!(ValidatorBoostList::<Runtime>::get(MOVR), None);
	});
}

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

use crate::{
	mock::*,
	primitives::{
		MoonriverLedgerUpdateEntry, OneToManyDelegationAction, OneToManyScheduledRequest,
	},
	Junction::Parachain,
	Junctions::X2,
};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::AccountIdConversion;
use xcm::opaque::latest::NetworkId::Any;

use crate::{
	primitives::{OneToManyDelegatorStatus, OneToManyLedger},
	MOVR, *,
};
use codec::alloc::collections::BTreeMap;
use node_primitives::Balance;

#[test]
fn initialize_moonriver_delegator() {
	ExtBuilder::default().build().execute_with(|| {
		// let bifrost_parachain_account_id_20: [u8; 20] =
		// 	hex_literal::hex!["7369626cd1070000000000000000000000000000"].into();
		let bifrost_parachain_account_id_20: [u8; 20] =
			<Runtime as frame_system::Config>::AccountId::encode(
				&ParaId::from(2001u32).into_account(),
			)
			.as_slice()[..20]
				.try_into()
				.unwrap();

		// subaccount_id_0: 0x863c1faef3c3b8f8735ecb7f8ed18996356dd3de
		let subaccount_id_0 = Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 0);
		println!("subaccount_id_0: {:?}", subaccount_id_0);

		// subaccount_id_1: 0x3afe20b0c85801b74e65586fe7070df827172574
		let subaccount_id_1 = Slp::derivative_account_id_20(bifrost_parachain_account_id_20, 1);
		println!("subaccountId1: {:?}", subaccount_id_1);

		let subaccount0_location = MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(2023),
				Junction::AccountKey20 {
					network: Any,
					key: [
						134, 60, 31, 174, 243, 195, 184, 248, 115, 94, 203, 127, 142, 209, 137,
						150, 53, 109, 211, 222,
					],
				},
			),
		};

		assert_ok!(Slp::initialize_delegator(Origin::signed(ALICE), MOVR,));
		assert_eq!(DelegatorNextIndex::<Runtime>::get(MOVR), 1);
		assert_eq!(
			DelegatorsIndex2Multilocation::<Runtime>::get(MOVR, 0),
			Some(subaccount0_location.clone())
		);
		assert_eq!(
			DelegatorsMultilocation2Index::<Runtime>::get(MOVR, subaccount0_location),
			Some(0)
		);
	});
}

fn moonriver_setup() {
	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	// set operate_origins
	assert_ok!(Slp::set_operate_origin(Origin::signed(ALICE), MOVR, Some(ALICE)));

	// Set OngoingTimeUnitUpdateInterval as 1/3 round(600 blocks per round, 12 seconds per block)
	assert_ok!(Slp::set_ongoing_time_unit_update_interval(Origin::signed(ALICE), MOVR, Some(200)));

	System::set_block_number(300);

	// Initialize ongoing timeunit as 1.
	assert_ok!(Slp::update_ongoing_time_unit(Origin::signed(ALICE), MOVR, TimeUnit::Round(1)));

	// Initialize currency delays.
	let delay =
		Delays { unlock_delay: TimeUnit::Round(24), leave_delegators_delay: TimeUnit::Round(24) };
	assert_ok!(Slp::set_currency_delays(Origin::signed(ALICE), MOVR, Some(delay)));

	// First to setup index-multilocation relationship of subaccount_0
	assert_ok!(Slp::initialize_delegator(Origin::signed(ALICE), MOVR,));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::Bond,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::BondExtra,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::Unbond,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::Chill,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::Rebond,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::Undelegate,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::CancelLeave,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::Liquidize,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::ExecuteLeave,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::TransferBack,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::XtokensTransferBack,
		Some((20_000_000_000, 10_000_000_000)),
	));

	assert_ok!(Slp::set_xcm_dest_weight_and_fee(
		Origin::signed(ALICE),
		MOVR,
		XcmOperation::TransferTo,
		Some((20_000_000_000, 10_000_000_000)),
	));

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
	};

	// Set minimums and maximums
	assert_ok!(Slp::set_minimums_and_maximums(Origin::signed(ALICE), MOVR, Some(mins_and_maxs)));

	// Set delegator ledger
	assert_ok!(Slp::add_validator(Origin::signed(ALICE), MOVR, validator_0_location.clone(),));

	// initialize delegator
}

#[test]
fn moonriver_bond_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		assert_noop!(
			Slp::bond(
				Origin::signed(ALICE),
				MOVR,
				subaccount_0_location.clone(),
				5_000_000_000_000_000_000,
				Some(validator_0_location.clone())
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_bond_extra_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 5_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::bond_extra(
				Origin::signed(ALICE),
				MOVR,
				subaccount_0_location,
				Some(validator_0_location),
				5_000_000_000_000_000_000,
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_unbond_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::unbond(
				Origin::signed(ALICE),
				MOVR,
				subaccount_0_location,
				Some(validator_0_location),
				2_000_000_000_000_000_000,
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_unbond_all_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::unbond_all(Origin::signed(ALICE), MOVR, subaccount_0_location,),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_rebond_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(24), 2_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000_000_000,
			less_total: 2_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::rebond(
				Origin::signed(ALICE),
				MOVR,
				subaccount_0_location,
				Some(validator_0_location.clone()),
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_undelegate_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	let validator_1_account_id_20: [u8; 20] =
		hex_literal::hex!["f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"].into();

	let validator_1_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_1_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 5_000_000_000_000_000_000);
		delegation_set.insert(validator_1_location.clone(), 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::undelegate(
				Origin::signed(ALICE),
				MOVR,
				subaccount_0_location,
				vec![validator_0_location],
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_redelegate_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Revoke(8_000_000_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(24), 8_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000_000_000,
			less_total: 8_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Leaving(TimeUnit::Round(24)),
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::redelegate(Origin::signed(ALICE), MOVR, subaccount_0_location, None),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_liquidize_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 10_000_000_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000_000_000),
		};

		let mut request_list = Vec::new();
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(24), 2_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000_000_000,
			less_total: 2_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::liquidize(
				Origin::signed(ALICE),
				MOVR,
				subaccount_0_location.clone(),
				None,
				Some(validator_0_location.clone())
			),
			Error::<Runtime>::RequestNotDue
		);

		System::set_block_number(500);

		assert_ok!(Slp::update_ongoing_time_unit(Origin::signed(ALICE), MOVR, TimeUnit::Round(24)));

		assert_noop!(
			Slp::liquidize(
				Origin::signed(ALICE),
				MOVR,
				subaccount_0_location.clone(),
				None,
				Some(validator_0_location.clone())
			),
			Error::<Runtime>::XcmFailure
		);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 10_000_000_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(50),
			action: OneToManyDelegationAction::Revoke(10_000_000_000_000_000_000),
		};

		let mut request_list = Vec::new();
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set.insert(
			validator_0_location.clone(),
			(TimeUnit::Round(50), 10_000_000_000_000_000_000),
		);

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000_000_000,
			less_total: 10_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Leaving(TimeUnit::Round(48)),
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::liquidize(
				Origin::signed(ALICE),
				MOVR,
				subaccount_0_location.clone(),
				None,
				Some(validator_0_location.clone())
			),
			Error::<Runtime>::LeavingNotDue
		);

		System::set_block_number(700);

		assert_ok!(Slp::update_ongoing_time_unit(Origin::signed(ALICE), MOVR, TimeUnit::Round(48)));

		assert_noop!(
			Slp::liquidize(
				Origin::signed(ALICE),
				MOVR,
				subaccount_0_location,
				None,
				Some(validator_0_location)
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_bond_and_bond_extra_confirm_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		// set empty ledger
		let empty_delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		let old_request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		let old_ledger = OneToManyLedger::<MultiLocation, MultiLocation, BalanceOf<Runtime>> {
			account: subaccount_0_location.clone(),
			total: Zero::zero(),
			less_total: Zero::zero(),
			delegations: empty_delegation_set,
			requests: vec![],
			request_briefs: old_request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};
		let movr_ledger =
			Ledger::<MultiLocation, BalanceOf<Runtime>, MultiLocation>::Moonriver(old_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), movr_ledger);

		// Bond confirm
		// setup updateEntry
		let query_id = 0;
		let update_entry = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: Some(validator_0_location.clone()),
			if_bond: true,
			if_unlock: false,
			if_revoke: false,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: false,
			amount: 5_000_000_000_000_000_000,
			unlock_time: None,
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry, 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 5_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);

		// BondExtra confirm
		let query_id = 1;
		let update_entry_1 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: Some(validator_0_location.clone()),
			if_bond: true,
			if_unlock: false,
			if_revoke: false,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: false,
			amount: 5_000_000_000_000_000_000,
			unlock_time: None,
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_1.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_1, 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 10_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);
	});
}

#[test]
fn moonriver_unbond_confirm_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 10_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger.clone());

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);

		// Unbond confirm
		let query_id = 2;
		let update_entry_2 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: Some(validator_0_location.clone()),
			if_bond: false,
			if_unlock: true,
			if_revoke: false,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: false,
			amount: 2_000_000_000_000_000_000,
			unlock_time: Some(TimeUnit::Round(24)),
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_2.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_2, 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 10_000_000_000_000_000_000);
		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(24), 2_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000_000_000,
			less_total: 2_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);

		// Unbond confirm
		let query_id = 3;
		let update_entry_3 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: Some(validator_0_location.clone()),
			if_bond: false,
			if_unlock: false,
			if_revoke: false,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: false,
			amount: 0,
			unlock_time: Some(TimeUnit::Round(0)),
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_3.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_3.clone(), 1000))
		);

		assert_noop!(
			Slp::confirm_delegator_ledger_query_response(Origin::signed(ALICE), MOVR, query_id),
			Error::<Runtime>::RequestNotDue
		);

		assert_ok!(Slp::fail_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		),);

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		System::set_block_number(500);

		// Not working because time is not right.
		assert_ok!(Slp::update_ongoing_time_unit(Origin::signed(ALICE), MOVR, TimeUnit::Round(24)));

		let query_id = 4;
		let update_entry_4 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: Some(validator_0_location.clone()),
			if_bond: false,
			if_unlock: false,
			if_revoke: false,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: false,
			amount: 0,
			unlock_time: Some(TimeUnit::Round(24)),
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_4.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_4.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);
	});
}

#[test]
fn moonriver_unbond_all_confirm_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		// unbond_all confirm
		// schedule leave
		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Leaving(TimeUnit::Round(48)),
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger.clone());

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);

		let query_id = 5;
		let update_entry_5 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: None,
			if_bond: false,
			if_unlock: false,
			if_revoke: false,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: true,
			amount: 0,
			unlock_time: Some(TimeUnit::Round(24)),
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_5.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_5.clone(), 1000))
		);

		assert_noop!(
			Slp::confirm_delegator_ledger_query_response(Origin::signed(ALICE), MOVR, query_id),
			Error::<Runtime>::LeavingNotDue
		);

		assert_ok!(Slp::fail_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		),);

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		System::set_block_number(500);

		// Not working because time is not right.
		assert_ok!(Slp::update_ongoing_time_unit(Origin::signed(ALICE), MOVR, TimeUnit::Round(48)));

		let query_id = 6;
		let update_entry_6 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: Some(validator_0_location.clone()),
			if_bond: false,
			if_unlock: false,
			if_revoke: false,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: true,
			amount: 0,
			unlock_time: Some(TimeUnit::Round(48)),
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_6.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_6.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let empty_delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		let new_ledger = OneToManyLedger::<MultiLocation, MultiLocation, BalanceOf<Runtime>> {
			account: subaccount_0_location.clone(),
			total: Zero::zero(),
			less_total: Zero::zero(),
			delegations: empty_delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};
		let movr_ledger =
			Ledger::<MultiLocation, BalanceOf<Runtime>, MultiLocation>::Moonriver(new_ledger);

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(movr_ledger)
		);
	});
}

#[test]
fn moonriver_rebond_confirm_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		// confirm rebond
		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 10_000_000_000_000_000_000);
		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(24), 2_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000_000_000,
			less_total: 2_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger.clone());

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);

		let query_id = 7;
		let update_entry_7 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: Some(validator_0_location.clone()),
			if_bond: false,
			if_unlock: false,
			if_revoke: false,
			if_cancel: true,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: false,
			amount: 0,
			unlock_time: Some(TimeUnit::Round(48)),
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_7.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_7.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 10_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);
	});
}

#[test]
fn moonriver_undelegate_confirm_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	let validator_1_account_id_20: [u8; 20] =
		hex_literal::hex!["f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"].into();

	let validator_1_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_1_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		// undelegate confirm
		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 5_000_000_000_000_000_000);
		delegation_set.insert(validator_1_location.clone(), 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger);

		let query_id = 8;
		let update_entry_8 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: Some(validator_0_location.clone()),
			if_bond: false,
			if_unlock: false,
			if_revoke: true,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: false,
			amount: 0,
			unlock_time: Some(TimeUnit::Round(24)),
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_8.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_8.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 5_000_000_000_000_000_000);
		delegation_set.insert(validator_1_location.clone(), 5_000_000_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::<Balance>::Revoke(5_000_000_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(24), 5_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000_000_000,
			less_total: 5_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);

		// execute revoke confirm
		let query_id = 9;
		let update_entry_9 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: Some(validator_0_location.clone()),
			if_bond: false,
			if_unlock: false,
			if_revoke: false,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: false,
			amount: 0,
			unlock_time: Some(TimeUnit::Round(21)),
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_9.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_9.clone(), 1000))
		);

		assert_noop!(
			Slp::confirm_delegator_ledger_query_response(Origin::signed(ALICE), MOVR, query_id),
			Error::<Runtime>::RequestNotDue
		);

		let query_id = 10;
		let update_entry_10 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: Some(validator_0_location.clone()),
			if_bond: false,
			if_unlock: false,
			if_revoke: false,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: false,
			if_execute_leave: false,
			amount: 0,
			unlock_time: Some(TimeUnit::Round(24)),
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_10.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_10.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_1_location.clone(), 5_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 5_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);
	});
}

#[test]
fn moonriver_redelegate_confirm_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	let validator_0_account_id_20: [u8; 20] =
		hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();

	let validator_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: validator_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Revoke(8_000_000_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(24), 8_000_000_000_000_000_000));

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000_000_000,
			less_total: 8_000_000_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Leaving(TimeUnit::Round(24)),
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(MOVR, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::redelegate(Origin::signed(ALICE), MOVR, subaccount_0_location.clone(), None),
			Error::<Runtime>::XcmFailure
		);

		let query_id = 8;
		let update_entry_8 = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: None,
			if_bond: false,
			if_unlock: false,
			if_revoke: false,
			if_cancel: false,
			if_leave: false,
			if_cancel_leave: true,
			if_execute_leave: false,
			amount: 0,
			unlock_time: None,
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(query_id, (update_entry_8.clone(), 1000));

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((update_entry_8.clone(), 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			Origin::signed(ALICE),
			MOVR,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let moonriver_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonriver(moonriver_ledger);

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(MOVR, subaccount_0_location.clone()),
			Some(ledger)
		);
	});
}

#[test]
fn moonriver_transfer_back_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		let exit_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746f75740000000000000000000000000000000000000000"]
				.into();

		let exit_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: Any, id: exit_account_id_32 }),
		};

		assert_noop!(
			Slp::transfer_back(
				Origin::signed(ALICE),
				MOVR,
				Box::new(subaccount_0_location.clone()),
				Box::new(exit_account_location.clone()),
				5_000_000_000_000_000_000,
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn moonriver_transfer_to_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		let entrance_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746b696e0000000000000000000000000000000000000000"]
				.into();

		let entrance_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: Any, id: entrance_account_id_32 }),
		};

		assert_noop!(
			Slp::transfer_to(
				Origin::signed(ALICE),
				MOVR,
				Box::new(entrance_account_location.clone()),
				Box::new(subaccount_0_location.clone()),
				5_000_000_000_000_000_000,
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn supplement_fee_account_whitelist_works() {
	let subaccount_0_account_id_20: [u8; 20] =
		hex_literal::hex!["863c1faef3c3b8f8735ecb7f8ed18996356dd3de"].into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2023),
			Junction::AccountKey20 { network: Any, key: subaccount_0_account_id_20 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		moonriver_setup();
		let entrance_account_id: AccountId =
			hex_literal::hex!["6d6f646c62662f76746b696e0000000000000000000000000000000000000000"]
				.into();

		let entrance_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746b696e0000000000000000000000000000000000000000"]
				.into();

		let entrance_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: Any, id: entrance_account_id_32 }),
		};

		let exit_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746f75740000000000000000000000000000000000000000"]
				.into();

		let exit_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: Any, id: exit_account_id_32 }),
		};

		let source_account_id_32: [u8; 32] =
			hex_literal::hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"]
				.into();
		let source_location = Slp::account_32_to_local_location(source_account_id_32).unwrap();
		assert_ok!(Slp::set_fee_source(
			Origin::signed(ALICE),
			MOVR,
			Some((source_location.clone(), 1_000_000_000_000_000_000))
		));

		// Dest should be one of delegators, operateOrigins or accounts in the whitelist.
		assert_noop!(
			Slp::supplement_fee_reserve(Origin::signed(ALICE), MOVR, subaccount_0_location.clone(),),
			Error::<Runtime>::XcmFailure
		);

		assert_noop!(
			Slp::supplement_fee_reserve(
				Origin::signed(ALICE),
				MOVR,
				entrance_account_location.clone(),
			),
			Error::<Runtime>::DestAccountNotValid
		);

		// register entrance_account_location as operateOrigin
		assert_ok!(Slp::set_operate_origin(Origin::signed(ALICE), MOVR, Some(entrance_account_id)));

		assert_noop!(
			Slp::supplement_fee_reserve(
				Origin::signed(ALICE),
				MOVR,
				entrance_account_location.clone(),
			),
			Error::<Runtime>::XcmFailure
		);

		assert_noop!(
			Slp::supplement_fee_reserve(Origin::signed(ALICE), MOVR, exit_account_location.clone(),),
			Error::<Runtime>::DestAccountNotValid
		);

		// register exit_account_location into whitelist
		assert_ok!(Slp::add_supplement_fee_account_to_whitelist(
			Origin::signed(ALICE),
			MOVR,
			exit_account_location.clone(),
		));

		assert_noop!(
			Slp::supplement_fee_reserve(Origin::signed(ALICE), MOVR, exit_account_location.clone(),),
			Error::<Runtime>::XcmFailure
		);

		// remove exit_account_location from whitelist
		assert_ok!(Slp::remove_supplement_fee_account_from_whitelist(
			Origin::signed(ALICE),
			MOVR,
			exit_account_location.clone(),
		));

		assert_noop!(
			Slp::supplement_fee_reserve(Origin::signed(ALICE), MOVR, exit_account_location.clone(),),
			Error::<Runtime>::DestAccountNotValid
		);
	});
}

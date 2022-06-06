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

use crate::{mock::*, primitives::MoonriverLedgerUpdateEntry, Junction::Parachain, Junctions::X2};
use frame_support::{assert_noop, assert_ok};
use orml_traits::MultiCurrency;
use sp_runtime::traits::AccountIdConversion;
use xcm::opaque::latest::NetworkId::Any;

use crate::{
	primitives::{OneToManyDelegatorStatus, OneToManyLedger},
	MOVR, *,
};
use codec::alloc::collections::BTreeMap;

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

	// Initialize ongoing timeunit as 0.
	assert_ok!(Slp::update_ongoing_time_unit(Origin::signed(ALICE), MOVR, TimeUnit::Round(0)));

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

		// check updateEntry
		let update_entry = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: validator_0_location.clone(),
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
		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(0), Some((update_entry, 1000)));
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
fn moonriver_confirm_delegator_ledger_query_response_works() {
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
		// assert_ok!(Slp::bond(
		// 	Origin::signed(ALICE),
		// 	MOVR,
		// 	subaccount_0_location.clone(),
		// 	5_000_000_000_000_000_000,
		// 	Some(validator_0_location.clone())
		// ));

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

		// setup updateEntry
		let update_entry = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: subaccount_0_location.clone(),
			validator_id: validator_0_location.clone(),
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

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(0, (update_entry.clone(), 1000));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(0), Some((update_entry, 1000)));

		assert_ok!(Slp::confirm_delegator_ledger_query_response(Origin::signed(ALICE), MOVR, 0));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(0), None);

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
	});
}

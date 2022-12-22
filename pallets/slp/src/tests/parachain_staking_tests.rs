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
	mocks::mock::*,
	primitives::{OneToManyDelegationAction, OneToManyScheduledRequest},
};
use frame_support::{assert_noop, assert_ok};
use xcm::opaque::latest::NetworkId::Any;

use crate::{
	primitives::{OneToManyDelegatorStatus, OneToManyLedger},
	BNC, *,
};
use codec::alloc::collections::BTreeMap;

#[test]
fn initialize_parachain_staking_delegator() {
	ExtBuilder::default().build().execute_with(|| {
		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 100_000_000_000,
			bond_extra_minimum: 100_000_000_000,
			unbond_minimum: 100_000_000_000,
			rebond_minimum: 100_000_000_000,
			unbond_record_maximum: 1,
			validators_back_maximum: 100,
			delegator_active_staking_maximum: 200_000_000_000_000,
			validators_reward_maximum: 300,
			delegation_amount_minimum: 500_000_000,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		assert_ok!(Slp::set_minimums_and_maximums(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Some(mins_and_maxs)
		));

		assert_ok!(Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), BNC, None,));
		assert_eq!(DelegatorNextIndex::<Runtime>::get(BNC), 1);
	});
}

fn parachain_staking_setup() {
	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: BOB.into() }) };

	let treasury_account_id_32: [u8; 32] =
		hex_literal::hex!["6d6f646c62662f74727372790000000000000000000000000000000000000000"]
			.into();
	let treasury_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: Any, id: treasury_account_id_32 }),
	};

	// set operate_origins
	assert_ok!(Slp::set_operate_origin(RuntimeOrigin::signed(ALICE), BNC, Some(ALICE)));

	// Set OngoingTimeUnitUpdateInterval as 1/3 round(600 blocks per round, 12 seconds per block)
	assert_ok!(Slp::set_ongoing_time_unit_update_interval(
		RuntimeOrigin::signed(ALICE),
		BNC,
		Some(200)
	));

	System::set_block_number(300);

	// Initialize ongoing timeunit as 1.
	assert_ok!(Slp::update_ongoing_time_unit(
		RuntimeOrigin::signed(ALICE),
		BNC,
		TimeUnit::Round(1)
	));

	// Initialize currency delays.
	let delay =
		Delays { unlock_delay: TimeUnit::Round(24), leave_delegators_delay: TimeUnit::Round(24) };
	assert_ok!(Slp::set_currency_delays(RuntimeOrigin::signed(ALICE), BNC, Some(delay)));

	let mins_and_maxs = MinimumsMaximums {
		delegator_bonded_minimum: 100_000_000_000,
		bond_extra_minimum: 100_000_000_000,
		unbond_minimum: 100_000_000_000,
		rebond_minimum: 100_000_000_000,
		unbond_record_maximum: 1,
		validators_back_maximum: 100,
		delegator_active_staking_maximum: 200_000_000_000_000,
		validators_reward_maximum: 300,
		delegation_amount_minimum: 500_000_000,
		delegators_maximum: 100,
		validators_maximum: 300,
	};

	// Set minimums and maximums
	assert_ok!(Slp::set_minimums_and_maximums(
		RuntimeOrigin::signed(ALICE),
		BNC,
		Some(mins_and_maxs)
	));

	// First to setup index-multilocation relationship of subaccount_0
	assert_ok!(Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), BNC, None,));

	// update some BNC balance to treasury account
	assert_ok!(Currencies::update_balance(
		RuntimeOrigin::root(),
		treasury_account_id_32.into(),
		BNC,
		1_000_000_000_000_000_000,
	));

	// Set fee source
	assert_ok!(Slp::set_fee_source(
		RuntimeOrigin::signed(ALICE),
		BNC,
		Some((treasury_location, 1_000_000_000_000)),
	));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::Bond,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::BondExtra,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::Unbond,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::Chill,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::Rebond,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::Undelegate,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::CancelLeave,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::Liquidize,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::ExecuteLeave,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::TransferBack,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::XtokensTransferBack,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// assert_ok!(Slp::set_xcm_dest_weight_and_fee(
	// 	RuntimeOrigin::signed(ALICE),
	// 	BNC,
	// 	XcmOperation::TransferTo,
	// 	Some((20_000_000_000, 10_000_000_000)),
	// ));

	// Set delegator ledger
	assert_ok!(Slp::add_validator(
		RuntimeOrigin::signed(ALICE),
		BNC,
		Box::new(validator_0_location.clone()),
	));

	// initialize delegator
}

#[test]
fn parachain_staking_bond_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();
		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location.clone()),
				5_000_000_000_000,
				Some(validator_0_location.clone())
			),
			Error::<Runtime>::Unexpected
		);
	});
}

#[test]
fn parachain_staking_bond_extra_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 5_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 5_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonbeam(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::bond_extra(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				Some(validator_0_location),
				5_000_000_000_000,
			),
			Error::<Runtime>::DelegatorNotExist
		);
	});
}

#[test]
fn parachain_staking_unbond_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonbeam(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				Some(validator_0_location),
				2_000_000_000_000,
			),
			Error::<Runtime>::DelegatorNotExist
		);
	});
}

#[test]
fn parachain_staking_unbond_all_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonbeam(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::unbond_all(RuntimeOrigin::signed(ALICE), BNC, Box::new(subaccount_0_location),),
			Error::<Runtime>::DelegatorNotExist
		);
	});
}

#[test]
fn parachain_staking_rebond_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(24), 2_000_000_000_000));

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000,
			less_total: 2_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonbeam(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::rebond(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				Some(validator_0_location.clone()),
				None
			),
			Error::<Runtime>::DelegatorNotExist
		);
	});
}

#[test]
fn parachain_staking_undelegate_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: BOB.into() }) };

	let validator_1_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: DAVE.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 5_000_000_000_000);
		delegation_set.insert(validator_1_location.clone(), 5_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonbeam(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::undelegate(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				vec![validator_0_location],
			),
			Error::<Runtime>::DelegatorNotExist
		);
	});
}

#[test]
fn parachain_staking_redelegate_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 8_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Revoke(8_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(24), 8_000_000_000_000));

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 8_000_000_000_000,
			less_total: 8_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Leaving(TimeUnit::Round(24)),
		};

		let ledger = Ledger::Moonbeam(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, Box::new(subaccount_0_location.clone()), ledger);

		assert_noop!(
			Slp::redelegate(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				None
			),
			Error::<Runtime>::DelegatorNotExist
		);
	});
}

#[test]
fn parachain_staking_liquidize_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 10_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000),
		};

		let mut request_list = Vec::new();
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(24), 2_000_000_000_000));

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000,
			less_total: 2_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::Moonbeam(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location.clone()),
				None,
				Some(validator_0_location.clone())
			),
			Error::<Runtime>::DelegatorNotExist
		);

		System::set_block_number(500);

		assert_ok!(Slp::update_ongoing_time_unit(
			RuntimeOrigin::signed(ALICE),
			BNC,
			TimeUnit::Round(24)
		));

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location.clone()),
				None,
				Some(validator_0_location.clone())
			),
			Error::<Runtime>::DelegatorNotExist
		);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location.clone(), 10_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location.clone(),
			when_executable: TimeUnit::Round(50),
			action: OneToManyDelegationAction::Revoke(10_000_000_000_000),
		};

		let mut request_list = Vec::new();
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set
			.insert(validator_0_location.clone(), (TimeUnit::Round(50), 10_000_000_000_000));

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location.clone(),
			total: 10_000_000_000_000,
			less_total: 10_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Leaving(TimeUnit::Round(48)),
		};

		let ledger = Ledger::Moonbeam(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location.clone(), ledger);

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location.clone()),
				None,
				Some(validator_0_location.clone())
			),
			Error::<Runtime>::DelegatorNotExist
		);

		System::set_block_number(700);

		assert_ok!(Slp::update_ongoing_time_unit(
			RuntimeOrigin::signed(ALICE),
			BNC,
			TimeUnit::Round(48)
		));

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				None,
				Some(validator_0_location)
			),
			Error::<Runtime>::DelegatorNotExist
		);
	});
}

#[test]
fn parachain_staking_transfer_back_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();
		let exit_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746f75740000000000000000000000000000000000000000"]
				.into();

		let exit_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: Any, id: exit_account_id_32 }),
		};

		DelegatorsIndex2Multilocation::<Runtime>::insert(BNC, 0, subaccount_0_location.clone());
		DelegatorsMultilocation2Index::<Runtime>::insert(BNC, subaccount_0_location.clone(), 0);

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			CHARLIE,
			BNC,
			1000_000_000_000_000,
		));

		assert_ok!(Slp::transfer_back(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(subaccount_0_location.clone()),
			Box::new(exit_account_location.clone()),
			5_000_000_000_000,
		));
	});
}

#[test]
fn parachain_staking_transfer_to_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let entrance_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746b696e0000000000000000000000000000000000000000"]
				.into();

		let entrance_account_location = MultiLocation {
			parents: 0,
			interior: X1(Junction::AccountId32 { network: Any, id: entrance_account_id_32 }),
		};

		DelegatorsIndex2Multilocation::<Runtime>::insert(BNC, 0, subaccount_0_location.clone());
		DelegatorsMultilocation2Index::<Runtime>::insert(BNC, subaccount_0_location.clone(), 0);

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			entrance_account_id_32.into(),
			BNC,
			1000_000_000_000_000,
		));

		assert_ok!(Slp::transfer_to(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(entrance_account_location.clone()),
			Box::new(subaccount_0_location.clone()),
			5_000_000_000_000,
		));
	});
}

#[test]
fn supplement_fee_account_whitelist_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: Any, id: CHARLIE.into() }),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();
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
			RuntimeOrigin::signed(ALICE),
			BNC,
			Some((source_location.clone(), 1_000_000_000_000))
		));

		// Dest should be one of delegators, operateRuntimeOrigins or accounts in the whitelist.
		assert_noop!(
			Slp::supplement_fee_reserve(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location.clone()),
			),
			Error::<Runtime>::DestAccountNotValid
		);

		assert_noop!(
			Slp::supplement_fee_reserve(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(entrance_account_location.clone()),
			),
			Error::<Runtime>::DestAccountNotValid
		);

		// register entrance_account_location as operateRuntimeOrigin
		assert_ok!(Slp::set_operate_origin(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Some(entrance_account_id)
		));

		// assert_noop!(
		// 	Slp::supplement_fee_reserve(
		// 		RuntimeOrigin::signed(ALICE),
		// 		BNC,
		// 		Box::new(entrance_account_location.clone()),
		// 	),
		// 	Error::<Runtime>::XcmFailure
		// );

		assert_noop!(
			Slp::supplement_fee_reserve(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(exit_account_location.clone()),
			),
			Error::<Runtime>::DestAccountNotValid
		);

		// register exit_account_location into whitelist
		assert_ok!(Slp::add_supplement_fee_account_to_whitelist(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(exit_account_location.clone()),
		));

		// assert_noop!(
		// 	Slp::supplement_fee_reserve(
		// 		RuntimeOrigin::signed(ALICE),
		// 		BNC,
		// 		Box::new(exit_account_location.clone()),
		// 	),
		// 	Error::<Runtime>::XcmFailure
		// );

		// remove exit_account_location from whitelist
		assert_ok!(Slp::remove_supplement_fee_account_from_whitelist(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(exit_account_location.clone()),
		));

		assert_noop!(
			Slp::supplement_fee_reserve(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(exit_account_location.clone()),
			),
			Error::<Runtime>::DestAccountNotValid
		);
	});
}

#[test]
fn add_validator_and_remove_validator_works() {
	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: Any, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		let mut valis = vec![];
		let multi_hash_0 =
			<Runtime as frame_system::Config>::Hashing::hash(&validator_0_location.encode());

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 100_000_000_000,
			bond_extra_minimum: 100_000_000_000,
			unbond_minimum: 100_000_000_000,
			rebond_minimum: 100_000_000_000,
			unbond_record_maximum: 1,
			validators_back_maximum: 100,
			delegator_active_staking_maximum: 200_000_000_000_000,
			validators_reward_maximum: 300,
			delegation_amount_minimum: 500_000_000,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		assert_ok!(Slp::set_minimums_and_maximums(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Some(mins_and_maxs)
		));

		// Set delegator ledger
		assert_ok!(Slp::add_validator(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(validator_0_location.clone()),
		));

		// The storage is reordered by hash. So we need to adjust the push order here.
		valis.push((validator_0_location.clone(), multi_hash_0));

		assert_eq!(Slp::get_validators(BNC), Some(valis));

		assert_ok!(Slp::remove_validator(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(validator_0_location.clone()),
		));

		assert_eq!(Slp::get_validators(BNC), Some(vec![]));
	});
}

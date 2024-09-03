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
use crate::{
	mocks::mock::*,
	primitives::{
		OneToManyDelegationAction, OneToManyDelegatorStatus, OneToManyLedger,
		OneToManyScheduledRequest,
	},
	BNC, *,
};
use bifrost_parachain_staking::{Round, RoundInfo};
use bifrost_primitives::VBNC;
use frame_support::{assert_noop, assert_ok, PalletId};
use parity_scale_codec::alloc::collections::BTreeMap;
use sp_runtime::traits::AccountIdConversion;

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
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: BOB.into() }) };

	let treasury_account_id_32: [u8; 32] = PalletId(*b"bf/trsry").into_account_truncating();
	let treasury_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: treasury_account_id_32 }),
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

	// Set delegator ledger
	assert_ok!(Slp::add_validator(
		RuntimeOrigin::signed(ALICE),
		BNC,
		Box::new(validator_0_location),
	));

	// initialize delegator
}

#[test]
fn parachain_staking_bond_to_liquidize_works() {
	env_logger::try_init().unwrap_or(());

	let subaccount_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: ALICE.into() }) };

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: BOB.into() }) };

	ExtBuilder::default().init_for_alice_n_bob().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();
		initialize_parachain_staking_delegator();
		env_logger::try_init().unwrap_or(());

		DelegatorsIndex2Multilocation::<Runtime>::insert(BNC, 0, subaccount_0_location);
		DelegatorsMultilocation2Index::<Runtime>::insert(BNC, subaccount_0_location, 0);

		assert_ok!(ParachainStaking::join_candidates(
			RuntimeOrigin::signed(BOB),
			10_000_000_000_000u128,
			10_000_000u32
		));
		assert_ok!(ParachainStaking::join_candidates(
			RuntimeOrigin::signed(CHARLIE),
			10_000_000_000_000u128,
			10_000_000u32
		));

		let entrance_account_id_32: [u8; 32] = PalletId(*b"bf/vtkin").into_account_truncating();

		let entrance_account_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: entrance_account_id_32 }),
		};
		let entrance_account = AccountId::new(entrance_account_id_32);
		assert_eq!(Balances::free_balance(&entrance_account), 100000000000000);

		assert_ok!(Slp::transfer_to(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(entrance_account_location),
			Box::new(subaccount_0_location),
			5_000_000_000_000,
		));
		assert_ok!(Slp::bond(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(subaccount_0_location),
			5_000_000_000_000,
			Some(validator_0_location),
			None
		));
		assert_ok!(Slp::bond_extra(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(subaccount_0_location),
			Some(validator_0_location),
			5_000_000_000_000,
			None
		));
		assert_ok!(Slp::unbond(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(subaccount_0_location),
			Some(validator_0_location),
			2_000_000_000_000,
			None
		));

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location, 10_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location,
			when_executable: TimeUnit::Round(50),
			action: OneToManyDelegationAction::Revoke(10_000_000_000_000),
		};

		let mut request_list = Vec::new();

		// random account to test ordering
		let validator_10: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746b696e0000000000000000000000000000000000000000"]
				.into();
		let validator_10_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: validator_10 }),
		};
		let request10 = OneToManyScheduledRequest {
			validator: validator_10_location,
			when_executable: TimeUnit::Round(50),
			action: OneToManyDelegationAction::Revoke(10_000_000_000_000),
		};

		// random account to test ordering
		let validator_11: [u8; 32] =
			hex_literal::hex!["624d6a004c72a1abcf93131e185515ebe1410e43a301fe1f25d20d8da345376e"]
				.into();
		let validator_11_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: validator_11 }),
		};
		let request11 = OneToManyScheduledRequest {
			validator: validator_11_location,
			when_executable: TimeUnit::Round(50),
			action: OneToManyDelegationAction::Revoke(10_000_000_000_000),
		};
		request_list.push(request11);
		request_list.push(request10);
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set.insert(validator_0_location, (TimeUnit::Round(50), 10_000_000_000_000));
		// set delegator_0 ledger
		let parachain_staking_ledger2 = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000,
			less_total: 10_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};
		let ledger2 = Ledger::ParachainStaking(parachain_staking_ledger2);
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location, ledger2.clone());

		System::set_block_number(700_000);
		assert_ok!(Slp::update_ongoing_time_unit(
			RuntimeOrigin::signed(ALICE),
			BNC,
			TimeUnit::Round(48)
		));
		bifrost_parachain_staking::Round::<Runtime>::set(RoundInfo::new(10000000, 0, 1));
		assert_eq!(Round::<Runtime>::get(), RoundInfo::new(10000000, 0, 1));
		assert_ok!(VtokenMinting::update_ongoing_time_unit(BNC, TimeUnit::Round(1000)));

		// let delegation_scheduled_requests = ParachainStaking::delegation_scheduled_requests(BOB);
		// log::debug!("test5{:?}", delegation_scheduled_requests);

		assert_ok!(Slp::liquidize(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(subaccount_0_location),
			None,
			Some(validator_0_location),
			None,
			None
		));
	});
}

#[test]
fn parachain_staking_bond_extra_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location, 5_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 5_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location, ledger);

		assert_noop!(
			Slp::bond_extra(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				Some(validator_0_location),
				5_000_000_000_000,
				None
			),
			Error::<Runtime>::Unexpected
		);
	});
}

#[test]
fn parachain_staking_unbond_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location, 8_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 8_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location, ledger);

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				Some(validator_0_location),
				2_000_000_000_000,
				None
			),
			Error::<Runtime>::Unexpected
		);
	});
}

#[test]
fn parachain_staking_unbond_all_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location, 8_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 8_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location, ledger);

		assert_noop!(
			Slp::unbond_all(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				None
			),
			Error::<Runtime>::Unexpected
		);
	});
}

#[test]
fn parachain_staking_rebond_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location, 8_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location,
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set.insert(validator_0_location, (TimeUnit::Round(24), 2_000_000_000_000));

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 8_000_000_000_000,
			less_total: 2_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location, ledger);

		assert_noop!(
			Slp::rebond(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				Some(validator_0_location),
				None,
				None
			),
			Error::<Runtime>::Unexpected
		);
	});
}

#[test]
fn parachain_staking_undelegate_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: BOB.into() }) };

	let validator_1_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: DAVE.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location, 5_000_000_000_000);
		delegation_set.insert(validator_1_location, 5_000_000_000_000);
		let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000,
			less_total: 0,
			delegations: delegation_set,
			requests: vec![],
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location, ledger);

		assert_noop!(
			Slp::undelegate(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				vec![validator_0_location],
				None
			),
			Error::<Runtime>::Unexpected
		);
	});
}

#[test]
fn parachain_staking_redelegate_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location, 8_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location,
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Revoke(8_000_000_000_000),
		};
		let mut request_list = Vec::new();
		request_list.push(request);
		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set.insert(validator_0_location, (TimeUnit::Round(24), 8_000_000_000_000));

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 8_000_000_000_000,
			less_total: 8_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Leaving(TimeUnit::Round(24)),
		};

		let ledger = Ledger::ParachainStaking(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, Box::new(subaccount_0_location), ledger);

		assert_noop!(
			Slp::redelegate(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				None,
				None
			),
			Error::<Runtime>::Unexpected
		);
	});
}

#[test]
fn parachain_staking_liquidize_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: CHARLIE.into() }),
	};

	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location, 10_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location,
			when_executable: TimeUnit::Round(24),
			action: OneToManyDelegationAction::Decrease(2_000_000_000_000),
		};

		let mut request_list = Vec::new();
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set.insert(validator_0_location, (TimeUnit::Round(24), 2_000_000_000_000));

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000,
			less_total: 2_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};

		let ledger = Ledger::ParachainStaking(parachain_staking_ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location, ledger);

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				None,
				Some(validator_0_location),
				None,
				None
			),
			Error::<Runtime>::RequestNotDue
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
				Box::new(subaccount_0_location),
				None,
				Some(validator_0_location),
				None,
				None
			),
			Error::<Runtime>::Unexpected
		);

		let mut delegation_set: BTreeMap<MultiLocation, BalanceOf<Runtime>> = BTreeMap::new();
		delegation_set.insert(validator_0_location, 10_000_000_000_000);

		let request = OneToManyScheduledRequest {
			validator: validator_0_location,
			when_executable: TimeUnit::Round(50),
			action: OneToManyDelegationAction::Revoke(10_000_000_000_000),
		};

		let mut request_list = Vec::new();
		request_list.push(request);

		let mut request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<Runtime>)> =
			BTreeMap::new();
		request_briefs_set.insert(validator_0_location, (TimeUnit::Round(50), 10_000_000_000_000));

		assert_ok!(Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), BNC, None,));

		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				5_000_000_000_000,
				Some(validator_0_location),
				None
			),
			Error::<Runtime>::AlreadyBonded
		);

		// set delegator_0 ledger
		let parachain_staking_ledger = OneToManyLedger {
			account: subaccount_0_location,
			total: 10_000_000_000_000,
			less_total: 10_000_000_000_000,
			delegations: delegation_set,
			requests: request_list,
			request_briefs: request_briefs_set,
			status: OneToManyDelegatorStatus::Active,
		};
		let ledger = Ledger::ParachainStaking(parachain_staking_ledger);
		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(BNC, subaccount_0_location, ledger);

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				None,
				Some(validator_0_location),
				None,
				None
			),
			Error::<Runtime>::RequestNotDue
		);

		System::set_block_number(700);

		assert_ok!(Slp::update_ongoing_time_unit(
			RuntimeOrigin::signed(ALICE),
			BNC,
			TimeUnit::Round(1000)
		));

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				BNC,
				Box::new(subaccount_0_location),
				None,
				Some(validator_0_location),
				None,
				None
			),
			Error::<Runtime>::Unexpected
		);
	});
}

#[test]
fn parachain_staking_transfer_back_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: CHARLIE.into() }),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();
		let exit_account_id_32: [u8; 32] = PalletId(*b"bf/vtout").into_account_truncating();

		let exit_account_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: exit_account_id_32 }),
		};

		DelegatorsIndex2Multilocation::<Runtime>::insert(BNC, 0, subaccount_0_location);
		DelegatorsMultilocation2Index::<Runtime>::insert(BNC, subaccount_0_location, 0);

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			CHARLIE,
			BNC,
			1000_000_000_000_000,
		));

		assert_ok!(Slp::transfer_back(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(subaccount_0_location),
			Box::new(exit_account_location),
			5_000_000_000_000,
			None
		));
	});
}

#[test]
fn parachain_staking_transfer_to_works() {
	let subaccount_0_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: CHARLIE.into() }),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		let entrance_account_id_32: [u8; 32] = PalletId(*b"bf/vtkin").into_account_truncating();

		let entrance_account_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: entrance_account_id_32 }),
		};

		DelegatorsIndex2Multilocation::<Runtime>::insert(BNC, 0, subaccount_0_location);
		DelegatorsMultilocation2Index::<Runtime>::insert(BNC, subaccount_0_location, 0);

		assert_ok!(Currencies::update_balance(
			RuntimeOrigin::root(),
			entrance_account_id_32.into(),
			BNC,
			1000_000_000_000_000,
		));

		assert_ok!(Slp::transfer_to(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(entrance_account_location),
			Box::new(subaccount_0_location),
			5_000_000_000_000,
		));
	});
}

#[test]
fn add_validator_and_remove_validator_works() {
	let validator_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: BOB.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		let mut valis = vec![];

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
			Box::new(validator_0_location),
		));

		// The storage is reordered by hash. So we need to adjust the push order here.
		valis.push(validator_0_location);

		let bounded_valis = BoundedVec::try_from(valis).unwrap();
		assert_eq!(Validators::<Runtime>::get(BNC), Some(bounded_valis));

		assert_ok!(Slp::remove_validator(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Box::new(validator_0_location),
		));

		let empty_bounded_vec = BoundedVec::default();
		assert_eq!(Validators::<Runtime>::get(BNC), Some(empty_bounded_vec));
	});
}

#[test]
fn charge_host_fee_and_tune_vtoken_exchange_rate_works() {
	let subaccount_0_location =
		MultiLocation { parents: 0, interior: X1(AccountId32 { network: None, id: ALICE.into() }) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		parachain_staking_setup();

		// First set base vtoken exchange rate. Should be 1:1.
		assert_ok!(Currencies::deposit(VBNC, &ALICE, 1000));
		assert_ok!(Slp::increase_token_pool(RuntimeOrigin::signed(ALICE), BNC, 1000));

		// Set the hosting fee to be 20%, and the beneficiary to be bifrost treasury account.
		let pct = Permill::from_percent(20);
		let treasury_account_id_32: [u8; 32] = PalletId(*b"bf/trsry").into_account_truncating();
		let treasury_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: treasury_account_id_32 }),
		};

		assert_ok!(Slp::set_hosting_fees(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Some((pct, treasury_location))
		));

		let pct_100 = Permill::from_percent(100);
		assert_ok!(Slp::set_currency_tune_exchange_rate_limit(
			RuntimeOrigin::signed(ALICE),
			BNC,
			Some((1, pct_100))
		));

		// call the charge_host_fee_and_tune_vtoken_exchange_rate
		assert_ok!(Slp::charge_host_fee_and_tune_vtoken_exchange_rate(
			RuntimeOrigin::signed(ALICE),
			BNC,
			1000,
			Some(subaccount_0_location)
		));

		// check token pool, should be 1000 + 1000 = 2000
		assert_eq!(<Runtime as Config>::VtokenMinting::get_token_pool(BNC), 2000);
		// check vBNC issuance, should be 1000 + 20% * 1000 = 1200
		assert_eq!(Currencies::total_issuance(VBNC), 1200);
	});
}

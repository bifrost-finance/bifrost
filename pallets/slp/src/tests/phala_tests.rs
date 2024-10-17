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
	primitives::{PhalaLedger, SubstrateLedgerUpdateEntry, SubstrateLedgerUpdateOperation},
	Junction::{GeneralIndex, Parachain},
	Junctions::X2,
	*,
};
use bifrost_primitives::currency::{PHA, VPHA};
use frame_support::{assert_noop, assert_ok, PalletId};
use polkadot_parachain_primitives::primitives::Sibling;
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::AccountIdConversion;

// parents 0 means vault, parents 1 means stake_pool
const VALIDATOR_0_LOCATION: MultiLocation =
	MultiLocation { parents: 0, interior: X2(GeneralIndex(0), GeneralIndex(0)) };
const VALIDATOR_0_ACCOUNT_ID_32: [u8; 32] =
	hex_literal::hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
const VALIDATOR_0_LOCATION_WRONG: MultiLocation = MultiLocation {
	parents: 1,
	interior: X2(Parachain(2004), AccountId32 { network: None, id: VALIDATOR_0_ACCOUNT_ID_32 }),
};

const VALIDATOR_1_LOCATION: MultiLocation =
	MultiLocation { parents: 0, interior: X2(GeneralIndex(1), GeneralIndex(1)) };

#[test]
fn initialize_phala_delegator_works() {
	ExtBuilder::default().build().execute_with(|| {
		let bifrost_parachain_account_id_32_right: AccountId =
			// parachain_account: 43E7ZtPTcFQLEGnJCWmiNDoof4AKGukKFX47xA1VDJRtJ1ME
			hex_literal::hex!["7369626cd1070000000000000000000000000000000000000000000000000000"]
				.into();
		let bifrost_parachain_account_id_32: AccountId =
			Sibling::from(2001).into_account_truncating();
		assert_eq!(bifrost_parachain_account_id_32_right, bifrost_parachain_account_id_32);

		// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
		let subaccount_id_0_right: AccountId =
			hex_literal::hex!["290bf94235666a351d9c8082c77e689813a905d0bbffdbd8b4a619ec5303ba27"]
				.into();
		let subaccount_id_0 = SubAccountIndexMultiLocationConvertor::derivative_account_id(
			bifrost_parachain_account_id_32.clone(),
			0,
		);
		assert_eq!(subaccount_id_0_right, subaccount_id_0);

		// subaccount_id_1: 45AjrJZhmM7bateSg7f96BDu5915Rp3jP5zmC5yDr2cHTAw7
		let subaccount_id_1_right: AccountId =
			hex_literal::hex!["c94f02677ffb78dc23fbd3b95beb2650fe4fa5c466e5aedee74e89d96351800c"]
				.into();
		let subaccount_id_1 = SubAccountIndexMultiLocationConvertor::derivative_account_id(
			bifrost_parachain_account_id_32,
			1,
		);
		assert_eq!(subaccount_id_1_right, subaccount_id_1);

		let subaccount_0_location = MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(2004),
				AccountId32 { network: None, id: subaccount_id_0.into() },
			),
		};

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 1_000_000_000_000,
			bond_extra_minimum: 1_000_000_000_000,
			unbond_minimum: 1_000_000_000_000,
			rebond_minimum: 1_000_000_000_000,
			unbond_record_maximum: 1,
			validators_back_maximum: 1,
			delegator_active_staking_maximum: 1_000_000_000_000_000_000,
			validators_reward_maximum: 10_000,
			delegation_amount_minimum: 1_000_000_000_000,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		assert_ok!(Slp::set_minimums_and_maximums(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Some(mins_and_maxs)
		));

		assert_ok!(Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), PHA, None));
		assert_eq!(DelegatorNextIndex::<Runtime>::get(PHA), 1);
		assert_eq!(
			DelegatorsIndex2Multilocation::<Runtime>::get(PHA, 0),
			Some(subaccount_0_location)
		);
		assert_eq!(
			DelegatorsMultilocation2Index::<Runtime>::get(PHA, subaccount_0_location),
			Some(0)
		);
	});
}

#[test]
fn add_validator_works() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			Slp::add_validator(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(VALIDATOR_0_LOCATION_WRONG)
			),
			Error::<Runtime>::ValidatorMultilocationNotvalid
		);

		assert_noop!(
			Slp::add_validator(RuntimeOrigin::signed(ALICE), PHA, Box::new(VALIDATOR_0_LOCATION)),
			Error::<Runtime>::NotExist
		);

		let mins_and_maxs = MinimumsMaximums {
			delegator_bonded_minimum: 1_000_000_000_000,
			bond_extra_minimum: 1_000_000_000_000,
			unbond_minimum: 1_000_000_000_000,
			rebond_minimum: 1_000_000_000_000,
			unbond_record_maximum: 1,
			validators_back_maximum: 1,
			delegator_active_staking_maximum: 1_000_000_000_000_000_000,
			validators_reward_maximum: 10_000,
			delegation_amount_minimum: 1_000_000_000_000,
			delegators_maximum: 100,
			validators_maximum: 300,
		};

		// Set minimums and maximums
		assert_ok!(Slp::set_minimums_and_maximums(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Some(mins_and_maxs)
		));

		assert_ok!(Slp::add_validator(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(VALIDATOR_0_LOCATION)
		));
	});
}

#[test]
fn phala_delegate_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_id_0: AccountId =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0);

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(Parachain(2004), AccountId32 { network: None, id: subaccount_id_0.into() }),
	};

	ExtBuilder::default().build().execute_with(|| {
		initialize_preparation_setup();

		// delegate a validator
		assert_noop!(
			Slp::delegate(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				vec![VALIDATOR_0_LOCATION],
				None
			),
			Error::<Runtime>::DelegatorNotExist
		);

		assert_ok!(Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), PHA, None));

		assert_noop!(
			Slp::delegate(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				vec![],
				None
			),
			Error::<Runtime>::VectorEmpty
		);

		assert_noop!(
			Slp::delegate(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				vec![VALIDATOR_0_LOCATION_WRONG],
				None
			),
			Error::<Runtime>::ValidatorError
		);

		assert_noop!(
			Slp::delegate(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				vec![VALIDATOR_0_LOCATION],
				None
			),
			Error::<Runtime>::ValidatorSetNotExist
		);

		assert_ok!(Slp::add_validator(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(VALIDATOR_1_LOCATION)
		));

		assert_noop!(
			Slp::delegate(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				vec![VALIDATOR_0_LOCATION],
				None
			),
			Error::<Runtime>::ValidatorNotExist
		);

		assert_ok!(Slp::add_validator(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(VALIDATOR_0_LOCATION)
		));

		assert_ok!(Slp::delegate(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(subaccount_0_location),
			vec![VALIDATOR_0_LOCATION],
			None
		));

		let new_ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: Zero::zero(),
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		assert_eq!(
			DelegatorLedgers::<Runtime>::get(PHA, subaccount_0_location),
			Some(Ledger::<BalanceOf<Runtime>>::Phala(new_ledger))
		);
	});
}

fn initialize_preparation_setup() {
	// set operate_origins
	assert_ok!(Slp::set_operate_origin(RuntimeOrigin::signed(ALICE), PHA, Some(ALICE)));

	// Set OngoingTimeUnitUpdateInterval as 1/3 Hour(300 blocks per hour, 12 seconds per block)
	assert_ok!(Slp::set_ongoing_time_unit_update_interval(
		RuntimeOrigin::signed(ALICE),
		PHA,
		Some(100)
	));

	System::set_block_number(300);

	// Initialize ongoing timeunit as 1.
	assert_ok!(Slp::update_ongoing_time_unit(RuntimeOrigin::signed(ALICE), PHA, TimeUnit::Hour(1)));

	// Initialize currency delays.21 days = 21 *24 = 504 hours
	let delay =
		Delays { unlock_delay: TimeUnit::Hour(504), leave_delegators_delay: TimeUnit::Hour(504) };
	assert_ok!(Slp::set_currency_delays(RuntimeOrigin::signed(ALICE), PHA, Some(delay)));

	let mins_and_maxs = MinimumsMaximums {
		delegator_bonded_minimum: 1_000_000_000_000,
		bond_extra_minimum: 1_000_000_000_000,
		unbond_minimum: 1_000_000_000_000,
		rebond_minimum: 1_000_000_000_000,
		unbond_record_maximum: 1,
		validators_back_maximum: 1,
		delegator_active_staking_maximum: 1_000_000_000_000_000_000,
		validators_reward_maximum: 10_000,
		delegation_amount_minimum: 1_000_000_000_000,
		delegators_maximum: 100,
		validators_maximum: 300,
	};

	// Set minimums and maximums
	assert_ok!(Slp::set_minimums_and_maximums(
		RuntimeOrigin::signed(ALICE),
		PHA,
		Some(mins_and_maxs)
	));
}

fn phala_xcm_setup() {
	let treasury_account_id_32: [u8; 32] = PalletId(*b"bf/trsry").into_account_truncating();
	let treasury_location = MultiLocation {
		parents: 0,
		interior: X1(AccountId32 { network: None, id: treasury_account_id_32 }),
	};

	// update some PHA balance to treasury account
	assert_ok!(Tokens::set_balance(
		RuntimeOrigin::root(),
		treasury_account_id_32.into(),
		PHA,
		1_000_000_000_000_000_000,
		0
	));

	// Set fee source
	assert_ok!(Slp::set_fee_source(
		RuntimeOrigin::signed(ALICE),
		PHA,
		Some((treasury_location, 1_000_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		PHA,
		XcmOperationType::Bond,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		PHA,
		XcmOperationType::Unbond,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		PHA,
		XcmOperationType::TransferBack,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		PHA,
		XcmOperationType::TransferTo,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));

	assert_ok!(<Runtime as crate::Config>::XcmWeightAndFeeHandler::set_xcm_dest_weight_and_fee(
		PHA,
		XcmOperationType::ConvertAsset,
		Some((20_000_000_000.into(), 10_000_000_000)),
	));
}

fn phala_setup() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_id_0: AccountId =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0);

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(Parachain(2004), AccountId32 { network: None, id: subaccount_id_0.into() }),
	};

	initialize_preparation_setup();

	// First to setup index-multilocation relationship of subaccount_0
	assert_ok!(Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), PHA, None));

	phala_xcm_setup();

	// Set delegator ledger
	assert_ok!(Slp::add_validator(
		RuntimeOrigin::signed(ALICE),
		PHA,
		Box::new(VALIDATOR_0_LOCATION),
	));

	// delegate a validator for the delegator
	assert_ok!(Slp::delegate(
		RuntimeOrigin::signed(ALICE),
		PHA,
		Box::new(subaccount_0_location),
		vec![VALIDATOR_0_LOCATION],
		None
	));
}

#[test]
fn phala_bond_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32 },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		let share_price_multilocation =
			MultiLocation { parents: 1, interior: X2(GeneralIndex(2000), GeneralIndex(1000)) };

		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				1_000_000_000_000,
				Some(share_price_multilocation),
				None
			),
			Error::<Runtime>::DelegatorNotExist
		);

		// intialize a delegator
		initialize_preparation_setup();
		assert_ok!(Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), PHA, None));

		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				1_000_000_000_000,
				Some(share_price_multilocation),
				None
			),
			Error::<Runtime>::DelegatorNotExist
		);

		// delegate a validator for the delegator
		assert_ok!(Slp::add_validator(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(VALIDATOR_0_LOCATION),
		));
		assert_ok!(Slp::delegate(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(subaccount_0_location),
			vec![VALIDATOR_0_LOCATION],
			None
		));

		phala_xcm_setup();

		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				100_000_000_000,
				Some(share_price_multilocation),
				None
			),
			Error::<Runtime>::LowerThanMinimum
		);

		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				2_000_000_000_000_000_000,
				Some(share_price_multilocation),
				None
			),
			Error::<Runtime>::ExceedActiveMaximum
		);

		// wrong share price multilocation
		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				1_000_000_000_000,
				Some(subaccount_0_location),
				None
			),
			Error::<Runtime>::SharePriceNotValid
		);

		// wrong share price multilocation
		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				1_000_000_000_000,
				Some(VALIDATOR_0_LOCATION),
				None
			),
			Error::<Runtime>::DividedByZero
		);

		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				1_000_000_000_000,
				Some(share_price_multilocation),
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn phala_unbond_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_id_0: AccountId =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0);

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(Parachain(2004), AccountId32 { network: None, id: subaccount_id_0.into() }),
	};

	ExtBuilder::default().build().execute_with(|| {
		let share_price_multilocation =
			MultiLocation { parents: 1, interior: X2(GeneralIndex(1000), GeneralIndex(1000)) };

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(share_price_multilocation),
				1_000_000,
				None
			),
			Error::<Runtime>::DelegatorNotExist
		);

		// environment setup
		phala_setup();

		let ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 500_000_000_000,
			unlocking_shares: 1,
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		let phala_ledger = Ledger::<BalanceOf<Runtime>>::Phala(ledger);

		DelegatorLedgers::<Runtime>::insert(PHA, subaccount_0_location, phala_ledger);

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(share_price_multilocation),
				1_000_000,
				None
			),
			Error::<Runtime>::AlreadyRequested
		);

		let ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 500_000_000_000,
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		let phala_ledger = Ledger::<BalanceOf<Runtime>>::Phala(ledger);

		DelegatorLedgers::<Runtime>::insert(PHA, subaccount_0_location, phala_ledger);

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(share_price_multilocation),
				0,
				None
			),
			Error::<Runtime>::AmountZero
		);

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(subaccount_0_location),
				1_000_000,
				None
			),
			Error::<Runtime>::SharePriceNotValid
		);

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(VALIDATOR_0_LOCATION),
				1_000_000,
				None
			),
			Error::<Runtime>::DividedByZero
		);

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(share_price_multilocation),
				1_000_000,
				None
			),
			Error::<Runtime>::LowerThanMinimum
		);

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(share_price_multilocation),
				1_000_000_000_000,
				None
			),
			Error::<Runtime>::NotEnoughToUnbond
		);

		let ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 5_000_000_000_000,
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		let phala_ledger = Ledger::<BalanceOf<Runtime>>::Phala(ledger);

		DelegatorLedgers::<Runtime>::insert(PHA, subaccount_0_location, phala_ledger);

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(share_price_multilocation),
				1_000_000_000_000,
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn phala_rebond_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		let share_price_multilocation =
			MultiLocation { parents: 1, interior: X2(GeneralIndex(2000), GeneralIndex(1000)) };

		// environment setup
		phala_setup();

		assert_noop!(
			Slp::rebond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(share_price_multilocation),
				None,
				None
			),
			Error::<Runtime>::InvalidAmount
		);

		assert_noop!(
			Slp::rebond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(share_price_multilocation),
				Some(0),
				None
			),
			Error::<Runtime>::AmountZero
		);

		assert_noop!(
			Slp::rebond(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Some(share_price_multilocation),
				Some(1_000_000_000_000),
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn phala_undelegate_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		phala_setup();

		let ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 500_000_000_000,
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		let phala_ledger = Ledger::<BalanceOf<Runtime>>::Phala(ledger);

		DelegatorLedgers::<Runtime>::insert(PHA, subaccount_0_location, phala_ledger);

		assert_noop!(
			Slp::undelegate(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				vec![VALIDATOR_0_LOCATION],
				None
			),
			Error::<Runtime>::ValidatorStillInUse
		);

		let ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: Zero::zero(),
			unlocking_shares: 500_000_000_000,
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		let phala_ledger = Ledger::<BalanceOf<Runtime>>::Phala(ledger);

		DelegatorLedgers::<Runtime>::insert(PHA, subaccount_0_location, phala_ledger);

		assert_noop!(
			Slp::undelegate(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				vec![VALIDATOR_0_LOCATION],
				None
			),
			Error::<Runtime>::ValidatorStillInUse
		);

		let ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: Zero::zero(),
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		let phala_ledger = Ledger::<BalanceOf<Runtime>>::Phala(ledger);

		DelegatorLedgers::<Runtime>::insert(PHA, subaccount_0_location, phala_ledger);

		assert_ok!(Slp::undelegate(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(subaccount_0_location),
			vec![VALIDATOR_0_LOCATION],
			None
		));

		let undelegated_ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: Zero::zero(),
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: None,
			bonded_pool_collection_id: None,
			bonded_is_vault: None,
		};
		assert_eq!(
			DelegatorLedgers::<Runtime>::get(PHA, subaccount_0_location),
			Some(Ledger::<BalanceOf<Runtime>>::Phala(undelegated_ledger))
		);
	});
}

#[test]
fn phala_redelegate_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		phala_setup();

		let old_ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: Zero::zero(),
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		assert_eq!(
			DelegatorLedgers::<Runtime>::get(PHA, subaccount_0_location),
			Some(Ledger::<BalanceOf<Runtime>>::Phala(old_ledger))
		);

		assert_noop!(
			Slp::redelegate(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				None,
				None
			),
			Error::<Runtime>::ValidatorNotProvided
		);

		assert_ok!(Slp::add_validator(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(VALIDATOR_1_LOCATION)
		));

		assert_ok!(Slp::redelegate(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(subaccount_0_location),
			Some(vec![VALIDATOR_1_LOCATION]),
			None
		));

		let new_ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: Zero::zero(),
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(1),
			bonded_pool_collection_id: Some(1),
			bonded_is_vault: Some(true),
		};
		assert_eq!(
			DelegatorLedgers::<Runtime>::get(PHA, subaccount_0_location),
			Some(Ledger::<BalanceOf<Runtime>>::Phala(new_ledger))
		);
	});
}

#[test]
fn phala_liquidize_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		phala_setup();

		let ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 5_000_000_000_000,
			unlocking_shares: 1_000_000_000_000,
			unlocking_time_unit: Some(TimeUnit::Hour(100)),
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		let phala_ledger = Ledger::<BalanceOf<Runtime>>::Phala(ledger);

		DelegatorLedgers::<Runtime>::insert(PHA, subaccount_0_location, phala_ledger);

		assert_noop!(
			Slp::liquidize(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				None,
				None,
				Some(2_000_000_000_000),
				None
			),
			Error::<Runtime>::InvalidAmount
		);

		assert_ok!(Slp::liquidize(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(subaccount_0_location),
			None,
			None,
			Some(500_000_000_000),
			None
		));

		let compared_ledger_1 = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 5_000_000_000_000,
			unlocking_shares: 500_000_000_000,
			unlocking_time_unit: Some(TimeUnit::Hour(100)),
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		assert_eq!(
			DelegatorLedgers::<Runtime>::get(PHA, subaccount_0_location),
			Some(Ledger::<BalanceOf<Runtime>>::Phala(compared_ledger_1))
		);

		assert_ok!(Slp::liquidize(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(subaccount_0_location),
			None,
			None,
			Some(Zero::zero()),
			None
		));

		let compared_ledger_2 = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 5_000_000_000_000,
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		assert_eq!(
			DelegatorLedgers::<Runtime>::get(PHA, subaccount_0_location),
			Some(Ledger::<BalanceOf<Runtime>>::Phala(compared_ledger_2))
		);
	});
}

#[test]
fn phala_bond_confirm_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		phala_setup();

		// set ledger
		let ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 5_000_000_000_000,
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		let phala_ledger = Ledger::<BalanceOf<Runtime>>::Phala(ledger);

		DelegatorLedgers::<Runtime>::insert(PHA, subaccount_0_location, phala_ledger);

		// Bond confirm
		// setup updateEntry
		let query_id = 0;
		let bond_update_entry = LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
			currency_id: PHA,
			delegator_id: subaccount_0_location,
			update_operation: SubstrateLedgerUpdateOperation::Bond,
			amount: 10_000_000_000_000,
			unlock_time: None,
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(
			query_id,
			(bond_update_entry.clone(), 1000),
		);

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((bond_update_entry, 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			PHA,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let ledger_new = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 15_000_000_000_000,
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(PHA, subaccount_0_location),
			Some(Ledger::<BalanceOf<Runtime>>::Phala(ledger_new))
		);
	});
}

#[test]
fn phala_unbond_confirm_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		phala_setup();

		// set ledger
		let ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 15_000_000_000_000,
			unlocking_shares: Zero::zero(),
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		let phala_ledger = Ledger::<BalanceOf<Runtime>>::Phala(ledger);

		DelegatorLedgers::<Runtime>::insert(PHA, subaccount_0_location, phala_ledger);

		// Unlock confirm
		let query_id = 1;
		let unlock_update_entry = LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
			currency_id: PHA,
			delegator_id: subaccount_0_location,
			update_operation: SubstrateLedgerUpdateOperation::Unlock,
			amount: 10_000_000_000_000,
			unlock_time: Some(TimeUnit::Hour(200)),
		});

		DelegatorLedgerXcmUpdateQueue::<Runtime>::insert(
			query_id,
			(unlock_update_entry.clone(), 1000),
		);

		assert_eq!(
			DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id),
			Some((unlock_update_entry, 1000))
		);

		assert_ok!(Slp::confirm_delegator_ledger_query_response(
			RuntimeOrigin::signed(ALICE),
			PHA,
			query_id
		));

		assert_eq!(DelegatorLedgerXcmUpdateQueue::<Runtime>::get(query_id), None);

		let ledger_new = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 5_000_000_000_000,
			unlocking_shares: 10_000_000_000_000,
			unlocking_time_unit: Some(TimeUnit::Hour(200)),
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};

		assert_eq!(
			DelegatorLedgers::<Runtime>::get(PHA, subaccount_0_location),
			Some(Ledger::<BalanceOf<Runtime>>::Phala(ledger_new))
		);
	});
}

#[test]
fn phala_transfer_back_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		phala_setup();
		let exit_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746f75740000000000000000000000000000000000000000"]
				.into();

		let exit_account_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: exit_account_id_32 }),
		};

		assert_noop!(
			Slp::transfer_back(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				Box::new(exit_account_location),
				Zero::zero(),
				None
			),
			Error::<Runtime>::AmountZero
		);

		assert_noop!(
			Slp::transfer_back(
				RuntimeOrigin::signed(ALICE),
				PHA,
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
fn phala_transfer_to_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		phala_setup();
		let entrance_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746b696e0000000000000000000000000000000000000000"]
				.into();

		let entrance_account_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: entrance_account_id_32 }),
		};

		let exit_account_id_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f76746f75740000000000000000000000000000000000000000"]
				.into();

		let exit_account_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: exit_account_id_32 }),
		};

		assert_noop!(
			Slp::transfer_to(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(exit_account_location),
				Box::new(subaccount_0_location),
				5_000_000_000_000_000_000,
			),
			Error::<Runtime>::InvalidAccount
		);

		assert_noop!(
			Slp::transfer_to(
				RuntimeOrigin::signed(ALICE),
				PHA,
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
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		let treasury_id: AccountId = PalletId(*b"bf/trsry").into_account_truncating();
		let treasury_32: [u8; 32] = PalletId(*b"bf/trsry").into_account_truncating();

		phala_setup();

		bifrost_vtoken_minting::OngoingTimeUnit::<Runtime>::insert(PHA, TimeUnit::Hour(1));

		let ledger = PhalaLedger::<BalanceOf<Runtime>> {
			account: subaccount_0_location,
			active_shares: 500_000_000_000,
			unlocking_shares: 1,
			unlocking_time_unit: None,
			bonded_pool_id: Some(0),
			bonded_pool_collection_id: Some(0),
			bonded_is_vault: Some(true),
		};
		let phala_ledger = Ledger::<BalanceOf<Runtime>>::Phala(ledger);

		// Set delegator ledger
		DelegatorLedgers::<Runtime>::insert(PHA, subaccount_0_location, phala_ledger);

		// Set the hosting fee to be 20%, and the beneficiary to be bifrost treasury account.
		let pct = Permill::from_percent(20);
		let treasury_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: treasury_32 }),
		};

		assert_ok!(Slp::set_hosting_fees(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Some((pct, treasury_location))
		));

		let pct_100 = Permill::from_percent(100);
		assert_ok!(Slp::set_currency_tune_exchange_rate_limit(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Some((1, pct_100))
		));

		// First set base vtoken exchange rate. Should be 1:1.
		assert_ok!(Currencies::deposit(VPHA, &ALICE, 100));
		assert_ok!(Slp::increase_token_pool(RuntimeOrigin::signed(ALICE), PHA, 100));

		// call the charge_host_fee_and_tune_vtoken_exchange_rate
		assert_ok!(Slp::charge_host_fee_and_tune_vtoken_exchange_rate(
			RuntimeOrigin::signed(ALICE),
			PHA,
			100,
			Some(subaccount_0_location)
		));

		// Tokenpool should have been added 100.
		let new_token_pool_amount = <Runtime as Config>::VtokenMinting::get_token_pool(PHA);
		assert_eq!(new_token_pool_amount, 200);

		// let tune_record = DelegatorLatestTuneRecord::<Runtime>::get(PHA,
		// &subaccount_0_location); assert_eq!(tune_record, (1, Some(TimeUnit::Hour(1))));

		let tune_record = CurrencyLatestTuneRecord::<Runtime>::get(PHA);
		assert_eq!(tune_record, Some((TimeUnit::Hour(1), 1)));

		// Treasury account has been issued a fee of 20 vpha which equals to the value of 20 pha
		// before new exchange rate tuned.
		let treasury_vpha = Currencies::free_balance(VPHA, &treasury_id);
		assert_eq!(treasury_vpha, 20);
	});
}

#[test]
fn add_validator_and_remove_validator_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		let mut valis = vec![];

		initialize_preparation_setup();

		assert_ok!(Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), PHA, None));

		// Set delegator ledger
		assert_ok!(Slp::add_validator(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(VALIDATOR_0_LOCATION),
		));

		// The storage is reordered by hash. So we need to adjust the push order here.
		valis.push(VALIDATOR_0_LOCATION);

		let bounded_valis = BoundedVec::try_from(valis).unwrap();
		assert_eq!(Validators::<Runtime>::get(PHA), Some(bounded_valis));

		assert_ok!(Slp::delegate(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(subaccount_0_location),
			vec![VALIDATOR_0_LOCATION],
			None
		));

		assert_ok!(Slp::remove_validator(
			RuntimeOrigin::signed(ALICE),
			PHA,
			Box::new(VALIDATOR_0_LOCATION),
		));

		let empty_bounded_vec = BoundedVec::default();
		assert_eq!(Validators::<Runtime>::get(PHA), Some(empty_bounded_vec));
	});
}

#[test]
fn phala_convert_asset_works() {
	let bifrost_parachain_account_id: AccountId = Sibling::from(2001).into_account_truncating();
	// subaccount_id_0: 41YcGwBLwxbFV7VfbF6zYGgUnYbt96dHcA2DWruRJkWtANFD
	let subaccount_0_account_id_32: [u8; 32] =
		Utility::derivative_account_id(bifrost_parachain_account_id, 0).into();

	let subaccount_0_location = MultiLocation {
		parents: 1,
		interior: X2(
			Parachain(2004),
			AccountId32 { network: None, id: subaccount_0_account_id_32.into() },
		),
	};

	ExtBuilder::default().build().execute_with(|| {
		phala_setup();

		assert_noop!(
			Slp::convert_asset(
				RuntimeOrigin::signed(ALICE),
				PHA,
				Box::new(subaccount_0_location),
				1_000_000_000_000,
				true,
				None
			),
			Error::<Runtime>::XcmFailure
		);
	});
}

#[test]
fn generate_derivative_account() {
	ExtBuilder::default().build().execute_with(|| {
		// PublicKey: 0x70617261d1070000000000000000000000000000000000000000000000000000
		// AccountId(42): 5Ec4AhPV91i9yNuiWuNunPf6AQCYDhFTTA4G5QCbtqYApH9E
		let sovereign_account = <ParaId as AccountIdConversion<AccountId>>::into_account_truncating(
			&ParaId::from(2001),
		);
		println!("sovereign_account: {:?}", sovereign_account);
		// PublicKey: 0x5a53736d8e96f1c007cf0d630acf5209b20611617af23ce924c8e25328eb5d28
		// AccountId(42): 5E78xTBiaN3nAGYtcNnqTJQJqYAkSDGggKqaDfpNsKyPpbcb
		let sovereign_account_derivative_0 =
			Utility::derivative_account_id(sovereign_account.clone(), 0);
		assert_eq!(
			sovereign_account_derivative_0,
			AccountId::from_ss58check("5E78xTBiaN3nAGYtcNnqTJQJqYAkSDGggKqaDfpNsKyPpbcb").unwrap()
		);
		// PublicKey: 0xf1c5ca0368e7a567945a59aaea92b9be1e0794fe5e077d017462b7ce8fc1ed7c
		// AccountId(42): 5HXi9pzWnTQzk7VKzY6VQn92KfWCcA5NbSm53uKHrYU1VsjP
		let sovereign_account_derivative_1 =
			Utility::derivative_account_id(sovereign_account.clone(), 1);
		assert_eq!(
			sovereign_account_derivative_1,
			AccountId::from_ss58check("5HXi9pzWnTQzk7VKzY6VQn92KfWCcA5NbSm53uKHrYU1VsjP").unwrap()
		);
		// PublicKey: 0x1e365411cfd0b0f78466be433a2ec5f7d545c5e28cb2e9a31ce97d4a28447dfc
		// AccountId(42): 5CkKS3YMx64TguUYrMERc5Bn6Mn2aKMUkcozUFREQDgHS3Tv
		let sovereign_account_derivative_2 =
			Utility::derivative_account_id(sovereign_account.clone(), 2);
		assert_eq!(
			sovereign_account_derivative_2,
			AccountId::from_ss58check("5CkKS3YMx64TguUYrMERc5Bn6Mn2aKMUkcozUFREQDgHS3Tv").unwrap()
		);
	})
}

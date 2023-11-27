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

use crate::{mocks::mock::*, primitives::FilecoinLedger, *};
use bifrost_primitives::currency::{FIL, VFIL};
use frame_support::{assert_noop, assert_ok, PalletId};
use sp_runtime::traits::AccountIdConversion;

fn mins_maxs_setup() {
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
	MinimumsAndMaximums::<Runtime>::insert(FIL, mins_and_maxs);
}

fn initialize_delegator_setup() {
	let location =
		MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

	mins_maxs_setup();
	let _ = Slp::initialize_delegator(RuntimeOrigin::signed(ALICE), FIL, Some(Box::new(location)));
}

fn delegate_setup() {
	let location =
		MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

	let owner_location =
		MultiLocation { parents: 111, interior: X1(Junction::from(BoundedVec::default())) };

	initialize_delegator_setup();

	let _ = Slp::add_validator(RuntimeOrigin::signed(ALICE), FIL, Box::new(owner_location));
	let _ = Slp::delegate(
		RuntimeOrigin::signed(ALICE),
		FIL,
		Box::new(location),
		vec![owner_location],
		None,
		None,
	);
}

fn bond_setup() {
	let location =
		MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

	delegate_setup();

	let _ = Slp::bond(
		RuntimeOrigin::signed(ALICE),
		FIL,
		Box::new(location),
		1_000_000_000_000,
		None,
		None,
		None,
	);
}

#[test]
fn initialize_delegator_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location =
			MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

		System::set_block_number(1);

		mins_maxs_setup();
		assert_ok!(Slp::initialize_delegator(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Some(Box::new(location))
		));

		assert_eq!(DelegatorNextIndex::<Runtime>::get(FIL), 1);
		assert_eq!(DelegatorsIndex2Multilocation::<Runtime>::get(FIL, 0), Some(location));
		assert_eq!(DelegatorsMultilocation2Index::<Runtime>::get(FIL, location), Some(0));
	});
}

#[test]
fn bond_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location =
			MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

		System::set_block_number(1);

		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				FIL,
				Box::new(location),
				1_000_000_000_000,
				None,
				None,
				None
			),
			Error::<Runtime>::NotExist
		);

		delegate_setup();

		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				FIL,
				Box::new(location),
				1_000,
				None,
				None,
				None
			),
			Error::<Runtime>::LowerThanMinimum
		);

		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				FIL,
				Box::new(location),
				300_000_000_000_000,
				None,
				None,
				None
			),
			Error::<Runtime>::ExceedActiveMaximum
		);

		assert_ok!(Slp::bond(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Box::new(location),
			1_000_000_000_000,
			None,
			None,
			None
		));

		let fil_ledger = FilecoinLedger { account: location, initial_pledge: 1000000000000 };
		let ledger = Ledger::Filecoin(fil_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(FIL, location), Some(ledger));

		assert_noop!(
			Slp::bond(
				RuntimeOrigin::signed(ALICE),
				FIL,
				Box::new(location),
				1_000_000_000_000,
				None,
				None,
				None
			),
			Error::<Runtime>::AlreadyBonded
		);
	});
}

#[test]
fn delegate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location =
			MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

		let owner_location =
			MultiLocation { parents: 111, interior: X1(Junction::from(BoundedVec::default())) };

		System::set_block_number(1);

		initialize_delegator_setup();

		assert_ok!(Slp::add_validator(RuntimeOrigin::signed(ALICE), FIL, Box::new(owner_location)));

		let validator_list = BoundedVec::try_from(vec![owner_location]).unwrap();
		assert_eq!(Validators::<Runtime>::get(FIL), Some(validator_list.clone()));

		assert_ok!(Slp::delegate(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Box::new(location),
			vec![owner_location],
			None,
			None
		));

		assert_eq!(
			ValidatorsByDelegator::<Runtime>::get(FIL, location),
			Some(validator_list.clone())
		);
	});
}

#[test]
fn bond_extra_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location =
			MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

		assert_noop!(
			Slp::bond_extra(
				RuntimeOrigin::signed(ALICE),
				FIL,
				Box::new(location),
				None,
				1_000_000_000_000,
				None,
				None
			),
			Error::<Runtime>::DelegatorNotBonded
		);

		bond_setup();

		assert_ok!(Slp::bond_extra(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Box::new(location),
			None,
			1_000_000_000_000,
			None,
			None
		));

		let fil_ledger = FilecoinLedger { account: location, initial_pledge: 2000000000000 };
		let ledger = Ledger::Filecoin(fil_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(FIL, location), Some(ledger));
	});
}

#[test]
fn unbond_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location =
			MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

		assert_noop!(
			Slp::unbond(
				RuntimeOrigin::signed(ALICE),
				FIL,
				Box::new(location),
				None,
				500_000_000_000,
				None,
				None
			),
			Error::<Runtime>::DelegatorNotBonded
		);

		bond_setup();

		assert_ok!(Slp::unbond(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Box::new(location),
			None,
			500_000_000_000,
			None,
			None
		));

		let fil_ledger = FilecoinLedger { account: location, initial_pledge: 500000000000 };
		let ledger = Ledger::Filecoin(fil_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(FIL, location), Some(ledger));
	});
}

#[test]
fn undelegate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location =
			MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

		let owner_location =
			MultiLocation { parents: 111, interior: X1(Junction::from(BoundedVec::default())) };

		let other_location =
			MultiLocation { parents: 120, interior: X1(Junction::from(BoundedVec::default())) };

		assert_noop!(
			Slp::undelegate(
				RuntimeOrigin::signed(ALICE),
				FIL,
				Box::new(location),
				vec![owner_location],
				None,
				None
			),
			Error::<Runtime>::DelegatorNotBonded
		);

		bond_setup();

		let validator_list = BoundedVec::try_from(vec![owner_location]).unwrap();
		assert_eq!(ValidatorsByDelegator::<Runtime>::get(FIL, location), Some(validator_list));

		assert_noop!(
			Slp::undelegate(
				RuntimeOrigin::signed(ALICE),
				FIL,
				Box::new(location),
				vec!(owner_location),
				None,
				None
			),
			Error::<Runtime>::AmountNotZero
		);

		// set ledger to zero
		let fil_ledger = FilecoinLedger { account: location, initial_pledge: 0 };
		let ledger = Ledger::Filecoin(fil_ledger);
		DelegatorLedgers::<Runtime>::insert(FIL, location, ledger);

		assert_noop!(
			Slp::undelegate(
				RuntimeOrigin::signed(ALICE),
				FIL,
				Box::new(location),
				vec![other_location],
				None,
				None
			),
			Error::<Runtime>::ValidatorError
		);

		assert_ok!(Slp::undelegate(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Box::new(location),
			vec![owner_location],
			None,
			None
		));

		assert_eq!(ValidatorsByDelegator::<Runtime>::get(FIL, location), None);
	});
}

#[test]
fn charge_host_fee_and_tune_vtoken_exchange_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location =
			MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

		let treasury_id: AccountId = PalletId(*b"bf/trsry").into_account_truncating();
		let treasury_32: [u8; 32] = treasury_id.clone().into();

		assert_noop!(
			Slp::charge_host_fee_and_tune_vtoken_exchange_rate(
				RuntimeOrigin::signed(ALICE),
				FIL,
				100,
				Some(location)
			),
			Error::<Runtime>::TuneExchangeRateLimitNotSet
		);

		bifrost_vtoken_minting::OngoingTimeUnit::<Runtime>::insert(FIL, TimeUnit::Era(1));

		// Set the hosting fee to be 20%, and the beneficiary to be bifrost treasury account.
		let pct = Permill::from_percent(20);
		let treasury_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: None, id: treasury_32 }),
		};

		assert_ok!(Slp::set_hosting_fees(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Some((pct, treasury_location))
		));

		let pct_100 = Permill::from_percent(100);
		assert_ok!(Slp::set_currency_tune_exchange_rate_limit(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Some((1, pct_100))
		));

		// insert validator into validators list.
		let validator_list = BoundedVec::try_from(vec![location]).unwrap();
		Validators::<Runtime>::insert(FIL, validator_list);

		// First set base vtoken exchange rate. Should be 1:1.
		assert_ok!(Currencies::deposit(VFIL, &ALICE, 100));
		assert_ok!(Slp::increase_token_pool(RuntimeOrigin::signed(ALICE), FIL, 100));

		bond_setup();

		// call the charge_host_fee_and_tune_vtoken_exchange_rate
		assert_ok!(Slp::charge_host_fee_and_tune_vtoken_exchange_rate(
			RuntimeOrigin::signed(ALICE),
			FIL,
			100,
			Some(location)
		));

		// Tokenpool should have been added 100.
		let new_token_pool_amount = <Runtime as Config>::VtokenMinting::get_token_pool(FIL);
		assert_eq!(new_token_pool_amount, 180);

		let tune_record = DelegatorLatestTuneRecord::<Runtime>::get(FIL, &location);
		assert_eq!(tune_record, Some(TimeUnit::Era(1)));

		// Treasury account has been issued a fee of 20 vksm which equals to the value of 20 ksm
		// before new exchange rate tuned.
		let treasury_fil = Currencies::free_balance(FIL, &treasury_id);
		assert_eq!(treasury_fil, 20);
	});
}

#[test]
fn remove_delegator_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location =
			MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

		assert_noop!(
			Slp::remove_delegator(RuntimeOrigin::signed(ALICE), FIL, Box::new(location)),
			Error::<Runtime>::DelegatorNotBonded
		);

		bond_setup();

		let fil_ledger = FilecoinLedger { account: location, initial_pledge: 1_000_000_000_000 };
		let ledger = Ledger::Filecoin(fil_ledger);
		assert_eq!(DelegatorsIndex2Multilocation::<Runtime>::get(FIL, 0), Some(location));
		assert_eq!(DelegatorsMultilocation2Index::<Runtime>::get(FIL, location), Some(0));
		assert_eq!(DelegatorLedgers::<Runtime>::get(FIL, location), Some(ledger));

		assert_noop!(
			Slp::remove_delegator(RuntimeOrigin::signed(ALICE), FIL, Box::new(location)),
			Error::<Runtime>::AmountNotZero
		);

		// set ledger to zero
		let fil_ledger1 = FilecoinLedger { account: location, initial_pledge: 0 };
		let ledger1 = Ledger::Filecoin(fil_ledger1);
		DelegatorLedgers::<Runtime>::insert(FIL, location, ledger1);

		assert_ok!(Slp::remove_delegator(RuntimeOrigin::signed(ALICE), FIL, Box::new(location)));

		assert_eq!(DelegatorsIndex2Multilocation::<Runtime>::get(FIL, 0), None);
		assert_eq!(DelegatorsMultilocation2Index::<Runtime>::get(FIL, location), None);
		assert_eq!(DelegatorLedgers::<Runtime>::get(FIL, location), None);
	});
}

#[test]
fn remove_validator_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location =
			MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

		let owner_location =
			MultiLocation { parents: 111, interior: X1(Junction::from(BoundedVec::default())) };

		assert_noop!(
			Slp::remove_validator(RuntimeOrigin::signed(ALICE), FIL, Box::new(owner_location)),
			Error::<Runtime>::ValidatorSetNotExist
		);

		bond_setup();

		let validator_list = BoundedVec::try_from(vec![owner_location]).unwrap();
		assert_eq!(Validators::<Runtime>::get(FIL), Some(validator_list.clone()));

		// set ledger to zero
		let fil_ledger = FilecoinLedger { account: location, initial_pledge: 0 };
		let ledger = Ledger::Filecoin(fil_ledger);
		DelegatorLedgers::<Runtime>::insert(FIL, location, ledger);

		assert_ok!(Slp::undelegate(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Box::new(location),
			vec![owner_location],
			None,
			None
		));

		assert_ok!(Slp::remove_validator(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Box::new(owner_location)
		));

		let empty_vec = BoundedVec::default();
		assert_eq!(Validators::<Runtime>::get(FIL), Some(empty_vec));
	});
}

#[test]
fn filecoin_transfer_to_works() {
	// miner
	let location =
		MultiLocation { parents: 100, interior: X1(Junction::from(BoundedVec::default())) };

	// worker
	let owner_location =
		MultiLocation { parents: 111, interior: X1(Junction::from(BoundedVec::default())) };

	ExtBuilder::default().build().execute_with(|| {
		// environment setup
		bond_setup();
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
				FIL,
				Box::new(exit_account_location),
				Box::new(owner_location),
				5_000_000_000_000_000_000,
			),
			Error::<Runtime>::InvalidAccount
		);

		assert_noop!(
			Slp::transfer_to(
				RuntimeOrigin::signed(ALICE),
				FIL,
				Box::new(entrance_account_location),
				Box::new(location),
				5_000_000_000_000_000_000,
			),
			Error::<Runtime>::ValidatorNotExist
		);

		assert_ok!(Slp::transfer_to(
			RuntimeOrigin::signed(ALICE),
			FIL,
			Box::new(entrance_account_location),
			Box::new(owner_location),
			0,
		));
	});
}

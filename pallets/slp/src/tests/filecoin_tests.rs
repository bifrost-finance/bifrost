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
	mock::{VFIL, *},
	primitives::FilecoinLedger,
	FIL, *,
};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::WeakBoundedVec;
use xcm::opaque::latest::NetworkId::Any;

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
	let location = MultiLocation {
		parents: 100,
		interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
	};

	mins_maxs_setup();
	let _ = Slp::initialize_delegator(Origin::signed(ALICE), FIL, Some(Box::new(location.clone())));
}

fn delegate_setup() {
	let location = MultiLocation {
		parents: 100,
		interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
	};

	let owner_location = MultiLocation {
		parents: 111,
		interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
	};

	initialize_delegator_setup();

	let _ = Slp::add_validator(Origin::signed(ALICE), FIL, Box::new(owner_location.clone()));
	let _ = Slp::delegate(
		Origin::signed(ALICE),
		FIL,
		Box::new(location.clone()),
		vec![owner_location.clone()],
	);
}

fn bond_setup() {
	let location = MultiLocation {
		parents: 100,
		interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
	};

	delegate_setup();

	let _ =
		Slp::bond(Origin::signed(ALICE), FIL, Box::new(location.clone()), 1_000_000_000_000, None);
}

#[test]
fn initialize_delegator_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		System::set_block_number(1);

		mins_maxs_setup();
		assert_ok!(Slp::initialize_delegator(
			Origin::signed(ALICE),
			FIL,
			Some(Box::new(location.clone()))
		));

		assert_eq!(DelegatorNextIndex::<Runtime>::get(FIL), 1);
		assert_eq!(DelegatorsIndex2Multilocation::<Runtime>::get(FIL, 0), Some(location.clone()));
		assert_eq!(DelegatorsMultilocation2Index::<Runtime>::get(FIL, location), Some(0));
	});
}

#[test]
fn bond_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		System::set_block_number(1);

		assert_noop!(
			Slp::bond(
				Origin::signed(ALICE),
				FIL,
				Box::new(location.clone()),
				1_000_000_000_000,
				None
			),
			Error::<Runtime>::NotExist
		);

		delegate_setup();

		assert_noop!(
			Slp::bond(Origin::signed(ALICE), FIL, Box::new(location.clone()), 1_000, None),
			Error::<Runtime>::LowerThanMinimum
		);

		assert_noop!(
			Slp::bond(
				Origin::signed(ALICE),
				FIL,
				Box::new(location.clone()),
				300_000_000_000_000,
				None
			),
			Error::<Runtime>::ExceedActiveMaximum
		);

		assert_ok!(Slp::bond(
			Origin::signed(ALICE),
			FIL,
			Box::new(location.clone()),
			1_000_000_000_000,
			None
		));

		let fil_ledger =
			FilecoinLedger { account: location.clone(), initial_pledge: 1000000000000 };
		let ledger = Ledger::Filecoin(fil_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(FIL, location.clone()), Some(ledger));

		assert_noop!(
			Slp::bond(
				Origin::signed(ALICE),
				FIL,
				Box::new(location.clone()),
				1_000_000_000_000,
				None
			),
			Error::<Runtime>::AlreadyBonded
		);
	});
}

#[test]
fn delegate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		let owner_location = MultiLocation {
			parents: 111,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		System::set_block_number(1);

		initialize_delegator_setup();

		assert_ok!(Slp::add_validator(
			Origin::signed(ALICE),
			FIL,
			Box::new(owner_location.clone())
		));

		let multi_hash =
			<Runtime as frame_system::Config>::Hashing::hash(&owner_location.clone().encode());
		let validator_list = vec![(owner_location.clone(), multi_hash)];
		assert_eq!(Validators::<Runtime>::get(FIL), Some(validator_list.clone()));

		assert_ok!(Slp::delegate(
			Origin::signed(ALICE),
			FIL,
			Box::new(location.clone()),
			vec![owner_location.clone()]
		));

		assert_eq!(
			ValidatorsByDelegator::<Runtime>::get(FIL, location.clone()),
			Some(validator_list.clone())
		);
	});
}

#[test]
fn bond_extra_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		assert_noop!(
			Slp::bond_extra(
				Origin::signed(ALICE),
				FIL,
				Box::new(location.clone()),
				None,
				1_000_000_000_000,
			),
			Error::<Runtime>::DelegatorNotBonded
		);

		bond_setup();

		assert_ok!(Slp::bond_extra(
			Origin::signed(ALICE),
			FIL,
			Box::new(location.clone()),
			None,
			1_000_000_000_000,
		));

		let fil_ledger =
			FilecoinLedger { account: location.clone(), initial_pledge: 2000000000000 };
		let ledger = Ledger::Filecoin(fil_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(FIL, location.clone()), Some(ledger));
	});
}

#[test]
fn unbond_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		assert_noop!(
			Slp::unbond(
				Origin::signed(ALICE),
				FIL,
				Box::new(location.clone()),
				None,
				500_000_000_000,
			),
			Error::<Runtime>::DelegatorNotBonded
		);

		bond_setup();

		assert_ok!(Slp::unbond(
			Origin::signed(ALICE),
			FIL,
			Box::new(location.clone()),
			None,
			500_000_000_000,
		));

		let fil_ledger = FilecoinLedger { account: location.clone(), initial_pledge: 500000000000 };
		let ledger = Ledger::Filecoin(fil_ledger);

		assert_eq!(DelegatorLedgers::<Runtime>::get(FIL, location.clone()), Some(ledger));
	});
}

#[test]
fn undelegate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		let owner_location = MultiLocation {
			parents: 111,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		let other_location = MultiLocation {
			parents: 120,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		assert_noop!(
			Slp::undelegate(
				Origin::signed(ALICE),
				FIL,
				Box::new(location.clone()),
				vec![owner_location.clone()]
			),
			Error::<Runtime>::DelegatorNotBonded
		);

		bond_setup();

		let multi_hash =
			<Runtime as frame_system::Config>::Hashing::hash(&owner_location.clone().encode());
		let validator_list = vec![(owner_location.clone(), multi_hash)];
		assert_eq!(
			ValidatorsByDelegator::<Runtime>::get(FIL, location.clone()),
			Some(validator_list)
		);

		assert_noop!(
			Slp::undelegate(
				Origin::signed(ALICE),
				FIL,
				Box::new(location.clone()),
				vec!(owner_location.clone())
			),
			Error::<Runtime>::AmountNotZero
		);

		// set ledger to zero
		let fil_ledger = FilecoinLedger { account: location.clone(), initial_pledge: 0 };
		let ledger = Ledger::Filecoin(fil_ledger);
		DelegatorLedgers::<Runtime>::insert(FIL, location.clone(), ledger);

		assert_noop!(
			Slp::undelegate(
				Origin::signed(ALICE),
				FIL,
				Box::new(location.clone()),
				vec![other_location.clone()]
			),
			Error::<Runtime>::ValidatorError
		);

		assert_ok!(Slp::undelegate(
			Origin::signed(ALICE),
			FIL,
			Box::new(location.clone()),
			vec![owner_location.clone()]
		));

		assert_eq!(ValidatorsByDelegator::<Runtime>::get(FIL, location.clone()), None);
	});
}

#[test]
fn charge_host_fee_and_tune_vtoken_exchange_rate_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		let treasury_id: AccountId =
			hex_literal::hex!["6d6f646c62662f74727372790000000000000000000000000000000000000000"]
				.into();
		let treasury_32: [u8; 32] =
			hex_literal::hex!["6d6f646c62662f74727372790000000000000000000000000000000000000000"];

		assert_noop!(
			Slp::charge_host_fee_and_tune_vtoken_exchange_rate(
				Origin::signed(ALICE),
				FIL,
				100,
				Some(location.clone())
			),
			Error::<Runtime>::TuneExchangeRateLimitNotSet
		);

		bifrost_vtoken_minting::OngoingTimeUnit::<Runtime>::insert(FIL, TimeUnit::Era(1));

		// Set the hosting fee to be 20%, and the beneficiary to be bifrost treasury account.
		let pct = Permill::from_percent(20);
		let treasury_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: Any, id: treasury_32 }),
		};

		assert_ok!(Slp::set_hosting_fees(
			Origin::signed(ALICE),
			FIL,
			Some((pct, treasury_location))
		));

		let pct_100 = Permill::from_percent(100);
		assert_ok!(Slp::set_currency_tune_exchange_rate_limit(
			Origin::signed(ALICE),
			FIL,
			Some((1, pct_100))
		));

		// insert validator into validators list.
		let multi_hash =
			<Runtime as frame_system::Config>::Hashing::hash(&location.clone().encode());
		let validator_list = vec![(location.clone(), multi_hash)];
		Validators::<Runtime>::insert(FIL, validator_list);

		// First set base vtoken exchange rate. Should be 1:1.
		assert_ok!(Currencies::deposit(VFIL, &ALICE, 100));
		assert_ok!(Slp::increase_token_pool(Origin::signed(ALICE), FIL, 100));

		bond_setup();

		// call the charge_host_fee_and_tune_vtoken_exchange_rate
		assert_ok!(Slp::charge_host_fee_and_tune_vtoken_exchange_rate(
			Origin::signed(ALICE),
			FIL,
			100,
			Some(location.clone())
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
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		assert_noop!(
			Slp::remove_delegator(Origin::signed(ALICE), FIL, Box::new(location.clone())),
			Error::<Runtime>::DelegatorNotBonded
		);

		bond_setup();

		let fil_ledger =
			FilecoinLedger { account: location.clone(), initial_pledge: 1_000_000_000_000 };
		let ledger = Ledger::Filecoin(fil_ledger);
		assert_eq!(DelegatorsIndex2Multilocation::<Runtime>::get(FIL, 0), Some(location.clone()));
		assert_eq!(DelegatorsMultilocation2Index::<Runtime>::get(FIL, location.clone()), Some(0));
		assert_eq!(DelegatorLedgers::<Runtime>::get(FIL, location.clone()), Some(ledger));

		assert_noop!(
			Slp::remove_delegator(Origin::signed(ALICE), FIL, Box::new(location.clone())),
			Error::<Runtime>::AmountNotZero
		);

		// set ledger to zero
		let fil_ledger1 = FilecoinLedger { account: location.clone(), initial_pledge: 0 };
		let ledger1 = Ledger::Filecoin(fil_ledger1);
		DelegatorLedgers::<Runtime>::insert(FIL, location.clone(), ledger1);

		assert_ok!(Slp::remove_delegator(Origin::signed(ALICE), FIL, Box::new(location.clone())));

		assert_eq!(DelegatorsIndex2Multilocation::<Runtime>::get(FIL, 0), None);
		assert_eq!(DelegatorsMultilocation2Index::<Runtime>::get(FIL, location.clone()), None);
		assert_eq!(DelegatorLedgers::<Runtime>::get(FIL, location.clone()), None);
	});
}

#[test]
fn remove_validator_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		let owner_location = MultiLocation {
			parents: 111,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		assert_noop!(
			Slp::remove_validator(Origin::signed(ALICE), FIL, Box::new(owner_location.clone())),
			Error::<Runtime>::ValidatorSetNotExist
		);

		bond_setup();

		let multi_hash =
			<Runtime as frame_system::Config>::Hashing::hash(&owner_location.clone().encode());
		let validator_list = vec![(owner_location.clone(), multi_hash)];
		assert_eq!(Validators::<Runtime>::get(FIL), Some(validator_list.clone()));

		assert_noop!(
			Slp::remove_validator(Origin::signed(ALICE), FIL, Box::new(owner_location.clone())),
			Error::<Runtime>::ValidatorStillInUse
		);

		// set ledger to zero
		let fil_ledger = FilecoinLedger { account: location.clone(), initial_pledge: 0 };
		let ledger = Ledger::Filecoin(fil_ledger);
		DelegatorLedgers::<Runtime>::insert(FIL, location.clone(), ledger);

		assert_ok!(Slp::undelegate(
			Origin::signed(ALICE),
			FIL,
			Box::new(location.clone()),
			vec![owner_location.clone()]
		));

		assert_ok!(Slp::remove_validator(
			Origin::signed(ALICE),
			FIL,
			Box::new(owner_location.clone())
		));

		let empty_vec = vec![];
		assert_eq!(Validators::<Runtime>::get(FIL), Some(empty_vec));
	});
}

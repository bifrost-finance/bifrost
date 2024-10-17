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

use crate::{mock::*, *};
use bifrost_primitives::currency::KSM;
use frame_support::{assert_noop, assert_ok, WeakBoundedVec};
use sp_runtime::DispatchError::BadOrigin;
#[allow(deprecated)]
use xcm::opaque::v2::{Junction, Junctions::X1};

#[allow(deprecated)]
#[test]
fn cross_in_and_cross_out_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};
		CrossCurrencyRegistry::<Runtime>::insert(KSM, ());
		CrossingMinimumAmount::<Runtime>::insert(KSM, (1, 1));
		AccountToOuterMultilocation::<Runtime>::insert(KSM, ALICE, location.clone());
		OuterMultilocationToAccount::<Runtime>::insert(KSM, location.clone(), ALICE);
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 100);

		assert_ok!(CrossInOut::cross_out(RuntimeOrigin::signed(ALICE), KSM, 50));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 50);
	});
}

#[test]
fn add_to_and_remove_from_register_whitelist_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_eq!(RegisterWhiteList::<Runtime>::get(KSM), None);

		assert_ok!(CrossInOut::add_to_register_whitelist(RuntimeOrigin::signed(ALICE), KSM, ALICE));
		assert_eq!(
			RegisterWhiteList::<Runtime>::get(KSM),
			Some(BoundedVec::try_from(vec![ALICE]).unwrap())
		);

		assert_noop!(
			CrossInOut::remove_from_register_whitelist(RuntimeOrigin::signed(ALICE), KSM, BOB),
			Error::<Runtime>::NotExist
		);

		assert_ok!(CrossInOut::remove_from_register_whitelist(
			RuntimeOrigin::signed(ALICE),
			KSM,
			ALICE
		));
		assert_eq!(RegisterWhiteList::<Runtime>::get(KSM), Some(BoundedVec::default()));
	});
}

#[allow(deprecated)]
#[test]
fn register_linked_account_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		let location2 = MultiLocation {
			parents: 111,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		assert_noop!(
			CrossInOut::register_linked_account(
				RuntimeOrigin::signed(ALICE),
				KSM,
				BOB,
				Box::new(location.clone())
			),
			Error::<Runtime>::NotAllowed
		);

		RegisterWhiteList::<Runtime>::insert(KSM, BoundedVec::try_from(vec![ALICE]).unwrap());

		assert_noop!(
			CrossInOut::register_linked_account(
				RuntimeOrigin::signed(ALICE),
				KSM,
				BOB,
				Box::new(location.clone())
			),
			Error::<Runtime>::CurrencyNotSupportCrossInAndOut
		);

		CrossCurrencyRegistry::<Runtime>::insert(KSM, ());

		assert_ok!(CrossInOut::register_linked_account(
			RuntimeOrigin::signed(ALICE),
			KSM,
			ALICE,
			Box::new(location.clone())
		));

		assert_noop!(
			CrossInOut::register_linked_account(
				RuntimeOrigin::signed(ALICE),
				KSM,
				ALICE,
				Box::new(location2)
			),
			Error::<Runtime>::AlreadyExist
		);
	});
}

#[test]
fn register_and_deregister_currency_for_cross_in_out_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		CrossCurrencyRegistry::<Runtime>::insert(KSM, ());

		assert_ok!(CrossInOut::deregister_currency_for_cross_in_out(
			RuntimeOrigin::signed(ALICE),
			KSM,
		));

		assert_eq!(CrossCurrencyRegistry::<Runtime>::get(KSM), None);
	});
}

#[allow(deprecated)]
#[test]
fn change_outer_linked_account_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		let location2 = MultiLocation {
			parents: 111,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		AccountToOuterMultilocation::<Runtime>::insert(KSM, BOB, location.clone());
		OuterMultilocationToAccount::<Runtime>::insert(KSM, location.clone(), BOB);

		assert_noop!(
			CrossInOut::change_outer_linked_account(
				RuntimeOrigin::signed(BOB),
				KSM,
				Box::new(location2.clone()),
				BOB
			),
			BadOrigin
		);

		assert_noop!(
			CrossInOut::change_outer_linked_account(
				RuntimeOrigin::signed(ALICE),
				KSM,
				Box::new(location.clone()),
				BOB
			),
			Error::<Runtime>::CurrencyNotSupportCrossInAndOut
		);

		CrossCurrencyRegistry::<Runtime>::insert(KSM, ());

		assert_noop!(
			CrossInOut::change_outer_linked_account(
				RuntimeOrigin::signed(ALICE),
				KSM,
				Box::new(location.clone()),
				BOB
			),
			Error::<Runtime>::AlreadyExist
		);

		assert_ok!(CrossInOut::change_outer_linked_account(
			RuntimeOrigin::signed(ALICE),
			KSM,
			Box::new(location2.clone()),
			BOB
		));
	});
}

#[test]
fn set_crossing_minimum_amount_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_noop!(
			CrossInOut::set_crossing_minimum_amount(RuntimeOrigin::signed(BOB), KSM, 100, 100),
			BadOrigin
		);

		assert_ok!(CrossInOut::set_crossing_minimum_amount(
			RuntimeOrigin::signed(ALICE),
			KSM,
			100,
			100
		));

		assert_eq!(CrossingMinimumAmount::<Runtime>::get(KSM), Some((100, 100)));
	});
}

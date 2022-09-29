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

use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok, WeakBoundedVec};
use xcm::opaque::latest::{Junction, Junctions::X1};

#[test]
fn cross_in_and_cross_out_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let location = MultiLocation {
			parents: 100,
			interior: X1(Junction::GeneralKey(WeakBoundedVec::default())),
		};

		assert_noop!(
			CrossInOut::cross_in(Origin::signed(ALICE), Box::new(location.clone()), KSM, 100, None),
			Error::<Runtime>::CurrencyNotSupportCrossInAndOut
		);

		CrossCurrencyRegistry::<Runtime>::insert(KSM, ());

		assert_noop!(
			CrossInOut::cross_in(Origin::signed(ALICE), Box::new(location.clone()), KSM, 100, None),
			Error::<Runtime>::NotAllowed
		);

		IssueWhiteList::<Runtime>::insert(KSM, vec![ALICE]);

		assert_noop!(
			CrossInOut::cross_in(Origin::signed(ALICE), Box::new(location.clone()), KSM, 100, None),
			Error::<Runtime>::NoAccountIdMapping
		);

		AccountToOuterMultilocation::<Runtime>::insert(KSM, ALICE, location.clone());
		OuterMultilocationToAccount::<Runtime>::insert(KSM, location.clone(), ALICE);

		assert_eq!(Tokens::free_balance(KSM, &ALICE), 0);
		assert_ok!(CrossInOut::cross_in(Origin::signed(ALICE), Box::new(location), KSM, 100, None));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 100);

		assert_ok!(CrossInOut::cross_out(Origin::signed(ALICE), KSM, 50));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 50);
	});
}

#[test]
fn add_to_and_remove_from_issue_whitelist_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_eq!(CrossInOut::get_issue_whitelist(KSM), None);

		assert_ok!(CrossInOut::add_to_issue_whitelist(Origin::signed(ALICE), KSM, ALICE));
		assert_eq!(CrossInOut::get_issue_whitelist(KSM), Some(vec![ALICE]));

		assert_noop!(
			CrossInOut::remove_from_issue_whitelist(Origin::signed(ALICE), KSM, BOB),
			Error::<Runtime>::NotExist
		);

		assert_ok!(CrossInOut::remove_from_issue_whitelist(Origin::signed(ALICE), KSM, ALICE));
		assert_eq!(CrossInOut::get_issue_whitelist(KSM), Some(vec![]));
	});
}

#[test]
fn add_to_and_remove_from_register_whitelist_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_eq!(CrossInOut::get_register_whitelist(KSM), None);

		assert_ok!(CrossInOut::add_to_register_whitelist(Origin::signed(ALICE), KSM, ALICE));
		assert_eq!(CrossInOut::get_register_whitelist(KSM), Some(vec![ALICE]));

		assert_noop!(
			CrossInOut::remove_from_register_whitelist(Origin::signed(ALICE), KSM, BOB),
			Error::<Runtime>::NotExist
		);

		assert_ok!(CrossInOut::remove_from_register_whitelist(Origin::signed(ALICE), KSM, ALICE));
		assert_eq!(CrossInOut::get_register_whitelist(KSM), Some(vec![]));
	});
}

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
				Origin::signed(ALICE),
				KSM,
				BOB,
				Box::new(location.clone())
			),
			Error::<Runtime>::NotAllowed
		);

		RegisterWhiteList::<Runtime>::insert(KSM, vec![ALICE]);

		assert_noop!(
			CrossInOut::register_linked_account(
				Origin::signed(ALICE),
				KSM,
				BOB,
				Box::new(location.clone())
			),
			Error::<Runtime>::CurrencyNotSupportCrossInAndOut
		);

		CrossCurrencyRegistry::<Runtime>::insert(KSM, ());

		assert_ok!(CrossInOut::register_linked_account(
			Origin::signed(ALICE),
			KSM,
			ALICE,
			Box::new(location.clone())
		));

		assert_noop!(
			CrossInOut::register_linked_account(
				Origin::signed(ALICE),
				KSM,
				ALICE,
				Box::new(location2)
			),
			Error::<Runtime>::AlreadyExist
		);
	});
}

#[test]
fn register_currency_for_cross_in_out_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(CrossInOut::register_currency_for_cross_in_out(
			Origin::signed(ALICE),
			KSM,
			Some(())
		));

		assert_eq!(CrossCurrencyRegistry::<Runtime>::get(KSM), Some(()));

		assert_ok!(CrossInOut::register_currency_for_cross_in_out(
			Origin::signed(ALICE),
			KSM,
			None
		));

		assert_eq!(CrossCurrencyRegistry::<Runtime>::get(KSM), None);
	});
}

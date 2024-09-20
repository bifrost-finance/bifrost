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

use bifrost_primitives::currency::ZLK;
use frame_support::{assert_noop, assert_ok};

use crate::{mock::*, *};

fn initialize_charlie_as_issue_whitelist_member() {
	// Add Charlie
	assert_ok!(TokenIssuer::add_to_issue_whitelist(
		pallet_collective::RawOrigin::Members(2, 3).into(),
		ZLK,
		CHARLIE
	));
	// Issue some ZLK to Charlie's account
	assert_ok!(TokenIssuer::issue(RuntimeOrigin::signed(CHARLIE), CHARLIE, ZLK, 1000));
}

#[test]
fn add_to_issue_whitelist_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		// Charlie is not allowed to issue ZLK.
		assert_noop!(
			TokenIssuer::issue(RuntimeOrigin::signed(CHARLIE), ALICE, ZLK, 800),
			Error::<Runtime>::NotAllowed
		);
		// Chalie is added to the issue whitelist to have the ability of issuing ZLK.
		assert_ok!(TokenIssuer::add_to_issue_whitelist(
			pallet_collective::RawOrigin::Members(2, 3).into(),
			ZLK,
			CHARLIE
		));

		let bounded_list = BoundedVec::try_from(vec![CHARLIE]).unwrap();
		assert_eq!(IssueWhiteList::<Runtime>::get(ZLK), Some(bounded_list));
		// Charlie succuessfully issue 800 unit of ZLK to Alice account
		assert_ok!(TokenIssuer::issue(RuntimeOrigin::signed(CHARLIE), ALICE, ZLK, 800));
		assert_eq!(Tokens::free_balance(ZLK, &ALICE), 800);
	});
}

#[test]
fn remove_from_issue_whitelist_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		// Charlie is not in the issue whitelist
		assert_noop!(
			TokenIssuer::remove_from_issue_whitelist(
				pallet_collective::RawOrigin::Members(2, 3).into(),
				ZLK,
				CHARLIE
			),
			Error::<Runtime>::NotExist
		);
		// Add Charlie
		assert_ok!(TokenIssuer::add_to_issue_whitelist(
			pallet_collective::RawOrigin::Members(2, 3).into(),
			ZLK,
			CHARLIE
		));

		// Charlie succuessfully issue 800 unit of ZLK to Alice account
		assert_ok!(TokenIssuer::issue(RuntimeOrigin::signed(CHARLIE), ALICE, ZLK, 800));
		assert_eq!(Tokens::free_balance(ZLK, &ALICE), 800);

		// Successfully remove Charlie
		assert_ok!(TokenIssuer::remove_from_issue_whitelist(
			pallet_collective::RawOrigin::Members(2, 3).into(),
			ZLK,
			CHARLIE
		));
		// Charlie is no longer able to issue token to any account
		assert_noop!(
			TokenIssuer::issue(RuntimeOrigin::signed(CHARLIE), ALICE, ZLK, 800),
			Error::<Runtime>::NotAllowed
		);
	});
}

#[test]
fn add_to_transfer_whitelist_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		initialize_charlie_as_issue_whitelist_member();

		// Charlie is not allowed to transfer ZLK.
		assert_noop!(
			TokenIssuer::transfer(RuntimeOrigin::signed(CHARLIE), ALICE, ZLK, 800),
			Error::<Runtime>::NotAllowed
		);
		// Chalie is added to the transfer whitelist to have the ability of transferring ZLK.
		assert_ok!(TokenIssuer::add_to_transfer_whitelist(
			pallet_collective::RawOrigin::Members(2, 3).into(),
			ZLK,
			CHARLIE
		));

		let bounded_list = BoundedVec::try_from(vec![CHARLIE]).unwrap();
		assert_eq!(TransferWhiteList::<Runtime>::get(ZLK), Some(bounded_list));
		// Charlie succuessfully transfer 800 unit of ZLK to Alice account
		assert_ok!(TokenIssuer::transfer(RuntimeOrigin::signed(CHARLIE), ALICE, ZLK, 800));
		assert_eq!(Tokens::free_balance(ZLK, &ALICE), 800);
		// exceed balance
		assert_noop!(
			TokenIssuer::transfer(RuntimeOrigin::signed(CHARLIE), ALICE, ZLK, 300),
			Error::<Runtime>::NotEnoughBalance
		);
	});
}

#[test]
fn remove_from_transfer_whitelist_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		initialize_charlie_as_issue_whitelist_member();

		// Charlie is not in the transfer whitelist
		assert_noop!(
			TokenIssuer::remove_from_transfer_whitelist(
				pallet_collective::RawOrigin::Members(2, 3).into(),
				ZLK,
				CHARLIE
			),
			Error::<Runtime>::NotExist
		);
		// Add Charlie
		assert_ok!(TokenIssuer::add_to_transfer_whitelist(
			pallet_collective::RawOrigin::Members(2, 3).into(),
			ZLK,
			CHARLIE
		));

		// Charlie succuessfully transfer 800 unit of ZLK to Alice account
		assert_ok!(TokenIssuer::transfer(RuntimeOrigin::signed(CHARLIE), ALICE, ZLK, 800));
		assert_eq!(Tokens::free_balance(ZLK, &ALICE), 800);

		// Successfully remove Charlie
		assert_ok!(TokenIssuer::remove_from_transfer_whitelist(
			pallet_collective::RawOrigin::Members(2, 3).into(),
			ZLK,
			CHARLIE
		));
		// Charlie is no longer able to transfer token to any account
		assert_noop!(
			TokenIssuer::transfer(RuntimeOrigin::signed(CHARLIE), ALICE, ZLK, 800),
			Error::<Runtime>::NotAllowed
		);
	});
}

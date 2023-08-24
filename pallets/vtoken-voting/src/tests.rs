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

// Ensure we're `no_std` when compiling for Wasm.

use super::*;
use crate::mock::*;
use frame_support::{
	assert_ok,
	traits::{
		fungibles::Inspect,
		tokens::{Fortitude::Polite, Preservation::Expendable},
	},
};
use node_primitives::currency::VKSM;
use pallet_conviction_voting::Vote;
use pallet_xcm::Origin as XcmOrigin;

fn aye(amount: Balance, conviction: u8) -> AccountVote<Balance> {
	let vote = Vote { aye: true, conviction: conviction.try_into().unwrap() };
	AccountVote::Standard { vote, balance: amount }
}

fn nay(amount: Balance, conviction: u8) -> AccountVote<Balance> {
	let vote = Vote { aye: false, conviction: conviction.try_into().unwrap() };
	AccountVote::Standard { vote, balance: amount }
}

fn split(aye: Balance, nay: Balance) -> AccountVote<Balance> {
	AccountVote::Split { aye, nay }
}

fn split_abstain(aye: Balance, nay: Balance, abstain: Balance) -> AccountVote<Balance> {
	AccountVote::SplitAbstain { aye, nay, abstain }
}

fn tally(vtoken: CurrencyId, poll_index: u32) -> TallyOf<Runtime> {
	VtokenVoting::as_ongoing(vtoken, poll_index).expect("No poll")
}

fn class(vtoken: CurrencyId, poll_index: u32) -> PollIndexOf<Runtime> {
	poll_index
}

fn usable_balance(vtoken: CurrencyId, who: &AccountId) -> Balance {
	Tokens::reducible_balance(vtoken, who, Expendable, Polite)
}

fn origin_response(location: MultiLocation) -> RuntimeOrigin {
	XcmOrigin::Response(location).into()
}

#[test]
fn basic_voting_works() {
	new_test_ext().execute_with(|| {
		let poll_index = 3;
		let vtoken = VKSM;

		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, aye(2, 5)));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(10, 0, 2));
		System::assert_last_event(RuntimeEvent::VtokenVoting(Event::Voted {
			who: ALICE,
			vtoken,
			poll_index,
			vote: aye(2, 5),
		}));
		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, nay(2, 5)));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 10, 0));
		assert_eq!(usable_balance(vtoken, &ALICE), 8);
		System::assert_last_event(RuntimeEvent::VtokenVoting(Event::Voted {
			who: ALICE,
			vtoken,
			poll_index,
			vote: nay(2, 5),
		}));

		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, aye(5, 1)));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(5, 0, 5));
		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, nay(5, 1)));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 5, 0));
		assert_eq!(usable_balance(vtoken, &ALICE), 5);

		assert_ok!(VtokenVoting::vote(
			RuntimeOrigin::signed(ALICE),
			vtoken,
			poll_index,
			aye(10, 0),
		));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(1, 0, 10));
		assert_ok!(VtokenVoting::vote(
			RuntimeOrigin::signed(ALICE),
			vtoken,
			poll_index,
			nay(10, 0)
		));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 1, 0));
		assert_eq!(usable_balance(vtoken, &ALICE), 0);

		assert_ok!(VtokenVoting::try_remove_vote(&ALICE, vtoken, poll_index, UnvoteScope::Any));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 0, 0));

		assert_ok!(VtokenVoting::update_lock(&ALICE, vtoken, &class(vtoken, poll_index)));
		assert_eq!(usable_balance(vtoken, &ALICE), 10);
	});
}

#[test]
fn split_voting_works() {
	new_test_ext().execute_with(|| {
		let poll_index = 3;
		let vtoken = VKSM;

		assert_ok!(VtokenVoting::vote(
			RuntimeOrigin::signed(ALICE),
			vtoken,
			poll_index,
			split(10, 0)
		));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(1, 0, 10));
		System::assert_last_event(RuntimeEvent::VtokenVoting(Event::Voted {
			who: ALICE,
			vtoken,
			poll_index,
			vote: split(10, 0),
		}));
		assert_ok!(VtokenVoting::vote(
			RuntimeOrigin::signed(ALICE),
			vtoken,
			poll_index,
			split(5, 5)
		));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 0, 5));
		assert_eq!(usable_balance(vtoken, &ALICE), 0);

		assert_ok!(VtokenVoting::try_remove_vote(&ALICE, vtoken, poll_index, UnvoteScope::Any));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 0, 0));

		assert_ok!(VtokenVoting::update_lock(&ALICE, vtoken, &class(vtoken, poll_index)));
		assert_eq!(usable_balance(vtoken, &ALICE), 10);
	});
}

#[test]
fn abstain_voting_works() {
	new_test_ext().execute_with(|| {
		let poll_index = 3;
		let vtoken = VKSM;

		assert_ok!(VtokenVoting::vote(
			RuntimeOrigin::signed(ALICE),
			vtoken,
			poll_index,
			split_abstain(0, 0, 10)
		));
		System::assert_last_event(RuntimeEvent::VtokenVoting(Event::Voted {
			who: ALICE,
			vtoken,
			poll_index,
			vote: split_abstain(0, 0, 10),
		}));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 0, 10));
		assert_ok!(VtokenVoting::vote(
			RuntimeOrigin::signed(BOB),
			vtoken,
			poll_index,
			split_abstain(0, 0, 20)
		));
		System::assert_last_event(RuntimeEvent::VtokenVoting(Event::Voted {
			who: BOB,
			vtoken,
			poll_index,
			vote: split_abstain(0, 0, 20),
		}));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 0, 30));
		assert_ok!(VtokenVoting::vote(
			RuntimeOrigin::signed(BOB),
			vtoken,
			poll_index,
			split_abstain(10, 0, 10)
		));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(1, 0, 30));
		assert_eq!(usable_balance(vtoken, &ALICE), 0);
		assert_eq!(usable_balance(vtoken, &BOB), 0);

		assert_ok!(VtokenVoting::try_remove_vote(&ALICE, vtoken, poll_index, UnvoteScope::Any));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(1, 0, 20));

		assert_ok!(VtokenVoting::try_remove_vote(&BOB, vtoken, poll_index, UnvoteScope::Any));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 0, 0));

		assert_ok!(VtokenVoting::update_lock(&ALICE, vtoken, &class(vtoken, poll_index)));
		assert_eq!(usable_balance(vtoken, &ALICE), 10);

		assert_ok!(VtokenVoting::update_lock(&BOB, vtoken, &class(vtoken, poll_index)));
		assert_eq!(usable_balance(vtoken, &BOB), 20);
	});
}

#[test]
fn voting_balance_gets_locked() {
	new_test_ext().execute_with(|| {
		let poll_index = 3;
		let vtoken = VKSM;

		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, aye(2, 5)));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(10, 0, 2));
		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, nay(2, 5)));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 10, 0));
		assert_eq!(usable_balance(vtoken, &ALICE), 8);

		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, aye(5, 1)));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(5, 0, 5));
		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, nay(5, 1)));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 5, 0));
		assert_eq!(usable_balance(vtoken, &ALICE), 5);

		assert_ok!(VtokenVoting::vote(
			RuntimeOrigin::signed(ALICE),
			vtoken,
			poll_index,
			aye(10, 0)
		));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(1, 0, 10));
		assert_ok!(VtokenVoting::vote(
			RuntimeOrigin::signed(ALICE),
			vtoken,
			poll_index,
			nay(10, 0)
		));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 1, 0));
		assert_eq!(usable_balance(vtoken, &ALICE), 0);

		assert_ok!(VtokenVoting::try_remove_vote(&ALICE, vtoken, poll_index, UnvoteScope::Any));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 0, 0));

		assert_ok!(VtokenVoting::update_lock(&ALICE, vtoken, &class(vtoken, poll_index)));
		assert_eq!(usable_balance(vtoken, &ALICE), 10);
	});
}

#[test]
fn successful_but_zero_conviction_vote_balance_can_be_unlocked() {
	new_test_ext().execute_with(|| {
		let poll_index = 3;
		let vtoken = VKSM;

		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, aye(1, 1)));
		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(BOB), vtoken, poll_index, nay(20, 0)));

		let c = class(vtoken, poll_index);
		assert_ok!(VtokenVoting::set_referendum_status(
			RuntimeOrigin::signed(CONTROLLER),
			vtoken,
			poll_index,
			ReferendumInfoOf::<Runtime>::Completed(3),
		));
		assert_ok!(VtokenVoting::try_remove_vote(&BOB, vtoken, poll_index, UnvoteScope::Any));
		assert_ok!(VtokenVoting::update_lock(&BOB, vtoken, &c));
		assert_eq!(usable_balance(vtoken, &BOB), 20);
	});
}

#[test]
fn unsuccessful_conviction_vote_balance_can_be_unlocked() {
	new_test_ext().execute_with(|| {
		let poll_index = 3;
		let vtoken = VKSM;

		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, aye(1, 1)));
		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(BOB), vtoken, poll_index, nay(20, 0)));

		let c = class(vtoken, poll_index);
		assert_ok!(VtokenVoting::set_referendum_status(
			RuntimeOrigin::signed(CONTROLLER),
			vtoken,
			poll_index,
			ReferendumInfoOf::<Runtime>::Completed(3),
		));
		assert_ok!(VtokenVoting::try_remove_vote(&ALICE, vtoken, poll_index, UnvoteScope::Any));
		assert_ok!(VtokenVoting::update_lock(&ALICE, vtoken, &c));
		assert_eq!(usable_balance(vtoken, &ALICE), 10);
	});
}

#[test]
fn successful_conviction_vote_balance_stays_locked_for_correct_time() {
	new_test_ext().execute_with(|| {
		let poll_index = 3;
		let vtoken = VKSM;

		for i in 1..=5 {
			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(i),
				vtoken,
				poll_index,
				aye(10, i as u8)
			));
		}
		let c = class(vtoken, poll_index);
		assert_ok!(VtokenVoting::set_referendum_status(
			RuntimeOrigin::signed(CONTROLLER),
			vtoken,
			poll_index,
			ReferendumInfoOf::<Runtime>::Completed(3),
		));
		for i in 1..=5 {
			assert_ok!(VtokenVoting::try_remove_vote(&i, vtoken, poll_index, UnvoteScope::Any));
		}
		for block in 1..=(3 + 5 * 3) {
			run_to(block);
			for i in 1..=5 {
				assert_ok!(VtokenVoting::update_lock(&i, vtoken, &c));
				let _expired = block >= (3 << (i - 1)) + 3;
				// assert_eq!(
				// 	usable_balance(vtoken, &i),
				// 	i as u128 * 10 - if expired { 0 } else { 10 }
				// );
			}
		}
	});
}

#[test]
fn notify_vote_success_works() {
	new_test_ext().execute_with(|| {
		let poll_index = 3;
		let vtoken = VKSM;
		let query_id = 0;
		let response = Response::DispatchResult(MaybeErrorCode::Success);

		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, aye(2, 5)));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(10, 0, 2));
		System::assert_last_event(RuntimeEvent::VtokenVoting(Event::Voted {
			who: ALICE,
			vtoken,
			poll_index,
			vote: aye(2, 5),
		}));

		assert_ok!(VtokenVoting::notify_vote(
			pallet_xcm::Origin::Response(Parent.into()).into(),
			query_id,
			response,
		));
	});
}

#[test]
fn notify_vote_fail_works() {
	new_test_ext().execute_with(|| {
		let poll_index = 3;
		let vtoken = VKSM;
		let query_id = 0;
		let response = Response::DispatchResult(MaybeErrorCode::Error(
			BoundedVec::try_from(vec![0u8, 1u8]).unwrap(),
		));

		assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, poll_index, aye(2, 5)));
		assert_eq!(tally(vtoken, poll_index), Tally::from_parts(10, 0, 2));
		System::assert_last_event(RuntimeEvent::VtokenVoting(Event::Voted {
			who: ALICE,
			vtoken,
			poll_index,
			vote: aye(2, 5),
		}));

		assert_ok!(VtokenVoting::notify_vote(origin_response(Parent.into()), query_id, response,));
	});
}

#[test]
fn notify_vote_with_no_data_works() {
	new_test_ext().execute_with(|| {
		let query_id = 0;
		let response = Response::DispatchResult(MaybeErrorCode::Success);

		assert_ok!(VtokenVoting::notify_vote(origin_response(Parent.into()), query_id, response,));
	});
}

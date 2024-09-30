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

// Ensure we're `no_std` when compiling for Wasm.
use crate::{mock::*, *};
use bifrost_primitives::currency::VPHA;
use frame_support::{
	assert_noop, assert_ok,
	traits::{
		fungibles::Inspect,
		tokens::{Fortitude::Polite, Preservation::Expendable},
	},
};

const TOKENS: &[CurrencyId] = if cfg!(feature = "polkadot") { &[VBNC] } else { &[] };

fn aye(amount: Balance, conviction: u8) -> AccountVote<Balance> {
	let vote = Vote { aye: true, conviction: conviction.try_into().unwrap() };
	AccountVote::Standard { vote, balance: amount }
}

fn nay(amount: Balance, conviction: u8) -> AccountVote<Balance> {
	let vote = Vote { aye: false, conviction: conviction.try_into().unwrap() };
	AccountVote::Standard { vote, balance: amount }
}

fn tally(vtoken: CurrencyId, poll_index: u32) -> TallyOf<Runtime> {
	VtokenVoting::ensure_referendum_ongoing(vtoken, poll_index)
		.expect("No poll")
		.tally
}

fn usable_balance(vtoken: CurrencyId, who: &AccountId) -> Balance {
	Tokens::reducible_balance(vtoken, who, Expendable, Polite)
}

#[test]
fn basic_voting_works() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			let poll_index = 3;

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index,
				aye(2, 5)
			));
			assert_eq!(tally(vtoken, poll_index), Tally::from_parts(20, 0, 4));
			System::assert_last_event(RuntimeEvent::VtokenVoting(Event::Voted {
				who: ALICE,
				vtoken,
				poll_index,
				token_vote: aye(4, 5),
				delegator_vote: aye(200, 0),
			}));

			assert_ok!(VtokenVoting::try_remove_vote(&ALICE, vtoken, poll_index, UnvoteScope::Any));
			assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 0, 0));

			assert_ok!(VtokenVoting::update_lock(&ALICE, vtoken));
			assert_eq!(usable_balance(vtoken, &ALICE), 10);
		});
	}
}

#[test]
fn voting_balance_gets_locked() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			let poll_index = 3;

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index,
				nay(10, 0)
			));
			assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 2, 0));
			assert_eq!(usable_balance(vtoken, &ALICE), 0);

			assert_ok!(VtokenVoting::try_remove_vote(&ALICE, vtoken, poll_index, UnvoteScope::Any));
			assert_eq!(tally(vtoken, poll_index), Tally::from_parts(0, 0, 0));

			assert_ok!(VtokenVoting::update_lock(&ALICE, vtoken));
			assert_eq!(usable_balance(vtoken, &ALICE), 10);
		});
	}
}

#[test]
fn successful_but_zero_conviction_vote_balance_can_be_unlocked() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			let poll_index = 3;

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index,
				aye(1, 1)
			));
			assert_eq!(usable_balance(vtoken, &ALICE), 9);
			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index,
				aye(3, 1)
			));
			assert_eq!(usable_balance(vtoken, &ALICE), 7);

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(BOB),
				vtoken,
				poll_index,
				nay(20, 0)
			));
			assert_eq!(usable_balance(vtoken, &BOB), 0);

			assert_ok!(VtokenVoting::set_vote_locking_period(RuntimeOrigin::root(), vtoken, 10));
			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				poll_index,
				ReferendumInfoOf::<Runtime>::Completed(3),
			));

			assert_ok!(VtokenVoting::unlock(RuntimeOrigin::signed(BOB), vtoken, poll_index));
			assert_eq!(usable_balance(vtoken, &BOB), 20);

			RelaychainDataProvider::set_block_number(13);
			assert_ok!(VtokenVoting::unlock(RuntimeOrigin::signed(ALICE), vtoken, poll_index));
			assert_eq!(usable_balance(vtoken, &ALICE), 10);
		});
	}
}

#[test]
fn unsuccessful_conviction_vote_balance_can_be_unlocked() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			let poll_index = 3;
			let locking_period = 10;
			assert_ok!(VtokenVoting::set_vote_locking_period(
				RuntimeOrigin::root(),
				vtoken,
				locking_period,
			));

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index,
				aye(1, 1)
			));
			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(BOB),
				vtoken,
				poll_index,
				nay(20, 0)
			));

			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				poll_index,
				ReferendumInfoOf::<Runtime>::Completed(3),
			));
			RelaychainDataProvider::set_block_number(13);
			assert_ok!(VtokenVoting::try_remove_vote(&ALICE, vtoken, poll_index, UnvoteScope::Any));
			assert_ok!(VtokenVoting::update_lock(&ALICE, vtoken));
			assert_eq!(usable_balance(vtoken, &ALICE), 10);
		});
	}
}

#[test]
fn ensure_balance_after_unlock() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			let poll_index = 3;
			let poll_index_2 = 4;
			let locking_period = 10;
			assert_ok!(VtokenVoting::set_vote_locking_period(
				RuntimeOrigin::root(),
				vtoken,
				locking_period,
			));

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index,
				aye(10, 1)
			));
			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index_2,
				aye(10, 5)
			));

			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				poll_index,
				ReferendumInfoOf::<Runtime>::Completed(3),
			));
			RelaychainDataProvider::set_block_number(13);
			assert_ok!(VtokenVoting::unlock(RuntimeOrigin::signed(ALICE), vtoken, poll_index));
			assert_eq!(usable_balance(vtoken, &ALICE), 0);
			assert_eq!(Tokens::accounts(&ALICE, vtoken).frozen, 10);
			assert_eq!(VotingFor::<Runtime>::get(&ALICE).locked_balance(), 10);
		});
	}
}

#[test]
fn ensure_comprehensive_balance_after_unlock() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			let poll_index = 3;
			let poll_index_2 = 4;
			let poll_index_3 = 5;
			let locking_period = 10;
			assert_ok!(VtokenVoting::set_vote_locking_period(
				RuntimeOrigin::root(),
				vtoken,
				locking_period,
			));

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index,
				aye(2, 1)
			));
			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index_2,
				aye(1, 5)
			));
			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index_3,
				aye(2, 5)
			));

			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				poll_index,
				ReferendumInfoOf::<Runtime>::Completed(3),
			));
			RelaychainDataProvider::set_block_number(13);
			assert_ok!(VtokenVoting::unlock(RuntimeOrigin::signed(ALICE), vtoken, poll_index));
			assert_eq!(usable_balance(vtoken, &ALICE), 8);
			assert_eq!(Tokens::accounts(&ALICE, vtoken).frozen, 2);
			assert_eq!(VotingFor::<Runtime>::get(&ALICE).locked_balance(), 2);

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index_2,
				aye(10, 5)
			));

			assert_eq!(usable_balance(vtoken, &ALICE), 0);
			assert_eq!(Tokens::accounts(&ALICE, vtoken).frozen, 10);
			assert_eq!(VotingFor::<Runtime>::get(&ALICE).locked_balance(), 10);
		});
	}
}

#[test]
fn successful_conviction_vote_balance_stays_locked_for_correct_time() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			let poll_index = 3;
			let locking_period = 10;
			assert_ok!(VtokenVoting::set_vote_locking_period(
				RuntimeOrigin::root(),
				vtoken,
				locking_period,
			));
			for i in 1..=5 {
				assert_ok!(VtokenVoting::vote(
					RuntimeOrigin::signed(i),
					vtoken,
					poll_index,
					aye(10, i as u8)
				));
			}
			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				poll_index,
				ReferendumInfoOf::<Runtime>::Completed(3),
			));
			RelaychainDataProvider::set_block_number(163);
			for i in 1..=5 {
				assert_ok!(VtokenVoting::try_remove_vote(&i, vtoken, poll_index, UnvoteScope::Any));
			}
			for i in 1..=5 {
				assert_ok!(VtokenVoting::update_lock(&i, vtoken));
				assert_eq!(usable_balance(vtoken, &i), 10 * i as u128);
			}
		});
	}
}

#[test]
fn lock_amalgamation_valid_with_multiple_removed_votes() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, 0, aye(5, 1)));
			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![(vtoken, 5),])
					.unwrap()
			);

			assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, 1, aye(10, 1)));
			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![(vtoken, 10),])
					.unwrap()
			);

			assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, 1, aye(5, 1)));
			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![(vtoken, 5),])
					.unwrap()
			);
			assert_eq!(usable_balance(vtoken, &ALICE), 5);

			assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, 2, aye(10, 2)));
			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![(vtoken, 10),])
					.unwrap()
			);

			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				0,
				ReferendumInfoOf::<Runtime>::Completed(1),
			));
			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				1,
				ReferendumInfoOf::<Runtime>::Completed(1),
			));
			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				2,
				ReferendumInfoOf::<Runtime>::Completed(1),
			));

			let locking_period = 10;
			assert_ok!(VtokenVoting::set_vote_locking_period(
				RuntimeOrigin::root(),
				vtoken,
				locking_period,
			));
			assert_eq!(VoteLockingPeriod::<Runtime>::get(vtoken), Some(10));

			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![(vtoken, 10),])
					.unwrap()
			);

			RelaychainDataProvider::set_block_number(10);
			assert_noop!(
				VtokenVoting::unlock(RuntimeOrigin::signed(ALICE), vtoken, 0),
				Error::<Runtime>::NoPermissionYet
			);
			assert_eq!(VotingFor::<Runtime>::get(&ALICE).locked_balance(), 10);
			assert_eq!(usable_balance(vtoken, &ALICE), 0);

			RelaychainDataProvider::set_block_number(11);
			assert_ok!(VtokenVoting::unlock(RuntimeOrigin::signed(ALICE), vtoken, 0));
			assert_eq!(VotingFor::<Runtime>::get(&ALICE).locked_balance(), 10);
			assert_eq!(usable_balance(vtoken, &ALICE), 0);
			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![(vtoken, 10),])
					.unwrap()
			);

			RelaychainDataProvider::set_block_number(11);
			assert_ok!(VtokenVoting::unlock(RuntimeOrigin::signed(ALICE), vtoken, 1));
			assert_eq!(usable_balance(vtoken, &ALICE), 0);
			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![(vtoken, 10)])
					.unwrap()
			);

			RelaychainDataProvider::set_block_number(21);
			assert_ok!(VtokenVoting::unlock(RuntimeOrigin::signed(ALICE), vtoken, 2));
			assert_eq!(usable_balance(vtoken, &ALICE), 10);
			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![]).unwrap()
			);
		});
	}
}

#[test]
fn removed_votes_when_referendum_killed() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, 0, aye(5, 1)));
			assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, 1, aye(10, 1)));
			assert_ok!(VtokenVoting::vote(RuntimeOrigin::signed(ALICE), vtoken, 2, aye(5, 2)));
			assert_eq!(usable_balance(vtoken, &ALICE), 0);

			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				0,
				ReferendumInfoOf::<Runtime>::Completed(1),
			));
			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				1,
				ReferendumInfoOf::<Runtime>::Completed(1),
			));
			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				2,
				ReferendumInfoOf::<Runtime>::Completed(1),
			));

			assert_ok!(VtokenVoting::kill_referendum(RuntimeOrigin::root(), vtoken, 0));
			assert_ok!(VtokenVoting::kill_referendum(RuntimeOrigin::root(), vtoken, 1));
			assert_ok!(VtokenVoting::kill_referendum(RuntimeOrigin::root(), vtoken, 2));

			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![(vtoken, 10),])
					.unwrap()
			);

			assert_ok!(VtokenVoting::unlock(RuntimeOrigin::signed(ALICE), vtoken, 0));
			assert_eq!(usable_balance(vtoken, &ALICE), 0);
			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![(vtoken, 10),])
					.unwrap()
			);

			assert_ok!(VtokenVoting::unlock(RuntimeOrigin::signed(ALICE), vtoken, 1));
			assert_eq!(usable_balance(vtoken, &ALICE), 5);
			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![(vtoken, 5)])
					.unwrap()
			);

			assert_ok!(VtokenVoting::unlock(RuntimeOrigin::signed(ALICE), vtoken, 2));
			assert_eq!(usable_balance(vtoken, &ALICE), 10);
			assert_eq!(
				ClassLocksFor::<Runtime>::get(&ALICE),
				BoundedVec::<(CurrencyId, u128), ConstU32<256>>::try_from(vec![]).unwrap()
			);
		});
	}
}

#[test]
fn errors_with_vote_works() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			assert_noop!(
				VtokenVoting::vote(RuntimeOrigin::signed(1), VPHA, 0, aye(10, 0)),
				Error::<Runtime>::VTokenNotSupport
			);
			assert_noop!(
				VtokenVoting::vote(RuntimeOrigin::signed(1), vtoken, 3, aye(11, 0)),
				Error::<Runtime>::InsufficientFunds
			);

			for poll_index in 0..256 {
				assert_ok!(VtokenVoting::vote(
					RuntimeOrigin::signed(1),
					vtoken,
					poll_index,
					aye(10, 0)
				));
			}
			assert_noop!(
				VtokenVoting::vote(RuntimeOrigin::signed(1), vtoken, 256, aye(10, 0)),
				Error::<Runtime>::MaxVotesReached
			);
		});
	}
}

#[test]
fn kill_referendum_works() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			let poll_index = 3;

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index,
				aye(5, 1)
			));
			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				poll_index,
				ReferendumInfoOf::<Runtime>::Completed(1),
			));
			assert_ok!(VtokenVoting::kill_referendum(RuntimeOrigin::root(), vtoken, poll_index));
			System::assert_last_event(RuntimeEvent::VtokenVoting(Event::ReferendumKilled {
				vtoken,
				poll_index,
			}));
		});
	}
}

#[test]
fn kill_referendum_with_origin_signed_fails() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			let poll_index = 3;

			assert_ok!(VtokenVoting::vote(
				RuntimeOrigin::signed(ALICE),
				vtoken,
				poll_index,
				aye(5, 1)
			));
			assert_ok!(VtokenVoting::set_referendum_status(
				RuntimeOrigin::root(),
				vtoken,
				poll_index,
				ReferendumInfoOf::<Runtime>::Completed(1),
			));
			assert_noop!(
				VtokenVoting::kill_referendum(RuntimeOrigin::signed(ALICE), vtoken, poll_index),
				DispatchError::BadOrigin
			);
		});
	}
}

#[test]
fn compute_delegator_total_vote_works() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(10, 0)),
				Ok(aye(10, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(2, 1)),
				Ok(aye(20, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(2, 2)),
				Ok(aye(40, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(2, 3)),
				Ok(aye(60, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(2, 4)),
				Ok(aye(80, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(2, 5)),
				Ok(aye(100, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(2, 6)),
				Ok(aye(120, 0))
			);

			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, nay(10, 0)),
				Ok(nay(10, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, nay(2, 1)),
				Ok(nay(20, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, nay(2, 2)),
				Ok(nay(40, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, nay(2, 3)),
				Ok(nay(60, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, nay(2, 4)),
				Ok(nay(80, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, nay(2, 5)),
				Ok(nay(100, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, nay(2, 6)),
				Ok(nay(120, 0))
			);

			SimpleVTokenSupplyProvider::set_token_supply(10_000_000);
			assert_eq!(VtokenVoting::vote_cap(vtoken), Ok(1_000_000));
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(1_000_000, 0)),
				Ok(aye(1_000_000, 0))
			);
			for i in 1..=6_u8 {
				assert_eq!(
					VtokenVoting::compute_delegator_total_vote(
						vtoken,
						aye(10_000_000 * i as Balance, 0)
					),
					Ok(aye(1_000_000, i))
				);
			}

			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(100_000, 1)),
				Ok(aye(1_000_000, 0))
			);
			for i in 1..=6_u8 {
				assert_eq!(
					VtokenVoting::compute_delegator_total_vote(
						vtoken,
						aye(1_000_000 * i as Balance, 1)
					),
					Ok(aye(1_000_000, i))
				);
			}
			assert_noop!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(6_000_006, 1)),
				Error::<Runtime>::InsufficientFunds
			);

			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(50_000, 2)),
				Ok(aye(1_000_000, 0))
			);
			for i in 1..=6_u8 {
				assert_eq!(
					VtokenVoting::compute_delegator_total_vote(
						vtoken,
						aye(500_000 * i as Balance, 2)
					),
					Ok(aye(1_000_000, i))
				);
			}
			assert_noop!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(3_000_003, 2)),
				Error::<Runtime>::InsufficientFunds
			);

			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(33_333, 3)),
				Ok(aye(999_990, 0))
			);
			for i in 1..=6_u8 {
				assert_eq!(
					VtokenVoting::compute_delegator_total_vote(
						vtoken,
						aye(333_333 * i as Balance, 3)
					),
					Ok(aye(999_999, i))
				);
			}
			assert_noop!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(2_000_002, 3)),
				Error::<Runtime>::InsufficientFunds
			);

			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(25_000, 4)),
				Ok(aye(1_000_000, 0))
			);
			for i in 1..=6_u8 {
				assert_eq!(
					VtokenVoting::compute_delegator_total_vote(
						vtoken,
						aye(250_000 * i as Balance, 4)
					),
					Ok(aye(1_000_000, i))
				);
			}
			assert_noop!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(1_500_002, 4)),
				Error::<Runtime>::InsufficientFunds
			);

			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(20_000, 5)),
				Ok(aye(1_000_000, 0))
			);
			for i in 1..=6_u8 {
				assert_eq!(
					VtokenVoting::compute_delegator_total_vote(
						vtoken,
						aye(200_000 * i as Balance, 5)
					),
					Ok(aye(1_000_000, i))
				);
			}
			assert_noop!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(1_200_002, 5)),
				Error::<Runtime>::InsufficientFunds
			);

			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(16_666, 6)),
				Ok(aye(999_960, 0))
			);
			for i in 1..=6_u8 {
				assert_eq!(
					VtokenVoting::compute_delegator_total_vote(
						vtoken,
						aye(166_666 * i as Balance, 6)
					),
					Ok(aye(999_996, i))
				);
			}
			assert_noop!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(1_000_001, 6)),
				Error::<Runtime>::InsufficientFunds
			);
		});
	}
}

#[test]
fn compute_delegator_total_vote_with_low_value_will_loss() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, aye(9, 0)),
				Ok(aye(0, 0))
			);
			assert_eq!(
				VtokenVoting::compute_delegator_total_vote(vtoken, nay(9, 0)),
				Ok(nay(0, 0))
			);
		});
	}
}

#[test]
fn allocate_delegator_votes_works() {
	for &vtoken in TOKENS {
		new_test_ext().execute_with(|| {
			let poll_index = 3;

			for conviction in 0..=6 {
				let vote = aye(5e9 as Balance, conviction);
				let delegator_votes =
					VtokenVoting::allocate_delegator_votes(vtoken, poll_index, vote);
				assert_eq!(
					delegator_votes,
					Ok(vec![(0, aye(4294967295, conviction)), (1, aye(705032705, conviction))])
				);
				assert_eq!(
					delegator_votes
						.unwrap()
						.into_iter()
						.map(|(_derivative_index, vote)| vote)
						.fold(aye(0, conviction), |mut acc, vote| {
							let _ = acc.checked_add(vote);
							acc
						},),
					vote
				);
			}

			for conviction in 0..=6 {
				let vote = aye(3e10 as Balance, conviction);
				let delegator_votes =
					VtokenVoting::allocate_delegator_votes(vtoken, poll_index, vote);
				assert_eq!(
					delegator_votes,
					Ok(vec![
						(0, aye(4294967295, conviction)),
						(1, aye(4294967295, conviction)),
						(2, aye(4294967295, conviction)),
						(3, aye(4294967295, conviction)),
						(4, aye(4294967295, conviction)),
						(5, aye(4294967295, conviction)),
						(10, aye(4230196230, conviction))
					])
				);
				assert_eq!(
					delegator_votes
						.unwrap()
						.into_iter()
						.map(|(_derivative_index, vote)| vote)
						.fold(aye(0, conviction), |mut acc, vote| {
							let _ = acc.checked_add(vote);
							acc
						},),
					vote
				);
			}
		});
	}
}

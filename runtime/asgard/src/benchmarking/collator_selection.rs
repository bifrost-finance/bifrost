// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use frame_benchmarking::{account, whitelisted_caller};
use frame_support::{assert_ok, pallet_prelude::Decode, traits::Currency};
use frame_system::RawOrigin;
// use pallet_collator_selection::POINT_PER_BLOCK;
use orml_benchmarking::{runtime_benchmarks, whitelist_account};
use orml_traits::MultiCurrencyExtended;
use pallet_authorship::EventHandler;
use pallet_session::SessionManager;
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

use crate::{
	AccountId, Balance, Balances, CollatorSelection, Currencies, CurrencyId, Event, MaxCandidates,
	MaxInvulnerables, NativeCurrencyId, Runtime, Session, SessionKeys, System,
};

const SEED: u32 = 0;
const NATIVE: CurrencyId = NativeCurrencyId::get();

pub fn set_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) {
	assert_ok!(<Currencies as MultiCurrencyExtended<_>>::update_balance(
		currency_id,
		who,
		balance.saturated_into()
	));
}

fn assert_last_event(generic_event: Event) {
	System::assert_last_event(generic_event.into());
}

fn register_candidates(count: u32) {
	let candidates = (0..count).map(|c| account("candidate", c, SEED)).collect::<Vec<_>>();
	assert!(
		pallet_collator_selection::CandidacyBond::<Runtime>::get() > Balance::from(0u32),
		"Bond cannot be zero!"
	);

	for (index, who) in candidates.iter().enumerate() {
		Balances::make_free_balance_be(
			&who,
			pallet_collator_selection::CandidacyBond::<Runtime>::get()
				.checked_mul(2u32.into())
				.unwrap(),
		);
		let mut keys = [1u8; 128];
		keys[0..4].copy_from_slice(&(index as u32).to_be_bytes());
		let keys: SessionKeys = Decode::decode(&mut &keys[..]).unwrap();
		Session::set_keys(RawOrigin::Signed(who.clone()).into(), keys, vec![]).unwrap();
		CollatorSelection::register_as_candidate(RawOrigin::Signed(who.clone()).into()).unwrap();
	}
}

runtime_benchmarks! {
	{ Runtime, pallet_collator_selection }

	set_invulnerables {
		let b in 1 .. MaxInvulnerables::get();
		let new_invulnerables = (0..b).map(|c| account("candidate", c, SEED)).collect::<Vec<_>>();
	}: {
		assert_ok!(
			CollatorSelection::set_invulnerables(RawOrigin::Root.into(), new_invulnerables.clone())
		);
	}
	verify {
		assert_last_event(pallet_collator_selection::Event::NewInvulnerables(new_invulnerables).into());
	}

	set_desired_candidates {
		let max: u32 = MaxInvulnerables::get();
	}: {
		assert_ok!(
			CollatorSelection::set_desired_candidates(RawOrigin::Root.into(), max.clone())
		);
	}
	verify {
		assert_last_event(pallet_collator_selection::Event::NewDesiredCandidates(max).into());
	}

	set_candidacy_bond {
		let bond: Balance = Balances::minimum_balance().checked_mul(10u32.into()).unwrap();
	}: {
		assert_ok!(
			CollatorSelection::set_candidacy_bond(RawOrigin::Root.into(), bond.clone())
		);
	}
	verify {
		assert_last_event(pallet_collator_selection::Event::NewCandidacyBond(bond).into());
	}

	// worse case is when we have all the max-candidate slots filled except one, and we fill that
	// one.
	register_as_candidate {
		// MinCandidates = 5, so begin with 5.
		let c in 5 .. MaxCandidates::get();
		pallet_collator_selection::CandidacyBond::<Runtime>::put(Balances::minimum_balance());
		pallet_collator_selection::DesiredCandidates::<Runtime>::put(c);
		register_candidates(c-1);

		let caller: AccountId = whitelisted_caller();
		set_balance(NATIVE, &caller, Balances::minimum_balance());
		Session::set_keys(RawOrigin::Signed(caller.clone()).into(), SessionKeys::default(), vec![]).unwrap();
	}: _(RawOrigin::Signed(caller.clone()))

	// worse case is the last candidate leaving.
	leave_intent {
		// MinCandidates = 5, so begin with 6.
		let c in 6 .. MaxCandidates::get();
		pallet_collator_selection::CandidacyBond::<Runtime>::put(Balances::minimum_balance());
		pallet_collator_selection::DesiredCandidates::<Runtime>::put(c);
		register_candidates(c);

		let leaving = pallet_collator_selection::Candidates::<Runtime>::get().last().unwrap().who.clone();
		whitelist_account!(leaving);
	}: _(RawOrigin::Signed(leaving.clone()))
	verify {
		assert_last_event(pallet_collator_selection::Event::CandidateRemoved(leaving).into());
	}

	// worse case is paying a non-existing candidate account.
	note_author {
		let c = MaxCandidates::get();
		pallet_collator_selection::CandidacyBond::<Runtime>::put(Balances::minimum_balance());
		pallet_collator_selection::DesiredCandidates::<Runtime>::put(c);
		register_candidates(c);

		Balances::make_free_balance_be(
			&CollatorSelection::account_id(),
			Balances::minimum_balance().checked_mul(2u32.into()).unwrap()
		);
		let author = account("author", 0, SEED);
		Balances::make_free_balance_be(
			&author,
			Balances::minimum_balance()
		);
		assert!(Balances::free_balance(&author) == Balances::minimum_balance());
	}: {
		CollatorSelection::note_author(author.clone())
	}

	// worse case is on new session.
	new_session {
		let c in 1 .. MaxCandidates::get();
		let r in 1 .. MaxCandidates::get();
		pallet_collator_selection::CandidacyBond::<Runtime>::put(Balances::minimum_balance());
		pallet_collator_selection::DesiredCandidates::<Runtime>::put(c);
		System::set_block_number(0u32.into());
		register_candidates(c);

		System::set_block_number(20u32.into());

		assert!(pallet_collator_selection::Candidates::<Runtime>::get().len() == c as usize);
	}: {
		CollatorSelection::new_session(0)
	}
}

#[cfg(test)]
mod tests {
	use orml_benchmarking::impl_benchmark_test_suite;

	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;

	impl_benchmark_test_suite!(new_test_ext(),);
}

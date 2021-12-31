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

// Ensure we're `no_std` when compiling for Wasm.
#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use node_primitives::ParaId;
use sp_runtime::{traits::Bounded, SaturatedConversion};
use sp_std::prelude::*;

pub use crate::{Pallet as Salp, *};

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let frame_system::EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

fn create_fund<T: Config>(id: u32) -> ParaId {
	let cap = BalanceOf::<T>::max_value();
	let first_period = (0 as u32).into();
	let last_period = (7 as u32).into();
	let para_id = id;

	assert_ok!(Salp::<T>::create(RawOrigin::Root.into(), para_id, cap, first_period, last_period));

	para_id
}

#[allow(dead_code)]
fn contribute_fund<T: Config>(who: &T::AccountId, index: ParaId) {
	let value = T::MinContribution::get();
	assert_ok!(Salp::<T>::set_balance(who, value));
	assert_ok!(Salp::<T>::contribute(RawOrigin::Signed(who.clone()).into(), index, value));
}

benchmarks! {
	contribute {
		let fund_index = create_fund::<T>(1);
		let caller: T::AccountId = whitelisted_caller();
		let contribution = T::MinContribution::get();
		assert_ok!(Salp::<T>::set_balance(&caller, contribution));
	}: _(RawOrigin::Signed(caller.clone()), fund_index, contribution)
	verify {
		let fund = Salp::<T>::funds(fund_index).unwrap();
		let (_, status) = Salp::<T>::contribution(fund.trie_index, &caller);
		assert_eq!(status, ContributionStatus::Contributing(contribution));
	}

	refund {
		let fund_index = create_fund::<T>(1);
		let caller: T::AccountId = whitelisted_caller();
		let caller_origin: T::Origin = RawOrigin::Signed(caller.clone()).into();
		let contribution = T::MinContribution::get();
		let confirmer: T::Origin = RawOrigin::Signed(Salp::<T>::multisig_confirm_account()).into();
		contribute_fund::<T>(&caller,fund_index);
		assert_ok!(Salp::<T>::confirm_contribute(
			confirmer,
			caller.clone(),
			fund_index,
			true,
			[0; 32]
		));
		assert_ok!(Salp::<T>::fund_fail(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::withdraw(RawOrigin::Root.into(), fund_index));
		let fund = Salp::<T>::funds(fund_index).unwrap();
		let (_, status) = Salp::<T>::contribution(fund.trie_index, &caller);
		assert_eq!(status, ContributionStatus::Idle);
	}: _(RawOrigin::Signed(caller.clone()), fund_index)
	verify {
		let (_, status) = Salp::<T>::contribution(fund.trie_index, &caller);
		assert_eq!(status, ContributionStatus::Idle);
		assert_last_event::<T>(Event::<T>::Refunded(caller.clone(), fund_index, contribution).into())
	}

	unlock {
		let fund_index = create_fund::<T>(1);
		let caller: T::AccountId = whitelisted_caller();
		let caller_origin: T::Origin = RawOrigin::Signed(caller.clone()).into();
		let contribution = T::MinContribution::get();
		let confirmer: T::Origin = RawOrigin::Signed(Salp::<T>::multisig_confirm_account()).into();
		contribute_fund::<T>(&caller,fund_index);
		assert_ok!(Salp::<T>::confirm_contribute(
			confirmer,
			caller.clone(),
			fund_index,
			true,
			[0; 32]
		));
		assert_ok!(Salp::<T>::fund_success(RawOrigin::Root.into(), fund_index));
	}: _(RawOrigin::Root, caller.clone(),fund_index)
	verify {
		let fund = Salp::<T>::funds(fund_index).unwrap();
		let (_, status) = Salp::<T>::contribution(fund.trie_index, &caller);
		assert_eq!(status, ContributionStatus::Unlocked);
	}

	batch_unlock {
		let k in 1 .. T::RemoveKeysLimit::get();
		let fund_index = create_fund::<T>(1);
		let contribution = T::MinContribution::get();
		let mut caller: T::AccountId = whitelisted_caller();
		let confirmer: T::Origin = RawOrigin::Signed(Salp::<T>::multisig_confirm_account()).into();
		for i in 0 .. k {
			caller = account("contributor", i, 0);
			contribute_fund::<T>(&caller,fund_index);
			let _ = Salp::<T>::confirm_contribute(
				confirmer.clone(),
				caller.clone(),
				fund_index,
				true,
				[0; 32]
			);
		}
		assert_ok!(Salp::<T>::fund_success(RawOrigin::Root.into(), fund_index));
	}: _(RawOrigin::Signed(caller.clone()), fund_index)
	verify {
		let fund = Salp::<T>::funds(fund_index).unwrap();
		let (_, status) = Salp::<T>::contribution(fund.trie_index, &caller);
		assert_eq!(status, ContributionStatus::Unlocked);
		assert_last_event::<T>(Event::<T>::AllUnlocked(fund_index).into());
	}

	redeem {
		let fund_index = create_fund::<T>(1);
		let caller: T::AccountId = whitelisted_caller();
		let caller_origin: T::Origin = RawOrigin::Signed(caller.clone()).into();
		let contribution = T::MinContribution::get();
		let confirmer: T::Origin = RawOrigin::Signed(Salp::<T>::multisig_confirm_account()).into();
		contribute_fund::<T>(&caller,fund_index);
		assert_ok!(Salp::<T>::confirm_contribute(
			confirmer,
			caller.clone(),
			fund_index,
			true,
			[0; 32]
		));
		assert_ok!(Salp::<T>::fund_success(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::unlock(RawOrigin::Root.into(), caller.clone(), fund_index));
		assert_ok!(Salp::<T>::fund_retire(RawOrigin::Root.into(), fund_index));
		assert_ok!(Salp::<T>::withdraw(RawOrigin::Root.into(), fund_index));
		assert_eq!(Salp::<T>::redeem_pool(), T::MinContribution::get());
	}: _(RawOrigin::Signed(caller.clone()), fund_index,contribution)
	verify {
		assert_eq!(Salp::<T>::redeem_pool(), 0_u32.saturated_into());
		assert_last_event::<T>(Event::<T>::Redeemed(caller.clone(), fund_index, (0 as u32).into(),(7 as u32).into(),contribution).into())
	}
}

impl_benchmark_test_suite!(Salp, crate::mock::new_test_ext(), crate::mock::Test);

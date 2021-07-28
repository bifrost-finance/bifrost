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
use sp_runtime::traits::Bounded;
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

	let caller = account("fund_creator", id, 0);

	assert_ok!(<T as Config>::MultiCurrency::deposit(
		<T as Config>::DepositToken::get(),
		&caller,
		T::SubmissionDeposit::get()
	));

	assert_ok!(Salp::<T>::create(
		RawOrigin::Signed(caller).into(),
		para_id,
		cap,
		first_period,
		last_period,
	));

	para_id
}

#[allow(dead_code)]
fn contribute_fund<T: Config>(who: &T::AccountId, index: ParaId) {
	let value = T::SubmissionDeposit::get();

	assert_ok!(Salp::<T>::contribute(RawOrigin::Signed(who.clone()).into(), index, value));
}

benchmarks! {
	create {
		let para_id = 1 as u32;
		let cap = BalanceOf::<T>::max_value();
		let first_period = 0u32.into();
		let last_period = 3u32.into();

		let caller: T::AccountId = whitelisted_caller();

		<T as Config>::MultiCurrency::deposit(
			<T as Config>::DepositToken::get(),
			&caller,
			T::SubmissionDeposit::get(),
		)?;

	}: _(RawOrigin::Signed(caller), para_id, cap, first_period, last_period)
	verify {
		assert_last_event::<T>(Event::<T>::Created(para_id).into())
	}

	contribute {
		let fund_index = create_fund::<T>(1);
		let caller: T::AccountId = whitelisted_caller();
		let contribution = T::MinContribution::get();

	}: _(RawOrigin::Signed(caller.clone()), fund_index, contribution)
	verify {
		assert_last_event::<T>(Event::<T>::Contributing(caller, fund_index, contribution).into());
	}

	on_finalize {
		let end_block: T::BlockNumber = T::ReleaseCycle::get();
		let n in 2 .. 100;

		for i in 0 .. n {
			let fund_index = create_fund::<T>(i);
			let contributor: T::AccountId = account("contributor", i, 0);
			let contribution = T::MinContribution::get() * (i + 1).into();

			Salp::<T>::contribute(RawOrigin::Signed(contributor).into(), fund_index, contribution)?;
		}
	}: {
		Salp::<T>::on_finalize(end_block);
	}
}

impl_benchmark_test_suite!(Salp, crate::mock::new_test_ext(), crate::mock::Test);

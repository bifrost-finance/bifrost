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

use crate::*;
use assert_matches::assert_matches;
use bifrost_primitives::{currency::VKSM, XcmOperationType as XcmOperation};
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_conviction_voting::{Conviction, Vote};
use sp_runtime::traits::UniqueSaturatedFrom;

const SEED: u32 = 0;

fn funded_account<T: Config>(name: &'static str, index: u32) -> AccountIdOf<T> {
	let caller = account(name, index, SEED);
	assert_ok!(T::MultiCurrency::deposit(
		VKSM,
		&caller,
		BalanceOf::<T>::unique_saturated_from(1000000000000u128)
	));
	caller
}

fn account_vote<T: Config>(b: BalanceOf<T>) -> AccountVote<BalanceOf<T>> {
	let v = Vote { aye: true, conviction: Conviction::Locked1x };

	AccountVote::Standard { vote: v, balance: b }
}

fn init_vote<T: Config>(vtoken: CurrencyIdOf<T>) -> Result<(), BenchmarkError> {
	let derivative_index = 0;
	let token = CurrencyId::to_token(&vtoken).unwrap();
	T::XcmDestWeightAndFee::set_xcm_dest_weight_and_fee(
		token,
		XcmOperation::Vote,
		Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
	)?;
	T::DerivativeAccount::init_minimums_and_maximums(token);
	T::DerivativeAccount::add_delegator(token, derivative_index, xcm::v3::Parent.into());
	T::DerivativeAccount::new_delegator_ledger(token, xcm::v3::Parent.into());
	Pallet::<T>::set_undeciding_timeout(RawOrigin::Root.into(), vtoken, Zero::zero())?;
	Pallet::<T>::add_delegator(RawOrigin::Root.into(), vtoken, derivative_index)?;
	Pallet::<T>::set_vote_cap_ratio(RawOrigin::Root.into(), vtoken, Perbill::from_percent(10))?;

	Ok(())
}

#[benchmarks(where T::MaxVotes: core::fmt::Debug)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn vote_new() -> Result<(), BenchmarkError> {
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let vtoken = VKSM;
		let poll_index = 0u32;
		let vote = account_vote::<T>(100u32.into());
		let control_origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		init_vote::<T>(vtoken)?;
		let r = T::MaxVotes::get() - 1;
		let response = Response::DispatchResult(MaybeErrorCode::Success);
		for (i, index) in (0..T::MaxVotes::get()).collect::<Vec<_>>().iter().skip(1).enumerate() {
			Pallet::<T>::on_idle(Zero::zero(), Weight::MAX);
			Pallet::<T>::vote(RawOrigin::Signed(caller.clone()).into(), vtoken, *index, vote)?;
			Pallet::<T>::notify_vote(
				control_origin.clone() as <T as frame_system::Config>::RuntimeOrigin,
				i as QueryId,
				response.clone(),
			)?;
		}
		let votes = match VotingFor::<T>::get(&caller) {
			Voting::Casting(Casting { votes, .. }) => votes,
			_ => return Err("Votes are not direct".into()),
		};
		assert_eq!(votes.len(), r as usize, "Votes were not recorded.");

		#[extrinsic_call]
		Pallet::<T>::vote(RawOrigin::Signed(caller.clone()), vtoken, poll_index, vote);

		assert_matches!(
			VotingFor::<T>::get(&caller),
			Voting::Casting(Casting { votes, .. }) if votes.len() == (r + 1) as usize
		);

		Ok(())
	}

	#[benchmark]
	fn vote_existing() -> Result<(), BenchmarkError> {
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let vtoken = VKSM;
		let old_vote = account_vote::<T>(100u32.into());
		let control_origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		init_vote::<T>(vtoken)?;
		let r = 50;
		let response = Response::DispatchResult(MaybeErrorCode::Success);
		for index in (0..r).collect::<Vec<_>>().iter() {
			Pallet::<T>::vote(RawOrigin::Signed(caller.clone()).into(), vtoken, *index, old_vote)?;
			Pallet::<T>::notify_vote(
				control_origin.clone() as <T as frame_system::Config>::RuntimeOrigin,
				*index as QueryId,
				response.clone(),
			)?;
		}
		let votes = match VotingFor::<T>::get(&caller) {
			Voting::Casting(Casting { votes, .. }) => votes,
			_ => return Err("Votes are not direct".into()),
		};
		assert_eq!(votes.len(), r as usize, "Votes were not recorded.");

		let poll_index = 1u32;
		let new_vote = account_vote::<T>(200u32.into());
		#[extrinsic_call]
		Pallet::<T>::vote(RawOrigin::Signed(caller.clone()), vtoken, poll_index, new_vote);

		assert_matches!(
			VotingFor::<T>::get(&caller),
			Voting::Casting(Casting { votes, .. }) if votes.len() == r as usize
		);

		Ok(())
	}

	#[benchmark]
	pub fn unlock() -> Result<(), BenchmarkError> {
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let origin = RawOrigin::Signed(caller);
		let vtoken = VKSM;
		let poll_index = 0u32;
		let vote = account_vote::<T>(100u32.into());

		init_vote::<T>(vtoken)?;
		Pallet::<T>::vote(origin.clone().into(), vtoken, poll_index, vote)?;
		Pallet::<T>::set_referendum_status(
			RawOrigin::Root.into(),
			vtoken,
			poll_index,
			ReferendumInfo::Completed(0u32.into()),
		)?;
		Pallet::<T>::set_vote_locking_period(RawOrigin::Root.into(), vtoken, 0u32.into())?;

		let notify_origin =
			T::ResponseOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let query_id = 0u64;
		let response = Response::DispatchResult(MaybeErrorCode::Success);
		Pallet::<T>::notify_vote(notify_origin, query_id, response)?;

		#[extrinsic_call]
		_(origin, vtoken, poll_index);

		Ok(())
	}

	#[benchmark]
	pub fn remove_delegator_vote() -> Result<(), BenchmarkError> {
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let origin = RawOrigin::Signed(caller);
		let vtoken = VKSM;
		let class = 0u16;
		let poll_index = 0u32;
		let vote = account_vote::<T>(100u32.into());
		let derivative_index = 0u16;

		init_vote::<T>(vtoken)?;
		Pallet::<T>::vote(origin.clone().into(), vtoken, poll_index, vote)?;

		let notify_origin =
			T::ResponseOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let query_id = 0u64;
		let response = Response::DispatchResult(MaybeErrorCode::Success);
		Pallet::<T>::notify_vote(notify_origin, query_id, response)?;

		Pallet::<T>::set_referendum_status(
			RawOrigin::Root.into(),
			vtoken,
			poll_index,
			ReferendumInfo::Completed(0u32.into()),
		)?;
		Pallet::<T>::set_vote_locking_period(RawOrigin::Root.into(), vtoken, 0u32.into())?;
		let token = CurrencyId::to_token(&vtoken).unwrap();
		T::XcmDestWeightAndFee::set_xcm_dest_weight_and_fee(
			token,
			XcmOperation::RemoveVote,
			Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into())),
		)?;

		#[extrinsic_call]
		_(origin, vtoken, class, poll_index, derivative_index);

		Ok(())
	}

	#[benchmark]
	pub fn kill_referendum() -> Result<(), BenchmarkError> {
		let origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let poll_index = 0u32;
		let vote = account_vote::<T>(100u32.into());

		init_vote::<T>(vtoken)?;
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let origin_caller = RawOrigin::Signed(caller);
		Pallet::<T>::vote(origin_caller.into(), vtoken, poll_index, vote)?;
		Pallet::<T>::set_referendum_status(
			RawOrigin::Root.into(),
			vtoken,
			poll_index,
			ReferendumInfo::Completed(0u32.into()),
		)?;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, vtoken, poll_index);

		Ok(())
	}

	#[benchmark]
	pub fn add_delegator() -> Result<(), BenchmarkError> {
		let origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let derivative_index = 10;

		init_vote::<T>(vtoken)?;
		T::DerivativeAccount::add_delegator(
			CurrencyId::to_token(&vtoken).unwrap(),
			derivative_index,
			xcm::v3::Parent.into(),
		);

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, vtoken, derivative_index);

		Ok(())
	}

	#[benchmark]
	pub fn set_referendum_status() -> Result<(), BenchmarkError> {
		let origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let poll_index = 0u32;
		let info = ReferendumInfo::Completed(<frame_system::Pallet<T>>::block_number());

		init_vote::<T>(vtoken)?;
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let origin_caller = RawOrigin::Signed(caller);
		let vote = account_vote::<T>(100u32.into());
		Pallet::<T>::vote(origin_caller.into(), vtoken, poll_index, vote)?;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, vtoken, poll_index, info);

		Ok(())
	}

	#[benchmark]
	pub fn set_undeciding_timeout() -> Result<(), BenchmarkError> {
		let origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let vote_locking_period = 100u32.into();

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, vtoken, vote_locking_period);

		Ok(())
	}

	#[benchmark]
	pub fn set_vote_locking_period() -> Result<(), BenchmarkError> {
		let origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let undeciding_timeout = 100u32.into();

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, vtoken, undeciding_timeout);

		Ok(())
	}

	#[benchmark]
	pub fn notify_vote() -> Result<(), BenchmarkError> {
		let origin =
			T::ResponseOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let poll_index = 0u32;
		let query_id = 1u64;
		let response = Response::DispatchResult(MaybeErrorCode::Success);

		init_vote::<T>(vtoken)?;
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let origin_caller = RawOrigin::Signed(caller);
		let vote = account_vote::<T>(100u32.into());
		Pallet::<T>::vote(origin_caller.into(), vtoken, poll_index, vote)?;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, query_id, response);

		Ok(())
	}

	#[benchmark]
	pub fn notify_remove_delegator_vote() -> Result<(), BenchmarkError> {
		let origin =
			T::ResponseOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let poll_index = 0u32;
		let query_id = 1u64;
		let response = Response::DispatchResult(MaybeErrorCode::Success);

		init_vote::<T>(vtoken)?;
		let caller = funded_account::<T>("caller", 0);
		whitelist_account!(caller);
		let origin_caller = RawOrigin::Signed(caller);
		let vote = account_vote::<T>(100u32.into());
		Pallet::<T>::vote(origin_caller.into(), vtoken, poll_index, vote)?;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, query_id, response);

		Ok(())
	}

	#[benchmark]
	pub fn set_vote_cap_ratio() -> Result<(), BenchmarkError> {
		let origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let vote_cap_ratio = Perbill::from_percent(10);

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, vtoken, vote_cap_ratio);

		Ok(())
	}

	// This line generates test cases for benchmarking, and could be run by:
	//   `cargo test -p pallet-example-basic --all-features`, you will see one line per case:
	//   `test benchmarking::bench_sort_vector ... ok`
	//   `test benchmarking::bench_accumulate_dummy ... ok`
	//   `test benchmarking::bench_set_dummy_benchmark ... ok` in the result.
	//
	// The line generates three steps per benchmark, with repeat=1 and the three steps are
	//   [low, mid, high] of the range.
	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext_benchmark(), crate::mock::Runtime);
}

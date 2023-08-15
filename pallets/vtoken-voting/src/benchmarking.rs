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

use crate::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use node_primitives::currency::VKSM;
use pallet_conviction_voting::{Conviction, Vote};

fn account_vote<T: Config>(b: BalanceOf<T>) -> AccountVote<BalanceOf<T>> {
	let v = Vote { aye: true, conviction: Conviction::Locked1x };

	AccountVote::Standard { vote: v, balance: b }
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn vote() -> Result<(), BenchmarkError> {
		let caller = whitelisted_caller();
		let origin = RawOrigin::Signed(caller);
		let vtoken = VKSM;
		let poll_index = 0u32;
		let account_vote = account_vote::<T>(100u32.into());

		#[extrinsic_call]
		vote(origin, vtoken, poll_index, account_vote);

		Ok(())
	}

	#[benchmark]
	pub fn unlock() -> Result<(), BenchmarkError> {
		let caller = whitelisted_caller();
		let origin = RawOrigin::Signed(caller);
		let vtoken = VKSM;
		let poll_index = 0u32;

		#[extrinsic_call]
		_(origin, vtoken, poll_index);

		Ok(())
	}

	#[benchmark]
	pub fn update_referendum_status() -> Result<(), BenchmarkError> {
		let caller = whitelisted_caller();
		let origin = RawOrigin::Signed(caller);
		let vtoken = VKSM;
		let poll_index = 0u32;

		#[extrinsic_call]
		_(origin, vtoken, poll_index);

		Ok(())
	}

	#[benchmark]
	pub fn unlock_delegator_token() -> Result<(), BenchmarkError> {
		let caller = whitelisted_caller();
		let origin = RawOrigin::Signed(caller);
		let vtoken = VKSM;
		let delegator: T::AccountId = whitelisted_caller();
		let delegator_lookup = T::Lookup::unlookup(delegator.clone());

		#[extrinsic_call]
		_(origin, vtoken, delegator_lookup);

		Ok(())
	}

	#[benchmark]
	pub fn kill_referendum() -> Result<(), BenchmarkError> {
		let origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let poll_index = 0u32;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, vtoken, poll_index);

		Ok(())
	}

	#[benchmark]
	pub fn set_delegator_role() -> Result<(), BenchmarkError> {
		let origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let derivative_index = 0;
		let vote_role = VoteRole::SplitAbstain;

		#[extrinsic_call]
		_(
			origin as <T as frame_system::Config>::RuntimeOrigin,
			vtoken,
			derivative_index,
			vote_role,
		);

		Ok(())
	}

	#[benchmark]
	pub fn set_referendum_status() -> Result<(), BenchmarkError> {
		let origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let poll_index = 0u32;
		let status = ReferendumStatus::Ongoing;

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, vtoken, poll_index, status);

		Ok(())
	}

	#[benchmark]
	pub fn set_vote_locking_period() -> Result<(), BenchmarkError> {
		let origin =
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let vtoken = VKSM;
		let vote_locking_period = BlockNumberFor::<T>::from(100u32);

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, vtoken, vote_locking_period);

		Ok(())
	}

	#[benchmark]
	pub fn update_referendum_status_notification_received() -> Result<(), BenchmarkError> {
		let origin =
			T::ResponseOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let query_id = 1u64;
		let response = Response::DispatchResult(MaybeErrorCode::Success);

		#[extrinsic_call]
		_(origin as <T as frame_system::Config>::RuntimeOrigin, query_id, response);

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
	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Runtime);
}

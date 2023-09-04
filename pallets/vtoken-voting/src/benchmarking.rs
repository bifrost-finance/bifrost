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

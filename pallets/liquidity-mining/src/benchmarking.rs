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

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::sp_runtime::traits::UniqueSaturatedFrom;
use frame_system::RawOrigin;

use super::*;
use super::mock::*;
#[allow(unused_imports)]
use crate::Pallet as LM;

fn create_pool<T: Config>() {
    assert_ok!(LM::create_pool(
			pallet_collective::RawOrigin::Member(TC_MEMBER_1).into(),
			(FARMING_DEPOSIT_1, FARMING_DEPOSIT_2),
			(REWARD_1, REWARD_AMOUNT),
			vec![(REWARD_2, REWARD_AMOUNT)],
			PoolType::Farming,
			DAYS,
			1_000 * UNIT,
			0
		));
}

benchmarks! {
	charge {
		let caller: T::AccountId = INVESTOR;
	}
}

impl_benchmark_test_suite!(LM, crate::mock::new_test_ext(), crate::mock::T);
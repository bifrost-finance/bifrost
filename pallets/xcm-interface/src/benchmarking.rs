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

#![cfg(feature = "runtime-benchmarks")]
use super::*;
use bifrost_primitives::{XcmOperationType, BNC};
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn update_xcm_dest_weight_and_fee() {
		let updates = vec![
			(BNC, XcmOperationType::Bond, Weight::zero(), 0u32.into()),
			(BNC, XcmOperationType::Bond, Weight::zero(), 0u32.into()),
			(BNC, XcmOperationType::Bond, Weight::zero(), 0u32.into()),
			(BNC, XcmOperationType::Bond, Weight::zero(), 0u32.into()),
			(BNC, XcmOperationType::Bond, Weight::zero(), 0u32.into()),
		];
		#[extrinsic_call]
		_(RawOrigin::Root, updates);
	}

	impl_benchmark_test_suite!(Pallet, mock::new_test_ext(), mock::Test);
}

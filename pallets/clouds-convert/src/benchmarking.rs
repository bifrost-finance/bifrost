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
#![cfg(feature = "runtime-benchmarks")]

use bifrost_primitives::{currency::CLOUD, VBNC};
use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use sp_runtime::traits::{UniqueSaturatedFrom, Zero};

use crate::{BalanceOf, Call, Config, Pallet as CloudsConvert, Pallet};
use orml_traits::MultiCurrency;

benchmarks! {
clouds_to_vebnc {
	let test_account: T::AccountId = account("seed",1,1);

	T::MultiCurrency::deposit(CLOUD, &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
	T::MultiCurrency::deposit(VBNC, &CloudsConvert::<T>::clouds_pool_account(), BalanceOf::<T>::unique_saturated_from(100_000_000_000_000_000_000u128))?;

}: _(RawOrigin::Signed(test_account), BalanceOf::<T>::unique_saturated_from(10_000_000_000_000u128), Zero::zero())

charge_vbnc {
	let test_account: T::AccountId = account("seed",1,1);

	T::MultiCurrency::deposit(VBNC, &test_account, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;

}: _(RawOrigin::Signed(test_account),BalanceOf::<T>::unique_saturated_from(50_000_000_000u128))

	impl_benchmark_test_suite!(CloudsConvert,crate::mock::ExtBuilder::default().build(),crate::mock::Runtime);
}

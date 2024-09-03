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

use crate::{BalanceOf, Call, Config, Pallet, Pallet as BuyBack, *};
use bifrost_primitives::VDOT;
use frame_benchmarking::v1::{account, benchmarks, BenchmarkError};
use frame_support::{
	assert_ok,
	traits::{EnsureOrigin, Hooks},
};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use sp_runtime::traits::UniqueSaturatedFrom;

benchmarks! {
	set_vtoken {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
	}: _<T::RuntimeOrigin>(origin,VDOT,1_000_000u32.into(),Permill::from_percent(2),1000u32.into(),1000u32.into(),true,Some(Permill::from_percent(2)),Permill::from_percent(2))

	charge {
		let test_account: T::AccountId = account("seed",1,1);

		T::MultiCurrency::deposit(VDOT, &test_account, BalanceOf::<T>::unique_saturated_from(1_000_000_000_000_000u128))?;
	}: _(RawOrigin::Signed(test_account),VDOT,BalanceOf::<T>::unique_saturated_from(9_000_000_000_000u128))

	remove_vtoken {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		assert_ok!(BuyBack::<T>::set_vtoken(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			VDOT,
			1_000_000u32.into(),
			Permill::from_percent(2),
			1000u32.into(),
			1000u32.into(),
			true,
			Some(Permill::from_percent(2)),
			Permill::from_percent(2)
		));
	}: _<T::RuntimeOrigin>(origin,VDOT)


	on_initialize {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		assert_ok!(BuyBack::<T>::set_vtoken(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			VDOT,
			1_000_000u32.into(),
			Permill::from_percent(2),
			1000u32.into(),
			1000u32.into(),
			true,
			Some(Permill::from_percent(2)),
			Permill::from_percent(2)
		));
	}: {
		BuyBack::<T>::on_initialize(BlockNumberFor::<T>::from(0u32));
	}

	impl_benchmark_test_suite!(BuyBack,crate::mock::ExtBuilder::default().build(),crate::mock::Runtime);
}

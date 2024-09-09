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
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_runtime::traits::{UniqueSaturatedFrom, Zero};

#[benchmarks(where T: Config)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn convert_to_vbnc_p() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let currency = VBNC;
		let value = BalanceOf::<T>::unique_saturated_from(100_000_000_000_000_000_000u128);
		let vbnc_pool_account = Pallet::<T>::vbnc_p_pool_account();

		// Ensure the pool has enough balance
		T::MultiCurrency::deposit(VBNC_P, &vbnc_pool_account, value)?;

		// Make sure the user has enough balance in the provided currency
		T::MultiCurrency::deposit(currency, &caller, value)?;

		#[extrinsic_call]
		Pallet::<T>::convert_to_vbnc_p(RawOrigin::Signed(caller.clone()), currency, value);

		assert!(T::MultiCurrency::free_balance(VBNC_P, &caller) > Zero::zero());

		Ok(())
	}

	#[benchmark]
	fn charge_vbnc_p() -> Result<(), BenchmarkError> {
		let caller: T::AccountId = whitelisted_caller();
		let amount = BalanceOf::<T>::unique_saturated_from(100_000_000_000_000_000_000u128);
		let vbnc_pool_account = Pallet::<T>::vbnc_p_pool_account();

		// Ensure the caller has enough vBNC-P balance
		T::MultiCurrency::deposit(VBNC_P, &caller, amount)?;

		#[extrinsic_call]
		Pallet::<T>::charge_vbnc_p(RawOrigin::Signed(caller.clone()), amount);

		assert_eq!(T::MultiCurrency::free_balance(VBNC_P, &caller), Zero::zero());
		assert_eq!(T::MultiCurrency::free_balance(VBNC_P, &vbnc_pool_account), amount);

		Ok(())
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext_benchmark(), crate::mock::Runtime);
}

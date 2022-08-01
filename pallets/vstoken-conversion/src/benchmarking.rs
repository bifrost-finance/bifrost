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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;

#[allow(unused_imports)]
pub use crate::{Pallet as VstokenConversion, *};

benchmarks! {
	set_exchange_fee {
		let caller: T::AccountId = whitelisted_caller();
		let exchange_fee = VstokenConversion::<T>::exchange_fee();
	}: set_exchange_fee(RawOrigin::Signed(caller.clone()), exchange_fee.clone())
	verify {
		assert_eq!(VstokenConversion::<T>::exchange_fee(),exchange_fee.clone());
	}

	set_exchange_rate {
		let caller: T::AccountId = whitelisted_caller();
		let exchange_rate_percent: Percent = Percent::from_percent(5);
		let exchange_rate: VstokenConversionExchangeRate = VstokenConversionExchangeRate {
			vsbond_convert_to_vsdot: exchange_rate_percent,
			vsbond_convert_to_vsksm: exchange_rate_percent,
			vsksm_convert_to_vsbond: exchange_rate_percent,
			vsdot_convert_to_vsbond: exchange_rate_percent,
		};
	}: set_exchange_rate(RawOrigin::Signed(caller.clone()),0, exchange_rate.clone())
	verify {
		assert_eq!(VstokenConversion::<T>::exchange_rate(0),exchange_rate.clone());
	}

	set_kusama_lease {
		let caller: T::AccountId = whitelisted_caller();
	}: set_kusama_lease(RawOrigin::Signed(caller.clone()),100)
	verify {
		assert_eq!(VstokenConversion::<T>::kusama_lease(),100);
	}

	set_polkadot_lease {
		let caller: T::AccountId = whitelisted_caller();
	}: set_polkadot_lease(RawOrigin::Signed(caller.clone()),100)
	verify {
		assert_eq!(VstokenConversion::<T>::polkadot_lease(),100);
	}



}

impl_benchmark_test_suite!(
	VstokenConversion,
	crate::mock::ExtBuilder::default().build(),
	crate::mock::Runtime
);

//Todo:
//vsbond_convert_to_vsksm
//vsksm_convert_to_vsbond
//vsbond_convert_to_vsdot
//vsdot_convert_to_vsbond

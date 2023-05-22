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

use frame_benchmarking::v1::{account, benchmarks, BenchmarkError};
use frame_support::{
	assert_ok,
	traits::{EnsureOrigin, Get},
};
use frame_system::RawOrigin;
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
use sp_runtime::traits::{AccountIdConversion, UniqueSaturatedFrom};

use crate::{
	BalanceOf, Call, Config, Pallet as VstokenConversion, Pallet, Percent,
	VstokenConversionExchangeFee, VstokenConversionExchangeRate,
};
pub const VS_BOND: CurrencyId = CurrencyId::VSBond(TokenSymbol::BNC, 2001, 0, 8);
pub const VS_KSM: CurrencyId = CurrencyId::VSToken(TokenSymbol::KSM);

benchmarks! {
	set_exchange_fee {
		let fee: VstokenConversionExchangeFee<BalanceOf<T>> =
			VstokenConversionExchangeFee {
				vstoken_exchange_fee: 10u32.into(),
				vsbond_exchange_fee_of_vstoken: 10u32.into(),
			};
	}: _(RawOrigin::Root,fee)

	set_exchange_rate {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let fee: VstokenConversionExchangeFee<BalanceOf<T>> =
			VstokenConversionExchangeFee {
				vstoken_exchange_fee: 10u32.into(),
				vsbond_exchange_fee_of_vstoken: 10u32.into(),
			};
		let rate: VstokenConversionExchangeRate = VstokenConversionExchangeRate {
			vsbond_convert_to_vstoken: Percent::from_percent(5),
			vstoken_convert_to_vsbond: Percent::from_percent(5),
		};
	}: _<T::RuntimeOrigin>(origin,1,rate)

	set_relaychain_lease {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
	}: _<T::RuntimeOrigin>(origin,1)

	vsbond_convert_to_vstoken {
		let test_account: T::AccountId = account("seed",1,1);
		let fee: VstokenConversionExchangeFee<BalanceOf<T>> =
			VstokenConversionExchangeFee {
				vstoken_exchange_fee: 10u32.into(),
				vsbond_exchange_fee_of_vstoken: 10u32.into(),
			};
		let rate: VstokenConversionExchangeRate = VstokenConversionExchangeRate {
			vsbond_convert_to_vstoken: Percent::from_percent(95),
			vstoken_convert_to_vsbond: Percent::from_percent(95),
		};
		assert_ok!(
			VstokenConversion::<T>::set_exchange_fee(
				RawOrigin::Root.into(),
				fee
			));

		assert_ok!(
			VstokenConversion::<T>::set_exchange_rate(
				T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
				8,
				rate
			));

		assert_ok!(
			VstokenConversion::<T>::set_relaychain_lease(
				T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
				1
			));

		let vsbond_account: T::AccountId =
			<T as Config>::VsbondAccount::get().into_account_truncating();
		T::MultiCurrency::deposit(VS_KSM, &vsbond_account, BalanceOf::<T>::unique_saturated_from(1000000000000u128))?;
		T::MultiCurrency::deposit(VS_BOND, &test_account, BalanceOf::<T>::unique_saturated_from(1000000000000u128))?;
	}: _(RawOrigin::Signed(test_account),VS_BOND,BalanceOf::<T>::unique_saturated_from(100000000000u128),BalanceOf::<T>::unique_saturated_from(10000000000u128))

	vstoken_convert_to_vsbond {
		let test_account: T::AccountId = account("seed",1,1);
		let fee: VstokenConversionExchangeFee<BalanceOf<T>> =
			VstokenConversionExchangeFee {
				vstoken_exchange_fee: 10u32.into(),
				vsbond_exchange_fee_of_vstoken: 10u32.into(),
			};
		let rate: VstokenConversionExchangeRate = VstokenConversionExchangeRate {
			vsbond_convert_to_vstoken: Percent::from_percent(5),
			vstoken_convert_to_vsbond: Percent::from_percent(5),
		};
		assert_ok!(
			VstokenConversion::<T>::set_exchange_fee(
				RawOrigin::Root.into(),
				fee
			));

		assert_ok!(
			VstokenConversion::<T>::set_exchange_rate(
				T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
				8,
				rate
			));

		assert_ok!(
			VstokenConversion::<T>::set_relaychain_lease(
				T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
				1
			));

		let vsbond_account: T::AccountId =
			<T as Config>::VsbondAccount::get().into_account_truncating();
		T::MultiCurrency::deposit(VS_BOND, &vsbond_account, BalanceOf::<T>::unique_saturated_from(100000000000000u128))?;
		T::MultiCurrency::deposit(VS_KSM, &test_account, BalanceOf::<T>::unique_saturated_from(100000000000000u128))?;

	}: _(RawOrigin::Signed(test_account),VS_BOND,BalanceOf::<T>::unique_saturated_from(1000000000000u128),BalanceOf::<T>::unique_saturated_from(100000000000u128))


	impl_benchmark_test_suite!(VstokenConversion,crate::mock::ExtBuilder::default().build(),crate::mock::Runtime);
}

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
#![allow(deprecated)]

use bifrost_primitives::{CurrencyId, TokenSymbol};
use frame_benchmarking::v1::{account, benchmarks, whitelisted_caller, BenchmarkError};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::traits::{AccountIdConversion, UniqueSaturatedFrom};
use xcm::v2::prelude::*;

use super::*;
use crate::Pallet as CrossInOut;

benchmarks! {
	deregister_currency_for_cross_in_out {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		assert_ok!(CrossInOut::<T>::register_currency_for_cross_in_out(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::DOT)
		));
	}: _<T::RuntimeOrigin>(origin,CurrencyId::Token(TokenSymbol::DOT))

	set_crossing_minimum_amount {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
	}: _<T::RuntimeOrigin>(origin,CurrencyId::Token(TokenSymbol::DOT),100u32.into(),100u32.into())

	add_to_register_whitelist {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let caller = whitelisted_caller();
	}: _<T::RuntimeOrigin>(origin,CurrencyId::Token(TokenSymbol::DOT),caller)

	remove_from_register_whitelist {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(CrossInOut::<T>::add_to_register_whitelist(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::DOT),
			test_account.clone()
		));
	}: _<T::RuntimeOrigin>(origin,CurrencyId::Token(TokenSymbol::DOT),test_account)

	register_linked_account {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(CrossInOut::<T>::add_to_register_whitelist(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::DOT),
			test_account.clone()
		));

		assert_ok!(CrossInOut::<T>::register_currency_for_cross_in_out(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::DOT)
		));

		let location = Box::new(MultiLocation {
				parents: 0,
				interior: X1(AccountId32 {
					network: Any,
					id: T::EntrancePalletId::get().into_account_truncating(),
				}),
			});
	}: _(RawOrigin::Signed(test_account.clone()),CurrencyId::Token(TokenSymbol::DOT),account("seed",1,1),location)

	cross_out {
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(CrossInOut::<T>::register_currency_for_cross_in_out(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::DOT)
		));

		assert_ok!(CrossInOut::<T>::set_crossing_minimum_amount(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::DOT),100u32.into(),100u32.into()
		));

		assert_ok!(CrossInOut::<T>::add_to_register_whitelist(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::DOT),
			test_account.clone()
		));

		let location = Box::new(MultiLocation {
				parents: 0,
				interior: X1(AccountId32 {
					network: Any,
					id: T::EntrancePalletId::get().into_account_truncating(),
				}),
			});

		assert_ok!(CrossInOut::<T>::register_linked_account(
			RawOrigin::Signed(test_account.clone()).into(),
			CurrencyId::Token(TokenSymbol::DOT),
			test_account.clone(),
			location
		));

		T::MultiCurrency::deposit(CurrencyId::Token(TokenSymbol::DOT), &test_account, BalanceOf::<T>::unique_saturated_from(1000000000000u128))?;

	}: _(RawOrigin::Signed(test_account),CurrencyId::Token(TokenSymbol::DOT),BalanceOf::<T>::unique_saturated_from(100000000000u128))

	change_outer_linked_account {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let test_account: T::AccountId = account("seed",1,1);
		assert_ok!(CrossInOut::<T>::register_currency_for_cross_in_out(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::DOT)
		));

		assert_ok!(CrossInOut::<T>::add_to_register_whitelist(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::DOT),
			test_account.clone()
		));

		let location1 = Box::new(MultiLocation {
				parents: 0,
				interior: X1(AccountId32 {
					network: Any,
					id: T::EntrancePalletId::get().into_account_truncating(),
				}),
			});

		assert_ok!(CrossInOut::<T>::register_linked_account(
			RawOrigin::Signed(test_account.clone()).into(),
			CurrencyId::Token(TokenSymbol::DOT),
			test_account.clone(),
			location1
		));

		let location2 = Box::new(MultiLocation {
				parents: 1,
				interior: X1(AccountId32 {
					network: Any,
					id: T::EntrancePalletId::get().into_account_truncating(),
				}),
			});

	}: _<T::RuntimeOrigin>(origin,CurrencyId::Token(TokenSymbol::DOT),location2,test_account)

	impl_benchmark_test_suite!(CrossInOut,crate::mock::ExtBuilder::default().build(),crate::mock::Runtime);
}

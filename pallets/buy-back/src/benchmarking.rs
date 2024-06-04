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

use crate::{BalanceOf, Call, Config, Info, Pallet as SystemMaker, Pallet, *};
use bifrost_primitives::{CurrencyId, TokenSymbol};
use frame_benchmarking::v1::{account, benchmarks, BenchmarkError};
use frame_support::{
	assert_ok,
	traits::{EnsureOrigin, Hooks},
};
use frame_system::RawOrigin;
use orml_traits::MultiCurrency;
use sp_core::Get;
use sp_runtime::traits::{AccountIdConversion, UniqueSaturatedFrom};
// use crate::{Pallet as SystemMaker, *};

benchmarks! {
	set_config {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let info = Info {
				vcurrency_id: CurrencyId::VToken(TokenSymbol::KSM),
				annualization: 600_000u32,
				granularity: 1000u32.into(),
				minimum_redeem: 20000u32.into()
			};
	}: _<T::RuntimeOrigin>(origin,CurrencyId::Token(TokenSymbol::KSM),info)

	charge {
		let test_account: T::AccountId = account("seed",1,1);

		T::MultiCurrency::deposit(CurrencyId::Token(TokenSymbol::DOT), &test_account, BalanceOf::<T>::unique_saturated_from(100000000000u128))?;
	}: _(RawOrigin::Signed(test_account),CurrencyId::Token(TokenSymbol::DOT),BalanceOf::<T>::unique_saturated_from(10000000000u128))

	close {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		assert_ok!(SystemMaker::<T>::set_config(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::KSM),
			Info {
				vcurrency_id: CurrencyId::VToken(TokenSymbol::KSM),
				annualization: 600_000u32,
				granularity: 1000u32.into(),
				minimum_redeem: 20000u32.into()
			},
		));
	}: _<T::RuntimeOrigin>(origin,CurrencyId::Token(TokenSymbol::KSM))

	payout {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let pallet_account: T::AccountId = T::SystemMakerPalletId::get().into_account_truncating();
		T::MultiCurrency::deposit(CurrencyId::Token(TokenSymbol::DOT), &pallet_account, BalanceOf::<T>::unique_saturated_from(100000000000u128))?;
	}: _<T::RuntimeOrigin>(origin,CurrencyId::Token(TokenSymbol::DOT),BalanceOf::<T>::unique_saturated_from(10000000000u128))

	on_idle {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		assert_ok!(SystemMaker::<T>::set_config(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			CurrencyId::Token(TokenSymbol::KSM),
			Info {
				vcurrency_id: CurrencyId::VToken(TokenSymbol::KSM),
				annualization: 600_000u32,
				granularity: 1000u32.into(),
				minimum_redeem: 20000u32.into()
			},
		));
	}: {
		SystemMaker::<T>::on_idle(BlockNumberFor::<T>::from(0u32),Weight::from_parts(0, u64::MAX));
	}

	impl_benchmark_test_suite!(SystemMaker,crate::mock::ExtBuilder::default().build(),crate::mock::Runtime);
}

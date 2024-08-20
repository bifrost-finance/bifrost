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

use bifrost_primitives::{CurrencyId, TokenSymbol};
use frame_benchmarking::{benchmarks, v1::BenchmarkError};
use frame_support::traits::UnfilteredDispatchable;

use super::*;
#[allow(unused_imports)]
use crate::Pallet as CallSwitchgear;

benchmarks! {
	switchoff_transaction {
		let origin = T::UpdateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let switchoff_call = Call::<T>::switchoff_transaction{pallet_name: b"Balances".to_vec(), function_name: b"transfer".to_vec()};
	}: {switchoff_call.dispatch_bypass_filter(origin)?}

	switchon_transaction {
		let origin = T::UpdateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let switchoff_call = Call::<T>::switchoff_transaction{pallet_name: b"Balances".to_vec(), function_name: b"transfer".to_vec()};
		switchoff_call.dispatch_bypass_filter(origin.clone())?;
		let switchon_call = Call::<T>::switchon_transaction{pallet_name: b"Balances".to_vec(), function_name: b"transfer".to_vec()};
	}: {switchon_call.dispatch_bypass_filter(origin)?}

	disable_transfers {
		let origin = T::UpdateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let disable_call = Call::<T>::disable_transfers{currency_id: CurrencyId::Token(TokenSymbol::KSM)};
	}: {disable_call.dispatch_bypass_filter(origin)?}

	enable_transfers {
		let origin = T::UpdateOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let disable_call = Call::<T>::disable_transfers{currency_id: CurrencyId::Token(TokenSymbol::KSM)};
		disable_call.dispatch_bypass_filter(origin.clone())?;
		let enable_call = Call::<T>::enable_transfers{currency_id: CurrencyId::Token(TokenSymbol::KSM)};
	}: {enable_call.dispatch_bypass_filter(origin)?}

	impl_benchmark_test_suite!(
	CallSwitchgear,
	crate::mock::ExtBuilder::default().build(),
	crate::mock::Runtime
);
}

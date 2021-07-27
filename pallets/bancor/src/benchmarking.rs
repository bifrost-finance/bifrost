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

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use node_primitives::{CurrencyId, TokenSymbol};
use sp_runtime::traits::UniqueSaturatedFrom;

use super::*;
#[allow(unused_imports)]
use crate::Pallet as Bancor;

benchmarks! {
	add_token_to_pool {
		let currency_id = CurrencyId::Token(TokenSymbol::DOT);

		let caller: T::AccountId = whitelisted_caller();

		let token_amount = BalanceOf::<T>::unique_saturated_from(1_000_000_000_000 as u128);
	}: _(RawOrigin::Signed(caller), currency_id, token_amount)

}

impl_benchmark_test_suite!(
	Bancor,
	crate::mock::ExtBuilder::default()
		.one_precision_for_each_currency_type_for_whitelist_account()
		.build(),
	crate::mock::Test
);

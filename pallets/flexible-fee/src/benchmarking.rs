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

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;
use node_primitives::{CurrencyId, TokenSymbol};

use super::*;
#[allow(unused_imports)]
use crate::Pallet as FlexibleFee;

benchmarks! {
	set_user_fee_charge_order {
		let order_vec = vec![CurrencyId::Token(TokenSymbol::try_from(0u8).unwrap_or_default())];
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller), Some(order_vec))
}

impl_benchmark_test_suite!(
	FlexibleFee,
	crate::mock::ExtBuilder::default()
		.one_hundred_precision_for_each_currency_type_for_whitelist_account()
		.build(),
	crate::mock::Test
);

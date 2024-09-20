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

use frame_benchmarking::v1::{benchmarks, whitelisted_caller};
use frame_support::BoundedVec;

use bifrost_primitives::{CurrencyId, TokenSymbol};
use frame_system::RawOrigin;
use sp_std::vec;

use crate::{Call, Config, Pallet};

benchmarks! {
	set_user_default_fee_currency {
		let caller = whitelisted_caller();
	}: _(RawOrigin::Signed(caller),Some(CurrencyId::Token(TokenSymbol::DOT)))

	set_default_fee_currency_list {
		let default_list = BoundedVec::try_from(vec![CurrencyId::Token(TokenSymbol::DOT)]).unwrap();
	}: _(RawOrigin::Root,default_list)

	impl_benchmark_test_suite!(
	Pallet,
	crate::mock::new_test_ext(),
	crate::mock::Test)
}

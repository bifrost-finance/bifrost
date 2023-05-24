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

use frame_benchmarking::v1::{benchmarks, whitelisted_caller, BenchmarkError};
use frame_support::{assert_ok, traits::EnsureOrigin, BoundedVec};

use frame_system::RawOrigin;
use node_primitives::{CurrencyId, TokenSymbol};
use sp_std::vec;

use crate::{Call, Config, Pallet as FlexibleFee, Pallet};

benchmarks! {
	set_user_default_fee_currency {
		let caller = whitelisted_caller();
	}: _(RawOrigin::Signed(caller),Some(CurrencyId::Token(TokenSymbol::DOT)))

	set_universal_fee_currency_order_list {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let default_list = BoundedVec::try_from(vec![CurrencyId::Token(TokenSymbol::DOT)]).unwrap();
	}: _<T::RuntimeOrigin>(origin,default_list)

	remove_from_user_fee_charge_order_list {
		let caller = whitelisted_caller();
		let default_list = BoundedVec::try_from(vec![CurrencyId::Token(TokenSymbol::DOT)]).unwrap();
		assert_ok!(FlexibleFee::<T>::set_universal_fee_currency_order_list(
			T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?,
			default_list,
		));
	}: _(RawOrigin::Signed(caller))
}

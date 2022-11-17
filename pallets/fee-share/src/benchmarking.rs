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

use frame_benchmarking::{benchmarks, vec, whitelisted_caller};
use frame_support::{assert_ok, sp_runtime::traits::UniqueSaturatedFrom};
use frame_system::{Pallet as System, RawOrigin};
use node_primitives::{CurrencyId, TokenSymbol};

use crate::{Pallet as FeeShare, *};

benchmarks! {
	on_initialize {}:{FeeShare::<T>::on_idle(T::BlockNumber::from(10u32),0);}

	create_distribution {
		let caller: T::AccountId = whitelisted_caller();
		let tokens_proportion = vec![(caller.clone(), Perbill::from_percent(100))];
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_type = vec![KSM];
	}: _(RawOrigin::Root,
	token_type.clone(),
	tokens_proportion.clone(),
	true)

	edit_distribution {
		let caller: T::AccountId = whitelisted_caller();
		let tokens_proportion = vec![(caller.clone(), Perbill::from_percent(100))];
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_type = vec![KSM];
		assert_ok!(FeeShare::<T>::create_distribution(
			RawOrigin::Root.into(),
			vec![KSM],
			tokens_proportion.clone(),
			true,
		));
	}: _(RawOrigin::Root,
		0,
		None,
		Some(tokens_proportion.clone()),
		Some(true))
	set_era_length {}: _(RawOrigin::Root,T::BlockNumber::from(10u32))
	execute_distribute {
		let caller: T::AccountId = whitelisted_caller();
		let tokens_proportion = vec![(caller.clone(), Perbill::from_percent(100))];
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_type = vec![KSM];
		assert_ok!(FeeShare::<T>::create_distribution(
			RawOrigin::Root.into(),
			vec![KSM],
			tokens_proportion.clone(),
			true,
		));
	}: _(RawOrigin::Root,0)
	delete_distribution {
		let caller: T::AccountId = whitelisted_caller();
		let tokens_proportion = vec![(caller.clone(), Perbill::from_percent(100))];
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_type = vec![KSM];
		assert_ok!(FeeShare::<T>::create_distribution(
			RawOrigin::Root.into(),
			vec![KSM],
			tokens_proportion.clone(),
			true,
		));
	}: _(RawOrigin::Root,0)
}

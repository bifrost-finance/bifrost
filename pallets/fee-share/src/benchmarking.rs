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

use bifrost_primitives::{CurrencyId, TokenSymbol};
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_std::vec;

use crate::{Pallet as FeeShare, *};

benchmarks! {
	on_initialize {}:{FeeShare::<T>::on_idle(BlockNumberFor::<T>::from(10u32),Weight::from_parts(0, 0));}

	create_distribution {
		let caller: T::AccountId = whitelisted_caller();
		let tokens_proportion = vec![(caller.clone(), Perbill::from_percent(100))];
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_type = vec![KSM];
	}: _(RawOrigin::Root,
	BoundedVec::try_from(token_type.clone()).unwrap(),
	BoundedVec::try_from(tokens_proportion.clone()).unwrap(),
	true)

	edit_distribution {
		let caller: T::AccountId = whitelisted_caller();
		let tokens_proportion = vec![(caller.clone(), Perbill::from_percent(100))];
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_type = vec![KSM];
		assert_ok!(FeeShare::<T>::create_distribution(
			RawOrigin::Root.into(),
			BoundedVec::try_from(vec![KSM]).unwrap(),
			BoundedVec::try_from(tokens_proportion.clone()).unwrap(),
			true,
		));
	}: _(RawOrigin::Root,
		0,
		None,
		Some(BoundedVec::try_from(tokens_proportion.clone()).unwrap()),
		Some(true))
	set_era_length {}: _(RawOrigin::Root,BlockNumberFor::<T>::from(10u32))
	execute_distribute {
		let caller: T::AccountId = whitelisted_caller();
		let tokens_proportion = vec![(caller.clone(), Perbill::from_percent(100))];
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_type = vec![KSM];
		assert_ok!(FeeShare::<T>::create_distribution(
			RawOrigin::Root.into(),
			BoundedVec::try_from(vec![KSM]).unwrap(),
			BoundedVec::try_from(tokens_proportion.clone()).unwrap(),
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
			BoundedVec::try_from(vec![KSM]).unwrap(),
			BoundedVec::try_from(tokens_proportion.clone()).unwrap(),
			true,
		));
	}: _(RawOrigin::Root,0)
	set_usd_config {
		let caller: T::AccountId = whitelisted_caller();
		let tokens_proportion = vec![(caller.clone(), Perbill::from_percent(100))];
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_type = vec![KSM];
		assert_ok!(FeeShare::<T>::create_distribution(
			RawOrigin::Root.into(),
			BoundedVec::try_from(vec![KSM]).unwrap(),
			BoundedVec::try_from(tokens_proportion.clone()).unwrap(),
			true,
		));
	}: _(RawOrigin::Root,
		0,
		100u128,
		10u32.into(),
		caller)
}

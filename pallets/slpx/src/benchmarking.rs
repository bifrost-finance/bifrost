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

use crate::{Pallet as Slpx, *};
use frame_benchmarking::v1::{benchmarks, whitelisted_caller, BenchmarkError};
use frame_support::{assert_ok, sp_runtime::traits::UniqueSaturatedFrom, traits::EnsureOrigin};
use frame_system::RawOrigin;
use node_primitives::{CurrencyId, TokenSymbol};

benchmarks! {
	add_whitelist {
		let origin = <T as pallet::Config>::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
	let contract: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root,SupportChain::Astar, contract)

	remove_whitelist {
		let origin = <T as pallet::Config>::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let contract: T::AccountId = whitelisted_caller();
		assert_ok!(Slpx::<T>::add_whitelist(
			origin,
			SupportChain::Astar,
			contract.clone()
		));
	}: _(RawOrigin::Root,SupportChain::Astar, contract)

	set_execution_fee {
		let origin = <T as pallet::Config>::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let contract: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root,CurrencyId::Token2(0), 10u32.into())

	set_transfer_to_fee {
		let origin = <T as pallet::Config>::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let contract: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Root,SupportChain::Astar, 10u32.into())


	mint {
		let contract: T::AccountId = whitelisted_caller();
		assert_ok!(Slpx::<T>::add_whitelist(
				RawOrigin::Root.into(),
				SupportChain::Astar,
				contract.clone()
		));
		assert_ok!(Slpx::<T>::set_execution_fee(
				RawOrigin::Root.into(),
				CurrencyId::Native(TokenSymbol::BNC),
				0u32.into()
		));
		let addr: [u8; 20] = hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
		let receiver = H160::from(addr);
		let evm_caller_account_id = Slpx::<T>::h160_to_account_id(receiver);
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &evm_caller_account_id, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
	}: _(RawOrigin::Signed(contract), receiver, CurrencyId::Native(TokenSymbol::BNC), SupportChain::Astar)

	redeem {
		let contract: T::AccountId = whitelisted_caller();
		assert_ok!(Slpx::<T>::add_whitelist(
				RawOrigin::Root.into(),
				SupportChain::Astar,
				contract.clone()
		));
		assert_ok!(Slpx::<T>::set_execution_fee(
				RawOrigin::Root.into(),
				CurrencyId::VToken(TokenSymbol::BNC),
				0u32.into()
		));
		let addr: [u8; 20] = hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
		let receiver = H160::from(addr);
		let evm_caller_account_id = Slpx::<T>::h160_to_account_id(receiver);
		T::MultiCurrency::deposit(CurrencyId::VToken(TokenSymbol::BNC), &evm_caller_account_id, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
	}: _(RawOrigin::Signed(contract), receiver, CurrencyId::VToken(TokenSymbol::BNC), SupportChain::Astar)


	swap {
		let contract: T::AccountId = whitelisted_caller();
		assert_ok!(Slpx::<T>::add_whitelist(
				RawOrigin::Root.into(),
				SupportChain::Astar,
				contract.clone()
		));
		assert_ok!(Slpx::<T>::set_execution_fee(
				RawOrigin::Root.into(),
				CurrencyId::Native(TokenSymbol::BNC),
				0u32.into()
		));
		let addr: [u8; 20] = hex_literal::hex!["3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"].into();
		let receiver = H160::from(addr);
		let evm_caller_account_id = Slpx::<T>::h160_to_account_id(receiver);
		T::MultiCurrency::deposit(CurrencyId::Native(TokenSymbol::BNC), &evm_caller_account_id, BalanceOf::<T>::unique_saturated_from(100_000_000_000_000u128))?;
	}: _(RawOrigin::Signed(contract), receiver, CurrencyId::Native(TokenSymbol::BNC),CurrencyId::VToken(TokenSymbol::BNC), 0u32.into(),SupportChain::Astar)
}

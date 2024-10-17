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

use crate::{Pallet as VtokenMinting, *};
use bifrost_primitives::{CurrencyId, TokenSymbol, VtokenMintingOperator, VKSM};
use frame_benchmarking::v1::{benchmarks, whitelisted_caller, BenchmarkError};
use frame_support::{assert_ok, sp_runtime::traits::UniqueSaturatedFrom};
use frame_system::RawOrigin;

benchmarks! {
	set_minimum_mint {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let token = CurrencyId::Token(TokenSymbol::KSM);
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u32 as u128);
	}: _<T::RuntimeOrigin>(origin, token, token_amount)

	set_minimum_redeem {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let token = CurrencyId::Token(TokenSymbol::KSM);
		let token_amount = BalanceOf::<T>::unique_saturated_from(0u32 as u128);
	}: _<T::RuntimeOrigin>(origin, token, token_amount)

	set_unlock_duration {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let token = CurrencyId::Token(TokenSymbol::KSM);
	}: _<T::RuntimeOrigin>(origin,  token, TimeUnit::Era(1))

	add_support_rebond_token {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let token = CurrencyId::Token(TokenSymbol::KSM);
	}: _<T::RuntimeOrigin>(origin, token)

	remove_support_rebond_token {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let token = CurrencyId::Token(TokenSymbol::KSM);
		assert_ok!(VtokenMinting::<T>::add_support_rebond_token(T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, token));
	}: _<T::RuntimeOrigin>(origin, token)

	set_fees {
		const FEE: Permill = Permill::from_percent(5);
	}: _(RawOrigin::Root, FEE, FEE)

	set_hook_iteration_limit {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
	}: _<T::RuntimeOrigin>(origin, 10u32)

	set_unlocking_total {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let token = CurrencyId::Token(TokenSymbol::KSM);
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u32 as u128);
	}: _<T::RuntimeOrigin>(origin, token, token_amount)

	set_min_time_unit {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let token = CurrencyId::Token(TokenSymbol::KSM);
	}: _<T::RuntimeOrigin>(origin, token, TimeUnit::Era(1))

	set_ongoing_time_unit {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let token = CurrencyId::Token(TokenSymbol::KSM);
	}: _<T::RuntimeOrigin>(origin, token, TimeUnit::Era(1))

	mint {
		let caller: T::AccountId = whitelisted_caller();
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_amount = BalanceOf::<T>::unique_saturated_from(10000000000u128);
		T::MultiCurrency::deposit(KSM, &caller, token_amount)?;
	}: _(RawOrigin::Signed(caller), KSM, token_amount,BoundedVec::default(), None)

	redeem {
		let caller: T::AccountId = whitelisted_caller();
		const VKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let vtoken_amount = BalanceOf::<T>::unique_saturated_from(90u128);
		let redeem_amount = BalanceOf::<T>::unique_saturated_from(1000000000u128);
		let token_amount = BalanceOf::<T>::unique_saturated_from(10000000000u128);
		const FEE: Permill = Permill::from_percent(50);
		assert_ok!(VtokenMinting::<T>::set_fees(RawOrigin::Root.into(), FEE, FEE));
		assert_ok!(VtokenMinting::<T>::set_unlock_duration(T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, KSM, TimeUnit::Era(1)));
		// assert_ok!(VtokenMinting::<T>::increase_token_pool(KSM, token_amount));
		assert_ok!(VtokenMinting::<T>::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		// assert_ok!(VtokenMinting::<T>::set_minimum_redeem(RawOrigin::Root.into(), VKSM, vtoken_amount));
		T::MultiCurrency::deposit(KSM, &caller, token_amount)?;
		assert_ok!(VtokenMinting::<T>::mint(RawOrigin::Signed(caller.clone()).into(), KSM, token_amount,BoundedVec::default(), None));
	}: _(RawOrigin::Signed(caller.clone()), VKSM, redeem_amount)

	rebond {
		let caller: T::AccountId = whitelisted_caller();
		const VKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let rebond_amount = BalanceOf::<T>::unique_saturated_from(100000000000u128);
		let redeem_amount = BalanceOf::<T>::unique_saturated_from(1000000000000u128);
		let mint_amount = BalanceOf::<T>::unique_saturated_from(2000000000000u128);
		let token_amount = BalanceOf::<T>::unique_saturated_from(5000000000000u128);
		const FEE: Permill = Permill::from_percent(50);
		assert_ok!(VtokenMinting::<T>::set_fees(RawOrigin::Root.into(), FEE, FEE));
		assert_ok!(VtokenMinting::<T>::set_unlock_duration(T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::<T>::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		T::MultiCurrency::deposit(KSM, &caller, token_amount)?;
		T::MultiCurrency::deposit(VKSM, &caller, redeem_amount)?;
		assert_ok!(VtokenMinting::<T>::mint(RawOrigin::Signed(caller.clone()).into(), KSM, mint_amount,BoundedVec::default(), None));
		assert_ok!(VtokenMinting::<T>::redeem(RawOrigin::Signed(caller.clone()).into(), VKSM, redeem_amount));
		assert_ok!(VtokenMinting::<T>::add_support_rebond_token(T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, KSM));
	}: _(RawOrigin::Signed(caller), KSM, rebond_amount)

	rebond_by_unlock_id {
		let caller: T::AccountId = whitelisted_caller();
		const VKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let rebond_amount = BalanceOf::<T>::unique_saturated_from(100000000000u128);
		let redeem_amount = BalanceOf::<T>::unique_saturated_from(1000000000000u128);
		let mint_amount = BalanceOf::<T>::unique_saturated_from(2000000000000u128);
		let token_amount = BalanceOf::<T>::unique_saturated_from(5000000000000u128);
		const FEE: Permill = Permill::from_percent(50);
		assert_ok!(VtokenMinting::<T>::set_fees(RawOrigin::Root.into(), FEE, FEE));
		assert_ok!(VtokenMinting::<T>::set_unlock_duration(T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::<T>::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		T::MultiCurrency::deposit(KSM, &caller, token_amount)?;
		T::MultiCurrency::deposit(VKSM, &caller, redeem_amount)?;
		assert_ok!(VtokenMinting::<T>::mint(RawOrigin::Signed(caller.clone()).into(), KSM, mint_amount,BoundedVec::default(), None));
		assert_ok!(VtokenMinting::<T>::redeem(RawOrigin::Signed(caller.clone()).into(), VKSM, redeem_amount));
		assert_ok!(VtokenMinting::<T>::add_support_rebond_token(T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?, KSM));
		let unlock_id:UnlockId = 0;
	}: _(RawOrigin::Signed(caller), KSM, unlock_id)

	on_initialize {
		let block_num =BlockNumberFor::<T>::from(10u32);
	}:{VtokenMinting::<T>::on_initialize(block_num);}

	mint_with_lock {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let caller: T::AccountId = whitelisted_caller();
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_amount = BalanceOf::<T>::unique_saturated_from(10000000000u128);
		T::MultiCurrency::deposit(KSM, &caller, token_amount)?;

		pub const FEE: Permill = Permill::from_percent(5);
		assert_ok!(VtokenMinting::<T>::set_fees(RawOrigin::Root.into(), FEE, FEE));
		// Set minimum mint
		assert_ok!(VtokenMinting::<T>::set_minimum_mint(origin.clone(), KSM,  BalanceOf::<T>::unique_saturated_from(100u128)));
		// set vtoken coefficient
		assert_ok!(VtokenMinting::<T>::set_incentive_coef(origin.clone(), VKSM, Some(1)));
		// set incentive pool balance
		assert_ok!(T::MultiCurrency::deposit(
			VKSM,
			&VtokenMinting::<T>::incentive_pool_account(),
			BalanceOf::<T>::unique_saturated_from(100000000000000000000u128)
		));
		// set incentive lock blocks
		assert_ok!(VtokenMinting::<T>::set_vtoken_incentive_lock_blocks(
			origin.clone(),
			VKSM,
			Some(BlockNumberFor::<T>::from(100u32))
		));
	}: _(RawOrigin::Signed(caller), KSM, token_amount,BoundedVec::default(), None)

	unlock_incentive_minted_vtoken {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;

		let caller: T::AccountId = whitelisted_caller();
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_amount = BalanceOf::<T>::unique_saturated_from(10000000000u128);
		T::MultiCurrency::deposit(KSM, &caller, token_amount)?;

		pub const FEE: Permill = Permill::from_percent(5);
		assert_ok!(VtokenMinting::<T>::set_fees(RawOrigin::Root.into(), FEE, FEE));
		// Set minimum mint
		assert_ok!(VtokenMinting::<T>::set_minimum_mint(origin.clone(), KSM, BalanceOf::<T>::unique_saturated_from(100u128)));
		// set vtoken coefficient
		assert_ok!(VtokenMinting::<T>::set_incentive_coef(origin.clone(), VKSM, Some(1)));
		// set incentive pool balance
		assert_ok!(T::MultiCurrency::deposit(
			VKSM,
			&VtokenMinting::<T>::incentive_pool_account(),
			BalanceOf::<T>::unique_saturated_from(100000000000000000000u128)
		));
		// set incentive lock blocks
		assert_ok!(VtokenMinting::<T>::set_vtoken_incentive_lock_blocks(
			origin.clone(),
			VKSM,
			Some(BlockNumberFor::<T>::from(100u32))
		));
		// mint with lock
		assert_ok!(VtokenMinting::<T>::mint_with_lock(
			RawOrigin::Signed(caller.clone()).into(),
			KSM,
			BalanceOf::<T>::unique_saturated_from(10000000000u128),
			BoundedVec::default(),
			None
		));

		frame_system::Pallet::<T>::set_block_number(BlockNumberFor::<T>::from(101u32));

	}: _(RawOrigin::Signed(caller), VKSM)

	set_incentive_coef {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let coef = 1u128;
	}: _<T::RuntimeOrigin>(origin, VKSM, Some(coef))

	set_vtoken_incentive_lock_blocks {
		let origin = T::ControlOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let blocks = Some(BlockNumberFor::<T>::from(1000u32));
	}: _<T::RuntimeOrigin>(origin, VKSM, blocks)

	impl_benchmark_test_suite!(
	VtokenMinting,
	crate::mock::ExtBuilder::default().one_hundred_for_alice_n_bob().build(),
	crate::mock::Runtime,
);
}

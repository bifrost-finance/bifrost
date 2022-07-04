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

use crate::{Pallet as VtokenMinting, *};
use frame_benchmarking::{benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::{assert_ok, sp_runtime::traits::UniqueSaturatedFrom};
use frame_system::RawOrigin;
use node_primitives::{CurrencyId, TokenSymbol};

benchmarks! {
	set_minimum_mint {
		let token = CurrencyId::Token(TokenSymbol::try_from(0u8).unwrap_or_default());
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u32 as u128);
	}: _(RawOrigin::Root, token, token_amount)

	set_minimum_redeem {
		let token = CurrencyId::Token(TokenSymbol::try_from(0u8).unwrap_or_default());
		let token_amount = BalanceOf::<T>::unique_saturated_from(0u32 as u128);
	}: _(RawOrigin::Root, token, token_amount)

	set_unlock_duration {
		let token = CurrencyId::Token(TokenSymbol::try_from(0u8).unwrap_or_default());
	}: _(RawOrigin::Root, token, TimeUnit::Era(1))

	add_support_rebond_token {
		let token = CurrencyId::Token(TokenSymbol::try_from(0u8).unwrap_or_default());
	}: _(RawOrigin::Root, token)

	remove_support_rebond_token {
		let token = CurrencyId::Token(TokenSymbol::try_from(0u8).unwrap_or_default());
		assert_ok!(VtokenMinting::<T>::add_support_rebond_token(RawOrigin::Root.into(), token));
	}: _(RawOrigin::Root, token)

	set_fees {
		const FEE: Permill = Permill::from_percent(5);
	}: _(RawOrigin::Root, FEE, FEE)

	set_hook_iteration_limit {
	}: _(RawOrigin::Root, 10u32)

	mint {
		let caller: T::AccountId = whitelisted_caller();
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let token_amount = BalanceOf::<T>::unique_saturated_from(10000000000u128);
	}: _(RawOrigin::Signed(caller), KSM, token_amount)

	redeem {
		let caller: T::AccountId = whitelisted_caller();
		const VKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let vtoken_amount = BalanceOf::<T>::unique_saturated_from(90u128);
		let redeem_amount = BalanceOf::<T>::unique_saturated_from(1000000000u128);
		let token_amount = BalanceOf::<T>::unique_saturated_from(10000000000u128);
		const FEE: Permill = Permill::from_percent(50);
		assert_ok!(VtokenMinting::<T>::set_fees(RawOrigin::Root.into(), FEE, FEE));
		assert_ok!(VtokenMinting::<T>::set_unlock_duration(RawOrigin::Root.into(), KSM, TimeUnit::Era(1)));
		// assert_ok!(VtokenMinting::<T>::increase_token_pool(KSM, token_amount));
		assert_ok!(VtokenMinting::<T>::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		// assert_ok!(VtokenMinting::<T>::set_minimum_redeem(RawOrigin::Root.into(), VKSM, vtoken_amount));
		assert_ok!(VtokenMinting::<T>::mint(RawOrigin::Signed(caller.clone()).into(), KSM, token_amount));
	}: _(RawOrigin::Signed(caller.clone()), VKSM, redeem_amount)

	rebond {
		let caller: T::AccountId = whitelisted_caller();
		const VKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let rebond_amount = BalanceOf::<T>::unique_saturated_from(200u128);
		let redeem_amount = BalanceOf::<T>::unique_saturated_from(1000000000u128);
		let token_amount = BalanceOf::<T>::unique_saturated_from(10000000000u128);
		const FEE: Permill = Permill::from_percent(50);
		assert_ok!(VtokenMinting::<T>::set_fees(RawOrigin::Root.into(), FEE, FEE));
		assert_ok!(VtokenMinting::<T>::set_unlock_duration(RawOrigin::Root.into(), KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::<T>::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::<T>::mint(RawOrigin::Signed(caller.clone()).into(), KSM, token_amount));
		assert_ok!(VtokenMinting::<T>::redeem(RawOrigin::Signed(caller.clone()).into(), VKSM, redeem_amount));
		assert_ok!(VtokenMinting::<T>::add_support_rebond_token(RawOrigin::Root.into(), KSM));
	}: _(RawOrigin::Signed(caller), KSM, rebond_amount)

	rebond_by_unlock_id {
		let caller: T::AccountId = whitelisted_caller();
		const VKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
		const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
		let rebond_amount = BalanceOf::<T>::unique_saturated_from(200u128);
		let redeem_amount = BalanceOf::<T>::unique_saturated_from(1000000000u128);
		let token_amount = BalanceOf::<T>::unique_saturated_from(10000000000u128);
		const FEE: Permill = Permill::from_percent(50);
		assert_ok!(VtokenMinting::<T>::set_fees(RawOrigin::Root.into(), FEE, FEE));
		assert_ok!(VtokenMinting::<T>::set_unlock_duration(RawOrigin::Root.into(), KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::<T>::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::<T>::mint(RawOrigin::Signed(caller.clone()).into(), KSM, token_amount));
		assert_ok!(VtokenMinting::<T>::redeem(RawOrigin::Signed(caller.clone()).into(), VKSM, redeem_amount));
		assert_ok!(VtokenMinting::<T>::add_support_rebond_token(RawOrigin::Root.into(), KSM));
		let unlock_id:UnlockId = 0;
	}: _(RawOrigin::Signed(caller), KSM, unlock_id)

	on_initialize {
		let block_num = T::BlockNumber::from(10u32);
	}:{VtokenMinting::<T>::on_initialize(block_num);}
}

impl_benchmark_test_suite!(
	VtokenMinting,
	crate::mock::ExtBuilder::default().one_hundred_for_alice_n_bob().build(),
	crate::mock::Test,
);

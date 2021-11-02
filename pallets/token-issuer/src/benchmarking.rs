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

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::{dispatch::UnfilteredDispatchable, sp_runtime::traits::UniqueSaturatedFrom};
use frame_system::RawOrigin;
use node_primitives::{CurrencyId, TokenSymbol};

use super::*;
#[allow(unused_imports)]
use crate::Pallet as TokenIssuer;

benchmarks! {
	add_to_issue_whitelist {
		let origin = T::ControlOrigin::successful_origin();
		let account: T::AccountId = whitelisted_caller();
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let call = Call::<T>::add_to_issue_whitelist { currency_id, account };
	}: {call.dispatch_bypass_filter(origin)?}

	remove_from_issue_whitelist {
		let origin = T::ControlOrigin::successful_origin();
		let account: T::AccountId = whitelisted_caller();
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let add_call = Call::<T>::add_to_issue_whitelist { currency_id, account: account.clone() };
		add_call.dispatch_bypass_filter(origin.clone())?;

		let remove_call = Call::<T>::remove_from_issue_whitelist { currency_id, account };
	}: {remove_call.dispatch_bypass_filter(origin)?}

	add_to_transfer_whitelist {
		let origin = T::ControlOrigin::successful_origin();
		let account: T::AccountId = whitelisted_caller();
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let call = Call::<T>::add_to_transfer_whitelist { currency_id, account };
	}: {call.dispatch_bypass_filter(origin)?}

	remove_from_transfer_whitelist {
		let origin = T::ControlOrigin::successful_origin();
		let account: T::AccountId = whitelisted_caller();
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let add_call = Call::<T>::add_to_transfer_whitelist { currency_id, account: account.clone() };
		add_call.dispatch_bypass_filter(origin.clone())?;

		let remove_call = Call::<T>::remove_from_transfer_whitelist { currency_id, account };
	}: {remove_call.dispatch_bypass_filter(origin)?}

	issue {
		let origin = T::ControlOrigin::successful_origin();
		let caller: T::AccountId = whitelisted_caller();
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let add_call = Call::<T>::add_to_issue_whitelist { currency_id: currency_id.clone(), account: caller.clone() };
		add_call.dispatch_bypass_filter(origin.clone())?;

		let original_balance = T::MultiCurrency::free_balance(currency_id.clone(), &caller);
		let token_amount = BalanceOf::<T>::unique_saturated_from(1000u32 as u128);
	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), currency_id.clone(), token_amount)
	verify {
		assert_eq!(T::MultiCurrency::free_balance(currency_id.clone(), &caller), token_amount + original_balance);
	}

	transfer {
		let origin = T::ControlOrigin::successful_origin();
		let caller: T::AccountId = whitelisted_caller();
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);

		// add caller to the transfer whitelist
		let add_transfer_call = Call::<T>::add_to_transfer_whitelist { currency_id: currency_id.clone(), account: caller.clone() };
		add_transfer_call.dispatch_bypass_filter(origin.clone())?;

		// transfer some ksm from caller account to receiver account
		let receiver: T::AccountId = account("bechmarking_account_1", 0, 0);
		let transfer_token_amount = BalanceOf::<T>::unique_saturated_from(800u32 as u128);
		let caller_original_balance = T::MultiCurrency::free_balance(currency_id.clone(), &caller);
		let receiver_original_balance = T::MultiCurrency::free_balance(currency_id.clone(), &receiver);
	}: _(RawOrigin::Signed(caller.clone()), receiver.clone(), currency_id.clone(), transfer_token_amount)
	verify {
		assert_eq!(T::MultiCurrency::free_balance(currency_id.clone(), &caller), caller_original_balance - transfer_token_amount);
		assert_eq!(T::MultiCurrency::free_balance(currency_id.clone(), &receiver), transfer_token_amount+ receiver_original_balance);
	}
}

impl_benchmark_test_suite!(
	TokenIssuer,
	crate::mock::ExtBuilder::default()
		// .one_hundred_precision_for_each_currency_type_for_whitelist_account()
		.build(),
	crate::mock::Test
);

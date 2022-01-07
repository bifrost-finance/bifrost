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

use frame_benchmarking::{account, whitelisted_caller};
use frame_support::{assert_ok, traits::Currency};
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrencyExtended;
use sp_runtime::{
	traits::{StaticLookup, UniqueSaturatedInto},
	SaturatedConversion,
};
use sp_std::prelude::*;

use crate::{
	AccountId, Amount, Balance, Balances, Currencies, CurrencyId, ExistentialDeposit,
	NativeCurrencyId, Runtime, TokenSymbol,
};

const SEED: u32 = 0;
const NATIVE: CurrencyId = NativeCurrencyId::get();

pub fn set_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) {
	assert_ok!(<Currencies as MultiCurrencyExtended<_>>::update_balance(
		currency_id,
		who,
		balance.saturated_into()
	));
}

runtime_benchmarks! {
	{ Runtime, orml_currencies }

	// `transfer` non-native currency
	transfer_non_native_currency {
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let amount: Balance = Balances::minimum_balance().saturating_add(100u32.into());
		let from: AccountId = whitelisted_caller();
		set_balance(currency_id, &from, amount);

		let receiver: AccountId = account("bechmarking_account_1", 0, 0);
		let recipient =  <Runtime as frame_system::Config>::Lookup::unlookup(receiver.clone());
	}: transfer(RawOrigin::Signed(from), recipient, currency_id, amount)


	// `transfer` native currency and in worst case
	#[extra]
	transfer_native_currency_worst_case {
		let existential_deposit = ExistentialDeposit::get();
		let amount: Balance = existential_deposit.saturating_mul(1000);
		let from: AccountId = whitelisted_caller();
		set_balance(NATIVE, &from, amount);

		let to: AccountId = account("to", 0, SEED);
		let to_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(to.clone());

	}: transfer(RawOrigin::Signed(from), to_lookup, NATIVE, amount)

	// `transfer_native_currency` in worst case
	// * will create the `to` account.
	// * will kill the `from` account.
	transfer_native_currency {
		let existential_deposit = ExistentialDeposit::get();
		let amount: Balance = existential_deposit.saturating_mul(1000);
		let from: AccountId = whitelisted_caller();
		set_balance(NATIVE, &from, amount);

		let to: AccountId = account("to", 0, SEED);
		let to_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(to.clone());
	}: _(RawOrigin::Signed(from), to_lookup, amount)

	// `update_balance` for non-native currency
	update_balance_non_native_currency {
		let currency_id = CurrencyId::Token(TokenSymbol::KSM);
		let balance: Balance = 1_000_000_000_000u128.into();
		let amount: Amount = balance.unique_saturated_into();
		let who: AccountId = account("who", 0, SEED);
		let who_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(who.clone());
	}: update_balance(RawOrigin::Root, who_lookup, currency_id, amount)

	// `update_balance` for native currency
	// * will create the `who` account.
	update_balance_native_currency_creating {
		let existential_deposit = ExistentialDeposit::get();
		let balance: Balance = existential_deposit.saturating_mul(1000);
		let amount: Amount = balance.unique_saturated_into();
		let who: AccountId = account("who", 0, SEED);
		let who_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(who.clone());
	}: update_balance(RawOrigin::Root, who_lookup, NATIVE, amount)

	// `update_balance` for native currency
	// * will kill the `who` account.
	update_balance_native_currency_killing {
		let existential_deposit = ExistentialDeposit::get();
		let balance: Balance = existential_deposit.saturating_mul(1000);
		let amount: Amount = balance.unique_saturated_into();
		let who: AccountId = account("who", 0, SEED);
		let who_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(who.clone());
		set_balance(NATIVE, &who, balance);
	}: update_balance(RawOrigin::Root, who_lookup, NATIVE, -amount)
}

#[cfg(test)]
mod tests {
	use orml_benchmarking::impl_benchmark_test_suite;

	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;

	impl_benchmark_test_suite!(new_test_ext(),);
}

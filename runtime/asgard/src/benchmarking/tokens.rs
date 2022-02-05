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

use frame_benchmarking::{account, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use orml_benchmarking::runtime_benchmarks;
use orml_traits::MultiCurrencyExtended;
use sp_runtime::{
	traits::{StaticLookup, UniqueSaturatedInto},
	SaturatedConversion,
};
use sp_std::prelude::*;

use crate::{
	AccountId, Amount, Balance, Currencies, CurrencyId, ExistentialDeposit, Runtime,
	StableCurrencyId, TokenSymbol,
};

const SEED: u32 = 0;
const STABLECOIN: CurrencyId = StableCurrencyId::get();

pub fn update_balance(currency_id: CurrencyId, who: &AccountId, balance: Balance) {
	assert_ok!(<Currencies as MultiCurrencyExtended<_>>::update_balance(
		currency_id,
		who,
		balance.saturated_into()
	));
}

runtime_benchmarks! {
	{ Runtime, orml_tokens }

	transfer {
		let amount: Balance = 1_000_000_000_000u128;
		let from: AccountId = whitelisted_caller();
		update_balance(STABLECOIN, &from, amount);

		let to: AccountId = account("to", 0, SEED);
		let to_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(to.clone());
	}: _(RawOrigin::Signed(from), to_lookup, STABLECOIN, amount)

	transfer_all {
		let amount: Balance = 1_000_000_000_000u128;

		let from: AccountId = whitelisted_caller();
		update_balance(STABLECOIN, &from, amount);

		let to: AccountId = account("to", 0, SEED);
		let to_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(to.clone());
	}: _(RawOrigin::Signed(from.clone()), to_lookup, STABLECOIN, false)

	transfer_keep_alive {
		let balance: Balance = 1_000_000_000_000u128;
		let from: AccountId = whitelisted_caller();
		update_balance(STABLECOIN, &from, balance * 2);

		let to: AccountId = account("to", 0, SEED);
		let to_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(to.clone());
	}: _(RawOrigin::Signed(from), to_lookup, STABLECOIN, balance)

	force_transfer {
		let balance: Balance = 1_000_000_000_000u128;
		let from: AccountId = account("from", 0, SEED);
		let from_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(from.clone());
		update_balance(STABLECOIN, &from, 2 * balance);

		let to: AccountId = account("to", 0, SEED);
		let to_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(to.clone());
	}: _(RawOrigin::Root, from_lookup, to_lookup, STABLECOIN, balance)

	set_balance {
		let balance: Balance = 1_000_000_000_000u128;
		let who: AccountId = account("who", 0, SEED);
		let who_lookup = <Runtime as frame_system::Config>::Lookup::unlookup(who.clone());

	}: _(RawOrigin::Root, who_lookup, STABLECOIN, balance, balance)
}

#[cfg(test)]
mod tests {
	use orml_benchmarking::impl_benchmark_test_suite;

	use super::*;
	use crate::benchmarking::utils::tests::new_test_ext;

	impl_benchmark_test_suite!(new_test_ext(),);
}

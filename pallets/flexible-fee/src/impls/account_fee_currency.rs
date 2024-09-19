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

use crate::{Config, Error, Pallet, UniversalFeeCurrencyOrderList, UserDefaultFeeCurrency};
use bifrost_primitives::{AccountFeeCurrency, BalanceCmp, CurrencyId, WETH};
use frame_support::traits::{
	fungibles::Inspect,
	tokens::{Fortitude, Preservation},
};
use sp_arithmetic::traits::UniqueSaturatedInto;
use sp_core::U256;
use sp_std::cmp::Ordering;

/// Provides account's fee payment asset or default fee asset ( Native asset )
impl<T: Config> AccountFeeCurrency<T::AccountId> for Pallet<T> {
	type Error = Error<T>;

	/// Determines the appropriate currency to be used for paying transaction fees based on a
	/// prioritized order:
	/// 1. User's default fee currency (`UserDefaultFeeCurrency`)
	/// 2. WETH
	/// 3. Currencies in the `UniversalFeeCurrencyOrderList`
	///
	/// The method first checks if the balance of the highest-priority currency is sufficient to
	/// cover the fee.If the balance is insufficient, it iterates through the list of currencies in
	/// priority order.If no currency has a sufficient balance, it returns the currency with the
	/// highest balance.
	fn get_fee_currency(account: &T::AccountId, fee: U256) -> Result<CurrencyId, Error<T>> {
		let fee: u128 = fee.unique_saturated_into();
		let priority_currency = UserDefaultFeeCurrency::<T>::get(account);
		let mut currency_list = UniversalFeeCurrencyOrderList::<T>::get();

		let first_item_index = 0;
		currency_list
			.try_insert(first_item_index, WETH)
			.map_err(|_| Error::<T>::MaxCurrenciesReached)?;

		// When all currency balances are insufficient, return the one with the highest balance
		let mut hopeless_currency = WETH;

		if let Some(currency) = priority_currency {
			currency_list
				.try_insert(first_item_index, currency)
				.map_err(|_| Error::<T>::MaxCurrenciesReached)?;
			hopeless_currency = currency;
		}

		for maybe_currency in currency_list.iter() {
			let comp_res = Self::cmp_with_precision(account, maybe_currency, fee, 18)?;

			match comp_res {
				Ordering::Less => {
					// Get the currency with the highest balance
					let hopeless_currency_balance = T::MultiCurrency::reducible_balance(
						hopeless_currency,
						account,
						Preservation::Preserve,
						Fortitude::Polite,
					);
					let maybe_currency_balance = T::MultiCurrency::reducible_balance(
						*maybe_currency,
						account,
						Preservation::Preserve,
						Fortitude::Polite,
					);
					hopeless_currency = match hopeless_currency_balance.cmp(&maybe_currency_balance)
					{
						Ordering::Less => *maybe_currency,
						_ => hopeless_currency,
					};
					continue;
				},
				Ordering::Equal => return Ok(*maybe_currency),
				Ordering::Greater => return Ok(*maybe_currency),
			};
		}

		return Ok(hopeless_currency);
	}
}

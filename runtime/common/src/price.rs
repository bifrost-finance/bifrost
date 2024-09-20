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

use frame_support::{
	pallet_prelude::Get,
	traits::tokens::{Fortitude, Preservation},
};
use sp_core::U256;
use sp_runtime::{
	helpers_128bit::multiply_by_rational_with_rounding, traits::Convert, DispatchError, Rounding,
};
use sp_std::marker::PhantomData;
use xcm::latest::Weight;

use bifrost_primitives::{
	AccountFeeCurrency, AccountFeeCurrencyBalanceInCurrency, Balance, CurrencyId, PriceFeeder,
	PriceProvider,
};

use crate::Ratio;

pub struct OraclePriceProvider<PF>(PhantomData<PF>);

impl<PF> PriceProvider for OraclePriceProvider<PF>
where
	PF: PriceFeeder,
{
	type Price = Ratio;

	fn get_price(asset_a: CurrencyId, asset_b: CurrencyId) -> Option<Self::Price> {
		if let Some(a) = PF::get_normal_price(&asset_a) {
			if let Some(b) = PF::get_normal_price(&asset_b) {
				Some(Ratio::from((a, b)))
			} else {
				None
			}
		} else {
			None
		}
	}
}

pub struct FeeAssetBalanceInCurrency<T, C, AC, I>(PhantomData<(T, C, AC, I)>);

impl<T, C, AC, I> AccountFeeCurrencyBalanceInCurrency<T::AccountId>
	for FeeAssetBalanceInCurrency<T, C, AC, I>
where
	T: frame_system::Config,
	C: Convert<(CurrencyId, CurrencyId, Balance), Option<(Balance, Ratio)>>,
	AC: AccountFeeCurrency<T::AccountId>,
	I: frame_support::traits::fungibles::Inspect<
		T::AccountId,
		AssetId = CurrencyId,
		Balance = Balance,
	>,
{
	type Output = (Balance, Weight);
	type Error = DispatchError;

	fn get_balance_in_currency(
		to_currency: CurrencyId,
		account: &T::AccountId,
		fee: U256,
	) -> Result<Self::Output, DispatchError> {
		let from_currency = AC::get_fee_currency(account, fee)
			.map_err(|_| DispatchError::Other("Get Currency Error."))?;
		let account_balance =
			I::reducible_balance(from_currency, account, Preservation::Preserve, Fortitude::Polite);
		let price_weight = T::DbWeight::get().reads(2); // 1 read to get currency and 1 read to get balance

		if from_currency == to_currency {
			return Ok((account_balance, price_weight));
		}

		let Some((converted, _)) = C::convert((from_currency, to_currency, account_balance)) else {
			return Ok((0, price_weight));
		};
		Ok((converted, price_weight))
	}
}

pub struct ConvertAmount<P>(PhantomData<P>);

// Converts `amount` of `from_currency` to `to_currency` using given oracle
// Input: (from_currency, to_currency, amount)
// Output: Option<(converted_amount, price)>
impl<P> Convert<(CurrencyId, CurrencyId, Balance), Option<(Balance, Ratio)>> for ConvertAmount<P>
where
	P: PriceProvider<Price = Ratio>,
{
	fn convert(
		(from_currency, to_currency, amount): (CurrencyId, CurrencyId, Balance),
	) -> Option<(Balance, Ratio)> {
		if from_currency == to_currency {
			return Some((amount, Ratio::one()));
		}
		let price = P::get_price(from_currency, to_currency)?;
		let converted = multiply_by_rational_with_rounding(amount, price.n, price.d, Rounding::Up)?;
		Some((converted, price))
	}
}

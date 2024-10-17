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

#![cfg(test)]

use bifrost_primitives::{
	Balance, CurrencyId, OraclePriceProvider, Price, PriceDetail, BNC, DOT, DOT_U, KSM, MANTA,
	VDOT, VKSM, WETH,
};
use frame_support::parameter_types;
use sp_runtime::FixedU128;
use std::collections::BTreeMap;

parameter_types! {
	pub static StoragePrice: BTreeMap<CurrencyId, (Price, u128)> = BTreeMap::from([
		(BNC, (FixedU128::from_inner(200_000_000_000_000_000), 10u128.pow(12))),
		(MANTA, (FixedU128::from_inner(800_000_000_000_000_000), 10u128.pow(18))),
		(DOT, (FixedU128::from(5), 10u128.pow(10))),
		(VDOT, (FixedU128::from(6), 10u128.pow(10))),
		(DOT_U, (FixedU128::from(1), 10u128.pow(6))),
		(KSM, (FixedU128::from(20), 10u128.pow(12))),
		(VKSM, (FixedU128::from(25), 10u128.pow(12))),
		(WETH, (FixedU128::from(3000), 10u128.pow(18))),
	]);
}

pub struct MockOraclePriceProvider;
impl MockOraclePriceProvider {
	pub fn set_price(currency_id: CurrencyId, price: Price) {
		let mut storage_price = StoragePrice::get();
		match storage_price.get(&currency_id) {
			Some((_, mantissa)) => {
				storage_price.insert(currency_id, (price, *mantissa));
			},
			None => {
				storage_price.insert(currency_id, (price, 10u128.pow(12)));
			},
		};
		StoragePrice::set(storage_price);
	}
}

impl OraclePriceProvider for MockOraclePriceProvider {
	fn get_price(currency_id: &CurrencyId) -> Option<PriceDetail> {
		match StoragePrice::get().get(currency_id) {
			Some((price, _)) => Some((*price, 0)),
			None => None,
		}
	}

	fn get_amount_by_prices(
		currency_in: &CurrencyId,
		amount_in: Balance,
		currency_in_price: Price,
		currency_out: &CurrencyId,
		currency_out_price: Price,
	) -> Option<Balance> {
		if let Some((_, currency_in_mantissa)) = StoragePrice::get().get(&currency_in) {
			if let Some((_, currency_out_mantissa)) = StoragePrice::get().get(&currency_out) {
				let total_value = currency_in_price
					.mul(FixedU128::from_inner(amount_in))
					.div(FixedU128::from_inner(*currency_in_mantissa));
				let amount_out = total_value
					.mul(FixedU128::from_inner(*currency_out_mantissa))
					.div(currency_out_price);
				return Some(amount_out.into_inner());
			}
		}
		None
	}

	fn get_oracle_amount_by_currency_and_amount_in(
		currency_in: &CurrencyId,
		amount_in: Balance,
		currency_out: &CurrencyId,
	) -> Option<(Balance, Price, Price)> {
		if let Some((currency_in_price, currency_in_mantissa)) =
			StoragePrice::get().get(&currency_in)
		{
			if let Some((currency_out_price, currency_out_mantissa)) =
				StoragePrice::get().get(&currency_out)
			{
				let total_value = currency_in_price
					.mul(FixedU128::from_inner(amount_in))
					.div(FixedU128::from_inner(*currency_in_mantissa));
				let amount_out = total_value
					.mul(FixedU128::from_inner(*currency_out_mantissa))
					.div(*currency_out_price);
				return Some((amount_out.into_inner(), *currency_in_price, *currency_out_price));
			}
		}
		None
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn set_price() {
		assert_eq!(
			MockOraclePriceProvider::get_price(&BNC),
			Some((FixedU128::from_inner(200_000_000_000_000_000), 0))
		);
		MockOraclePriceProvider::set_price(BNC, FixedU128::from(100));
		assert_eq!(MockOraclePriceProvider::get_price(&BNC), Some((FixedU128::from(100), 0)));

		MockOraclePriceProvider::set_price(DOT, FixedU128::from(100));
		assert_eq!(MockOraclePriceProvider::get_price(&DOT), Some((FixedU128::from(100), 0)));
	}

	#[test]
	fn get_oracle_amount_by_currency_and_amount_in() {
		let bnc_amount = 100 * 10u128.pow(12);
		let dot_amount = 4 * 10u128.pow(10);
		let ksm_amount = 1 * 10u128.pow(12);
		let usdt_amount = 20 * 10u128.pow(6);
		let manta_amount = 25 * 10u128.pow(18);
		assert_eq!(
			MockOraclePriceProvider::get_oracle_amount_by_currency_and_amount_in(
				&BNC, bnc_amount, &DOT
			),
			Some((dot_amount, FixedU128::from_inner(200_000_000_000_000_000), FixedU128::from(5)))
		);
		assert_eq!(
			MockOraclePriceProvider::get_oracle_amount_by_currency_and_amount_in(
				&BNC, bnc_amount, &DOT_U
			),
			Some((usdt_amount, FixedU128::from_inner(200_000_000_000_000_000), FixedU128::from(1)))
		);
		assert_eq!(
			MockOraclePriceProvider::get_oracle_amount_by_currency_and_amount_in(
				&BNC, bnc_amount, &KSM
			),
			Some((ksm_amount, FixedU128::from_inner(200_000_000_000_000_000), FixedU128::from(20)))
		);
		assert_eq!(
			MockOraclePriceProvider::get_oracle_amount_by_currency_and_amount_in(
				&BNC, bnc_amount, &MANTA
			),
			Some((
				manta_amount,
				FixedU128::from_inner(200_000_000_000_000_000),
				FixedU128::from_inner(800_000_000_000_000_000)
			))
		);
		assert_eq!(
			MockOraclePriceProvider::get_oracle_amount_by_currency_and_amount_in(
				&DOT, dot_amount, &DOT_U
			),
			Some((usdt_amount, FixedU128::from(5), FixedU128::from(1)))
		);
		assert_eq!(
			MockOraclePriceProvider::get_oracle_amount_by_currency_and_amount_in(
				&DOT, dot_amount, &KSM
			),
			Some((ksm_amount, FixedU128::from(5), FixedU128::from(20)))
		);
	}
}

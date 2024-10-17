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

use crate::{Balance, CurrencyId, Price, PriceDetail};

pub trait OraclePriceProvider {
	fn get_price(asset_id: &CurrencyId) -> Option<PriceDetail>;
	fn get_amount_by_prices(
		currency_in: &CurrencyId,
		amount_in: Balance,
		currency_in_price: Price,
		currency_out: &CurrencyId,
		currency_out_price: Price,
	) -> Option<Balance>;
	fn get_oracle_amount_by_currency_and_amount_in(
		currency_in: &CurrencyId,
		amount_in: Balance,
		currency_out: &CurrencyId,
	) -> Option<(Balance, Price, Price)>;
}

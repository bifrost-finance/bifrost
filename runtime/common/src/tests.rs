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

use node_primitives::{CurrencyId, TokenSymbol};

use crate::{cent, micro};

#[test]
fn cal_currency_unit_by_decimal_should_work() {
	assert_eq!(1 * cent(CurrencyId::Token(TokenSymbol::DOT)), 100_000_000);
	assert_eq!(1 * micro(CurrencyId::Token(TokenSymbol::ZLK)), 1_000_000_000_000);
}

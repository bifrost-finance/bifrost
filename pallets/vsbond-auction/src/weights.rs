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

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for the pallet.
pub trait WeightInfo {
	fn create_order() -> Weight;
	fn revoke_order() -> Weight;
	fn clinch_order() -> Weight;
	fn partial_clinch_order() -> Weight;
	fn set_buy_and_sell_transaction_fee_rate() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn create_order() -> Weight {
		(50_000_000 as Weight)
	}

	fn revoke_order() -> Weight {
		(50_000_000 as Weight)
	}

	fn clinch_order() -> Weight {
		(50_000_000 as Weight)
	}

	fn partial_clinch_order() -> Weight {
		(50_000_000 as Weight)
	}

	fn set_buy_and_sell_transaction_fee_rate() -> Weight {
		(50_000_000 as Weight)
	}
}

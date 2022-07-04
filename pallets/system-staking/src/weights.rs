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
	fn on_initialize(x: u32) -> Weight;
	fn token_config() -> Weight;
	fn delete_token() -> Weight;
	fn refresh_token_info() -> Weight;
	fn payout() -> Weight;
	fn on_redeem_success() -> Weight;
	fn on_redeemed() -> Weight;
}

pub struct SubstrateWeight<T>(PhantomData<T>);
// For backwards compatibility and tests
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn on_initialize(x: u32) -> Weight {
		(50_000_000 as Weight).saturating_add((50_000_000 as Weight).saturating_mul(x as Weight))
	}

	fn token_config() -> Weight {
		(50_000_000 as Weight)
	}

	fn delete_token() -> Weight {
		(50_000_000 as Weight)
	}

	fn refresh_token_info() -> Weight {
		(100_000_000 as Weight)
	}

	fn payout() -> Weight {
		(100_000_000 as Weight)
	}

	fn on_redeem_success() -> Weight {
		(50_000_000 as Weight)
	}

	fn on_redeemed() -> Weight {
		(50_000_000 as Weight)
	}
}

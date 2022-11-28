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
	fn on_initialize() -> Weight;
	fn set_config() -> Weight;
	fn close() -> Weight;
	fn charge() -> Weight;
	fn payout() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn on_initialize() -> Weight {
		Weight::from_ref_time(50_000_000 as u64)
	}

	fn set_config() -> Weight {
		Weight::from_ref_time(50_000_000 as u64)
	}

	fn close() -> Weight {
		Weight::from_ref_time(50_000_000 as u64)
	}

	fn charge() -> Weight {
		Weight::from_ref_time(50_000_000 as u64)
	}

	fn payout() -> Weight {
		Weight::from_ref_time(50_000_000 as u64)
	}
}

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
	sp_std::marker::PhantomData,
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};

/// Weight functions needed for the pallet.
pub trait WeightInfo {
	fn charge() -> Weight;
	fn deposit() -> Weight;
	fn redeem() -> Weight;
	fn redeem_all() -> Weight;
	fn volunteer_to_redeem() -> Weight;
	fn claim() -> Weight;
	fn unlock() -> Weight;
	fn cancel_unlock() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn charge() -> Weight {
		(50_000_000 as Weight)
	}

	fn deposit() -> Weight {
		(50_000_000 as Weight)
	}

	fn redeem() -> Weight {
		(50_000_000 as Weight)
	}

	fn redeem_all() -> Weight {
		(50_000_000 as Weight)
	}

	fn volunteer_to_redeem() -> Weight {
		(50_000_000 as Weight)
	}

	fn claim() -> Weight {
		(50_000_000 as Weight)
	}

	fn unlock() -> Weight {
		(50_000_000 as Weight)
	}

	fn cancel_unlock() -> Weight {
		(50_000_000 as Weight)
	}
}

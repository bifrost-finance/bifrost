// Copyright 2019-2020 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{Weight, constants::RocksDbWeight as DbWeight};

pub struct WeightInfo;
impl pallet_vesting::WeightInfo for WeightInfo {
	fn vest_locked(l: u32, ) -> Weight {
		(82109000 as Weight)
			.saturating_add((332000 as Weight).saturating_mul(l as Weight))
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn vest_unlocked(l: u32, ) -> Weight {
		(88419000 as Weight)
			.saturating_add((3000 as Weight).saturating_mul(l as Weight))
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
	fn vest_other_locked(l: u32, ) -> Weight {
		(81277000 as Weight)
			.saturating_add((321000 as Weight).saturating_mul(l as Weight))
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
	fn vest_other_unlocked(l: u32, ) -> Weight {
		(87584000 as Weight)
			.saturating_add((19000 as Weight).saturating_mul(l as Weight))
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn vested_transfer(l: u32, ) -> Weight {
		(185916000 as Weight)
			.saturating_add((625000 as Weight).saturating_mul(l as Weight))
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn force_vested_transfer(l: u32, ) -> Weight {
		(185916000 as Weight)
			.saturating_add((625000 as Weight).saturating_mul(l as Weight))
			.saturating_add(DbWeight::get().reads(4 as Weight))
			.saturating_add(DbWeight::get().writes(4 as Weight))
	}
}

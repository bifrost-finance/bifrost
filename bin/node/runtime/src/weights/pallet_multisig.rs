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
impl pallet_multisig::WeightInfo for WeightInfo {
	fn as_multi_threshold_1(z: u32, ) -> Weight {
		(17_161_000 as Weight)
			.saturating_add((1_000 as Weight).saturating_mul(z as Weight))
	}
	fn as_multi_create(s: u32, z: u32, ) -> Weight {
		(79_857_000 as Weight)
			.saturating_add((131_000 as Weight).saturating_mul(s as Weight))
			.saturating_add((1_000 as Weight).saturating_mul(z as Weight))
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn as_multi_create_store(s: u32, z: u32, ) -> Weight {
		(90_218_000 as Weight)
			.saturating_add((129_000 as Weight).saturating_mul(s as Weight))
			.saturating_add((3_000 as Weight).saturating_mul(z as Weight))
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
	fn as_multi_approve(s: u32, z: u32, ) -> Weight {
		(48_402_000 as Weight)
			.saturating_add((132_000 as Weight).saturating_mul(s as Weight))
			.saturating_add((1_000 as Weight).saturating_mul(z as Weight))
			.saturating_add(DbWeight::get().reads(1 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn as_multi_approve_store(s: u32, z: u32, ) -> Weight {
		(88_390_000 as Weight)
			.saturating_add((120_000 as Weight).saturating_mul(s as Weight))
			.saturating_add((3_000 as Weight).saturating_mul(z as Weight))
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
	fn as_multi_complete(s: u32, z: u32, ) -> Weight {
		(98_960_000 as Weight)
			.saturating_add((276_000 as Weight).saturating_mul(s as Weight))
			.saturating_add((6_000 as Weight).saturating_mul(z as Weight))
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn approve_as_multi_create(s: u32, ) -> Weight {
		(80_185_000 as Weight)
			.saturating_add((121_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn approve_as_multi_approve(s: u32, ) -> Weight {
		(48_386_000 as Weight)
			.saturating_add((143_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(DbWeight::get().reads(1 as Weight))
			.saturating_add(DbWeight::get().writes(1 as Weight))
	}
	fn approve_as_multi_complete(s: u32, ) -> Weight {
		(177_181_000 as Weight)
			.saturating_add((273_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(DbWeight::get().reads(3 as Weight))
			.saturating_add(DbWeight::get().writes(3 as Weight))
	}
	fn cancel_as_multi(s: u32, ) -> Weight {
		(126_334_000 as Weight)
			.saturating_add((124_000 as Weight).saturating_mul(s as Weight))
			.saturating_add(DbWeight::get().reads(2 as Weight))
			.saturating_add(DbWeight::get().writes(2 as Weight))
	}
}

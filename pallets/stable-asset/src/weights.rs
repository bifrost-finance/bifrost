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

//! Autogenerated weights for bifrost_stable_asset
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-08-10, STEPS: `[50, ]`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// target/release/node
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=*
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --template=./templates/runtime-weight-template.hbs
// --output=./runtime/src/weights/


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;
use crate::WeightInfo;

#[allow(clippy::unnecessary_cast)]
impl WeightInfo for () {
	fn create_pool() -> Weight {
		Weight::from_parts(33_115_000 as u64, 0)
			.saturating_add(RocksDbWeight::get().reads(3 as u64))
			.saturating_add(RocksDbWeight::get().writes(3 as u64))
	}
	fn modify_a() -> Weight {
		Weight::from_parts(21_186_000 as u64, 0)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	fn modify_fees() -> Weight {
		Weight::from_parts(21_186_000 as u64, 0)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	fn modify_recipients() -> Weight {
		Weight::from_parts(21_186_000 as u64, 0)
			.saturating_add(RocksDbWeight::get().reads(1 as u64))
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}
	fn mint(u: u32) -> Weight {
		Weight::from_parts(85_694_000 as u64, 0)
			.saturating_add(Weight::from_parts(46_172_000 as u64, 0).saturating_mul(u as u64))
			.saturating_add(RocksDbWeight::get().reads(6 as u64))
			.saturating_add(RocksDbWeight::get().reads((3 as u64).saturating_mul(u as u64)))
			.saturating_add(RocksDbWeight::get().writes(6 as u64))
			.saturating_add(RocksDbWeight::get().writes((3 as u64).saturating_mul(u as u64)))
	}
	fn swap(u: u32) -> Weight {
		Weight::from_parts(124_402_000 as u64, 0)
			.saturating_add(Weight::from_parts(8_138_000 as u64, 0).saturating_mul(u as u64))
			.saturating_add(RocksDbWeight::get().reads(7 as u64))
			.saturating_add(RocksDbWeight::get().reads((1 as u64).saturating_mul(u as u64)))
			.saturating_add(RocksDbWeight::get().writes(9 as u64))
	}
	fn redeem_proportion(u: u32) -> Weight {
		Weight::from_parts(107_494_000 as u64, 0)
			.saturating_add(Weight::from_parts(43_376_000 as u64, 0).saturating_mul(u as u64))
			.saturating_add(RocksDbWeight::get().reads(4 as u64))
			.saturating_add(RocksDbWeight::get().reads((3 as u64).saturating_mul(u as u64)))
			.saturating_add(RocksDbWeight::get().writes(4 as u64))
			.saturating_add(RocksDbWeight::get().writes((3 as u64).saturating_mul(u as u64)))
	}
	fn redeem_single(u: u32) -> Weight {
		Weight::from_parts(114_847_000 as u64, 0)
			.saturating_add(Weight::from_parts(14_613_000 as u64, 0).saturating_mul(u as u64))
			.saturating_add(RocksDbWeight::get().reads(6 as u64))
			.saturating_add(RocksDbWeight::get().reads((1 as u64).saturating_mul(u as u64)))
			.saturating_add(RocksDbWeight::get().writes(7 as u64))
	}
	fn redeem_multi(u: u32) -> Weight {
		Weight::from_parts(86_888_000 as u64, 0)
			.saturating_add(Weight::from_parts(43_556_000 as u64, 0).saturating_mul(u as u64))
			.saturating_add(RocksDbWeight::get().reads(4 as u64))
			.saturating_add(RocksDbWeight::get().reads((3 as u64).saturating_mul(u as u64)))
			.saturating_add(RocksDbWeight::get().writes(4 as u64))
			.saturating_add(RocksDbWeight::get().writes((3 as u64).saturating_mul(u as u64)))
	}
}

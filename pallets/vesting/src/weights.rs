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

//! Weights for pallet_vesting
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0
//! DATE: 2020-10-27, STEPS: `[50, ]`, REPEAT: 20, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// target/release/substrate
// benchmark
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_vesting
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./frame/vesting/src/weights.rs
// --template=./.maintain/frame-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_vesting.
pub trait WeightInfo {
	fn vest_locked(l: u32) -> Weight;
	fn vest_unlocked(l: u32) -> Weight;
	fn vest_other_locked(l: u32) -> Weight;
	fn vest_other_unlocked(l: u32) -> Weight;
	fn vested_transfer(l: u32) -> Weight;
	fn force_vested_transfer(l: u32) -> Weight;
}

/// Weights for pallet_vesting using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn vest_locked(l: u32) -> Weight {
		(57_472_000 as Weight)
			.saturating_add((155_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}

	fn vest_unlocked(l: u32) -> Weight {
		(61_681_000 as Weight)
			.saturating_add((138_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}

	fn vest_other_locked(l: u32) -> Weight {
		(56_910_000 as Weight)
			.saturating_add((160_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}

	fn vest_other_unlocked(l: u32) -> Weight {
		(61_319_000 as Weight)
			.saturating_add((144_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}

	fn vested_transfer(l: u32) -> Weight {
		(124_996_000 as Weight)
			.saturating_add((209_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}

	fn force_vested_transfer(l: u32) -> Weight {
		(123_911_000 as Weight)
			.saturating_add((213_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn vest_locked(l: u32) -> Weight {
		(57_472_000 as Weight)
			.saturating_add((155_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}

	fn vest_unlocked(l: u32) -> Weight {
		(61_681_000 as Weight)
			.saturating_add((138_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}

	fn vest_other_locked(l: u32) -> Weight {
		(56_910_000 as Weight)
			.saturating_add((160_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}

	fn vest_other_unlocked(l: u32) -> Weight {
		(61_319_000 as Weight)
			.saturating_add((144_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}

	fn vested_transfer(l: u32) -> Weight {
		(124_996_000 as Weight)
			.saturating_add((209_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}

	fn force_vested_transfer(l: u32) -> Weight {
		(123_911_000 as Weight)
			.saturating_add((213_000 as Weight).saturating_mul(l as Weight))
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
	}
}

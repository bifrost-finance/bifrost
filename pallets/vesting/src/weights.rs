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
	fn vest_locked(l: u32, s: u32) -> Weight;
	fn vest_unlocked(l: u32, s: u32) -> Weight;
	fn vest_other_locked(l: u32, s: u32) -> Weight;
	fn vest_other_unlocked(l: u32, s: u32) -> Weight;
	fn vested_transfer(l: u32, s: u32) -> Weight;
	fn force_vested_transfer(l: u32, s: u32) -> Weight;
}

/// Weights for pallet_vesting using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[1, 28]`.
	fn vest_locked(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `381 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `9286`
		// Minimum execution time: 31_657_000 picoseconds.
		Weight::from_parts(30_569_947, 9286)
			// Standard Error: 794
			.saturating_add(Weight::from_parts(63_114, 0).saturating_mul(l.into()))
			// Standard Error: 1_413
			.saturating_add(Weight::from_parts(58_636, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[1, 28]`.
	fn vest_unlocked(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `381 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `9286`
		// Minimum execution time: 30_474_000 picoseconds.
		Weight::from_parts(30_227_344, 9286)
			// Standard Error: 1_005
			.saturating_add(Weight::from_parts(56_742, 0).saturating_mul(l.into()))
			// Standard Error: 1_788
			.saturating_add(Weight::from_parts(33_890, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[1, 28]`.
	fn vest_other_locked(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `484 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `12879`
		// Minimum execution time: 33_681_000 picoseconds.
		Weight::from_parts(32_540_534, 12879)
			// Standard Error: 2_642
			.saturating_add(Weight::from_parts(62_200, 0).saturating_mul(l.into()))
			// Standard Error: 4_701
			.saturating_add(Weight::from_parts(69_703, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[1, 28]`.
	fn vest_other_unlocked(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `484 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `12879`
		// Minimum execution time: 32_255_000 picoseconds.
		Weight::from_parts(31_637_918, 12879)
			// Standard Error: 3_135
			.saturating_add(Weight::from_parts(62_121, 0).saturating_mul(l.into()))
			// Standard Error: 5_579
			.saturating_add(Weight::from_parts(61_055, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[0, 27]`.
	fn vested_transfer(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `555 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `12879`
		// Minimum execution time: 51_697_000 picoseconds.
		Weight::from_parts(52_048_055, 12879)
			// Standard Error: 1_598
			.saturating_add(Weight::from_parts(60_508, 0).saturating_mul(l.into()))
			// Standard Error: 2_843
			.saturating_add(Weight::from_parts(37_870, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: System Account (r:2 w:2)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[0, 27]`.
	fn force_vested_transfer(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `658 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `15482`
		// Minimum execution time: 54_585_000 picoseconds.
		Weight::from_parts(54_492_070, 15482)
			// Standard Error: 1_694
			.saturating_add(Weight::from_parts(52_633, 0).saturating_mul(l.into()))
			// Standard Error: 3_014
			.saturating_add(Weight::from_parts(45_485, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[1, 28]`.
	fn vest_locked(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `381 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `9286`
		// Minimum execution time: 31_657_000 picoseconds.
		Weight::from_parts(30_569_947, 9286)
			// Standard Error: 794
			.saturating_add(Weight::from_parts(63_114, 0).saturating_mul(l.into()))
			// Standard Error: 1_413
			.saturating_add(Weight::from_parts(58_636, 0).saturating_mul(s.into()))
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[1, 28]`.
	fn vest_unlocked(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `381 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `9286`
		// Minimum execution time: 30_474_000 picoseconds.
		Weight::from_parts(30_227_344, 9286)
			// Standard Error: 1_005
			.saturating_add(Weight::from_parts(56_742, 0).saturating_mul(l.into()))
			// Standard Error: 1_788
			.saturating_add(Weight::from_parts(33_890, 0).saturating_mul(s.into()))
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[1, 28]`.
	fn vest_other_locked(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `484 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `12879`
		// Minimum execution time: 33_681_000 picoseconds.
		Weight::from_parts(32_540_534, 12879)
			// Standard Error: 2_642
			.saturating_add(Weight::from_parts(62_200, 0).saturating_mul(l.into()))
			// Standard Error: 4_701
			.saturating_add(Weight::from_parts(69_703, 0).saturating_mul(s.into()))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[1, 28]`.
	fn vest_other_unlocked(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `484 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `12879`
		// Minimum execution time: 32_255_000 picoseconds.
		Weight::from_parts(31_637_918, 12879)
			// Standard Error: 3_135
			.saturating_add(Weight::from_parts(62_121, 0).saturating_mul(l.into()))
			// Standard Error: 5_579
			.saturating_add(Weight::from_parts(61_055, 0).saturating_mul(s.into()))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[0, 27]`.
	fn vested_transfer(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `555 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `12879`
		// Minimum execution time: 51_697_000 picoseconds.
		Weight::from_parts(52_048_055, 12879)
			// Standard Error: 1_598
			.saturating_add(Weight::from_parts(60_508, 0).saturating_mul(l.into()))
			// Standard Error: 2_843
			.saturating_add(Weight::from_parts(37_870, 0).saturating_mul(s.into()))
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	/// Storage: Vesting Vesting (r:1 w:1)
	/// Proof: Vesting Vesting (max_values: None, max_size: Some(1057), added: 3532, mode:
	/// MaxEncodedLen) Storage: System Account (r:2 w:2)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode:
	/// MaxEncodedLen) Storage: Balances Locks (r:1 w:1)
	/// Proof: Balances Locks (max_values: None, max_size: Some(1299), added: 3774, mode:
	/// MaxEncodedLen) The range of component `l` is `[0, 49]`.
	/// The range of component `s` is `[0, 27]`.
	fn force_vested_transfer(l: u32, s: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `658 + l * (25 ±0) + s * (36 ±0)`
		//  Estimated: `15482`
		// Minimum execution time: 54_585_000 picoseconds.
		Weight::from_parts(54_492_070, 15482)
			// Standard Error: 1_694
			.saturating_add(Weight::from_parts(52_633, 0).saturating_mul(l.into()))
			// Standard Error: 3_014
			.saturating_add(Weight::from_parts(45_485, 0).saturating_mul(s.into()))
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
}

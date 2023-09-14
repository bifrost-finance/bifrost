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
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for bifrost_system_staking
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-14, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `MacBook-Pro-2`, CPU: `<UNKNOWN>`
//! WASM-EXECUTION: Compiled, CHAIN: Some("bifrost-kusama-local"), DB CACHE: 1024

// Executed Command:
// target/release/bifrost
// benchmark
// pallet
// --chain=bifrost-kusama-local
// --steps=50
// --repeat=20
// --pallet=bifrost_system_staking
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./pallets/system-staking/src/weights.rs
// --template=./weight-template/pallet-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for bifrost_system_staking.
pub trait WeightInfo {
	fn on_initialize() -> Weight;
	fn token_config() -> Weight;
	fn refresh_token_info() -> Weight;
	fn payout() -> Weight;
	fn on_redeem_success() -> Weight;
	fn on_redeemed() -> Weight;
	fn delete_token() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: SystemStaking TokenList (r:1 w:0)
	/// Proof Skipped: SystemStaking TokenList (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: SystemStaking Round (r:1 w:0)
	/// Proof Skipped: SystemStaking Round (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: SystemStaking TokenStatus (r:2 w:0)
	/// Proof Skipped: SystemStaking TokenStatus (max_values: None, max_size: None, mode: Measured)
	fn on_initialize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `445`
		//  Estimated: `6385`
		// Minimum execution time: 12_000_000 picoseconds.
		Weight::from_parts(12_000_000, 6385)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
	}
	/// Storage: SystemStaking TokenStatus (r:1 w:1)
	/// Proof Skipped: SystemStaking TokenStatus (max_values: None, max_size: None, mode: Measured)
	/// Storage: SystemStaking TokenList (r:1 w:1)
	/// Proof Skipped: SystemStaking TokenList (max_values: Some(1), max_size: None, mode: Measured)
	fn token_config() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `109`
		//  Estimated: `3574`
		// Minimum execution time: 16_000_000 picoseconds.
		Weight::from_parts(16_000_000, 3574)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	/// Storage: SystemStaking TokenStatus (r:1 w:1)
	/// Proof Skipped: SystemStaking TokenStatus (max_values: None, max_size: None, mode: Measured)
	/// Storage: Farming PoolInfos (r:1 w:0)
	/// Proof Skipped: Farming PoolInfos (max_values: None, max_size: None, mode: Measured)
	fn refresh_token_info() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `403`
		//  Estimated: `3868`
		// Minimum execution time: 20_000_000 picoseconds.
		Weight::from_parts(21_000_000, 3868)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: SystemStaking TokenStatus (r:1 w:0)
	/// Proof Skipped: SystemStaking TokenStatus (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:1 w:0)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TokenPool (r:1 w:0)
	/// Proof: VtokenMinting TokenPool (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:0)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	fn payout() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1190`
		//  Estimated: `4655`
		// Minimum execution time: 28_000_000 picoseconds.
		Weight::from_parts(28_000_000, 4655)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
	}
	fn on_redeem_success() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 0_000 picoseconds.
		Weight::from_parts(1_000_000, 0)
	}
	fn on_redeemed() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 0_000 picoseconds.
		Weight::from_parts(1_000_000, 0)
	}
	/// Storage: SystemStaking TokenList (r:1 w:1)
	/// Proof Skipped: SystemStaking TokenList (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: SystemStaking TokenStatus (r:0 w:1)
	/// Proof Skipped: SystemStaking TokenStatus (max_values: None, max_size: None, mode: Measured)
	fn delete_token() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `169`
		//  Estimated: `1654`
		// Minimum execution time: 8_000_000 picoseconds.
		Weight::from_parts(9_000_000, 1654)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
}

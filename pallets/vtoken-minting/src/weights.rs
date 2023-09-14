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

//! Autogenerated weights for bifrost_vtoken_minting
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
// --pallet=bifrost_vtoken_minting
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./pallets/vtoken-minting/src/weights.rs
// --template=./weight-template/pallet-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for bifrost_vtoken_minting.
pub trait WeightInfo {
	fn set_minimum_mint() -> Weight;
	fn set_minimum_redeem() -> Weight;
	fn set_unlock_duration() -> Weight;
	fn add_support_rebond_token() -> Weight;
	fn remove_support_rebond_token() -> Weight;
	fn set_fees() -> Weight;
	fn set_hook_iteration_limit() -> Weight;
	fn mint() -> Weight;
	fn redeem() -> Weight;
	fn rebond() -> Weight;
	fn rebond_by_unlock_id() -> Weight;
	fn on_initialize() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: VtokenMinting MinimumMint (r:1 w:1)
	/// Proof: VtokenMinting MinimumMint (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	fn set_minimum_mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `732`
		//  Estimated: `4197`
		// Minimum execution time: 17_000_000 picoseconds.
		Weight::from_parts(18_000_000, 4197)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: VtokenMinting MinimumRedeem (r:1 w:1)
	/// Proof: VtokenMinting MinimumRedeem (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	fn set_minimum_redeem() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `180`
		//  Estimated: `3503`
		// Minimum execution time: 11_000_000 picoseconds.
		Weight::from_parts(11_000_000, 3503)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: VtokenMinting UnlockDuration (r:1 w:1)
	/// Proof: VtokenMinting UnlockDuration (max_values: None, max_size: Some(27), added: 2502, mode: MaxEncodedLen)
	fn set_unlock_duration() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `180`
		//  Estimated: `3492`
		// Minimum execution time: 11_000_000 picoseconds.
		Weight::from_parts(11_000_000, 3492)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: VtokenMinting TokenToRebond (r:1 w:1)
	/// Proof: VtokenMinting TokenToRebond (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	fn add_support_rebond_token() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `180`
		//  Estimated: `3503`
		// Minimum execution time: 11_000_000 picoseconds.
		Weight::from_parts(12_000_000, 3503)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: VtokenMinting TokenToRebond (r:1 w:1)
	/// Proof: VtokenMinting TokenToRebond (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	fn remove_support_rebond_token() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `230`
		//  Estimated: `3503`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 3503)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: VtokenMinting Fees (r:1 w:1)
	/// Proof: VtokenMinting Fees (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	fn set_fees() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `180`
		//  Estimated: `1493`
		// Minimum execution time: 11_000_000 picoseconds.
		Weight::from_parts(11_000_000, 1493)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: VtokenMinting HookIterationLimit (r:1 w:1)
	/// Proof: VtokenMinting HookIterationLimit (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	fn set_hook_iteration_limit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `180`
		//  Estimated: `1489`
		// Minimum execution time: 10_000_000 picoseconds.
		Weight::from_parts(11_000_000, 1489)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: VtokenMinting MinimumMint (r:1 w:0)
	/// Proof: VtokenMinting MinimumMint (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TokenPool (r:1 w:1)
	/// Proof: VtokenMinting TokenPool (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:1)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: VtokenMinting Fees (r:1 w:0)
	/// Proof: VtokenMinting Fees (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:3 w:3)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn mint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2041`
		//  Estimated: `8769`
		// Minimum execution time: 90_000_000 picoseconds.
		Weight::from_parts(92_000_000, 8769)
			.saturating_add(RocksDbWeight::get().reads(9_u64))
			.saturating_add(RocksDbWeight::get().writes(6_u64))
	}
	/// Storage: VtokenMinting MinimumRedeem (r:1 w:0)
	/// Proof: VtokenMinting MinimumRedeem (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Slp DelegationsOccupied (r:1 w:0)
	/// Proof Skipped: Slp DelegationsOccupied (max_values: None, max_size: None, mode: Measured)
	/// Storage: VtokenMinting Fees (r:1 w:0)
	/// Proof: VtokenMinting Fees (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TokenPool (r:1 w:1)
	/// Proof: VtokenMinting TokenPool (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:1)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: VtokenMinting OngoingTimeUnit (r:1 w:0)
	/// Proof: VtokenMinting OngoingTimeUnit (max_values: None, max_size: Some(27), added: 2502, mode: MaxEncodedLen)
	/// Storage: VtokenMinting UnlockDuration (r:1 w:0)
	/// Proof: VtokenMinting UnlockDuration (max_values: None, max_size: Some(27), added: 2502, mode: MaxEncodedLen)
	/// Storage: VtokenMinting UnlockingTotal (r:1 w:1)
	/// Proof: VtokenMinting UnlockingTotal (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TokenUnlockNextId (r:1 w:1)
	/// Proof: VtokenMinting TokenUnlockNextId (max_values: None, max_size: Some(26), added: 2501, mode: MaxEncodedLen)
	/// Storage: VtokenMinting UserUnlockLedger (r:1 w:1)
	/// Proof: VtokenMinting UserUnlockLedger (max_values: None, max_size: Some(135), added: 2610, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TimeUnitUnlockLedger (r:1 w:1)
	/// Proof: VtokenMinting TimeUnitUnlockLedger (max_values: None, max_size: Some(282), added: 2757, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TokenUnlockLedger (r:0 w:1)
	/// Proof: VtokenMinting TokenUnlockLedger (max_values: None, max_size: Some(136), added: 2611, mode: MaxEncodedLen)
	fn redeem() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2187`
		//  Estimated: `6176`
		// Minimum execution time: 108_000_000 picoseconds.
		Weight::from_parts(109_000_000, 6176)
			.saturating_add(RocksDbWeight::get().reads(15_u64))
			.saturating_add(RocksDbWeight::get().writes(10_u64))
	}
	/// Storage: VtokenMinting TokenToRebond (r:1 w:1)
	/// Proof: VtokenMinting TokenToRebond (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: VtokenMinting UserUnlockLedger (r:1 w:1)
	/// Proof: VtokenMinting UserUnlockLedger (max_values: None, max_size: Some(135), added: 2610, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TokenUnlockLedger (r:1 w:1)
	/// Proof: VtokenMinting TokenUnlockLedger (max_values: None, max_size: Some(136), added: 2611, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TimeUnitUnlockLedger (r:1 w:1)
	/// Proof: VtokenMinting TimeUnitUnlockLedger (max_values: None, max_size: Some(282), added: 2757, mode: MaxEncodedLen)
	/// Storage: VtokenMinting UnlockingTotal (r:1 w:1)
	/// Proof: VtokenMinting UnlockingTotal (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TokenPool (r:1 w:1)
	/// Proof: VtokenMinting TokenPool (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:1)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: VtokenMinting Fees (r:1 w:0)
	/// Proof: VtokenMinting Fees (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:3 w:3)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	fn rebond() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2620`
		//  Estimated: `8769`
		// Minimum execution time: 101_000_000 picoseconds.
		Weight::from_parts(103_000_000, 8769)
			.saturating_add(RocksDbWeight::get().reads(12_u64))
			.saturating_add(RocksDbWeight::get().writes(10_u64))
	}
	/// Storage: VtokenMinting TokenToRebond (r:1 w:1)
	/// Proof: VtokenMinting TokenToRebond (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TokenUnlockLedger (r:1 w:1)
	/// Proof: VtokenMinting TokenUnlockLedger (max_values: None, max_size: Some(136), added: 2611, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TimeUnitUnlockLedger (r:1 w:1)
	/// Proof: VtokenMinting TimeUnitUnlockLedger (max_values: None, max_size: Some(282), added: 2757, mode: MaxEncodedLen)
	/// Storage: VtokenMinting UserUnlockLedger (r:1 w:1)
	/// Proof: VtokenMinting UserUnlockLedger (max_values: None, max_size: Some(135), added: 2610, mode: MaxEncodedLen)
	/// Storage: VtokenMinting UnlockingTotal (r:1 w:1)
	/// Proof: VtokenMinting UnlockingTotal (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: VtokenMinting TokenPool (r:1 w:1)
	/// Proof: VtokenMinting TokenPool (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: Tokens TotalIssuance (r:1 w:1)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	/// Storage: VtokenMinting Fees (r:1 w:0)
	/// Proof: VtokenMinting Fees (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
	/// Storage: Tokens Accounts (r:3 w:3)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	fn rebond_by_unlock_id() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2620`
		//  Estimated: `8769`
		// Minimum execution time: 97_000_000 picoseconds.
		Weight::from_parts(98_000_000, 8769)
			.saturating_add(RocksDbWeight::get().reads(12_u64))
			.saturating_add(RocksDbWeight::get().writes(10_u64))
	}
	/// Storage: VtokenMinting OngoingTimeUnit (r:1 w:0)
	/// Proof: VtokenMinting OngoingTimeUnit (max_values: None, max_size: Some(27), added: 2502, mode: MaxEncodedLen)
	fn on_initialize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `211`
		//  Estimated: `3492`
		// Minimum execution time: 4_000_000 picoseconds.
		Weight::from_parts(5_000_000, 3492)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
	}
}

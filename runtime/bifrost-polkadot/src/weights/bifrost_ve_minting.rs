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
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for bifrost_ve_minting
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-14, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `bifrost-jenkins`, CPU: `Intel(R) Xeon(R) CPU E5-26xx v4`
//! WASM-EXECUTION: Compiled, CHAIN: Some("bifrost-polkadot-local"), DB CACHE: 1024

// Executed Command:
// target/release/bifrost
// benchmark
// pallet
// --chain=bifrost-polkadot-local
// --steps=50
// --repeat=20
// --pallet=bifrost_ve_minting
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./runtime/bifrost-polkadot/src/weights/bifrost_ve_minting.rs
// --template=./weight-template/runtime-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions for bifrost_ve_minting.
pub struct BifrostWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> bifrost_ve_minting::WeightInfo for BifrostWeight<T> {
	// Storage: VeMinting VeConfigs (r:1 w:1)
	// Proof Skipped: VeMinting VeConfigs (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: System Number (r:1 w:0)
	// Proof: System Number (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Proof: System ExecutionPhase (max_values: Some(1), max_size: Some(5), added: 500, mode: MaxEncodedLen)
	// Storage: System EventCount (r:1 w:1)
	// Proof: System EventCount (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System Events (r:1 w:1)
	// Proof Skipped: System Events (max_values: Some(1), max_size: None, mode: Measured)
	fn set_config() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `200`
		//  Estimated: `1685`
		// Minimum execution time: 33_022 nanoseconds.
		Weight::from_parts(33_924_000, 1685)
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	// Storage: VeMinting VeConfigs (r:1 w:0)
	// Proof Skipped: VeMinting VeConfigs (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting Locked (r:1 w:1)
	// Proof Skipped: VeMinting Locked (max_values: None, max_size: None, mode: Measured)
	// Storage: System Number (r:1 w:0)
	// Proof: System Number (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: VeMinting Supply (r:1 w:1)
	// Proof Skipped: VeMinting Supply (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: Tokens Accounts (r:2 w:2)
	// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	// Storage: System Account (r:2 w:1)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Proof: System ExecutionPhase (max_values: Some(1), max_size: Some(5), added: 500, mode: MaxEncodedLen)
	// Storage: System EventCount (r:1 w:1)
	// Proof: System EventCount (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System Events (r:1 w:1)
	// Proof Skipped: System Events (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting IncentiveConfigs (r:1 w:1)
	// Proof Skipped: VeMinting IncentiveConfigs (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting Epoch (r:1 w:1)
	// Proof Skipped: VeMinting Epoch (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting PointHistory (r:1 w:1)
	// Proof Skipped: VeMinting PointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserPointEpoch (r:1 w:1)
	// Proof Skipped: VeMinting UserPointEpoch (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting Rewards (r:1 w:0)
	// Proof Skipped: VeMinting Rewards (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting SlopeChanges (r:2 w:1)
	// Proof Skipped: VeMinting SlopeChanges (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserPointHistory (r:0 w:1)
	// Proof Skipped: VeMinting UserPointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserRewardPerTokenPaid (r:0 w:1)
	// Proof Skipped: VeMinting UserRewardPerTokenPaid (max_values: None, max_size: None, mode: Measured)
	fn create_lock() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1439`
		//  Estimated: `7379`
		// Minimum execution time: 257_228 nanoseconds.
		Weight::from_parts(260_165_000, 7379)
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().writes(14))
	}
	// Storage: VeMinting VeConfigs (r:1 w:0)
	// Proof Skipped: VeMinting VeConfigs (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting Locked (r:1 w:1)
	// Proof Skipped: VeMinting Locked (max_values: None, max_size: None, mode: Measured)
	// Storage: System Number (r:1 w:0)
	// Proof: System Number (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: VeMinting Supply (r:1 w:1)
	// Proof Skipped: VeMinting Supply (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: Tokens Accounts (r:2 w:2)
	// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	// Storage: System Account (r:1 w:0)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Proof: System ExecutionPhase (max_values: Some(1), max_size: Some(5), added: 500, mode: MaxEncodedLen)
	// Storage: System EventCount (r:1 w:1)
	// Proof: System EventCount (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System Events (r:1 w:1)
	// Proof Skipped: System Events (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting IncentiveConfigs (r:1 w:1)
	// Proof Skipped: VeMinting IncentiveConfigs (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting Epoch (r:1 w:1)
	// Proof Skipped: VeMinting Epoch (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting PointHistory (r:1 w:1)
	// Proof Skipped: VeMinting PointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserPointEpoch (r:1 w:1)
	// Proof Skipped: VeMinting UserPointEpoch (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserPointHistory (r:1 w:1)
	// Proof Skipped: VeMinting UserPointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting Rewards (r:1 w:1)
	// Proof Skipped: VeMinting Rewards (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserRewardPerTokenPaid (r:1 w:1)
	// Proof Skipped: VeMinting UserRewardPerTokenPaid (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting SlopeChanges (r:1 w:1)
	// Proof Skipped: VeMinting SlopeChanges (max_values: None, max_size: None, mode: Measured)
	fn increase_amount() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2083`
		//  Estimated: `6176`
		// Minimum execution time: 282_580 nanoseconds.
		Weight::from_parts(291_800_000, 6176)
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().writes(14))
	}
	// Storage: VeMinting VeConfigs (r:1 w:0)
	// Proof Skipped: VeMinting VeConfigs (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting Locked (r:1 w:1)
	// Proof Skipped: VeMinting Locked (max_values: None, max_size: None, mode: Measured)
	// Storage: System Number (r:1 w:0)
	// Proof: System Number (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: VeMinting Supply (r:1 w:1)
	// Proof Skipped: VeMinting Supply (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting IncentiveConfigs (r:1 w:1)
	// Proof Skipped: VeMinting IncentiveConfigs (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting Epoch (r:1 w:1)
	// Proof Skipped: VeMinting Epoch (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting PointHistory (r:1 w:1)
	// Proof Skipped: VeMinting PointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserPointEpoch (r:1 w:1)
	// Proof Skipped: VeMinting UserPointEpoch (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserPointHistory (r:1 w:1)
	// Proof Skipped: VeMinting UserPointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting Rewards (r:1 w:1)
	// Proof Skipped: VeMinting Rewards (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserRewardPerTokenPaid (r:1 w:1)
	// Proof Skipped: VeMinting UserRewardPerTokenPaid (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting SlopeChanges (r:2 w:2)
	// Proof Skipped: VeMinting SlopeChanges (max_values: None, max_size: None, mode: Measured)
	// Storage: Tokens Accounts (r:1 w:0)
	// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Proof: System ExecutionPhase (max_values: Some(1), max_size: Some(5), added: 500, mode: MaxEncodedLen)
	// Storage: System EventCount (r:1 w:1)
	// Proof: System EventCount (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System Events (r:1 w:1)
	// Proof Skipped: System Events (max_values: Some(1), max_size: None, mode: Measured)
	fn increase_unlock_time() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1589`
		//  Estimated: `7529`
		// Minimum execution time: 228_588 nanoseconds.
		Weight::from_parts(233_726_000, 7529)
			.saturating_add(T::DbWeight::get().reads(17))
			.saturating_add(T::DbWeight::get().writes(13))
	}
	// Storage: VeMinting Locked (r:1 w:1)
	// Proof Skipped: VeMinting Locked (max_values: None, max_size: None, mode: Measured)
	// Storage: System Number (r:1 w:0)
	// Proof: System Number (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: VeMinting Supply (r:1 w:1)
	// Proof Skipped: VeMinting Supply (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: Tokens Accounts (r:2 w:2)
	// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	// Storage: System Account (r:1 w:1)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Proof: System ExecutionPhase (max_values: Some(1), max_size: Some(5), added: 500, mode: MaxEncodedLen)
	// Storage: System EventCount (r:1 w:1)
	// Proof: System EventCount (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System Events (r:1 w:1)
	// Proof Skipped: System Events (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting IncentiveConfigs (r:1 w:1)
	// Proof Skipped: VeMinting IncentiveConfigs (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting Epoch (r:1 w:1)
	// Proof Skipped: VeMinting Epoch (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting PointHistory (r:1 w:105)
	// Proof Skipped: VeMinting PointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting SlopeChanges (r:104 w:0)
	// Proof Skipped: VeMinting SlopeChanges (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserPointEpoch (r:1 w:1)
	// Proof Skipped: VeMinting UserPointEpoch (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserPointHistory (r:1 w:1)
	// Proof Skipped: VeMinting UserPointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting Rewards (r:1 w:1)
	// Proof Skipped: VeMinting Rewards (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserRewardPerTokenPaid (r:1 w:1)
	// Proof Skipped: VeMinting UserRewardPerTokenPaid (max_values: None, max_size: None, mode: Measured)
	fn withdraw() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2059`
		//  Estimated: `260449`
		// Minimum execution time: 1_301_179 nanoseconds.
		Weight::from_parts(1_319_174_000, 260449)
			.saturating_add(T::DbWeight::get().reads(121))
			.saturating_add(T::DbWeight::get().writes(118))
	}
	// Storage: VeMinting IncentiveConfigs (r:1 w:1)
	// Proof Skipped: VeMinting IncentiveConfigs (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: System Number (r:1 w:0)
	// Proof: System Number (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: VeMinting Epoch (r:1 w:0)
	// Proof Skipped: VeMinting Epoch (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting PointHistory (r:1 w:0)
	// Proof Skipped: VeMinting PointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting SlopeChanges (r:104 w:0)
	// Proof Skipped: VeMinting SlopeChanges (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserPointEpoch (r:1 w:0)
	// Proof Skipped: VeMinting UserPointEpoch (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserPointHistory (r:1 w:0)
	// Proof Skipped: VeMinting UserPointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting Rewards (r:1 w:1)
	// Proof Skipped: VeMinting Rewards (max_values: None, max_size: None, mode: Measured)
	// Storage: VeMinting UserRewardPerTokenPaid (r:1 w:1)
	// Proof Skipped: VeMinting UserRewardPerTokenPaid (max_values: None, max_size: None, mode: Measured)
	// Storage: Balances TotalIssuance (r:1 w:1)
	// Proof: Balances TotalIssuance (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	// Storage: System Account (r:3 w:3)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Proof: System ExecutionPhase (max_values: Some(1), max_size: Some(5), added: 500, mode: MaxEncodedLen)
	// Storage: System EventCount (r:1 w:1)
	// Proof: System EventCount (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System Events (r:1 w:1)
	// Proof Skipped: System Events (max_values: Some(1), max_size: None, mode: Measured)
	fn get_rewards() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1384`
		//  Estimated: `259774`
		// Minimum execution time: 754_097 nanoseconds.
		Weight::from_parts(761_819_000, 259774)
			.saturating_add(T::DbWeight::get().reads(119))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	// Storage: VeMinting IncentiveConfigs (r:1 w:1)
	// Proof Skipped: VeMinting IncentiveConfigs (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: System Number (r:1 w:0)
	// Proof: System Number (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Proof: System ExecutionPhase (max_values: Some(1), max_size: Some(5), added: 500, mode: MaxEncodedLen)
	// Storage: System EventCount (r:1 w:1)
	// Proof: System EventCount (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	// Storage: System Events (r:1 w:1)
	// Proof Skipped: System Events (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting Epoch (r:1 w:0)
	// Proof Skipped: VeMinting Epoch (max_values: Some(1), max_size: None, mode: Measured)
	// Storage: VeMinting PointHistory (r:1 w:0)
	// Proof Skipped: VeMinting PointHistory (max_values: None, max_size: None, mode: Measured)
	// Storage: System Account (r:2 w:2)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	// Storage: Balances TotalIssuance (r:1 w:0)
	// Proof: Balances TotalIssuance (max_values: Some(1), max_size: Some(16), added: 511, mode: MaxEncodedLen)
	fn notify_rewards() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `508`
		//  Estimated: `6196`
		// Minimum execution time: 176_065 nanoseconds.
		Weight::from_parts(177_901_000, 6196)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(5))
	}
}

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

//! Autogenerated weights for bifrost_cross_in_out
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-14, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `bifrost-jenkins`, CPU: `Intel(R) Xeon(R) CPU E5-26xx v4`
//! WASM-EXECUTION: Compiled, CHAIN: Some("bifrost-kusama-local"), DB CACHE: 1024

// Executed Command:
// target/release/bifrost
// benchmark
// pallet
// --chain=bifrost-kusama-local
// --steps=50
// --repeat=20
// --pallet=bifrost_cross_in_out
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./runtime/bifrost-kusama/src/weights/bifrost_cross_in_out.rs
// --template=./weight-template/runtime-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions for bifrost_cross_in_out.
pub struct BifrostWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> bifrost_cross_in_out::WeightInfo for BifrostWeight<T> {
	// Storage: CrossInOut CrossCurrencyRegistry (r:1 w:1)
	// Proof Skipped: CrossInOut CrossCurrencyRegistry (max_values: None, max_size: None, mode: Measured)
	fn register_currency_for_cross_in_out() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76`
		//  Estimated: `3541`
		// Minimum execution time: 30_069 nanoseconds.
		Weight::from_parts(30_714_000, 3541)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: CrossInOut CrossCurrencyRegistry (r:1 w:1)
	// Proof Skipped: CrossInOut CrossCurrencyRegistry (max_values: None, max_size: None, mode: Measured)
	fn deregister_currency_for_cross_in_out() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `119`
		//  Estimated: `3584`
		// Minimum execution time: 32_781 nanoseconds.
		Weight::from_parts(33_720_000, 3584)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: CrossInOut CrossingMinimumAmount (r:0 w:1)
	// Proof Skipped: CrossInOut CrossingMinimumAmount (max_values: None, max_size: None, mode: Measured)
	fn set_crossing_minimum_amount() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 24_811 nanoseconds.
		Weight::from_parts(25_172_000, 0)
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: CrossInOut IssueWhiteList (r:1 w:1)
	// Proof Skipped: CrossInOut IssueWhiteList (max_values: None, max_size: None, mode: Measured)
	fn add_to_issue_whitelist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76`
		//  Estimated: `3541`
		// Minimum execution time: 34_593 nanoseconds.
		Weight::from_parts(35_760_000, 3541)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: CrossInOut IssueWhiteList (r:1 w:1)
	// Proof Skipped: CrossInOut IssueWhiteList (max_values: None, max_size: None, mode: Measured)
	fn remove_from_issue_whitelist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `154`
		//  Estimated: `3619`
		// Minimum execution time: 35_748 nanoseconds.
		Weight::from_parts(36_569_000, 3619)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: CrossInOut RegisterWhiteList (r:1 w:1)
	// Proof Skipped: CrossInOut RegisterWhiteList (max_values: None, max_size: None, mode: Measured)
	fn add_to_register_whitelist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76`
		//  Estimated: `3541`
		// Minimum execution time: 38_561 nanoseconds.
		Weight::from_parts(39_371_000, 3541)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: CrossInOut RegisterWhiteList (r:1 w:1)
	// Proof Skipped: CrossInOut RegisterWhiteList (max_values: None, max_size: None, mode: Measured)
	fn remove_from_register_whitelist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `154`
		//  Estimated: `3619`
		// Minimum execution time: 35_886 nanoseconds.
		Weight::from_parts(36_589_000, 3619)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	// Storage: CrossInOut CrossCurrencyRegistry (r:1 w:0)
	// Proof Skipped: CrossInOut CrossCurrencyRegistry (max_values: None, max_size: None, mode: Measured)
	// Storage: CrossInOut CrossingMinimumAmount (r:1 w:0)
	// Proof Skipped: CrossInOut CrossingMinimumAmount (max_values: None, max_size: None, mode: Measured)
	// Storage: CrossInOut IssueWhiteList (r:1 w:0)
	// Proof Skipped: CrossInOut IssueWhiteList (max_values: None, max_size: None, mode: Measured)
	// Storage: Tokens Accounts (r:1 w:1)
	// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	// Storage: Tokens TotalIssuance (r:1 w:1)
	// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	// Storage: System Account (r:1 w:1)
	// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn cross_in() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1758`
		//  Estimated: `5223`
		// Minimum execution time: 146_344 nanoseconds.
		Weight::from_parts(147_835_000, 5223)
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	// Storage: CrossInOut RegisterWhiteList (r:1 w:0)
	// Proof Skipped: CrossInOut RegisterWhiteList (max_values: None, max_size: None, mode: Measured)
	// Storage: CrossInOut CrossCurrencyRegistry (r:1 w:0)
	// Proof Skipped: CrossInOut CrossCurrencyRegistry (max_values: None, max_size: None, mode: Measured)
	// Storage: CrossInOut AccountToOuterMultilocation (r:1 w:1)
	// Proof Skipped: CrossInOut AccountToOuterMultilocation (max_values: None, max_size: None, mode: Measured)
	// Storage: CrossInOut OuterMultilocationToAccount (r:0 w:1)
	// Proof Skipped: CrossInOut OuterMultilocationToAccount (max_values: None, max_size: None, mode: Measured)
	fn register_linked_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `193`
		//  Estimated: `3658`
		// Minimum execution time: 58_096 nanoseconds.
		Weight::from_parts(59_952_000, 3658)
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	// Storage: CrossInOut CrossCurrencyRegistry (r:1 w:0)
	// Proof Skipped: CrossInOut CrossCurrencyRegistry (max_values: None, max_size: None, mode: Measured)
	// Storage: CrossInOut CrossingMinimumAmount (r:1 w:0)
	// Proof Skipped: CrossInOut CrossingMinimumAmount (max_values: None, max_size: None, mode: Measured)
	// Storage: Tokens Accounts (r:1 w:1)
	// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	// Storage: CrossInOut AccountToOuterMultilocation (r:1 w:0)
	// Proof Skipped: CrossInOut AccountToOuterMultilocation (max_values: None, max_size: None, mode: Measured)
	// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	// Storage: Tokens TotalIssuance (r:1 w:1)
	// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	fn cross_out() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1781`
		//  Estimated: `5246`
		// Minimum execution time: 121_799 nanoseconds.
		Weight::from_parts(124_055_000, 5246)
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	// Storage: CrossInOut CrossCurrencyRegistry (r:1 w:0)
	// Proof Skipped: CrossInOut CrossCurrencyRegistry (max_values: None, max_size: None, mode: Measured)
	// Storage: CrossInOut AccountToOuterMultilocation (r:1 w:1)
	// Proof Skipped: CrossInOut AccountToOuterMultilocation (max_values: None, max_size: None, mode: Measured)
	// Storage: CrossInOut OuterMultilocationToAccount (r:0 w:1)
	// Proof Skipped: CrossInOut OuterMultilocationToAccount (max_values: None, max_size: None, mode: Measured)
	fn change_outer_linked_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `312`
		//  Estimated: `3777`
		// Minimum execution time: 56_108 nanoseconds.
		Weight::from_parts(57_171_000, 3777)
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}

	fn set_crossout_fee() -> Weight {
		Weight::from_parts(57_171_000, 3777)
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
}

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

//! Autogenerated weights for bifrost_fee_share
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
// --pallet=bifrost_fee_share
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./pallets/fee-share/src/weights.rs
// --template=./weight-template/pallet-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for bifrost_fee_share.
pub trait WeightInfo {
	fn on_initialize() -> Weight;
	fn create_distribution() -> Weight;
	fn edit_distribution() -> Weight;
	fn set_era_length() -> Weight;
	fn execute_distribute() -> Weight;
	fn delete_distribution() -> Weight;
	fn usd_cumulation() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: FeeShare AutoEra (r:1 w:0)
	/// Proof Skipped: FeeShare AutoEra (max_values: Some(1), max_size: None, mode: Measured)
	fn on_initialize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4`
		//  Estimated: `1489`
		// Minimum execution time: 6_481_000 picoseconds.
		Weight::from_parts(6_774_000, 1489)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
	}
	/// Storage: FeeShare DistributionNextId (r:1 w:1)
	/// Proof Skipped: FeeShare DistributionNextId (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: FeeShare DistributionInfos (r:0 w:1)
	/// Proof Skipped: FeeShare DistributionInfos (max_values: None, max_size: None, mode: Measured)
	fn create_distribution() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4`
		//  Estimated: `1489`
		// Minimum execution time: 37_036_000 picoseconds.
		Weight::from_parts(38_003_000, 1489)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	/// Storage: FeeShare DistributionInfos (r:1 w:1)
	/// Proof Skipped: FeeShare DistributionInfos (max_values: None, max_size: None, mode: Measured)
	fn edit_distribution() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `139`
		//  Estimated: `3604`
		// Minimum execution time: 38_459_000 picoseconds.
		Weight::from_parts(39_181_000, 3604)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: FeeShare AutoEra (r:0 w:1)
	/// Proof Skipped: FeeShare AutoEra (max_values: Some(1), max_size: None, mode: Measured)
	fn set_era_length() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 22_953_000 picoseconds.
		Weight::from_parts(23_565_000, 0)
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: FeeShare DistributionInfos (r:1 w:0)
	/// Proof Skipped: FeeShare DistributionInfos (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:0)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	fn execute_distribute() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1616`
		//  Estimated: `6176`
		// Minimum execution time: 76_615_000 picoseconds.
		Weight::from_parts(78_181_000, 6176)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
	}
	/// Storage: FeeShare DistributionInfos (r:1 w:1)
	/// Proof Skipped: FeeShare DistributionInfos (max_values: None, max_size: None, mode: Measured)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:0)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	fn delete_distribution() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1616`
		//  Estimated: `6176`
		// Minimum execution time: 79_367_000 picoseconds.
		Weight::from_parts(80_687_000, 6176)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: `FeeShare::DistributionInfos` (r:1 w:0)
	/// Proof: `FeeShare::DistributionInfos` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `FeeShare::DollarStandardInfos` (r:0 w:1)
	/// Proof: `FeeShare::DollarStandardInfos` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn usd_cumulation() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `94`
		//  Estimated: `3559`
		// Minimum execution time: 9_077_000 picoseconds.
		Weight::from_parts(9_408_000, 0)
			.saturating_add(Weight::from_parts(0, 3559))
			.saturating_add(RocksDbWeight::get().reads(1))
			.saturating_add(RocksDbWeight::get().writes(1))
	}
}

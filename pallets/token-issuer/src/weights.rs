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

//! Autogenerated weights for bifrost_token_issuer
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
// --pallet=bifrost_token_issuer
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./pallets/token-issuer/src/weights.rs
// --template=./weight-template/pallet-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for bifrost_token_issuer.
pub trait WeightInfo {
	fn add_to_issue_whitelist() -> Weight;
	fn remove_from_issue_whitelist() -> Weight;
	fn add_to_transfer_whitelist() -> Weight;
	fn remove_from_transfer_whitelist() -> Weight;
	fn issue() -> Weight;
	fn transfer() -> Weight;
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: TokenIssuer IssueWhiteList (r:1 w:1)
	/// Proof Skipped: TokenIssuer IssueWhiteList (max_values: None, max_size: None, mode: Measured)
	fn add_to_issue_whitelist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `142`
		//  Estimated: `3607`
		// Minimum execution time: 32_693_000 picoseconds.
		Weight::from_parts(33_651_000, 3607)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: TokenIssuer IssueWhiteList (r:1 w:1)
	/// Proof Skipped: TokenIssuer IssueWhiteList (max_values: None, max_size: None, mode: Measured)
	fn remove_from_issue_whitelist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `220`
		//  Estimated: `3685`
		// Minimum execution time: 34_136_000 picoseconds.
		Weight::from_parts(35_196_000, 3685)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: TokenIssuer TransferWhiteList (r:1 w:1)
	/// Proof Skipped: TokenIssuer TransferWhiteList (max_values: None, max_size: None, mode: Measured)
	fn add_to_transfer_whitelist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `142`
		//  Estimated: `3607`
		// Minimum execution time: 32_485_000 picoseconds.
		Weight::from_parts(33_735_000, 3607)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: TokenIssuer TransferWhiteList (r:1 w:1)
	/// Proof Skipped: TokenIssuer TransferWhiteList (max_values: None, max_size: None, mode: Measured)
	fn remove_from_transfer_whitelist() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `220`
		//  Estimated: `3685`
		// Minimum execution time: 33_573_000 picoseconds.
		Weight::from_parts(34_609_000, 3685)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: TokenIssuer IssueWhiteList (r:1 w:0)
	/// Proof Skipped: TokenIssuer IssueWhiteList (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:1 w:1)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens TotalIssuance (r:1 w:1)
	/// Proof: Tokens TotalIssuance (max_values: None, max_size: Some(38), added: 2513, mode: MaxEncodedLen)
	fn issue() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1870`
		//  Estimated: `5335`
		// Minimum execution time: 99_796_000 picoseconds.
		Weight::from_parts(100_707_000, 5335)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	/// Storage: TokenIssuer TransferWhiteList (r:1 w:0)
	/// Proof Skipped: TokenIssuer TransferWhiteList (max_values: None, max_size: None, mode: Measured)
	/// Storage: Tokens Accounts (r:2 w:2)
	/// Proof: Tokens Accounts (max_values: None, max_size: Some(118), added: 2593, mode: MaxEncodedLen)
	/// Storage: AssetRegistry CurrencyMetadatas (r:1 w:0)
	/// Proof Skipped: AssetRegistry CurrencyMetadatas (max_values: None, max_size: None, mode: Measured)
	fn transfer() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1989`
		//  Estimated: `6176`
		// Minimum execution time: 114_723_000 picoseconds.
		Weight::from_parts(115_788_000, 6176)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
}

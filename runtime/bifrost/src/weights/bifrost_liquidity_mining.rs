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

//! Autogenerated weights for `bifrost_liquidity_mining`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-09-23, STEPS: `50`, REPEAT: 1, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("bifrost-local"), DB CACHE: 128

// Executed Command:
// target/release/bifrost
// benchmark
// --chain=bifrost-local
// --steps=50
// --repeat=1
// --pallet=bifrost-liquidity-mining
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --header=./HEADER-GPL3
// --output=./runtime/bifrost/src/weights/bifrost_liquidity_mining


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for bifrost_liquidity_mining.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> bifrost_liquidity_mining::WeightInfo for WeightInfo<T> {
	// Storage: LiquidityMining ChargedPoolIds (r:1 w:1)
	// Storage: LiquidityMining TotalPoolInfos (r:1 w:1)
	// Storage: Tokens Accounts (r:2 w:2)
	// Storage: System Account (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn charge() -> Weight {
		(132_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(9 as Weight))
			.saturating_add(T::DbWeight::get().writes(7 as Weight))
	}
	// Storage: LiquidityMining TotalPoolInfos (r:1 w:1)
	// Storage: LiquidityMining TotalDepositData (r:1 w:1)
	// Storage: Tokens Accounts (r:2 w:2)
	// Storage: Tokens Locks (r:2 w:2)
	// Storage: System Number (r:1 w:0)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn deposit() -> Weight {
		(88_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(10 as Weight))
			.saturating_add(T::DbWeight::get().writes(8 as Weight))
	}
	// Storage: LiquidityMining TotalPoolInfos (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: LiquidityMining TotalDepositData (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Tokens Accounts (r:4 w:4)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: Tokens Locks (r:2 w:2)
	fn redeem() -> Weight {
		(179_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(13 as Weight))
			.saturating_add(T::DbWeight::get().writes(11 as Weight))
	}
	// Storage: LiquidityMining TotalPoolInfos (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: LiquidityMining TotalDepositData (r:2 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Tokens Accounts (r:4 w:4)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	// Storage: Tokens Locks (r:2 w:2)
	fn volunteer_to_redeem() -> Weight {
		(199_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(14 as Weight))
			.saturating_add(T::DbWeight::get().writes(11 as Weight))
	}
	// Storage: LiquidityMining TotalPoolInfos (r:1 w:1)
	// Storage: System Number (r:1 w:0)
	// Storage: LiquidityMining TotalDepositData (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Tokens Accounts (r:2 w:2)
	// Storage: System ExecutionPhase (r:1 w:0)
	// Storage: System EventCount (r:1 w:1)
	// Storage: System Events (r:1 w:1)
	fn claim() -> Weight {
		(117_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(9 as Weight))
			.saturating_add(T::DbWeight::get().writes(7 as Weight))
	}
}

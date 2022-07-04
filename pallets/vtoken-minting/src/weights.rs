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

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
	traits::Get,
	weights::{constants::RocksDbWeight, Weight},
};
use sp_std::marker::PhantomData;

/// Weight functions needed for the pallet.
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
	// Storage: VtokenMinting MinimumMint (r:1 w:1)
	fn set_minimum_mint() -> Weight {
		(9_949_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: VtokenMinting MinimumRedeem (r:1 w:1)
	fn set_minimum_redeem() -> Weight {
		(9_588_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: VtokenMinting UnlockDuration (r:1 w:1)
	fn set_unlock_duration() -> Weight {
		(9_668_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: VtokenMinting TokenToRebond (r:1 w:1)
	fn add_support_rebond_token() -> Weight {
		(9_728_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: VtokenMinting TokenToRebond (r:1 w:1)
	fn remove_support_rebond_token() -> Weight {
		(10_570_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: VtokenMinting Fees (r:1 w:1)
	fn set_fees() -> Weight {
		(9_478_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: VtokenMinting HookIterationLimit (r:1 w:1)
	fn set_hook_iteration_limit() -> Weight {
		(9_528_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
	// Storage: unknown [0x3a7472616e73616374696f6e5f6c6576656c3a] (r:1 w:1)
	// Storage: VtokenMinting MinimumMint (r:1 w:0)
	// Storage: VtokenMinting TokenPool (r:1 w:1)
	// Storage: Tokens TotalIssuance (r:1 w:1)
	// Storage: VtokenMinting Fees (r:1 w:0)
	// Storage: Tokens Accounts (r:3 w:3)
	// Storage: System Account (r:1 w:1)
	fn mint() -> Weight {
		(50_566_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(9 as Weight))
			.saturating_add(RocksDbWeight::get().writes(7 as Weight))
	}
	// Storage: unknown [0x3a7472616e73616374696f6e5f6c6576656c3a] (r:1 w:1)
	// Storage: VtokenMinting MinimumRedeem (r:1 w:0)
	// Storage: VtokenMinting Fees (r:1 w:0)
	// Storage: Tokens Accounts (r:2 w:2)
	// Storage: System Account (r:1 w:1)
	// Storage: VtokenMinting TokenPool (r:1 w:1)
	// Storage: Tokens TotalIssuance (r:1 w:1)
	// Storage: VtokenMinting OngoingTimeUnit (r:1 w:0)
	// Storage: VtokenMinting UnlockDuration (r:1 w:0)
	// Storage: VtokenMinting CurrencyUnlockingTotal (r:1 w:1)
	// Storage: VtokenMinting TokenUnlockNextId (r:1 w:1)
	// Storage: VtokenMinting UserUnlockLedger (r:1 w:1)
	// Storage: VtokenMinting TimeUnitUnlockLedger (r:1 w:1)
	// Storage: VtokenMinting TokenUnlockLedger (r:0 w:1)
	fn redeem() -> Weight {
		(67_898_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(14 as Weight))
			.saturating_add(RocksDbWeight::get().writes(11 as Weight))
	}
	// Storage: unknown [0x3a7472616e73616374696f6e5f6c6576656c3a] (r:1 w:1)
	// Storage: VtokenMinting TokenToRebond (r:1 w:1)
	// Storage: VtokenMinting UserUnlockLedger (r:1 w:1)
	// Storage: VtokenMinting TokenUnlockLedger (r:1 w:1)
	// Storage: VtokenMinting TimeUnitUnlockLedger (r:1 w:1)
	// Storage: VtokenMinting CurrencyUnlockingTotal (r:1 w:1)
	// Storage: VtokenMinting TokenPool (r:1 w:1)
	// Storage: Tokens TotalIssuance (r:1 w:1)
	// Storage: VtokenMinting Fees (r:1 w:0)
	// Storage: Tokens Accounts (r:3 w:3)
	fn rebond() -> Weight {
		(65_915_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(12 as Weight))
			.saturating_add(RocksDbWeight::get().writes(11 as Weight))
	}
	// Storage: unknown [0x3a7472616e73616374696f6e5f6c6576656c3a] (r:1 w:1)
	// Storage: VtokenMinting TokenToRebond (r:1 w:1)
	// Storage: VtokenMinting TokenUnlockLedger (r:1 w:1)
	// Storage: VtokenMinting TimeUnitUnlockLedger (r:1 w:1)
	// Storage: VtokenMinting UserUnlockLedger (r:1 w:1)
	// Storage: VtokenMinting CurrencyUnlockingTotal (r:1 w:1)
	// Storage: VtokenMinting TokenPool (r:1 w:1)
	// Storage: Tokens TotalIssuance (r:1 w:1)
	// Storage: VtokenMinting Fees (r:1 w:0)
	// Storage: Tokens Accounts (r:3 w:3)
	fn rebond_by_unlock_id() -> Weight {
		(64_001_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(12 as Weight))
			.saturating_add(RocksDbWeight::get().writes(11 as Weight))
	}
	// Storage: unknown [0x3a7472616e73616374696f6e5f6c6576656c3a] (r:1 w:1)
	// Storage: VtokenMinting OngoingTimeUnit (r:1 w:0)
	fn on_initialize() -> Weight {
		(5_049_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
}

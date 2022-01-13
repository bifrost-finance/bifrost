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

//! A set of constant values used for all runtimes in common.

#[allow(non_snake_case)]
pub mod parachains {
	pub mod karura {
		pub const ID: u32 = 2000;
		pub const KAR_KEY: &[u8] = &[0, 128];
		pub const KUSD_KEY: &[u8] = &[0, 129];
	}
	pub mod Statemine {
		pub const ID: u32 = 1000;
		pub const PALLET_ID: u8 = 50;
		pub const RMRK_ID: u32 = 8;
	}
	pub mod phala {
		pub const ID: u32 = 2004;
	}
}

/// Time.
pub mod time {
	use node_primitives::{BlockNumber, Moment};
	pub const MILLISECS_PER_BLOCK: Moment = 12000;
	pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;

	pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

	// 1 in 4 blocks (on average, not counting collisions) will be primary BABE blocks.
	pub const PRIMARY_PROBABILITY: (u64, u64) = (1, 4);

	pub const EPOCH_DURATION_IN_BLOCKS: BlockNumber = 6 * HOURS;
	pub const EPOCH_DURATION_IN_SLOTS: u64 = {
		const SLOT_FILL_RATE: f64 = MILLISECS_PER_BLOCK as f64 / SLOT_DURATION as f64;

		(EPOCH_DURATION_IN_BLOCKS as f64 * SLOT_FILL_RATE) as u64
	};

	// These time units are defined in number of blocks.
	pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
	pub const HOURS: BlockNumber = MINUTES * 60;
	pub const DAYS: BlockNumber = HOURS * 24;
	pub const WEEKS: BlockNumber = DAYS * 7;

	// The `LeasePeriod` defination from `polkadot`.
	pub const POLKA_LEASE_PERIOD: BlockNumber = 12 * WEEKS;
	pub const KUSAMA_LEASE_PERIOD: BlockNumber = 6 * WEEKS;
	pub const ROCOCO_LEASE_PERIOD: BlockNumber = 1 * DAYS;
	pub const WESTEND_LEASE_PERIOD: BlockNumber = 28 * DAYS;
}

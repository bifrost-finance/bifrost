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

//! Low-level types used throughout the Bifrost code.

use parity_scale_codec::MaxEncodedLen;
use scale_info::TypeInfo;
use sp_core::{Decode, Encode, RuntimeDebug};

// For vtoken-minting and slp modules
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, TypeInfo, MaxEncodedLen)]
pub enum TimeUnit {
	// Kusama staking time unit
	Era(#[codec(compact)] u32),
	SlashingSpan(#[codec(compact)] u32),
	// Moonriver staking time unit
	Round(#[codec(compact)] u32),
	// 1000 blocks. Can be used by Filecoin.
	// 30 seconds per block. Kblock means 8.33 hours.
	Kblock(#[codec(compact)] u32),
	// 1 hour. Should be Unix Timstamp in seconds / 3600
	Hour(#[codec(compact)] u32),
}

impl TimeUnit {
	pub fn add_one(self) -> Self {
		match self {
			TimeUnit::Era(a) => TimeUnit::Era(a.saturating_add(1)),
			TimeUnit::SlashingSpan(a) => TimeUnit::SlashingSpan(a.saturating_add(1)),
			TimeUnit::Round(a) => TimeUnit::Round(a.saturating_add(1)),
			TimeUnit::Kblock(a) => TimeUnit::Kblock(a.saturating_add(1)),
			TimeUnit::Hour(a) => TimeUnit::Hour(a.saturating_add(1)),
		}
	}

	pub fn add(self, other_time: Self) -> Option<Self> {
		match (self, other_time) {
			(TimeUnit::Era(a), TimeUnit::Era(b)) => Some(TimeUnit::Era(a.saturating_add(b))),
			(TimeUnit::SlashingSpan(a), TimeUnit::SlashingSpan(b)) =>
				Some(TimeUnit::SlashingSpan(a.saturating_add(b))),
			(TimeUnit::Round(a), TimeUnit::Round(b)) => Some(TimeUnit::Round(a.saturating_add(b))),
			(TimeUnit::Kblock(a), TimeUnit::Kblock(b)) =>
				Some(TimeUnit::Kblock(a.saturating_add(b))),
			(TimeUnit::Hour(a), TimeUnit::Hour(b)) => Some(TimeUnit::Hour(a.saturating_add(b))),
			_ => None,
		}
	}

	pub fn into_value(self) -> u32 {
		match self {
			TimeUnit::Era(a) => a,
			TimeUnit::SlashingSpan(a) => a,
			TimeUnit::Round(a) => a,
			TimeUnit::Kblock(a) => a,
			TimeUnit::Hour(a) => a,
		}
	}
}

impl Default for TimeUnit {
	fn default() -> Self {
		TimeUnit::Era(0u32)
	}
}

impl PartialEq for TimeUnit {
	fn eq(&self, other: &Self) -> bool {
		match (&self, other) {
			(Self::Era(a), Self::Era(b)) => a.eq(b),
			(Self::SlashingSpan(a), Self::SlashingSpan(b)) => a.eq(b),
			(Self::Round(a), Self::Round(b)) => a.eq(b),
			(Self::Kblock(a), Self::Kblock(b)) => a.eq(b),
			(Self::Hour(a), Self::Hour(b)) => a.eq(b),
			_ => false,
		}
	}
}

impl Ord for TimeUnit {
	fn cmp(&self, other: &Self) -> sp_std::cmp::Ordering {
		match (&self, other) {
			(Self::Era(a), Self::Era(b)) => a.cmp(b),
			(Self::SlashingSpan(a), Self::SlashingSpan(b)) => a.cmp(b),
			(Self::Round(a), Self::Round(b)) => a.cmp(b),
			(Self::Kblock(a), Self::Kblock(b)) => a.cmp(b),
			(Self::Hour(a), Self::Hour(b)) => a.cmp(b),
			_ => sp_std::cmp::Ordering::Less,
		}
	}
}

impl PartialOrd for TimeUnit {
	fn partial_cmp(&self, other: &Self) -> Option<sp_std::cmp::Ordering> {
		match (&self, other) {
			(Self::Era(a), Self::Era(b)) => Some(a.cmp(b)),
			(Self::SlashingSpan(a), Self::SlashingSpan(b)) => Some(a.cmp(b)),
			(Self::Round(a), Self::Round(b)) => Some(a.cmp(b)),
			(Self::Kblock(a), Self::Kblock(b)) => Some(a.cmp(b)),
			(Self::Hour(a), Self::Hour(b)) => Some(a.cmp(b)),
			_ => None,
		}
	}
}

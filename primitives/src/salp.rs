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
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Copy, RuntimeDebug)]
pub enum ContributionStatus<BalanceOf> {
	Idle,
	Contributing(BalanceOf),
	Refunded,
	Unlocked,
	Redeemed,
	MigrateToIdle,
}

#[derive(
	Encode,
	Decode,
	Clone,
	PartialEq,
	Eq,
	Copy,
	RuntimeDebug,
	scale_info::TypeInfo,
	Serialize,
	Deserialize,
)]
pub enum RpcContributionStatus {
	Idle,
	Contributing,
	Refunded,
	Unlocked,
	Redeemed,
	MigratedIdle,
}

impl<BalanceOf> ContributionStatus<BalanceOf>
where
	BalanceOf: frame_support::sp_runtime::traits::Zero + Clone + Copy,
{
	pub fn is_contributing(&self) -> bool {
		match self {
			Self::Contributing(_) => true,
			Self::Unlocked => true,
			_ => false,
		}
	}

	pub fn contributing(&self) -> BalanceOf {
		match self {
			Self::Contributing(contributing) => *contributing,
			_ => frame_support::sp_runtime::traits::Zero::zero(),
		}
	}

	pub fn to_rpc(&self) -> RpcContributionStatus {
		match self {
			Self::Idle => RpcContributionStatus::Idle,
			Self::Contributing(_) => RpcContributionStatus::Contributing,
			Self::Refunded => RpcContributionStatus::Refunded,
			Self::Unlocked => RpcContributionStatus::Unlocked,
			Self::Redeemed => RpcContributionStatus::Redeemed,
			Self::MigrateToIdle => RpcContributionStatus::MigratedIdle,
		}
	}
}

impl<BalanceOf> Default for ContributionStatus<BalanceOf> {
	fn default() -> Self {
		Self::Idle
	}
}

pub type MessageId = [u8; 32];

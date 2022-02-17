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

use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use scale_info::TypeInfo;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum Ledger<DelegatorId, Balance> {
	Substrate(SubstrateLedger<DelegatorId, Balance>),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SubstrateLedger<DelegatorId, Balance> {
	/// The delegator account Id
	pub account: DelegatorId,
	/// The total amount of the delegator's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	pub total: Balance,
	/// The total amount of the delegator's balance that will be at stake in any forthcoming
	/// rounds.
	pub active: Balance,
	/// Any balance that is becoming free, which may eventually be transferred out
	/// of the delegator (assuming it doesn't get slashed first).
	pub unlocking: Vec<UnlockChunk<Balance>>,
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UnlockChunk<Balance> {
	/// Amount of funds to be unlocked.
	value: Balance,
	/// Era number at which point it'll be unlocked.
	unlock_time: TimeUnit,
}

/// Timing units for different chains.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum TimeUnit {
	Era(u64),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Delays {
	/// Payout delays once the staking calculation period has finished.
	payout_delay: TimeUnit,
	/// Time delays to take effect after a delegator submit its supporting validators.
	delegate_delay: TimeUnit,
	/// Time delays to take effect if a delegator change its supporting validators.
	redelegate_delay: TimeUnit,
	/// Time delays to get its money back if a delegator unbonds.
	unbond_delay: TimeUnit,
}

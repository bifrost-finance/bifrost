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

mod filecoin_primitives;
mod moonbeam_primitives;
mod phala_primitives;
mod polkadot_primitives;

pub use filecoin_primitives::*;
pub use moonbeam_primitives::*;
pub use phala_primitives::*;
pub use polkadot_primitives::*;

use crate::XcmWeight;
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use node_primitives::TimeUnit;
use scale_info::TypeInfo;

pub type QueryId = u64;
pub const TIMEOUT_BLOCKS: u32 = 1000;
pub const BASE_WEIGHT: XcmWeight = 1000;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum Ledger<Balance> {
	Substrate(SubstrateLedger<Balance>),
	Moonbeam(OneToManyLedger<Balance>),
	ParachainStaking(OneToManyLedger<Balance>),
	Filecoin(FilecoinLedger<Balance>),
	Phala(PhalaLedger<Balance>),
}

/// A type for accommodating delegator update entries for different kinds of currencies.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum LedgerUpdateEntry<Balance> {
	/// A type for substrate ledger updating entries
	Substrate(SubstrateLedgerUpdateEntry<Balance>),
	Moonbeam(MoonbeamLedgerUpdateEntry<Balance>),
	ParachainStaking(MoonbeamLedgerUpdateEntry<Balance>),
}

/// A type for accommodating validators by delegator update entries for different kinds of
/// currencies.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum ValidatorsByDelegatorUpdateEntry<HashT> {
	/// A type for substrate validators by delegator updating entries
	Substrate(SubstrateValidatorsByDelegatorUpdateEntry<HashT>),
}

/// Different minimum and maximum requirements for different chain
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct MinimumsMaximums<Balance> {
	/// The minimum bonded amount for a delegator at any time.
	#[codec(compact)]
	pub delegator_bonded_minimum: Balance,
	/// The minimum amount each time a delegator needs to bond for extra
	#[codec(compact)]
	pub bond_extra_minimum: Balance,
	/// The minimum unbond amount each time a delegator to unbond.
	#[codec(compact)]
	pub unbond_minimum: Balance,
	/// The minimum amount each time a delegator needs to rebond
	#[codec(compact)]
	pub rebond_minimum: Balance,
	/// The maximum number of unbond records at the same time.
	#[codec(compact)]
	pub unbond_record_maximum: u32,
	/// The maximum number of validators for a delegator to support at the same time.
	#[codec(compact)]
	pub validators_back_maximum: u32,
	/// The maximum amount of active staking for a delegator. It is used to control ROI.
	#[codec(compact)]
	pub delegator_active_staking_maximum: Balance,
	/// The maximum number of delegators for a validator to reward.
	#[codec(compact)]
	pub validators_reward_maximum: u32,
	/// The minimum amount for a delegation.
	#[codec(compact)]
	pub delegation_amount_minimum: Balance,
	// Maximum delegators count.
	#[codec(compact)]
	pub delegators_maximum: u16,
	// Maximum validators candidates in the whitelist(Validators<T>)
	#[codec(compact)]
	pub validators_maximum: u16,
}

/// Different delay params for different chain
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Delays {
	/// The unlock delay for the unlocking amount to be able to be liquidized.
	pub unlock_delay: TimeUnit,
	/// Leave from delegator set delay.
	pub leave_delegators_delay: TimeUnit,
}

/// XCM operations list
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, TypeInfo)]
pub enum XcmOperation {
	// XTokens
	XtokensTransfer,
	Bond,
	WithdrawUnbonded,
	BondExtra,
	Unbond,
	Rebond,
	Delegate,
	Payout,
	Liquidize,
	TransferBack,
	TransferTo,
	Chill,
	Undelegate,
	CancelLeave,
	XtokensTransferBack,
	ExecuteLeave,
	ConvertAsset,
}

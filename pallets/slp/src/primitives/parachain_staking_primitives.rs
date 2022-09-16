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

use codec::{alloc::collections::BTreeMap, Decode, Encode};
use frame_support::RuntimeDebug;
use node_primitives::{CurrencyId, TimeUnit, TokenSymbol};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

pub const BNC: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ParachainStakingOneToManyLedger<DelegatorId, ValidatorId, Balance> {
	pub account: DelegatorId,
	pub delegations: BTreeMap<ValidatorId, Balance>,
	pub total: Balance,
	pub less_total: Balance,
	// request details.
	pub requests: Vec<ParachainStakingOneToManyScheduledRequest<ValidatorId, Balance>>,
	// fast check if request exists
	pub request_briefs: BTreeMap<ValidatorId, (TimeUnit, Balance)>,
	pub status: ParachainStakingOneToManyDelegatorStatus,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum ParachainStakingOneToManyDelegatorStatus {
	Active,
	Leaving(TimeUnit),
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, PartialOrd, Ord)]
pub struct ParachainStakingOneToManyScheduledRequest<ValidatorId, Balance> {
	pub validator: ValidatorId,
	pub when_executable: TimeUnit,
	pub action: ParachainStakingOneToManyDelegationAction<Balance>,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, PartialOrd, Ord)]
pub enum ParachainStakingOneToManyDelegationAction<Balance> {
	Revoke(Balance),
	Decrease(Balance),
}

/// A type for Moonbeam ledger updating entries
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ParachainStakingLedgerUpdateEntry<Balance, DelegatorId, ValidatorId> {
	/// The currency id of the delegator that needs to be update
	pub currency_id: CurrencyId,
	/// The delegator id that needs to be update
	pub delegator_id: DelegatorId,
	/// The validator id that needs to be update
	pub validator_id: Option<ValidatorId>,
	/// Update operation type
	pub update_operation: ParachainStakingLedgerUpdateOperation,
	#[codec(compact)]
	pub amount: Balance,
	/// If this entry is an unlocking entry, it should have unlock_time value. If it is a bonding
	/// entry, this field should be None. If it is a liquidize entry, this filed is the ongoing
	/// timeunit when the xcm message is sent.
	pub unlock_time: Option<TimeUnit>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum ParachainStakingLedgerUpdateOperation {
	Bond,
	BondLess,
	Revoke,
	CancelRequest,
	LeaveDelegator,
	CancelLeave,
	ExecuteLeave,
	ExecuteRequest,
}

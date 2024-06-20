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

use bifrost_primitives::{CurrencyId, TimeUnit};
use parity_scale_codec::{alloc::collections::BTreeMap, Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;
use xcm::v3::MultiLocation;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct OneToManyLedger<Balance> {
	pub account: MultiLocation,
	pub delegations: BTreeMap<MultiLocation, Balance>,
	pub total: Balance,
	pub less_total: Balance,
	// request details.
	pub requests: Vec<OneToManyScheduledRequest<Balance>>,
	// fast check if request exists
	pub request_briefs: BTreeMap<MultiLocation, (TimeUnit, Balance)>,
	pub status: OneToManyDelegatorStatus,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum OneToManyDelegatorStatus {
	Active,
	Leaving(TimeUnit),
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, PartialOrd, Ord)]
pub struct OneToManyScheduledRequest<Balance> {
	pub validator: MultiLocation,
	pub when_executable: TimeUnit,
	pub action: OneToManyDelegationAction<Balance>,
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, TypeInfo, PartialOrd, Ord)]
pub enum OneToManyDelegationAction<Balance> {
	Revoke(Balance),
	Decrease(Balance),
}

/// A type for ParachainStaking ledger updating entries
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ParachainStakingLedgerUpdateEntry<Balance> {
	/// The currency id of the delegator that needs to be update
	pub currency_id: CurrencyId,
	/// The delegator id that needs to be update
	pub delegator_id: MultiLocation,
	/// The validator id that needs to be update
	pub validator_id: Option<MultiLocation>,
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

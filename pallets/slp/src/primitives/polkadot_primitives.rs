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

use crate::MultiLocation;
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use node_primitives::{CurrencyId, TimeUnit, TokenSymbol};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

pub const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SubstrateLedger<Balance> {
	/// The delegator account Id
	pub account: MultiLocation,
	/// The total amount of the delegator's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	#[codec(compact)]
	pub total: Balance,
	/// The total amount of the delegator's balance that will be at stake in any forthcoming
	/// rounds.
	#[codec(compact)]
	pub active: Balance,
	/// Any balance that is becoming free, which may eventually be transferred out
	/// of the delegator (assuming it doesn't get slashed first).
	pub unlocking: Vec<UnlockChunk<Balance>>,
}

/// Just a Balance/BlockNumber tuple to encode when a chunk of funds will be unlocked.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct UnlockChunk<Balance> {
	/// Amount of funds to be unlocked.
	#[codec(compact)]
	pub value: Balance,
	/// Era number at which point it'll be unlocked.
	pub unlock_time: TimeUnit,
}

/// A type for substrate ledger updating entries
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SubstrateLedgerUpdateEntry<Balance> {
	/// The currency id of the delegator that needs to be update
	pub currency_id: CurrencyId,
	/// The delegator id that needs to be update
	pub delegator_id: MultiLocation,
	/// Update operation type
	pub update_operation: SubstrateLedgerUpdateOperation,
	/// The unlocking/bonding amount.
	#[codec(compact)]
	pub amount: Balance,
	/// If this entry is an unlocking entry, it should have unlock_time value. If it is a bonding
	/// entry, this field should be None. If it is a liquidize entry, this filed is the ongoing
	/// timeunit when the xcm message is sent.
	pub unlock_time: Option<TimeUnit>,
}

/// A type for substrate validators by delegator updating entries
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct SubstrateValidatorsByDelegatorUpdateEntry {
	/// The currency id of the delegator that needs to be update
	pub currency_id: CurrencyId,
	/// The delegator id that needs to be update
	pub delegator_id: MultiLocation,
	/// Validators vec to be updated
	pub validators: Vec<MultiLocation>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum SubstrateLedgerUpdateOperation {
	Bond,
	Unlock,
	Rebond,
	Liquidize,
}

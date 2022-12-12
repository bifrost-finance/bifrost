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


#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct PhalaLedger<Balance> {
	/// The delegator multilocation
	pub account: MultiLocation,
	/// The total amount of the delegator's balance that will be at stake in any forthcoming
	/// rounds.
	#[codec(compact)]
	pub active_shares: Balance,
	/// Any balance that is becoming free, which may eventually be transferred out
	/// of the delegator (assuming it doesn't get slashed first).
	#[codec(compact)]
	pub unlocking_shares: Balance,
	/// If the delegator is bonded, it should record the bonded pool id.
	pub bonded_pool_id: Option<u64>,
	/// If the delegator is bonded, it should record the bonded pool NFT collection id.
	pub bonded_pool_collection_id: Option<u32>
}
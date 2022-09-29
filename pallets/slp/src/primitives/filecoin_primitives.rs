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

use crate::{CurrencyId, MultiLocation};
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use scale_info::TypeInfo;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct FilecoinLedger<Balance> {
	/// The delegator account Id
	pub account: MultiLocation,
	// Initial pledge collateral for the miner
	#[codec(compact)]
	pub initial_pledge: Balance,
}

/// A type for filecoin validator(owner) by delegator(miner) updating entries
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct FilecoinOwnerByMinerEntry {
	pub currency_id: CurrencyId,
	pub miner_id: MultiLocation,
	pub owner_id: MultiLocation,
}

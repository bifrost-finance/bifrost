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

use bifrost_primitives::{Balance, BlockNumber, CurrencyId, TimeUnit};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H160;
use sp_runtime::Permill;
use xcm::v4::{Location, Weight};

/// Sovereign addresses generate subaccounts via DelegatorIndex
pub type DelegatorIndex = u16;
/// Pallet index in remote chain.
pub type PalletIndex = u8;
pub const AS_DERIVATIVE_CALL_INDEX: u8 = 1;
pub const LIMITED_RESERVE_TRANSFER_ASSETS_CALL_INDEX: u8 = 8;

#[derive(Encode, Decode, MaxEncodedLen, Default, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub struct ProtocolConfiguration {
	pub xcm_task_fee: XcmFee,
	pub protocol_fee_rate: Permill,
	pub unlock_period: TimeUnit,
	pub max_update_token_exchange_rate: Permill,
	pub update_time_unit_interval: BlockNumber,
	pub update_exchange_rate_interval: BlockNumber,
}

#[derive(Encode, Decode, MaxEncodedLen, Default, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub struct StakingProtocolInfo {
	pub utility_pallet_index: PalletIndex,
	pub polkadot_xcm_pallet_index: PalletIndex,
	pub currency_id: CurrencyId,
	pub unlock_period: TimeUnit,
	pub remote_fee_location: Location,
	pub remote_refund_beneficiary: Location,
	pub remote_dest_location: Location,
	pub bifrost_dest_location: Location,
}

/// Delegator account
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum Delegator<AccountId> {
	/// Substrate account
	Substrate(AccountId),
	/// Ethereum address.
	Ethereum(H160),
}

#[derive(Encode, Decode, MaxEncodedLen, Default, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct XcmFee {
	pub weight: Weight,
	pub fee: Balance,
}

#[cfg(feature = "kusama")]
pub mod kusama;
#[cfg(feature = "kusama")]
pub use kusama::*;
#[cfg(feature = "polkadot")]
pub mod polkadot;
#[cfg(feature = "polkadot")]
pub use polkadot::*;

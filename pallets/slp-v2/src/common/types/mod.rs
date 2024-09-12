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
/// As derivative call index
pub const AS_DERIVATIVE_CALL_INDEX: u8 = 1;
/// Reserve transfer assets call index
pub const LIMITED_RESERVE_TRANSFER_ASSETS_CALL_INDEX: u8 = 8;

/// Configuration of the protocol
#[derive(Encode, Decode, MaxEncodedLen, Default, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub struct ProtocolConfiguration<AccountId> {
	/// Xcm fee for the task
	pub xcm_task_fee: XcmFee,
	/// Protocol fee rate
	pub protocol_fee_rate: Permill,
	/// Unlock period
	pub unlock_period: TimeUnit,
	/// Staking Protocol operator
	pub operator: AccountId,
	/// Max update token exchange rate
	pub max_update_token_exchange_rate: Permill,
	/// Update time unit interval
	pub update_time_unit_interval: BlockNumber,
	/// Update exchange rate interval
	pub update_exchange_rate_interval: BlockNumber,
}

/// Staking protocol information
#[derive(Encode, Decode, MaxEncodedLen, Default, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub struct StakingProtocolInfo {
	/// Utility pallet index
	pub utility_pallet_index: PalletIndex,
	/// Xcm pallet index
	pub xcm_pallet_index: PalletIndex,
	/// Currency Id for Staking Protocol
	pub currency_id: CurrencyId,
	/// Unlock period
	pub unlock_period: TimeUnit,
	/// Remote chain supports fee location.
	pub remote_fee_location: Location,
	/// Remote chain supports refund location.
	pub remote_refund_beneficiary: Location,
	/// Dest location for remote chain
	pub remote_dest_location: Location,
	/// Bifrost dest location
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

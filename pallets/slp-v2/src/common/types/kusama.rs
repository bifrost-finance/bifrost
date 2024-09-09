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

use crate::{
	common::types::{Delegator, DelegatorIndex, StakingProtocolInfo},
	Config, Error,
};
use bifrost_primitives::{
	Balance, BifrostKusamaChainId, MoonbeamChainId, MoonriverChainId, TimeUnit, KSM, MOVR,
};
use frame_support::traits::Get;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use polkadot_parachain_primitives::primitives::Sibling;
use scale_info::TypeInfo;
use sp_core::H160;
use sp_runtime::traits::AccountIdConversion;
use xcm::{
	prelude::{AccountId32, AccountKey20, PalletInstance, Parachain},
	v4::Location,
};

/// Supported staking protocols.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum StakingProtocol {
	/// ParachainStaking on Moonriver.
	MoonriverParachainStaking,
	/// Staking on Kusama
	KusamaStaking,
}

impl StakingProtocol {
	pub(crate) fn info(&self) -> StakingProtocolInfo {
		match self {
			StakingProtocol::MoonriverParachainStaking => StakingProtocolInfo {
				utility_pallet_index: 30,
				xcm_pallet_index: 103,
				currency_id: MOVR,
				unlock_period: TimeUnit::Round(28),
				remote_fee_location: Location::new(0, [PalletInstance(10)]),
				remote_refund_beneficiary: Location::new(
					0,
					[AccountKey20 {
						network: None,
						key: Sibling::from(BifrostKusamaChainId::get()).into_account_truncating(),
					}],
				),
				remote_dest_location: Location::new(1, [Parachain(MoonbeamChainId::get())]),
				bifrost_dest_location: Location::new(1, Parachain(BifrostKusamaChainId::get())),
			},
			StakingProtocol::KusamaStaking => StakingProtocolInfo {
				utility_pallet_index: 24,
				xcm_pallet_index: 99,
				currency_id: KSM,
				unlock_period: TimeUnit::Era(28),
				remote_fee_location: Location::here(),
				remote_refund_beneficiary: Location::new(
					0,
					[Parachain(BifrostKusamaChainId::get())],
				),
				remote_dest_location: Location::parent(),
				bifrost_dest_location: Location::new(0, Parachain(BifrostKusamaChainId::get())),
			},
		}
	}

	pub fn get_dest_beneficiary_location<T: Config>(
		&self,
		delegator: Delegator<T::AccountId>,
	) -> Option<Location> {
		match (self, delegator) {
			(StakingProtocol::KusamaStaking, Delegator::Substrate(account_id)) =>
				account_id.encode().try_into().ok().and_then(|account_id| {
					Some(Location::new(1, [AccountId32 { network: None, id: account_id }]))
				}),
			(StakingProtocol::MoonriverParachainStaking, Delegator::Ethereum(account_id)) =>
				Some(Location::new(
					1,
					[
						Parachain(MoonriverChainId::get()),
						AccountKey20 { network: None, key: account_id.to_fixed_bytes() },
					],
				)),
			_ => None,
		}
	}

	pub fn get_delegator<T: Config>(
		&self,
		_delegator_index: DelegatorIndex,
	) -> Result<Delegator<T::AccountId>, Error<T>> {
		match &self {
			_ => unreachable!(),
		}
	}

	pub fn get_default_ledger(&self) -> Ledger {
		match self {
			_ => unreachable!(),
		}
	}
}

/// Validator in slp protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub enum Validator<AccountId> {
	MoonriverParachainStaking(H160),
	KusamaStaking(AccountId),
}

/// Ledger in slp protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub enum Ledger {}

#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum XcmTask<AccountId> {
	Todo(AccountId),
}

/// PendingStatus in slp protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum PendingStatus<AccountId> {
	Todo(AccountId),
}

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
	astar_dapp_staking::types::{AstarDappStakingLedger, AstarValidator, DappStaking},
	Config,
};
use bifrost_primitives::{Balance, CurrencyId, TimeUnit, ASTR, DOT, GLMR, KSM, MOVR};
use cumulus_primitives_core::ParaId;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use polkadot_parachain_primitives::primitives::Sibling;
use scale_info::TypeInfo;
use sp_core::{Get, H160};
use sp_runtime::traits::AccountIdConversion;
use xcm::{
	prelude::{AccountId32, AccountKey20, PalletInstance, Parachain},
	v4::{Location, Weight},
};

/// Sovereign addresses generate subaccounts via DelegatorIndex
pub type DelegatorIndex = u16;
/// Pallet index in remote chain.
pub type PalletIndex = u8;
pub const AS_DERIVATIVE_CALL_INDEX: u8 = 1;
pub const LIMITED_RESERVE_TRANSFER_ASSETS_CALL_INDEX: u8 = 8;

/// Supported staking protocols.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum StakingProtocol {
	/// DappStaking on Astar.
	AstarDappStaking,
	/// ParachainStaking on Moonbeam.
	MoonbeamParachainStaking,
	/// Staking on Polkadot
	PolkadotStaking,
}

impl StakingProtocol {
	pub fn get_utility_pallet_index(&self) -> PalletIndex {
		match self {
			StakingProtocol::AstarDappStaking => 11,
			StakingProtocol::MoonbeamParachainStaking => 30,
			#[cfg(feature = "polkadot")]
			StakingProtocol::PolkadotStaking => 26,
			#[cfg(feature = "kusama")]
			StakingProtocol::PolkadotStaking => 24,
		}
	}
	pub fn get_polkadot_xcm_pallet_index(&self) -> PalletIndex {
		match self {
			StakingProtocol::AstarDappStaking => 51,
			StakingProtocol::MoonbeamParachainStaking => 103,
			StakingProtocol::PolkadotStaking => 99,
		}
	}
	pub fn get_currency_id(&self) -> CurrencyId {
		match self {
			StakingProtocol::AstarDappStaking => ASTR,
			#[cfg(feature = "polkadot")]
			StakingProtocol::MoonbeamParachainStaking => GLMR,
			#[cfg(feature = "kusama")]
			StakingProtocol::MoonbeamParachainStaking => MOVR,
			#[cfg(feature = "polkadot")]
			StakingProtocol::PolkadotStaking => DOT,
			#[cfg(feature = "kusama")]
			StakingProtocol::PolkadotStaking => KSM,
		}
	}
	pub fn get_fee_location(&self) -> Location {
		match self {
			StakingProtocol::MoonbeamParachainStaking => Location::new(0, [PalletInstance(10)]),
			_ => Location::here(),
		}
	}
	pub fn get_refund_beneficiary<T: Config>(&self) -> Location {
		match self {
			StakingProtocol::AstarDappStaking => Location::new(
				0,
				[AccountId32 {
					network: None,
					id: Sibling::from(T::ParachainId::get()).into_account_truncating(),
				}],
			),
			StakingProtocol::MoonbeamParachainStaking => Location::new(
				0,
				[AccountKey20 {
					network: None,
					key: Sibling::from(T::ParachainId::get()).into_account_truncating(),
				}],
			),
			StakingProtocol::PolkadotStaking =>
				Location::new(0, [Parachain(T::ParachainId::get().into())]),
		}
	}
	pub fn get_dest_location(&self) -> Location {
		match self {
			StakingProtocol::AstarDappStaking => Location::new(1, [Parachain(2006)]),
			#[cfg(feature = "polkadot")]
			StakingProtocol::MoonbeamParachainStaking => Location::new(1, [Parachain(2004)]),
			#[cfg(feature = "kusama")]
			StakingProtocol::MoonbeamParachainStaking => Location::new(1, [Parachain(2023)]),
			StakingProtocol::PolkadotStaking => Location::parent(),
		}
	}
	pub fn get_dest_beneficiary_location<T: Config>(
		&self,
		delegator: Delegator<T::AccountId>,
	) -> Option<Location> {
		match (self, delegator) {
			(StakingProtocol::AstarDappStaking, Delegator::Substrate(account_id)) =>
				account_id.encode().try_into().ok().and_then(|account_id| {
					Some(Location::new(
						1,
						[Parachain(2006), AccountId32 { network: None, id: account_id }],
					))
				}),
			(StakingProtocol::PolkadotStaking, Delegator::Substrate(account_id)) =>
				account_id.encode().try_into().ok().and_then(|account_id| {
					Some(Location::new(1, [AccountId32 { network: None, id: account_id }]))
				}),
			#[cfg(feature = "polkadot")]
			(StakingProtocol::MoonbeamParachainStaking, Delegator::Ethereum(account_id)) =>
				Some(Location::new(
					1,
					[
						Parachain(2004),
						AccountKey20 { network: None, key: account_id.to_fixed_bytes() },
					],
				)),
			#[cfg(feature = "kusama")]
			(StakingProtocol::MoonbeamParachainStaking, Delegator::Ethereum(account_id)) =>
				Some(Location::new(
					1,
					[
						Parachain(2023),
						AccountKey20 { network: None, key: account_id.to_fixed_bytes() },
					],
				)),
			_ => None,
		}
	}
	pub fn get_sovereign_account<T: Config>(&self) -> T::AccountId {
		match self {
			StakingProtocol::PolkadotStaking =>
				ParaId::from(T::ParachainId::get()).into_account_truncating(),
			_ => Sibling::from(T::ParachainId::get()).into_account_truncating(),
		}
	}
	pub fn get_report_transact_status_dest_location<T: Config>(&self) -> Location {
		match self {
			StakingProtocol::PolkadotStaking =>
				Location::new(0, [Parachain(T::ParachainId::get().into())]),
			_ => Location::new(1, [Parachain(T::ParachainId::get().into())]),
		}
	}
	pub fn get_default_ledger(&self) -> Ledger {
		match self {
			StakingProtocol::AstarDappStaking =>
				Ledger::AstarDappStaking(AstarDappStakingLedger::default()),
			_ => unreachable!(),
		}
	}
	pub fn get_default_time_unit(&self) -> TimeUnit {
		match self {
			StakingProtocol::AstarDappStaking => TimeUnit::Era(1),
			_ => unreachable!(),
		}
	}
	pub fn get_unlock_period(&self) -> TimeUnit {
		match self {
			StakingProtocol::AstarDappStaking => TimeUnit::Era(9),
			StakingProtocol::MoonbeamParachainStaking => TimeUnit::Round(28),
			StakingProtocol::PolkadotStaking => TimeUnit::Era(28),
		}
	}
	pub fn get_bifrost_dest_location<T: Config>(&self) -> Location {
		match self {
			StakingProtocol::AstarDappStaking =>
				Location::new(1, Parachain(T::ParachainId::get().into())),
			StakingProtocol::MoonbeamParachainStaking =>
				Location::new(1, Parachain(T::ParachainId::get().into())),
			StakingProtocol::PolkadotStaking =>
				Location::new(0, Parachain(T::ParachainId::get().into())),
		}
	}

	pub fn get_native_token_location<T: Config>(&self) -> Location {
		match self {
			StakingProtocol::AstarDappStaking =>
				Location::new(1, Parachain(T::ParachainId::get().into())),
			StakingProtocol::MoonbeamParachainStaking =>
				Location::new(1, Parachain(T::ParachainId::get().into())),
			StakingProtocol::PolkadotStaking =>
				Location::new(0, Parachain(T::ParachainId::get().into())),
		}
	}
}

/// Delegator account
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum Delegator<AccountId> {
	/// Substrate account
	Substrate(AccountId),
	/// Ethereum address.
	Ethereum(H160),
}

/// Validator in slp protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub enum Validator<AccountId> {
	/// DappStaking on Astar.
	DappStaking(AstarValidator<AccountId>),
}

#[derive(Encode, Decode, MaxEncodedLen, Default, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub struct XcmFee {
	pub weight: Weight,
	pub fee: Balance,
}

/// Delegator in slp protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub enum Ledger {
	/// DappStaking on Astar.
	AstarDappStaking(AstarDappStakingLedger),
}

/// Delegator in slp protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum XcmTask {
	/// DappStaking on Astar.
	AstarDappStakingLock,
	AstarDappStakingUnLock,
	AstarDappStakingClaimUnlocked,
	AstarDappStakingRelockUnlocking,
	AstarDappStakingStake,
	AstarDappStakingUnstake,
	AstarDappStakingClaimStakerRewards,
	AstarDappStakingClaimBonusReward,
	AstarDappStakingTransferBack,
}

#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum XcmTaskWithParams<AccountId> {
	AstarDappStaking(DappStaking<AccountId>),
}

/// Delegator in slp protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum PendingStatus<AccountId> {
	/// DappStaking on Astar.
	AstarDappStakingLock(Delegator<AccountId>, Balance),
	AstarDappStakingUnLock(Delegator<AccountId>, Balance),
	AstarDappStakingClaimUnlocked(Delegator<AccountId>),
}

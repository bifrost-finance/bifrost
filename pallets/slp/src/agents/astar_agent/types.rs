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

use crate::Config;
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use scale_info::TypeInfo;
use sp_core::H160;
use sp_std::{boxed::Box, vec::Vec};
use xcm::{opaque::v3::WeightLimit, VersionedMultiAssets, VersionedMultiLocation};

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum AstarCall<T: Config> {
	#[codec(index = 34)]
	Staking(AstarDappsStakingCall<T>),
	#[codec(index = 11)]
	Utility(Box<AstarUtilityCall<Self>>),
	#[codec(index = 51)]
	Xcm(Box<XcmCall>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum AstarUtilityCall<AstarCall> {
	#[codec(index = 1)]
	AsDerivative(u16, Box<AstarCall>),
	#[codec(index = 2)]
	BatchAll(Box<Vec<Box<AstarCall>>>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum AstarDappsStakingCall<T: Config> {
	#[codec(index = 3)]
	BondAndStake(SmartContract<T::AccountId>, #[codec(compact)] u128),
	#[codec(index = 4)]
	UnbondAndUnstake(SmartContract<T::AccountId>, #[codec(compact)] u128),
	#[codec(index = 5)]
	WithdrawUnbonded,
	#[codec(index = 6)]
	NominationTransfer(
		SmartContract<T::AccountId>,
		#[codec(compact)] u128,
		SmartContract<T::AccountId>,
	),
	#[codec(index = 7)]
	ClaimStaker(SmartContract<T::AccountId>),
	#[codec(index = 11)]
	SetRewardDestination(RewardDestination),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum XcmCall {
	#[codec(index = 8)]
	LimitedReserveTransferAssets(
		Box<VersionedMultiLocation>,
		Box<VersionedMultiLocation>,
		Box<VersionedMultiAssets>,
		u32,
		WeightLimit,
	),
}

/// Instruction on how to handle reward payout for stakers.
/// In order to make staking more competitive, majority of stakers will want to
/// automatically restake anything they earn.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum RewardDestination {
	/// Rewards are transferred to stakers free balance without any further action.
	FreeBalance,
	/// Rewards are transferred to stakers balance and are immediately re-staked
	/// on the contract from which the reward was received.
	StakeBalance,
}

/// Multi-VM pointer to smart contract instance.
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum SmartContract<AccountId> {
	/// EVM smart contract instance.
	Evm(H160),
	/// Wasm smart contract instance.
	Wasm(AccountId),
}

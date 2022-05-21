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

use crate::{agents::SystemCall, Weight};
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use scale_info::TypeInfo;
use sp_core::H160;
use sp_runtime::traits::{IdentityLookup, StaticLookup};
use sp_std::{boxed::Box, vec::Vec};
use xcm::VersionedMultiLocation;

use crate::{BalanceOf, Config};

#[derive(Encode, Decode, RuntimeDebug)]
pub enum MoonriverCall<T: Config> {
	#[codec(index = 0)]
	System(SystemCall),
	#[codec(index = 10)]
	Balances(MoonriverBalancesCall<T>),
	#[codec(index = 20)]
	Staking(MoonriverParachainStakingCall<T>),
	#[codec(index = 30)]
	Utility(Box<MoonriverUtilityCall<Self>>),
	#[codec(index = 106)]
	Xtokens(MoonriverXtokensCall<T>),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum MoonriverBalancesCall<T: Config> {
	#[codec(index = 3)]
	TransferKeepAlive(
		<IdentityLookup<H160> as StaticLookup>::Source,
		#[codec(compact)] BalanceOf<T>,
	),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum MoonriverUtilityCall<MoonriverCall> {
	#[codec(index = 1)]
	AsDerivative(u16, Box<MoonriverCall>),
	#[codec(index = 2)]
	BatchAll(Box<Vec<Box<MoonriverCall>>>),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum MoonriverParachainStakingCall<T: Config> {
	#[codec(index = 17)]
	Delegate(H160, #[codec(compact)] BalanceOf<T>, u32, u32),
	#[codec(index = 18)]
	ScheduleLeaveDelegators,
	#[codec(index = 19)]
	ExecuteLeaveDelegators(H160, u32),
	#[codec(index = 20)]
	CancelLeaveDelegators,
	#[codec(index = 21)]
	ScheduleRevokeDelegation(H160),
	#[codec(index = 22)]
	DelegatorBondMore(H160, #[codec(compact)] BalanceOf<T>),
	#[codec(index = 23)]
	ScheduleDelegatorBondLess(H160, #[codec(compact)] BalanceOf<T>),
	#[codec(index = 24)]
	ExecuteDelegationRequest(H160, H160),
	#[codec(index = 25)]
	CancelDelegationRequest(H160),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum MoonriverXtokensCall<T: Config> {
	#[codec(index = 0)]
	Transfer(
		MoonriverCurrencyId,
		#[codec(compact)] BalanceOf<T>,
		Box<VersionedMultiLocation>,
		Weight,
	),
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum MoonriverCurrencyId {
	// Our native token
	SelfReserve,
	// Assets representing other chains native tokens
	ForeignAsset(u128),
	// Our local assets
	LocalAssetReserve(u128),
}

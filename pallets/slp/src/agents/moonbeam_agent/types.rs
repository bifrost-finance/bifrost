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

use crate::agents::SystemCall;
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use scale_info::TypeInfo;
use sp_core::H160;
use sp_runtime::traits::{IdentityLookup, StaticLookup};
use sp_std::{boxed::Box, vec::Vec};
use xcm::VersionedMultiLocation;

use crate::{BalanceOf, Config};

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum MoonbeamCall<T: Config> {
	#[codec(index = 0)]
	System(SystemCall),
	#[codec(index = 10)]
	Balances(MoonbeamBalancesCall<T>),
	#[codec(index = 20)]
	Staking(MoonbeamParachainStakingCall<T>),
	#[codec(index = 30)]
	Utility(Box<MoonbeamUtilityCall<Self>>),
	#[codec(index = 106)]
	Xtokens(MoonbeamXtokensCall<T>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum MoonbeamBalancesCall<T: Config> {
	#[codec(index = 3)]
	TransferKeepAlive(
		<IdentityLookup<H160> as StaticLookup>::Source,
		#[codec(compact)] BalanceOf<T>,
	),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum MoonbeamUtilityCall<MoonbeamCall> {
	#[codec(index = 1)]
	AsDerivative(u16, Box<MoonbeamCall>),
	#[codec(index = 2)]
	BatchAll(Box<Vec<Box<MoonbeamCall>>>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum MoonbeamParachainStakingCall<T: Config> {
	#[codec(index = 17)]
	Delegate(H160, BalanceOf<T>, u32, u32),
	#[codec(index = 19)]
	ScheduleLeaveDelegators,
	#[codec(index = 20)]
	ExecuteLeaveDelegators(H160, u32),
	#[codec(index = 21)]
	CancelLeaveDelegators,
	#[codec(index = 22)]
	ScheduleRevokeDelegation(H160),
	#[codec(index = 23)]
	DelegatorBondMore(H160, BalanceOf<T>),
	#[codec(index = 24)]
	ScheduleDelegatorBondLess(H160, BalanceOf<T>),
	#[codec(index = 25)]
	ExecuteDelegationRequest(H160, H160),
	#[codec(index = 26)]
	CancelDelegationRequest(H160),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum MoonbeamXtokensCall<T: Config> {
	#[codec(index = 0)]
	Transfer(MoonbeamCurrencyId, BalanceOf<T>, Box<VersionedMultiLocation>, u64),
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum MoonbeamCurrencyId {
	// Our native token
	SelfReserve,
	// Assets representing other chains native tokens
	ForeignAsset(u128),
	// Our local assets
	LocalAssetReserve(u128),
}

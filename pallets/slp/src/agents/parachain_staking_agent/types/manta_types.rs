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
use crate::{BalanceOf, Config, Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{traits::StaticLookup, RuntimeDebug};
use sp_std::{boxed::Box, vec::Vec};
use xcm::{opaque::v3::WeightLimit, VersionedMultiLocation};

#[derive(Encode, Decode, RuntimeDebug)]
pub enum MantaCall<T: Config> {
	#[codec(index = 10)]
	Balances(MantaBalancesCall<T>),
	#[codec(index = 34)]
	Xtokens(MantaXtokensCall<T>),
	#[codec(index = 40)]
	Utility(Box<MantaUtilityCall<Self>>),
	#[codec(index = 48)]
	ParachainStaking(MantaParachainStakingCall<T>),
}

impl<T: Config> MantaCall<T> {
	pub fn encode(&self) -> Vec<u8> {
		self.using_encoded(|x| x.to_vec())
	}
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum MantaBalancesCall<T: Config> {
	#[codec(index = 3)]
	TransferKeepAlive(<T::Lookup as StaticLookup>::Source, #[codec(compact)] BalanceOf<T>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum MantaUtilityCall<MantaCall> {
	#[codec(index = 1)]
	AsDerivative(u16, Box<MantaCall>),
	#[codec(index = 2)]
	BatchAll(Box<Vec<Box<MantaCall>>>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum MantaParachainStakingCall<T: Config> {
	#[codec(index = 18)]
	Delegate(
		T::AccountId,
		#[codec(compact)] BalanceOf<T>,
		#[codec(compact)] u32,
		#[codec(compact)] u32,
	),
	// schedule_revoke_delegation
	#[codec(index = 19)]
	ScheduleLeaveDelegators,
	// execute_delegation_request
	#[codec(index = 20)]
	ExecuteLeaveDelegators(T::AccountId, #[codec(compact)] u32),
	// cancel_delegation_request
	#[codec(index = 21)]
	CancelLeaveDelegators,
	#[codec(index = 22)]
	ScheduleRevokeDelegation(T::AccountId),
	#[codec(index = 23)]
	DelegatorBondMore(T::AccountId, #[codec(compact)] BalanceOf<T>),
	#[codec(index = 24)]
	ScheduleDelegatorBondLess(T::AccountId, #[codec(compact)] BalanceOf<T>),
	#[codec(index = 25)]
	ExecuteDelegationRequest(T::AccountId, T::AccountId),
	#[codec(index = 26)]
	CancelDelegationRequest(T::AccountId),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum MantaXtokensCall<T: Config> {
	#[codec(index = 0)]
	Transfer(MantaCurrencyId, BalanceOf<T>, Box<VersionedMultiLocation>, WeightLimit),
}

#[derive(Clone, Eq, Debug, PartialEq, Ord, PartialOrd, Encode, Decode, TypeInfo)]
pub enum MantaCurrencyId {
	/// assetId 1 is Manta native token
	MantaCurrency(u128),
}

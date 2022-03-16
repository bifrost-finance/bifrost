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

use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use scale_info::TypeInfo;
use sp_runtime::traits::StaticLookup;
use sp_std::{boxed::Box, vec::Vec};
use xcm::{VersionedMultiAssets, VersionedMultiLocation};

use crate::{BalanceOf, Config};

#[derive(Encode, Decode, RuntimeDebug)]
pub enum KusamaCall<T: Config> {
	#[codec(index = 4)]
	Balances(BalancesCall<T>),
	#[codec(index = 6)]
	Staking(StakingCall<T>),
	#[codec(index = 24)]
	Utility(Box<UtilityCall<Self>>),
	#[codec(index = 99)]
	Xcm(XcmCall),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum BalancesCall<T: Config> {
	#[codec(index = 3)]
	TransferKeepAlive(<T::Lookup as StaticLookup>::Source, #[codec(compact)] BalanceOf<T>),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum UtilityCall<KusamaCall> {
	#[codec(index = 1)]
	AsDerivative(u16, KusamaCall),
	#[codec(index = 2)]
	BatchAll(Vec<KusamaCall>),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum StakingCall<T: Config> {
	/// Kusama has the same account Id type as Bifrost.
	#[codec(index = 0)]
	Bond(
		<T::Lookup as StaticLookup>::Source,
		#[codec(compact)] BalanceOf<T>,
		RewardDestination<T::AccountId>,
	),
	#[codec(index = 1)]
	BondExtra(#[codec(compact)] BalanceOf<T>),
	#[codec(index = 2)]
	Unbond(#[codec(compact)] BalanceOf<T>),
	#[codec(index = 3)]
	WithdrawUnbonded(u32),
	#[codec(index = 5)]
	Nominate(Vec<<T::Lookup as StaticLookup>::Source>),
	#[codec(index = 6)]
	Chill,
	#[codec(index = 18)]
	PayoutStakers(T::AccountId, u32),
	#[codec(index = 19)]
	Rebond(#[codec(compact)] BalanceOf<T>),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum XcmCall {
	#[codec(index = 2)]
	ReserveTransferAssets(
		Box<VersionedMultiLocation>,
		Box<VersionedMultiLocation>,
		Box<VersionedMultiAssets>,
		u32,
	),
}

/// A destination account for payment.
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum RewardDestination<AccountId> {
	/// Pay into the stash account, increasing the amount at stake accordingly.
	Staked,
	/// Pay into the stash account, not increasing the amount at stake.
	Stash,
	/// Pay into the controller account.
	Controller,
	/// Pay into a specified account.
	Account(AccountId),
	/// Receive no reward.
	None,
}

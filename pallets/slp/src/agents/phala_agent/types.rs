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

use crate::agents::{BalancesCall, SystemCall, XcmCall};
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use scale_info::TypeInfo;
use sp_runtime::traits::StaticLookup;
use sp_std::{boxed::Box, vec::Vec};
use xcm::{VersionedMultiAssets, VersionedMultiLocation};
use xcm_interface::UtilityCall;

use crate::{BalanceOf, Config};

#[derive(Encode, Decode, RuntimeDebug)]
pub enum PhalaCall<T: Config> {
	#[codec(index = 0)]
	System(SystemCall),
	#[codec(index = 3)]
	Utility(Box<PhalaUtilityCall<Self>>),
	#[codec(index = 33)]
	Xcm(Box<XcmCall>),
	#[codec(index = 40)]
	Balances(BalancesCall<T>),
	#[codec(index = 94)]
	PhalaVault(VaultCall<T>),
	#[codec(index = 95)]
	PhalaWrappedBalances(WrappedBalancesCall<T>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum VaultCall<T: Config> {
	#[codec(index = 4)]
	CheckAndMaybeForceWithdraw(u64),
	#[codec(index = 5)]
	Contribute(u64, BalanceOf<T>),
	#[codec(index = 6)]
	Withdraw(u64, BalanceOf<T>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum WrappedBalancesCall<T: Config> {
	#[codec(index = 0)]
	Wrap(BalanceOf<T>),
	#[codec(index = 1)]
	UnwrapAll,
	#[codec(index = 2)]
	Unwrap(BalanceOf<T>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum PhalaUtilityCall<PhalaCall> {
	#[codec(index = 1)]
	AsDerivative(u16, Box<PhalaCall>),
	#[codec(index = 2)]
	BatchAll(Box<Vec<Box<PhalaCall>>>),
}

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

use crate::{agents::BalancesCall, BalanceOf, Config, MultiLocation};
use parity_scale_codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::{boxed::Box, vec::Vec};
use xcm::{opaque::v3::MultiAsset, v3::Weight as XCMWeight};

#[derive(Encode, Decode, RuntimeDebug)]
pub enum PhalaCall<T: Config> {
	#[codec(index = 0)]
	System(PhalaSystemCall),
	#[codec(index = 3)]
	Utility(Box<PhalaUtilityCall<Self>>),
	#[codec(index = 40)]
	Balances(BalancesCall<T>),
	#[codec(index = 82)]
	Xtransfer(XtransferCall),
	#[codec(index = 93)]
	PhalaStakePoolv2(StakePoolv2Call<T>),
	#[codec(index = 94)]
	PhalaVault(VaultCall<T>),
	#[codec(index = 95)]
	PhalaWrappedBalances(WrappedBalancesCall<T>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum XtransferCall {
	#[codec(index = 0)]
	Transfer(Box<MultiAsset>, Box<MultiLocation>, Option<XCMWeight>),
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

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum PhalaSystemCall {
	#[codec(index = 8)]
	RemarkWithEvent(Box<Vec<u8>>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum StakePoolv2Call<T: Config> {
	#[codec(index = 8)]
	CheckAndMaybeForceWithdraw(u64),
	#[codec(index = 9)]
	Contribute(u64, BalanceOf<T>, Option<u64>),
	#[codec(index = 10)]
	Withdraw(u64, BalanceOf<T>, Option<u64>),
}

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

use frame_support::sp_runtime::MultiSignature;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::{boxed::Box, vec::Vec};
use xcm::{v4::WeightLimit, VersionedAssets, VersionedLocation};

use crate::ChainId;

#[derive(Encode, Decode, RuntimeDebug)]
pub enum UtilityCall<RelayChainCall> {
	#[codec(index = 1)]
	AsDerivative(u16, RelayChainCall),
	#[codec(index = 2)]
	BatchAll(Vec<RelayChainCall>),
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum StakingCall {
	#[codec(index = 3)]
	WithdrawUnbonded(u32),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum PolkadotXcmCall {
	#[codec(index = 2)]
	ReserveTransferAssets(
		Box<VersionedLocation>,
		Box<VersionedLocation>,
		Box<VersionedAssets>,
		u32,
	),
	#[codec(index = 8)]
	LimitedReserveTransferAssets(
		Box<VersionedLocation>,
		Box<VersionedLocation>,
		Box<VersionedAssets>,
		u32,
		WeightLimit,
	),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum SystemCall {
	#[codec(index = 7)]
	RemarkWithEvent(Vec<u8>),
}

pub mod kusama {

	pub use crate::calls::*;

	#[derive(Encode, Decode, RuntimeDebug)]
	pub enum RelaychainCall<BalanceOf, AccountIdOf, BlockNumberOf> {
		#[codec(index = 73)]
		Crowdloan(ContributeCall<BalanceOf, AccountIdOf>),
		#[codec(index = 30)]
		Proxy(ProxyCall<AccountIdOf, BlockNumberOf>),
	}

	#[derive(Encode, Decode, RuntimeDebug)]
	pub enum AssetHubCall {
		#[codec(index = 31)]
		PolkadotXcm(PolkadotXcmCall),
	}
}

pub mod polkadot {
	pub use crate::calls::*;

	#[derive(Encode, Decode, RuntimeDebug)]
	pub enum RelaychainCall<BalanceOf, AccountIdOf, BlockNumberOf> {
		#[codec(index = 73)]
		Crowdloan(ContributeCall<BalanceOf, AccountIdOf>),
		#[codec(index = 29)]
		Proxy(ProxyCall<AccountIdOf, BlockNumberOf>),
	}

	#[derive(Encode, Decode, RuntimeDebug)]
	pub enum AssetHubCall {
		#[codec(index = 31)]
		PolkadotXcm(PolkadotXcmCall),
	}
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum ContributeCall<BalanceOf, AccountIdOf> {
	#[codec(index = 1)]
	Contribute(Contribution<BalanceOf>),
	#[codec(index = 2)]
	Withdraw(Withdraw<AccountIdOf>),
	#[codec(index = 6)]
	AddMemo(AddMemo),
}

#[derive(PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Contribution<BalanceOf> {
	#[codec(compact)]
	pub index: ChainId,
	#[codec(compact)]
	pub value: BalanceOf,
	pub signature: Option<MultiSignature>,
}

#[derive(PartialEq, Encode, Decode, RuntimeDebug)]
pub struct Withdraw<AccountIdOf> {
	pub who: AccountIdOf,
	#[codec(compact)]
	pub index: ChainId,
}

#[derive(PartialEq, Encode, Decode, RuntimeDebug)]
pub struct AddMemo {
	pub index: ChainId,
	pub memo: Vec<u8>,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, RuntimeDebug)]
pub enum ProxyType {
	Any,
	NonTransfer,
	Governance,
	Staking,
	IdentityJudgement,
	CancelProxy,
}

#[derive(Encode, Decode, RuntimeDebug)]
pub enum ProxyCall<AccountIdOf, BlockNumberOf> {
	#[codec(index = 1)]
	Add(AddProxy<AccountIdOf, BlockNumberOf>),
	#[codec(index = 2)]
	Remove(RemoveProxy<AccountIdOf, BlockNumberOf>),
}

#[derive(PartialEq, Encode, Decode, RuntimeDebug)]
pub struct AddProxy<AccountIdOf, BlockNumberOf> {
	pub delegate: AccountIdOf,
	pub proxy_type: ProxyType,
	pub delay: BlockNumberOf,
}

#[derive(PartialEq, Encode, Decode, RuntimeDebug)]
pub struct RemoveProxy<AccountIdOf, BlockNumberOf> {
	pub delegate: AccountIdOf,
	pub proxy_type: ProxyType,
	pub delay: BlockNumberOf,
}

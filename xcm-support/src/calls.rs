// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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
use node_primitives::ParaId;
use sp_runtime::MultiSignature;

#[derive(Encode, Decode)]
pub enum CrowdloanContributeCall<BalanceOf> {
	#[codec(index = 73)]
	CrowdloanContribute(ContributeCall<BalanceOf>),
}

#[derive(Encode, Decode)]
pub enum CrowdloanWithdrawCall<AccountIdOf> {
	#[codec(index = 73)]
	CrowdloanWithdraw(WithdrawCall<AccountIdOf>),
}

#[derive(Debug, PartialEq, Encode, Decode)]
pub struct Contribution<BalanceOf> {
	#[codec(compact)]
	pub index: ParaId,
	#[codec(compact)]
	pub value: BalanceOf,
	pub signature: Option<MultiSignature>,
}

#[derive(Encode, Decode)]
pub enum ContributeCall<BalanceOf> {
	#[codec(index = 1)]
	Contribute(Contribution<BalanceOf>),
}

#[derive(Debug, PartialEq, Encode, Decode)]
pub struct Withdraw<AccountIdOf> {
	pub who: AccountIdOf,
	#[codec(compact)]
	pub index: ParaId,
}

#[derive(Encode, Decode)]
pub enum WithdrawCall<AccountIdOf> {
	#[codec(index = 2)]
	Withdraw(Withdraw<AccountIdOf>),
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug)]
pub enum ProxyType {
	Any,
	NonTransfer,
	Governance,
	Staking,
	IdentityJudgement,
	CancelProxy,
}

#[derive(Encode, Decode)]
pub enum ProxyAddCall<BalanceOf, BlockNumberOf> {
	#[codec(index = 30)]
	ProxyAdd(AddProxyCall<BalanceOf, BlockNumberOf>),
}

#[derive(Encode, Decode)]
pub enum AddProxyCall<AccountIdOf, BlockNumberOf> {
	#[codec(index = 1)]
	Add(AddProxy<AccountIdOf, BlockNumberOf>),
}

#[derive(Debug, PartialEq, Encode, Decode)]
pub struct AddProxy<AccountIdOf, BlockNumberOf> {
	pub delegate: AccountIdOf,
	pub proxy_type: ProxyType,
	pub delay: BlockNumberOf,
}

#[derive(Encode, Decode)]
pub enum ProxyRemoveCall<BalanceOf, BlockNumberOf> {
	#[codec(index = 30)]
	ProxyRemove(RemoveProxyCall<BalanceOf, BlockNumberOf>),
}

#[derive(Encode, Decode)]
pub enum RemoveProxyCall<AccountIdOf, BlockNumberOf> {
	#[codec(index = 1)]
	Remove(RemoveProxy<AccountIdOf, BlockNumberOf>),
}

#[derive(Debug, PartialEq, Encode, Decode)]
pub struct RemoveProxy<AccountIdOf, BlockNumberOf> {
	pub delegate: AccountIdOf,
	pub proxy_type: ProxyType,
	pub delay: BlockNumberOf,
}

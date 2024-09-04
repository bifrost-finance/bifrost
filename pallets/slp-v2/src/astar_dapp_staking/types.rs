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

use crate::{common::types::Delegator, Config};
use bifrost_primitives::{Balance, TimeUnit};
use frame_support::{
	pallet_prelude::{Decode, Encode, MaxEncodedLen, TypeInfo},
	BoundedVec,
};
use sp_core::{ConstU32, H160};
use sp_runtime::Saturating;

/// Multi-VM pointer to smart contract instance.
#[derive(PartialEq, Eq, Clone, Encode, Decode, Debug, Copy, MaxEncodedLen, TypeInfo)]
pub enum AstarValidator<AccountId> {
	/// EVM smart contract instance.
	Evm(H160),
	/// Wasm smart contract instance.
	Wasm(AccountId),
}

/// Dapp staking extrinsic call.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum DappStaking<AccountId> {
	#[codec(index = 7)]
	Lock(#[codec(compact)] Balance),
	#[codec(index = 8)]
	Unlock(#[codec(compact)] Balance),
	#[codec(index = 9)]
	ClaimUnlocked,
	#[codec(index = 10)]
	RelockUnlocking,
	#[codec(index = 11)]
	Stake(AstarValidator<AccountId>, #[codec(compact)] Balance),
	#[codec(index = 12)]
	Unstake(AstarValidator<AccountId>, #[codec(compact)] Balance),
	#[codec(index = 13)]
	ClaimStakerRewards,
	#[codec(index = 14)]
	ClaimBonusReward(AstarValidator<AccountId>),
}

/// Astar extrinsic call.
#[derive(Encode, Decode, Debug, Clone)]
pub enum AstarCall<T: Config> {
	#[codec(index = 34)]
	DappStaking(DappStaking<T::AccountId>),
}

/// Astar unlocking record.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub struct AstarUnlockingRecord {
	pub amount: Balance,
	pub unlock_time: TimeUnit,
}

/// Astar dapp staking ledger.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Debug, Default, PartialEq, Eq, TypeInfo)]
pub struct AstarDappStakingLedger {
	/// How much active locked amount an account has. This can be used for staking.
	#[codec(compact)]
	pub locked: Balance,
	/// Vector of all the unlocking chunks. This is also considered _locked_ but cannot be used for
	/// staking.
	pub unlocking: BoundedVec<AstarUnlockingRecord, ConstU32<8>>,
}

impl AstarDappStakingLedger {
	/// Adds the specified amount to the total locked amount.
	pub fn add_lock_amount(&mut self, amount: Balance) {
		self.locked.saturating_accrue(amount);
	}

	/// Subtracts the specified amount of the total locked amount.
	pub fn subtract_lock_amount(&mut self, amount: Balance) {
		self.locked.saturating_reduce(amount);
	}
}

/// PendingStatus in slp protocol.
#[derive(Encode, Decode, MaxEncodedLen, Clone, Copy, Debug, PartialEq, Eq, TypeInfo)]
pub enum AstarDappStakingPendingStatus<AccountId> {
	Lock(Delegator<AccountId>, Balance),
	UnLock(Delegator<AccountId>, Balance),
	ClaimUnlocked(Delegator<AccountId>),
}

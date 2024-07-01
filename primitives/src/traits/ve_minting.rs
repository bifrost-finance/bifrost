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

// Ensure we're `no_std` when compiling for Wasm.
use crate::{DispatchResult, DispatchError, PoolId, Point, U256, Vec, 
	IncentiveConfig, FixedU128, Decode, RuntimeDebug, MaxEncodedLen,
	TypeInfo, Encode,
};

pub trait VeMintingInterface<AccountId, CurrencyId, Balance, BlockNumber> {
	fn deposit_for(_who: &AccountId, position: u128, value: Balance) -> DispatchResult;
	fn withdraw_inner(who: &AccountId, position: u128) -> DispatchResult;
	fn balance_of(addr: &AccountId, time: Option<BlockNumber>) -> Result<Balance, DispatchError>;
	fn total_supply(t: BlockNumber) -> Result<Balance, DispatchError>;
	fn supply_at(
		point: Point<Balance, BlockNumber>,
		t: BlockNumber,
	) -> Result<Balance, DispatchError>;
	fn find_block_epoch(_block: BlockNumber, max_epoch: U256) -> U256;
	fn create_lock_inner(
		who: &AccountId,
		_value: Balance,
		_unlock_time: BlockNumber,
	) -> DispatchResult; // Deposit `_value` BNC for `addr` and lock until `_unlock_time`
	fn increase_amount_inner(who: &AccountId, position: u128, value: Balance) -> DispatchResult; // Deposit `_value` additional BNC for `addr` without modifying the unlock time
	fn increase_unlock_time_inner(
		who: &AccountId,
		position: u128,
		_unlock_time: BlockNumber,
	) -> DispatchResult; // Extend the unlock time for `addr` to `_unlock_time`
	fn auto_notify_reward(
		pool_id: PoolId,
		n: BlockNumber,
		rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult;
	fn update_reward(
		pool_id: PoolId,
		addr: Option<&AccountId>,
		share_info: Option<(Balance, Balance)>,
	) -> DispatchResult;
	fn get_rewards(
		pool_id: PoolId,
		addr: &AccountId,
		share_info: Option<(Balance, Balance)>,
	) -> DispatchResult;
	fn set_incentive(
		pool_id: PoolId,
		rewards_duration: Option<BlockNumber>,
		controller: Option<AccountId>,
	);
	fn add_reward(
		addr: &AccountId,
		conf: &mut IncentiveConfig<CurrencyId, Balance, BlockNumber, AccountId>,
		rewards: &Vec<(CurrencyId, Balance)>,
		remaining: Balance,
	) -> DispatchResult;
	fn notify_reward(
		pool_id: PoolId,
		addr: &Option<AccountId>,
		rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult;
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct UserMarkupInfo {
	// pub old_locked: LockedBalance<Balance, BlockNumber>,
	pub old_markup_coefficient: FixedU128,
	pub markup_coefficient: FixedU128,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct LockedToken<Balance, BlockNumber> {
	// pub asset_id: CurrencyId,
	pub amount: Balance,
	pub markup_coefficient: FixedU128,
	pub refresh_block: BlockNumber,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct MarkupCoefficientInfo<BlockNumber> {
	pub markup_coefficient: FixedU128,
	pub hardcap: FixedU128,
	pub update_block: BlockNumber,
}

pub trait MarkupInfo<AccountId> {
	fn update_markup_info(
		addr: &AccountId,
		new_markup_coefficient: FixedU128,
		user_markup_info: &mut UserMarkupInfo,
	);
}
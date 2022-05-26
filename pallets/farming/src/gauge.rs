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

use codec::HasCompact;
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{Zero, *},
	ArithmeticError, Permill, RuntimeDebug, SaturatedConversion,
};
use sp_std::prelude::*;

use crate::*;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct GaugeInfo<BalanceOf: HasCompact, BlockNumberFor, AccountIdOf> {
	pub who: Option<AccountIdOf>,
	pub gauge_amount: BalanceOf,
	pub total_time_factor: u128,
	pub latest_time_factor: u128,
	pub claimed_time_factor: u128,
	pub gauge_start_block: BlockNumberFor,
	pub gauge_stop_block: BlockNumberFor,
	pub gauge_last_block: BlockNumberFor,
	pub last_claim_block: BlockNumberFor,
}

impl<BalanceOf, BlockNumberFor, AccountIdOf> Default
	for GaugeInfo<BalanceOf, BlockNumberFor, AccountIdOf>
where
	BalanceOf: Default + HasCompact,
	BlockNumberFor: Default,
{
	fn default() -> Self {
		Self {
			who: None,
			gauge_amount: Default::default(),
			total_time_factor: Default::default(),
			latest_time_factor: Default::default(),
			claimed_time_factor: Default::default(),
			gauge_start_block: Default::default(),
			gauge_stop_block: Default::default(),
			gauge_last_block: Default::default(),
			last_claim_block: Default::default(),
		}
	}
}

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct GaugePoolInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, BlockNumberFor> {
	pub pid: PoolId,
	pub token: CurrencyIdOf,
	pub rewards: BTreeMap<CurrencyIdOf, (BalanceOf, BalanceOf)>,
	pub coefficient: Permill,
	pub max_block: BlockNumberFor,
	pub gauge_amount: BalanceOf,
	pub total_time_factor: u128,
	pub gauge_state: GaugeState,
	pub gauge_last_block: BlockNumberFor,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum GaugeState {
	Unbond,
	Bonded,
}

impl<BalanceOf, CurrencyIdOf, BlockNumberFor> Default
	for GaugePoolInfo<BalanceOf, CurrencyIdOf, BlockNumberFor>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord + Default,
	BlockNumberFor: Default,
{
	fn default() -> Self {
		Self {
			pid: Default::default(),
			token: Default::default(),
			rewards: BTreeMap::new(),
			coefficient: Default::default(),
			max_block: Default::default(),
			gauge_amount: Default::default(),
			total_time_factor: Default::default(),
			gauge_last_block: Default::default(),
			gauge_state: GaugeState::Unbond,
		}
	}
}

impl<BalanceOf, CurrencyIdOf, BlockNumberFor> GaugePoolInfo<BalanceOf, CurrencyIdOf, BlockNumberFor>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord + Default,
	BlockNumberFor: Clone,
{
	pub fn new(
		pid: PoolId,
		token: CurrencyIdOf,
		coefficient: Permill,
		max_block: BlockNumberFor,
		current_block_number: BlockNumberFor,
	) -> Self {
		Self {
			pid,
			token,
			rewards: BTreeMap::new(),
			coefficient,
			max_block,
			gauge_amount: Default::default(),
			total_time_factor: Default::default(),
			gauge_last_block: current_block_number,
			gauge_state: GaugeState::Bonded,
		}
	}
}

impl<T: Config> Pallet<T>
where
	BlockNumberFor<T>: Into<u128> + Into<BalanceOf<T>>,
	BalanceOf<T>: Into<u128>,
{
	pub fn create_gauge_pool(
		pid: PoolId,
		pool_info: &mut PoolInfo<BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>, BlockNumberFor<T>>,
		gauge_token: CurrencyIdOf<T>,
		coefficient: Permill,
		max_block: BlockNumberFor<T>,
	) -> DispatchResult {
		let gid = Self::gauge_pool_next_id();
		pool_info.gauge = Some(gid);
		let current_block_number = frame_system::Pallet::<T>::block_number();
		let gauge_pool_info =
			GaugePoolInfo::new(pid, gauge_token, coefficient, max_block, current_block_number);

		GaugePoolInfos::<T>::insert(gid, &gauge_pool_info);
		GaugePoolNextId::<T>::mutate(|id| -> DispatchResult {
			*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}

	pub fn gauge_add(
		who: &AccountIdOf<T>,
		pid: PoolId,
		gid: PoolId,
		gauge_value: BalanceOf<T>,
		gauge_block: BlockNumberFor<T>,
	) -> DispatchResult {
		let current_block_number = frame_system::Pallet::<T>::block_number();
		GaugePoolInfos::<T>::mutate(gid, |gauge_pool_info| -> DispatchResult {
			gauge_pool_info.gauge_last_block = current_block_number;
			gauge_pool_info.gauge_amount = gauge_pool_info
				.gauge_amount
				.checked_add(&gauge_value)
				.ok_or(ArithmeticError::Overflow)?;
			let incease_total_time_factor =
				GaugeInfos::<T>::mutate(gid, who, |gauge_info| -> Result<u128, DispatchError> {
					ensure!(
						gauge_info.gauge_stop_block >= current_block_number ||
							gauge_info.gauge_stop_block == Default::default(),
						Error::<T>::LastGaugeNotClaim
					);

					ensure!(
						gauge_pool_info.max_block >=
							gauge_info.gauge_stop_block - gauge_info.gauge_start_block +
								gauge_block,
						Error::<T>::GaugeMaxBlockOverflow
					);

					let incease_total_time_factor = if gauge_info.gauge_amount.is_zero() {
						gauge_info.gauge_stop_block = current_block_number;
						gauge_info.gauge_start_block = current_block_number;
						gauge_info.last_claim_block = current_block_number;
						gauge_info.total_time_factor = gauge_block
							.saturated_into::<u128>()
							.checked_mul(gauge_value.into())
							.ok_or(ArithmeticError::Overflow)?;
						gauge_info.total_time_factor
					} else {
						let time_factor_a = gauge_value
							.saturated_into::<u128>()
							.checked_mul(
								(gauge_info.gauge_stop_block - current_block_number).into(),
							)
							.ok_or(ArithmeticError::Overflow)?;
						let time_factor_b = gauge_block
							.saturated_into::<u128>()
							.checked_mul((gauge_value + gauge_info.gauge_amount).into())
							.ok_or(ArithmeticError::Overflow)?;
						let incease_total_time_factor = time_factor_a + time_factor_b;
						gauge_info.total_time_factor = gauge_info
							.total_time_factor
							.checked_add(incease_total_time_factor)
							.ok_or(ArithmeticError::Overflow)?;
						// latest_time_factor only increases in not first gauge_deposit
						let increase_latest_time_factor = gauge_info
							.gauge_amount
							.saturated_into::<u128>()
							.checked_mul(
								(current_block_number - gauge_info.gauge_last_block).into(),
							)
							.ok_or(ArithmeticError::Overflow)?;
						gauge_info.latest_time_factor = gauge_info
							.latest_time_factor
							.checked_add(increase_latest_time_factor)
							.ok_or(ArithmeticError::Overflow)?;
						incease_total_time_factor
					};

					gauge_info.gauge_last_block = current_block_number;
					gauge_info.gauge_amount = gauge_info
						.gauge_amount
						.checked_add(&gauge_value)
						.ok_or(ArithmeticError::Overflow)?;
					gauge_info.gauge_stop_block = gauge_info.gauge_stop_block + gauge_block;
					Ok(incease_total_time_factor)
				})?;
			gauge_pool_info.total_time_factor = gauge_pool_info
				.total_time_factor
				.checked_add(incease_total_time_factor)
				.ok_or(ArithmeticError::Overflow)?;
			let pool_info = Self::pool_infos(&pid);
			T::MultiCurrency::transfer(
				gauge_pool_info.token,
				who,
				&pool_info.keeper.ok_or(Error::<T>::KeeperNotExist)?,
				gauge_value,
			)?;
			Ok(())
		})?;
		Ok(())
	}

	pub fn gauge_claim_inner(who: &AccountIdOf<T>, gid: PoolId) -> DispatchResult {
		if !GaugeInfos::<T>::contains_key(gid, who) {
			return Ok(());
		}
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		let mut gauge_pool_info = GaugePoolInfos::<T>::get(gid);
		let pool_info = PoolInfos::<T>::get(gauge_pool_info.pid);
		GaugeInfos::<T>::mutate(gid, who, |gauge_info| -> DispatchResult {
			ensure!(gauge_info.gauge_start_block <= current_block_number, Error::<T>::CanNotClaim);
			let start_block = if current_block_number > gauge_info.gauge_stop_block {
				gauge_info.gauge_stop_block
			} else {
				current_block_number
			};

			let latest_claimed_time_factor = gauge_info.latest_time_factor +
				gauge_info
					.gauge_amount
					.saturated_into::<u128>()
					.checked_mul((start_block - gauge_info.gauge_last_block).into())
					.ok_or(ArithmeticError::Overflow)?;
			let gauge_rate = Permill::from_rational(
				latest_claimed_time_factor - gauge_info.claimed_time_factor,
				gauge_pool_info.total_time_factor,
			);
			gauge_pool_info.rewards.iter_mut().try_for_each(
				|(reward_currency, (reward_amount, withdrawn_reward))| -> DispatchResult {
					let reward = reward_amount
						.checked_sub(&withdrawn_reward)
						.ok_or(ArithmeticError::Overflow)?;
					let reward_to_claim = gauge_rate * reward;
					match pool_info.keeper {
						None => return Err(Error::<T>::PoolKeeperNotExist.into()),
						Some(ref keeper) => {
							*withdrawn_reward = withdrawn_reward
								.checked_add(&reward_to_claim)
								.ok_or(ArithmeticError::Overflow)?;
							T::MultiCurrency::transfer(
								*reward_currency,
								&keeper,
								&who,
								reward_to_claim,
							)?
						},
					};
					Ok(())
				},
			)?;
			gauge_info.last_claim_block = current_block_number;
			gauge_info.claimed_time_factor = latest_claimed_time_factor;
			let _ = if gauge_info.gauge_stop_block <= current_block_number {
				if let Some(ref keeper) = pool_info.keeper {
					T::MultiCurrency::transfer(
						gauge_pool_info.token,
						&keeper,
						&who,
						gauge_info.gauge_amount,
					)?;
					GaugeInfos::<T>::remove(gid, who);
					gauge_pool_info.total_time_factor = gauge_pool_info
						.total_time_factor
						.checked_sub(gauge_info.total_time_factor)
						.ok_or(ArithmeticError::Overflow)?;
					// GaugePoolInfos::<T>::insert(gid, gauge_pool_info);
				};
			};
			GaugePoolInfos::<T>::insert(gid, gauge_pool_info);
			Ok(())
		})?;
		Ok(())
	}
}

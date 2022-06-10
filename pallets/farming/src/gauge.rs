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
use sp_core::U256;
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

impl<BalanceOf, BlockNumberFor, AccountIdOf> GaugeInfo<BalanceOf, BlockNumberFor, AccountIdOf>
where
	BalanceOf: Default + HasCompact,
	BlockNumberFor: Default,
{
	fn new() -> Self {
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
pub struct GaugePoolInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, AccountIdOf, BlockNumberFor> {
	pub pid: PoolId,
	pub token: CurrencyIdOf,
	pub keeper: AccountIdOf,
	pub reward_issuer: AccountIdOf,
	pub rewards: BTreeMap<CurrencyIdOf, (BalanceOf, BalanceOf, BalanceOf)>,
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

impl<BalanceOf, CurrencyIdOf, AccountIdOf, BlockNumberFor>
	GaugePoolInfo<BalanceOf, CurrencyIdOf, AccountIdOf, BlockNumberFor>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord + Default,
	BlockNumberFor: Clone,
{
	pub fn new(
		pid: PoolId,
		token: CurrencyIdOf,
		keeper: AccountIdOf,
		reward_issuer: AccountIdOf,
		coefficient: Permill,
		max_block: BlockNumberFor,
		current_block_number: BlockNumberFor,
	) -> Self {
		Self {
			pid,
			token,
			keeper,
			reward_issuer,
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
	BlockNumberFor<T>: AtLeast32BitUnsigned + Copy,
	BalanceOf<T>: AtLeast32BitUnsigned + Copy,
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
		let gauge_pool_info = GaugePoolInfo::new(
			pid,
			gauge_token,
			pool_info.keeper.clone(),
			pool_info.reward_issuer.clone(),
			coefficient,
			max_block,
			current_block_number,
		);

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
		GaugePoolInfos::<T>::mutate(gid, |gauge_pool_info_old| -> DispatchResult {
			if let Some(mut gauge_pool_info) = gauge_pool_info_old.take() {
				gauge_pool_info.gauge_last_block = current_block_number;
				gauge_pool_info.gauge_amount = gauge_pool_info
					.gauge_amount
					.checked_add(&gauge_value)
					.ok_or(ArithmeticError::Overflow)?;
				let mut gauge_info =
					GaugeInfos::<T>::get(gid, who).unwrap_or_else(|| GaugeInfo::new());

				ensure!(
					gauge_info.gauge_stop_block >= current_block_number ||
						gauge_info.gauge_stop_block == Default::default(),
					Error::<T>::LastGaugeNotClaim
				);

				ensure!(
					gauge_pool_info.max_block >=
						gauge_info.gauge_stop_block - gauge_info.gauge_start_block + gauge_block,
					Error::<T>::GaugeMaxBlockOverflow
				);

				let incease_total_time_factor = if gauge_info.gauge_amount.is_zero() {
					gauge_info.gauge_stop_block = current_block_number;
					gauge_info.gauge_start_block = current_block_number;
					gauge_info.last_claim_block = current_block_number;
					gauge_info.total_time_factor = gauge_block
						.saturated_into::<u128>()
						.checked_mul(gauge_value.saturated_into::<u128>())
						.ok_or(ArithmeticError::Overflow)?;
					gauge_info.total_time_factor
				} else {
					let time_factor_a = gauge_value
						.saturated_into::<u128>()
						.checked_mul(
							(gauge_info.gauge_stop_block - current_block_number)
								.saturated_into::<u128>(),
						)
						.ok_or(ArithmeticError::Overflow)?;
					let time_factor_b = gauge_block
						.saturated_into::<u128>()
						.checked_mul(
							(gauge_value + gauge_info.gauge_amount).saturated_into::<u128>(),
						)
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
							(current_block_number - gauge_info.gauge_last_block)
								.saturated_into::<u128>(),
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

				gauge_pool_info.total_time_factor = gauge_pool_info
					.total_time_factor
					.checked_add(incease_total_time_factor)
					.ok_or(ArithmeticError::Overflow)?;
				T::MultiCurrency::transfer(
					gauge_pool_info.token,
					who,
					&gauge_pool_info.keeper,
					gauge_value,
				)?;
				GaugeInfos::<T>::insert(gid, who, gauge_info);
				*gauge_pool_info_old = Some(gauge_pool_info);
				Ok(())
			} else {
				Err(Error::<T>::GaugePoolDoesNotExist)?
			}
		})?;
		Ok(())
	}

	pub fn gauge_claim_inner(who: &AccountIdOf<T>, gid: PoolId) -> DispatchResult {
		if !GaugeInfos::<T>::contains_key(gid, who) {
			return Ok(());
		}
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		let mut gauge_pool_info =
			GaugePoolInfos::<T>::get(gid).ok_or(Error::<T>::GaugePoolDoesNotExist)?;
		let pool_info =
			PoolInfos::<T>::get(gauge_pool_info.pid).ok_or(Error::<T>::PoolDoesNotExist)?;
		GaugeInfos::<T>::mutate_exists(gid, who, |maybe_gauge_info| -> DispatchResult {
			if let Some(mut gauge_info) = maybe_gauge_info.take() {
				ensure!(
					gauge_info.gauge_start_block <= current_block_number,
					Error::<T>::CanNotClaim
				);
				let start_block = if current_block_number > gauge_info.gauge_stop_block {
					gauge_info.gauge_stop_block
				} else {
					current_block_number
				};

				let latest_claimed_time_factor = gauge_info.latest_time_factor +
					gauge_info
						.gauge_amount
						.saturated_into::<u128>()
						.checked_mul(
							(start_block - gauge_info.gauge_last_block).saturated_into::<u128>(),
						)
						.ok_or(ArithmeticError::Overflow)?;
				let gauge_rate = Permill::from_rational(
					latest_claimed_time_factor - gauge_info.claimed_time_factor,
					gauge_pool_info.total_time_factor,
				);
				let total_shares =
					U256::from(pool_info.total_shares.to_owned().saturated_into::<u128>());
				let share_info = SharesAndWithdrawnRewards::<T>::get(gauge_pool_info.pid, who)
					.ok_or(Error::<T>::ShareInfoNotExists)?;
				gauge_pool_info.rewards.iter_mut().try_for_each(
					|(
						reward_currency,
						(reward_amount, total_gauged_reward, total_withdrawn_reward),
					)|
					 -> DispatchResult {
						let reward = reward_amount
							.checked_sub(&total_gauged_reward)
							.ok_or(ArithmeticError::Overflow)?;
						// gauge_reward = gauge rate * gauge coefficient * existing rewards in the
						// gauge pool
						let gauge_reward = gauge_rate * reward;
						// reward_to_claim = farming rate * gauge rate * gauge coefficient *
						// existing rewards in the gauge pool
						let reward_to_claim: BalanceOf<T> =
							U256::from(share_info.share.to_owned().saturated_into::<u128>())
								.saturating_mul(U256::from(
									gauge_reward.to_owned().saturated_into::<u128>(),
								))
								.checked_div(total_shares)
								.unwrap_or_default()
								.as_u128()
								.unique_saturated_into();
						*total_gauged_reward = total_gauged_reward
							.checked_add(&gauge_reward)
							.ok_or(ArithmeticError::Overflow)?;
						*total_withdrawn_reward = total_withdrawn_reward
							.checked_add(&reward_to_claim)
							.ok_or(ArithmeticError::Overflow)?;
						T::MultiCurrency::transfer(
							*reward_currency,
							&gauge_pool_info.reward_issuer,
							&who,
							reward_to_claim,
						)
					},
				)?;
				gauge_info.last_claim_block = current_block_number;
				gauge_info.claimed_time_factor = latest_claimed_time_factor;
				if gauge_info.gauge_stop_block <= current_block_number {
					T::MultiCurrency::transfer(
						gauge_pool_info.token,
						&gauge_pool_info.keeper,
						&who,
						gauge_info.gauge_amount,
					)?;
					gauge_pool_info.total_time_factor = gauge_pool_info
						.total_time_factor
						.checked_sub(gauge_info.total_time_factor)
						.ok_or(ArithmeticError::Overflow)?;
				} else {
					*maybe_gauge_info = Some(gauge_info);
				};
				GaugePoolInfos::<T>::insert(gid, gauge_pool_info);
			}
			Ok(())
		})?;
		Ok(())
	}

	pub fn get_farming_rewards(
		who: &T::AccountId,
		pid: PoolId,
	) -> Result<Vec<(CurrencyId, BalanceOf<T>)>, DispatchError> {
		let share_info =
			SharesAndWithdrawnRewards::<T>::get(pid, who).ok_or(Error::<T>::ShareInfoNotExists)?;
		let pool_info = PoolInfos::<T>::get(pid).ok_or(Error::<T>::PoolDoesNotExist)?;
		let total_shares = U256::from(pool_info.total_shares.to_owned().saturated_into::<u128>());
		let mut result_vec = Vec::<(CurrencyId, BalanceOf<T>)>::new();

		pool_info.rewards.iter().try_for_each(
			|(reward_currency, (total_reward, total_withdrawn_reward))| -> DispatchResult {
				let withdrawn_reward =
					share_info.withdrawn_rewards.get(reward_currency).copied().unwrap_or_default();

				let total_reward_proportion: BalanceOf<T> =
					U256::from(share_info.share.to_owned().saturated_into::<u128>())
						.saturating_mul(U256::from(
							total_reward.to_owned().saturated_into::<u128>(),
						))
						.checked_div(total_shares)
						.unwrap_or_default()
						.as_u128()
						.unique_saturated_into();

				let reward_to_withdraw = total_reward_proportion
					.saturating_sub(withdrawn_reward)
					.min(total_reward.saturating_sub(*total_withdrawn_reward));

				if reward_to_withdraw.is_zero() {
					return Ok(());
				};

				result_vec.push((*reward_currency, reward_to_withdraw));
				Ok(())
			},
		)?;
		Ok(result_vec)
	}

	pub fn get_gauge_rewards(
		who: &T::AccountId,
		pid: PoolId,
	) -> Result<Vec<(CurrencyId, BalanceOf<T>)>, DispatchError> {
		let pool_info = PoolInfos::<T>::get(pid).ok_or(Error::<T>::PoolDoesNotExist)?;
		let mut result_vec = Vec::<(CurrencyId, BalanceOf<T>)>::new();

		match pool_info.gauge {
			None => (),
			Some(gid) => {
				let current_block_number: BlockNumberFor<T> =
					frame_system::Pallet::<T>::block_number();
				let gauge_pool_info =
					GaugePoolInfos::<T>::get(gid).ok_or(Error::<T>::GaugePoolDoesNotExist)?;
				let gauge_info =
					GaugeInfos::<T>::get(gid, who).ok_or(Error::<T>::GaugePoolDoesNotExist)?;
				let start_block = if current_block_number > gauge_info.gauge_stop_block {
					gauge_info.gauge_stop_block
				} else {
					current_block_number
				};

				let latest_claimed_time_factor = gauge_info.latest_time_factor +
					gauge_info
						.gauge_amount
						.saturated_into::<u128>()
						.checked_mul(
							(start_block - gauge_info.gauge_last_block).saturated_into::<u128>(),
						)
						.ok_or(ArithmeticError::Overflow)?;
				let gauge_rate = Permill::from_rational(
					latest_claimed_time_factor - gauge_info.claimed_time_factor,
					gauge_pool_info.total_time_factor,
				);
				let total_shares =
					U256::from(pool_info.total_shares.to_owned().saturated_into::<u128>());
				let share_info = SharesAndWithdrawnRewards::<T>::get(gauge_pool_info.pid, who)
					.ok_or(Error::<T>::ShareInfoNotExists)?;
				gauge_pool_info.rewards.iter().try_for_each(
					|(
						reward_currency,
						(reward_amount, total_gauged_reward, _total_withdrawn_reward),
					)|
					 -> DispatchResult {
						let reward = reward_amount
							.checked_sub(&total_gauged_reward)
							.ok_or(ArithmeticError::Overflow)?;
						// gauge_reward = gauge rate * gauge coefficient * existing rewards in the
						// gauge pool
						let gauge_reward = gauge_rate * reward;
						// reward_to_claim = farming rate * gauge rate * gauge coefficient *
						// existing rewards in the gauge pool
						let reward_to_claim: BalanceOf<T> =
							U256::from(share_info.share.to_owned().saturated_into::<u128>())
								.saturating_mul(U256::from(
									gauge_reward.to_owned().saturated_into::<u128>(),
								))
								.checked_div(total_shares)
								.unwrap_or_default()
								.as_u128()
								.unique_saturated_into();
						result_vec.push((*reward_currency, reward_to_claim));
						Ok(())
					},
				)?;
			},
		};
		Ok(result_vec)
	}
}

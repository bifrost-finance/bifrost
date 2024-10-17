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

use frame_support::pallet_prelude::*;
use parity_scale_codec::HasCompact;
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::{
	traits::{Zero, *},
	ArithmeticError, Perbill, RuntimeDebug, SaturatedConversion,
};
use sp_std::prelude::*;

use crate::*;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct GaugeInfo<BalanceOf: HasCompact, BlockNumberFor, AccountIdOf> {
	/// Gauge pool controller
	pub who: AccountIdOf,
	/// The amount of the user deposited in the gauge pool.
	pub gauge_amount: BalanceOf,
	/// Total time factor
	pub total_time_factor: u128,
	/// The latest time factor when the user deposit/withdraw.
	pub latest_time_factor: u128,
	/// The time factor when the user claimed the rewards last time.
	pub claimed_time_factor: u128,
	/// The block number when the pool started to gauge.
	pub gauge_start_block: BlockNumberFor,
	/// The block number when the pool stopped to gauge.
	pub gauge_stop_block: BlockNumberFor,
	/// The block number when the user deposit/withdraw last time.
	pub gauge_last_block: BlockNumberFor,
	/// The block number when the user claimed the rewards last time.
	pub last_claim_block: BlockNumberFor,
}

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct GaugePoolInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, AccountIdOf, BlockNumberFor> {
	pub pid: PoolId,
	pub token: CurrencyIdOf,
	pub keeper: AccountIdOf,
	pub reward_issuer: AccountIdOf,
	pub rewards: BTreeMap<CurrencyIdOf, (BalanceOf, BalanceOf, BalanceOf)>,
	pub gauge_basic_rewards: BTreeMap<CurrencyIdOf, BalanceOf>,
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
		keeper: AccountIdOf,
		reward_issuer: AccountIdOf,
		gauge_basic_rewards: BTreeMap<CurrencyIdOf, BalanceOf>,
		max_block: BlockNumberFor,
		current_block_number: BlockNumberFor,
	) -> Self {
		Self {
			pid,
			token: Default::default(),
			keeper,
			reward_issuer,
			rewards: BTreeMap::new(),
			gauge_basic_rewards,
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
		gauge_basic_rewards: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
		max_block: BlockNumberFor<T>,
	) -> DispatchResult {
		pool_info.gauge = Some(pid);
		let current_block_number = frame_system::Pallet::<T>::block_number();
		let gauge_pool_info = GaugePoolInfo::new(
			pid,
			pool_info.keeper.clone(),
			pool_info.reward_issuer.clone(),
			gauge_basic_rewards,
			max_block,
			current_block_number,
		);

		GaugePoolInfos::<T>::insert(pid, &gauge_pool_info);
		GaugePoolNextId::<T>::mutate(|id| -> DispatchResult {
			*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
			Ok(())
		})?;

		let controller = T::GaugeRewardIssuer::get().into_sub_account_truncating(pid);
		T::BbBNC::set_incentive(pid, Some(max_block), Some(controller));
		Ok(())
	}

	pub fn get_farming_rewards(
		who: &T::AccountId,
		pid: PoolId,
	) -> Result<Vec<(T::CurrencyId, BalanceOf<T>)>, DispatchError> {
		let share_info =
			SharesAndWithdrawnRewards::<T>::get(pid, who).ok_or(Error::<T>::ShareInfoNotExists)?;
		let pool_info = PoolInfos::<T>::get(pid).ok_or(Error::<T>::PoolDoesNotExist)?;
		let total_shares = U256::from(pool_info.total_shares.to_owned().saturated_into::<u128>());
		let mut result_vec = Vec::<(T::CurrencyId, BalanceOf<T>)>::new();

		pool_info.rewards.iter().try_for_each(
			|(reward_currency, (total_reward, total_withdrawn_reward))| -> DispatchResult {
				let withdrawn_reward =
					share_info.withdrawn_rewards.get(reward_currency).copied().unwrap_or_default();

				let total_reward_proportion: BalanceOf<T> = u128::try_from(
					U256::from(share_info.share.to_owned().saturated_into::<u128>())
						.saturating_mul(U256::from(
							total_reward.to_owned().saturated_into::<u128>(),
						))
						.checked_div(total_shares)
						.unwrap_or_default(),
				)
				.map_err(|_| ArithmeticError::Overflow)?
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
	) -> Result<Vec<(T::CurrencyId, BalanceOf<T>)>, DispatchError> {
		let pool_info = PoolInfos::<T>::get(pid).ok_or(Error::<T>::PoolDoesNotExist)?;
		let mut result_vec = Vec::<(T::CurrencyId, BalanceOf<T>)>::new();

		match pool_info.gauge {
			None => (),
			Some(gid) => {
				let current_block_number: BlockNumberFor<T> =
					frame_system::Pallet::<T>::block_number();
				let gauge_pool_info =
					GaugePoolInfos::<T>::get(gid).ok_or(Error::<T>::GaugePoolNotExist)?;
				let gauge_info =
					GaugeInfos::<T>::get(gid, who).ok_or(Error::<T>::GaugeInfoNotExist)?;
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
							(start_block
								.checked_sub(&gauge_info.gauge_last_block)
								.ok_or(ArithmeticError::Overflow)?)
							.saturated_into::<u128>(),
						)
						.ok_or(ArithmeticError::Overflow)?;
				let gauge_rate = Perbill::from_rational(
					latest_claimed_time_factor
						.checked_sub(gauge_info.claimed_time_factor)
						.ok_or(ArithmeticError::Overflow)?,
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
						// gauge_reward = gauge rate * gauge rewards * existing rewards in the
						// gauge pool
						let gauge_reward = gauge_rate * reward;
						// reward_to_claim = farming rate * gauge rate * gauge rewards *
						// existing rewards in the gauge pool
						let reward_to_claim: BalanceOf<T> = u128::try_from(
							U256::from(share_info.share.to_owned().saturated_into::<u128>())
								.saturating_mul(U256::from(
									gauge_reward.to_owned().saturated_into::<u128>(),
								))
								.checked_div(total_shares)
								.unwrap_or_default(),
						)
						.map_err(|_| ArithmeticError::Overflow)?
						.unique_saturated_into();
						result_vec.push((*reward_currency, reward_to_claim));
						Ok(())
					},
				)?;
			},
		};
		Ok(result_vec)
	}

	pub fn update_reward(who: &AccountIdOf<T>, pid: PoolId) -> Result<(), DispatchError> {
		let pool_info = PoolInfos::<T>::get(pid).ok_or(Error::<T>::PoolDoesNotExist)?;
		let share_info =
			SharesAndWithdrawnRewards::<T>::get(pid, who).ok_or(Error::<T>::ShareInfoNotExists)?;
		if T::BbBNC::balance_of(who, None)? == BalanceOf::<T>::zero() {
			return Ok(());
		}
		T::BbBNC::update_reward(pid, Some(who), Some((share_info.share, pool_info.total_shares)))?;
		Ok(())
	}
}

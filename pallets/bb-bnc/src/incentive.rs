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

use crate::{traits::BbBNCInterface, *};
use bifrost_primitives::PoolId;
pub use pallet::*;
use sp_std::collections::btree_map::BTreeMap;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct IncentiveConfig<CurrencyId, Balance, BlockNumber, AccountId> {
	/// Reward per block number per currency_id, which will change at notify_reward.
	pub reward_rate: BTreeMap<CurrencyId, Balance>,
	/// Each currency_id is rewarded against each TokenType and grows with user actions.
	pub reward_per_token_stored: BTreeMap<CurrencyId, Balance>,
	/// Round duration.
	pub rewards_duration: BlockNumber,
	/// The time when this round ends.
	pub period_finish: BlockNumber,
	/// Last time rewards were updated, any user action will update this field.
	pub last_update_time: BlockNumber,
	/// When a round is started, the corresponding value will be transferred from this account to
	/// the system account.
	pub incentive_controller: Option<AccountId>,
	/// When a round is started, the value to be transferred will be obtained from this field.
	pub last_reward: Vec<(CurrencyId, Balance)>,
}

impl<CurrencyId, Balance, BlockNumber, AccountId> Default
	for IncentiveConfig<CurrencyId, Balance, BlockNumber, AccountId>
where
	CurrencyId: Default,
	Balance: Default,
	BlockNumber: Default,
{
	fn default() -> Self {
		IncentiveConfig {
			reward_rate: Default::default(),
			reward_per_token_stored: Default::default(),
			rewards_duration: Default::default(),
			period_finish: Default::default(),
			last_update_time: Default::default(),
			incentive_controller: None,
			last_reward: Default::default(),
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Check if the current block number is within the end time of the reward pool
	pub fn last_time_reward_applicable(pool_id: PoolId) -> BlockNumberFor<T> {
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		if current_block_number < IncentiveConfigs::<T>::get(pool_id).period_finish {
			current_block_number
		} else {
			IncentiveConfigs::<T>::get(pool_id).period_finish
		}
	}

	/// Calculate the reward per token for the given pool
	pub fn reward_per_token(
		pool_id: PoolId,
	) -> Result<BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>, DispatchError> {
		let mut conf = IncentiveConfigs::<T>::get(pool_id);
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		let total_supply = Self::total_supply(current_block_number)?;
		if total_supply == BalanceOf::<T>::zero() {
			return Ok(conf.reward_per_token_stored);
		}
		// Iterate over each currency and its associated reward rate
		conf.reward_rate.iter().try_for_each(|(currency, &reward)| -> DispatchResult {
			let increment: BalanceOf<T> = U512::from(
				Self::last_time_reward_applicable(pool_id)
					.saturating_sub(conf.last_update_time)
					.saturated_into::<u128>(),
			)
			.checked_mul(U512::from(reward.saturated_into::<u128>()))
			.ok_or(ArithmeticError::Overflow)?
			.checked_mul(U512::from(T::Multiplier::get().saturated_into::<u128>()))
			.ok_or(ArithmeticError::Overflow)?
			.checked_div(U512::from(total_supply.saturated_into::<u128>()))
			.map(|x| u128::try_from(x))
			.ok_or(ArithmeticError::Overflow)?
			.map_err(|_| ArithmeticError::Overflow)?
			.unique_saturated_into();
			conf.reward_per_token_stored
				.entry(*currency)
				.and_modify(|total_reward| *total_reward = total_reward.saturating_add(increment))
				.or_insert(increment);
			Ok(())
		})?;

		IncentiveConfigs::<T>::set(pool_id, conf.clone());
		Ok(conf.reward_per_token_stored)
	}

	/// Calculates the reward earned by an account from a specific reward pool
	pub fn earned(
		pool_id: PoolId,
		who: &AccountIdOf<T>,
		share_info: Option<(BalanceOf<T>, BalanceOf<T>)>,
	) -> Result<BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>, DispatchError> {
		let reward_per_token = Self::reward_per_token(pool_id)?;
		let bbbnc_balance = Self::balance_of_current_block(who)?;
		let mut rewards = if let Some(rewards) = Rewards::<T>::get(who) {
			rewards
		} else {
			BTreeMap::<CurrencyIdOf<T>, BalanceOf<T>>::default()
		};
		reward_per_token.iter().try_for_each(|(currency, reward)| -> DispatchResult {
			let increment = U256::from(bbbnc_balance.saturated_into::<u128>())
				.checked_mul(U256::from(
					reward
						.saturating_sub(
							*UserRewardPerTokenPaid::<T>::get(who)
								.get(currency)
								.unwrap_or(&BalanceOf::<T>::zero()),
						)
						.saturated_into::<u128>(),
				))
				.ok_or(ArithmeticError::Overflow)?
				.checked_div(U256::from(T::Multiplier::get().saturated_into::<u128>()))
				.ok_or(ArithmeticError::Overflow)?;
			// .map(|x| u128::try_from(x))
			// .ok_or(ArithmeticError::Overflow)?
			// .map_err(|_| ArithmeticError::Overflow)?
			// .unique_saturated_into();

			// If share information is provided, calculate the reward based on the individual share
			// and total share.
			match share_info {
				Some((share, total_share)) => {
					let mut pools = UserFarmingPool::<T>::get(who);
					if share.is_zero() {
						if let Some(pos) = pools.iter().position(|&x| x == pool_id) {
							pools.remove(pos);
						}
					} else {
						pools.try_push(pool_id).map_err(|_| Error::<T>::UserFarmingPoolOverflow)?;
					}
					UserFarmingPool::<T>::insert(who, pools);
					let reward = increment
						.checked_mul(U256::from(share.saturated_into::<u128>()))
						.ok_or(ArithmeticError::Overflow)?
						.checked_div(U256::from(total_share.saturated_into::<u128>()))
						.map(|x| u128::try_from(x))
						.ok_or(ArithmeticError::Overflow)?
						.map_err(|_| ArithmeticError::Overflow)?
						.unique_saturated_into();
					rewards
						.entry(*currency)
						.and_modify(|total_reward| {
							*total_reward = total_reward.saturating_add(reward);
						})
						.or_insert(reward);
				},
				// If no share information is provided, calculate the reward directly
				None => {
					let reward = u128::try_from(increment)
						.map_err(|_| ArithmeticError::Overflow)?
						.unique_saturated_into();
					rewards
						.entry(*currency)
						.and_modify(|total_reward| {
							*total_reward = total_reward.saturating_add(reward);
						})
						.or_insert(reward);
				},
			}
			Ok(())
		})?;
		Ok(rewards)
	}

	// Used to update reward when notify_reward or user call
	// create_lock/increase_amount/increase_unlock_time/withdraw/get_rewards
	pub fn update_reward(
		pool_id: PoolId,
		who: Option<&AccountIdOf<T>>,
		share_info: Option<(BalanceOf<T>, BalanceOf<T>)>,
	) -> DispatchResult {
		let reward_per_token_stored = Self::reward_per_token(pool_id)?;

		IncentiveConfigs::<T>::mutate(pool_id, |item| {
			item.reward_per_token_stored = reward_per_token_stored.clone();
			item.last_update_time = Self::last_time_reward_applicable(pool_id);
		});
		// If an account is provided, update the rewards
		if let Some(account) = who {
			let earned = Self::earned(pool_id, account, share_info)?;
			// If the account has earned rewards, update the rewards storage
			if earned != BTreeMap::<CurrencyIdOf<T>, BalanceOf<T>>::default() {
				Rewards::<T>::insert(account, earned);
			}
			UserRewardPerTokenPaid::<T>::insert(account, reward_per_token_stored.clone());
		}
		Ok(())
	}

	/// Update reward for all pools
	pub fn update_reward_all(who: &AccountIdOf<T>) -> DispatchResult {
		UserFarmingPool::<T>::get(who)
			.iter()
			.try_for_each(|&pool_id| -> DispatchResult {
				Self::update_reward(pool_id, Some(who), None)
			})?;
		Self::update_reward(BB_BNC_SYSTEM_POOL_ID, Some(who), None)?;
		Ok(())
	}

	///Transfer rewards into an account
	pub fn get_rewards_inner(
		pool_id: PoolId,
		who: &AccountIdOf<T>,
		share_info: Option<(BalanceOf<T>, BalanceOf<T>)>,
	) -> DispatchResult {
		Self::update_reward(pool_id, Some(who), share_info)?;
		if Self::balance_of_current_block(who)? == BalanceOf::<T>::zero() {
			return Ok(());
		} // Excit earlier if balance of token is zero
		if let Some(rewards) = Rewards::<T>::get(who) {
			rewards.iter().try_for_each(|(currency, &reward)| -> DispatchResult {
				T::MultiCurrency::transfer(
					*currency,
					&T::IncentivePalletId::get().into_account_truncating(),
					who,
					reward,
				)
			})?;
			Rewards::<T>::remove(who);
			Self::deposit_event(Event::Rewarded {
				who: who.to_owned(),
				rewards: rewards.into_iter().collect(),
			});
		}
		Ok(())
	}

	// Motion
	pub fn notify_reward_amount(
		pool_id: PoolId,
		who: &Option<AccountIdOf<T>>,
		rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
	) -> DispatchResult {
		let account = match who {
			Some(who) => who,
			None => return Err(Error::<T>::NoController.into()),
		};
		Self::update_reward(pool_id, None, None)?;
		let mut conf = IncentiveConfigs::<T>::get(pool_id);
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();

		if current_block_number >= conf.period_finish {
			Self::add_reward(&account, &mut conf, &rewards, Zero::zero())?;
		} else {
			let remaining = T::BlockNumberToBalance::convert(
				conf.period_finish.saturating_sub(current_block_number),
			);
			Self::add_reward(&account, &mut conf, &rewards, remaining)?;
		};

		conf.last_update_time = current_block_number;
		conf.period_finish = current_block_number.saturating_add(conf.rewards_duration);
		conf.incentive_controller = Some(account.clone());
		conf.last_reward = rewards.clone();
		IncentiveConfigs::<T>::set(pool_id, conf);

		Self::deposit_event(Event::RewardAdded { rewards });
		Ok(())
	}
}

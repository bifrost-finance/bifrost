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

use crate::{
	traits::{Incentive, VeMintingInterface},
	*,
};
pub use pallet::*;
use sp_std::collections::btree_map::BTreeMap;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct IncentiveConfig<CurrencyId, Balance, BlockNumber> {
	pub reward_rate: BTreeMap<CurrencyId, Balance>,
	pub reward_per_token_stored: BTreeMap<CurrencyId, Balance>,
	pub rewards_duration: BlockNumber,
	pub period_finish: BlockNumber,
	pub last_update_time: BlockNumber,
}

impl<T: Config> Pallet<T> {
	pub fn last_time_reward_applicable() -> BlockNumberFor<T> {
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		if current_block_number < Self::incentive_configs().period_finish {
			current_block_number
		} else {
			Self::incentive_configs().period_finish
		}
	}

	pub fn reward_per_token() -> Result<BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>, DispatchError> {
		let mut conf = Self::incentive_configs();
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		let total_supply = Self::total_supply(current_block_number)?;
		if total_supply == BalanceOf::<T>::zero() {
			return Ok(conf.reward_per_token_stored);
		}
		conf.reward_rate
			.iter()
			.try_for_each(|(currency, &reward)| -> DispatchResult {
				let increment: BalanceOf<T> = U512::from(
					Self::last_time_reward_applicable()
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
					.and_modify(|total_reward| {
						*total_reward = total_reward.saturating_add(increment)
					})
					.or_insert(increment);
				Ok(())
			})?;

		IncentiveConfigs::<T>::set(conf.clone());
		Ok(conf.reward_per_token_stored)
	}

	pub fn earned(
		addr: &AccountIdOf<T>,
	) -> Result<BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>, DispatchError> {
		let reward_per_token = Self::reward_per_token()?;
		let vetoken_balance = Self::balance_of_current_block(addr)?;
		let mut rewards = if let Some(rewards) = Self::rewards(addr) {
			rewards
		} else {
			BTreeMap::<CurrencyIdOf<T>, BalanceOf<T>>::default()
		};
		reward_per_token
			.iter()
			.try_for_each(|(currency, reward)| -> DispatchResult {
				let increment: BalanceOf<T> = U256::from(vetoken_balance.saturated_into::<u128>())
					.checked_mul(U256::from(
						reward
							.saturating_sub(
								*Self::user_reward_per_token_paid(addr)
									.get(currency)
									.unwrap_or(&BalanceOf::<T>::zero()),
							)
							.saturated_into::<u128>(),
					))
					.ok_or(ArithmeticError::Overflow)?
					.checked_div(U256::from(T::Multiplier::get().saturated_into::<u128>()))
					.map(|x| u128::try_from(x))
					.ok_or(ArithmeticError::Overflow)?
					.map_err(|_| ArithmeticError::Overflow)?
					.unique_saturated_into();
				rewards
					.entry(*currency)
					.and_modify(|total_reward| {
						*total_reward = total_reward.saturating_add(increment);
					})
					.or_insert(increment);
				Ok(())
			})?;
		Ok(rewards)
	}

	// Used to update reward when notify_reward or user call
	// create_lock/increase_amount/increase_unlock_time/withdraw/get_rewards
	pub fn update_reward(addr: Option<&AccountIdOf<T>>) -> DispatchResult {
		let reward_per_token_stored = Self::reward_per_token()?;

		IncentiveConfigs::<T>::mutate(|item| {
			item.reward_per_token_stored = reward_per_token_stored.clone();
			item.last_update_time = Self::last_time_reward_applicable();
		});
		if let Some(address) = addr {
			let earned = Self::earned(&address)?;
			if earned != BTreeMap::<CurrencyIdOf<T>, BalanceOf<T>>::default() {
				Rewards::<T>::insert(address, earned);
			}
			UserRewardPerTokenPaid::<T>::insert(address, reward_per_token_stored.clone());
		}
		Ok(())
	}

	pub fn get_rewards_inner(addr: &AccountIdOf<T>) -> DispatchResult {
		Self::update_reward(Some(addr))?;

		if let Some(rewards) = Self::rewards(addr) {
			rewards
				.iter()
				.try_for_each(|(currency, &reward)| -> DispatchResult {
					T::MultiCurrency::transfer(
						*currency,
						&T::IncentivePalletId::get().into_account_truncating(),
						addr,
						reward,
					)
				})?;
			Rewards::<T>::remove(addr);
			Self::deposit_event(Event::Rewarded {
				addr: addr.to_owned(),
				rewards: rewards.into_iter().collect(),
			});
		} else {
			return Err(Error::<T>::NoRewards.into());
		}
		Ok(())
	}

	// Motion
	pub fn notify_reward_amount(
		addr: &AccountIdOf<T>,
		rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
	) -> DispatchResult {
		Self::update_reward(None)?;
		let mut conf = Self::incentive_configs();
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();

		if current_block_number >= conf.period_finish {
			Self::add_reward(addr, &mut conf, &rewards, Zero::zero())?;
		} else {
			let remaining = T::BlockNumberToBalance::convert(
				conf.period_finish.saturating_sub(current_block_number),
			);
			Self::add_reward(addr, &mut conf, &rewards, remaining)?;
		};

		conf.last_update_time = current_block_number;
		conf.period_finish = current_block_number.saturating_add(conf.rewards_duration);
		IncentiveConfigs::<T>::set(conf);

		Self::deposit_event(Event::RewardAdded { rewards });
		Ok(())
	}
}

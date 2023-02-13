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

use crate::{traits::VeMintingInterface, *};
pub use pallet::*;
use sp_std::collections::btree_map::BTreeMap;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct IncentiveConfig<CurrencyId, Balance, BlockNumber> {
	reward_rate: BTreeMap<CurrencyId, Balance>,
	reward_per_token_stored: BTreeMap<CurrencyId, Balance>,
	pub rewards_duration: BlockNumber,
	period_finish: BlockNumber,
	last_update_time: BlockNumber,
}

impl<T: Config> Pallet<T> {
	pub fn last_time_reward_applicable() -> T::BlockNumber {
		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number();
		if current_block_number < Self::incentive_configs().period_finish {
			current_block_number
		} else {
			Self::incentive_configs().period_finish
		}
	}

	#[transactional]
	pub fn reward_per_token() -> Result<BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>, DispatchError> {
		let mut conf = Self::incentive_configs();
		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number().into();
		let _total_supply = Self::total_supply(current_block_number)?;
		if _total_supply == BalanceOf::<T>::zero() {
			return Ok(conf.reward_per_token_stored);
		}
		conf.reward_rate.iter().for_each(|(currency, reward)| {
			conf.reward_per_token_stored
				.entry(*currency)
				.and_modify(|total_reward| {
					log::debug!(
						"reward_per_token:{:?}Self::last_time_reward_applicable():{:?}conf.last_update_time:{:?}total_reward:{:?}",
						reward,
						Self::last_time_reward_applicable(),
						conf.last_update_time,
						total_reward
					);
					*total_reward = total_reward
						.saturating_add(T::BlockNumberToBalance::convert(
							Self::last_time_reward_applicable().saturating_sub(conf.last_update_time)
						)
						.saturating_mul(*reward)
						.saturating_mul(Self::ve_configs().multiplier.unique_saturated_into())
						.checked_div(&_total_supply)
						.unwrap_or_default())
				})
				.or_insert(*reward);
		});

		IncentiveConfigs::<T>::set(conf.clone());
		Ok(conf.reward_per_token_stored)
	}

	#[transactional]
	pub fn earned(
		addr: &AccountIdOf<T>,
	) -> Result<BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>, DispatchError> {
		let reward_per_token = Self::reward_per_token()?;
		let vetoken_balance = Self::balance_of(addr, None)?;
		let mut rewards = if let Some(rewards) = Self::rewards(addr) {
			rewards
		} else {
			BTreeMap::<CurrencyIdOf<T>, BalanceOf<T>>::default()
		};
		log::debug!("earned---reward_per_token:{:?}", reward_per_token.clone(),);
		reward_per_token.iter().try_for_each(|(currency, reward)| -> DispatchResult {
			rewards
				.entry(*currency)
				.and_modify(|total_reward| {
					*total_reward = total_reward.saturating_add(
						vetoken_balance
							.saturating_mul(
								reward.saturating_sub(
									*Self::user_reward_per_token_paid(addr)
										.get(currency)
										.unwrap_or(&BalanceOf::<T>::zero()),
								),
							)
							.checked_div(&Self::ve_configs().multiplier.unique_saturated_into())
							.unwrap_or_default(),
					);
				})
				.or_insert(
					vetoken_balance
						.saturating_mul(
							Self::reward_per_token()?
								.get(currency)
								.unwrap_or(&BalanceOf::<T>::zero())
								.saturating_sub(
									*Self::user_reward_per_token_paid(addr)
										.get(currency)
										.unwrap_or(&BalanceOf::<T>::zero()),
								),
						)
						.checked_div(&Self::ve_configs().multiplier.unique_saturated_into())
						.unwrap_or_default(),
				);
			Ok(())
		})?;
		Ok(rewards)
	}

	#[transactional]
	pub fn update_reward(addr: Option<&AccountIdOf<T>>) -> DispatchResult {
		let reward_per_token_stored = Self::reward_per_token()?;

		IncentiveConfigs::<T>::mutate(|item| {
			item.reward_per_token_stored = reward_per_token_stored.clone();
			item.last_update_time = Self::last_time_reward_applicable();
		});
		if let Some(address) = addr {
			log::debug!(
				"update_reward---reward_per_token_stored:{:?}Self::earned(&address)?:{:?}",
				reward_per_token_stored.clone(),
				Self::earned(&address)?
			);
			Rewards::<T>::insert(address, Self::earned(&address)?);
			UserRewardPerTokenPaid::<T>::insert(address, reward_per_token_stored.clone());
		}
		Ok(())
	}

	#[transactional]
	pub fn get_reward(addr: &AccountIdOf<T>) -> DispatchResult {
		Self::update_reward(Some(addr))?;

		if let Some(rewards) = Self::rewards(addr) {
			rewards.iter().try_for_each(|(currency, &reward)| -> DispatchResult {
				log::debug!("get_reward---currency:{:?}reward:{:?}", currency, reward);
				T::MultiCurrency::transfer(
					*currency,
					&T::IncentivePalletId::get().into_account_truncating(),
					addr,
					reward,
				)
			})?;
			Rewards::<T>::remove(addr);
		}
		Ok(())
	}

	// Motion
	#[transactional]
	pub fn notify_reward_amount(rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>) -> DispatchResult {
		Self::update_reward(None)?;
		let mut conf = Self::incentive_configs();
		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number().into();

		if current_block_number >= conf.period_finish {
			rewards.iter().try_for_each(|(currency, reward)| -> DispatchResult {
				let currency_amount = T::MultiCurrency::free_balance(
					*currency,
					&T::IncentivePalletId::get().into_account_truncating(),
				);
				ensure!(*reward <= currency_amount, Error::<T>::NotEnoughBalance);
				let new_reward = reward
					.checked_div(&T::BlockNumberToBalance::convert(conf.rewards_duration))
					.ok_or(ArithmeticError::Overflow)?;
				conf.reward_rate
					.entry(*currency)
					.and_modify(|total_reward| {
						*total_reward = new_reward;
					})
					.or_insert(new_reward);
				Ok(())
			})?;
		} else {
			let remaining = T::BlockNumberToBalance::convert(
				conf.period_finish.saturating_sub(current_block_number),
			);
			rewards.iter().try_for_each(|(currency, reward)| -> DispatchResult {
				let leftover: BalanceOf<T> = reward.saturating_mul(remaining);
				let total_reward: BalanceOf<T> = reward.saturating_add(leftover);
				let currency_amount = T::MultiCurrency::free_balance(
					*currency,
					&T::IncentivePalletId::get().into_account_truncating(),
				);
				ensure!(total_reward <= currency_amount, Error::<T>::NotEnoughBalance);
				let new_reward = total_reward
					.checked_div(&T::BlockNumberToBalance::convert(conf.rewards_duration))
					.ok_or(ArithmeticError::Overflow)?;
				conf.reward_rate
					.entry(*currency)
					.and_modify(|total_reward| {
						*total_reward = new_reward;
					})
					.or_insert(new_reward);
				Ok(())
			})?;
		};

		conf.last_update_time = current_block_number;
		conf.period_finish = current_block_number.saturating_add(conf.rewards_duration);

		IncentiveConfigs::<T>::set(conf);
		Self::deposit_event(Event::RewardAdded { rewards });
		Ok(())
	}
}

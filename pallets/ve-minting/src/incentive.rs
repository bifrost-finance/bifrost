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
// use frame_system::pallet_prelude::*;
// use node_primitives::currency;
pub use pallet::*;
use sp_std::collections::btree_map::BTreeMap; //{borrow::ToOwned, collections::btree_map::BTreeMap, prelude::*};

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct IncentiveConfig<CurrencyId, Balance> {
	reward_rate: BTreeMap<CurrencyId, Balance>, // Balance,
	reward_per_token_stored: BTreeMap<CurrencyId, Balance>,
	pub rewards_duration: Timestamp,
	period_finish: Timestamp,
	last_update_time: Timestamp,
}

impl<T: Config> Pallet<T> {
	pub fn last_time_reward_applicable() -> Timestamp {
		let current_timestamp: Timestamp = T::UnixTime::now().as_millis().saturated_into();
		if current_timestamp < Self::incentive_configs().period_finish {
			current_timestamp
		} else {
			Self::incentive_configs().period_finish
		}
	}

	pub fn reward_per_token() -> BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> {
		let mut conf = Self::incentive_configs();
		let current_timestamp: Timestamp = T::UnixTime::now().as_millis().saturated_into();
		let _total_supply = Self::total_supply(current_timestamp);
		if _total_supply == BalanceOf::<T>::zero() {
			return conf.reward_per_token_stored;
		}
		conf.reward_per_token_stored.iter_mut().for_each(|(currency, reward)| {
			*reward = reward.saturating_add(
				Self::last_time_reward_applicable()
					.saturated_into::<BalanceOf<T>>()
					.saturating_sub(conf.last_update_time.saturated_into::<BalanceOf<T>>())
					.saturating_mul(
						*conf.reward_rate.get(currency).unwrap_or(&BalanceOf::<T>::zero()),
					)
					// .mul(1e18)
					.checked_div(&_total_supply)
					.unwrap_or_default(), // .ok_or(Error::<T>::CalculationOverflow)?,
			);
		});

		IncentiveConfigs::<T>::set(conf.clone());
		conf.reward_per_token_stored
	}

	pub fn earned(
		addr: &AccountIdOf<T>,
	) -> Result<BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>, DispatchError> {
		let reward_per_token = Self::reward_per_token();
		// let mut rewards: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> = Self::rewards(addr);
		let vetoken_balance = Self::balance_of(addr, None)?;
		// rewards.iter_mut().for_each(|(currency, reward)| {
		// 	*reward = reward.saturating_add(
		// 		vetoken_balance.saturating_mul(
		// 			Self::reward_per_token()
		// 				.get(currency)
		// 				.unwrap_or(&BalanceOf::<T>::zero())
		// 				.saturating_sub(
		// 					*Self::user_reward_per_token_paid(addr)
		// 						.get(currency)
		// 						.unwrap_or(&BalanceOf::<T>::zero()),
		// 				),
		// 		),
		// 	);
		// });
		let mut rewards = if let Some(rewards) = Self::rewards(addr) {
			rewards
		} else {
			BTreeMap::<CurrencyIdOf<T>, BalanceOf<T>>::default()
		};

		reward_per_token.iter().for_each(|(currency, reward)| {
			rewards
				.entry(*currency)
				.and_modify(|total_reward| {
					*total_reward = total_reward.saturating_add(
						vetoken_balance.saturating_mul(
							Self::reward_per_token()
								.get(currency)
								.unwrap_or(&BalanceOf::<T>::zero())
								.saturating_sub(
									*Self::user_reward_per_token_paid(addr)
										.get(currency)
										.unwrap_or(&BalanceOf::<T>::zero()),
								),
						),
					);
				})
				.or_insert(
					vetoken_balance.saturating_mul(
						Self::reward_per_token()
							.get(currency)
							.unwrap_or(&BalanceOf::<T>::zero())
							.saturating_sub(
								*Self::user_reward_per_token_paid(addr)
									.get(currency)
									.unwrap_or(&BalanceOf::<T>::zero()),
							),
					),
				);
		});
		Ok(rewards)
		// Ok(Self::balance_of(addr, current_timestamp)?
		// 	.saturating_mul(
		// 		Self::reward_per_token().saturating_sub(Self::user_reward_per_token_paid(addr)),
		// 	)
		// 	// .div(1e18)
		// 	.saturating_add(Self::rewards(addr)))
	}

	pub fn update_reward(addr: Option<&AccountIdOf<T>>) -> DispatchResult {
		let reward_per_token_stored = Self::reward_per_token();
		IncentiveConfigs::<T>::mutate(|item| {
			item.reward_per_token_stored = reward_per_token_stored.clone();
			item.last_update_time = Self::last_time_reward_applicable();
		});
		if let Some(address) = addr {
			Rewards::<T>::insert(address, Self::earned(&address)?);
			UserRewardPerTokenPaid::<T>::insert(address, reward_per_token_stored);
		}
		Ok(())
	}

	// pub fn staking(addr: &AccountIdOf<T>, reward: BalanceOf<T>) -> DispatchResult {
	// 	Self::update_reward(Some(addr))
	// }

	#[transactional]
	pub fn get_reward(addr: &AccountIdOf<T>) -> DispatchResult {
		Self::update_reward(Some(addr))?;

		if let Some(rewards) = Self::rewards(addr) {
			rewards.iter().try_for_each(|(currency, &reward)| -> DispatchResult {
				T::MultiCurrency::transfer(
					*currency,
					&T::VeMintingPalletId::get().into_account_truncating(),
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
		let current_timestamp: Timestamp = T::UnixTime::now().as_millis().saturated_into();
		let rewards_map: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> =
			rewards.iter().clone().map(|(k, v)| (*k, *v)).collect();

		if current_timestamp >= conf.period_finish {
			rewards.iter().try_for_each(|(currency, reward)| -> DispatchResult {
				let currency_amount = T::MultiCurrency::free_balance(
					*currency,
					&T::VeMintingPalletId::get().into_account_truncating(),
				);
				ensure!(*reward <= currency_amount, Error::<T>::Expired);
				let new_reward = reward
					.checked_div(&conf.rewards_duration.saturated_into::<BalanceOf<T>>())
					.unwrap_or_else(Zero::zero);
				conf.reward_rate
					.entry(*currency)
					.and_modify(|total_reward| {
						*total_reward = new_reward;
					})
					.or_insert(new_reward);
				Ok(())
			})?;

		// conf.reward_rate = reward
		// 	.checked_div(&conf.rewards_duration.saturated_into::<BalanceOf<T>>())
		// 	.ok_or(Error::<T>::CalculationOverflow)?;
		} else {
			let remaining = conf
				.period_finish
				.saturating_sub(current_timestamp)
				.saturated_into::<BalanceOf<T>>();
			// let leftover: BalanceOf<T> = remaining.saturating_mul(conf.reward_rate);
			// conf.reward_rate = reward
			// 	.saturating_add(leftover)
			// 	.checked_div(&conf.rewards_duration.saturated_into::<BalanceOf<T>>())
			// 	.ok_or(Error::<T>::CalculationOverflow)?;
			rewards.iter().try_for_each(|(currency, reward)| -> DispatchResult {
				let leftover: BalanceOf<T> = reward.saturating_mul(remaining);
				let total_reward: BalanceOf<T> = reward.saturating_add(leftover);
				let currency_amount = T::MultiCurrency::free_balance(
					*currency,
					&T::VeMintingPalletId::get().into_account_truncating(),
				);
				ensure!(total_reward <= currency_amount, Error::<T>::Expired);
				let new_reward = total_reward
					.checked_div(&conf.rewards_duration.saturated_into::<BalanceOf<T>>())
					.unwrap_or_else(|| BalanceOf::<T>::zero());
				conf.reward_rate
					.entry(*currency)
					.and_modify(|total_reward| {
						*total_reward = new_reward;
					})
					.or_insert(new_reward);
				Ok(())
			})?;
		};
		let balance =
			Self::balance_of(&T::VeMintingPalletId::get().into_account_truncating(), None)?;

		conf.last_update_time = current_timestamp;
		conf.period_finish = current_timestamp.saturating_add(conf.rewards_duration);

		IncentiveConfigs::<T>::set(conf);
		Ok(())
	}
}

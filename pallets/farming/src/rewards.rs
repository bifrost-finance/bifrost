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

// #![allow(clippy::unused_unit)]
// #![cfg_attr(not(feature = "std"), no_std)]

use core::default;

// mod mock;
// mod tests;
use codec::HasCompact;
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::{
	traits::{Saturating, UniqueSaturatedInto, Zero},
	RuntimeDebug, SaturatedConversion,
};
use sp_std::{borrow::ToOwned, collections::btree_map::BTreeMap, prelude::*};

use crate::*;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct ShareInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, BlockNumberFor, AccountIdOf> {
	pub who: Option<AccountIdOf>,
	pub share: BalanceOf,
	pub withdrawn_rewards: BTreeMap<CurrencyIdOf, BalanceOf>,
	pub withdraw_last_block: BlockNumberFor,
	pub claim_last_block: BlockNumberFor,
	pub withdraw_list: Vec<(BlockNumberFor, BalanceOf)>,
}

impl<BalanceOf, CurrencyIdOf, BlockNumberFor, AccountIdOf> Default
	for ShareInfo<BalanceOf, CurrencyIdOf, BlockNumberFor, AccountIdOf>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord,
	BlockNumberFor: Default,
{
	fn default() -> Self {
		Self {
			who: None,
			share: Default::default(),
			withdrawn_rewards: BTreeMap::new(),
			withdraw_last_block: Default::default(),
			claim_last_block: Default::default(),
			withdraw_list: Default::default(),
		}
	}
}

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PoolInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, AccountIdOf, BlockNumberFor> {
	/// Total shares amount
	pub tokens_proportion: BTreeMap<CurrencyIdOf, Permill>,
	/// Total shares amount
	pub total_shares: BalanceOf,
	pub basic_rewards: BTreeMap<CurrencyIdOf, BalanceOf>,
	/// Reward infos <reward_currency, (total_reward, total_withdrawn_reward)>
	pub rewards: BTreeMap<CurrencyIdOf, (BalanceOf, BalanceOf)>,
	pub state: PoolState,
	pub keeper: Option<AccountIdOf>,
	/// Gauge pool id
	pub gauge: Option<PoolId>,
	pub block_startup: Option<BlockNumberFor>,
	pub min_deposit_to_start: BalanceOf,
	pub after_block_to_start: BlockNumberFor,
	pub withdraw_limit_time: BlockNumberFor,
	pub claim_limit_time: BlockNumberFor,
	pub withdraw_limit_count: u8,
}

impl<BalanceOf, CurrencyIdOf, AccountIdOf, BlockNumberFor> Default
	for PoolInfo<BalanceOf, CurrencyIdOf, AccountIdOf, BlockNumberFor>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord,
	BlockNumberFor: Default,
{
	fn default() -> Self {
		Self {
			tokens_proportion: BTreeMap::new(),
			total_shares: Default::default(),
			basic_rewards: BTreeMap::new(),
			rewards: BTreeMap::new(),
			state: PoolState::UnCharged,
			keeper: None,
			gauge: None,
			block_startup: None,
			min_deposit_to_start: Default::default(),
			after_block_to_start: Default::default(),
			withdraw_limit_time: Default::default(),
			claim_limit_time: Default::default(),
			withdraw_limit_count: Default::default(),
		}
	}
}

impl<BalanceOf, CurrencyIdOf, AccountIdOf, BlockNumberFor>
	PoolInfo<BalanceOf, CurrencyIdOf, AccountIdOf, BlockNumberFor>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord,
{
	pub fn new(
		keeper: AccountIdOf,
		tokens_proportion: BTreeMap<CurrencyIdOf, Permill>,
		basic_rewards: BTreeMap<CurrencyIdOf, BalanceOf>,
		gauge: Option<PoolId>,
		min_deposit_to_start: BalanceOf,
		after_block_to_start: BlockNumberFor,
		withdraw_limit_time: BlockNumberFor,
		claim_limit_time: BlockNumberFor,
		withdraw_limit_count: u8,
	) -> Self {
		Self {
			tokens_proportion,
			total_shares: Default::default(),
			basic_rewards,
			rewards: BTreeMap::new(),
			state: PoolState::UnCharged,
			keeper: Some(keeper),
			gauge,
			block_startup: None,
			min_deposit_to_start,
			after_block_to_start,
			withdraw_limit_time,
			claim_limit_time,
			withdraw_limit_count,
		}
	}

	pub fn reset(
		keeper: AccountIdOf,
		tokens_proportion: BTreeMap<CurrencyIdOf, Permill>,
		basic_rewards: BTreeMap<CurrencyIdOf, BalanceOf>,
		state: PoolState,
		gauge: Option<PoolId>,
		min_deposit_to_start: BalanceOf,
		after_block_to_start: BlockNumberFor,
		withdraw_limit_time: BlockNumberFor,
		claim_limit_time: BlockNumberFor,
		withdraw_limit_count: u8,
	) -> Self {
		Self {
			tokens_proportion,
			total_shares: Default::default(),
			basic_rewards,
			rewards: BTreeMap::new(),
			state,
			keeper: Some(keeper),
			gauge,
			block_startup: None,
			min_deposit_to_start,
			after_block_to_start,
			withdraw_limit_time,
			claim_limit_time,
			withdraw_limit_count,
		}
	}
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum PoolState {
	UnCharged,
	Charged,
	Ongoing,
	Dead,
	Retired,
}

pub type PoolId = u32;

impl<T: Config> Pallet<T> {
	pub fn accumulate_reward(
		pool: PoolId,
		reward_currency: CurrencyIdOf<T>,
		reward_increment: BalanceOf<T>,
	) -> DispatchResult {
		if reward_increment.is_zero() {
			return Ok(());
		}
		PoolInfos::<T>::mutate_exists(pool, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;

			pool_info
				.rewards
				.entry(reward_currency)
				.and_modify(|(total_reward, _)| {
					*total_reward = total_reward.saturating_add(reward_increment);
				})
				.or_insert((reward_increment, Zero::zero()));

			Ok(())
		})
	}

	pub fn add_share(
		who: &T::AccountId,
		pool: PoolId,
		add_amount: BalanceOf<T>,
		// add_value: &BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
	) {
		if add_amount.is_zero() {
			return;
		}

		PoolInfos::<T>::mutate(pool, |pool_info| {
			let initial_total_shares = pool_info.total_shares;
			pool_info.total_shares = pool_info.total_shares.saturating_add(add_amount);

			let mut withdrawn_inflation = Vec::<(CurrencyIdOf<T>, BalanceOf<T>)>::new();

			pool_info.rewards.iter_mut().for_each(
				|(reward_currency, (total_reward, total_withdrawn_reward))| {
					let reward_inflation = if initial_total_shares.is_zero() {
						Zero::zero()
					} else {
						U256::from(add_amount.to_owned().saturated_into::<u128>())
							.saturating_mul(total_reward.to_owned().saturated_into::<u128>().into())
							.checked_div(
								initial_total_shares.to_owned().saturated_into::<u128>().into(),
							)
							.unwrap_or_default()
							.as_u128()
							.saturated_into()
					};
					*total_reward = total_reward.saturating_add(reward_inflation);
					*total_withdrawn_reward =
						total_withdrawn_reward.saturating_add(reward_inflation);

					withdrawn_inflation.push((*reward_currency, reward_inflation));
				},
			);

			SharesAndWithdrawnRewards::<T>::mutate(pool, who, |share_info| {
				share_info.who = Some(who.clone());
				share_info.share = share_info.share.saturating_add(add_amount);
				// update withdrawn inflation for each reward currency
				withdrawn_inflation.into_iter().for_each(|(reward_currency, reward_inflation)| {
					share_info
						.withdrawn_rewards
						.entry(reward_currency)
						.and_modify(|withdrawn_reward| {
							*withdrawn_reward = withdrawn_reward.saturating_add(reward_inflation);
						})
						.or_insert(reward_inflation);
				});
			});
		});
	}

	pub fn remove_share(
		who: &T::AccountId,
		pool: PoolId,
		remove_amount_input: Option<BalanceOf<T>>,
		withdraw_limit_time: BlockNumberFor<T>,
	) -> DispatchResult {
		if let Some(remove_amount_input) = remove_amount_input {
			if remove_amount_input.is_zero() {
				return Ok(());
			}
		}

		// claim rewards firstly
		Self::claim_rewards(who, pool)?;

		SharesAndWithdrawnRewards::<T>::mutate(pool, who, |mut share_info| -> DispatchResult {
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			// if let Some(mut share_info) = share_info_old.take() {
			// (mut share, mut withdrawn_rewards)S
			let remove_amount;
			if let Some(remove_amount_input) = remove_amount_input {
				remove_amount = remove_amount_input.min(share_info.share);
			} else {
				remove_amount = share_info.share;
			}

			if remove_amount.is_zero() {
				return Ok(());
			}

			PoolInfos::<T>::mutate(pool, |mut pool_info| -> DispatchResult {
				// if let Some(mut pool_info) = maybe_pool_info.take() {
				// ensure!(
				// 	share_info.withdraw_last_block + pool_info.withdraw_limit_time <=
				// 		current_block_number,
				// 	Error::<T>::CanNotWithdraw
				// );

				ensure!(
					share_info.withdraw_list.len() < pool_info.withdraw_limit_count.into(),
					Error::<T>::WithdrawLimitCountExceeded
				);
				share_info
					.withdraw_list
					.push((current_block_number + withdraw_limit_time, remove_amount));

				let removing_share = U256::from(remove_amount.saturated_into::<u128>());

				pool_info.total_shares = pool_info.total_shares.saturating_sub(remove_amount);

				// update withdrawn rewards for each reward currency
				share_info.withdrawn_rewards.iter_mut().try_for_each(
					|(reward_currency, withdrawn_reward)| -> DispatchResult {
						let withdrawn_reward_to_remove: BalanceOf<T> = removing_share
							.saturating_mul(
								withdrawn_reward.to_owned().saturated_into::<u128>().into(),
							)
							.checked_div(share_info.share.saturated_into::<u128>().into())
							.unwrap_or_default()
							.as_u128()
							.saturated_into();

						if let Some((total_reward, total_withdrawn_reward)) =
							pool_info.rewards.get_mut(reward_currency)
						{
							*total_reward = total_reward.saturating_sub(withdrawn_reward_to_remove);
							*total_withdrawn_reward =
								total_withdrawn_reward.saturating_sub(withdrawn_reward_to_remove);

							// remove if all reward is withdrawn
							if total_reward.is_zero() {
								pool_info.rewards.remove(reward_currency);
							}
						}
						*withdrawn_reward =
							withdrawn_reward.saturating_sub(withdrawn_reward_to_remove);
						Ok(())
					},
				)?;

				// 	if !pool_info.total_shares.is_zero() {
				// 		*maybe_pool_info = Some(pool_info);
				// 	}
				// }
				Ok(())
			})?;

			share_info.withdraw_last_block = current_block_number;
			share_info.share = share_info.share.saturating_sub(remove_amount);
			// 	if !share_info.share.is_zero() {
			// 		*share_info_old = Some(share_info);
			// 	};
			// }
			Ok(())
		})?;
		Ok(())
	}

	// pub fn set_share(who: &T::AccountId, pool: PoolId, new_share: BalanceOf<T>) {
	// 	let share_info = Self::shares_and_withdrawn_rewards(pool, who);

	// 	if new_share > share_info.share {
	// 		Self::add_share(who, pool, new_share.saturating_sub(share_info.share));
	// 	} else {
	// 		Self::remove_share(who, pool, share_info.share.saturating_sub(new_share));
	// 	}
	// }

	pub fn claim_rewards(who: &T::AccountId, pool: PoolId) -> DispatchResult {
		SharesAndWithdrawnRewards::<T>::mutate_exists(
			pool,
			who,
			|maybe_share_withdrawn| -> DispatchResult {
				let current_block_number: BlockNumberFor<T> =
					frame_system::Pallet::<T>::block_number();
				if let Some(share_info) = maybe_share_withdrawn {
					if share_info.share.is_zero() {
						return Ok(());
					}

					PoolInfos::<T>::mutate(pool, |pool_info| -> DispatchResult {
						ensure!(
							share_info.claim_last_block + pool_info.claim_limit_time <=
								current_block_number,
							Error::<T>::CanNotClaim
						);

						let total_shares =
							U256::from(pool_info.total_shares.to_owned().saturated_into::<u128>());
						pool_info.rewards.iter_mut().try_for_each(
							|(reward_currency, (total_reward, total_withdrawn_reward))|  -> DispatchResult {
								let withdrawn_reward = share_info
									.withdrawn_rewards
									.get(reward_currency)
									.copied()
									.unwrap_or_default();

								let total_reward_proportion: BalanceOf<T> = U256::from(
									share_info.share.to_owned().saturated_into::<u128>(),
								)
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
								}

								*total_withdrawn_reward =
									total_withdrawn_reward.saturating_add(reward_to_withdraw);
								share_info.withdrawn_rewards.insert(
									*reward_currency,
									withdrawn_reward.saturating_add(reward_to_withdraw),
								);

								// pay reward to `who`
								if let Some(ref keeper) = pool_info.keeper {
									T::MultiCurrency::transfer(
										*reward_currency,
										&keeper,
										&who,
										reward_to_withdraw,
									)?
								};
								Ok(())
							},
						)?;
						Ok(())
					})?;
					share_info.claim_last_block = current_block_number;
				};
				Ok(())
			},
		)?;
		Ok(())
	}

	pub fn process_withraw_list(
		who: &T::AccountId,
		pool: PoolId,
		pool_info: &PoolInfo<BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>, BlockNumberFor<T>>,
	) -> DispatchResult {
		SharesAndWithdrawnRewards::<T>::mutate_exists(
			pool,
			who,
			|share_info_old| -> DispatchResult {
				if let Some(mut share_info) = share_info_old.take() {
					let current_block_number: BlockNumberFor<T> =
						frame_system::Pallet::<T>::block_number();
					let mut tmp: Vec<(BlockNumberFor<T>, BalanceOf<T>)> = Default::default();
					let tokens_proportion_values: Vec<Permill> =
						pool_info.tokens_proportion.values().cloned().collect();
					share_info.withdraw_list.iter().try_for_each(
						|(dest_block, remove_value)| -> DispatchResult {
							if *dest_block <= current_block_number {
								let native_amount = tokens_proportion_values[0]
									.saturating_reciprocal_mul(*remove_value);
								pool_info.tokens_proportion.iter().try_for_each(
									|(token, &proportion)| -> DispatchResult {
										if let Some(ref keeper) = pool_info.keeper {
											T::MultiCurrency::transfer(
												*token,
												&keeper,
												who,
												proportion * native_amount,
											)?
										};
										Ok(())
									},
								);
							} else {
								tmp.push((*dest_block, *remove_value));
							};
							Ok(())
						},
					);
					share_info.withdraw_list = tmp;

					// if withdraw_list and share both are empty, remove it.
					if share_info.withdraw_list !=
						Vec::<(BlockNumberFor<T>, BalanceOf<T>)>::default() ||
						!share_info.share.is_zero()
					{
						*share_info_old = Some(share_info);
					};
				};
				Ok(())
			},
		)
	}
}

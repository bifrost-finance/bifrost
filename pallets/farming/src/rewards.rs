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

// mod mock;
// mod tests;
use codec::{FullCodec, HasCompact};
use frame_support::pallet_prelude::*;
use node_primitives::CurrencyId;
use orml_traits::RewardHandler;
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::{
	traits::{
		AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, Saturating, UniqueSaturatedInto,
		Zero,
	},
	FixedPointOperand, RuntimeDebug, SaturatedConversion,
};
use sp_std::{borrow::ToOwned, collections::btree_map::BTreeMap, fmt::Debug, prelude::*};

use crate::*;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct ShareInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, BlockNumberFor> {
	pub share: BalanceOf,
	pub share_total: BTreeMap<CurrencyIdOf, BalanceOf>,
	pub withdrawn_rewards: BTreeMap<CurrencyIdOf, BalanceOf>,
	pub gauge_amount: BalanceOf,
	pub gauge_time_factor: u128,
	pub gauge_start_block: BlockNumberFor,
	pub gauge_last_block: BlockNumberFor,
}

impl<BalanceOf, CurrencyIdOf, BlockNumberFor> Default
	for ShareInfo<BalanceOf, CurrencyIdOf, BlockNumberFor>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord,
	BlockNumberFor: Default,
{
	fn default() -> Self {
		Self {
			share: Default::default(),
			share_total: BTreeMap::new(),
			withdrawn_rewards: BTreeMap::new(),
			gauge_amount: Default::default(),
			gauge_time_factor: Default::default(),
			gauge_start_block: Default::default(),
			gauge_last_block: Default::default(),
		}
	}
}

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PoolInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, AccountIdOf, BlockNumberFor> {
	/// Total shares amount
	pub tokens: BTreeMap<CurrencyIdOf, BalanceOf>,
	/// Total shares amount
	pub total_shares: BalanceOf,
	pub basic_rewards: BTreeMap<CurrencyIdOf, (BalanceOf, BalanceOf)>,
	/// Reward infos <reward_currency, (total_reward, total_withdrawn_reward)>
	pub rewards: BTreeMap<CurrencyIdOf, (BalanceOf, BalanceOf)>,
	pub state: PoolState,
	pub keeper: Option<AccountIdOf>,
	/// Gauge pool id
	pub gauge: Option<PoolId>,
	pub block_startup: Option<BlockNumberFor>,
	pub starting_token_values: Vec<BalanceOf>,
	pub min_deposit_to_start: BTreeMap<CurrencyIdOf, BalanceOf>,
	pub after_block_to_start: BlockNumberFor,
	pub withdraw_limit_time: BlockNumberFor,
	pub claim_limit_time: BlockNumberFor,
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
			tokens: BTreeMap::new(),
			total_shares: Default::default(),
			basic_rewards: BTreeMap::new(),
			rewards: BTreeMap::new(),
			state: PoolState::UnCharged,
			keeper: None,
			gauge: None,
			block_startup: None,
			starting_token_values: Default::default(),
			min_deposit_to_start: BTreeMap::new(),
			after_block_to_start: Default::default(),
			withdraw_limit_time: Default::default(),
			claim_limit_time: Default::default(),
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
		tokens: BTreeMap<CurrencyIdOf, BalanceOf>,
		basic_rewards: BTreeMap<CurrencyIdOf, (BalanceOf, BalanceOf)>,
		starting_token_values: Vec<BalanceOf>,
		gauge: Option<PoolId>,
		min_deposit_to_start: BTreeMap<CurrencyIdOf, BalanceOf>,
		after_block_to_start: BlockNumberFor,
		withdraw_limit_time: BlockNumberFor,
		claim_limit_time: BlockNumberFor,
	) -> Self {
		Self {
			tokens,
			total_shares: Default::default(),
			basic_rewards,
			rewards: BTreeMap::new(),
			state: PoolState::UnCharged,
			keeper: Some(keeper),
			gauge,
			block_startup: None,
			starting_token_values,
			min_deposit_to_start,
			after_block_to_start,
			withdraw_limit_time,
			claim_limit_time,
		}
	}

	pub fn reset(
		keeper: AccountIdOf,
		tokens: BTreeMap<CurrencyIdOf, BalanceOf>,
		basic_rewards: BTreeMap<CurrencyIdOf, (BalanceOf, BalanceOf)>,
		state: PoolState,
		starting_token_values: Vec<BalanceOf>,
		gauge: Option<PoolId>,
		min_deposit_to_start: BTreeMap<CurrencyIdOf, BalanceOf>,
		after_block_to_start: BlockNumberFor,
		withdraw_limit_time: BlockNumberFor,
		claim_limit_time: BlockNumberFor,
	) -> Self {
		Self {
			tokens,
			total_shares: Default::default(),
			basic_rewards,
			rewards: BTreeMap::new(),
			state,
			keeper: Some(keeper),
			gauge,
			block_startup: None,
			starting_token_values,
			min_deposit_to_start,
			after_block_to_start,
			withdraw_limit_time,
			claim_limit_time,
		}
	}
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum PoolState {
	UnCharged,
	Charged,
	Ongoing,
	Dead,
}

pub type PoolId = u32;
// pub type PoolId = Vec<CurrencyId>;

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

	pub fn add_share(who: &T::AccountId, pool: PoolId, add_amount: BalanceOf<T>) {
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

	pub fn remove_share(who: &T::AccountId, pool: PoolId, remove_amount: BalanceOf<T>) {
		if remove_amount.is_zero() {
			return;
		}

		// claim rewards firstly
		Self::claim_rewards(who, pool);

		SharesAndWithdrawnRewards::<T>::mutate_exists(pool, who, |share_info_old| {
			if let Some(mut share_info) = share_info_old.take() {
				// (mut share, mut withdrawn_rewards)S
				let remove_amount = remove_amount.min(share_info.share);

				if remove_amount.is_zero() {
					return;
				}

				PoolInfos::<T>::mutate_exists(pool, |maybe_pool_info| {
					if let Some(mut pool_info) = maybe_pool_info.take() {
						let removing_share = U256::from(remove_amount.saturated_into::<u128>());

						pool_info.total_shares =
							pool_info.total_shares.saturating_sub(remove_amount);

						// update withdrawn rewards for each reward currency
						share_info.withdrawn_rewards.iter_mut().for_each(
							|(reward_currency, withdrawn_reward)| {
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
									*total_reward =
										total_reward.saturating_sub(withdrawn_reward_to_remove);
									*total_withdrawn_reward = total_withdrawn_reward
										.saturating_sub(withdrawn_reward_to_remove);

									// remove if all reward is withdrawn
									if total_reward.is_zero() {
										pool_info.rewards.remove(reward_currency);
									}
								}
								*withdrawn_reward =
									withdrawn_reward.saturating_sub(withdrawn_reward_to_remove);
							},
						);

						if !pool_info.total_shares.is_zero() {
							*maybe_pool_info = Some(pool_info);
						}
					}
				});

				share_info.share = share_info.share.saturating_sub(remove_amount);
				if !share_info.share.is_zero() {
					*share_info_old = Some(share_info);
				}
			}
		});
	}

	pub fn set_share(who: &T::AccountId, pool: PoolId, new_share: BalanceOf<T>) {
		let share_info = Self::shares_and_withdrawn_rewards(pool, who);

		if new_share > share_info.share {
			Self::add_share(who, pool, new_share.saturating_sub(share_info.share));
		} else {
			Self::remove_share(who, pool, share_info.share.saturating_sub(new_share));
		}
	}

	pub fn claim_rewards(who: &T::AccountId, pool: PoolId) {
		SharesAndWithdrawnRewards::<T>::mutate_exists(pool, who, |maybe_share_withdrawn| {
			if let Some(share_info) = maybe_share_withdrawn {
				if share_info.share.is_zero() {
					return;
				}

				PoolInfos::<T>::mutate(pool, |pool_info| {
					let total_shares =
						U256::from(pool_info.total_shares.to_owned().saturated_into::<u128>());
					pool_info.rewards.iter_mut().for_each(
						|(reward_currency, (total_reward, total_withdrawn_reward))| {
							let withdrawn_reward = share_info
								.withdrawn_rewards
								.get(reward_currency)
								.copied()
								.unwrap_or_default();

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
								return;
							}

							*total_withdrawn_reward =
								total_withdrawn_reward.saturating_add(reward_to_withdraw);
							share_info.withdrawn_rewards.insert(
								*reward_currency,
								withdrawn_reward.saturating_add(reward_to_withdraw),
							);

							// pay reward to `who`
							// T::Handler::payout(who, pool, *reward_currency, reward_to_withdraw);
							if let Some(ref keeper) = pool_info.keeper {
								T::MultiCurrency::transfer(
									*reward_currency,
									&keeper,
									&who,
									reward_to_withdraw,
								);
							}
						},
					);
				});
			}
		});
	}
}

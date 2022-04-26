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

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PoolInfo<Share: HasCompact, Balance: HasCompact, CurrencyId: Ord> {
	/// Total shares amount
	pub total_shares: Share,
	/// Reward infos <reward_currency, (total_reward, total_withdrawn_reward)>
	pub rewards: BTreeMap<CurrencyId, (Balance, Balance)>,
}

impl<Share, Balance, CurrencyId> Default for PoolInfo<Share, Balance, CurrencyId>
where
	Share: Default + HasCompact,
	Balance: HasCompact,
	CurrencyId: Ord,
{
	fn default() -> Self {
		Self { total_shares: Default::default(), rewards: BTreeMap::new() }
	}
}

/// PoolId for various rewards pools
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum PoolId {
	/// Rewards and shares pool for DEX makers who stake LP token(LPCurrencyId)
	Dex(CurrencyId),
}

impl<T: Config> Pallet<T> {
	pub fn accumulate_reward(
		pool: PoolId,
		reward_currency: T::CurrencyId,
		reward_increment: T::Balance,
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

	pub fn add_share(who: &T::AccountId, pool: PoolId, add_amount: T::Share) {
		if add_amount.is_zero() {
			return;
		}

		PoolInfos::<T>::mutate(pool, |pool_info| {
			let initial_total_shares = pool_info.total_shares;
			pool_info.total_shares = pool_info.total_shares.saturating_add(add_amount);

			let mut withdrawn_inflation = Vec::<(T::CurrencyId, T::Balance)>::new();

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

			SharesAndWithdrawnRewards::<T>::mutate(pool, who, |(share, withdrawn_rewards)| {
				*share = share.saturating_add(add_amount);
				// update withdrawn inflation for each reward currency
				withdrawn_inflation.into_iter().for_each(|(reward_currency, reward_inflation)| {
					withdrawn_rewards
						.entry(reward_currency)
						.and_modify(|withdrawn_reward| {
							*withdrawn_reward = withdrawn_reward.saturating_add(reward_inflation);
						})
						.or_insert(reward_inflation);
				});
			});
		});
	}

	pub fn remove_share(who: &T::AccountId, pool: PoolId, remove_amount: T::Share) {
		if remove_amount.is_zero() {
			return;
		}

		// claim rewards firstly
		Self::claim_rewards(who, pool);

		SharesAndWithdrawnRewards::<T>::mutate_exists(pool, who, |share_info| {
			if let Some((mut share, mut withdrawn_rewards)) = share_info.take() {
				let remove_amount = remove_amount.min(share);

				if remove_amount.is_zero() {
					return;
				}

				PoolInfos::<T>::mutate_exists(pool, |maybe_pool_info| {
					if let Some(mut pool_info) = maybe_pool_info.take() {
						let removing_share = U256::from(remove_amount.saturated_into::<u128>());

						pool_info.total_shares =
							pool_info.total_shares.saturating_sub(remove_amount);

						// update withdrawn rewards for each reward currency
						withdrawn_rewards.iter_mut().for_each(
							|(reward_currency, withdrawn_reward)| {
								let withdrawn_reward_to_remove: T::Balance = removing_share
									.saturating_mul(
										withdrawn_reward.to_owned().saturated_into::<u128>().into(),
									)
									.checked_div(share.saturated_into::<u128>().into())
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

				share = share.saturating_sub(remove_amount);
				if !share.is_zero() {
					*share_info = Some((share, withdrawn_rewards));
				}
			}
		});
	}

	pub fn set_share(who: &T::AccountId, pool: PoolId, new_share: T::Share) {
		let (share, _) = Self::shares_and_withdrawn_rewards(pool, who);

		if new_share > share {
			Self::add_share(who, pool, new_share.saturating_sub(share));
		} else {
			Self::remove_share(who, pool, share.saturating_sub(new_share));
		}
	}

	pub fn claim_rewards(who: &T::AccountId, pool: PoolId) {
		SharesAndWithdrawnRewards::<T>::mutate_exists(pool, who, |maybe_share_withdrawn| {
			if let Some((share, withdrawn_rewards)) = maybe_share_withdrawn {
				if share.is_zero() {
					return;
				}

				PoolInfos::<T>::mutate(pool, |pool_info| {
					let total_shares =
						U256::from(pool_info.total_shares.to_owned().saturated_into::<u128>());
					pool_info.rewards.iter_mut().for_each(
						|(reward_currency, (total_reward, total_withdrawn_reward))| {
							let withdrawn_reward =
								withdrawn_rewards.get(reward_currency).copied().unwrap_or_default();

							let total_reward_proportion: T::Balance =
								U256::from(share.to_owned().saturated_into::<u128>())
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
							withdrawn_rewards.insert(
								*reward_currency,
								withdrawn_reward.saturating_add(reward_to_withdraw),
							);

							// pay reward to `who`
							// T::Handler::payout(who, pool, *reward_currency, reward_to_withdraw);
						},
					);
				});
			}
		});
	}
}

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

use crate::*;
use codec::HasCompact;
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::{
	traits::{CheckedAdd, Saturating, UniqueSaturatedInto, Zero},
	RuntimeDebug, SaturatedConversion,
};
use sp_std::{borrow::ToOwned, collections::btree_map::BTreeMap, prelude::*};

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct ShareInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, BlockNumberFor, AccountIdOf> {
	pub who: AccountIdOf,
	pub share: BalanceOf,
	pub withdrawn_rewards: BTreeMap<CurrencyIdOf, BalanceOf>,
	pub claim_last_block: BlockNumberFor,
	pub withdraw_list: Vec<(BlockNumberFor, BalanceOf)>,
}

impl<BalanceOf, CurrencyIdOf, BlockNumberFor, AccountIdOf>
	ShareInfo<BalanceOf, CurrencyIdOf, BlockNumberFor, AccountIdOf>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord,
{
	fn new(who: AccountIdOf, claim_last_block: BlockNumberFor) -> Self {
		Self {
			who,
			share: Default::default(),
			withdrawn_rewards: BTreeMap::new(),
			claim_last_block,
			withdraw_list: Default::default(),
		}
	}
}

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PoolInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, AccountIdOf, BlockNumberFor> {
	pub tokens_proportion: BTreeMap<CurrencyIdOf, Perbill>,
	/// Total shares amount
	pub total_shares: BalanceOf,
	pub basic_rewards: BTreeMap<CurrencyIdOf, BalanceOf>,
	/// Reward infos <reward_currency, (total_reward, total_withdrawn_reward)>
	pub rewards: BTreeMap<CurrencyIdOf, (BalanceOf, BalanceOf)>,
	pub state: PoolState,
	pub keeper: AccountIdOf,
	pub reward_issuer: AccountIdOf,
	/// Gauge pool id
	pub gauge: Option<PoolId>,
	pub block_startup: Option<BlockNumberFor>,
	pub min_deposit_to_start: BalanceOf,
	pub after_block_to_start: BlockNumberFor,
	pub withdraw_limit_time: BlockNumberFor,
	pub claim_limit_time: BlockNumberFor,
	pub withdraw_limit_count: u8,
}

impl<BalanceOf, CurrencyIdOf, AccountIdOf, BlockNumberFor>
	PoolInfo<BalanceOf, CurrencyIdOf, AccountIdOf, BlockNumberFor>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord,
{
	pub fn new(
		keeper: AccountIdOf,
		reward_issuer: AccountIdOf,
		tokens_proportion: BTreeMap<CurrencyIdOf, Perbill>,
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
			keeper,
			reward_issuer,
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
		pid: PoolId,
		pool_info: &mut PoolInfo<BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>, BlockNumberFor<T>>,
		add_amount: BalanceOf<T>,
	) {
		if add_amount.is_zero() {
			return;
		}

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
				*total_withdrawn_reward = total_withdrawn_reward.saturating_add(reward_inflation);

				withdrawn_inflation.push((*reward_currency, reward_inflation));
			},
		);

		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();

		let mut share_info = SharesAndWithdrawnRewards::<T>::get(pid, who)
			.unwrap_or_else(|| ShareInfo::new(who.clone(), current_block_number));
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
		SharesAndWithdrawnRewards::<T>::insert(pid, who, share_info);
		PoolInfos::<T>::insert(&pid, pool_info);
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

		SharesAndWithdrawnRewards::<T>::mutate(pool, who, |share_info_old| -> DispatchResult {
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			if let Some(mut share_info) = share_info_old.take() {
				let remove_amount;
				if let Some(remove_amount_input) = remove_amount_input {
					remove_amount = remove_amount_input.min(share_info.share);
				} else {
					remove_amount = share_info.share;
				}

				if remove_amount.is_zero() {
					return Ok(());
				}

				PoolInfos::<T>::mutate(pool, |maybe_pool_info| -> DispatchResult {
					let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;

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
							Ok(())
						},
					)?;
					Ok(())
				})?;

				share_info.share = share_info.share.saturating_sub(remove_amount);
				*share_info_old = Some(share_info);
			}
			Ok(())
		})?;
		Ok(())
	}

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

					PoolInfos::<T>::mutate(pool, |maybe_pool_info| -> DispatchResult {
						let pool_info =
							maybe_pool_info.as_mut().ok_or(Error::<T>::PoolDoesNotExist)?;

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

								let ed = T::MultiCurrency::minimum_balance(*reward_currency);
								let mut account_to_send = who.clone();

								if reward_to_withdraw < ed {
									let receiver_balance = T::MultiCurrency::total_balance(*reward_currency, &who);

									let receiver_balance_after =
										receiver_balance.checked_add(&reward_to_withdraw).ok_or(ArithmeticError::Overflow)?;
									if receiver_balance_after < ed {
										account_to_send = T::TreasuryAccount::get();
									}
								}
								// pay reward to `who`
								T::MultiCurrency::transfer(
									*reward_currency,
									&pool_info.reward_issuer,
									&account_to_send,
									reward_to_withdraw,
								)
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
		if_remove: bool,
	) -> DispatchResult {
		SharesAndWithdrawnRewards::<T>::mutate_exists(
			pool,
			who,
			|share_info_old| -> DispatchResult {
				if let Some(mut share_info) = share_info_old.take() {
					let current_block_number: BlockNumberFor<T> =
						frame_system::Pallet::<T>::block_number();
					let mut tmp: Vec<(BlockNumberFor<T>, BalanceOf<T>)> = Default::default();
					let tokens_proportion_values: Vec<Perbill> =
						pool_info.tokens_proportion.values().cloned().collect();
					share_info.withdraw_list.iter().try_for_each(
						|(dest_block, remove_value)| -> DispatchResult {
							if *dest_block <= current_block_number {
								let native_amount = tokens_proportion_values[0]
									.saturating_reciprocal_mul(*remove_value);
								pool_info.tokens_proportion.iter().try_for_each(
									|(token, &proportion)| -> DispatchResult {
										let withdraw_amount = proportion * native_amount;
										let ed = T::MultiCurrency::minimum_balance(*token);
										let mut account_to_send = who.clone();

										if withdraw_amount < ed {
											let receiver_balance =
												T::MultiCurrency::total_balance(*token, &who);

											let receiver_balance_after = receiver_balance
												.checked_add(&withdraw_amount)
												.ok_or(ArithmeticError::Overflow)?;
											if receiver_balance_after < ed {
												account_to_send = T::TreasuryAccount::get();
											}
										}
										T::MultiCurrency::transfer(
											*token,
											&pool_info.keeper,
											&account_to_send,
											withdraw_amount,
										)
									},
								)?;
							} else {
								tmp.push((*dest_block, *remove_value));
							};
							Ok(())
						},
					)?;
					share_info.withdraw_list = tmp;

					// if withdraw_list and share both are empty, and if_remove is true, remove it.
					if share_info.withdraw_list !=
						Vec::<(BlockNumberFor<T>, BalanceOf<T>)>::default() ||
						!share_info.share.is_zero() ||
						!if_remove
					{
						*share_info_old = Some(share_info);
					};
				};
				Ok(())
			},
		)
	}
}

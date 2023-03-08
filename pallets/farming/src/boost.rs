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
use bifrost_ve_minting::VeMintingInterface;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct BoostPoolInfo<CurrencyId, Balance, BlockNumber> {
	pub rewards: BTreeMap<CurrencyId, Balance>, // Total rewards
	pub basic_rewards: BTreeMap<CurrencyId, Balance>, // Basic rewards per block
	pub voting_pools: BTreeMap<PoolId, Balance>, // Vec<(PoolId, Balance)>
	pub total_votes: Balance,                   // Total number of veBNC voting
	pub start_round: BlockNumber,
	pub end_round: BlockNumber,
	pub round_length: BlockNumber,
	pub whitelist: Vec<PoolId>, // Need to be sorted and deduplicated
	pub next_round_whitelist: Vec<PoolId>, // Need to be sorted and deduplicated
}

#[derive(Clone, Encode, Decode, TypeInfo)]
pub struct UserBoostInfo<Balance, BlockNumber> {
	pub vote_amount: Balance,
	pub vote_list: Vec<(PoolId, Percent)>,
	pub last_block: BlockNumber,
}

pub trait BoostInterface<AccountId, CurrencyId, Balance, BlockNumber> {
	fn refresh_vebnc_farming(who: &AccountId) -> DispatchResult;
}

impl<T: Config> BoostInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>
	for Pallet<T>
{
	fn refresh_vebnc_farming(who: &AccountIdOf<T>) -> DispatchResult {
		let mut boost_pool_info = Self::boost_pool_infos();
		let new_vote_amount = T::VeMinting::balance_of(who, None)?;

		// TODO: if boost_pool_info is default, return
		if let Some(mut user_boost_info) = Self::user_boost_infos(who) {
			// If the user's last voting block height is greater than or equal to the block height
			// at the beginning of this round, subtract.
			if user_boost_info.last_block >= boost_pool_info.start_round {
				user_boost_info.vote_list.iter().for_each(|(pid, proportion)| {
					boost_pool_info
						.voting_pools
						.entry(*pid)
						.and_modify(|total_votes| {
							*total_votes = total_votes
								.saturating_sub(*proportion * user_boost_info.vote_amount);
						})
						.or_insert(Zero::zero());
					boost_pool_info
						.voting_pools
						.entry(*pid)
						.and_modify(|total_votes| {
							*total_votes =
								total_votes.saturating_add(*proportion * new_vote_amount);
						})
						.or_insert(*proportion * new_vote_amount);
				});
				boost_pool_info.total_votes.saturating_sub(user_boost_info.vote_amount);
				boost_pool_info.total_votes.saturating_add(new_vote_amount);
				BoostPoolInfos::<T>::set(boost_pool_info);
				user_boost_info.vote_amount = new_vote_amount;
				UserBoostInfos::<T>::insert(who, user_boost_info);
			}
		}
		Ok(())
	}
}

impl<T: Config> Pallet<T> {
	pub(crate) fn start_boost_round_inner(round_length: BlockNumberFor<T>) -> DispatchResult {
		ensure!(round_length != Zero::zero(), Error::<T>::RoundLengthNotSet);
		let mut boost_pool_info = Self::boost_pool_infos();
		ensure!(boost_pool_info.end_round == Zero::zero(), Error::<T>::RoundNotOver);
		// Update whitelist
		if !boost_pool_info.next_round_whitelist.is_empty() {
			boost_pool_info.whitelist = boost_pool_info.next_round_whitelist;
			boost_pool_info.next_round_whitelist = Vec::<PoolId>::new();
		} else {
			ensure!(!boost_pool_info.whitelist.is_empty(), Error::<T>::WhitelistEmpty);
		}

		Self::send_boost_rewards(&boost_pool_info)?;
		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number();
		boost_pool_info.start_round = current_block_number;
		boost_pool_info.round_length = round_length;
		boost_pool_info.end_round = current_block_number + round_length;
		BoostPoolInfos::<T>::set(boost_pool_info);
		Self::deposit_event(Event::RoundStart { round_length });
		Ok(())
	}

	pub(crate) fn end_boost_round_inner() {
		let mut boost_pool_info = Self::boost_pool_infos();
		// Empty BoostBasicRewards
		boost_pool_info
			.voting_pools
			.iter()
			.filter_map(|(pid, _)| match Self::pool_infos(pid) {
				Some(pool_info) => Some((pid, pool_info)),
				None => None,
			})
			.for_each(|(pid, pool_info)| {
				pool_info.basic_rewards.keys().for_each(|currency| {
					BoostBasicRewards::<T>::mutate_exists(pid, currency, |value| *value = None);
				});
			});

		Self::deposit_event(Event::RoundEnd {
			voting_pools: boost_pool_info.voting_pools,
			total_votes: boost_pool_info.total_votes,
			start_round: boost_pool_info.start_round,
			end_round: boost_pool_info.end_round,
		});
		boost_pool_info.start_round = Zero::zero();
		boost_pool_info.voting_pools = BTreeMap::<PoolId, BalanceOf<T>>::new();
		boost_pool_info.total_votes = Zero::zero();
		boost_pool_info.end_round = Zero::zero();
		BoostPoolInfos::<T>::set(boost_pool_info);
	}

	// Only used in hook
	pub(crate) fn auto_start_boost_round() {
		let mut boost_pool_info = Self::boost_pool_infos();
		if !boost_pool_info.next_round_whitelist.is_empty() {
			boost_pool_info.whitelist = boost_pool_info.next_round_whitelist;
			boost_pool_info.next_round_whitelist = Vec::<PoolId>::new();
		} else if boost_pool_info.whitelist.is_empty() {
			return;
		}

		Self::send_boost_rewards(&boost_pool_info)
			.map_err(|e| {
				Self::deposit_event(Event::RoundStartError { info: e });
			})
			.ok();
		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number();
		boost_pool_info.start_round = current_block_number;
		boost_pool_info.end_round = current_block_number + boost_pool_info.round_length;
		Self::deposit_event(Event::RoundStart { round_length: boost_pool_info.round_length });
		BoostPoolInfos::<T>::set(boost_pool_info);
	}

	pub(crate) fn send_boost_rewards(
		boost_pool_info: &BoostPoolInfo<CurrencyIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>,
	) -> DispatchResult {
		boost_pool_info
			.voting_pools
			.iter()
			.filter_map(|(pid, value)| match Self::pool_infos(pid) {
				Some(pool_info) => Some((pid, value, pool_info)),
				None => None,
			})
			.try_for_each(|(pid, value, pool_info)| -> DispatchResult {
				let proportion = Percent::from_rational(*value, boost_pool_info.total_votes);
				pool_info.basic_rewards.keys().try_for_each(|currency| -> DispatchResult {
					// proportion * free_balance
					let transfer_balance: BalanceOf<T> =
						proportion.mul_floor(T::MultiCurrency::free_balance(
							*currency,
							&T::FarmingBoost::get().into_account_truncating(),
						));

					BoostBasicRewards::<T>::mutate_exists(pid, currency, |value| {
						// Store None if overflow
						*value = transfer_balance.checked_div(&T::BlockNumberToBalance::convert(
							boost_pool_info.round_length,
						));
					});
					T::MultiCurrency::transfer(
						*currency,
						&T::FarmingBoost::get().into_account_truncating(),
						&T::RewardIssuer::get().into_sub_account_truncating(pid),
						transfer_balance,
					)
				})?;

				Ok(())
			})
	}

	pub(crate) fn vote_inner(
		who: &AccountIdOf<T>,
		vote_list: Vec<(PoolId, Percent)>,
	) -> DispatchResult {
		let current_block_number = frame_system::Pallet::<T>::block_number();
		let mut boost_pool_info = Self::boost_pool_infos();

		if let Some(user_boost_info) = Self::user_boost_infos(who) {
			// If the user's last voting block height is greater than or equal to the block height
			// at the beginning of this round, subtract.
			if user_boost_info.last_block >= boost_pool_info.start_round {
				user_boost_info.vote_list.iter().for_each(|(pid, proportion)| {
					boost_pool_info
						.voting_pools
						.entry(*pid)
						.and_modify(|total_votes| {
							*total_votes = total_votes
								.saturating_sub(*proportion * user_boost_info.vote_amount);
						})
						.or_insert(Zero::zero());
				});
				boost_pool_info.total_votes.saturating_sub(user_boost_info.vote_amount);
			}
		}

		let new_vote_amount = T::VeMinting::balance_of(who, None)?;
		vote_list.iter().try_for_each(|(pid, proportion)| -> DispatchResult {
			boost_pool_info
				.whitelist
				.binary_search(pid)
				.map_err(|_| Error::<T>::CalculationOverflow)?;
			boost_pool_info
				.voting_pools
				.entry(*pid)
				.and_modify(|total_votes| {
					*total_votes = total_votes.saturating_add(*proportion * new_vote_amount);
				})
				.or_insert(*proportion * new_vote_amount);
			boost_pool_info.total_votes.saturating_add(new_vote_amount);
			Ok(())
		})?;
		BoostPoolInfos::<T>::set(boost_pool_info);
		let new_user_boost_info = UserBoostInfo {
			vote_amount: new_vote_amount,
			vote_list,
			last_block: current_block_number,
		};
		UserBoostInfos::<T>::insert(who, new_user_boost_info);
		Ok(())
	}
}

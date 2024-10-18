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

use crate::*;
use bb_bnc::BbBNCInterface;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct BoostPoolInfo<Balance, BlockNumber> {
	pub total_votes: Balance, // Total number of veBNC voting
	pub start_round: BlockNumber,
	pub end_round: BlockNumber,
	pub round_length: BlockNumber,
}

#[derive(Clone, Encode, Decode, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UserBoostInfo<T: Config> {
	pub vote_amount: BalanceOf<T>,
	pub vote_list: BoundedVec<(PoolId, Percent), T::WhitelistMaximumLimit>,
	pub last_vote: BlockNumberFor<T>, // Change only when voting
}

pub trait BoostInterface<AccountId, CurrencyId, Balance, BlockNumber> {
	fn refresh_vebnc_farming(who: &AccountId) -> DispatchResult;
}

impl<T: Config> BoostInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>
	for Pallet<T>
{
	fn refresh_vebnc_farming(who: &AccountIdOf<T>) -> DispatchResult {
		let mut boost_pool_info = BoostPoolInfos::<T>::get();
		let new_vote_amount = T::BbBNC::balance_of(who, None)?;

		if let Some(mut user_boost_info) = UserBoostInfos::<T>::get(who) {
			// If the user's last voting block height is greater than or equal to the block height
			// at the beginning of this round, refresh.
			if user_boost_info.last_vote >= boost_pool_info.start_round {
				user_boost_info.vote_list.iter().try_for_each(
					|(pid, proportion)| -> DispatchResult {
						BoostVotingPools::<T>::mutate(pid, |maybe_total_votes| -> DispatchResult {
							// Must have been voted.
							let total_votes =
								maybe_total_votes.as_mut().ok_or(Error::<T>::NobodyVoting)?;
							*total_votes = total_votes
								.checked_sub(&(*proportion * user_boost_info.vote_amount))
								.ok_or(ArithmeticError::Overflow)?;
							*total_votes = total_votes
								.checked_add(&(*proportion * new_vote_amount))
								.ok_or(ArithmeticError::Overflow)?;
							Ok(())
						})
					},
				)?;
				boost_pool_info.total_votes = boost_pool_info
					.total_votes
					.checked_sub(&user_boost_info.vote_amount)
					.ok_or(ArithmeticError::Overflow)?;
				boost_pool_info.total_votes = boost_pool_info
					.total_votes
					.checked_add(&new_vote_amount)
					.ok_or(ArithmeticError::Overflow)?;
				BoostPoolInfos::<T>::set(boost_pool_info);
				user_boost_info.vote_amount = new_vote_amount;
				UserBoostInfos::<T>::insert(who, user_boost_info);
			}
		}
		Ok(())
	}
}

impl<T: Config> Pallet<T> {
	// Update whitelist, send boost rewards to the corresponding farming pool and record
	// BoostBasicRewards, then clear BoostVotingPools and boost_pool_info.total_votes to initialize
	// the next round.
	pub(crate) fn start_boost_round_inner(round_length: BlockNumberFor<T>) -> DispatchResult {
		ensure!(round_length != Zero::zero(), Error::<T>::RoundLengthNotSet);
		let mut boost_pool_info = BoostPoolInfos::<T>::get();
		ensure!(boost_pool_info.end_round == Zero::zero(), Error::<T>::RoundNotOver);

		// Update whitelist
		if BoostNextRoundWhitelist::<T>::iter_keys().count() != 0 {
			let _ = BoostWhitelist::<T>::clear(u32::max_value(), None);
			BoostNextRoundWhitelist::<T>::iter_keys().for_each(|pid| {
				BoostWhitelist::<T>::insert(pid, ());
			});
			let _ = BoostNextRoundWhitelist::<T>::clear(u32::max_value(), None);
		} else {
			ensure!(BoostWhitelist::<T>::iter_keys().count() != 0, Error::<T>::WhitelistEmpty);
		}

		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		boost_pool_info.start_round = current_block_number;
		boost_pool_info.end_round = current_block_number.saturating_add(round_length);
		boost_pool_info.total_votes = Zero::zero();
		boost_pool_info.round_length = round_length;
		Self::send_boost_rewards(&boost_pool_info)?;
		BoostPoolInfos::<T>::set(boost_pool_info);
		let _ = BoostVotingPools::<T>::clear(u32::max_value(), None);
		Self::deposit_event(Event::RoundStart { round_length });
		Ok(())
	}

	// Clear boost_basic_rewards and boost_pool_info.end_round to eliminate the influence of boost
	// in hook
	pub(crate) fn end_boost_round_inner() {
		let mut boost_pool_info = BoostPoolInfos::<T>::get();
		let _ = BoostBasicRewards::<T>::clear(u32::max_value(), None);
		Self::deposit_event(Event::RoundEnd {
			total_votes: boost_pool_info.total_votes,
			start_round: boost_pool_info.start_round,
			end_round: boost_pool_info.end_round,
		});
		boost_pool_info.start_round = Zero::zero();
		boost_pool_info.end_round = Zero::zero();
		BoostPoolInfos::<T>::set(boost_pool_info);
	}

	// Only used in hook
	pub(crate) fn auto_start_boost_round() {
		let mut boost_pool_info = BoostPoolInfos::<T>::get();
		let whitelist_iter = BoostWhitelist::<T>::iter_keys();
		// Update whitelist
		if BoostNextRoundWhitelist::<T>::iter().count() != 0 {
			let _ = BoostWhitelist::<T>::clear(u32::max_value(), None);
			whitelist_iter.for_each(|pid| {
				BoostWhitelist::<T>::insert(pid, ());
			});
			let _ = BoostNextRoundWhitelist::<T>::clear(u32::max_value(), None);
		} else if whitelist_iter.count() == 0 {
			return;
		}

		Self::send_boost_rewards(&boost_pool_info)
			.map_err(|e| {
				Self::deposit_event(Event::RoundStartError { info: e });
			})
			.ok();
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		boost_pool_info.start_round = current_block_number;
		boost_pool_info.end_round =
			current_block_number.saturating_add(boost_pool_info.round_length);
		boost_pool_info.total_votes = Zero::zero();
		Self::deposit_event(Event::RoundStart { round_length: boost_pool_info.round_length });
		BoostPoolInfos::<T>::set(boost_pool_info);
		let _ = BoostVotingPools::<T>::clear(u32::max_value(), None);
	}

	pub(crate) fn send_boost_rewards(
		boost_pool_info: &BoostPoolInfo<BalanceOf<T>, BlockNumberFor<T>>,
	) -> DispatchResult {
		BoostVotingPools::<T>::iter()
			.filter_map(|(pid, value)| match PoolInfos::<T>::get(pid) {
				Some(pool_info) => Some((pid, value, pool_info)),
				None => None,
			})
			.try_for_each(|(pid, value, pool_info)| -> DispatchResult {
				let proportion = Percent::from_rational(value, boost_pool_info.total_votes);
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
		let mut boost_pool_info = BoostPoolInfos::<T>::get();

		if let Some(user_boost_info) = UserBoostInfos::<T>::get(who) {
			// If the user's last voting block height is greater than or equal to the block height
			// at the beginning of this round, subtract.
			if user_boost_info.last_vote >= boost_pool_info.start_round {
				user_boost_info.vote_list.iter().try_for_each(
					|(pid, proportion)| -> DispatchResult {
						BoostVotingPools::<T>::mutate(pid, |maybe_total_votes| -> DispatchResult {
							// Must have been voted.
							let total_votes =
								maybe_total_votes.as_mut().ok_or(Error::<T>::NobodyVoting)?;
							*total_votes = total_votes
								.checked_sub(&(*proportion * user_boost_info.vote_amount))
								.ok_or(ArithmeticError::Overflow)?;
							Ok(())
						})
					},
				)?;
				boost_pool_info.total_votes = boost_pool_info
					.total_votes
					.checked_sub(&user_boost_info.vote_amount)
					.ok_or(ArithmeticError::Overflow)?;
			}
		}

		let new_vote_amount = T::BbBNC::balance_of(who, None)?;
		let mut percent_check = Percent::from_percent(0);
		vote_list.iter().try_for_each(|(pid, proportion)| -> DispatchResult {
			ensure!(BoostWhitelist::<T>::get(pid) != None, Error::<T>::NotInWhitelist);
			let increace = *proportion * new_vote_amount;
			percent_check =
				percent_check.checked_add(proportion).ok_or(Error::<T>::PercentOverflow)?;
			BoostVotingPools::<T>::mutate(pid, |maybe_total_votes| -> DispatchResult {
				match maybe_total_votes.as_mut() {
					Some(total_votes) =>
						*total_votes =
							total_votes.checked_add(&increace).ok_or(ArithmeticError::Overflow)?,
					None => *maybe_total_votes = Some(increace),
				}
				Ok(())
			})?;
			boost_pool_info.total_votes = boost_pool_info
				.total_votes
				.checked_add(&new_vote_amount)
				.ok_or(ArithmeticError::Overflow)?;
			Ok(())
		})?;
		BoostPoolInfos::<T>::set(boost_pool_info);

		let vote_list_bound =
			BoundedVec::<(PoolId, Percent), T::WhitelistMaximumLimit>::try_from(vote_list)
				.map_err(|_| Error::<T>::WhitelistLimitExceeded)?;
		let new_user_boost_info = UserBoostInfo {
			vote_amount: new_vote_amount,
			vote_list: vote_list_bound,
			last_vote: current_block_number,
		};
		UserBoostInfos::<T>::insert(who, new_user_boost_info);
		Ok(())
	}
}

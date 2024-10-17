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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod boost;
pub mod gauge;
pub mod rewards;
pub mod weights;
pub use weights::WeightInfo;

use crate::boost::*;
use bifrost_primitives::{FarmingInfo, PoolId};
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{
			AccountIdConversion, AtLeast32BitUnsigned, CheckedAdd, CheckedDiv, CheckedSub, Convert,
			Saturating, Zero,
		},
		ArithmeticError, Perbill, Percent,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
pub use gauge::*;
use orml_traits::MultiCurrency;
pub use pallet::*;
pub use rewards::*;
use sp_runtime::SaturatedConversion;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

use bb_bnc::BbBNCInterface;
use parity_scale_codec::FullCodec;
use sp_std::fmt::Debug;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type CurrencyId: FullCodec
			+ Eq
			+ PartialEq
			+ Copy
			+ MaybeSerializeDeserialize
			+ Debug
			+ scale_info::TypeInfo
			+ MaxEncodedLen
			+ Ord
			+ Default;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = Self::CurrencyId>;

		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Set default weight.
		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type Keeper: Get<PalletId>;

		#[pallet::constant]
		type RewardIssuer: Get<PalletId>;

		#[pallet::constant]
		type FarmingBoost: Get<PalletId>;

		type BbBNC: bb_bnc::BbBNCInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
			BlockNumberFor<Self>,
		>;

		type BlockNumberToBalance: Convert<BlockNumberFor<Self>, BalanceOf<Self>>;

		#[pallet::constant]
		type WhitelistMaximumLimit: Get<u32>;

		#[pallet::constant]
		type GaugeRewardIssuer: Get<PalletId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A farming pool is created.
		FarmingPoolCreated {
			/// The pool id of the new pool.
			pid: PoolId,
		},
		/// The farming pool is reset.
		FarmingPoolReset {
			/// The pool id of the pool to reset.
			pid: PoolId,
		},
		/// The farming pool is closed.
		FarmingPoolClosed {
			/// The pool id of the pool to close.
			pid: PoolId,
		},
		/// The farming pool is killed.
		FarmingPoolKilled {
			/// The pool id of the pool to kill.
			pid: PoolId,
		},
		/// The farming pool is edited.
		FarmingPoolEdited {
			/// The pool id of the pool to edit.
			pid: PoolId,
		},
		/// The pool is charged.
		Charged {
			/// The exchanger who charged the pool.
			who: AccountIdOf<T>,
			/// Charged pool id.
			pid: PoolId,
			/// Charged rewards.
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
			/// Returns true if the reward is for gauge pool, false otherwise.
			if_gauge: bool,
		},
		/// The pool is deposited.
		Deposited {
			/// The exchanger who deposited the pool.
			who: AccountIdOf<T>,
			/// Deposited pool id.
			pid: PoolId,
			/// Deposited value.
			add_value: BalanceOf<T>,
		},
		/// The pool is withdrawn.
		Withdrawn {
			/// The exchanger who withdrew the pool.
			who: AccountIdOf<T>,
			/// Withdrawn pool id.
			pid: PoolId,
			/// Withdrawn value.
			remove_value: Option<BalanceOf<T>>,
		},
		/// The pool is claimed.
		Claimed {
			/// The exchanger who claimed the pool.
			who: AccountIdOf<T>,
			/// Claimed pool id.
			pid: PoolId,
		},
		/// The pool is withdrawn claimed.
		WithdrawClaimed {
			/// The exchanger who withdrew claimed the pool.
			who: AccountIdOf<T>,
			/// Withdraw claimed pool id.
			pid: PoolId,
		},
		/// The gauge pool is withdrawn.
		GaugeWithdrawn {
			/// The exchanger who withdrew the gauge pool.
			who: AccountIdOf<T>,
			/// Withdrawn gauge pool id.
			gid: PoolId,
		},
		/// All gauge pools have been claimed.
		AllForceGaugeClaimed {
			/// Last claimed gauge pool id.
			gid: PoolId,
		},
		/// Partially gauge pools have been claimed.
		PartiallyForceGaugeClaimed {
			/// Last claimed gauge pool id.
			gid: PoolId,
		},
		/// All pools is retired.
		AllRetired {
			/// Last retired pool id.
			pid: PoolId,
		},
		/// Partially pools is retired.
		PartiallyRetired {
			/// Last retired pool id.
			pid: PoolId,
		},
		/// The retire limit is set.
		RetireLimitSet {
			/// The retire limit.
			limit: u32,
		},
		/// The round has ended.
		RoundEnd {
			// voting_pools: BTreeMap<PoolId, BalanceOf<T>>,
			total_votes: BalanceOf<T>,
			/// The start block of the round.
			start_round: BlockNumberFor<T>,
			/// The end block of the round.
			end_round: BlockNumberFor<T>,
		},
		/// The round has started to fail.
		RoundStartError {
			/// The error
			info: DispatchError,
		},
		/// The round has started.
		RoundStart {
			/// The length of the round.
			round_length: BlockNumberFor<T>,
		},
		/// The exchanger is voted.
		Voted {
			/// The exchanger who voted.
			who: AccountIdOf<T>,
			/// Voted pool id.
			vote_list: Vec<(PoolId, Percent)>,
		},
		/// The boost pool is charged.
		BoostCharged {
			/// The exchanger who charged the boost pool.
			who: AccountIdOf<T>,
			/// Charged boost pool id.
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The field tokens_proportion cannot be empty.
		NotNullable,
		/// The pool does not exist.
		PoolDoesNotExist,
		/// The gauge pool does not exist.
		GaugePoolNotExist,
		/// The gauge info does not exist.
		GaugeInfoNotExist,
		/// The pool is not in the correct state.
		InvalidPoolState,
		/// claim_limit_time exceeded
		CanNotClaim,
		/// gauge pool max_block exceeded
		GaugeMaxBlockOverflow,
		/// withdraw_limit_time exceeded
		WithdrawLimitCountExceeded,
		/// User's personal share info does not exist
		ShareInfoNotExists,
		/// The current block height needs to be greater than the field after_block_to_start in
		/// order to execute deposit.
		CanNotDeposit,
		/// Whitelist cannot be empty
		WhitelistEmpty,
		/// When starting a round, the field end_round needs to be 0 to indicate that the previous
		/// round has ended.
		RoundNotOver,
		/// The round length needs to be set when starting a round
		RoundLengthNotSet,
		/// Whitelist maximum limit exceeded
		WhitelistLimitExceeded,
		/// No one voted for this pool.
		NobodyVoting,
		/// The pool is not in the whitelist
		NotInWhitelist,
		/// The total voting percentage of users cannot exceed 100%.
		PercentOverflow,
		/// The pool cannot be cleaned completely
		PoolNotCleared,
		/// Invalid remove amount
		InvalidRemoveAmount,
	}

	/// Record the id of the new pool.
	#[pallet::storage]
	pub type PoolNextId<T: Config> = StorageValue<_, PoolId, ValueQuery>;

	/// Record the id of the new gauge pool.
	#[pallet::storage]
	pub type GaugePoolNextId<T: Config> = StorageValue<_, PoolId, ValueQuery>;

	/// The upper limit of a single retirement pool
	#[pallet::storage]
	pub type RetireLimit<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Record reward pool info.
	///
	/// map PoolId => PoolInfo
	#[pallet::storage]
	pub type PoolInfos<T: Config> = StorageMap<
		_,
		Twox64Concat,
		PoolId,
		PoolInfo<BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>, BlockNumberFor<T>>,
	>;

	/// Record gauge farming pool info.
	///
	/// map PoolId => GaugePoolInfo
	#[pallet::storage]
	pub type GaugePoolInfos<T: Config> = StorageMap<
		_,
		Twox64Concat,
		PoolId,
		GaugePoolInfo<BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>, BlockNumberFor<T>>,
	>;

	/// Record gauge config
	#[pallet::storage]
	pub type GaugeInfos<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		PoolId,
		Twox64Concat,
		T::AccountId,
		GaugeInfo<BalanceOf<T>, BlockNumberFor<T>, AccountIdOf<T>>,
	>;

	/// Record share amount, reward currency and withdrawn reward amount for
	/// specific `AccountId` under `PoolId`.
	///
	/// double_map (PoolId, AccountId) => ShareInfo
	#[pallet::storage]
	pub type SharesAndWithdrawnRewards<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		PoolId,
		Twox64Concat,
		T::AccountId,
		ShareInfo<BalanceOf<T>, CurrencyIdOf<T>, BlockNumberFor<T>, AccountIdOf<T>>,
	>;

	/// Record all voting pool information.
	#[pallet::storage]
	pub type BoostPoolInfos<T: Config> =
		StorageValue<_, BoostPoolInfo<BalanceOf<T>, BlockNumberFor<T>>, ValueQuery>;

	/// Record the voting pool id and the voting percentage of the user.
	#[pallet::storage]
	pub type UserBoostInfos<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, UserBoostInfo<T>>;

	/// Record the pools which the user can voted for.
	#[pallet::storage]
	pub type BoostWhitelist<T: Config> = StorageMap<_, Twox64Concat, PoolId, ()>;

	/// Record the pools which the user can voted for in the next round.
	#[pallet::storage]
	pub type BoostNextRoundWhitelist<T: Config> = StorageMap<_, Twox64Concat, PoolId, ()>;

	/// Record the voting amount for each pool.
	#[pallet::storage]
	pub type BoostVotingPools<T: Config> = StorageMap<_, Twox64Concat, PoolId, BalanceOf<T>>;

	/// Voting rewards for corresponding currency.
	#[pallet::storage]
	pub type BoostBasicRewards<T: Config> =
		StorageDoubleMap<_, Twox64Concat, PoolId, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			PoolInfos::<T>::iter().for_each(|(pid, mut pool_info)| match pool_info.state {
				PoolState::Ongoing => {
					pool_info.basic_rewards.clone().iter_mut().for_each(
						|(reward_currency_id, reward_amount)| {
							if let Some(boost_basic_reward) =
								BoostBasicRewards::<T>::get(pid, reward_currency_id)
							{
								*reward_amount = reward_amount.saturating_add(boost_basic_reward);
							}
							pool_info
								.rewards
								.entry(*reward_currency_id)
								.and_modify(|(total_reward, _)| {
									*total_reward = total_reward.saturating_add(*reward_amount);
								})
								.or_insert((*reward_amount, Zero::zero()));
						},
					);
					PoolInfos::<T>::insert(pid, &pool_info);
				},
				PoolState::Charged => {
					if n >= pool_info.after_block_to_start &&
						pool_info.total_shares >= pool_info.min_deposit_to_start
					{
						pool_info.block_startup = Some(n);
						pool_info.state = PoolState::Ongoing;
						PoolInfos::<T>::insert(pid, &pool_info);
					}
				},
				_ => (),
			});

			GaugePoolInfos::<T>::iter().for_each(|(gid, gauge_pool_info)| {
				match gauge_pool_info.gauge_state {
					GaugeState::Bonded => {
						let rewards = gauge_pool_info.gauge_basic_rewards.into_iter().collect();
						T::BbBNC::auto_notify_reward(gid, n, rewards).unwrap_or_default();
					},
					_ => (),
				}
			});

			if n == BoostPoolInfos::<T>::get().end_round {
				Self::end_boost_round_inner();
				Self::auto_start_boost_round();
			}

			T::WeightInfo::on_initialize()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		BlockNumberFor<T>: AtLeast32BitUnsigned + Copy,
		BalanceOf<T>: AtLeast32BitUnsigned + Copy,
	{
		/// Create a farming pool.
		///
		/// The state of the pool will be set to `Ongoing` if the current block number is greater
		/// than or equal to the field `after_block_to_start` or the total shares of the pool is
		/// greater than or equal to the field `min_deposit_to_start`.
		///
		/// - `tokens_proportion`: The proportion of each token in the pool.
		/// - `basic_rewards`: The basic reward of each token in the pool.
		/// - `gauge_init`: The initial gauge pool info.
		/// - `min_deposit_to_start`: The minimum deposit to start the pool.
		/// - `after_block_to_start`: The block number to start the pool.
		/// - `withdraw_limit_time`: The block number to limit the withdraw.
		/// - `claim_limit_time`: The block number to limit the claim.
		/// - `withdraw_limit_count`: The count to limit the withdraw.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_farming_pool())]
		pub fn create_farming_pool(
			origin: OriginFor<T>,
			tokens_proportion: Vec<(CurrencyIdOf<T>, Perbill)>,
			basic_rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
			gauge_init: Option<(BlockNumberFor<T>, Vec<(CurrencyIdOf<T>, BalanceOf<T>)>)>,
			min_deposit_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
			#[pallet::compact] withdraw_limit_time: BlockNumberFor<T>,
			#[pallet::compact] claim_limit_time: BlockNumberFor<T>,
			withdraw_limit_count: u8,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let pid = PoolNextId::<T>::get();
			let keeper = T::Keeper::get().into_sub_account_truncating(pid);
			let reward_issuer = T::RewardIssuer::get().into_sub_account_truncating(pid);
			let basic_token = *tokens_proportion.get(0).ok_or(Error::<T>::NotNullable)?;
			let tokens_proportion_map: BTreeMap<CurrencyIdOf<T>, Perbill> =
				tokens_proportion.into_iter().collect();
			let basic_rewards_map: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> =
				basic_rewards.into_iter().collect();

			let mut pool_info = PoolInfo::new(
				keeper,
				reward_issuer,
				tokens_proportion_map,
				basic_token,
				basic_rewards_map,
				None,
				min_deposit_to_start,
				after_block_to_start,
				withdraw_limit_time,
				claim_limit_time,
				withdraw_limit_count,
			);

			if let Some((max_block, gauge_basic_rewards)) = gauge_init {
				let gauge_basic_rewards_map: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> =
					gauge_basic_rewards.into_iter().collect();

				Self::create_gauge_pool(pid, &mut pool_info, gauge_basic_rewards_map, max_block)?;
			};

			PoolInfos::<T>::insert(pid, &pool_info);
			PoolNextId::<T>::mutate(|id| -> DispatchResult {
				*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::FarmingPoolCreated { pid });
			Ok(())
		}

		/// Charge the pool.
		///
		/// Transfer the rewards from the exchanger to the pool. It will charge the rewards to the
		/// gauge pool if the `if_gauge` is true, otherwise it will charge the rewards to the
		/// farming pool.
		///
		/// - `pid`: The pool id.
		/// - `rewards`: The rewards to charge.
		/// - `if_gauge`: If the rewards are for the gauge pool.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::charge())]
		pub fn charge(
			origin: OriginFor<T>,
			pid: PoolId,
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
			if_gauge: bool,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			let mut pool_info = PoolInfos::<T>::get(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;

			match if_gauge {
				true => {
					let gauge_reward_issuer =
						T::GaugeRewardIssuer::get().into_sub_account_truncating(pid);
					rewards.iter().try_for_each(|(reward_currency, reward)| -> DispatchResult {
						T::MultiCurrency::transfer(
							*reward_currency,
							&exchanger,
							&gauge_reward_issuer,
							*reward,
						)
					})?;
				},
				false => {
					ensure!(
						pool_info.state == PoolState::UnCharged ||
							pool_info.state == PoolState::Ongoing,
						Error::<T>::InvalidPoolState
					);
					rewards.iter().try_for_each(|(reward_currency, reward)| -> DispatchResult {
						T::MultiCurrency::transfer(
							*reward_currency,
							&exchanger,
							&pool_info.reward_issuer,
							*reward,
						)
					})?;
					if pool_info.state == PoolState::UnCharged {
						pool_info.state = PoolState::Charged
					}
					PoolInfos::<T>::insert(&pid, pool_info);
				},
			};

			Self::deposit_event(Event::Charged { who: exchanger, pid, rewards, if_gauge });
			Ok(())
		}

		/// Deposit the pool.
		///
		/// Mint the share to the exchanger and transfer the tokens to the pool. The state of the
		/// pool should be `Ongoing` or `Charged`. The current block number should be greater than
		/// or equal to the field `after_block_to_start` if the state of the pool is `Charged`.
		///
		/// - `pid`: The pool id.
		/// - `add_value`: The value to deposit.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::deposit())]
		pub fn deposit(
			origin: OriginFor<T>,
			pid: PoolId,
			add_value: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let mut pool_info = PoolInfos::<T>::get(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(
				pool_info.state == PoolState::Ongoing || pool_info.state == PoolState::Charged,
				Error::<T>::InvalidPoolState
			);

			if let PoolState::Charged = pool_info.state {
				let current_block_number: BlockNumberFor<T> =
					frame_system::Pallet::<T>::block_number();
				ensure!(
					current_block_number >= pool_info.after_block_to_start,
					Error::<T>::CanNotDeposit
				);
			}

			let native_amount = pool_info.basic_token.1.saturating_reciprocal_mul(add_value);
			pool_info.tokens_proportion.iter().try_for_each(
				|(token, proportion)| -> DispatchResult {
					T::MultiCurrency::transfer(
						*token,
						&exchanger,
						&pool_info.keeper,
						*proportion * native_amount,
					)
				},
			)?;
			Self::add_share(&exchanger, pid, &mut pool_info, add_value);
			Self::update_reward(&exchanger, pid)?;

			Self::deposit_event(Event::Deposited { who: exchanger, pid, add_value });
			Ok(())
		}

		/// Withdraw from the pool.
		///
		/// The state of the pool should be `Ongoing`, `Charged` or `Dead`.
		/// User's withdraw limit count should be less than the field `withdraw_limit_count`.
		/// It will remove the share from the user, but not transfer the tokens to the user
		/// immediately.
		///
		/// - `pid`: The pool id.
		/// - `remove_value`: The value to withdraw.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn withdraw(
			origin: OriginFor<T>,
			pid: PoolId,
			remove_value: Option<BalanceOf<T>>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let pool_info = PoolInfos::<T>::get(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(
				pool_info.state == PoolState::Ongoing ||
					pool_info.state == PoolState::Charged ||
					pool_info.state == PoolState::Dead,
				Error::<T>::InvalidPoolState
			);
			let share_info = SharesAndWithdrawnRewards::<T>::get(&pid, &exchanger)
				.ok_or(Error::<T>::ShareInfoNotExists)?;
			ensure!(
				share_info.withdraw_list.len() < pool_info.withdraw_limit_count.into(),
				Error::<T>::WithdrawLimitCountExceeded
			);

			Self::remove_share(&exchanger, pid, remove_value, pool_info.withdraw_limit_time)?;
			Self::update_reward(&exchanger, pid)?;

			Self::deposit_event(Event::Withdrawn { who: exchanger, pid, remove_value });
			Ok(())
		}

		/// Claim the rewards from the pool.
		///
		/// The state of the pool should be `Ongoing` or `Dead`.
		/// The user should not claim the rewards within the field `claim_limit_time`.
		/// It will claim the rewards to the user, and transfer the tokens to the user immediately.
		///
		/// - `pid`: The pool id.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::claim())]
		pub fn claim(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let pool_info = PoolInfos::<T>::get(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(
				pool_info.state == PoolState::Ongoing || pool_info.state == PoolState::Dead,
				Error::<T>::InvalidPoolState
			);

			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			let share_info = SharesAndWithdrawnRewards::<T>::get(&pid, &exchanger)
				.ok_or(Error::<T>::ShareInfoNotExists)?;
			ensure!(
				share_info.claim_last_block.saturating_add(pool_info.claim_limit_time) <=
					current_block_number,
				Error::<T>::CanNotClaim
			);

			Self::claim_rewards(&exchanger, pid)?;
			Self::process_withdraw_list(&exchanger, pid, &pool_info, true)?;

			Self::deposit_event(Event::Claimed { who: exchanger, pid });
			Ok(())
		}

		/// Withdraw the claim from the pool.
		///
		/// It will immediately transfer the withdrawable tokens to the user.
		///
		/// - `pid`: The pool id.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::withdraw_claim())]
		pub fn withdraw_claim(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let pool_info = PoolInfos::<T>::get(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			Self::process_withdraw_list(&exchanger, pid, &pool_info, false)?;

			Self::deposit_event(Event::WithdrawClaimed { who: exchanger, pid });
			Ok(())
		}

		/// Force retire the pool.
		///
		/// The state of the pool should be `Dead`.
		/// It will retire the pool and transfer the withdrawable tokens to the users.
		///
		/// - `pid`: The pool id.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::force_retire_pool())]
		pub fn force_retire_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut pool_info = PoolInfos::<T>::get(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(pool_info.state == PoolState::Dead, Error::<T>::InvalidPoolState);
			let withdraw_limit_time = BlockNumberFor::<T>::default();
			let retire_limit = RetireLimit::<T>::get();
			let mut all_retired = true;
			let share_infos = SharesAndWithdrawnRewards::<T>::iter_prefix_values(pid);
			for (retire_count, share_info) in share_infos.enumerate() {
				if retire_count.saturated_into::<u32>() >= retire_limit {
					all_retired = false;
					break;
				}
				let who = share_info.who;
				Self::remove_share(&who, pid, None, withdraw_limit_time)?;
				Self::process_withdraw_list(&who, pid, &pool_info, true)?;
			}

			if all_retired {
				if let Some(ref gid) = pool_info.gauge {
					let mut gauge_pool_info =
						GaugePoolInfos::<T>::get(gid).ok_or(Error::<T>::GaugePoolNotExist)?;
					gauge_pool_info.gauge_state = GaugeState::Unbond;
					GaugePoolInfos::<T>::insert(&gid, gauge_pool_info);
				}
				pool_info.state = PoolState::Retired;
				pool_info.gauge = None;
				PoolInfos::<T>::insert(&pid, pool_info);
				Self::deposit_event(Event::AllRetired { pid });
			} else {
				Self::deposit_event(Event::PartiallyRetired { pid });
			}
			Ok(())
		}

		/// Set the retire limit.
		///
		/// - `limit`: The retire limit.
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::set_retire_limit())]
		pub fn set_retire_limit(origin: OriginFor<T>, limit: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			RetireLimit::<T>::mutate(|old_limit| {
				*old_limit = limit;
			});

			Self::deposit_event(Event::RetireLimitSet { limit });
			Ok(())
		}

		/// Close the pool.
		///
		/// Change the state of the pool to `Dead` before retiring the pool.
		///
		/// - `pid`: The pool id.
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::close_pool())]
		pub fn close_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut pool_info = PoolInfos::<T>::get(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(pool_info.state == PoolState::Ongoing, Error::<T>::InvalidPoolState);
			pool_info.state = PoolState::Dead;
			PoolInfos::<T>::insert(&pid, pool_info);

			Self::deposit_event(Event::FarmingPoolClosed { pid });
			Ok(())
		}

		/// Reuse retired pools
		///
		/// - `pid`: The pool id.
		/// - `basic_rewards`: The basic reward of each token in the pool.
		/// - `min_deposit_to_start`: The minimum deposit to start the pool.
		/// - `after_block_to_start`: The block number to start the pool.
		/// - `withdraw_limit_time`: The block number to limit the withdraw.
		/// - `claim_limit_time`: The block number to limit the claim.
		/// - `withdraw_limit_count`: The count to limit the withdraw.
		/// - `gauge_init`: The initial gauge pool info.
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::reset_pool())]
		pub fn reset_pool(
			origin: OriginFor<T>,
			pid: PoolId,
			basic_rewards: Option<Vec<(CurrencyIdOf<T>, BalanceOf<T>)>>,
			min_deposit_to_start: Option<BalanceOf<T>>,
			after_block_to_start: Option<BlockNumberFor<T>>,
			withdraw_limit_time: Option<BlockNumberFor<T>>,
			claim_limit_time: Option<BlockNumberFor<T>>,
			withdraw_limit_count: Option<u8>,
			gauge_init: Option<(BlockNumberFor<T>, Vec<(CurrencyIdOf<T>, BalanceOf<T>)>)>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut pool_info = PoolInfos::<T>::get(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(pool_info.state == PoolState::Retired, Error::<T>::InvalidPoolState);
			if let Some(basic_rewards) = basic_rewards {
				let basic_rewards_map: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> =
					basic_rewards.into_iter().collect();
				pool_info.basic_rewards = basic_rewards_map;
			};
			if let Some(min_deposit_to_start) = min_deposit_to_start {
				pool_info.min_deposit_to_start = min_deposit_to_start;
			};
			if let Some(after_block_to_start) = after_block_to_start {
				pool_info.after_block_to_start = after_block_to_start;
			};
			if let Some(withdraw_limit_time) = withdraw_limit_time {
				pool_info.withdraw_limit_time = withdraw_limit_time;
			};
			if let Some(claim_limit_time) = claim_limit_time {
				pool_info.claim_limit_time = claim_limit_time;
			};
			if let Some(withdraw_limit_count) = withdraw_limit_count {
				pool_info.withdraw_limit_count = withdraw_limit_count;
			};
			if let Some((max_block, gauge_basic_rewards)) = gauge_init {
				let gauge_basic_rewards_map: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> =
					gauge_basic_rewards.into_iter().collect();

				Self::create_gauge_pool(pid, &mut pool_info, gauge_basic_rewards_map, max_block)?;
			};
			pool_info.total_shares = Default::default();
			pool_info.rewards = BTreeMap::new();
			pool_info.state = PoolState::UnCharged;
			pool_info.block_startup = None;
			PoolInfos::<T>::insert(pid, &pool_info);

			Self::deposit_event(Event::FarmingPoolReset { pid });
			Ok(())
		}

		/// Kill the pool after retired.
		///
		/// - `pid`: The pool id.
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::kill_pool())]
		pub fn kill_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let pool_info = PoolInfos::<T>::get(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(
				pool_info.state == PoolState::Retired || pool_info.state == PoolState::UnCharged,
				Error::<T>::InvalidPoolState
			);
			let res = SharesAndWithdrawnRewards::<T>::clear_prefix(pid, u32::max_value(), None);
			ensure!(res.maybe_cursor.is_none(), Error::<T>::PoolNotCleared);
			PoolInfos::<T>::remove(pid);

			Self::deposit_event(Event::FarmingPoolKilled { pid });
			Ok(())
		}

		/// Edit the pool at the state of `Retired`, `Ongoing`, `Charged` or `UnCharged`.
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::edit_pool())]
		pub fn edit_pool(
			origin: OriginFor<T>,
			pid: PoolId,
			basic_rewards: Option<Vec<(CurrencyIdOf<T>, BalanceOf<T>)>>,
			withdraw_limit_time: Option<BlockNumberFor<T>>,
			claim_limit_time: Option<BlockNumberFor<T>>,
			gauge_basic_rewards: Option<Vec<(CurrencyIdOf<T>, BalanceOf<T>)>>,
			withdraw_limit_count: Option<u8>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut pool_info = PoolInfos::<T>::get(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(
				pool_info.state == PoolState::Retired ||
					pool_info.state == PoolState::Ongoing ||
					pool_info.state == PoolState::Charged ||
					pool_info.state == PoolState::UnCharged,
				Error::<T>::InvalidPoolState
			);
			if let Some(basic_rewards) = basic_rewards {
				let basic_rewards_map: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> =
					basic_rewards.into_iter().collect();
				pool_info.basic_rewards = basic_rewards_map;
			};
			if let Some(withdraw_limit_time) = withdraw_limit_time {
				pool_info.withdraw_limit_time = withdraw_limit_time;
			};
			if let Some(claim_limit_time) = claim_limit_time {
				pool_info.claim_limit_time = claim_limit_time;
			};
			if let Some(withdraw_limit_count) = withdraw_limit_count {
				pool_info.withdraw_limit_count = withdraw_limit_count;
			};
			if let Some(gauge_basic_rewards) = gauge_basic_rewards {
				let gauge_basic_rewards_map: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> =
					gauge_basic_rewards.into_iter().collect();
				GaugePoolInfos::<T>::mutate(
					pool_info.gauge.ok_or(Error::<T>::GaugePoolNotExist)?,
					|gauge_pool_info_old| {
						if let Some(mut gauge_pool_info) = gauge_pool_info_old.take() {
							gauge_pool_info.gauge_basic_rewards = gauge_basic_rewards_map;
							*gauge_pool_info_old = Some(gauge_pool_info);
						}
					},
				);
			};
			PoolInfos::<T>::insert(pid, &pool_info);

			Self::deposit_event(Event::FarmingPoolEdited { pid });
			Ok(())
		}

		/// Withdraw the rewards from the gauge pool.
		///
		/// - `gid`: The gauge pool id.
		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::gauge_withdraw())]
		pub fn gauge_withdraw(origin: OriginFor<T>, gid: PoolId) -> DispatchResult {
			// Check origin
			let who = ensure_signed(origin)?;

			let pool_info = PoolInfos::<T>::get(gid).ok_or(Error::<T>::PoolDoesNotExist)?;
			let share_info = SharesAndWithdrawnRewards::<T>::get(gid, &who)
				.ok_or(Error::<T>::ShareInfoNotExists)?;
			T::BbBNC::get_rewards(gid, &who, Some((share_info.share, pool_info.total_shares)))?;

			Self::deposit_event(Event::GaugeWithdrawn { who, gid });
			Ok(())
		}

		/// Force claim the rewards from the gauge pool.
		///
		/// Control origin can force claim the rewards from the gauge pool to the users.
		///
		/// - `gid`: The gauge pool id.
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::force_gauge_claim())]
		pub fn force_gauge_claim(origin: OriginFor<T>, gid: PoolId) -> DispatchResult {
			// Check origin
			T::ControlOrigin::ensure_origin(origin)?;

			let gauge_infos = GaugeInfos::<T>::iter_prefix_values(&gid);
			let retire_limit = RetireLimit::<T>::get();
			let mut all_retired = true;
			for (retire_count, gauge_info) in gauge_infos.enumerate() {
				if retire_count.saturated_into::<u32>() >= retire_limit {
					all_retired = false;
					break;
				}
				let pool_info = PoolInfos::<T>::get(gid).ok_or(Error::<T>::PoolDoesNotExist)?;
				let share_info = SharesAndWithdrawnRewards::<T>::get(gid, &gauge_info.who)
					.ok_or(Error::<T>::ShareInfoNotExists)?;
				T::BbBNC::get_rewards(
					gid,
					&gauge_info.who,
					Some((share_info.share, pool_info.total_shares)),
				)?;
			}

			if all_retired {
				Self::deposit_event(Event::AllForceGaugeClaimed { gid });
			} else {
				Self::deposit_event(Event::PartiallyForceGaugeClaimed { gid });
			}
			Ok(())
		}

		/// Add whitelist and take effect immediately
		///
		/// - `whitelist`: The whitelist to add
		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::add_boost_pool_whitelist())]
		pub fn add_boost_pool_whitelist(
			origin: OriginFor<T>,
			whitelist: Vec<PoolId>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			whitelist.iter().for_each(|pid| {
				BoostWhitelist::<T>::insert(pid, ());
				BoostVotingPools::<T>::mutate_exists(pid, |maybe_total_votes| {
					if maybe_total_votes.is_none() {
						*maybe_total_votes = Some(Zero::zero());
					}
				})
			});
			Ok(())
		}

		/// Whitelist for next round in effect
		///
		/// - `whitelist`: The whitelist for the next round
		#[pallet::call_index(15)]
		#[pallet::weight(T::WeightInfo::set_next_round_whitelist())]
		pub fn set_next_round_whitelist(
			origin: OriginFor<T>,
			whitelist: Vec<PoolId>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			let res = BoostNextRoundWhitelist::<T>::clear(u32::max_value(), None);
			ensure!(res.maybe_cursor.is_none(), Error::<T>::PoolNotCleared);
			whitelist.iter().for_each(|pid| {
				BoostNextRoundWhitelist::<T>::insert(pid, ());
			});
			Ok(())
		}

		/// Vote for the pool
		///
		/// - `vote_list`: The vote list for the pool
		#[pallet::call_index(16)]
		#[pallet::weight(T::WeightInfo::claim())]
		pub fn vote(origin: OriginFor<T>, vote_list: Vec<(PoolId, Percent)>) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::vote_inner(&exchanger, vote_list.clone())?;
			Self::deposit_event(Event::Voted { who: exchanger, vote_list });
			Ok(())
		}

		/// Start the boost round
		///
		/// - `round_length`: The length of the round
		#[pallet::call_index(17)]
		#[pallet::weight(T::WeightInfo::claim())]
		pub fn start_boost_round(
			origin: OriginFor<T>,
			round_length: BlockNumberFor<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::start_boost_round_inner(round_length)?;
			Ok(())
		}

		/// Force end of boost round
		#[pallet::call_index(18)]
		#[pallet::weight(T::WeightInfo::claim())]
		pub fn end_boost_round(origin: OriginFor<T>) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::end_boost_round_inner();
			Ok(())
		}

		/// Charge the boost rewards to the FarmingBoost account
		///
		/// - `rewards`: The rewards to charge
		#[pallet::call_index(19)]
		#[pallet::weight(T::WeightInfo::claim())]
		pub fn charge_boost(
			origin: OriginFor<T>,
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			rewards.iter().try_for_each(|(currency, reward)| -> DispatchResult {
				T::MultiCurrency::transfer(
					*currency,
					&exchanger,
					&T::FarmingBoost::get().into_account_truncating(),
					*reward,
				)
			})?;
			Self::deposit_event(Event::BoostCharged { who: exchanger, rewards });
			Ok(())
		}
	}
}

impl<T: Config> FarmingInfo<BalanceOf<T>, CurrencyIdOf<T>> for Pallet<T> {
	fn get_token_shares(pool_id: PoolId, currency_id: CurrencyIdOf<T>) -> BalanceOf<T> {
		if let Some(pool_info) = PoolInfos::<T>::get(&pool_id) {
			if let Some(token_proportion_value) = pool_info.tokens_proportion.get(&currency_id) {
				let native_amount =
					pool_info.basic_token.1.saturating_reciprocal_mul(pool_info.total_shares);
				return *token_proportion_value * native_amount;
			}
		}
		Zero::zero()
	}
}

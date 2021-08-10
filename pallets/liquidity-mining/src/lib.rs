// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::{SaturatedConversion, Saturating, Zero},
	sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	traits::EnsureOrigin,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, LeasePeriod, ParaId};
use orml_traits::{MultiCurrency, MultiLockableCurrency, MultiReservableCurrency};
pub use pallet::*;
use substrate_fixed::{traits::FromFixed, types::U64F64};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct PoolInfo<T: Config> {
	/// The creator of the liquidity-pool
	creator: AccountIdOf<T>,
	/// The trading-pair supported by the liquidity-pool
	trading_pair: (CurrencyId, CurrencyId),
	/// The length of time the liquidity-pool releases rewards
	duration: BlockNumberFor<T>,
	/// The liquidity-pool type
	r#type: PoolType,

	/// The First Condition
	///
	/// When starts the liquidity-pool, the amount deposited in the liquidity-pool
	/// should be greater than the value.
	min_deposited_amount_to_start: BalanceOf<T>,
	/// The Second Condition
	///
	/// When starts the liquidity-pool, the current block should be greater than the value.
	after_block_to_start: BlockNumberFor<T>,

	/// The total amount deposited in the liquidity-pool
	deposited: BalanceOf<T>,

	/// The reward infos about the liquidity-pool
	rewards: BTreeMap<CurrencyId, RewardData<T>>,
	/// The liquidity-pool state
	state: PoolState<T>,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Debug)]
pub enum PoolType {
	Mining,
	Farming,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq)]
pub enum PoolState<T: Config> {
	Init,
	Activated,
	Ongoing(BlockNumberFor<T>),
	Retired,
	Dead,
}

impl<T: Config> PoolState<T> {
	pub fn is_ongoing(&self) -> bool {
		match self {
			Self::Ongoing(..) => true,
			_ => false,
		}
	}

	pub fn block_started(&self) -> BlockNumberFor<T> {
		match self {
			Self::Ongoing(block_number) => *block_number,
			_ => Zero::zero(),
		}
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct DepositData<T: Config> {
	/// The id of liquidity-pool
	pid: PoolId,

	/// The amount of trading-pair deposited in the liquidity-pool
	deposited: BalanceOf<T>,
	/// Important data used to calculate rewards,
	/// updated when the `DepositData`'s owner redeems or claims from the liquidity-pool.
	///
	/// - Arg0: The average gain in pico by 1 pico deposited from the startup of the liquidity-pool
	/// - Arg1: The update block number
	gain_avgs: BTreeMap<CurrencyId, (U64F64, BlockNumberFor<T>)>,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct RewardData<T: Config> {
	/// The total amount of token to reward
	total: BalanceOf<T>,
	/// The amount of token to reward per block
	per_block: BalanceOf<T>,

	/// The amount of token was already rewarded
	claimed: BalanceOf<T>,
	/// Important data used to calculate rewards,
	/// updated when anyone deposits to / redeems from / claims from the liquidity-pool.
	///
	/// - Arg0: The average gain in pico by 1 pico deposited from the startup of the liquidity-pool
	/// - Arg1: The update block number
	gain_avg: (U64F64, BlockNumberFor<T>),
}

impl<T: Config> RewardData<T> {
	fn new(
		total: BalanceOf<T>,
		duration: BlockNumberFor<T>,
		min_per_block: BalanceOf<T>,
	) -> Result<Self, Error<T>> {
		let total: u128 = total.saturated_into();
		let (per_block, total) = {
			let duration: u128 = duration.saturated_into();

			let per_block = u128::from_fixed((U64F64::from_num(total) / duration).floor());
			let total = per_block * duration;

			(BalanceOf::<T>::saturated_from(per_block), BalanceOf::<T>::saturated_from(total))
		};

		ensure!(per_block > min_per_block, Error::<T>::InvalidRewardPerBlock);

		Ok(RewardData {
			total,
			per_block,

			claimed: Zero::zero(),

			gain_avg: (U64F64::from_num(0), Zero::zero()),
		})
	}
}

impl<T: Config> core::fmt::Debug for RewardData<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_tuple("")
			.field(&self.total)
			.field(&self.per_block)
			.field(&self.claimed)
			.field(&self.gain_avg)
			.finish()
	}
}

#[allow(type_alias_bounds)]
type AccountIdOf<T: Config> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

type PoolId = u128;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Origin for anyone able to create/activate/kill the liquidity-pool.
		type ControlOrigin: EnsureOrigin<Self::Origin>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The value used to construct vsbond when creating a farming-liquidity-pool
		#[pallet::constant]
		type RelayChainToken: Get<CurrencyId>;

		/// The amount deposited into a liquidity-pool should be greater than the value
		#[pallet::constant]
		type MinimumDeposit: Get<BalanceOf<Self>>;

		/// The amount deposited into a liquidity-pool should be less than the value
		#[pallet::constant]
		type MaximumDeposit: Get<BalanceOf<Self>>;

		/// The amount of token to reward per block should be greater than the value
		#[pallet::constant]
		type MinimumRewardPerBlock: Get<BalanceOf<Self>>;

		/// The duration of a liquidity-pool should be greater than the value
		#[pallet::constant]
		type MinimumDuration: Get<BlockNumberFor<Self>>;

		/// The number of liquidity-pool activated should be less than the value
		#[pallet::constant]
		type MaximumActivated: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidTradingPair,
		InvalidDuration,
		InvalidRewardPerBlock,
		InvalidCondition,
		InvalidPoolId,
		InvalidPoolState,
		InvalidPoolOwner,
		DuplicateReward,
		NotReachDurationEnd,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The liquidity-pool has been created
		///
		/// [pool_id, pool_type, trading_pair, creator]
		PoolCreated(PoolId, PoolType, (CurrencyId, CurrencyId), AccountIdOf<T>),
		/// The liquidity-pool has been activated
		///
		/// [pool_id, pool_type, trading_pair]
		PoolActivated(PoolId, PoolType, (CurrencyId, CurrencyId)),
		/// The liquidity-pool has been started up
		///
		/// [pool_id, pool_type, trading_pair]
		PoolStarted(PoolId, PoolType, (CurrencyId, CurrencyId)),
		/// The liquidity-pool has been killed
		///
		/// [pool_id, pool_type, trading_pair]
		PoolKilled(PoolId, PoolType, (CurrencyId, CurrencyId)),
		/// The liquidity-pool has been retired
		///
		/// [pool_id, pool_type, trading_pair]
		PoolRetired(PoolId, PoolType, (CurrencyId, CurrencyId)),
		/// User has deposited some trading-pair to a liquidity-pool
		///
		/// [pool_id, pool_type, trading_pair, amount_deposited, user]
		UserDeposited(PoolId, PoolType, (CurrencyId, CurrencyId), BalanceOf<T>, AccountIdOf<T>),
		/// User has been redeemed some trading-pair from a liquidity-pool
		///
		/// [pool_id, pool_type, trading_pair, amount_redeemed, user]
		UserRedeemed(PoolId, PoolType, (CurrencyId, CurrencyId), BalanceOf<T>, AccountIdOf<T>),
		/// User has been claimed the rewards from a liquidity-pool
		///
		/// [pool_id, pool_type, trading_pair, token_rewarded, amount_claimed, user]
		UserClaimed(
			PoolId,
			PoolType,
			(CurrencyId, CurrencyId),
			CurrencyId,
			BalanceOf<T>,
			AccountIdOf<T>,
		),
	}

	#[pallet::storage]
	#[pallet::getter(fn pool_id)]
	pub(crate) type NextOrderId<T: Config> = StorageValue<_, PoolId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn activated_pids)]
	pub(crate) type ActivatedPoolIds<T: Config> = StorageValue<_, BTreeSet<PoolId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pool)]
	pub(crate) type TotalPoolInfos<T: Config> = StorageMap<_, Twox64Concat, PoolId, PoolInfo<T>>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_mining_pool(
			origin: OriginFor<T>,
			trading_pair: (CurrencyId, CurrencyId),
			main_reward: (CurrencyId, BalanceOf<T>),
			option_rewards: Vec<(CurrencyId, BalanceOf<T>)>,
			#[pallet::compact] duration: BlockNumberFor<T>,
			#[pallet::compact] min_deposited_amount_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			Self::create_pool(
				origin,
				trading_pair,
				main_reward,
				option_rewards,
				PoolType::Mining,
				duration,
				min_deposited_amount_to_start,
				after_block_to_start,
			)?;

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn create_farming_pool(
			origin: OriginFor<T>,
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
			main_reward: (CurrencyId, BalanceOf<T>),
			option_rewards: Vec<(CurrencyId, BalanceOf<T>)>,
			#[pallet::compact] duration: BlockNumberFor<T>,
			#[pallet::compact] min_deposited_amount_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			#[allow(non_snake_case)]
			let trading_pair = Self::vsAssets(index, first_slot, last_slot);

			Self::create_pool(
				origin,
				trading_pair,
				main_reward,
				option_rewards,
				PoolType::Farming,
				duration,
				min_deposited_amount_to_start,
				after_block_to_start,
			)?;

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn activate_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?;

			ensure!(pool.state == PoolState::Init, Error::<T>::InvalidPoolState);

			ActivatedPoolIds::<T>::mutate(|pids| pids.insert(pid));

			let pool_activated = PoolInfo { state: PoolState::Activated, ..pool.clone() };
			TotalPoolInfos::<T>::insert(pid, pool_activated);

			Self::deposit_event(Event::PoolActivated(pid, pool.r#type, pool.trading_pair));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn kill_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let signed = ensure_signed(origin)?;

			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?;

			ensure!(signed == pool.creator, Error::<T>::InvalidPoolOwner);

			ensure!(pool.state == PoolState::Init, Error::<T>::InvalidPoolState);

			let pool_killed = PoolInfo { state: PoolState::Dead, ..pool.clone() };
			TotalPoolInfos::<T>::insert(pid, pool_killed);

			Self::deposit_event(Event::PoolKilled(pid, pool.r#type, pool.trading_pair));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn retire_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let _ = ensure_signed(origin)?;

			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?;

			ensure!(pool.state.is_ongoing(), Error::<T>::InvalidPoolState);

			let block_past = <frame_system::Pallet<T>>::block_number() - pool.state.block_started();
			ensure!(block_past >= pool.duration, Error::<T>::NotReachDurationEnd);

			let pool_retired = PoolInfo { state: PoolState::Retired, ..pool.clone() };
			TotalPoolInfos::<T>::insert(pid, pool_retired);

			Self::deposit_event(Event::PoolRetired(pid, pool.r#type, pool.trading_pair));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn deposit(origin: OriginFor<T>, value: BalanceOf<T>) -> DispatchResultWithPostInfo {
			todo!()
		}

		#[pallet::weight(1_000)]
		pub fn redeem(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			todo!()
		}

		#[pallet::weight(1_000)]
		pub fn claim(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			todo!()
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn create_pool(
			origin: OriginFor<T>,
			trading_pair: (CurrencyId, CurrencyId),
			main_reward: (CurrencyId, BalanceOf<T>),
			option_rewards: Vec<(CurrencyId, BalanceOf<T>)>,
			r#type: PoolType,
			duration: BlockNumberFor<T>,
			min_deposited_amount_to_start: BalanceOf<T>,
			after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResult {
			let creator = ensure_signed(origin)?;

			// Check the trading-pair
			ensure!(trading_pair.0 != trading_pair.1, Error::<T>::InvalidTradingPair);

			// Check the duration
			ensure!(duration >= T::MinimumDuration::get(), Error::<T>::InvalidDuration);

			// Check the condition
			ensure!(
				min_deposited_amount_to_start >= T::MinimumDeposit::get(),
				Error::<T>::InvalidCondition
			);
			ensure!(
				min_deposited_amount_to_start <= T::MaximumDeposit::get(),
				Error::<T>::InvalidCondition
			);

			// Check & Construct the rewards
			let raw_rewards: Vec<(CurrencyId, BalanceOf<T>)> =
				option_rewards.into_iter().chain(Some(main_reward).into_iter()).collect();
			let mut rewards: BTreeMap<CurrencyId, RewardData<T>> = BTreeMap::new();
			for (token, total) in raw_rewards.into_iter() {
				ensure!(!rewards.contains_key(&token), Error::<T>::DuplicateReward);

				let reward = RewardData::new(total, duration, T::MinimumRewardPerBlock::get())?;

				// Reserve the reward
				T::MultiCurrency::reserve(token, &creator, reward.total)?;

				rewards.insert(token, reward);
			}

			// Construct the PoolInfo
			let pool_id = Self::next_pool_id();
			let mining_pool = PoolInfo {
				creator: creator.clone(),
				trading_pair,
				duration,
				r#type,

				min_deposited_amount_to_start,
				after_block_to_start,

				deposited: Zero::zero(),

				rewards,
				state: PoolState::Init,
			};

			TotalPoolInfos::<T>::insert(pool_id, mining_pool);

			Self::deposit_event(Event::PoolCreated(pool_id, r#type, trading_pair, creator));

			Ok(().into())
		}

		pub(crate) fn next_pool_id() -> PoolId {
			let next_pool_id = Self::pool_id();
			NextOrderId::<T>::mutate(|current| *current += 1);
			next_pool_id
		}

		#[allow(non_snake_case)]
		pub(crate) fn vsAssets(
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
		) -> (CurrencyId, CurrencyId) {
			let token_symbol = *T::RelayChainToken::get();

			let vsToken = CurrencyId::VSToken(token_symbol);
			let vsBond = CurrencyId::VSBond(token_symbol, index, first_slot, last_slot);

			(vsToken, vsBond)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			// Check whether pool-activated is meet the startup condition
			for pid in Self::activated_pids() {
				if let Some(pool) = Self::pool(pid) {
					if n >= pool.after_block_to_start &&
						pool.deposited >= pool.min_deposited_amount_to_start
					{
						let block_started = n + BlockNumberFor::<T>::from(1 as u32);
						let pool_started =
							PoolInfo { state: PoolState::Ongoing(block_started), ..pool.clone() };

						ActivatedPoolIds::<T>::mutate(|pids| pids.remove(&pid));
						TotalPoolInfos::<T>::insert(pid, pool_started);
					}
				}
			}
		}

		fn on_initialize(_n: BlockNumberFor<T>) -> frame_support::weights::Weight {
			// TODO estimate weight
			Zero::zero()
		}
	}
}

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
	Idle,
	Activated,
	Ongoing(BlockNumberFor<T>),
	Retired,
	Dead,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct DepositData<T: Config> {
	/// The id of liquidity-pool
	id: PoolId,

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
		DuplicateReward,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The mining-liquidity-pool has been created
		///
		/// [pool_id, creator]
		MiningPoolCreated(PoolId, AccountIdOf<T>),
		/// The farming-liquidity-pool has been created
		///
		/// [pool_id, creator]
		FarmingPoolCreated(PoolId, AccountIdOf<T>),
	}

	#[pallet::storage]
	#[pallet::getter(fn pool_id)]
	pub(crate) type NextOrderId<T: Config> = StorageValue<_, PoolId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn idle_pids)]
	pub(crate) type IdlePoolIds<T: Config> = StorageValue<_, BTreeSet<PoolId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn activated_pids)]
	pub(crate) type ActivatedPoolIds<T: Config> = StorageValue<_, BTreeSet<PoolId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ongoing_pids)]
	pub(crate) type OngoingPoolIds<T: Config> = StorageValue<_, BTreeSet<PoolId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn dead_pids)]
	pub(crate) type DeadPoolIds<T: Config> = StorageValue<_, BTreeSet<PoolId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn retired_pids)]
	pub(crate) type RetiredPoolIds<T: Config> = StorageValue<_, BTreeSet<PoolId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pool_info)]
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
				r#type: PoolType::Mining,

				min_deposited_amount_to_start,
				after_block_to_start,

				deposited: Zero::zero(),

				rewards,
				state: PoolState::Idle,
			};

			TotalPoolInfos::<T>::insert(pool_id, mining_pool);
			IdlePoolIds::<T>::mutate(|pids| pids.insert(pool_id));

			Self::deposit_event(Event::MiningPoolCreated(pool_id, creator));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn create_farming_pool(
			origin: OriginFor<T>,
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
			main_reward: (CurrencyId, BalanceOf<T>),
			option_reward: [(CurrencyId, BalanceOf<T>); 4],
			#[pallet::compact] duration: BlockNumberFor<T>,
			#[pallet::compact] min_staked_amount_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			let creator = ensure_signed(origin)?;

			todo!()
		}

		#[pallet::weight(1_000)]
		pub fn activate_pool(origin: OriginFor<T>, id: PoolId) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			// TODO: Query the `PoolInfo` by id

			// TODO: Check the state of `PoolInfo`

			// TODO: Check the balance of rewards

			// TODO: Change the state of `PoolInfo`

			todo!()
		}

		#[pallet::weight(1_000)]
		pub fn kill_pool(origin: OriginFor<T>, id: PoolId) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			// TODO: Query the `PoolInfo` by id

			// TODO: Check the state of `PoolInfo`

			// TODO: Change the state of `PoolInfo`

			todo!()
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
		pub(crate) fn next_pool_id() -> PoolId {
			let next_pool_id = Self::pool_id();
			NextOrderId::<T>::mutate(|current| *current += 1);
			next_pool_id
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			// TODO: Check whether pool-activated is meet the startup condition

			// TODO: Check whether pool-ongoing reach the release reward time

			// TODO: Check whether pool-ongoing reach the end time
		}

		fn on_initialize(_n: BlockNumberFor<T>) -> frame_support::weights::Weight {
			// TODO estimate weight
			Zero::zero()
		}
	}
}

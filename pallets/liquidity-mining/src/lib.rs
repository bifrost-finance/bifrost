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
	sp_std::{
		cmp::{max, min},
		collections::{btree_map::BTreeMap, btree_set::BTreeSet},
	},
	traits::{BalanceStatus, EnsureOrigin},
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, LeasePeriod, ParaId};
use orml_traits::{LockIdentifier, MultiCurrency, MultiLockableCurrency, MultiReservableCurrency};
pub use pallet::*;
use substrate_fixed::{traits::FromFixed, types::U64F64};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

const DEPOSIT_ID: LockIdentifier = *b"deposit ";

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
	state: PoolState,
	/// The block number when the liquidity-pool startup
	block_startup: Option<BlockNumberFor<T>>,
}

impl<T: Config> PoolInfo<T> {
	/// Trying to update the rewards
	pub(crate) fn try_update(mut self) -> Self {
		// When pool in `PoolState::Ongoing` or `PoolState::Retired`
		if let Some(block_startup) = self.block_startup {
			let block_end = self.duration + block_startup;
			let n = min(frame_system::Pallet::<T>::block_number(), block_end);

			for (_, reward) in self.rewards.iter_mut() {
				reward.update(self.deposited, block_startup, n);
			}
		}

		self
	}

	/// Trying to change the state from `PoolState::Activated` to `PoolState::Ongoing`
	pub(crate) fn try_startup(mut self, pid: PoolId) -> Self {
		if self.state == PoolState::Activated {
			let n = frame_system::Pallet::<T>::block_number();

			if n >= self.after_block_to_start &&
				self.deposited >= self.min_deposited_amount_to_start
			{
				self.block_startup = Some(n);
				self.state = PoolState::Ongoing;

				Pallet::<T>::deposit_event(Event::PoolStarted(pid, self.r#type, self.trading_pair));
			}
		}

		self
	}

	/// Trying to change the state from `PoolState::Ongoing` to `PoolState::Retired`
	///
	/// __NOTE__: Only called in the `Hook`
	pub(crate) fn try_retire(mut self, pid: PoolId) -> Self {
		if self.state == PoolState::Ongoing {
			let n = frame_system::Pallet::<T>::block_number();

			if let Some(block_startup) = self.block_startup {
				if n >= block_startup + self.duration {
					self.state = PoolState::Retired;

					Pallet::<T>::deposit_event(Event::PoolRetired(
						pid,
						self.r#type,
						self.trading_pair,
					));
				}
			}
		}

		self
	}
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Debug)]
pub enum PoolType {
	Mining,
	Farming,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq)]
pub enum PoolState {
	Init,
	Activated,
	Ongoing,
	Retired,
	Dead,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct DepositData<T: Config> {
	/// The amount of trading-pair deposited in the liquidity-pool
	deposited: BalanceOf<T>,
	/// Important data used to calculate rewards,
	/// updated when the `DepositData`'s owner redeems or claims from the liquidity-pool.
	///
	/// - Arg0: The average gain in pico by 1 pico deposited from the startup of the liquidity-pool
	/// - Arg1: The update block number
	gain_avgs: BTreeMap<CurrencyId, (U64F64, BlockNumberFor<T>)>,
}

impl<T: Config> Default for DepositData<T> {
	fn default() -> Self {
		Self { deposited: Zero::zero(), gain_avgs: BTreeMap::new() }
	}
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
	) -> Result<Self, DispatchError> {
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

	pub(crate) fn per_block_per_deposited(&self, deposited: BalanceOf<T>) -> U64F64 {
		let per_block: u128 = self.per_block.saturated_into();
		let deposited: u128 = deposited.saturated_into();

		U64F64::from_num(per_block) / deposited
	}

	/// Trying to update the gain_avg
	pub(crate) fn update(
		&mut self,
		deposited: BalanceOf<T>,
		block_startup: BlockNumberFor<T>,
		n: BlockNumberFor<T>,
	) {
		let pbpd = self.per_block_per_deposited(deposited);

		let b_prev = max(self.gain_avg.1, block_startup);
		let b_past: u128 = (n - b_prev).saturated_into();

		let gain_avg_new = self.gain_avg.0 + pbpd * b_past;

		self.gain_avg = (gain_avg_new, n);
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

		/// The amount deposited into a liquidity-pool should be less than the value
		#[pallet::constant]
		type MaximumDepositedInPool: Get<BalanceOf<Self>>;

		/// The amount deposited by a user to a liquidity-pool should be greater than the value
		#[pallet::constant]
		type MinimumDeposit: Get<BalanceOf<Self>>;

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
		InvalidDepositLimit,
		InvalidPoolId,
		InvalidPoolState,
		InvalidPoolOwner,
		/// Find duplicate reward when creating the liquidity-pool
		DuplicateReward,
		/// When the amount deposited in a liquidity-pool exceeds the `MaximumDepositInPool`
		ExceedMaximumDeposited,
		///
		NotEnoughBalanceToLock,
		/// Not enough balance of reward to unreserve
		FailOnUnReserve,
		/// Not enough deposited by the user in the liquidity-pool to claim the rewards
		NotEnoughDepositedToClaim,
		/// Too low balance to deposit
		TooLowToDeposit,
		/// IMPOSSIBLE TO HAPPEN
		Unexpected,
		/// __TEMP__: NotImpl
		NotImpl,
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
		/// [pool_id, pool_type, trading_pair, rewards, user]
		UserClaimed(
			PoolId,
			PoolType,
			(CurrencyId, CurrencyId),
			Vec<(CurrencyId, BalanceOf<T>)>,
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

	#[pallet::storage]
	#[pallet::getter(fn user_deposit_data)]
	pub(crate) type TotalDepositData<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		PoolId,
		DepositData<T>,
		ValueQuery,
	>;

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

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			let pool_activated = PoolInfo { state: PoolState::Activated, ..pool };
			TotalPoolInfos::<T>::insert(pid, pool_activated);

			Self::deposit_event(Event::PoolActivated(pid, r#type, trading_pair));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn kill_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let signed = ensure_signed(origin)?;

			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?;

			ensure!(signed == pool.creator, Error::<T>::InvalidPoolOwner);

			ensure!(pool.state == PoolState::Init, Error::<T>::InvalidPoolState);

			for (token, reward) in pool.rewards.iter() {
				let total = reward.total;
				let remain = T::MultiCurrency::unreserve(*token, &signed, total);
				ensure!(remain == Zero::zero(), Error::<T>::FailOnUnReserve);
			}

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			let pool_killed = PoolInfo { state: PoolState::Dead, ..pool };
			TotalPoolInfos::<T>::insert(pid, pool_killed);

			Self::deposit_event(Event::PoolKilled(pid, r#type, trading_pair));

			Ok(().into())
		}

		// TODO: 当最后一个参与者redeem离开, 调用该功能
		// TODO: 删除该函数, 将功能触发放到最后一个redeem时候
		#[pallet::weight(1_000)]
		pub fn refund_remain_rewards(
			origin: OriginFor<T>,
			pid: PoolId,
		) -> DispatchResultWithPostInfo {
			todo!()
		}

		// TODO: 0. 若为Ongoing, 检查是否能Retired, 能, Retire掉Pool
		// TODO: 1. 只有当Activated或Ongoing时才能进行Deposit
		// TODO: 2. 全新的Deposit会增加池中的质押人数
		// TODO: 3. 二次质押时, 会先进行Claim结算
		// TODO: 4. 在Activated时, 单位平均收益恒为0
		// TODO: 5. 质押金额必须大于最小质押金额
		// TODO: 6. 质押后在下一个区块起息, 不一定!
		#[pallet::weight(1_000)]
		pub fn deposit(
			origin: OriginFor<T>,
			pid: PoolId,
			value: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			let mut pool =
				Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire(pid).try_update();

			ensure!(
				pool.state == PoolState::Activated || pool.state == PoolState::Ongoing,
				Error::<T>::InvalidPoolState
			);

			ensure!(value >= T::MinimumDeposit::get(), Error::<T>::TooLowToDeposit);

			let mut deposit_data = Self::user_deposit_data(&user, &pid);

			// To lock the deposited
			if pool.r#type == PoolType::Mining {
				return Err(Error::<T>::NotImpl.into());
			} else {
				let (token_a, token_b) = pool.trading_pair;

				ensure!(
					T::MultiCurrency::free_balance(token_a, &user) >= value,
					Error::<T>::NotEnoughBalanceToLock
				);
				ensure!(
					T::MultiCurrency::free_balance(token_b, &user) >= value,
					Error::<T>::NotEnoughBalanceToLock
				);

				T::MultiCurrency::extend_lock(DEPOSIT_ID, token_a, &user, value)?;
				T::MultiCurrency::extend_lock(DEPOSIT_ID, token_b, &user, value)?;
			}

			// TODO: 若状态为Activated
			// 	- 更新PoolInfo.deposited
			// 	- 更新DepositData.deposited

			// TODO: 若状态为Ongoing
			// 	- 根据DepositData和PoolInfo中的平均单位收益差值, 结算当前奖励
			// 	- 更新Rewards信息与DepositData中的Rewards快照
			// 	- 更新PoolInfo.deposited
			// 	- 更新DepositData.deposited

			deposit_data.deposited = deposit_data.deposited.saturating_add(value);
			pool.deposited = pool.deposited.saturating_add(value);
			ensure!(
				pool.deposited <= T::MaximumDepositedInPool::get(),
				Error::<T>::ExceedMaximumDeposited
			);

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			TotalPoolInfos::<T>::insert(pid, pool);
			TotalDepositData::<T>::insert(user.clone(), pid, deposit_data);

			Self::deposit_event(Event::UserDeposited(pid, r#type, trading_pair, value, user));

			Ok(().into())
		}

		// TODO: 0. 若为Ongoing, 检查是否能Retired, 能, Retire掉Pool
		// TODO: 1. 只能在Ongoing或Retired的情况下进行Redeem
		// TODO: 2. Redeem之前会先进性Claim结算
		// TODO: 3. 当处于Ongoing状态时, Pool中质押的Token不能完全抽干, 为了防止空放攻击
		#[pallet::weight(1_000)]
		pub fn redeem(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			todo!()
		}

		/// User claims the reward from a liquidity-pool.
		///
		/// The conditions to claim:
		/// 	- User should have enough deposited in the liquidity-pool;
		/// 	- The liquidity-pool should in special states: `PoolState::Ongoing`,
		///    `PoolState::Retired`;
		#[pallet::weight(1_000)]
		pub fn claim(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			let mut pool: PoolInfo<T> =
				Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire(pid).try_update();

			ensure!(
				pool.state == PoolState::Ongoing || pool.state == PoolState::Retired,
				Error::<T>::InvalidPoolState
			);

			let mut deposit_data: DepositData<T> = Self::user_deposit_data(&user, &pid);

			ensure!(
				deposit_data.deposited >= T::MinimumDeposit::get(),
				Error::<T>::NotEnoughDepositedToClaim
			);

			Self::accounting_rewards(&mut pool, &mut deposit_data, pid, user.clone())?;

			TotalPoolInfos::<T>::insert(pid, pool);
			TotalDepositData::<T>::insert(user, pid, deposit_data);

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Account & Transfer the rewards to user
		pub(crate) fn accounting_rewards(
			pool: &mut PoolInfo<T>,
			deposit_data: &mut DepositData<T>,
			pid: PoolId,
			user: AccountIdOf<T>,
		) -> Result<(), DispatchError> {
			let mut to_rewards = Vec::<(CurrencyId, BalanceOf<T>)>::new();

			let bs = pool.block_startup.ok_or(Error::<T>::Unexpected)?;
			for (rtoken, reward) in pool.rewards.iter_mut() {
				let (vn, un) = reward.gain_avg;
				let (vo, uo) = *deposit_data.gain_avgs.get(rtoken).ok_or(Error::<T>::Unexpected)?;
				let uo = max(uo, bs);

				let block_past: u128 = (un - uo).saturated_into();
				let amount = BalanceOf::<T>::saturated_from(u128::from_fixed(
					((vn - vo) * block_past).floor(),
				));

				// Update the claimed of the reward
				reward.claimed = reward.claimed.saturating_add(amount);
				// Sync the gain_avg between `DepositData` and `RewardData`
				deposit_data.gain_avgs.insert(*rtoken, (vn, un));

				to_rewards.push((*rtoken, amount));
			}

			for (rtoken, amount) in to_rewards.iter() {
				T::MultiCurrency::repatriate_reserved(
					*rtoken,
					&pool.creator,
					&user,
					*amount,
					BalanceStatus::Free,
				)?;
			}

			Self::deposit_event(Event::UserClaimed(
				pid,
				pool.r#type,
				pool.trading_pair,
				to_rewards,
				user,
			));

			Ok(().into())
		}

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
				Error::<T>::InvalidDepositLimit
			);
			ensure!(
				min_deposited_amount_to_start <= T::MaximumDepositedInPool::get(),
				Error::<T>::InvalidDepositLimit
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
				block_startup: None,
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
				if let Some(mut pool) = Self::pool(pid) {
					pool = pool.try_startup(pid);

					if pool.state == PoolState::Ongoing {
						ActivatedPoolIds::<T>::mutate(|pids| pids.remove(&pid));
						TotalPoolInfos::<T>::insert(pid, pool);
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

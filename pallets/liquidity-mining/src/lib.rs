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
	sp_runtime::{
		traits::{SaturatedConversion, Saturating, Zero},
		FixedPointNumber, FixedU128,
	},
	sp_std::{
		cmp::{max, min},
		collections::{btree_map::BTreeMap, btree_set::BTreeSet},
		convert::TryFrom,
		vec::Vec,
	},
	traits::{BalanceStatus, EnsureOrigin},
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, CurrencyIdExt, LeasePeriod, ParaId, TokenInfo, TokenSymbol};
use orml_traits::{LockIdentifier, MultiCurrency, MultiLockableCurrency, MultiReservableCurrency};
pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

const DEPOSIT_ID: LockIdentifier = *b"deposit ";

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct PoolInfo<T: Config> {
	/// Id of the liquidity-pool
	pool_id: PoolId,
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
	min_deposit_to_start: BalanceOf<T>,
	/// The Second Condition
	///
	/// When starts the liquidity-pool, the current block should be greater than the value.
	after_block_to_start: BlockNumberFor<T>,

	/// The total amount deposited in the liquidity-pool
	deposit: BalanceOf<T>,

	/// The reward infos about the liquidity-pool
	rewards: BTreeMap<CurrencyId, RewardData<T>>,
	/// The block of the last update of the rewards
	update_b: BlockNumberFor<T>,
	/// The liquidity-pool state
	state: PoolState,
	/// The block number when the liquidity-pool startup
	block_startup: Option<BlockNumberFor<T>>,
	/// The block number when the liquidity-pool retired
	block_retired: Option<BlockNumberFor<T>>,
}

impl<T: Config> PoolInfo<T> {
	/// Trying to update the rewards
	pub(crate) fn try_update(mut self) -> Self {
		// When pool in `PoolState::Ongoing` or `PoolState::Retired`
		if let Some(block_startup) = self.block_startup {
			let block_retired = match self.block_retired {
				Some(block_retired) => block_retired,
				None => self.duration + block_startup,
			};
			let n = min(frame_system::Pallet::<T>::block_number(), block_retired);

			for (_, reward) in self.rewards.iter_mut() {
				reward.update(self.deposit, block_startup, self.update_b, n);
			}

			self.update_b = n;
		}

		self
	}

	/// Trying to change the state from `PoolState::Approved` to `PoolState::Ongoing`
	///
	/// __NOTE__: Only called in the `Hook`
	pub(crate) fn try_startup(mut self, n: BlockNumberFor<T>) -> Self {
		if self.state == PoolState::Approved {
			if n >= self.after_block_to_start && self.deposit >= self.min_deposit_to_start {
				self.block_startup = Some(n);
				self.state = PoolState::Ongoing;

				Pallet::<T>::deposit_event(Event::PoolStarted(
					self.pool_id,
					self.r#type,
					self.trading_pair,
				));
			}
		}

		self
	}

	/// Trying to change the state from `PoolState::Ongoing` to `PoolState::Retired`
	pub(crate) fn try_retire(mut self) -> Self {
		if self.state == PoolState::Ongoing {
			let n = frame_system::Pallet::<T>::block_number();

			if let Some(block_startup) = self.block_startup {
				let block_retired = block_startup + self.duration;
				if n >= block_retired {
					self.state = PoolState::Retired;
					self.block_retired = Some(block_retired);

					Pallet::<T>::deposit_event(Event::PoolRetired(
						self.pool_id,
						self.r#type,
						self.trading_pair,
					));
				}
			}
		}

		self
	}

	/// Trying account & transfer the rewards to user
	pub(crate) fn try_settle_and_transfer(
		&mut self,
		deposit_data: &mut DepositData<T>,
		user: AccountIdOf<T>,
	) -> Result<(), DispatchError> {
		let mut to_rewards = Vec::<(CurrencyId, BalanceOf<T>)>::new();

		// The pool was startup before.
		if let Some(_block_startup) = self.block_startup {
			for (rtoken, reward) in self.rewards.iter_mut() {
				let v_new = reward.gain_avg;
				if let Some(gain_avg) = deposit_data.gain_avgs.get(rtoken) {
					let v_old = *gain_avg;

					let user_deposit: u128 = deposit_data.deposit.saturated_into();
					let amount = BalanceOf::<T>::saturated_from(
						(v_new - v_old).saturating_mul_int(user_deposit),
					);

					// Update the claimed of the reward
					reward.claimed = reward.claimed.saturating_add(amount);
					// Sync the gain_avg between `DepositData` and `RewardData`
					deposit_data.gain_avgs.insert(*rtoken, v_new);
					deposit_data.update_b = self.update_b;

					to_rewards.push((*rtoken, amount));
				}
			}
		}

		for (rtoken, amount) in to_rewards.iter() {
			let remain = T::MultiCurrency::repatriate_reserved(
				*rtoken,
				&self.creator,
				&user,
				*amount,
				BalanceStatus::Free,
			)?;

			ensure!(remain == Zero::zero(), Error::<T>::Unexpected);
		}

		Pallet::<T>::deposit_event(Event::UserClaimed(
			self.pool_id,
			self.r#type,
			self.trading_pair,
			to_rewards,
			user,
		));

		Ok(().into())
	}
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Debug)]
pub enum PoolType {
	Mining,
	Farming,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Debug)]
pub enum PoolState {
	UnderAudit,
	Approved,
	Ongoing,
	Retired,
	Dead,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct DepositData<T: Config> {
	/// The amount of trading-pair deposited in the liquidity-pool
	deposit: BalanceOf<T>,
	/// The average gain in pico by 1 pico deposited from the startup of the liquidity-pool,
	/// updated when the `DepositData`'s owner deposits/redeems/claims from the liquidity-pool.
	///
	/// - Arg0: The average gain in pico by 1 pico deposited from the startup of the liquidity-pool
	/// - Arg1: The block number updated lastest
	gain_avgs: BTreeMap<CurrencyId, FixedU128>,
	update_b: BlockNumberFor<T>,
}

impl<T: Config> DepositData<T> {
	pub(crate) fn from_pool(pool: &PoolInfo<T>) -> Self {
		let mut gain_avgs = BTreeMap::<CurrencyId, FixedU128>::new();

		for (rtoken, reward) in pool.rewards.iter() {
			gain_avgs.insert(*rtoken, reward.gain_avg);
		}

		Self { deposit: Zero::zero(), gain_avgs, update_b: pool.update_b }
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
	/// The average gain in pico by 1 pico deposited from the startup of the liquidity-pool,
	/// updated when anyone deposits to / redeems from / claims from the liquidity-pool.
	gain_avg: FixedU128,
}

impl<T: Config> RewardData<T> {
	fn new(total: BalanceOf<T>, duration: BlockNumberFor<T>) -> Result<Self, DispatchError> {
		let total: u128 = total.saturated_into();
		let (per_block, total) = {
			let duration: u128 = duration.saturated_into();

			let per_block = total / duration;
			let total = per_block * duration;

			(BalanceOf::<T>::saturated_from(per_block), BalanceOf::<T>::saturated_from(total))
		};

		ensure!(per_block > T::MinimumRewardPerBlock::get(), Error::<T>::InvalidRewardPerBlock);

		Ok(RewardData { total, per_block, claimed: Zero::zero(), gain_avg: 0.into() })
	}

	pub(crate) fn per_block_per_deposited(&self, deposited: BalanceOf<T>) -> FixedU128 {
		let per_block: u128 = self.per_block.saturated_into();
		let deposit: u128 = deposited.saturated_into();

		match deposit {
			0 => 0.into(),
			_ => FixedU128::from((per_block, deposit)),
		}
	}

	/// Trying to update the gain_avg
	pub(crate) fn update(
		&mut self,
		deposit: BalanceOf<T>,
		block_startup: BlockNumberFor<T>,
		block_last_updated: BlockNumberFor<T>,
		n: BlockNumberFor<T>,
	) {
		let pbpd = self.per_block_per_deposited(deposit);

		let b_prev = max(block_last_updated, block_startup);
		let b_past: u128 = (n - b_prev).saturated_into();

		let gain_avg_new = self.gain_avg + pbpd * b_past.into();

		self.gain_avg = gain_avg_new;
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

		/// Origin for anyone able to create/approve/kill the liquidity-pool.
		type ControlOrigin: EnsureOrigin<Self::Origin>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The value used to construct vsbond when creating a farming-liquidity-pool
		#[pallet::constant]
		type RelayChainTokenSymbol: Get<TokenSymbol>;

		/// The amount deposited into a liquidity-pool should be less than the value
		#[pallet::constant]
		type MaximumDepositInPool: Get<BalanceOf<Self>>;

		/// The amount deposited by a user to a liquidity-pool should be greater than the value
		#[pallet::constant]
		type MinimumDepositOfUser: Get<BalanceOf<Self>>;

		/// The amount of token to reward per block should be greater than the value
		#[pallet::constant]
		type MinimumRewardPerBlock: Get<BalanceOf<Self>>;

		/// The duration of a liquidity-pool should be greater than the value
		#[pallet::constant]
		type MinimumDuration: Get<BlockNumberFor<Self>>;

		/// The number of liquidity-pool approved should be less than the value
		#[pallet::constant]
		type MaximumApproved: Get<u32>;
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
		ExceedMaximumDeposit,
		/// When the number of pool-approved exceeds the `MaximumApproved`
		ExceedMaximumApproved,
		/// Not enough balance to deposit
		NotEnoughToDeposit,
		/// Not enough balance of reward to unreserve
		FailOnUnReserve,
		/// Not enough deposit of the user in the liquidity-pool
		NoDepositOfUser,
		/// Too low balance to deposit
		TooLowToDeposit,
		/// The deposit in liquidity-pool ongoing should be greater than `T::MinimumDeposit`
		TooLowDepositInPoolToRedeem,
		/// The interval between two claims is short
		TooShortBetweenTwoClaim,
		/// __NOTE__: ERROR HAPPEN
		Unexpected,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The liquidity-pool has been created
		///
		/// [pool_id, pool_type, trading_pair, creator]
		PoolCreated(PoolId, PoolType, (CurrencyId, CurrencyId), AccountIdOf<T>),
		/// The liquidity-pool has been approved
		///
		/// [pool_id, pool_type, trading_pair]
		PoolApproved(PoolId, PoolType, (CurrencyId, CurrencyId)),
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
	#[pallet::getter(fn approved_pids)]
	pub(crate) type ApprovedPoolIds<T: Config> = StorageValue<_, BTreeSet<PoolId>, ValueQuery>;

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
			#[pallet::compact] min_deposit_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			// Order the trading_pair
			let (token1, token2) = trading_pair;

			ensure!(!token1.is_vsbond() && !token1.is_lptoken(), Error::<T>::InvalidTradingPair);
			ensure!(!token2.is_vsbond() && !token2.is_lptoken(), Error::<T>::InvalidTradingPair);

			let (id1, id2) = (token1.currency_id(), token2.currency_id());
			let trading_pair = if id1 <= id2 { (token1, token2) } else { (token2, token1) };

			Self::create_pool(
				origin,
				trading_pair,
				main_reward,
				option_rewards,
				PoolType::Mining,
				duration,
				min_deposit_to_start,
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
			#[pallet::compact] min_deposit_to_start: BalanceOf<T>,
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
				min_deposit_to_start,
				after_block_to_start,
			)?;

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn approve_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			let num = Self::approved_pids().len() as u32;
			ensure!(num < T::MaximumApproved::get(), Error::<T>::ExceedMaximumApproved);

			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?;

			ensure!(pool.state == PoolState::UnderAudit, Error::<T>::InvalidPoolState);

			ApprovedPoolIds::<T>::mutate(|pids| pids.insert(pid));

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			let pool_approved = PoolInfo { state: PoolState::Approved, ..pool };
			TotalPoolInfos::<T>::insert(pid, pool_approved);

			Self::deposit_event(Event::PoolApproved(pid, r#type, trading_pair));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn kill_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let signed = ensure_signed(origin)?;

			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?;

			ensure!(signed == pool.creator, Error::<T>::InvalidPoolOwner);

			ensure!(pool.state == PoolState::UnderAudit, Error::<T>::InvalidPoolState);

			for (token, reward) in pool.rewards.iter() {
				let total = reward.total;
				let remain = T::MultiCurrency::unreserve(*token, &signed, total);
				ensure!(remain == Zero::zero(), Error::<T>::FailOnUnReserve);
			}

			let pool_killed = PoolInfo { state: PoolState::Dead, ..pool };
			TotalPoolInfos::<T>::remove(pid);

			Self::deposit_event(Event::PoolKilled(
				pid,
				pool_killed.r#type,
				pool_killed.trading_pair,
			));

			Ok(().into())
		}

		#[pallet::weight(1_000)]
		pub fn force_retire_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire();

			ensure!(
				pool.state == PoolState::Approved || pool.state == PoolState::Ongoing,
				Error::<T>::InvalidPoolState
			);

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			let pool_retired = PoolInfo {
				state: PoolState::Retired,
				block_retired: Some(frame_system::Pallet::<T>::block_number()),
				..pool
			};
			TotalPoolInfos::<T>::insert(pid, pool_retired);

			Self::deposit_event(Event::PoolRetired(pid, r#type, trading_pair));

			Ok(().into())
		}

		/// User deposits some token to a liquidity-pool.
		///
		/// The extrinsic will:
		/// - Try to retire the liquidity-pool which has reached the end of life.
		/// - Try to settle the rewards when the liquidity-pool in `Ongoing`.
		///
		/// The conditions to deposit:
		/// - User should deposit enough(greater than `T::MinimumDeposit`) token to liquidity-pool;
		/// - The liquidity-pool should be in special state: `Approved`, `Ongoing`;
		#[pallet::weight(1_000)]
		pub fn deposit(
			origin: OriginFor<T>,
			pid: PoolId,
			value: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			let mut pool: PoolInfo<T> =
				Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire().try_update();

			ensure!(
				pool.state == PoolState::Approved || pool.state == PoolState::Ongoing,
				Error::<T>::InvalidPoolState
			);

			ensure!(value >= T::MinimumDepositOfUser::get(), Error::<T>::TooLowToDeposit);

			let mut deposit_data: DepositData<T> =
				Self::user_deposit_data(&user, &pid).unwrap_or(DepositData::<T>::from_pool(&pool));

			if pool.state == PoolState::Ongoing && pool.update_b != deposit_data.update_b {
				pool.try_settle_and_transfer(&mut deposit_data, user.clone())?;
			}

			deposit_data.deposit = deposit_data.deposit.saturating_add(value);
			pool.deposit = pool.deposit.saturating_add(value);
			ensure!(
				pool.deposit <= T::MaximumDepositInPool::get(),
				Error::<T>::ExceedMaximumDeposit
			);

			// To lock the deposit
			if pool.r#type == PoolType::Mining {
				let lpt = Self::convert_to_lptoken(pool.trading_pair)?;

				T::MultiCurrency::ensure_can_withdraw(lpt, &user, value)
					.map_err(|_e| Error::<T>::NotEnoughToDeposit)?;

				T::MultiCurrency::extend_lock(DEPOSIT_ID, lpt, &user, deposit_data.deposit)?;
			} else {
				let (token_a, token_b) = pool.trading_pair;

				T::MultiCurrency::ensure_can_withdraw(token_a, &user, value)
					.map_err(|_e| Error::<T>::NotEnoughToDeposit)?;
				T::MultiCurrency::ensure_can_withdraw(token_b, &user, value)
					.map_err(|_e| Error::<T>::NotEnoughToDeposit)?;

				T::MultiCurrency::extend_lock(DEPOSIT_ID, token_a, &user, deposit_data.deposit)?;
				T::MultiCurrency::extend_lock(DEPOSIT_ID, token_b, &user, deposit_data.deposit)?;
			}

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			TotalPoolInfos::<T>::insert(pid, pool);
			TotalDepositData::<T>::insert(user.clone(), pid, deposit_data);

			Self::deposit_event(Event::UserDeposited(pid, r#type, trading_pair, value, user));

			Ok(().into())
		}

		/// User redeems all deposit from a liquidity-pool.
		/// The deposit in the liquidity-pool should be greater than `T::MinimumDeposit` when the
		/// liquidity-pool is on `Ongoing` state; So user may not be able to redeem completely
		/// until the liquidity-pool is on `Retire` state.
		///
		/// The extrinsic will:
		/// - Try to retire the liquidity-pool which has reached the end of life.
		/// - Try to settle the rewards when the liquidity-pool in `Ongoing`.
		/// - Try to unreserve the remaining rewards to the pool creator when the deposit in the
		///   liquidity-pool is clear.
		/// - Try to delete the liquidity-pool in which the deposit becomes zero.
		/// - Try to delete the deposit-data in which the deposit becomes zero.
		///
		/// The condition to redeem:
		/// - User should have some deposit in the liquidity-pool;
		/// - The liquidity-pool should be in special state: `Ongoing`, `Retired`;
		#[pallet::weight(1_000)]
		pub fn redeem(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			let mut pool: PoolInfo<T> =
				Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire().try_update();

			ensure!(
				pool.state == PoolState::Ongoing || pool.state == PoolState::Retired,
				Error::<T>::InvalidPoolState
			);

			let mut deposit_data: DepositData<T> =
				Self::user_deposit_data(&user, &pid).ok_or(Error::<T>::NoDepositOfUser)?;

			if pool.update_b != deposit_data.update_b {
				pool.try_settle_and_transfer(&mut deposit_data, user.clone())?;
			}

			// Keep minimum deposit in pool when the pool is ongoing.
			let minimum_in_pool = match pool.state {
				PoolState::Ongoing => T::MinimumDepositOfUser::get(),
				PoolState::Retired => Zero::zero(),
				_ => return Err(Error::<T>::InvalidPoolState.into()),
			};

			let try_redeemed = deposit_data.deposit;
			let left_in_pool = max(pool.deposit - try_redeemed, minimum_in_pool);
			let can_redeemed = pool.deposit - left_in_pool;
			let left_in_user = deposit_data.deposit - can_redeemed;

			ensure!(can_redeemed != Zero::zero(), Error::<T>::TooLowDepositInPoolToRedeem);

			// To unlock the deposit
			match pool.r#type {
				PoolType::Mining => {
					let lpt = Self::convert_to_lptoken(pool.trading_pair)?;
					match left_in_user.saturated_into() {
						0u128 => T::MultiCurrency::remove_lock(DEPOSIT_ID, lpt, &user)?,
						_ => T::MultiCurrency::set_lock(DEPOSIT_ID, lpt, &user, left_in_user)?,
					}
				},
				PoolType::Farming => {
					let (token_a, token_b) = pool.trading_pair;
					match left_in_user.saturated_into() {
						0u128 => {
							T::MultiCurrency::remove_lock(DEPOSIT_ID, token_a, &user)?;
							T::MultiCurrency::remove_lock(DEPOSIT_ID, token_b, &user)?;
						},
						_ => {
							T::MultiCurrency::set_lock(DEPOSIT_ID, token_a, &user, left_in_user)?;
							T::MultiCurrency::set_lock(DEPOSIT_ID, token_b, &user, left_in_user)?;
						},
					}
				},
			};

			deposit_data.deposit = left_in_user;
			pool.deposit = left_in_pool;

			if pool.state == PoolState::Retired && pool.deposit == Zero::zero() {
				for (rtoken, reward) in pool.rewards.iter() {
					let remain = reward.total - reward.claimed;
					ensure!(
						T::MultiCurrency::unreserve(*rtoken, &pool.creator, remain) == Zero::zero(),
						Error::<T>::Unexpected
					);
				}

				pool.state = PoolState::Dead;
			}

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			match pool.deposit.saturated_into() {
				0u128 => TotalPoolInfos::<T>::remove(pid),
				_ => TotalPoolInfos::<T>::insert(pid, pool),
			}

			match deposit_data.deposit.saturated_into() {
				0u128 => TotalDepositData::<T>::remove(user.clone(), pid),
				_ => TotalDepositData::<T>::insert(user.clone(), pid, deposit_data),
			}

			Self::deposit_event(Event::UserRedeemed(pid, r#type, trading_pair, try_redeemed, user));

			Ok(().into())
		}

		/// User claims the rewards from a liquidity-pool.
		///
		/// The extrinsic will:
		/// - Try to retire the liquidity-pool which has reached the end of life.
		///
		/// The conditions to claim:
		/// - User should have enough token deposited in the liquidity-pool;
		/// - The liquidity-pool should be in special states: `Ongoing`, `Retired`;
		#[pallet::weight(1_000)]
		pub fn claim(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			let mut pool: PoolInfo<T> =
				Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire().try_update();

			ensure!(
				pool.state == PoolState::Ongoing || pool.state == PoolState::Retired,
				Error::<T>::InvalidPoolState
			);

			let mut deposit_data: DepositData<T> =
				Self::user_deposit_data(&user, &pid).ok_or(Error::<T>::NoDepositOfUser)?;

			ensure!(pool.update_b != deposit_data.update_b, Error::<T>::TooShortBetweenTwoClaim);
			pool.try_settle_and_transfer(&mut deposit_data, user.clone())?;

			TotalPoolInfos::<T>::insert(pid, pool);
			TotalDepositData::<T>::insert(user, pid, deposit_data);

			Ok(().into())
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
			min_deposit_to_start: BalanceOf<T>,
			after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResult {
			let creator = ensure_signed(origin)?;

			// Check the trading-pair
			ensure!(trading_pair.0 != trading_pair.1, Error::<T>::InvalidTradingPair);

			// Check the duration
			ensure!(duration > T::MinimumDuration::get(), Error::<T>::InvalidDuration);

			// Check the condition
			ensure!(
				min_deposit_to_start >= T::MinimumDepositOfUser::get(),
				Error::<T>::InvalidDepositLimit
			);
			ensure!(
				min_deposit_to_start <= T::MaximumDepositInPool::get(),
				Error::<T>::InvalidDepositLimit
			);

			// Check & Construct the rewards
			let raw_rewards: Vec<(CurrencyId, BalanceOf<T>)> =
				option_rewards.into_iter().chain(Some(main_reward).into_iter()).collect();
			let mut rewards: BTreeMap<CurrencyId, RewardData<T>> = BTreeMap::new();
			for (token, total) in raw_rewards.into_iter() {
				ensure!(!rewards.contains_key(&token), Error::<T>::DuplicateReward);

				let reward = RewardData::new(total, duration)?;

				rewards.insert(token, reward);
			}

			// Reserve rewards
			for (token, reward) in rewards.iter() {
				T::MultiCurrency::reserve(*token, &creator, reward.total)?;
			}

			// Construct the PoolInfo
			let pool_id = Self::next_pool_id();
			let mining_pool = PoolInfo {
				pool_id,
				creator: creator.clone(),
				trading_pair,
				duration,
				r#type,

				min_deposit_to_start,
				after_block_to_start,

				deposit: Zero::zero(),

				rewards,
				update_b: Zero::zero(),
				state: PoolState::UnderAudit,
				block_startup: None,
				block_retired: None,
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

		pub(crate) fn convert_to_lptoken(
			trading_pair_ordered: (CurrencyId, CurrencyId),
		) -> Result<CurrencyId, DispatchError> {
			let (token1, token2) = trading_pair_ordered;
			let (discr1, discr2) = (token1.discriminant(), token2.discriminant());
			let (sid1, sid2) = (
				(token1.currency_id() & 0x0000_0000_0000_00ff) as u8,
				(token2.currency_id() & 0x0000_0000_0000_00ff) as u8,
			);
			let (sym1, sym2) = (
				TokenSymbol::try_from(sid1).map_err(|_| Error::<T>::InvalidTradingPair)?,
				TokenSymbol::try_from(sid2).map_err(|_| Error::<T>::InvalidTradingPair)?,
			);

			Ok(CurrencyId::LPToken(sym1, discr1, sym2, discr2))
		}

		#[allow(non_snake_case)]
		pub(crate) fn vsAssets(
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
		) -> (CurrencyId, CurrencyId) {
			let token_symbol = T::RelayChainTokenSymbol::get();

			let vsToken = CurrencyId::VSToken(token_symbol);
			let vsBond = CurrencyId::VSBond(token_symbol, index, first_slot, last_slot);

			(vsToken, vsBond)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			// Check whether pool-activated is meet the startup condition
			for pid in Self::approved_pids() {
				if let Some(mut pool) = Self::pool(pid) {
					pool = pool.try_startup(n);

					if pool.state == PoolState::Ongoing {
						ApprovedPoolIds::<T>::mutate(|pids| pids.remove(&pid));
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

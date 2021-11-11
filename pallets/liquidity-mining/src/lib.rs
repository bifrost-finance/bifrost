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
		traits::{AccountIdConversion, SaturatedConversion, Saturating, Zero},
		FixedPointNumber, FixedU128,
	},
	sp_std::{
		cmp::{max, min},
		collections::{btree_map::BTreeMap, btree_set::BTreeSet},
		convert::TryFrom,
		vec::Vec,
	},
	traits::EnsureOrigin,
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, CurrencyIdExt, LeasePeriod, ParaId, TokenInfo, TokenSymbol};
use orml_traits::{MultiCurrency, MultiLockableCurrency, MultiReservableCurrency};
pub use pallet::*;
use scale_info::TypeInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub use weights::*;

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
pub struct PoolInfo<T: Config> {
	/// Id of the liquidity-pool
	pool_id: PoolId,
	/// The keeper of the liquidity-pool
	keeper: AccountIdOf<T>,
	/// The man who charges the rewards to the pool
	investor: Option<AccountIdOf<T>>,
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
				None => self.duration.saturating_add(block_startup),
			};
			let n = min(frame_system::Pallet::<T>::block_number(), block_retired);

			for (_, reward) in self.rewards.iter_mut() {
				reward.update(self.deposit, block_startup, self.update_b, n);
			}

			self.update_b = n;
		}

		self
	}

	/// Trying to change the state from `PoolState::Charged` to `PoolState::Ongoing`
	///
	/// __NOTE__: Only called in the `Hook`
	pub(crate) fn try_startup(mut self, n: BlockNumberFor<T>) -> Self {
		if self.state == PoolState::Charged {
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
				let block_retired = block_startup.saturating_add(self.duration);
				if n >= block_retired {
					self.state = PoolState::Retired;
					self.block_retired = Some(block_retired);
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
						v_new.saturating_sub(v_old).saturating_mul_int(user_deposit),
					);

					// Sync the gain_avg between `DepositData` and `RewardData`
					deposit_data.gain_avgs.insert(*rtoken, v_new);
					deposit_data.update_b = self.update_b;

					let ed = T::MultiCurrency::minimum_balance(*rtoken);
					let total =
						T::MultiCurrency::total_balance(*rtoken, &user).saturating_add(amount);

					if total >= ed {
						// Update the claimed of the reward
						reward.claimed = reward.claimed.saturating_add(amount);
						to_rewards.push((*rtoken, amount));
					}
				}
			}
		}

		for (rtoken, amount) in to_rewards.iter() {
			T::MultiCurrency::ensure_can_withdraw(*rtoken, &self.keeper, *amount)?;
		}

		for (rtoken, amount) in to_rewards.iter() {
			T::MultiCurrency::transfer(*rtoken, &self.keeper, &user, *amount)?;
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

	/// Try to return back the remain of reward from keeper to investor
	pub(crate) fn try_withdraw_remain(&self) -> DispatchResult {
		let investor = self.investor.clone().ok_or(Error::<T>::Unexpected)?;

		for (rtoken, reward) in self.rewards.iter() {
			let remain = reward.total.saturating_sub(reward.claimed);
			let can_send =
				T::MultiCurrency::ensure_can_withdraw(*rtoken, &self.keeper, remain).is_ok();

			let ed = T::MultiCurrency::minimum_balance(*rtoken);
			let total = T::MultiCurrency::total_balance(*rtoken, &investor).saturating_add(remain);
			let can_get = total >= ed;

			if can_send && can_get {
				T::MultiCurrency::transfer(*rtoken, &self.keeper, &investor, remain)?;
			}
		}

		Ok(().into())
	}
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Debug, TypeInfo)]
pub enum PoolType {
	/// Only `LpToken` can deposit into the pool
	Mining,
	/// Only `vsToken` + `vsBond` can deposit into the pool
	Farming,
	/// Only `vsToken(reserved)` + `vsBond(reserved)` can deposit into the pool
	EBFarming,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Debug, TypeInfo)]
pub enum PoolState {
	UnCharged,
	Charged,
	Ongoing,
	Retired,
	Dead,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
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

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
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
			let total = per_block.saturating_mul(duration);

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
		let b_past: u128 = n.saturating_sub(b_prev).saturated_into();

		let gain_avg_new = self.gain_avg.saturating_add(pbpd.saturating_mul(b_past.into()));

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

type PoolId = u32;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + TypeInfo {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Origin for anyone able to create/kill/force_retire the liquidity-pool.
		type ControlOrigin: EnsureOrigin<Self::Origin>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The value used to construct vsbond when creating a farming-liquidity-pool
		#[pallet::constant]
		type RelayChainTokenSymbol: Get<TokenSymbol>;

		/// The deposit of a liquidity-pool should be less than the value
		#[pallet::constant]
		type MaximumDepositInPool: Get<BalanceOf<Self>>;

		/// The amount which be deposited to a liquidity-pool should be greater than the value
		#[pallet::constant]
		type MinimumDepositOfUser: Get<BalanceOf<Self>>;

		/// The amount of reward which will be released per block should be greater than the value
		#[pallet::constant]
		type MinimumRewardPerBlock: Get<BalanceOf<Self>>;

		/// The duration of a liquidity-pool should be greater than the value
		#[pallet::constant]
		type MinimumDuration: Get<BlockNumberFor<Self>>;

		/// The count of liquidity-pool charged should be less than the value
		#[pallet::constant]
		type MaximumCharged: Get<u32>;

		/// The count of option rewards should be less than the value
		#[pallet::constant]
		type MaximumOptionRewards: Get<u32>;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidTradingPair,
		InvalidDuration,
		InvalidRewardPerBlock,
		InvalidDepositLimit,
		InvalidPoolId,
		InvalidPoolState,
		InvalidPoolType,
		/// Find duplicate rewards when creating the liquidity-pool
		DuplicateReward,
		/// The deposit of a liquidity-pool exceeded the `MaximumDepositInPool`
		ExceedMaximumDeposit,
		/// The number of pool which be charged exceeded the `MaximumCharged`
		ExceedMaximumCharged,
		/// User doesn't have enough balance of which be deposited to pool
		NotEnoughToDeposit,
		/// Keeper doesn't have enough balance to be redeemed by the user(VERY SCARY ERR)
		NotEnoughToRedeem,
		/// User has nothing be deposited to the pool
		NoDepositOfUser,
		/// The balance which was tried to deposit to the pool less than `MinimumDepositOfUser`
		TooLowToDeposit,
		/// User doesn't have such amount deposit can be redeemed from the pool
		TooLowToRedeem,
		/// Duplicate claim actions were at same block height
		TooShortBetweenTwoClaim,
		/// The pool has been charged already
		PoolChargedAlready,
		/// __NOTE__: ERROR HAPPEN
		Unexpected,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The liquidity-pool was created
		///
		/// [pool_id, pool_type, trading_pair, keeper]
		PoolCreated(PoolId, PoolType, (CurrencyId, CurrencyId), AccountIdOf<T>),
		/// The liquidity-pool was charged
		///
		/// [pool_id, pool_type, trading_pair, investor]
		PoolCharged(PoolId, PoolType, (CurrencyId, CurrencyId), AccountIdOf<T>),
		/// The liquidity-pool was started up
		///
		/// [pool_id, pool_type, trading_pair]
		PoolStarted(PoolId, PoolType, (CurrencyId, CurrencyId)),
		/// The liquidity-pool was killed
		///
		/// [pool_id, pool_type, trading_pair]
		PoolKilled(PoolId, PoolType, (CurrencyId, CurrencyId)),
		/// The liquidity-pool was retired forcefully
		///
		/// [pool_id, pool_type, trading_pair]
		PoolRetiredForcefully(PoolId, PoolType, (CurrencyId, CurrencyId)),
		/// User deposited tokens to a liquidity-pool
		///
		/// [pool_id, pool_type, trading_pair, amount_deposited, user]
		UserDeposited(PoolId, PoolType, (CurrencyId, CurrencyId), BalanceOf<T>, AccountIdOf<T>),
		/// User redeemed tokens from a liquidity-mining
		///
		/// [pool_id, pool_type, trading_pair, amount_redeemed, user]
		UserRedeemed(PoolId, PoolType, (CurrencyId, CurrencyId), BalanceOf<T>, AccountIdOf<T>),
		/// User withdrew the rewards whose deserved from a liquidity-pool
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
	pub(crate) type NextPoolId<T: Config> = StorageValue<_, PoolId, ValueQuery>;

	/// The storage is used to store pool-ids which point to the Pools at `PoolState::Charged`.
	///
	/// Actually, the pools(that the storage points to) are pending to be activated by `Hook`;
	/// The activation means converting the pools from `PoolState::Charged` to `PoolState::Ongoing`
	/// after the conditions that are set at the pool-creation stage are met.
	#[pallet::storage]
	#[pallet::getter(fn charged_pids)]
	pub(crate) type ChargedPoolIds<T: Config> = StorageValue<_, BTreeSet<PoolId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pool)]
	pub(crate) type TotalPoolInfos<T: Config> = StorageMap<_, Twox64Concat, PoolId, PoolInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn user_deposit_data)]
	pub(crate) type TotalDepositData<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PoolId,
		Blake2_128Concat,
		AccountIdOf<T>,
		DepositData<T>,
	>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a liquidity-pool which type is `PoolType::Mining`, Only accepts `lpToken` as
		/// deposit.
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn create_mining_pool(
			origin: OriginFor<T>,
			trading_pair: (CurrencyId, CurrencyId),
			main_reward: (CurrencyId, BalanceOf<T>),
			option_rewards: BoundedVec<(CurrencyId, BalanceOf<T>), T::MaximumOptionRewards>,
			#[pallet::compact] duration: BlockNumberFor<T>,
			#[pallet::compact] min_deposit_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			// Order the trading_pair
			let (token1, token2) = trading_pair;

			ensure!(!token1.is_vsbond() && !token1.is_lptoken(), Error::<T>::InvalidTradingPair);
			ensure!(!token2.is_vsbond() && !token2.is_lptoken(), Error::<T>::InvalidTradingPair);

			let (id1, id2) = (token1.currency_id(), token2.currency_id());
			let trading_pair = if id1 <= id2 { (token1, token2) } else { (token2, token1) };

			Self::create_pool(
				trading_pair,
				main_reward,
				option_rewards,
				PoolType::Mining,
				duration,
				min_deposit_to_start,
				after_block_to_start,
			)
		}

		/// Create a liquidity-pool which type is `PoolType::Farming`, Only accepts free `vsToken`
		/// and free `vsBond` as deposit.
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn create_farming_pool(
			origin: OriginFor<T>,
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
			main_reward: (CurrencyId, BalanceOf<T>),
			option_rewards: BoundedVec<(CurrencyId, BalanceOf<T>), T::MaximumOptionRewards>,
			#[pallet::compact] duration: BlockNumberFor<T>,
			#[pallet::compact] min_deposit_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			#[allow(non_snake_case)]
			let trading_pair =
				CurrencyId::vsAssets(T::RelayChainTokenSymbol::get(), index, first_slot, last_slot);

			Self::create_pool(
				trading_pair,
				main_reward,
				option_rewards,
				PoolType::Farming,
				duration,
				min_deposit_to_start,
				after_block_to_start,
			)
		}

		/// Create a liquidity-pool which type is `PoolType::Farming`, Only accepts reserved
		/// `vsToken` and reserved `vsBond` as deposit.
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn create_eb_farming_pool(
			origin: OriginFor<T>,
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
			main_reward: (CurrencyId, BalanceOf<T>),
			option_rewards: BoundedVec<(CurrencyId, BalanceOf<T>), T::MaximumOptionRewards>,
			#[pallet::compact] duration: BlockNumberFor<T>,
			#[pallet::compact] min_deposit_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			#[allow(non_snake_case)]
			let trading_pair =
				CurrencyId::vsAssets(T::RelayChainTokenSymbol::get(), index, first_slot, last_slot);

			Self::create_pool(
				trading_pair,
				main_reward,
				option_rewards,
				PoolType::EBFarming,
				duration,
				min_deposit_to_start,
				after_block_to_start,
			)
		}

		/// Transfer the rewards which are used to distribute to depositors to a liquidity-pool.
		///
		/// _NOTE_: The extrinsic is only applied to the liquidity-pool at `PoolState::UnCharged`;
		/// 	When the extrinsic was executed successfully, the liquidity-pool would be at
		/// 	`PoolState::Charged`.
		#[pallet::weight(T::WeightInfo::charge())]
		pub fn charge(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let investor = ensure_signed(origin)?;

			let num = Self::charged_pids().len() as u32;
			ensure!(num < T::MaximumCharged::get(), Error::<T>::ExceedMaximumCharged);

			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?;

			ensure!(pool.state == PoolState::UnCharged, Error::<T>::InvalidPoolState);
			ensure!(pool.investor.is_none(), Error::<T>::PoolChargedAlready);

			for (token, reward) in pool.rewards.iter() {
				T::MultiCurrency::ensure_can_withdraw(*token, &investor, reward.total)?;
			}

			for (token, reward) in pool.rewards.iter() {
				T::MultiCurrency::transfer(*token, &investor, &pool.keeper, reward.total)?;
			}

			ChargedPoolIds::<T>::mutate(|pids| pids.insert(pid));

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			let pool_charged =
				PoolInfo { state: PoolState::Charged, investor: Some(investor.clone()), ..pool };
			TotalPoolInfos::<T>::insert(pid, pool_charged);

			Self::deposit_event(Event::PoolCharged(pid, r#type, trading_pair, investor));

			Ok(().into())
		}

		/// Kill a liquidity-pool at `PoolState::Uncharged`.
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn kill_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?;

			ensure!(pool.state == PoolState::UnCharged, Error::<T>::InvalidPoolState);

			let pool_killed = PoolInfo { state: PoolState::Dead, ..pool };
			TotalPoolInfos::<T>::remove(pid);

			Self::deposit_event(Event::PoolKilled(
				pid,
				pool_killed.r#type,
				pool_killed.trading_pair,
			));

			Ok(().into())
		}

		/// Make a liquidity-pool be at `PoolState::Retired` forcefully.
		///
		/// __NOTE__:
		/// 1. If the pool is at `PoolState::Charged` but doesn't have any deposit, the data about
		/// 	the pool would be deleted and the rewards charged would be returned back.
		///
		/// 2. If the pool is at `PoolState::Charged` and has some deposit, or `PoolState::Ongoing`,
		/// 	the field `block_retired` of the pool would be set to the current block height.
		#[pallet::weight((
		0,
		DispatchClass::Normal,
		Pays::No
		))]
		pub fn force_retire_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire();

			ensure!(
				pool.state == PoolState::Charged || pool.state == PoolState::Ongoing,
				Error::<T>::InvalidPoolState
			);

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			if pool.state == PoolState::Charged {
				ChargedPoolIds::<T>::mutate(|pids| pids.remove(&pid));
			}

			match pool.state {
				PoolState::Charged if pool.deposit == Zero::zero() => {
					pool.try_withdraw_remain()?;
					TotalPoolInfos::<T>::remove(pid);
				},
				PoolState::Charged | PoolState::Ongoing => {
					let pool_retired = PoolInfo {
						state: PoolState::Retired,
						block_retired: Some(frame_system::Pallet::<T>::block_number()),
						..pool
					};
					TotalPoolInfos::<T>::insert(pid, pool_retired);
				},
				_ => {},
			}

			Self::deposit_event(Event::PoolRetiredForcefully(pid, r#type, trading_pair));

			Ok(().into())
		}

		/// Caller deposits some token to a liquidity-pool.
		///
		/// __NOTE__: The unclaimed rewards of caller will be withdrawn automatically if there has.
		///
		/// The conditions to deposit:
		/// - The deposit caller was contributed to the pool should be bigger than
		///   `T::MinimumDeposit`;
		/// - The pool is at `PoolState::Charged` or `PoolState::Ongoing`;
		#[transactional]
		#[pallet::weight(T::WeightInfo::deposit())]
		pub fn deposit(
			origin: OriginFor<T>,
			pid: PoolId,
			value: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			let mut pool: PoolInfo<T> =
				Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire().try_update();

			ensure!(
				pool.state == PoolState::Charged || pool.state == PoolState::Ongoing,
				Error::<T>::InvalidPoolState
			);

			ensure!(value >= T::MinimumDepositOfUser::get(), Error::<T>::TooLowToDeposit);

			let mut deposit_data: DepositData<T> = Self::user_deposit_data(pid, user.clone())
				.unwrap_or(DepositData::<T>::from_pool(&pool));

			if pool.state == PoolState::Ongoing && pool.update_b != deposit_data.update_b {
				pool.try_settle_and_transfer(&mut deposit_data, user.clone())?;
			}

			deposit_data.deposit = deposit_data.deposit.saturating_add(value);
			pool.deposit = pool.deposit.saturating_add(value);
			ensure!(
				pool.deposit <= T::MaximumDepositInPool::get(),
				Error::<T>::ExceedMaximumDeposit
			);

			// To "lock" the deposit
			match pool.r#type {
				PoolType::Mining => {
					let lpt = Self::convert_to_lptoken(pool.trading_pair)?;

					T::MultiCurrency::transfer(lpt, &user, &pool.keeper, value)
						.map_err(|_e| Error::<T>::NotEnoughToDeposit)?;
				},
				PoolType::Farming => {
					let (token_a, token_b) = pool.trading_pair;

					T::MultiCurrency::transfer(token_a, &user, &pool.keeper, value)
						.map_err(|_e| Error::<T>::NotEnoughToDeposit)?;
					T::MultiCurrency::transfer(token_b, &user, &pool.keeper, value)
						.map_err(|_e| Error::<T>::NotEnoughToDeposit)?;
				},
				PoolType::EBFarming => {
					let (token_a, token_b) = pool.trading_pair;

					ensure!(
						T::MultiCurrency::reserved_balance(token_a, &user) >= deposit_data.deposit,
						Error::<T>::NotEnoughToDeposit
					);
					ensure!(
						T::MultiCurrency::reserved_balance(token_b, &user) >= deposit_data.deposit,
						Error::<T>::NotEnoughToDeposit
					);
				},
			}

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			TotalPoolInfos::<T>::insert(pid, pool);
			TotalDepositData::<T>::insert(pid, user.clone(), deposit_data);

			Self::deposit_event(Event::UserDeposited(pid, r#type, trading_pair, value, user));

			Ok(().into())
		}

		/// Caller redeems some deposit owned by self from a pool.
		///
		/// __NOTE__: The unclaimed rewards of caller will be withdrawn automatically if there has.
		///
		/// __NOTE__:
		/// 0. If the pool is at `PoolState::Ongoing`, the caller may not redeem successfully
		/// because of 	the `reward algorithm`, which requires `pool-ongoing` must have deposit more
		/// than `T::MinimumDeposit`.
		///
		/// 1. If the pool is at `PoolState::Retired`, the extrinsic will redeem all deposits
		/// owned by the caller, whatever the `value` is.
		///
		/// 2. If the pool is at `PoolState::Retired` and the deposit in the pool will become zero
		/// after calling the extrinsic, the remaining rewards left in the pool will be returned
		/// back to the charger.
		///
		/// The condition to redeem:
		/// - There is enough deposit owned by the caller in the pool.
		/// - The pool is at `PoolState::Ongoing` or `PoolState::Retired`.
		#[transactional]
		#[pallet::weight(T::WeightInfo::redeem())]
		pub fn redeem(
			origin: OriginFor<T>,
			pid: PoolId,
			value: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			if value == Zero::zero() {
				return Ok(().into());
			}

			let user = ensure_signed(origin)?;

			Self::redeem_inner(user, pid, Some(value))
		}

		/// Caller redeems all deposit owned by self from a pool.
		///
		/// __NOTE__: The unclaimed rewards of caller will be withdrawn automatically if there has.
		///
		/// __NOTE__:
		/// 0. If the pool is at `PoolState::Ongoing`, the caller may not redeem successfully
		/// because of 	the `reward algorithm`, which requires `pool-ongoing` must have deposit more
		/// than `T::MinimumDeposit`.
		///
		/// 1. If the pool is at `PoolState::Retired` and the deposit in the pool will become zero
		/// after calling the extrinsic, the remaining rewards left in the pool will be
		/// returned back to the charger.
		///
		/// The condition to redeem:
		/// - There is enough deposit owned by the caller in the pool.
		/// - The pool is at `PoolState::Ongoing` or `PoolState::Retired`.
		#[transactional]
		#[pallet::weight(T::WeightInfo::redeem_all())]
		pub fn redeem_all(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			Self::redeem_inner(user, pid, None)
		}

		/// A selfless man intimately helps depositors of the pool to redeem their deposit,
		/// aaaaaaah, such a grateful!!
		///
		/// If the `account` is `Option::None`, the extrinsic will give "freedom" for a lucky man
		/// randomly;
		///
		/// If the `account` is specific and a depositor of the pool indeed, who will be given
		/// "freedom" by the extrinsic.
		///
		/// The condition to redeem:
		/// - The pool is at `PoolState::Retired`.
		#[transactional]
		#[pallet::weight(T::WeightInfo::volunteer_to_redeem())]
		pub fn volunteer_to_redeem(
			_origin: OriginFor<T>,
			pid: PoolId,
			account: Option<AccountIdOf<T>>,
		) -> DispatchResultWithPostInfo {
			let pool: PoolInfo<T> = Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire();

			ensure!(pool.state == PoolState::Retired, Error::<T>::InvalidPoolState);

			let user = match account {
				Some(account) => account,
				None => {
					let (account, _) = TotalDepositData::<T>::iter_prefix(pid)
						.next()
						.ok_or(Error::<T>::NoDepositOfUser)?;

					account
				},
			};

			Self::redeem_inner(user, pid, None)
		}

		/// Caller withdraw the unclaimed rewards owned by self from a pool.
		///
		/// __NOTE__: The extrinsic will retire the pool, which is reached the end of life.
		///
		/// The conditions to claim:
		/// - There is enough deposit owned by the caller in the pool.
		/// - The pool is at `PoolState::Ongoing`.
		#[transactional]
		#[pallet::weight(T::WeightInfo::claim())]
		pub fn claim(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let user = ensure_signed(origin)?;

			let mut pool: PoolInfo<T> =
				Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire().try_update();

			ensure!(pool.state == PoolState::Ongoing, Error::<T>::InvalidPoolState);

			let mut deposit_data: DepositData<T> =
				Self::user_deposit_data(pid, user.clone()).ok_or(Error::<T>::NoDepositOfUser)?;

			ensure!(pool.update_b != deposit_data.update_b, Error::<T>::TooShortBetweenTwoClaim);
			pool.try_settle_and_transfer(&mut deposit_data, user.clone())?;

			TotalPoolInfos::<T>::insert(pid, pool);
			TotalDepositData::<T>::insert(pid, user, deposit_data);

			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn create_pool(
			trading_pair: (CurrencyId, CurrencyId),
			main_reward: (CurrencyId, BalanceOf<T>),
			option_rewards: BoundedVec<(CurrencyId, BalanceOf<T>), T::MaximumOptionRewards>,
			r#type: PoolType,
			duration: BlockNumberFor<T>,
			min_deposit_to_start: BalanceOf<T>,
			after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
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

			// Construct the PoolInfo
			let pool_id = Self::next_pool_id();
			let keeper: AccountIdOf<T> = T::PalletId::get().into_sub_account(pool_id);
			let mining_pool = PoolInfo {
				pool_id,
				keeper: keeper.clone(),
				investor: None,
				trading_pair,
				duration,
				r#type,

				min_deposit_to_start,
				after_block_to_start,

				deposit: Zero::zero(),

				rewards,
				update_b: Zero::zero(),
				state: PoolState::UnCharged,
				block_startup: None,
				block_retired: None,
			};

			TotalPoolInfos::<T>::insert(pool_id, mining_pool);

			Self::deposit_event(Event::PoolCreated(pool_id, r#type, trading_pair, keeper));

			Ok(().into())
		}

		pub(crate) fn redeem_inner(
			user: AccountIdOf<T>,
			pid: PoolId,
			value: Option<BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			let mut pool: PoolInfo<T> =
				Self::pool(pid).ok_or(Error::<T>::InvalidPoolId)?.try_retire().try_update();

			ensure!(
				pool.state == PoolState::Ongoing || pool.state == PoolState::Retired,
				Error::<T>::InvalidPoolState
			);

			let mut deposit_data: DepositData<T> =
				Self::user_deposit_data(pid, user.clone()).ok_or(Error::<T>::NoDepositOfUser)?;

			if pool.update_b != deposit_data.update_b {
				pool.try_settle_and_transfer(&mut deposit_data, user.clone())?;
			}

			// Keep minimum deposit in pool when the pool is ongoing.
			let minimum_in_pool = match pool.state {
				PoolState::Ongoing => T::MinimumDepositOfUser::get(),
				PoolState::Retired => Zero::zero(),
				_ => return Err(Error::<T>::Unexpected.into()),
			};

			let pool_can_redeem = pool.deposit.saturating_sub(minimum_in_pool);
			let user_can_redeem = min(deposit_data.deposit, pool_can_redeem);

			let try_redeem = match value {
				Some(value) if pool.state == PoolState::Ongoing => value,
				Some(_) if pool.state == PoolState::Retired => user_can_redeem,
				None => user_can_redeem,
				_ => return Err(Error::<T>::Unexpected.into()),
			};

			ensure!(
				user_can_redeem >= try_redeem && user_can_redeem != Zero::zero(),
				Error::<T>::TooLowToRedeem
			);

			pool.deposit = pool.deposit.saturating_sub(try_redeem);
			deposit_data.deposit = deposit_data.deposit.saturating_sub(try_redeem);

			// To unlock the deposit
			match pool.r#type {
				PoolType::Mining => {
					let lpt = Self::convert_to_lptoken(pool.trading_pair)?;
					T::MultiCurrency::transfer(lpt, &pool.keeper, &user, try_redeem)
						.map_err(|_e| Error::<T>::NotEnoughToRedeem)?;
				},
				PoolType::Farming => {
					let (token_a, token_b) = pool.trading_pair;

					T::MultiCurrency::transfer(token_a, &pool.keeper, &user, try_redeem)
						.map_err(|_e| Error::<T>::NotEnoughToRedeem)?;
					T::MultiCurrency::transfer(token_b, &pool.keeper, &user, try_redeem)
						.map_err(|_e| Error::<T>::NotEnoughToRedeem)?;
				},
				PoolType::EBFarming => {},
			};

			if pool.state == PoolState::Retired && pool.deposit == Zero::zero() {
				pool.try_withdraw_remain()?;
				pool.state = PoolState::Dead;
			}

			let r#type = pool.r#type;
			let trading_pair = pool.trading_pair;

			match pool.deposit.saturated_into() {
				0u128 => TotalPoolInfos::<T>::remove(pid),
				_ => TotalPoolInfos::<T>::insert(pid, pool),
			}

			match deposit_data.deposit.saturated_into() {
				0u128 => TotalDepositData::<T>::remove(pid, user.clone()),
				_ => TotalDepositData::<T>::insert(pid, user.clone(), deposit_data),
			}

			Self::deposit_event(Event::UserRedeemed(pid, r#type, trading_pair, try_redeem, user));

			Ok(().into())
		}

		pub(crate) fn next_pool_id() -> PoolId {
			let next_pool_id = Self::pool_id();
			NextPoolId::<T>::mutate(|current| *current = current.saturating_add(1));
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

		pub fn rewards(
			who: AccountIdOf<T>,
			pid: PoolId,
		) -> Result<Vec<(CurrencyId, BalanceOf<T>)>, ()> {
			let pool: PoolInfo<T> = Self::pool(pid).ok_or(())?.try_retire().try_update();
			let deposit_data: DepositData<T> =
				Self::user_deposit_data(pid, who.clone()).ok_or(())?;

			let mut to_rewards = Vec::<(CurrencyId, BalanceOf<T>)>::new();

			if let Some(_block_startup) = pool.block_startup {
				for (rtoken, reward) in pool.rewards.iter() {
					let v_new = reward.gain_avg;
					if let Some(gain_avg) = deposit_data.gain_avgs.get(rtoken) {
						let v_old = *gain_avg;

						let user_deposit: u128 = deposit_data.deposit.saturated_into();
						let amount = BalanceOf::<T>::saturated_from(
							v_new.saturating_sub(v_old).saturating_mul_int(user_deposit),
						);

						to_rewards.push((*rtoken, amount));
					}
				}
			}

			Ok(to_rewards)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			// Check whether pool-activated is meet the startup condition
			for pid in Self::charged_pids() {
				if let Some(mut pool) = Self::pool(pid) {
					pool = pool.try_startup(n);

					if pool.state == PoolState::Ongoing {
						ChargedPoolIds::<T>::mutate(|pids| pids.remove(&pid));
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

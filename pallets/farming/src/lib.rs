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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod gauge;
pub mod rewards;
pub mod weights;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, AtLeast32BitUnsigned, Saturating, Zero},
		ArithmeticError, Perbill,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
pub use gauge::*;
use node_primitives::{CurrencyId, PoolId};
use orml_traits::MultiCurrency;
pub use pallet::*;
pub use rewards::*;
// use sp_arithmetic::per_things::Percent;
use sp_runtime::SaturatedConversion;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};
pub use weights::WeightInfo;
#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type ControlOrigin: EnsureOrigin<Self::Origin>;

		/// Set default weight.
		type WeightInfo: WeightInfo;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type Keeper: Get<PalletId>;

		#[pallet::constant]
		type RewardIssuer: Get<PalletId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FarmingPoolCreated {
			pid: PoolId,
		},
		FarmingPoolReset {
			pid: PoolId,
		},
		FarmingPoolClosed {
			pid: PoolId,
		},
		FarmingPoolKilled {
			pid: PoolId,
		},
		FarmingPoolEdited {
			pid: PoolId,
		},
		Charged {
			who: AccountIdOf<T>,
			pid: PoolId,
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		},
		Deposited {
			who: AccountIdOf<T>,
			pid: PoolId,
			add_value: BalanceOf<T>,
			gauge_info: Option<(BalanceOf<T>, BlockNumberFor<T>)>,
		},
		Withdrawn {
			who: AccountIdOf<T>,
			pid: PoolId,
			remove_value: Option<BalanceOf<T>>,
		},
		Claimed {
			who: AccountIdOf<T>,
			pid: PoolId,
		},
		GaugeWithdrawn {
			who: AccountIdOf<T>,
			gid: PoolId,
		},
		AllForceGaugeClaimed {
			gid: PoolId,
		},
		PartiallyForceGaugeClaimed {
			gid: PoolId,
		},
		AllRetired {
			pid: PoolId,
		},
		PartiallyRetired {
			pid: PoolId,
		},
		RetireLimitSet {
			limit: u32,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		CalculationOverflow,
		PoolDoesNotExist,
		GaugePoolNotExist,
		GaugeInfoNotExist,
		InvalidPoolState,
		LastGaugeNotClaim,
		/// claim_limit_time exceeded
		CanNotClaim,
		/// gauge pool max_block exceeded
		GaugeMaxBlockOverflow,
		/// withdraw_limit_time exceeded
		WithdrawLimitCountExceeded,
		ShareInfoNotExists,
	}

	#[pallet::storage]
	#[pallet::getter(fn pool_next_id)]
	pub type PoolNextId<T: Config> = StorageValue<_, PoolId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn gauge_pool_next_id)]
	pub type GaugePoolNextId<T: Config> = StorageValue<_, PoolId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn retire_limit)]
	pub type RetireLimit<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Record reward pool info.
	///
	/// map PoolId => PoolInfo
	#[pallet::storage]
	#[pallet::getter(fn pool_infos)]
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
	#[pallet::getter(fn gauge_pool_infos)]
	pub type GaugePoolInfos<T: Config> = StorageMap<
		_,
		Twox64Concat,
		PoolId,
		GaugePoolInfo<BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>, BlockNumberFor<T>>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn gauge_infos)]
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
	#[pallet::getter(fn shares_and_withdrawn_rewards)]
	pub type SharesAndWithdrawnRewards<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		PoolId,
		Twox64Concat,
		T::AccountId,
		ShareInfo<BalanceOf<T>, CurrencyIdOf<T>, BlockNumberFor<T>, AccountIdOf<T>>,
	>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			PoolInfos::<T>::iter().for_each(|(pid, mut pool_info)| match pool_info.state {
				PoolState::Ongoing => {
					pool_info.basic_rewards.clone().iter().for_each(
						|(reward_currency_id, reward_amount)| {
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
					if n >= pool_info.after_block_to_start ||
						pool_info.total_shares >= pool_info.min_deposit_to_start
					{
						pool_info.block_startup = Some(n);
						pool_info.state = PoolState::Ongoing;
					}
					PoolInfos::<T>::insert(pid, &pool_info);
				},
				_ => (),
			});

			GaugePoolInfos::<T>::iter().for_each(
				|(gid, mut gauge_pool_info)| match gauge_pool_info.gauge_state {
					GaugeState::Bonded => {
						if let Some(pool_info) = Self::pool_infos(&gauge_pool_info.pid) {
							pool_info.basic_rewards.clone().iter().for_each(
								|(reward_currency_id, reward_amount)| {
									gauge_pool_info
										.rewards
										.entry(*reward_currency_id)
										.and_modify(|(total_reward, _, _)| {
											*total_reward = total_reward.saturating_add(
												gauge_pool_info.coefficient * *reward_amount,
											);
										})
										.or_insert((*reward_amount, Zero::zero(), Zero::zero()));
								},
							);
							GaugePoolInfos::<T>::insert(gid, &gauge_pool_info);
						}
					},
					_ => (),
				},
			);

			T::WeightInfo::on_initialize()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		BlockNumberFor<T>: AtLeast32BitUnsigned + Copy,
		BalanceOf<T>: AtLeast32BitUnsigned + Copy,
	{
		#[transactional]
		#[pallet::weight(0)]
		pub fn create_farming_pool(
			origin: OriginFor<T>,
			tokens_proportion: Vec<(CurrencyIdOf<T>, Perbill)>,
			basic_rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
			gauge_init: Option<(CurrencyIdOf<T>, Perbill, BlockNumberFor<T>)>,
			min_deposit_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
			#[pallet::compact] withdraw_limit_time: BlockNumberFor<T>,
			#[pallet::compact] claim_limit_time: BlockNumberFor<T>,
			withdraw_limit_count: u8,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let pid = Self::pool_next_id();
			let keeper = T::Keeper::get().into_sub_account(pid);
			let reward_issuer = T::RewardIssuer::get().into_sub_account(pid);
			let tokens_proportion_map: BTreeMap<CurrencyIdOf<T>, Perbill> =
				tokens_proportion.into_iter().map(|(k, v)| (k, v)).collect();
			let basic_rewards_map: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> =
				basic_rewards.into_iter().map(|(k, v)| (k, v)).collect();

			let mut pool_info = PoolInfo::new(
				keeper,
				reward_issuer,
				tokens_proportion_map,
				basic_rewards_map,
				None,
				min_deposit_to_start,
				after_block_to_start,
				withdraw_limit_time,
				claim_limit_time,
				withdraw_limit_count,
			);

			if let Some((gauge_token, coefficient, max_block)) = gauge_init {
				Self::create_gauge_pool(pid, &mut pool_info, gauge_token, coefficient, max_block)?;
			};

			PoolInfos::<T>::insert(pid, &pool_info);
			PoolNextId::<T>::mutate(|id| -> DispatchResult {
				*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::FarmingPoolCreated { pid });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn charge(
			origin: OriginFor<T>,
			pid: PoolId,
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			let mut pool_info = Self::pool_infos(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(pool_info.state == PoolState::UnCharged, Error::<T>::InvalidPoolState);
			rewards.iter().try_for_each(|(reward_currency, reward)| -> DispatchResult {
				T::MultiCurrency::transfer(
					*reward_currency,
					&exchanger,
					&pool_info.reward_issuer,
					*reward,
				)
			})?;
			pool_info.state = PoolState::Charged;
			PoolInfos::<T>::insert(&pid, pool_info);

			Self::deposit_event(Event::Charged { who: exchanger, pid, rewards });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn deposit(
			origin: OriginFor<T>,
			pid: PoolId,
			add_value: BalanceOf<T>,
			gauge_info: Option<(BalanceOf<T>, BlockNumberFor<T>)>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let mut pool_info = Self::pool_infos(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(
				pool_info.state == PoolState::Ongoing || pool_info.state == PoolState::Charged,
				Error::<T>::InvalidPoolState
			);

			let tokens_proportion_values: Vec<Perbill> =
				pool_info.tokens_proportion.values().cloned().collect();
			let native_amount = tokens_proportion_values[0].saturating_reciprocal_mul(add_value);
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

			match gauge_info {
				Some((gauge_value, gauge_block)) => {
					Self::gauge_add(
						&exchanger,
						pool_info.gauge.ok_or(Error::<T>::GaugePoolNotExist)?,
						gauge_value,
						gauge_block,
					)?;
				},
				None => (),
			};

			Self::deposit_event(Event::Deposited { who: exchanger, pid, add_value, gauge_info });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn withdraw(
			origin: OriginFor<T>,
			pid: PoolId,
			remove_value: Option<BalanceOf<T>>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let pool_info = Self::pool_infos(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(
				pool_info.state == PoolState::Ongoing ||
					pool_info.state == PoolState::Charged ||
					pool_info.state == PoolState::Dead,
				Error::<T>::InvalidPoolState
			);
			let share_info = Self::shares_and_withdrawn_rewards(&pid, &exchanger)
				.ok_or(Error::<T>::ShareInfoNotExists)?;
			ensure!(
				share_info.withdraw_list.len() < pool_info.withdraw_limit_count.into(),
				Error::<T>::WithdrawLimitCountExceeded
			);

			Self::remove_share(&exchanger, pid, remove_value, pool_info.withdraw_limit_time)?;

			Self::deposit_event(Event::Withdrawn { who: exchanger, pid, remove_value });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn claim(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let pool_info = Self::pool_infos(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(
				pool_info.state == PoolState::Ongoing || pool_info.state == PoolState::Dead,
				Error::<T>::InvalidPoolState
			);

			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			let share_info = Self::shares_and_withdrawn_rewards(&pid, &exchanger)
				.ok_or(Error::<T>::ShareInfoNotExists)?;
			ensure!(
				share_info.claim_last_block + pool_info.claim_limit_time <= current_block_number,
				Error::<T>::CanNotClaim
			);

			Self::claim_rewards(&exchanger, pid)?;
			if let Some(ref gid) = pool_info.gauge {
				Self::gauge_claim_inner(&exchanger, *gid)?;
			}
			Self::process_withraw_list(&exchanger, pid, &pool_info)?;

			Self::deposit_event(Event::Claimed { who: exchanger, pid });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn force_retire_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut pool_info = Self::pool_infos(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
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
				Self::claim_rewards(&who, pid)?;
				if let Some(ref gid) = pool_info.gauge {
					Self::gauge_claim_inner(&who, *gid)?;
				}
				Self::process_withraw_list(&who, pid, &pool_info)?;
			}

			if all_retired {
				if let Some(ref gid) = pool_info.gauge {
					let mut gauge_pool_info =
						Self::gauge_pool_infos(gid).ok_or(Error::<T>::GaugePoolNotExist)?;
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

		#[pallet::weight(0)]
		pub fn set_retire_limit(origin: OriginFor<T>, limit: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			RetireLimit::<T>::mutate(|old_limit| {
				*old_limit = limit;
			});

			Self::deposit_event(Event::RetireLimitSet { limit });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn close_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut pool_info = Self::pool_infos(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(pool_info.state == PoolState::Ongoing, Error::<T>::InvalidPoolState);
			pool_info.state = PoolState::Dead;
			PoolInfos::<T>::insert(&pid, pool_info);

			Self::deposit_event(Event::FarmingPoolClosed { pid });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn reset_pool(
			origin: OriginFor<T>,
			pid: PoolId,
			basic_rewards: Option<Vec<(CurrencyIdOf<T>, BalanceOf<T>)>>,
			min_deposit_to_start: Option<BalanceOf<T>>,
			after_block_to_start: Option<BlockNumberFor<T>>,
			withdraw_limit_time: Option<BlockNumberFor<T>>,
			claim_limit_time: Option<BlockNumberFor<T>>,
			withdraw_limit_count: Option<u8>,
			gauge_init: Option<(CurrencyIdOf<T>, Perbill, BlockNumberFor<T>)>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut pool_info = Self::pool_infos(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(pool_info.state == PoolState::Retired, Error::<T>::InvalidPoolState);
			if let Some(basic_rewards) = basic_rewards {
				let basic_rewards_map: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> =
					basic_rewards.into_iter().map(|(k, v)| (k, v)).collect();
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
			if let Some((gauge_token, coefficient, max_block)) = gauge_init {
				Self::create_gauge_pool(pid, &mut pool_info, gauge_token, coefficient, max_block)?;
			};
			PoolInfos::<T>::insert(pid, &pool_info);

			Self::deposit_event(Event::FarmingPoolReset { pid });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn kill_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let pool_info = Self::pool_infos(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(pool_info.state == PoolState::Retired, Error::<T>::InvalidPoolState);
			SharesAndWithdrawnRewards::<T>::remove_prefix(pid, None);
			PoolInfos::<T>::remove(pid);

			Self::deposit_event(Event::FarmingPoolKilled { pid });
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn edit_pool(
			origin: OriginFor<T>,
			pid: PoolId,
			basic_rewards: Option<Vec<(CurrencyIdOf<T>, BalanceOf<T>)>>,
			withdraw_limit_time: Option<BlockNumberFor<T>>,
			claim_limit_time: Option<BlockNumberFor<T>>,
			gauge_coefficient: Option<Perbill>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut pool_info = Self::pool_infos(&pid).ok_or(Error::<T>::PoolDoesNotExist)?;
			ensure!(pool_info.state == PoolState::Retired, Error::<T>::InvalidPoolState);
			if let Some(basic_rewards) = basic_rewards {
				let basic_rewards_map: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>> =
					basic_rewards.into_iter().map(|(k, v)| (k, v)).collect();
				pool_info.basic_rewards = basic_rewards_map;
			};
			if let Some(withdraw_limit_time) = withdraw_limit_time {
				pool_info.withdraw_limit_time = withdraw_limit_time;
			};
			if let Some(claim_limit_time) = claim_limit_time {
				pool_info.claim_limit_time = claim_limit_time;
			};
			if let Some(coefficient) = gauge_coefficient {
				GaugePoolInfos::<T>::mutate(
					pool_info.gauge.ok_or(Error::<T>::GaugePoolNotExist)?,
					|gauge_pool_info_old| {
						if let Some(mut gauge_pool_info) = gauge_pool_info_old.take() {
							gauge_pool_info.coefficient = coefficient;
							*gauge_pool_info_old = Some(gauge_pool_info);
						}
					},
				);
			};
			PoolInfos::<T>::insert(pid, &pool_info);

			Self::deposit_event(Event::FarmingPoolEdited { pid });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn gauge_withdraw(origin: OriginFor<T>, gid: PoolId) -> DispatchResult {
			// Check origin
			let who = ensure_signed(origin)?;

			let mut gauge_pool_info =
				GaugePoolInfos::<T>::get(gid).ok_or(Error::<T>::GaugePoolNotExist)?;
			match gauge_pool_info.gauge_state {
				GaugeState::Bonded => {
					Self::gauge_claim_inner(&who, gid)?;
				},
				GaugeState::Unbond => {
					let current_block_number: BlockNumberFor<T> =
						frame_system::Pallet::<T>::block_number();
					GaugeInfos::<T>::mutate(gid, &who, |maybe_gauge_info| -> DispatchResult {
						if let Some(gauge_info) = maybe_gauge_info.take() {
							if gauge_info.gauge_stop_block <= current_block_number {
								T::MultiCurrency::transfer(
									gauge_pool_info.token,
									&gauge_pool_info.keeper,
									&who,
									gauge_info.gauge_amount,
								)?;
								gauge_pool_info.total_time_factor = gauge_pool_info
									.total_time_factor
									.checked_sub(gauge_info.total_time_factor)
									.ok_or(ArithmeticError::Overflow)?;
								GaugePoolInfos::<T>::insert(gid, gauge_pool_info);
							} else {
								*maybe_gauge_info = Some(gauge_info);
							};
						}
						Ok(())
					})?;
				},
			}

			Self::deposit_event(Event::GaugeWithdrawn { who, gid });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
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
				Self::gauge_claim_inner(&gauge_info.who, gid)?;
			}

			if all_retired {
				Self::deposit_event(Event::AllForceGaugeClaimed { gid });
			} else {
				Self::deposit_event(Event::PartiallyForceGaugeClaimed { gid });
			}
			Ok(())
		}
	}
}

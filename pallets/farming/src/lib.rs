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
pub mod primitives;
pub mod rewards;
pub mod weights;

use frame_support::{
	log,
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, AtLeast32BitUnsigned, CheckedSub, Saturating, Zero},
		ArithmeticError, FixedPointOperand,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
pub use gauge::*;
use node_primitives::{Balance, BlockNumber, CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
pub use pallet::*;
pub use primitives::{VstokenConversionExchangeFee, VstokenConversionExchangeRate};
pub use rewards::*;
// use sp_arithmetic::per_things::Percent;
use sp_std::{collections::btree_map::BTreeMap, fmt::Debug, vec::Vec};
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

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type VsbondAccount: Get<PalletId>;

		/// Set default weight.
		type WeightInfo: WeightInfo;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type PalletId: Get<PalletId>;
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
		Charged {
			who: AccountIdOf<T>,
			pid: PoolId,
			rewards: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
		},
		Deposited {
			who: AccountIdOf<T>,
			pid: PoolId,
			add_value: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
			gauge_info: Option<(BalanceOf<T>, BlockNumberFor<T>)>,
		},
		Withdrew {
			who: AccountIdOf<T>,
			pid: PoolId,
			remove_value: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
		},
		WithdrewAll {
			who: AccountIdOf<T>,
			pid: PoolId,
		},
		Claimed {
			who: AccountIdOf<T>,
			pid: PoolId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		NotSupportTokenType,
		CalculationOverflow,
		PoolDoesNotExist,
		PoolKeeperNotExist,
		InvalidPoolState,
		/// The keeper in the farming pool does not exist
		KeeperNotExist,
		GaugePoolNotExist,
		LastGaugeNotClaim,
		CanNotClaim,
	}

	#[pallet::storage]
	#[pallet::getter(fn pool_next_id)]
	pub type PoolNextId<T: Config> = StorageValue<_, PoolId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn gauge_pool_next_id)]
	pub type GaugePoolNextId<T: Config> = StorageValue<_, PoolId, ValueQuery>;

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
		ValueQuery,
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
		GaugePoolInfo<BalanceOf<T>, CurrencyIdOf<T>, BlockNumberFor<T>>,
		ValueQuery,
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
		ValueQuery,
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
		ValueQuery,
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
					let min_deposit_to_start_value: Vec<BalanceOf<T>> =
						pool_info.min_deposit_to_start.values().cloned().collect();

					if n >= pool_info.after_block_to_start ||
						pool_info.total_shares >= min_deposit_to_start_value[0]
					{
						pool_info.block_startup = Some(n);
						pool_info.state = PoolState::Ongoing;
					}
					PoolInfos::<T>::insert(pid, &pool_info);
				},
				_ => (),
			});

			T::WeightInfo::on_initialize()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		BlockNumberFor<T>: Into<u128> + Into<BalanceOf<T>>,
		BalanceOf<T>: Into<u128>,
	{
		#[transactional]
		#[pallet::weight(0)]
		pub fn create_farming_pool(
			origin: OriginFor<T>,
			tokens: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
			basic_rewards: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
			gauge_token: Option<CurrencyIdOf<T>>,
			// charge_account: AccountIdOf<T>,
			min_deposit_to_start: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
			#[pallet::compact] withdraw_limit_time: BlockNumberFor<T>,
			#[pallet::compact] claim_limit_time: BlockNumberFor<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let pid = Self::pool_next_id();
			let gid = Self::gauge_pool_next_id();
			let keeper = T::PalletId::get().into_sub_account(pid);
			let starting_token_values: Vec<BalanceOf<T>> = tokens.values().cloned().collect();

			let mut pool_info = PoolInfo::new(
				keeper,
				tokens,
				basic_rewards,
				starting_token_values,
				Some(gid),
				min_deposit_to_start,
				after_block_to_start,
				withdraw_limit_time,
				claim_limit_time,
			);

			match gauge_token {
				Some(gauge) => Self::create_gauge_pool(pid, &mut pool_info, gauge),
				None => Ok(()),
			};

			PoolInfos::<T>::insert(pid, &pool_info);
			PoolNextId::<T>::mutate(|id| -> DispatchResult {
				*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;
			GaugePoolNextId::<T>::mutate(|id| -> DispatchResult {
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
			rewards: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			let mut pool_info = Self::pool_infos(&pid);
			ensure!(pool_info.state == PoolState::UnCharged, Error::<T>::InvalidPoolState);
			match pool_info.keeper {
				None => return Err(Error::<T>::PoolKeeperNotExist.into()),
				Some(ref keeper) =>
					rewards.iter().try_for_each(|(reward_currency, reward)| -> DispatchResult {
						T::MultiCurrency::transfer(*reward_currency, &exchanger, &keeper, *reward)?;
						Ok(())
					})?,
			}
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
			add_value: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
			gauge_info: Option<(BalanceOf<T>, BlockNumberFor<T>)>,
			// gauge_block: BlockNumberFor<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let pool_info = Self::pool_infos(&pid);
			ensure!(
				pool_info.state == PoolState::Ongoing || pool_info.state == PoolState::Charged,
				Error::<T>::InvalidPoolState
			);

			let values: Vec<BalanceOf<T>> = add_value.values().cloned().collect();
			Self::add_share(&exchanger, pid, values[0], &add_value);

			match gauge_info {
				Some((gauge_value, gauge_block)) => {
					Self::gauge_add(
						&exchanger,
						pid,
						pool_info.gauge.ok_or(Error::<T>::GaugePoolNotExist)?,
						gauge_value,
						gauge_block,
					)?;
				},
				None => (),
			};

			// match add_value.values().0 {
			// 	None => return Err(Error::<T>::InvalidPoolState.into()),
			// 	Some(entry) => Self::add_share(&exchanger, pid, *entry.get()),
			// }

			Self::deposit_event(Event::Deposited { who: exchanger, pid, add_value, gauge_info });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn withdraw(
			origin: OriginFor<T>,
			pid: PoolId,
			remove_value: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let pool_info = Self::pool_infos(&pid);
			ensure!(
				pool_info.state == PoolState::Ongoing || pool_info.state == PoolState::Charged,
				Error::<T>::InvalidPoolState
			);

			let values: Vec<BalanceOf<T>> = remove_value.values().cloned().collect();
			Self::remove_share(&exchanger, pid, Some(values[0]))?;

			Self::deposit_event(Event::Withdrew { who: exchanger, pid, remove_value });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn claim(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let pool_info = Self::pool_infos(&pid);
			ensure!(pool_info.state == PoolState::Ongoing, Error::<T>::InvalidPoolState);

			Self::claim_rewards(&exchanger, pid)?;
			if let Some(ref gid) = pool_info.gauge {
				Self::gauge_claim(&exchanger, *gid)?;
			}

			Self::deposit_event(Event::Claimed { who: exchanger, pid });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn force_retire_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut pool_info = Self::pool_infos(&pid);
			ensure!(pool_info.state == PoolState::Dead, Error::<T>::InvalidPoolState);
			let keeper = pool_info.keeper.as_ref().ok_or(Error::<T>::GaugePoolNotExist)?;
			/// TODO: gauge
			SharesAndWithdrawnRewards::<T>::iter_prefix_values(pid).try_for_each(
				|share_info| -> DispatchResult {
					let who = share_info.who.ok_or(Error::<T>::GaugePoolNotExist)?;
					Self::claim_rewards(&who, pid)?;
					share_info.share_total.iter().try_for_each(
						|(currency, share)| -> DispatchResult {
							if !share.is_zero() {
								T::MultiCurrency::transfer(*currency, &keeper, &who, *share)?;
							}
							Ok(())
						},
					)?;
					Ok(())
				},
			)?;

			pool_info.state = PoolState::Retired;
			pool_info.gauge = None;
			if let Some(ref gid) = pool_info.gauge {
				let mut gauge_info = Self::gauge_pool_infos(gid);
				gauge_info.gauge_state = GaugeState::Unbond;
				GaugePoolInfos::<T>::insert(&gid, gauge_info);
			}
			PoolInfos::<T>::insert(&pid, pool_info);

			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn close_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut pool_info = Self::pool_infos(&pid);
			ensure!(pool_info.state == PoolState::Ongoing, Error::<T>::InvalidPoolState);
			pool_info.state = PoolState::Dead;
			PoolInfos::<T>::insert(&pid, pool_info);

			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn reset_pool(
			origin: OriginFor<T>,
			pid: PoolId,
			tokens: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
			basic_rewards: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
			gauge_token: Option<CurrencyIdOf<T>>,
			// charge_account: AccountIdOf<T>,
			min_deposit_to_start: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
			#[pallet::compact] withdraw_limit_time: BlockNumberFor<T>,
			#[pallet::compact] claim_limit_time: BlockNumberFor<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let pool_info_origin = Self::pool_infos(&pid);
			ensure!(pool_info_origin.state == PoolState::Dead, Error::<T>::InvalidPoolState);
			let gid = Self::gauge_pool_next_id();
			let keeper = T::PalletId::get().into_sub_account(pid);
			let starting_token_values: Vec<BalanceOf<T>> = tokens.values().cloned().collect();

			let mut pool_info = PoolInfo::new(
				keeper,
				tokens,
				basic_rewards,
				starting_token_values,
				Some(gid),
				min_deposit_to_start,
				after_block_to_start,
				withdraw_limit_time,
				claim_limit_time,
			);

			match gauge_token {
				Some(gauge) => Self::create_gauge_pool(pid, &mut pool_info, gauge),
				None => Ok(()),
			};

			PoolInfos::<T>::insert(pid, &pool_info);
			PoolNextId::<T>::mutate(|id| -> DispatchResult {
				*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;
			GaugePoolNextId::<T>::mutate(|id| -> DispatchResult {
				*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::FarmingPoolReset { pid });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn kill_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let pool_info = Self::pool_infos(&pid);
			ensure!(pool_info.state == PoolState::Retired, Error::<T>::InvalidPoolState);
			SharesAndWithdrawnRewards::<T>::remove_prefix(pid, None);
			PoolInfos::<T>::remove(pid);

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn edit_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn withdraw_all(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			let pool_info = Self::pool_infos(&pid);
			ensure!(
				pool_info.state == PoolState::Ongoing || pool_info.state == PoolState::Charged,
				Error::<T>::InvalidPoolState
			);

			Self::remove_share(&exchanger, pid, None)?;

			Self::deposit_event(Event::WithdrewAll { who: exchanger, pid });
			Ok(())
		}
	}
}

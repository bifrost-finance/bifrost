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

pub mod incentive;
pub mod traits;
pub mod weights;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, CheckedDiv, Saturating, UniqueSaturatedInto, Zero},
		ArithmeticError, DispatchError, SaturatedConversion,
	},
	traits::{tokens::WithdrawReasons, Currency, LockIdentifier, LockableCurrency, UnixTime},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
pub use incentive::*;
use node_primitives::{CurrencyId, Timestamp, TokenSymbol};
use orml_traits::{MultiCurrency, MultiLockableCurrency};
pub use pallet::*;
use sp_core::U256;
use sp_std::{collections::btree_map::BTreeMap, vec, vec::Vec};
use traits::VeMintingInterface;
pub use weights::WeightInfo;

pub const COLLATOR_LOCK_ID: LockIdentifier = *b"vemintin";
pub const BNC: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);

#[allow(type_alias_bounds)]
type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct VeConfig<Balance> {
	amount: Balance,
	min_mint: Balance,
	min_time: Timestamp,
	max_time: Timestamp,
	multiplier: u128,
	week: Timestamp,
	vote_weight_multiplier: Balance,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct LockedBalance<Balance> {
	amount: Balance,
	end: Timestamp,
}

// pub type Epoch = U256;

#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct Point<Balance, BlockNumber> {
	bias: Balance,  // i128
	slope: Balance, // dweight / dt
	ts: Timestamp,
	blk: BlockNumber, // block
	fxs_amt: Balance,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type Currency: Currency<Self::AccountId> + LockableCurrency<Self::AccountId>;

		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type VeMintingPalletId: Get<PalletId>;

		type UnixTime: UnixTime;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created {},
		Minted { addr: AccountIdOf<T>, value: BalanceOf<T>, end: Timestamp, timestamp: Timestamp },
		Supply { supply_before: BalanceOf<T>, supply: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		NotSupportProportion,
		CalculationOverflow,
		ExistentialDeposit,
		DistributionNotExist,
		Expired,
	}

	#[pallet::storage]
	#[pallet::getter(fn supply)]
	pub type Supply<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ve_configs)]
	pub type VeConfigs<T: Config> = StorageValue<_, VeConfig<BalanceOf<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn epoch)]
	pub type Epoch<T: Config> = StorageValue<_, U256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn locked)]
	pub type Locked<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, LockedBalance<BalanceOf<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn point_history)]
	pub type PointHistory<T: Config> =
		StorageMap<_, Twox64Concat, U256, Point<BalanceOf<T>, T::BlockNumber>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_point_history)]
	pub type UserPointHistory<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		U256,
		Point<BalanceOf<T>, T::BlockNumber>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn user_point_epoch)]
	pub type UserPointEpoch<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, U256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn slope_changes)]
	pub type SlopeChanges<T: Config> = StorageMap<_, Twox64Concat, Timestamp, i128, ValueQuery>;

	// Incentive
	#[pallet::storage]
	#[pallet::getter(fn incentive_configs)]
	pub type IncentiveConfigs<T: Config> =
		StorageValue<_, IncentiveConfig<CurrencyIdOf<T>, BalanceOf<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_reward_per_token_paid)]
	pub type UserRewardPerTokenPaid<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn rewards)]
	pub type Rewards<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::set_minimum_mint())]
		pub fn set_config(
			origin: OriginFor<T>,
			min_mint: Option<BalanceOf<T>>, // Minimum mint balance
			min_time: Option<Timestamp>,    // Minimum lockup time
			max_time: Option<Timestamp>,    // Maximum lockup time
			multiplier: Option<u128>,
			week: Option<Timestamp>,
			vote_weight_multiplier: Option<BalanceOf<T>>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut ve_config = Self::ve_configs();
			if let Some(min_mint) = min_mint {
				ve_config.min_mint = min_mint;
			};
			if let Some(min_time) = min_time {
				ve_config.min_time = min_time;
			};
			if let Some(max_time) = max_time {
				ve_config.max_time = max_time;
			};
			if let Some(multiplier) = multiplier {
				ve_config.multiplier = multiplier;
			};
			if let Some(week) = week {
				ve_config.week = week;
			};
			if let Some(vote_weight_multiplier) = vote_weight_multiplier {
				ve_config.vote_weight_multiplier = vote_weight_multiplier;
			};
			VeConfigs::<T>::set(ve_config);

			Self::deposit_event(Event::Created {});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::mint())]
		pub fn create_lock(
			origin: OriginFor<T>,
			value: BalanceOf<T>,
			unlock_time: Timestamp,
		) -> DispatchResult {
			let exchanger: AccountIdOf<T> = ensure_signed(origin)?;
			Self::_create_lock(&exchanger, value, unlock_time)?;
			Self::deposit_event(Event::Created {});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::mint())]
		pub fn increase_amount(origin: OriginFor<T>, value: BalanceOf<T>) -> DispatchResult {
			let exchanger: AccountIdOf<T> = ensure_signed(origin)?;
			Self::_increase_amount(&exchanger, value)?;
			Self::deposit_event(Event::Created {});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::mint())]
		pub fn increase_unlock_time(origin: OriginFor<T>, time: Timestamp) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::_increase_unlock_time(&exchanger, time)?;
			Self::deposit_event(Event::Created {});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::mint())]
		pub fn withdraw(origin: OriginFor<T>) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::_withdraw(&exchanger)?;
			Self::deposit_event(Event::Created {});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::set_minimum_mint())]
		pub fn notify_rewards(
			origin: OriginFor<T>,
			rewards_duration: Option<Timestamp>,
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if let Some(rewards_duration) = rewards_duration {
				let mut incentive_config = Self::incentive_configs();
				incentive_config.rewards_duration = rewards_duration;
				IncentiveConfigs::<T>::set(incentive_config);
			};

			Self::notify_reward_amount(rewards)?;
			Self::deposit_event(Event::Created {});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn _checkpoint(
			addr: &AccountIdOf<T>,
			old_locked: LockedBalance<BalanceOf<T>>,
			new_locked: LockedBalance<BalanceOf<T>>,
		) -> DispatchResult {
			let mut u_old = Point::<BalanceOf<T>, T::BlockNumber>::default();
			let mut u_new = Point::<BalanceOf<T>, T::BlockNumber>::default();
			let mut old_dslope: i128; //  0_i128;
			let mut new_dslope = 0_i128;
			let mut g_epoch: U256 = Self::epoch();
			let ve_config = Self::ve_configs();
			let current_block_number: T::BlockNumber =
				frame_system::Pallet::<T>::block_number().into(); // T::BlockNumber
			let current_timestamp: Timestamp = T::UnixTime::now().as_millis().saturated_into();

			if old_locked.end > current_timestamp && old_locked.amount > BalanceOf::<T>::zero() {
				u_old.slope = old_locked
					.amount
					.checked_div(&Self::ve_configs().max_time.saturated_into::<BalanceOf<T>>())
					.ok_or(Error::<T>::CalculationOverflow)?;
				u_old.bias = u_old
					.slope
					.saturating_mul((old_locked.end - current_timestamp).saturated_into());
			}
			if new_locked.end > current_timestamp && new_locked.amount > BalanceOf::<T>::zero() {
				u_new.slope = U256::from(new_locked.amount.saturated_into::<u128>())
					.checked_div(U256::from(Self::ve_configs().max_time.saturated_into::<u128>()))
					.unwrap_or_default()
					.as_u128()
					.unique_saturated_into();
				u_new.bias = u_new
					.slope
					.saturating_mul((new_locked.end - current_timestamp).saturated_into());
			}

			old_dslope = Self::slope_changes(old_locked.end);
			if new_locked.end != 0 {
				if new_locked.end == old_locked.end {
					new_dslope = old_dslope
				} else {
					new_dslope = Self::slope_changes(new_locked.end)
				}
			}

			let mut last_point: Point<BalanceOf<T>, T::BlockNumber> = Point {
				bias: Zero::zero(),
				slope: Zero::zero(),
				ts: current_timestamp,
				blk: current_block_number,
				fxs_amt: Zero::zero(),
			};
			if g_epoch > U256::zero() {
				last_point = Self::point_history(g_epoch);
			} else {
				last_point.fxs_amt = Self::balance_of(addr, None)?;
			}
			let mut last_checkpoint = last_point.ts;
			let initial_last_point = last_point;
			let mut block_slope: u128 = Zero::zero();
			if current_timestamp > last_point.ts {
				block_slope = ve_config.multiplier *
					(current_block_number - last_point.blk).saturated_into::<u128>() /
					(current_timestamp - last_point.ts).saturated_into::<u128>()
			}
			let mut t_i: Timestamp = (last_checkpoint / ve_config.week) * ve_config.week;
			for _i in 0..255 {
				t_i += ve_config.week;
				let mut d_slope = Zero::zero();
				if t_i > current_timestamp {
					t_i = current_timestamp
				} else {
					d_slope = Self::slope_changes(t_i)
				}
				last_point.bias = U256::from(last_point.bias.saturated_into::<u128>())
					.checked_sub(
						U256::from(last_point.slope.saturated_into::<u128>()).saturating_mul(
							U256::from((t_i - last_checkpoint).saturated_into::<u128>()),
						),
					)
					.unwrap_or_default()
					.as_u128()
					.unique_saturated_into();

				last_point.slope += (d_slope as u128).saturated_into();
				if last_point.bias < Zero::zero() {
					// This can happen
					last_point.bias = Zero::zero()
				}
				if last_point.slope < Zero::zero() {
					//This cannot happen - just in case
					last_point.slope = Zero::zero()
				}
				last_checkpoint = t_i;
				last_point.ts = t_i;
				last_point.blk = initial_last_point.blk +
					(block_slope.saturating_mul(
						(t_i - initial_last_point.ts)
							.try_into()
							.map_err(|_| ArithmeticError::Overflow)?,
					) / ve_config.multiplier)
						.try_into()
						.map_err(|_| ArithmeticError::Overflow)?;
				g_epoch += U256::one();

				// Fill for the current block, if applicable
				if t_i == current_timestamp {
					last_point.blk = current_block_number;
					last_point.fxs_amt = Self::balance_of(addr, None)?;
					break;
				} else {
					PointHistory::<T>::insert(g_epoch, last_point);
				}
			}
			Epoch::<T>::set(g_epoch);

			last_point.slope += u_new.slope - u_old.slope;
			last_point.bias += u_new.bias - u_old.bias;
			if last_point.slope < Zero::zero() {
				last_point.slope = Zero::zero()
			}
			if last_point.bias < Zero::zero() {
				last_point.bias = Zero::zero()
			}
			PointHistory::<T>::insert(g_epoch, last_point);

			if old_locked.end > current_timestamp {
				// old_dslope was <something> - u_old.slope, so we cancel that
				old_dslope += u_old.slope.saturated_into::<u128>() as i128;
				if new_locked.end == old_locked.end {
					old_dslope -= u_new.slope.saturated_into::<u128>() as i128;
				} // It was a new deposit, not extension
				SlopeChanges::<T>::insert(old_locked.end, old_dslope);
			}

			if new_locked.end > current_timestamp {
				if new_locked.end > old_locked.end {
					new_dslope = new_dslope
						.checked_sub(u_new.slope.saturated_into::<u128>() as i128)
						.ok_or(ArithmeticError::Overflow)?;
					SlopeChanges::<T>::insert(new_locked.end, new_dslope);
				}
				// else: we recorded it already in old_dslope
			}

			// Now handle user history
			let user_epoch = Self::user_point_epoch(addr) + U256::one();
			UserPointEpoch::<T>::insert(addr, user_epoch);
			u_new.ts = current_timestamp;
			u_new.blk = current_block_number;
			u_new.fxs_amt = Self::locked(addr).amount;

			UserPointHistory::<T>::insert(addr, user_epoch, u_new);
			Self::update_reward(Some(addr))?;

			Ok(())
		}

		pub fn _deposit_for(
			addr: &AccountIdOf<T>,
			value: BalanceOf<T>,
			unlock_time: Timestamp,
			locked_balance: LockedBalance<BalanceOf<T>>,
		) -> DispatchResult {
			let ve_config = Self::ve_configs();
			ensure!(value >= ve_config.min_mint, Error::<T>::Expired);

			let current_timestamp: Timestamp = T::UnixTime::now().as_millis().saturated_into();

			let mut _locked = locked_balance;
			let supply_before = Self::supply();
			Supply::<T>::set(supply_before + value);

			let old_locked = _locked.clone();
			_locked.amount += value;
			if unlock_time != 0 {
				_locked.end = unlock_time
			}
			Locked::<T>::insert(addr, _locked.clone());

			Self::_checkpoint(addr, old_locked, _locked.clone())?;

			if value != BalanceOf::<T>::zero() {
				T::MultiCurrency::extend_lock(COLLATOR_LOCK_ID, BNC, addr, value)?;
			}

			Self::deposit_event(Event::Minted {
				addr: addr.clone(),
				value,
				end: _locked.end,
				timestamp: current_timestamp,
			});
			Self::deposit_event(Event::Supply { supply_before, supply: supply_before + value });
			Ok(())
		}
	}
}

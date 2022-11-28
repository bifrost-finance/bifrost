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

pub mod weights;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{
			AccountIdConversion, CheckedAdd, CheckedSub, Saturating, UniqueSaturatedInto, Zero,
		},
		ArithmeticError, Perbill, SaturatedConversion,
	},
	traits::{tokens::WithdrawReasons, Currency, LockIdentifier, LockableCurrency},
	PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{AccountId, CurrencyId, Timestamp}; // BlockNumber, Balance
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_core::U256;
// use sp_std::vec::Vec;
pub use weights::WeightInfo;

pub const COLLATOR_LOCK_ID: LockIdentifier = *b"vemintin";

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
// type BalanceOf<T> =
// 	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct VeConfig<Balance> {
	amount: Balance,
	max_time: Balance,
	MULTIPLIER: u128,
	WEEK: Timestamp,
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
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
		type Currency: Currency<Self::AccountId>
			// + ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;

		type ControlOrigin: EnsureOrigin<Self::Origin>;

		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type VeMintingPalletId: Get<PalletId>;

		// #[pallet::constant]
		// type MULTIPLIER: u128;

		// #[pallet::constant]
		// type WEEK: Timestamp;
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
		NotExpire,
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
		StorageMap<_, Twox64Concat, U256, Point<BalanceOf<T>, BlockNumberFor<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_point_history)]
	pub type UserPointHistory<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		U256,
		Point<BalanceOf<T>, BlockNumberFor<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn user_point_epoch)]
	pub type UserPointEpoch<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, U256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn slope_changes)]
	pub type SlopeChanges<T: Config> = StorageMap<_, Twox64Concat, Timestamp, i128, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(_bn: BlockNumberFor<T>, _remaining_weight: Weight) -> Weight {
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::set_minimum_mint())]
		pub fn set_config(
			origin: OriginFor<T>,
			// min_mint: BalanceOf<T>,             // 最小铸造值
			// min_lock_period: BlockNumberFor<T>, // 最小锁仓期
			max_time: Option<BalanceOf<T>>, // 最大锁仓期
			MULTIPLIER: Option<u128>,
			WEEK: Option<Timestamp>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut ve_config = Self::ve_configs();
			if let Some(max_time) = max_time {
				ve_config.max_time = max_time;
			};
			if let Some(MULTIPLIER) = MULTIPLIER {
				ve_config.MULTIPLIER = MULTIPLIER;
			};
			if let Some(WEEK) = WEEK {
				ve_config.WEEK = WEEK;
			};

			VeConfigs::<T>::set(ve_config);

			Self::deposit_event(Event::Created {});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::set_minimum_mint())]
		pub fn create_distribution(origin: OriginFor<T>) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

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
			let mut u_old = Point::<BalanceOf<T>, BlockNumberFor<T>>::default();
			let mut u_new = Point::<BalanceOf<T>, BlockNumberFor<T>>::default();
			let mut old_dslope = 0_i128;
			let mut new_dslope = 0_i128;
			let mut g_epoch: U256 = Self::epoch();
			let ve_config = Self::ve_configs();
			let current_block_number: BlockNumberFor<T> =
				frame_system::Pallet::<T>::block_number().into(); // BlockNumberFor<T>
			let current_timestamp: Timestamp =
				sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();

			if old_locked.end > current_timestamp && old_locked.amount > BalanceOf::<T>::zero() {
				u_old.slope = old_locked.amount / Self::ve_configs().max_time;
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
				// u_new.slope = new_locked.amount / Self::ve_configs().max_time;
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

			let mut last_point: Point<BalanceOf<T>, BlockNumberFor<T>> = Point {
				bias: Zero::zero(),
				slope: Zero::zero(),
				ts: current_timestamp,
				blk: current_block_number,
				fxs_amt: Zero::zero(),
			};
			if g_epoch > U256::zero() {
				last_point = Self::point_history(g_epoch);
				// } else {
				// 	last_point.fxs_amt = Self::balanceOf(addr)
			}
			let mut last_checkpoint = last_point.ts;
			let initial_last_point = last_point.clone();
			let mut block_slope: u128 = Zero::zero();
			if current_timestamp > last_point.ts {
				block_slope = ve_config.MULTIPLIER *
					(current_block_number - last_point.blk).saturated_into::<u128>() /
					(current_timestamp - last_point.ts).saturated_into::<u128>()
			}
			let mut t_i: Timestamp = (last_checkpoint / ve_config.WEEK) * ve_config.WEEK;
			for i in 0..255 {
				t_i += ve_config.WEEK;
				let mut d_slope = Zero::zero();
				if t_i > current_timestamp {
					t_i = current_timestamp
				} else {
					d_slope = Self::slope_changes(t_i)
				}
				last_point.bias = U256::from(last_point.bias.saturated_into::<u128>())
					.checked_sub(
						U256::from(last_point.slope.saturated_into::<u128>()).saturating_mul(
							U256::from((t_i - last_point.ts).saturated_into::<u128>()),
						),
					)
					// .checked_div(total_shares)
					.unwrap_or_default()
					.as_u128()
					.unique_saturated_into();

				// last_point.bias -=
				// 	last_point.slope.saturating_mul((t_i - last_point.ts).saturated_into());
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
					) / ve_config.MULTIPLIER)
						.try_into()
						.map_err(|_| ArithmeticError::Overflow)?;
				g_epoch += U256::one();

				// Fill for the current block, if applicable
				if t_i == current_timestamp {
					last_point.blk = current_block_number;
					// last_point.fxs_amt = ERC20(self.token).balanceOf(self);
					break;
				} else {
					PointHistory::<T>::insert(g_epoch, last_point);
					// Self::point_history(g_epoch) = last_point
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
					old_dslope -= u_new.slope.clone().saturated_into::<u128>() as i128;
				} // It was a new deposit, not extension
				SlopeChanges::<T>::insert(old_locked.end, old_dslope);
			}

			if new_locked.end > current_timestamp {
				if new_locked.end > old_locked.end {
					new_dslope = new_dslope
						.checked_sub(u_new.slope.saturated_into::<u128>() as i128)
						.ok_or(ArithmeticError::Overflow)?;
					// new_dslope -= u_new.slope; old slope disappeared at this point
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

			Ok(())
		}

		pub fn _deposit_for(
			addr: &AccountIdOf<T>,
			value: BalanceOf<T>,
			unlock_time: Timestamp,
			locked_balance: LockedBalance<BalanceOf<T>>,
		) -> DispatchResult {
			let current_timestamp: Timestamp =
				sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();

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
				T::Currency::extend_lock(COLLATOR_LOCK_ID, addr, value, WithdrawReasons::all());
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

		pub fn withdraw(addr: &AccountIdOf<T>) -> DispatchResult {
			let mut _locked = Self::locked(addr);
			let current_timestamp: Timestamp =
				sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
			ensure!(current_timestamp > _locked.end, Error::<T>::NotExpire);
			let value = _locked.amount;
			let old_locked: LockedBalance<BalanceOf<T>> = _locked.clone();
			_locked.end = Zero::zero();
			_locked.amount = Zero::zero();
			Locked::<T>::insert(addr, _locked.clone());

			let supply_before = Self::supply();
			Supply::<T>::set(supply_before - value);

			Self::_checkpoint(addr, old_locked, _locked.clone())?;

			// TODO: set_lock
			T::Currency::set_lock(COLLATOR_LOCK_ID, addr, Zero::zero(), WithdrawReasons::all());

			Self::deposit_event(Event::Supply { supply_before, supply: supply_before - value });
			Ok(())
		}

		pub fn balanceOf(addr: &AccountIdOf<T>, _t: Timestamp) -> BalanceOf<T> {
			let u_epoch = Self::user_point_epoch(addr);
			if u_epoch == U256::zero() {
				return Zero::zero();
			} else {
				let mut last_point: Point<BalanceOf<T>, BlockNumberFor<T>> =
					Self::user_point_history(addr, u_epoch);
				last_point.bias -=
					last_point.slope.saturating_mul((_t - last_point.ts).saturated_into());
				// .ok_or(ArithmeticError::Overflow)?;
				if last_point.bias < Zero::zero() {
					last_point.bias = Zero::zero();
				}
				last_point.bias
			}
		}
	}
}

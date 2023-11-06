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

use crate::traits::Incentive;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{
			AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert,
			Saturating, UniqueSaturatedInto, Zero,
		},
		ArithmeticError, DispatchError, SaturatedConversion,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
pub use incentive::*;
use node_primitives::CurrencyId;
use orml_traits::{MultiCurrency, MultiLockableCurrency};
pub use pallet::*;
use sp_core::U256;
use sp_std::{borrow::ToOwned, collections::btree_map::BTreeMap, vec, vec::Vec};
pub use traits::VeMintingInterface;
pub use weights::WeightInfo;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct VeConfig<Balance, BlockNumber> {
	amount: Balance,
	min_mint: Balance,
	min_block: BlockNumber,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct LockedBalance<Balance, BlockNumber> {
	amount: Balance,
	end: BlockNumber,
}

#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct Point<Balance, BlockNumber> {
	bias: i128,  // i128
	slope: i128, // dweight / dt
	block: BlockNumber,
	amount: Balance,
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

		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type TokenType: Get<CurrencyId>;

		#[pallet::constant]
		type VeMintingPalletId: Get<PalletId>;

		#[pallet::constant]
		type IncentivePalletId: Get<PalletId>;

		/// Convert the block number into a balance.
		type BlockNumberToBalance: Convert<Self::BlockNumber, BalanceOf<Self>>;

		#[pallet::constant]
		type Week: Get<Self::BlockNumber>;

		#[pallet::constant]
		type MaxBlock: Get<Self::BlockNumber>;

		#[pallet::constant]
		type Multiplier: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type VoteWeightMultiplier: Get<BalanceOf<Self>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ConfigSet {
			config: VeConfig<BalanceOf<T>, T::BlockNumber>,
		},
		Minted {
			addr: AccountIdOf<T>,
			value: BalanceOf<T>,
			end: T::BlockNumber,
			now: T::BlockNumber,
		},
		Supply {
			supply_before: BalanceOf<T>,
			supply: BalanceOf<T>,
		},
		LockCreated {
			addr: AccountIdOf<T>,
			value: BalanceOf<T>,
			unlock_time: T::BlockNumber,
		},
		UnlockTimeIncreased {
			addr: AccountIdOf<T>,
			unlock_time: T::BlockNumber,
		},
		AmountIncreased {
			addr: AccountIdOf<T>,
			value: BalanceOf<T>,
		},
		Withdrawn {
			addr: AccountIdOf<T>,
			value: BalanceOf<T>,
		},
		IncentiveSet {
			rewards_duration: T::BlockNumber,
		},
		RewardAdded {
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		},
		Rewarded {
			addr: AccountIdOf<T>,
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		Expired,
		BelowMinimumMint,
		LockNotExist,
		LockExist,
		NoRewards,
	}

	#[pallet::storage]
	#[pallet::getter(fn supply)]
	pub type Supply<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ve_configs)]
	pub type VeConfigs<T: Config> =
		StorageValue<_, VeConfig<BalanceOf<T>, T::BlockNumber>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn epoch)]
	pub type Epoch<T: Config> = StorageValue<_, U256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn locked)]
	pub type Locked<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		LockedBalance<BalanceOf<T>, T::BlockNumber>,
		ValueQuery,
	>;

	// Each week has a Point struct stored in PointHistory.
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
	pub type SlopeChanges<T: Config> =
		StorageMap<_, Twox64Concat, T::BlockNumber, i128, ValueQuery>;

	// Incentive
	#[pallet::storage]
	#[pallet::getter(fn incentive_configs)]
	pub type IncentiveConfigs<T: Config> =
		StorageValue<_, IncentiveConfig<CurrencyIdOf<T>, BalanceOf<T>, T::BlockNumber>, ValueQuery>;

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
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_config())]
		pub fn set_config(
			origin: OriginFor<T>,
			min_mint: Option<BalanceOf<T>>,    // Minimum mint balance
			min_block: Option<T::BlockNumber>, // Minimum lockup time
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut ve_config = Self::ve_configs();
			if let Some(min_mint) = min_mint {
				ve_config.min_mint = min_mint;
			};
			if let Some(min_block) = min_block {
				ve_config.min_block = min_block;
			};
			VeConfigs::<T>::set(ve_config.clone());

			Self::deposit_event(Event::ConfigSet { config: ve_config });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::create_lock())]
		pub fn create_lock(
			origin: OriginFor<T>,
			value: BalanceOf<T>,
			unlock_time: T::BlockNumber,
		) -> DispatchResult {
			let exchanger: AccountIdOf<T> = ensure_signed(origin)?;
			Self::create_lock_inner(&exchanger, value, unlock_time)
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::increase_amount())]
		pub fn increase_amount(origin: OriginFor<T>, value: BalanceOf<T>) -> DispatchResult {
			let exchanger: AccountIdOf<T> = ensure_signed(origin)?;
			Self::increase_amount_inner(&exchanger, value)
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::increase_unlock_time())]
		pub fn increase_unlock_time(origin: OriginFor<T>, time: T::BlockNumber) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::increase_unlock_time_inner(&exchanger, time)
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn withdraw(origin: OriginFor<T>) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::withdraw_inner(&exchanger)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::notify_rewards())]
		pub fn notify_rewards(
			origin: OriginFor<T>,
			incentive_from: AccountIdOf<T>,
			rewards_duration: Option<T::BlockNumber>,
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::set_incentive(rewards_duration);
			Self::notify_reward_amount(&incentive_from, rewards)
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::get_rewards())]
		pub fn get_rewards(origin: OriginFor<T>) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::get_rewards_inner(&exchanger)
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn _checkpoint(
			addr: &AccountIdOf<T>,
			old_locked: LockedBalance<BalanceOf<T>, T::BlockNumber>,
			new_locked: LockedBalance<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			Self::update_reward(Some(addr))?;

			let mut u_old = Point::<BalanceOf<T>, T::BlockNumber>::default();
			let mut u_new = Point::<BalanceOf<T>, T::BlockNumber>::default();
			let mut new_dslope = 0_i128;
			let mut g_epoch: U256 = Self::epoch();
			let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number();

			if old_locked.end > current_block_number && old_locked.amount > BalanceOf::<T>::zero() {
				u_old.slope = U256::from(old_locked.amount.saturated_into::<u128>())
					.checked_div(U256::from(T::MaxBlock::get().saturated_into::<u128>()))
					.unwrap_or_default()
					.as_u128()
					.unique_saturated_into();
				u_old.bias = u_old
					.slope
					.checked_mul(
						(old_locked.end.saturated_into::<u128>() as i128) -
							(current_block_number.saturated_into::<u128>() as i128),
					)
					.ok_or(ArithmeticError::Overflow)?;
			}
			if new_locked.end > current_block_number && new_locked.amount > BalanceOf::<T>::zero() {
				u_new.slope = U256::from(new_locked.amount.saturated_into::<u128>())
					.checked_div(U256::from(T::MaxBlock::get().saturated_into::<u128>()))
					.unwrap_or_default()
					.as_u128()
					.unique_saturated_into();
				u_new.bias = u_new
					.slope
					.checked_mul(
						(new_locked.end.saturated_into::<u128>() as i128) -
							(current_block_number.saturated_into::<u128>() as i128),
					)
					.ok_or(ArithmeticError::Overflow)?;
			}
			let mut old_dslope = Self::slope_changes(old_locked.end);
			if new_locked.end != Zero::zero() {
				if new_locked.end == old_locked.end {
					new_dslope = old_dslope
				} else {
					new_dslope = Self::slope_changes(new_locked.end)
				}
			}

			let mut last_point: Point<BalanceOf<T>, T::BlockNumber> = Point {
				bias: 0_i128,
				slope: 0_i128,
				block: current_block_number,
				amount: Zero::zero(),
			};
			if g_epoch > U256::zero() {
				last_point = Self::point_history(g_epoch);
			} else {
				last_point.amount = T::MultiCurrency::free_balance(
					T::TokenType::get(),
					&T::VeMintingPalletId::get().into_account_truncating(),
				);
			}
			let mut last_checkpoint = last_point.block;
			let mut t_i: T::BlockNumber = last_checkpoint
				.checked_div(&T::Week::get())
				.ok_or(ArithmeticError::Overflow)?
				.checked_mul(&T::Week::get())
				.ok_or(ArithmeticError::Overflow)?;
			for _i in 0..255 {
				t_i = t_i.checked_add(&T::Week::get()).ok_or(ArithmeticError::Overflow)?;
				let mut d_slope = Zero::zero();
				if t_i > current_block_number {
					t_i = current_block_number
				} else {
					d_slope = Self::slope_changes(t_i)
				}
				last_point.bias = last_point
					.bias
					.checked_sub(
						last_point
							.slope
							.checked_mul(
								t_i.checked_sub(&last_checkpoint)
									.ok_or(ArithmeticError::Overflow)?
									.saturated_into::<u128>()
									.unique_saturated_into(),
							)
							.ok_or(ArithmeticError::Overflow)?,
					)
					.ok_or(ArithmeticError::Overflow)?;

				last_point.slope =
					last_point.slope.checked_add(d_slope).ok_or(ArithmeticError::Overflow)?;
				if last_point.slope < 0_i128 {
					//This cannot happen - just in case
					last_point.slope = 0_i128
				}
				if last_point.bias < 0_i128 {
					// This can happen
					last_point.bias = 0_i128
				}

				last_checkpoint = t_i;
				last_point.block = t_i;
				g_epoch = g_epoch.checked_add(U256::one()).ok_or(ArithmeticError::Overflow)?;

				// Fill for the current block, if applicable
				if t_i == current_block_number {
					last_point.amount = T::MultiCurrency::free_balance(
						T::TokenType::get(),
						&T::VeMintingPalletId::get().into_account_truncating(),
					);
					break;
				} else {
					PointHistory::<T>::insert(g_epoch, last_point);
				}
			}
			Epoch::<T>::set(g_epoch);

			last_point.slope = u_new
				.slope
				.checked_add(last_point.slope)
				.ok_or(ArithmeticError::Overflow)?
				.checked_sub(u_old.slope)
				.ok_or(ArithmeticError::Overflow)?;
			last_point.bias = last_point
				.bias
				.checked_add(u_new.bias)
				.ok_or(ArithmeticError::Overflow)?
				.checked_sub(u_old.bias)
				.ok_or(ArithmeticError::Overflow)?;
			if last_point.slope < 0_i128 {
				last_point.slope = 0_i128
			}
			if last_point.bias < 0_i128 {
				last_point.bias = 0_i128
			}
			PointHistory::<T>::insert(g_epoch, last_point);

			if old_locked.end > current_block_number {
				// old_dslope was <something> - u_old.slope, so we cancel that
				old_dslope =
					old_dslope.checked_add(u_old.slope).ok_or(ArithmeticError::Overflow)?;
				if new_locked.end == old_locked.end {
					old_dslope =
						old_dslope.checked_sub(u_new.slope).ok_or(ArithmeticError::Overflow)?;
				} // It was a new deposit, not extension
				SlopeChanges::<T>::insert(old_locked.end, old_dslope);
			}

			if new_locked.end > current_block_number {
				if new_locked.end > old_locked.end {
					new_dslope =
						new_dslope.checked_sub(u_new.slope).ok_or(ArithmeticError::Overflow)?;
					SlopeChanges::<T>::insert(new_locked.end, new_dslope);
				}
				// else: we recorded it already in old_dslope
			}

			// Now handle user history
			let user_epoch = Self::user_point_epoch(addr)
				.checked_add(U256::one())
				.ok_or(ArithmeticError::Overflow)?;
			UserPointEpoch::<T>::insert(addr, user_epoch);
			u_new.block = current_block_number;
			u_new.amount = Self::locked(addr).amount;
			UserPointHistory::<T>::insert(addr, user_epoch, u_new);

			Ok(())
		}

		pub fn _deposit_for(
			addr: &AccountIdOf<T>,
			value: BalanceOf<T>,
			unlock_time: T::BlockNumber,
			locked_balance: LockedBalance<BalanceOf<T>, T::BlockNumber>,
		) -> DispatchResult {
			let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number();
			let mut _locked = locked_balance;
			let supply_before = Self::supply();
			Supply::<T>::set(supply_before.checked_add(&value).ok_or(ArithmeticError::Overflow)?);

			let old_locked = _locked.clone();
			_locked.amount = _locked.amount.checked_add(&value).ok_or(ArithmeticError::Overflow)?;
			if unlock_time != Zero::zero() {
				_locked.end = unlock_time
			}
			Locked::<T>::insert(addr, _locked.clone());

			if value != BalanceOf::<T>::zero() {
				T::MultiCurrency::transfer(
					T::TokenType::get(),
					addr,
					&T::VeMintingPalletId::get().into_account_truncating(),
					value,
				)?;
			}
			Self::_checkpoint(addr, old_locked, _locked.clone())?;

			Self::deposit_event(Event::Minted {
				addr: addr.clone(),
				value,
				end: _locked.end,
				now: current_block_number,
			});
			Self::deposit_event(Event::Supply {
				supply_before,
				supply: supply_before.checked_add(&value).ok_or(ArithmeticError::Overflow)?,
			});
			Ok(())
		}

		// Get the current voting power for `addr`
		pub(crate) fn balance_of_current_block(
			addr: &AccountIdOf<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number();
			let u_epoch = Self::user_point_epoch(addr);
			if u_epoch == U256::zero() {
				return Ok(Zero::zero());
			} else {
				let mut last_point: Point<BalanceOf<T>, T::BlockNumber> =
					Self::user_point_history(addr, u_epoch);

				last_point.bias = last_point
					.bias
					.checked_sub(
						last_point
							.slope
							.checked_mul(
								(current_block_number.saturated_into::<u128>() as i128)
									.checked_sub(last_point.block.saturated_into::<u128>() as i128)
									.ok_or(ArithmeticError::Overflow)?,
							)
							.ok_or(ArithmeticError::Overflow)?,
					)
					.ok_or(ArithmeticError::Overflow)?;

				if last_point.bias < 0_i128 {
					last_point.bias = 0_i128
				}

				Ok(last_point
					.amount
					.checked_add(
						&T::VoteWeightMultiplier::get()
							.checked_mul(&(last_point.bias as u128).unique_saturated_into())
							.ok_or(ArithmeticError::Overflow)?,
					)
					.ok_or(ArithmeticError::Overflow)?)
			}
		}

		// Measure voting power of `addr` at block height `block`
		pub(crate) fn balance_of_at(
			addr: &AccountIdOf<T>,
			block: T::BlockNumber,
		) -> Result<BalanceOf<T>, DispatchError> {
			let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number();
			ensure!(block <= current_block_number, Error::<T>::Expired);

			// Binary search
			let mut _min = U256::zero();
			let mut _max = Self::user_point_epoch(addr);
			for _i in 0..128 {
				if _min >= _max {
					break;
				}
				let _mid = (_min
					.checked_add(_max)
					.ok_or(ArithmeticError::Overflow)?
					.checked_add(U256::one())
					.ok_or(ArithmeticError::Overflow)?)
				.checked_div(U256::from(2_u128))
				.ok_or(ArithmeticError::Overflow)?;

				if Self::user_point_history(addr, _mid).block <= block {
					_min = _mid
				} else {
					_max = _mid.checked_sub(U256::one()).ok_or(ArithmeticError::Overflow)?
				}
			}

			let mut upoint: Point<BalanceOf<T>, T::BlockNumber> =
				Self::user_point_history(addr, _min);
			upoint.bias = upoint
				.bias
				.checked_sub(
					upoint
						.slope
						.checked_mul(
							(block.saturated_into::<u128>() as i128)
								.checked_sub(upoint.block.saturated_into::<u128>() as i128)
								.ok_or(ArithmeticError::Overflow)?,
						)
						.ok_or(ArithmeticError::Overflow)?,
				)
				.ok_or(ArithmeticError::Overflow)?;

			if upoint.bias < 0_i128 {
				upoint.bias = 0_i128
			}
			Ok(upoint
				.amount
				.checked_add(
					&T::VoteWeightMultiplier::get()
						.checked_mul(&(upoint.bias as u128).unique_saturated_into())
						.ok_or(ArithmeticError::Overflow)?,
				)
				.ok_or(ArithmeticError::Overflow)?)
		}
	}
}

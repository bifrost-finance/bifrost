// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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
use bifrost_primitives::{Balance, CurrencyId};
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{
			AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert,
			Saturating, UniqueSaturatedInto, Zero,
		},
		ArithmeticError, DispatchError, FixedU128, SaturatedConversion,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
pub use incentive::*;
use orml_traits::{LockIdentifier, MultiCurrency, MultiLockableCurrency};
pub use pallet::*;
use sp_core::{U256, U512};
use sp_std::{borrow::ToOwned, cmp::Ordering, collections::btree_map::BTreeMap, vec, vec::Vec};
pub use traits::{
	LockedToken, MarkupCoefficientInfo, MarkupInfo, UserMarkupInfo, VeMintingInterface,
};
pub use weights::WeightInfo;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

const VE_LOCK_ID: LockIdentifier = *b"vebnclck";
const MARKUP_LOCK_ID: LockIdentifier = *b"vebncmkp";

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

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId, Balance = Balance>
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
		type BlockNumberToBalance: Convert<BlockNumberFor<Self>, BalanceOf<Self>>;

		#[pallet::constant]
		type Week: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type MaxBlock: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type Multiplier: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type VoteWeightMultiplier: Get<BalanceOf<Self>>;

		/// The maximum number of positions that should exist on an account.
		#[pallet::constant]
		type MaxPositions: Get<u32>;

		#[pallet::constant]
		type MarkupRefreshLimit: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ConfigSet { config: VeConfig<BalanceOf<T>, BlockNumberFor<T>> },
		Minted { addr: u128, value: BalanceOf<T>, end: BlockNumberFor<T>, now: BlockNumberFor<T> },
		Supply { supply_before: BalanceOf<T>, supply: BalanceOf<T> },
		LockCreated { addr: AccountIdOf<T>, value: BalanceOf<T>, unlock_time: BlockNumberFor<T> },
		UnlockTimeIncreased { addr: u128, unlock_time: BlockNumberFor<T> },
		AmountIncreased { who: AccountIdOf<T>, position: u128, value: BalanceOf<T> },
		Withdrawn { addr: u128, value: BalanceOf<T> },
		IncentiveSet { rewards_duration: BlockNumberFor<T> },
		RewardAdded { rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)> },
		Rewarded { addr: AccountIdOf<T>, rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)> },
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		Expired,
		BelowMinimumMint,
		LockNotExist,
		LockExist,
		NoRewards,
		ArgumentsError,
		ExceedsMaxPositions,
	}

	#[pallet::storage]
	#[pallet::getter(fn supply)]
	pub type Supply<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ve_configs)]
	pub type VeConfigs<T: Config> =
		StorageValue<_, VeConfig<BalanceOf<T>, BlockNumberFor<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn epoch)]
	pub type Epoch<T: Config> = StorageValue<_, U256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn locked)]
	pub type Locked<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u128,
		LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn user_locked)]
	pub type UserLocked<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		BalanceOf<T>,
		// LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
		ValueQuery,
	>;

	// Each week has a Point struct stored in PointHistory.
	#[pallet::storage]
	#[pallet::getter(fn point_history)]
	pub type PointHistory<T: Config> =
		StorageMap<_, Twox64Concat, U256, Point<BalanceOf<T>, BlockNumberFor<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_point_history)]
	pub type UserPointHistory<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u128,
		Blake2_128Concat,
		U256,
		Point<BalanceOf<T>, BlockNumberFor<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn user_point_epoch)]
	pub type UserPointEpoch<T: Config> = StorageMap<_, Blake2_128Concat, u128, U256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn slope_changes)]
	pub type SlopeChanges<T: Config> =
		StorageMap<_, Twox64Concat, BlockNumberFor<T>, i128, ValueQuery>;

	// Incentive
	#[pallet::storage]
	#[pallet::getter(fn incentive_configs)]
	pub type IncentiveConfigs<T: Config> = StorageValue<
		_,
		IncentiveConfig<CurrencyIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>,
		ValueQuery,
	>;

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

	#[pallet::storage]
	#[pallet::getter(fn user_markup_infos)]
	pub type UserMarkupInfos<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, UserMarkupInfo>;

	#[pallet::storage]
	#[pallet::getter(fn locked_tokens)]
	pub type LockedTokens<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		Blake2_128Concat,
		AccountIdOf<T>,
		LockedToken<BalanceOf<T>, BlockNumberFor<T>>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn total_lock)]
	pub type TotalLock<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn markup_coefficient)]
	pub type MarkupCoefficient<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, MarkupCoefficientInfo<BlockNumberFor<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn position)]
	pub type Position<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	pub type UserPositions<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		BoundedVec<u128, T::MaxPositions>,
		ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_config())]
		pub fn set_config(
			origin: OriginFor<T>,
			min_mint: Option<BalanceOf<T>>,       // Minimum mint balance
			min_block: Option<BlockNumberFor<T>>, // Minimum lockup time
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
			unlock_time: BlockNumberFor<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::create_lock_inner(&exchanger, value, unlock_time)
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::increase_amount())]
		pub fn increase_amount(
			origin: OriginFor<T>,
			position: u128,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			// TODO: ensure postion is owned by exchanger
			let user_positions = UserPositions::<T>::get(&exchanger);
			ensure!(user_positions.contains(&position), Error::<T>::LockNotExist);
			Self::increase_amount_inner(&exchanger, position, value)
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::increase_unlock_time())]
		pub fn increase_unlock_time(
			origin: OriginFor<T>,
			position: u128,
			time: BlockNumberFor<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			// TODO: ensure postion is owned by exchanger
			let user_positions = UserPositions::<T>::get(&exchanger);
			ensure!(user_positions.contains(&position), Error::<T>::LockNotExist);
			Self::increase_unlock_time_inner(&exchanger, position, time)
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn withdraw(origin: OriginFor<T>, position: u128) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			let user_positions = UserPositions::<T>::get(&exchanger);
			ensure!(user_positions.contains(&position), Error::<T>::LockNotExist);
			Self::withdraw_inner(&exchanger, position)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::notify_rewards())]
		pub fn notify_rewards(
			origin: OriginFor<T>,
			incentive_from: AccountIdOf<T>,
			rewards_duration: Option<BlockNumberFor<T>>,
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

		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::get_rewards())]
		pub fn redeem_unlock(origin: OriginFor<T>) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			// TODO: redeem_unlock
			Self::get_rewards_inner(&exchanger)
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::get_rewards())]
		pub fn set_markup_coefficient(
			origin: OriginFor<T>,
			asset_id: CurrencyId, // token类型
			markup: FixedU128,    // 单位token的加成系数
			hardcap: FixedU128,   // token对应加成硬顶
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			ensure!(markup <= hardcap, Error::<T>::ArgumentsError);

			if !TotalLock::<T>::contains_key(asset_id) {
				TotalLock::<T>::insert(asset_id, BalanceOf::<T>::zero());
			}
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			MarkupCoefficient::<T>::insert(
				asset_id,
				MarkupCoefficientInfo {
					markup_coefficient: markup,
					hardcap,
					update_block: current_block_number,
				},
			);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn _checkpoint(
			who: &AccountIdOf<T>,
			addr: u128,
			old_locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
			new_locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			Self::update_reward(Some(who))?;

			let mut u_old = Point::<BalanceOf<T>, BlockNumberFor<T>>::default();
			let mut u_new = Point::<BalanceOf<T>, BlockNumberFor<T>>::default();
			let mut new_dslope = 0_i128;
			let mut g_epoch: U256 = Self::epoch();
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();

			if old_locked.end > current_block_number && old_locked.amount > BalanceOf::<T>::zero() {
				u_old.slope = U256::from(old_locked.amount.saturated_into::<u128>())
					.checked_div(U256::from(T::MaxBlock::get().saturated_into::<u128>()))
					.map(|x| u128::try_from(x))
					.ok_or(ArithmeticError::Overflow)?
					.map_err(|_| ArithmeticError::Overflow)?
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
					.map(|x| u128::try_from(x))
					.ok_or(ArithmeticError::Overflow)?
					.map_err(|_| ArithmeticError::Overflow)?
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

			let mut last_point: Point<BalanceOf<T>, BlockNumberFor<T>> = Point {
				bias: 0_i128,
				slope: 0_i128,
				block: current_block_number,
				amount: Zero::zero(),
			};
			if g_epoch > U256::zero() {
				last_point = Self::point_history(g_epoch);
			} else {
				// last_point.amount = T::MultiCurrency::free_balance(
				// 	T::TokenType::get(),
				// 	&T::VeMintingPalletId::get().into_account_truncating(),
				// );
				last_point.amount = Self::supply();
			}
			let mut last_checkpoint = last_point.block;
			let mut t_i: BlockNumberFor<T> = last_checkpoint
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
					last_point.amount = Self::supply();
					// last_point.amount = T::MultiCurrency::free_balance(
					// 	T::TokenType::get(),
					// 	&T::VeMintingPalletId::get().into_account_truncating(),
					// );
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
			who: &AccountIdOf<T>,
			addr: u128,
			value: BalanceOf<T>,
			unlock_time: BlockNumberFor<T>,
			locked_balance: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			let mut _locked = locked_balance;
			let supply_before = Self::supply();
			Supply::<T>::set(supply_before.checked_add(value).ok_or(ArithmeticError::Overflow)?);

			let old_locked = _locked.clone();
			_locked.amount = _locked.amount.checked_add(value).ok_or(ArithmeticError::Overflow)?;
			if unlock_time != Zero::zero() {
				_locked.end = unlock_time
			}
			Locked::<T>::insert(addr, _locked.clone());

			let free_balance = T::MultiCurrency::free_balance(T::TokenType::get(), &who);
			if value != BalanceOf::<T>::zero() && value <= free_balance {
				// TODO: compare with free balance
				let new_locked_balance = UserLocked::<T>::get(who)
					.checked_add(value)
					.ok_or(ArithmeticError::Underflow)?;
				T::MultiCurrency::set_lock(
					VE_LOCK_ID,
					T::TokenType::get(),
					who,
					new_locked_balance,
				)?;
				UserLocked::<T>::set(who, new_locked_balance);
			}
			// TODO: one user can have multiple positions
			Self::markup_calc(
				who,
				addr,
				old_locked,
				_locked.clone(),
				UserMarkupInfos::<T>::get(who).as_ref(),
			)?;
			// Self::_checkpoint(addr, old_locked, _locked.clone())?;

			Self::deposit_event(Event::Minted {
				addr: addr.clone(),
				value,
				end: _locked.end,
				now: current_block_number,
			});
			Self::deposit_event(Event::Supply {
				supply_before,
				supply: supply_before.checked_add(value).ok_or(ArithmeticError::Overflow)?,
			});
			Ok(())
		}

		// Get the current voting power for `addr`
		pub(crate) fn balance_of_position_current_block(
			addr: u128,
		) -> Result<BalanceOf<T>, DispatchError> {
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			let u_epoch = Self::user_point_epoch(addr);
			if u_epoch == U256::zero() {
				return Ok(Zero::zero());
			} else {
				let mut last_point: Point<BalanceOf<T>, BlockNumberFor<T>> =
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
						T::VoteWeightMultiplier::get()
							.checked_mul((last_point.bias as u128).unique_saturated_into())
							.ok_or(ArithmeticError::Overflow)?,
					)
					.ok_or(ArithmeticError::Overflow)?)
			}
		}

		// Measure voting power of `addr` at block height `block`
		pub(crate) fn balance_of_position_at(
			addr: u128,
			block: BlockNumberFor<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
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

			let mut upoint: Point<BalanceOf<T>, BlockNumberFor<T>> =
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
					T::VoteWeightMultiplier::get()
						.checked_mul((upoint.bias as u128).unique_saturated_into())
						.ok_or(ArithmeticError::Overflow)?,
				)
				.ok_or(ArithmeticError::Overflow)?)
		}

		pub(crate) fn balance_of_at(
			addr: &AccountIdOf<T>,
			block: BlockNumberFor<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			let mut balance = BalanceOf::<T>::zero();
			UserPositions::<T>::get(addr).into_iter().try_for_each(
				|position| -> DispatchResult {
					balance = balance
						.checked_add(Self::balance_of_position_at(position, block)?)
						.ok_or(ArithmeticError::Overflow)?;
					Ok(())
				},
			)?;
			Ok(balance)
		}

		pub(crate) fn balance_of_current_block(
			addr: &AccountIdOf<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			let mut balance = BalanceOf::<T>::zero();
			UserPositions::<T>::get(addr).into_iter().try_for_each(
				|position| -> DispatchResult {
					balance = balance
						.checked_add(Self::balance_of_position_current_block(position)?)
						.ok_or(ArithmeticError::Overflow)?;
					Ok(())
				},
			)?;
			Ok(balance)
		}

		pub fn markup_calc(
			addr: &AccountIdOf<T>,
			position: u128,
			mut old_locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
			mut new_locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
			user_markup_info: Option<&UserMarkupInfo>,
		) -> DispatchResult {
			// let mut user_markup_info = UserMarkupInfos::<T>::get(addr);
			// user_markup_info.old_locked = old_locked.clone();
			if let Some(info) = user_markup_info {
				old_locked.amount = FixedU128::from_inner(old_locked.amount)
					.checked_mul(&info.old_markup_coefficient)
					.and_then(|x| x.into_inner().checked_add(old_locked.amount))
					.ok_or(ArithmeticError::Overflow)?;
				new_locked.amount = FixedU128::from_inner(new_locked.amount)
					.checked_mul(&info.markup_coefficient)
					.and_then(|x| x.into_inner().checked_add(new_locked.amount))
					.ok_or(ArithmeticError::Overflow)?;
			}

			// Locked::<T>::insert(addr, new_locked.clone());

			// TODO: position
			Self::_checkpoint(addr, position, old_locked.clone(), new_locked.clone())?;
			Ok(())
		}

		fn deposit_markup(
			origin: OriginFor<T>,
			asset_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let addr = ensure_signed(origin)?;
			let markup_coefficient =
				MarkupCoefficient::<T>::get(asset_id).ok_or(Error::<T>::ArgumentsError)?; // Ensure it is the correct token type.
			ensure!(!value.is_zero(), Error::<T>::ArgumentsError);

			TotalLock::<T>::try_mutate(asset_id, |total_lock| -> DispatchResult {
				*total_lock = total_lock.checked_add(value).ok_or(ArithmeticError::Overflow)?;
				// T::MultiCurrency::transfer(asset_id, &addr, &T::VeMintingPalletId::get(),
				// value)?;
				Ok(())
			})?;

			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();

			let mut user_markup_info = UserMarkupInfos::<T>::get(&addr).unwrap_or_default();
			let delta_coefficient: FixedU128 = markup_coefficient
				.markup_coefficient
				.checked_mul(&FixedU128::from_inner(value))
				.ok_or(ArithmeticError::Overflow)?;
			// let mut new_locked_token =
			// 	LockedToken { asset_id, amount: value, markup_coefficient: delta_coefficient };
			let mut locked_token = LockedTokens::<T>::get(asset_id, &addr).unwrap_or(LockedToken {
				amount: Zero::zero(),
				markup_coefficient: Zero::zero(),
				refresh_block: current_block_number,
			});
			// let new_locked_token = match locked_tokens {
			// 	Some(mut l) => {
			let asset_id_markup_coefficient =
				locked_token.markup_coefficient.saturating_add(delta_coefficient);
			locked_token.markup_coefficient =
				match markup_coefficient.hardcap.cmp(&asset_id_markup_coefficient) {
					Ordering::Less => {
						Self::update_markup_info(
							&addr,
							user_markup_info
								.markup_coefficient
								.saturating_sub(locked_token.markup_coefficient)
								.saturating_add(markup_coefficient.hardcap),
							&mut user_markup_info,
						);
						markup_coefficient.hardcap
					},
					Ordering::Equal | Ordering::Greater => {
						// TODO: need logic of hardcap
						Self::update_markup_info(
							&addr,
							user_markup_info.markup_coefficient.saturating_add(delta_coefficient),
							&mut user_markup_info,
						);
						asset_id_markup_coefficient
					},
				};
			locked_token.amount = locked_token.amount.saturating_add(value);
			locked_token.refresh_block = current_block_number;

			T::MultiCurrency::set_lock(MARKUP_LOCK_ID, asset_id, &addr, locked_token.amount)?;
			LockedTokens::<T>::insert(&asset_id, &addr, locked_token);
			UserPositions::<T>::get(&addr).into_iter().try_for_each(
				|position| -> DispatchResult {
					let _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> =
						Self::locked(position);
					ensure!(!_locked.amount.is_zero(), Error::<T>::ArgumentsError);
					Self::markup_calc(
						&addr,
						position,
						_locked.clone(),
						_locked,
						Some(&user_markup_info),
					)
				},
			)?;

			// Locked cannot be updated because it is markup, not a lock vBNC
			// Locked::<T>::insert(addr, _locked.clone());
			// UserMarkupInfos::<T>::insert(&addr, user_markup_info);
			// Self::deposit_event(Event::Minted {
			// 	addr: addr.clone(),
			// 	value,
			// 	end: _locked.end,
			// 	now: current_block_number,
			// });
			// Self::deposit_event(Event::Supply {
			// 	supply_before,
			// 	supply: supply_before.checked_add(&value).ok_or(ArithmeticError::Overflow)?,
			// });
			Ok(())
		}

		fn withdraw_markup(origin: OriginFor<T>, asset_id: CurrencyIdOf<T>) -> DispatchResult {
			let addr = ensure_signed(origin)?;
			let markup_coefficient =
				MarkupCoefficient::<T>::get(asset_id).ok_or(Error::<T>::ArgumentsError)?; // Ensure it is the correct token type.

			let mut user_markup_info = UserMarkupInfos::<T>::get(&addr).unwrap_or_default();

			let mut locked_token =
				LockedTokens::<T>::get(&asset_id, &addr).ok_or(Error::<T>::LockNotExist)?;
			Self::update_markup_info(
				&addr,
				user_markup_info
					.markup_coefficient
					.saturating_sub(locked_token.markup_coefficient),
				&mut user_markup_info,
			);
			TotalLock::<T>::try_mutate(asset_id, |total_lock| -> DispatchResult {
				*total_lock =
					total_lock.checked_sub(locked_token.amount).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;
			T::MultiCurrency::remove_lock(MARKUP_LOCK_ID, asset_id, &addr)?;

			LockedTokens::<T>::remove(&asset_id, &addr);
			UserPositions::<T>::get(&addr).into_iter().try_for_each(
				|position| -> DispatchResult {
					let _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> =
						Self::locked(position);
					ensure!(!_locked.amount.is_zero(), Error::<T>::ArgumentsError); // TODO
					Self::markup_calc(
						&addr,
						position,
						_locked.clone(),
						_locked,
						Some(&user_markup_info),
					)
				},
			)?;
			// UserMarkupInfos::<T>::insert(&addr, user_markup_info);
			Ok(())
		}

		fn refresh(origin: OriginFor<T>, asset_id: CurrencyIdOf<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let markup_coefficient =
				MarkupCoefficient::<T>::get(asset_id).ok_or(Error::<T>::ArgumentsError)?;
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			let limit = T::MarkupRefreshLimit::get();
			let mut all_refreshed = true;
			let mut refresh_count = 0;
			let locked_tokens = LockedTokens::<T>::iter_prefix(&asset_id);

			for (addr, mut locked_token) in locked_tokens {
				if refresh_count >= limit {
					all_refreshed = false;
					break;
				}

				if locked_token.refresh_block <= markup_coefficient.update_block {
					locked_token.refresh_block = current_block_number;

					let new_markup_coefficient = markup_coefficient
						.markup_coefficient
						.checked_mul(&FixedU128::from_inner(locked_token.amount))
						.ok_or(ArithmeticError::Overflow)?;
					let mut user_markup_info =
						UserMarkupInfos::<T>::get(&addr).ok_or(Error::<T>::LockNotExist)?;

					locked_token.markup_coefficient =
						match markup_coefficient.hardcap.cmp(&new_markup_coefficient) {
							Ordering::Less => {
								Self::update_markup_info(
									&addr,
									user_markup_info
										.markup_coefficient
										.saturating_sub(locked_token.markup_coefficient)
										.saturating_add(markup_coefficient.hardcap),
									&mut user_markup_info,
								);
								markup_coefficient.hardcap
							},
							Ordering::Equal | Ordering::Greater => {
								Self::update_markup_info(
									&addr,
									user_markup_info
										.markup_coefficient
										.saturating_sub(locked_token.markup_coefficient)
										.saturating_add(new_markup_coefficient),
									&mut user_markup_info,
								);
								new_markup_coefficient
							},
						};

					LockedTokens::<T>::insert(&asset_id, &addr, locked_token);
					UserPositions::<T>::get(&addr).into_iter().try_for_each(
						|position| -> DispatchResult {
							let _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> =
								Self::locked(position);
							ensure!(!_locked.amount.is_zero(), Error::<T>::ArgumentsError); // TODO
							Self::markup_calc(
								&addr,
								position,
								_locked.clone(),
								_locked,
								Some(&user_markup_info),
							)
						},
					)?;

					refresh_count += 1;
				}
			}
			Ok(())
		}
	}
}

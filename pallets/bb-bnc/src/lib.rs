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

use bifrost_primitives::{Balance, CurrencyId, PoolId};
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{
			AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Convert,
			Saturating, UniqueSaturatedInto, Zero,
		},
		ArithmeticError, DispatchError, FixedPointNumber, FixedU128, SaturatedConversion,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
pub use incentive::*;
use orml_traits::{LockIdentifier, MultiCurrency, MultiLockableCurrency};
use sp_core::{U256, U512};
use sp_std::{borrow::ToOwned, cmp::Ordering, collections::btree_map::BTreeMap, vec, vec::Vec};
pub use traits::{BbBNCInterface, LockedToken, MarkupCoefficientInfo, MarkupInfo, UserMarkupInfo};
pub use weights::WeightInfo;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

const BB_LOCK_ID: LockIdentifier = *b"bbbnclck";
const MARKUP_LOCK_ID: LockIdentifier = *b"bbbncmkp";
pub const BB_BNC_SYSTEM_POOL_ID: PoolId = u32::MAX;
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct BbConfig<Balance, BlockNumber> {
	/// Minimum number of TokenType that users can lock
	min_mint: Balance,
	/// Minimum time that users can lock
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
		type IncentivePalletId: Get<PalletId>;

		#[pallet::constant]
		type BuyBackAccount: Get<PalletId>;

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

		/// Maximum number of users per refresh.
		#[pallet::constant]
		type MarkupRefreshLimit: Get<u32>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The minimum number of TokenType and minimum time that users can lock has been set.
		ConfigSet { config: BbConfig<BalanceOf<T>, BlockNumberFor<T>> },
		/// A successful call of the `create_lock` function.
		Minted {
			/// the user who mint
			who: AccountIdOf<T>,
			/// the position of this minting
			position: u128,
			/// the value of this minting
			value: BalanceOf<T>,
			/// total mint value for this user
			total_value: BalanceOf<T>,
			/// withdrawable time
			end: BlockNumberFor<T>,
			/// current time
			now: BlockNumberFor<T>,
		},
		/// Change in TokenType locked after calling.
		Supply {
			/// The balance before the change.
			supply_before: BalanceOf<T>,
			/// The balance after the change.
			supply: BalanceOf<T>,
		},
		/// A position was created.
		LockCreated {
			/// Position owner
			who: AccountIdOf<T>,
			/// Position ID
			position: u128,
			/// Locked value
			value: BalanceOf<T>,
			/// withdrawable time
			unlock_time: BlockNumberFor<T>,
		},
		/// A position was extended.
		UnlockTimeIncreased {
			/// Position owner
			who: AccountIdOf<T>,
			/// Position ID
			position: u128,
			/// New withdrawable time
			unlock_time: BlockNumberFor<T>,
		},
		/// A position was increased.
		AmountIncreased {
			/// Position owner
			who: AccountIdOf<T>,
			/// Position ID
			position: u128,
			/// Increased value, not new locked value
			value: BalanceOf<T>,
		},
		/// A position was withdrawn.
		Withdrawn {
			/// Position owner
			who: AccountIdOf<T>,
			/// Position ID
			position: u128,
			/// Withdrawn value
			value: BalanceOf<T>,
		},
		/// Incentive config set.
		IncentiveSet {
			incentive_config:
				IncentiveConfig<CurrencyIdOf<T>, BalanceOf<T>, BlockNumberFor<T>, AccountIdOf<T>>,
		},
		/// The rewards for this round have been added to the system account.
		RewardAdded { rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)> },
		/// The user has received the reward.
		Rewarded { who: AccountIdOf<T>, rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)> },
		/// This currency_id has been refreshed.
		AllRefreshed { currency_id: CurrencyIdOf<T> },
		/// This currency_id has been partially refreshed.
		PartiallyRefreshed { currency_id: CurrencyIdOf<T> },
		/// Notify reward failed.
		NotifyRewardFailed { rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)> },
		/// Markup has been deposited.
		MarkupDeposited {
			/// The user who deposited
			who: AccountIdOf<T>,
			/// The token type of the deposit
			currency_id: CurrencyIdOf<T>,
			/// The amount of currency_id to be deposited this time
			value: BalanceOf<T>,
		},
		/// Markup has been withdrawn.
		MarkupWithdrawn { who: AccountIdOf<T>, currency_id: CurrencyIdOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not enough balance
		NotEnoughBalance,
		/// Block number is expired
		Expired,
		/// Below minimum mint
		BelowMinimumMint,
		/// Lock does not exist
		LockNotExist,
		/// Lock already exists
		LockExist,
		/// Arguments error
		ArgumentsError,
		/// Exceeds max positions
		ExceedsMaxPositions,
		/// No controller
		NoController,
		/// User farming pool overflow
		UserFarmingPoolOverflow,
	}

	/// Total supply of locked tokens
	#[pallet::storage]
	pub type Supply<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Configurations
	#[pallet::storage]
	pub type BbConfigs<T: Config> =
		StorageValue<_, BbConfig<BalanceOf<T>, BlockNumberFor<T>>, ValueQuery>;

	/// Global epoch
	#[pallet::storage]
	pub type Epoch<T: Config> = StorageValue<_, U256, ValueQuery>;

	/// Locked tokens. [position => LockedBalance]
	#[pallet::storage]
	pub type Locked<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u128,
		LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
		ValueQuery,
	>;

	/// User locked tokens. [who => value]
	#[pallet::storage]
	pub type UserLocked<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// Each week has a Point struct stored in PointHistory.
	#[pallet::storage]
	pub type PointHistory<T: Config> =
		StorageMap<_, Twox64Concat, U256, Point<BalanceOf<T>, BlockNumberFor<T>>, ValueQuery>;

	/// User point history. [(who, epoch) => Point]
	#[pallet::storage]
	pub type UserPointHistory<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u128,
		Blake2_128Concat,
		U256,
		Point<BalanceOf<T>, BlockNumberFor<T>>,
		ValueQuery,
	>;

	/// User point epoch. [who => epoch]
	#[pallet::storage]
	pub type UserPointEpoch<T: Config> = StorageMap<_, Blake2_128Concat, u128, U256, ValueQuery>;

	/// Slope changes. [block => slope]
	#[pallet::storage]
	pub type SlopeChanges<T: Config> =
		StorageMap<_, Twox64Concat, BlockNumberFor<T>, i128, ValueQuery>;

	/// Farming pool incentive configurations.[pool_id => IncentiveConfig]
	#[pallet::storage]
	pub type IncentiveConfigs<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		PoolId,
		IncentiveConfig<CurrencyIdOf<T>, BalanceOf<T>, BlockNumberFor<T>, AccountIdOf<T>>,
		ValueQuery,
	>;

	/// User reward per token paid. [who => reward per token]
	#[pallet::storage]
	pub type UserRewardPerTokenPaid<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
		ValueQuery,
	>;

	/// User rewards. [who => rewards]
	#[pallet::storage]
	pub type Rewards<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>>;

	/// User markup infos. [who => UserMarkupInfo]
	#[pallet::storage]
	pub type UserMarkupInfos<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, UserMarkupInfo>;

	/// Locked tokens for markup. [(token, who) => value]
	#[pallet::storage]
	pub type LockedTokens<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		Blake2_128Concat,
		AccountIdOf<T>,
		LockedToken<BalanceOf<T>, BlockNumberFor<T>>,
	>;

	/// Total locked tokens for markup. [token => value]
	#[pallet::storage]
	pub type TotalLock<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// Markup coefficient. [token => MarkupCoefficientInfo]
	#[pallet::storage]
	pub type MarkupCoefficient<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, MarkupCoefficientInfo<BlockNumberFor<T>>>;

	/// The last position of all.
	#[pallet::storage]
	pub type Position<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// Positions owned by the user. [who => positions]
	#[pallet::storage]
	pub type UserPositions<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		BoundedVec<u128, T::MaxPositions>,
		ValueQuery,
	>;

	/// The pool ID of the user participating in the farming pool.
	#[pallet::storage]
	pub type UserFarmingPool<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		BoundedVec<PoolId, ConstU32<256>>,
		ValueQuery,
	>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let conf = IncentiveConfigs::<T>::get(BB_BNC_SYSTEM_POOL_ID);
			if n == conf.period_finish {
				if let Some(e) = Self::notify_reward_amount(
					BB_BNC_SYSTEM_POOL_ID,
					&conf.incentive_controller,
					conf.last_reward.clone(),
				)
				.err()
				{
					log::error!(
						target: "bb-bnc::notify_reward_amount",
						"Received invalid justification for {:?}",
						e,
					);
					Self::deposit_event(Event::NotifyRewardFailed { rewards: conf.last_reward });
				}
			}

			T::DbWeight::get().writes(1_u64)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set configuration.
		///
		/// Set the minimum number of tokens and minimum time that users can lock.
		///
		/// - `min_mint`: The minimum mint balance
		/// - `min_block`: The minimum lockup time
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_config())]
		pub fn set_config(
			origin: OriginFor<T>,
			min_mint: Option<BalanceOf<T>>,
			min_block: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut bb_config = BbConfigs::<T>::get();
			if let Some(min_mint) = min_mint {
				bb_config.min_mint = min_mint;
			};
			if let Some(min_block) = min_block {
				bb_config.min_block = min_block;
			};
			BbConfigs::<T>::set(bb_config.clone());

			Self::deposit_event(Event::ConfigSet { config: bb_config });
			Ok(())
		}

		/// Create a lock.
		///
		/// If the signer already has a position, the position will not be extended. it will be
		/// created a new position until the maximum number of positions is reached.
		///
		/// - `value`: The amount of tokens to lock
		/// - `unlock_time`: The lockup time
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

		/// Increase the lock amount.
		///
		/// If the signer does not have the position, it doesn't work and the position will not be
		/// created. Only the position existed and owned by the signer, the locking amount will be
		/// increased.
		///
		/// - `position`: The lock position
		/// - `value`: The amount of tokens to increase
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::increase_amount())]
		pub fn increase_amount(
			origin: OriginFor<T>,
			position: u128,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			let user_positions = UserPositions::<T>::get(&exchanger);
			ensure!(user_positions.contains(&position), Error::<T>::LockNotExist);
			Self::increase_amount_inner(&exchanger, position, value)
		}

		/// Increase the unlock time.
		///
		/// If the signer does not have the position, it doesn't work and the position will not be
		/// created. Only the position existed and owned by the signer, the locking time will be
		/// increased.
		///
		/// - `position`: The lock position
		/// - `time`: Additional lock time
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::increase_unlock_time())]
		pub fn increase_unlock_time(
			origin: OriginFor<T>,
			position: u128,
			time: BlockNumberFor<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			let user_positions = UserPositions::<T>::get(&exchanger);
			ensure!(user_positions.contains(&position), Error::<T>::LockNotExist);
			Self::increase_unlock_time_inner(&exchanger, position, time)
		}

		/// Withdraw the locked tokens after unlock time.
		///
		/// - `position`: The lock position
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::withdraw())]
		pub fn withdraw(origin: OriginFor<T>, position: u128) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			let user_positions = UserPositions::<T>::get(&exchanger);
			ensure!(user_positions.contains(&position), Error::<T>::LockNotExist);
			Self::withdraw_inner(&exchanger, position)
		}

		/// Notify rewards.
		///
		/// Set the incentive controller and rewards token type for future round. Reward duration
		/// should be one round interval. It will notify the rewards from incentive controller to
		/// the system account and start a new round immediately, and the next round will auto start
		/// at now + rewards_duration.
		///
		/// - `incentive_from`: The incentive controller
		/// - `rewards_duration`: The rewards duration
		/// - `rewards`: The rewards
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::notify_rewards())]
		pub fn notify_rewards(
			origin: OriginFor<T>,
			incentive_from: AccountIdOf<T>,
			rewards_duration: Option<BlockNumberFor<T>>,
			rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::set_incentive(
				BB_BNC_SYSTEM_POOL_ID,
				rewards_duration,
				Some(incentive_from.clone()),
			);
			Self::notify_reward_amount(BB_BNC_SYSTEM_POOL_ID, &Some(incentive_from), rewards)
		}

		/// Get rewards for the signer.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::get_rewards())]
		pub fn get_rewards(origin: OriginFor<T>) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::get_rewards_inner(BB_BNC_SYSTEM_POOL_ID, &exchanger, None)
		}

		/// Fast unlocking, handling fee applies
		///
		/// When users want to redeem early regardless of cost, they can use this call.
		///
		/// - `position`: The lock position
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::redeem_unlock())]
		pub fn redeem_unlock(origin: OriginFor<T>, position: u128) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::redeem_unlock_inner(&exchanger, position)
		}

		/// Set markup configurations.
		///
		/// - `currency_id`: The token type
		/// - `markup`: The markup coefficient
		/// - `hardcap`: The markup hardcap
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::set_markup_coefficient())]
		pub fn set_markup_coefficient(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			markup: FixedU128,
			hardcap: FixedU128,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if !TotalLock::<T>::contains_key(currency_id) {
				TotalLock::<T>::insert(currency_id, BalanceOf::<T>::zero());
			}
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			MarkupCoefficient::<T>::insert(
				currency_id,
				MarkupCoefficientInfo {
					markup_coefficient: markup,
					hardcap,
					update_block: current_block_number,
				},
			);
			Ok(())
		}

		/// Deposit markup.
		///
		/// Deposit the token to the system account for the markup.
		///
		/// - `currency_id`: The token type
		/// - `value`: The amount of tokens to deposit
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::deposit_markup())]
		pub fn deposit_markup(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::deposit_markup_inner(&exchanger, currency_id, value)
		}

		/// Withdraw markup.
		///
		/// Withdraw the token from the system account for the markup.
		///
		/// - `currency_id`: The token type
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::withdraw_markup())]
		pub fn withdraw_markup(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			Self::withdraw_markup_inner(&exchanger, currency_id)
		}

		/// Refresh the markup.
		///
		/// Any user can call this function to refresh the markup coefficient. The maximum number of
		/// accounts that can be refreshed in one execution is MarkupRefreshLimit.
		///
		/// - `currency_id`: The token type
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::refresh())]
		pub fn refresh(origin: OriginFor<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
			let _exchanger = ensure_signed(origin)?;
			Self::refresh_inner(currency_id)
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn _checkpoint(
			who: &AccountIdOf<T>,
			position: u128,
			old_locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
			new_locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			Self::update_reward_all(who)?;

			let mut u_old = Point::<BalanceOf<T>, BlockNumberFor<T>>::default();
			let mut u_new = Point::<BalanceOf<T>, BlockNumberFor<T>>::default();
			let mut new_dslope = 0_i128;
			let mut g_epoch: U256 = Epoch::<T>::get();
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
			let mut old_dslope = SlopeChanges::<T>::get(old_locked.end);
			if new_locked.end != Zero::zero() {
				if new_locked.end == old_locked.end {
					new_dslope = old_dslope
				} else {
					new_dslope = SlopeChanges::<T>::get(new_locked.end)
				}
			}

			let mut last_point: Point<BalanceOf<T>, BlockNumberFor<T>> = Point {
				bias: 0_i128,
				slope: 0_i128,
				block: current_block_number,
				amount: Zero::zero(),
			};
			if g_epoch > U256::zero() {
				last_point = PointHistory::<T>::get(g_epoch);
			} else {
				last_point.amount = Supply::<T>::get();
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
					d_slope = SlopeChanges::<T>::get(t_i)
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
					last_point.amount = Supply::<T>::get();
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
			let user_epoch = UserPointEpoch::<T>::get(position)
				.checked_add(U256::one())
				.ok_or(ArithmeticError::Overflow)?;
			UserPointEpoch::<T>::insert(position, user_epoch);
			u_new.block = current_block_number;
			u_new.amount = new_locked.amount;
			UserPointHistory::<T>::insert(position, user_epoch, u_new);

			Ok(())
		}

		pub fn _deposit_for(
			who: &AccountIdOf<T>,
			position: u128,
			value: BalanceOf<T>,
			unlock_time: BlockNumberFor<T>,
			locked_balance: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			let mut _locked = locked_balance;
			let supply_before = Supply::<T>::get();
			let supply_after = supply_before.checked_add(value).ok_or(ArithmeticError::Overflow)?;
			Supply::<T>::set(supply_after);

			let old_locked = _locked.clone();
			_locked.amount = _locked.amount.checked_add(value).ok_or(ArithmeticError::Overflow)?;
			if unlock_time != Zero::zero() {
				_locked.end = unlock_time
			}
			Locked::<T>::insert(position, _locked.clone());

			let free_balance = T::MultiCurrency::free_balance(T::TokenType::get(), &who);
			if value != BalanceOf::<T>::zero() {
				let new_locked_balance = UserLocked::<T>::get(who)
					.checked_add(value)
					.ok_or(ArithmeticError::Overflow)?;
				ensure!(new_locked_balance <= free_balance, Error::<T>::NotEnoughBalance);
				Self::set_ve_locked(who, new_locked_balance)?;
			}

			Self::markup_calc(
				who,
				position,
				old_locked,
				_locked.clone(),
				UserMarkupInfos::<T>::get(who).as_ref(),
			)?;

			Self::deposit_event(Event::Minted {
				who: who.clone(),
				position,
				value,
				total_value: _locked.amount,
				end: _locked.end,
				now: current_block_number,
			});
			Self::deposit_event(Event::Supply { supply_before, supply: supply_after });
			Ok(())
		}

		// Get the current voting power for `position`
		pub(crate) fn balance_of_position_current_block(
			position: u128,
		) -> Result<BalanceOf<T>, DispatchError> {
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			let u_epoch = UserPointEpoch::<T>::get(position);
			if u_epoch == U256::zero() {
				return Ok(Zero::zero());
			} else {
				let mut last_point: Point<BalanceOf<T>, BlockNumberFor<T>> =
					UserPointHistory::<T>::get(position, u_epoch);

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

				Ok(T::VoteWeightMultiplier::get()
					.checked_mul((last_point.bias as u128).unique_saturated_into())
					.ok_or(ArithmeticError::Overflow)?)
			}
		}

		// Measure voting power of `position` at block height `block`
		pub(crate) fn balance_of_position_at(
			position: u128,
			block: BlockNumberFor<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			ensure!(block <= current_block_number, Error::<T>::Expired);

			// Binary search
			let mut _min = U256::zero();
			let mut _max = UserPointEpoch::<T>::get(position);
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

				if UserPointHistory::<T>::get(position, _mid).block <= block {
					_min = _mid
				} else {
					_max = _mid.checked_sub(U256::one()).ok_or(ArithmeticError::Overflow)?
				}
			}

			let mut upoint: Point<BalanceOf<T>, BlockNumberFor<T>> =
				UserPointHistory::<T>::get(position, _min);
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
			Ok(T::VoteWeightMultiplier::get()
				.checked_mul((upoint.bias as u128).unique_saturated_into())
				.ok_or(ArithmeticError::Overflow)?)
		}

		pub(crate) fn balance_of_at(
			who: &AccountIdOf<T>,
			block: BlockNumberFor<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			let mut balance = BalanceOf::<T>::zero();
			UserPositions::<T>::get(who).into_iter().try_for_each(
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
			who: &AccountIdOf<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			let mut balance = BalanceOf::<T>::zero();
			UserPositions::<T>::get(who).into_iter().try_for_each(
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
			who: &AccountIdOf<T>,
			position: u128,
			mut old_locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
			mut new_locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
			user_markup_info: Option<&UserMarkupInfo>,
		) -> DispatchResult {
			if let Some(info) = user_markup_info {
				old_locked.amount = info
					.old_markup_coefficient
					.checked_mul_int(old_locked.amount)
					.and_then(|x| x.checked_add(old_locked.amount))
					.ok_or(ArithmeticError::Overflow)?;
				new_locked.amount = info
					.markup_coefficient
					.checked_mul_int(new_locked.amount)
					.and_then(|x| x.checked_add(new_locked.amount))
					.ok_or(ArithmeticError::Overflow)?;
			}

			Self::_checkpoint(who, position, old_locked.clone(), new_locked.clone())?;
			Ok(())
		}

		pub fn deposit_markup_inner(
			who: &AccountIdOf<T>,
			currency_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let markup_coefficient =
				MarkupCoefficient::<T>::get(currency_id).ok_or(Error::<T>::ArgumentsError)?; // Ensure it is the correct token type.
			ensure!(!value.is_zero(), Error::<T>::ArgumentsError);

			TotalLock::<T>::try_mutate(currency_id, |total_lock| -> DispatchResult {
				*total_lock = total_lock.checked_add(value).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;

			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();

			let mut user_markup_info = UserMarkupInfos::<T>::get(&who).unwrap_or_default();
			let mut locked_token =
				LockedTokens::<T>::get(currency_id, &who).unwrap_or(LockedToken {
					amount: Zero::zero(),
					markup_coefficient: Zero::zero(),
					refresh_block: current_block_number,
				});
			locked_token.amount = locked_token.amount.saturating_add(value);

			let left: FixedU128 = FixedU128::checked_from_integer(locked_token.amount)
				.and_then(|x| x.checked_mul(&markup_coefficient.markup_coefficient))
				.and_then(|x| {
					x.checked_div(&FixedU128::checked_from_integer(TotalLock::<T>::get(
						currency_id,
					))?)
				})
				.ok_or(ArithmeticError::Overflow)?;

			let total_issuance = T::MultiCurrency::total_issuance(currency_id);
			let right: FixedU128 = FixedU128::checked_from_integer(locked_token.amount)
				.and_then(|x| x.checked_mul(&markup_coefficient.markup_coefficient))
				.and_then(|x| x.checked_div(&FixedU128::checked_from_integer(total_issuance)?))
				.ok_or(ArithmeticError::Overflow)?;

			let currency_id_markup_coefficient: FixedU128 =
				left.checked_add(&right).ok_or(ArithmeticError::Overflow)?;
			let new_markup_coefficient =
				match markup_coefficient.hardcap.cmp(&currency_id_markup_coefficient) {
					Ordering::Less => markup_coefficient.hardcap,
					Ordering::Equal | Ordering::Greater => currency_id_markup_coefficient,
				};
			Self::update_markup_info(
				&who,
				user_markup_info
					.markup_coefficient
					.saturating_sub(locked_token.markup_coefficient)
					.saturating_add(new_markup_coefficient),
				&mut user_markup_info,
			);
			locked_token.markup_coefficient = new_markup_coefficient;
			locked_token.refresh_block = current_block_number;

			T::MultiCurrency::set_lock(MARKUP_LOCK_ID, currency_id, &who, locked_token.amount)?;
			LockedTokens::<T>::insert(&currency_id, &who, locked_token);
			UserPositions::<T>::get(&who).into_iter().try_for_each(
				|position| -> DispatchResult {
					let _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> =
						Locked::<T>::get(position);
					ensure!(!_locked.amount.is_zero(), Error::<T>::ArgumentsError);
					Self::markup_calc(
						&who,
						position,
						_locked.clone(),
						_locked,
						Some(&user_markup_info),
					)
				},
			)?;

			// Locked cannot be updated because it is markup, not a lock vBNC
			Self::deposit_event(Event::MarkupDeposited { who: who.clone(), currency_id, value });
			Ok(())
		}

		pub fn withdraw_markup_inner(
			who: &AccountIdOf<T>,
			currency_id: CurrencyIdOf<T>,
		) -> DispatchResult {
			let _ = MarkupCoefficient::<T>::get(currency_id).ok_or(Error::<T>::ArgumentsError)?; // Ensure it is the correct token type.

			let mut user_markup_info = UserMarkupInfos::<T>::get(&who).unwrap_or_default();

			let locked_token =
				LockedTokens::<T>::get(&currency_id, &who).ok_or(Error::<T>::LockNotExist)?;
			Self::update_markup_info(
				&who,
				user_markup_info
					.markup_coefficient
					.saturating_sub(locked_token.markup_coefficient),
				&mut user_markup_info,
			);
			TotalLock::<T>::try_mutate(currency_id, |total_lock| -> DispatchResult {
				*total_lock =
					total_lock.checked_sub(locked_token.amount).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;
			T::MultiCurrency::remove_lock(MARKUP_LOCK_ID, currency_id, &who)?;

			LockedTokens::<T>::remove(&currency_id, &who);
			UserPositions::<T>::get(&who).into_iter().try_for_each(
				|position| -> DispatchResult {
					let _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> =
						Locked::<T>::get(position);
					ensure!(!_locked.amount.is_zero(), Error::<T>::ArgumentsError); // TODO
					Self::markup_calc(
						&who,
						position,
						_locked.clone(),
						_locked,
						Some(&user_markup_info),
					)
				},
			)?;

			Self::deposit_event(Event::MarkupWithdrawn { who: who.clone(), currency_id });
			Ok(())
		}

		pub fn refresh_inner(currency_id: CurrencyIdOf<T>) -> DispatchResult {
			let markup_coefficient =
				MarkupCoefficient::<T>::get(currency_id).ok_or(Error::<T>::ArgumentsError)?;
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			let limit = T::MarkupRefreshLimit::get();
			let mut all_refreshed = true;
			let mut refresh_count = 0;
			let locked_tokens = LockedTokens::<T>::iter_prefix(&currency_id);

			for (who, mut locked_token) in locked_tokens {
				if refresh_count >= limit {
					all_refreshed = false;
					break;
				}

				if locked_token.refresh_block <= markup_coefficient.update_block {
					locked_token.refresh_block = current_block_number;

					let left: FixedU128 = FixedU128::checked_from_integer(locked_token.amount)
						.and_then(|x| x.checked_mul(&markup_coefficient.markup_coefficient))
						.and_then(|x| {
							x.checked_div(&FixedU128::checked_from_integer(TotalLock::<T>::get(
								currency_id,
							))?)
						})
						.ok_or(ArithmeticError::Overflow)?;

					let total_issuance = T::MultiCurrency::total_issuance(currency_id);
					let right: FixedU128 = FixedU128::checked_from_integer(locked_token.amount)
						.and_then(|x| x.checked_mul(&markup_coefficient.markup_coefficient))
						.and_then(|x| {
							x.checked_div(&FixedU128::checked_from_integer(total_issuance)?)
						})
						.ok_or(ArithmeticError::Overflow)?;
					let currency_id_markup_coefficient: FixedU128 =
						left.checked_add(&right).ok_or(ArithmeticError::Overflow)?;

					let mut user_markup_info =
						UserMarkupInfos::<T>::get(&who).ok_or(Error::<T>::LockNotExist)?;

					let new_markup_coefficient =
						match markup_coefficient.hardcap.cmp(&currency_id_markup_coefficient) {
							Ordering::Less => markup_coefficient.hardcap,
							Ordering::Equal | Ordering::Greater => currency_id_markup_coefficient,
						};
					Self::update_markup_info(
						&who,
						user_markup_info
							.markup_coefficient
							.saturating_sub(locked_token.markup_coefficient)
							.saturating_add(new_markup_coefficient),
						&mut user_markup_info,
					);
					locked_token.markup_coefficient = new_markup_coefficient;
					LockedTokens::<T>::insert(&currency_id, &who, locked_token);
					UserPositions::<T>::get(&who).into_iter().try_for_each(
						|position| -> DispatchResult {
							let _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> =
								Locked::<T>::get(position);
							ensure!(!_locked.amount.is_zero(), Error::<T>::ArgumentsError); // TODO
							Self::markup_calc(
								&who,
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

			if all_refreshed {
				Self::deposit_event(Event::AllRefreshed { currency_id });
			} else {
				Self::deposit_event(Event::PartiallyRefreshed { currency_id });
			}
			Ok(())
		}

		/// Withdraw vBNC by position
		///
		/// # Arguments
		///
		/// * `who` - the user of the position
		/// * `position` - the ID of the position
		/// * `_locked` - user locked variable representation
		/// * `if_fast` - distinguish whether it is a fast withdraw

		pub fn withdraw_no_ensure(
			who: &AccountIdOf<T>,
			position: u128,
			mut _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>>,
			if_fast: Option<FixedU128>,
		) -> DispatchResult {
			let value = _locked.amount;
			let old_locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> = _locked.clone();
			_locked.end = Zero::zero();
			_locked.amount = Zero::zero();
			Locked::<T>::insert(position, _locked.clone());

			let supply_before = Supply::<T>::get();
			let supply_after = supply_before.checked_sub(value).ok_or(ArithmeticError::Overflow)?;
			Supply::<T>::set(supply_after);

			// BNC should be transferred before checkpoint
			UserPositions::<T>::mutate(who, |positions| {
				positions.retain(|&x| x != position);
			});
			UserPointEpoch::<T>::remove(position);
			let new_locked_balance =
				UserLocked::<T>::get(who).checked_sub(value).ok_or(ArithmeticError::Underflow)?;
			Self::set_ve_locked(who, new_locked_balance)?;
			if let Some(fast) = if_fast {
				if fast != FixedU128::zero() {
					T::MultiCurrency::transfer(
						T::TokenType::get(),
						who,
						&T::BuyBackAccount::get().into_account_truncating(),
						fast.checked_mul_int(value).ok_or(ArithmeticError::Overflow)?,
					)?;
				}
			}

			Self::_checkpoint(who, position, old_locked, _locked.clone())?;

			Self::deposit_event(Event::Withdrawn { who: who.clone(), position, value });
			Self::deposit_event(Event::Supply { supply_before, supply: supply_after });
			Ok(())
		}

		fn redeem_commission(
			remaining_blocks: BlockNumberFor<T>,
		) -> Result<FixedU128, ArithmeticError> {
			FixedU128::checked_from_integer(remaining_blocks.saturated_into::<u128>())
				.and_then(|x| {
					x.checked_add(&FixedU128::checked_from_integer(
						T::Week::get().saturated_into::<u128>().checked_mul(52)?,
					)?)
				}) // one years
				.and_then(|x| {
					x.checked_div(&FixedU128::checked_from_integer(
						T::Week::get().saturated_into::<u128>().checked_mul(208)?,
					)?)
				}) // four years
				.and_then(|x| Some(x.saturating_pow(2)))
				.ok_or(ArithmeticError::Overflow)
		}

		/// This function will check the lock and redeem it regardless of whether it has expired.
		pub fn redeem_unlock_inner(who: &AccountIdOf<T>, position: u128) -> DispatchResult {
			let mut _locked = Locked::<T>::get(position);
			let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
			ensure!(_locked.end > current_block_number, Error::<T>::Expired);
			let fast = Self::redeem_commission(_locked.end - current_block_number)?;
			Self::withdraw_no_ensure(who, position, _locked, Some(fast))
		}

		fn set_ve_locked(who: &AccountIdOf<T>, new_locked_balance: BalanceOf<T>) -> DispatchResult {
			match new_locked_balance {
				0 => {
					// Can not set lock to zero, should remove it.
					T::MultiCurrency::remove_lock(BB_LOCK_ID, T::TokenType::get(), who)?;
				},
				_ => {
					T::MultiCurrency::set_lock(
						BB_LOCK_ID,
						T::TokenType::get(),
						who,
						new_locked_balance,
					)?;
				},
			};
			UserLocked::<T>::set(who, new_locked_balance);
			Ok(())
		}
	}
}

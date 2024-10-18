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

use bifrost_primitives::PoolId;

// Ensure we're `no_std` when compiling for Wasm.
use crate::*;

pub trait BbBNCInterface<AccountId, CurrencyId, Balance, BlockNumber> {
	fn deposit_for(_who: &AccountId, position: u128, value: Balance) -> DispatchResult;
	fn withdraw_inner(who: &AccountId, position: u128) -> DispatchResult;
	fn balance_of(who: &AccountId, time: Option<BlockNumber>) -> Result<Balance, DispatchError>;
	fn total_supply(t: BlockNumber) -> Result<Balance, DispatchError>;
	fn supply_at(
		point: Point<Balance, BlockNumber>,
		t: BlockNumber,
	) -> Result<Balance, DispatchError>;
	fn find_block_epoch(_block: BlockNumber, max_epoch: U256) -> U256;
	fn create_lock_inner(
		who: &AccountId,
		_value: Balance,
		_unlock_time: BlockNumber,
	) -> DispatchResult; // Deposit `_value` BNC for `who` and lock until `_unlock_time`
	fn increase_amount_inner(who: &AccountId, position: u128, value: Balance) -> DispatchResult; // Deposit `_value` additional BNC for `who` without modifying the unlock time
	fn increase_unlock_time_inner(
		who: &AccountId,
		position: u128,
		_unlock_time: BlockNumber,
	) -> DispatchResult; // Extend the unlock time for `who` to `_unlock_time`
	fn auto_notify_reward(
		pool_id: PoolId,
		n: BlockNumber,
		rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult;
	fn update_reward(
		pool_id: PoolId,
		who: Option<&AccountId>,
		share_info: Option<(Balance, Balance)>,
	) -> DispatchResult;
	fn get_rewards(
		pool_id: PoolId,
		who: &AccountId,
		share_info: Option<(Balance, Balance)>,
	) -> DispatchResult;
	fn set_incentive(
		pool_id: PoolId,
		rewards_duration: Option<BlockNumber>,
		controller: Option<AccountId>,
	);
	fn add_reward(
		who: &AccountId,
		conf: &mut IncentiveConfig<CurrencyId, Balance, BlockNumber, AccountId>,
		rewards: &Vec<(CurrencyId, Balance)>,
		remaining: Balance,
	) -> DispatchResult;
	fn notify_reward(
		pool_id: PoolId,
		who: &Option<AccountId>,
		rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult;
}

impl<T: Config> BbBNCInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>
	for Pallet<T>
{
	fn create_lock_inner(
		who: &AccountIdOf<T>,
		_value: BalanceOf<T>,
		_unlock_time: BlockNumberFor<T>,
	) -> DispatchResult {
		let new_position = Position::<T>::get();
		let mut user_positions = UserPositions::<T>::get(who);
		user_positions
			.try_push(new_position)
			.map_err(|_| Error::<T>::ExceedsMaxPositions)?;
		UserPositions::<T>::insert(who, user_positions);
		Position::<T>::set(new_position + 1);

		let bb_config = BbConfigs::<T>::get();
		ensure!(_value >= bb_config.min_mint, Error::<T>::BelowMinimumMint);

		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		let _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> =
			Locked::<T>::get(new_position);
		let unlock_time: BlockNumberFor<T> = _unlock_time
			.saturating_add(current_block_number)
			.checked_div(&T::Week::get())
			.ok_or(ArithmeticError::Overflow)?
			.saturating_add(1u32.into())
			.checked_mul(&T::Week::get())
			.ok_or(ArithmeticError::Overflow)?;

		ensure!(
			unlock_time >= bb_config.min_block.saturating_add(current_block_number),
			Error::<T>::ArgumentsError
		);
		let max_block = T::MaxBlock::get()
			.saturating_add(current_block_number)
			.checked_div(&T::Week::get())
			.ok_or(ArithmeticError::Overflow)?
			.saturating_add(1u32.into())
			.checked_mul(&T::Week::get())
			.ok_or(ArithmeticError::Overflow)?;
		ensure!(unlock_time <= max_block, Error::<T>::ArgumentsError);
		ensure!(_locked.amount == BalanceOf::<T>::zero(), Error::<T>::LockExist); // Withdraw old tokens first

		Self::_deposit_for(who, new_position, _value, unlock_time, _locked)?;
		Self::deposit_event(Event::LockCreated {
			who: who.to_owned(),
			position: new_position,
			value: _value,
			unlock_time: _unlock_time,
		});
		Ok(())
	}

	fn increase_unlock_time_inner(
		who: &AccountIdOf<T>,
		position: u128,
		_unlock_time: BlockNumberFor<T>,
	) -> DispatchResult {
		let bb_config = BbConfigs::<T>::get();
		let _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> = Locked::<T>::get(position);
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();

		let unlock_time: BlockNumberFor<T> = _unlock_time
			.saturating_add(_locked.end)
			.checked_div(&T::Week::get())
			.ok_or(ArithmeticError::Overflow)?
			.saturating_add(1u32.into())
			.checked_mul(&T::Week::get())
			.ok_or(ArithmeticError::Overflow)?;

		ensure!(
			unlock_time >= bb_config.min_block.saturating_add(current_block_number),
			Error::<T>::ArgumentsError
		);
		let max_block = T::MaxBlock::get()
			.saturating_add(current_block_number)
			.checked_div(&T::Week::get())
			.ok_or(ArithmeticError::Overflow)?
			.saturating_add(1u32.into())
			.checked_mul(&T::Week::get())
			.ok_or(ArithmeticError::Overflow)?;
		ensure!(unlock_time <= max_block, Error::<T>::ArgumentsError);
		ensure!(_locked.amount > BalanceOf::<T>::zero(), Error::<T>::LockNotExist);
		ensure!(_locked.end > current_block_number, Error::<T>::Expired); // Cannot add to expired/non-existent lock

		Self::_deposit_for(who, position, BalanceOf::<T>::zero(), unlock_time, _locked)?;
		Self::deposit_event(Event::UnlockTimeIncreased {
			who: who.to_owned(),
			position,
			unlock_time,
		});
		Ok(())
	}

	fn increase_amount_inner(
		who: &AccountIdOf<T>,
		position: u128,
		value: BalanceOf<T>,
	) -> DispatchResult {
		let bb_config = BbConfigs::<T>::get();
		ensure!(value >= bb_config.min_mint, Error::<T>::BelowMinimumMint);
		let _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> = Locked::<T>::get(position);
		ensure!(_locked.amount > BalanceOf::<T>::zero(), Error::<T>::LockNotExist); // Need to be executed after create_lock
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		ensure!(_locked.end > current_block_number, Error::<T>::Expired); // Cannot add to expired/non-existent lock
		Self::_deposit_for(who, position, value, Zero::zero(), _locked)?;
		Self::deposit_event(Event::AmountIncreased { who: who.to_owned(), position, value });
		Ok(())
	}

	fn deposit_for(who: &AccountIdOf<T>, position: u128, value: BalanceOf<T>) -> DispatchResult {
		let _locked: LockedBalance<BalanceOf<T>, BlockNumberFor<T>> = Locked::<T>::get(position);
		Self::_deposit_for(who, position, value, Zero::zero(), _locked)
	}

	fn withdraw_inner(who: &AccountIdOf<T>, position: u128) -> DispatchResult {
		let mut _locked = Locked::<T>::get(position);
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		ensure!(current_block_number >= _locked.end, Error::<T>::Expired);
		Self::withdraw_no_ensure(who, position, _locked, None)
	}

	fn balance_of(
		who: &AccountIdOf<T>,
		time: Option<BlockNumberFor<T>>,
	) -> Result<BalanceOf<T>, DispatchError> {
		match time {
			Some(_t) => Self::balance_of_at(who, _t),
			None => Self::balance_of_current_block(who),
		}
	}

	fn find_block_epoch(_block: BlockNumberFor<T>, max_epoch: U256) -> U256 {
		let mut _min = U256::zero();
		let mut _max = max_epoch;
		for _i in 0..128 {
			if _min >= _max {
				break;
			}
			let _mid = (_min + _max + 1) / 2;

			if PointHistory::<T>::get(_mid).block <= _block {
				_min = _mid
			} else {
				_max = _mid - 1
			}
		}
		_min
	}

	fn total_supply(t: BlockNumberFor<T>) -> Result<BalanceOf<T>, DispatchError> {
		let g_epoch: U256 = Epoch::<T>::get();
		let last_point = PointHistory::<T>::get(g_epoch);
		Self::supply_at(last_point, t)
	}

	fn supply_at(
		point: Point<BalanceOf<T>, BlockNumberFor<T>>,
		t: BlockNumberFor<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let mut last_point = point;
		let mut t_i: BlockNumberFor<T> = last_point
			.block
			.checked_div(&T::Week::get())
			.ok_or(ArithmeticError::Overflow)?
			.checked_mul(&T::Week::get())
			.ok_or(ArithmeticError::Overflow)?;
		for _i in 0..255 {
			t_i += T::Week::get();
			let mut d_slope = Zero::zero();
			if t_i > t {
				t_i = t
			} else {
				d_slope = SlopeChanges::<T>::get(t_i)
			}

			last_point.bias = last_point
				.bias
				.checked_sub(
					last_point
						.slope
						.checked_mul(
							t_i.checked_sub(&last_point.block)
								.ok_or(ArithmeticError::Overflow)?
								.saturated_into::<u128>()
								.unique_saturated_into(),
						)
						.ok_or(ArithmeticError::Overflow)?,
				)
				.ok_or(ArithmeticError::Overflow)?;

			if t_i == t {
				break;
			}
			last_point.slope += d_slope;
			last_point.block = t_i
		}

		if last_point.bias < 0_i128 {
			last_point.bias = 0_i128
		}
		Ok(T::VoteWeightMultiplier::get()
			.checked_mul((last_point.bias as u128).unique_saturated_into())
			.ok_or(ArithmeticError::Overflow)?)
	}

	fn auto_notify_reward(
		pool_id: PoolId,
		n: BlockNumberFor<T>,
		rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
	) -> DispatchResult {
		let conf = IncentiveConfigs::<T>::get(pool_id);
		if n == conf.period_finish {
			Self::notify_reward_amount(pool_id, &conf.incentive_controller, rewards)?;
		}
		Ok(())
	}

	fn update_reward(
		pool_id: PoolId,
		who: Option<&AccountIdOf<T>>,
		share_info: Option<(BalanceOf<T>, BalanceOf<T>)>,
	) -> DispatchResult {
		Self::update_reward(pool_id, who, share_info)
	}

	fn get_rewards(
		pool_id: PoolId,
		who: &AccountIdOf<T>,
		share_info: Option<(BalanceOf<T>, BalanceOf<T>)>,
	) -> DispatchResult {
		Self::get_rewards_inner(pool_id, who, share_info)
	}

	fn set_incentive(
		pool_id: PoolId,
		rewards_duration: Option<BlockNumberFor<T>>,
		controller: Option<AccountIdOf<T>>,
	) {
		let mut incentive_config = IncentiveConfigs::<T>::get(pool_id);

		if let Some(rewards_duration) = rewards_duration {
			incentive_config.rewards_duration = rewards_duration;
		};
		if let Some(controller) = controller {
			incentive_config.incentive_controller = Some(controller.clone());
		}
		IncentiveConfigs::<T>::set(pool_id, incentive_config.clone());
		Self::deposit_event(Event::IncentiveSet { incentive_config });
	}

	fn add_reward(
		who: &AccountIdOf<T>,
		conf: &mut IncentiveConfig<
			CurrencyIdOf<T>,
			BalanceOf<T>,
			BlockNumberFor<T>,
			AccountIdOf<T>,
		>,
		rewards: &Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		remaining: BalanceOf<T>,
	) -> DispatchResult {
		rewards.iter().try_for_each(|(currency, reward)| -> DispatchResult {
			let mut total_reward: BalanceOf<T> = *reward;
			if remaining != BalanceOf::<T>::zero() {
				let leftover: BalanceOf<T> = conf
					.reward_rate
					.get(currency)
					.unwrap_or(&Zero::zero())
					.checked_mul(&remaining)
					.ok_or(ArithmeticError::Overflow)?;
				total_reward = total_reward.saturating_add(leftover);
			}
			let currency_amount = T::MultiCurrency::free_balance(
				*currency,
				&T::IncentivePalletId::get().into_account_truncating(),
			);
			// Make sure the new reward is less than or equal to the reward owned by the
			// IncentivePalletId
			ensure!(
				total_reward <= currency_amount.saturating_add(*reward),
				Error::<T>::NotEnoughBalance
			);
			let new_reward = total_reward
				.checked_div(T::BlockNumberToBalance::convert(conf.rewards_duration))
				.ok_or(ArithmeticError::Overflow)?;
			conf.reward_rate
				.entry(*currency)
				.and_modify(|total_reward| {
					*total_reward = new_reward;
				})
				.or_insert(new_reward);
			T::MultiCurrency::transfer(
				*currency,
				who,
				&T::IncentivePalletId::get().into_account_truncating(),
				*reward,
			)
		})
	}

	fn notify_reward(
		pool_id: PoolId,
		who: &Option<AccountIdOf<T>>,
		rewards: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
	) -> DispatchResult {
		Self::notify_reward_amount(pool_id, who, rewards)
	}
}

impl<AccountId, CurrencyId, Balance, BlockNumber>
	BbBNCInterface<AccountId, CurrencyId, Balance, BlockNumber> for ()
where
	Balance: orml_traits::arithmetic::Zero,
{
	fn create_lock_inner(
		_who: &AccountId,
		_value: Balance,
		_unlock_time: BlockNumber,
	) -> DispatchResult {
		Ok(())
	}

	fn increase_unlock_time_inner(
		_who: &AccountId,
		_position: u128,
		_unlock_time: BlockNumber,
	) -> DispatchResult {
		Ok(())
	}

	fn increase_amount_inner(_who: &AccountId, _position: u128, _value: Balance) -> DispatchResult {
		Ok(())
	}

	fn deposit_for(_who: &AccountId, _position: u128, _value: Balance) -> DispatchResult {
		Ok(())
	}

	fn withdraw_inner(_who: &AccountId, _position: u128) -> DispatchResult {
		Ok(())
	}

	fn balance_of(_who: &AccountId, _time: Option<BlockNumber>) -> Result<Balance, DispatchError> {
		Ok(Zero::zero())
	}

	fn find_block_epoch(_block: BlockNumber, _max_epoch: U256) -> U256 {
		U256::zero()
	}

	fn total_supply(_t: BlockNumber) -> Result<Balance, DispatchError> {
		Ok(Zero::zero())
	}

	fn supply_at(
		_point: Point<Balance, BlockNumber>,
		_t: BlockNumber,
	) -> Result<Balance, DispatchError> {
		Ok(Zero::zero())
	}

	fn auto_notify_reward(
		_pool_id: PoolId,
		_n: BlockNumber,
		_rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult {
		Ok(())
	}

	fn update_reward(
		_pool_id: PoolId,
		_who: Option<&AccountId>,
		_share_info: Option<(Balance, Balance)>,
	) -> DispatchResult {
		Ok(())
	}

	fn get_rewards(
		_pool_id: PoolId,
		_who: &AccountId,
		_share_info: Option<(Balance, Balance)>,
	) -> DispatchResult {
		Ok(())
	}

	fn set_incentive(
		_pool_id: PoolId,
		_rewards_duration: Option<BlockNumber>,
		_controller: Option<AccountId>,
	) {
	}
	fn add_reward(
		_who: &AccountId,
		_conf: &mut IncentiveConfig<CurrencyId, Balance, BlockNumber, AccountId>,
		_rewards: &Vec<(CurrencyId, Balance)>,
		_remaining: Balance,
	) -> DispatchResult {
		Ok(())
	}
	fn notify_reward(
		_pool_id: PoolId,
		_who: &Option<AccountId>,
		_rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult {
		Ok(())
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct UserMarkupInfo {
	// pub old_locked: LockedBalance<Balance, BlockNumber>,
	pub old_markup_coefficient: FixedU128,
	pub markup_coefficient: FixedU128,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct LockedToken<Balance, BlockNumber> {
	// pub currency_id: CurrencyId,
	pub amount: Balance,
	pub markup_coefficient: FixedU128,
	pub refresh_block: BlockNumber,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct MarkupCoefficientInfo<BlockNumber> {
	pub markup_coefficient: FixedU128,
	pub hardcap: FixedU128,
	pub update_block: BlockNumber,
}

pub trait MarkupInfo<AccountId> {
	fn update_markup_info(
		who: &AccountId,
		new_markup_coefficient: FixedU128,
		user_markup_info: &mut UserMarkupInfo,
	);
}

impl<T: Config> MarkupInfo<AccountIdOf<T>> for Pallet<T> {
	fn update_markup_info(
		who: &AccountIdOf<T>,
		new_markup_coefficient: FixedU128,
		user_markup_info: &mut UserMarkupInfo,
	) {
		user_markup_info.old_markup_coefficient = user_markup_info.markup_coefficient;
		user_markup_info.markup_coefficient = new_markup_coefficient;
		UserMarkupInfos::<T>::insert(who, user_markup_info);
	}
}

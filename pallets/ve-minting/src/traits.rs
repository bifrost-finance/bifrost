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
use crate::*;

pub trait VeMintingInterface<AccountId, CurrencyId, Balance, BlockNumber> {
	fn deposit_for(addr: &AccountId, value: Balance) -> DispatchResult;
	fn _withdraw(addr: &AccountId) -> DispatchResult;
	fn balance_of(addr: &AccountId, time: Option<BlockNumber>) -> Result<Balance, DispatchError>;
	fn balance_of_at(addr: &AccountId, block: BlockNumber) -> Result<Balance, DispatchError>;
	fn total_supply(t: BlockNumber) -> Result<Balance, DispatchError>;
	fn supply_at(
		point: Point<Balance, BlockNumber>,
		t: BlockNumber,
	) -> Result<Balance, DispatchError>;
	fn find_block_epoch(_block: BlockNumber, max_epoch: U256) -> U256;
	fn _create_lock(addr: &AccountId, _value: Balance, _unlock_time: BlockNumber)
		-> DispatchResult; // Deposit `_value` BNC for `addr` and lock until `_unlock_time`
	fn _increase_amount(addr: &AccountId, value: Balance) -> DispatchResult; // Deposit `_value` additional BNC for `addr` without modifying the unlock time
	fn _increase_unlock_time(addr: &AccountId, _unlock_time: BlockNumber) -> DispatchResult; // Extend the unlock time for `addr` to `_unlock_time`
}

impl<T: Config> VeMintingInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>, T::BlockNumber>
	for Pallet<T>
{
	fn _create_lock(
		addr: &AccountIdOf<T>,
		_value: BalanceOf<T>,
		_unlock_time: T::BlockNumber,
	) -> DispatchResult {
		let ve_config = Self::ve_configs();
		let _locked: LockedBalance<BalanceOf<T>, T::BlockNumber> = Self::locked(addr);
		let unlock_time: T::BlockNumber = (_unlock_time / ve_config.week) * ve_config.week;

		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number().into();
		ensure!(
			unlock_time > ve_config.min_time.saturating_add(current_block_number),
			Error::<T>::Expired
		);
		ensure!(
			unlock_time <= ve_config.max_time.saturating_add(current_block_number),
			Error::<T>::Expired
		);
		ensure!(_locked.amount == BalanceOf::<T>::zero(), Error::<T>::LockExist); // Withdraw old tokens first
		ensure!(_value >= ve_config.min_mint, Error::<T>::NotEnoughBalance); // need non-zero value

		Self::_deposit_for(addr, _value, unlock_time, _locked)
	}

	fn _increase_unlock_time(
		addr: &AccountIdOf<T>,
		_unlock_time: T::BlockNumber,
	) -> DispatchResult {
		let ve_config = Self::ve_configs();
		let _locked: LockedBalance<BalanceOf<T>, T::BlockNumber> = Self::locked(addr);
		let unlock_time: T::BlockNumber = (_unlock_time / ve_config.week) * ve_config.week;

		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number().into();
		ensure!(_locked.end > current_block_number, Error::<T>::Expired);
		ensure!(unlock_time >= ve_config.min_time.saturating_add(_locked.end), Error::<T>::Expired);
		ensure!(
			unlock_time <= ve_config.max_time.saturating_add(current_block_number),
			Error::<T>::Expired
		);
		ensure!(_locked.amount > BalanceOf::<T>::zero(), Error::<T>::LockNotExist);

		Self::_deposit_for(addr, BalanceOf::<T>::zero(), unlock_time, _locked)
	}

	fn _increase_amount(addr: &AccountIdOf<T>, value: BalanceOf<T>) -> DispatchResult {
		ensure!(value > Zero::zero(), Error::<T>::NotEnoughBalance);
		let _locked: LockedBalance<BalanceOf<T>, T::BlockNumber> = Self::locked(addr);
		ensure!(_locked.amount > Zero::zero(), Error::<T>::LockNotExist); // Need to be executed after create_lock
		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number().into();
		ensure!(_locked.end > current_block_number, Error::<T>::Expired); // Cannot add to expired lock
		Self::_deposit_for(addr, value, 0u32.unique_saturated_into(), _locked)
	}

	fn deposit_for(addr: &AccountIdOf<T>, value: BalanceOf<T>) -> DispatchResult {
		let _locked: LockedBalance<BalanceOf<T>, T::BlockNumber> = Self::locked(addr);
		Self::_deposit_for(addr, value, 0u32.unique_saturated_into(), _locked)
	}

	fn _withdraw(addr: &AccountIdOf<T>) -> DispatchResult {
		let mut _locked = Self::locked(addr);
		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number();
		ensure!(current_block_number >= _locked.end, Error::<T>::Expired);
		let value = _locked.amount;
		let old_locked: LockedBalance<BalanceOf<T>, T::BlockNumber> = _locked.clone();
		_locked.end = Zero::zero();
		_locked.amount = Zero::zero();
		Locked::<T>::insert(addr, _locked.clone());

		let supply_before = Self::supply();
		Supply::<T>::set(supply_before.saturating_sub(value));

		// BNC should be transferred before checkpoint
		T::MultiCurrency::transfer(
			BNC,
			&T::VeMintingPalletId::get().into_account_truncating(),
			addr,
			value,
		)?;

		Self::_checkpoint(addr, old_locked, _locked.clone())?;

		Self::deposit_event(Event::Supply {
			supply_before,
			supply: supply_before.saturating_sub(value),
		});
		Ok(())
	}

	fn balance_of(
		addr: &AccountIdOf<T>,
		time: Option<T::BlockNumber>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let _t = match time {
			Some(_t) => _t,
			None => frame_system::Pallet::<T>::block_number(),
		};
		let u_epoch = Self::user_point_epoch(addr);
		if u_epoch == U256::zero() {
			return Ok(Zero::zero());
		} else {
			let mut last_point: Point<BalanceOf<T>, T::BlockNumber> =
				Self::user_point_history(addr, u_epoch);

			log::debug!(
				"balance_of---:{:?}_t:{:?}last_point.ts:{:?}",
				(_t.saturated_into::<u128>() as i128)
					.saturating_sub(last_point.ts.saturated_into::<u128>() as i128),
				_t,
				last_point.ts
			);
			last_point.bias = last_point
				.bias
				.checked_sub(
					last_point
						.slope
						.checked_mul(
							(_t.saturated_into::<u128>() as i128)
								.checked_sub(last_point.ts.saturated_into::<u128>() as i128)
								.ok_or(ArithmeticError::Overflow)?,
						)
						.ok_or(ArithmeticError::Overflow)?,
				)
				.ok_or(ArithmeticError::Overflow)?;

			if last_point.bias < 0_i128 {
				last_point.bias = 0_i128
			}

			Ok(last_point
				.amt
				.checked_add(
					&Self::ve_configs()
						.vote_weight_multiplier
						.checked_mul(&(last_point.bias as u128).unique_saturated_into())
						.ok_or(ArithmeticError::Overflow)?,
				)
				.ok_or(ArithmeticError::Overflow)?)
		}
	}

	fn balance_of_at(
		addr: &AccountIdOf<T>,
		_block: T::BlockNumber,
	) -> Result<BalanceOf<T>, DispatchError> {
		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number().into();
		ensure!(_block <= current_block_number, Error::<T>::Expired);
		let current_block_number: T::BlockNumber = frame_system::Pallet::<T>::block_number().into();

		// Binary search
		let mut _min = U256::zero();
		let mut _max = Self::user_point_epoch(addr);
		for _i in 0..128 {
			if _min >= _max {
				break;
			}
			let _mid = (_min + _max + 1) / 2;

			if Self::user_point_history(addr, _mid).blk <= _block {
				_min = _mid
			} else {
				_max = _mid - 1
			}
		}

		let mut upoint: Point<BalanceOf<T>, T::BlockNumber> = Self::user_point_history(addr, _min);

		let max_epoch: U256 = Self::epoch();
		let _epoch: U256 = Self::find_block_epoch(_block, max_epoch);
		let point_0: Point<BalanceOf<T>, T::BlockNumber> = Self::point_history(_epoch);
		let d_block;
		let d_t;
		if _epoch < max_epoch {
			let point_1 = Self::point_history(_epoch + 1);
			d_block = point_1.blk - point_0.blk;
			d_t = point_1.ts - point_0.ts;
		} else {
			d_block = current_block_number - point_0.blk;
			d_t = current_block_number - point_0.ts;
		}

		let mut block_time = point_0.ts;
		if d_block != Zero::zero() {
			block_time += (d_t.saturating_mul(_block - point_0.blk))
				.saturated_into::<u128>()
				.saturating_div(d_block.saturated_into::<u128>())
				.unique_saturated_into();
		}
		upoint.bias -= upoint
			.bias
			.checked_sub(
				upoint
					.slope
					.checked_mul(
						(block_time.saturated_into::<u128>() as i128) -
							(upoint.ts.saturated_into::<u128>() as i128),
					)
					.ok_or(ArithmeticError::Overflow)?,
			)
			.ok_or(ArithmeticError::Overflow)?;

		if (upoint.bias >= 0_i128) || (upoint.amt >= Zero::zero()) {
			Ok(upoint.amt +
				(Self::ve_configs().vote_weight_multiplier *
					(upoint.bias as u128).unique_saturated_into()))
		} else {
			Ok(Zero::zero())
		}
	}

	fn find_block_epoch(_block: T::BlockNumber, max_epoch: U256) -> U256 {
		let mut _min = U256::zero();
		let mut _max = max_epoch;
		for _i in 0..128 {
			if _min >= _max {
				break;
			}
			let _mid = (_min + _max + 1) / 2;

			if Self::point_history(_mid).blk <= _block {
				_min = _mid
			} else {
				_max = _mid - 1
			}
		}
		_min
	}

	fn total_supply(t: T::BlockNumber) -> Result<BalanceOf<T>, DispatchError> {
		let g_epoch: U256 = Self::epoch();
		let last_point = Self::point_history(g_epoch);
		Self::supply_at(last_point, t)
	}

	fn supply_at(
		point: Point<BalanceOf<T>, T::BlockNumber>,
		t: T::BlockNumber,
	) -> Result<BalanceOf<T>, DispatchError> {
		let ve_config = Self::ve_configs();

		let mut last_point = point;
		let mut t_i: T::BlockNumber = (last_point.ts / ve_config.week) * ve_config.week;
		for _i in 0..255 {
			t_i += ve_config.week;
			let mut d_slope = Zero::zero();
			if t_i > t {
				t_i = t
			} else {
				d_slope = Self::slope_changes(t_i)
			}

			last_point.bias = last_point
				.bias
				.checked_sub(
					last_point
						.slope
						.checked_mul(
							t_i.checked_sub(&last_point.ts)
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
			last_point.ts = t_i
		}

		if last_point.bias < 0_i128 {
			last_point.bias = 0_i128
		}
		Ok(last_point
			.amt
			.checked_add(
				&Self::ve_configs()
					.vote_weight_multiplier
					.checked_mul(&(last_point.bias as u128).unique_saturated_into())
					.ok_or(ArithmeticError::Overflow)?,
			)
			.ok_or(ArithmeticError::Overflow)?)
	}
}

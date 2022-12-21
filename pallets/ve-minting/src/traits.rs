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
// use node_primitives::BlockNumber;

pub trait VeMintingInterface<AccountId, CurrencyId, Balance, BlockNumber> {
	fn deposit_for(addr: &AccountId, value: Balance) -> DispatchResult;
	fn _withdraw(addr: &AccountId) -> DispatchResult;
	fn balance_of(addr: &AccountId, time: Option<Timestamp>) -> Result<Balance, DispatchError>;
	fn balance_of_at(addr: &AccountId, block: BlockNumber) -> Result<Balance, DispatchError>;
	fn total_supply(t: Timestamp) -> Balance;
	fn supply_at(point: Point<Balance, BlockNumber>, t: Timestamp) -> Balance;
	fn find_block_epoch(_block: BlockNumber, max_epoch: U256) -> U256;
	fn _create_lock(addr: &AccountId, _value: Balance, _unlock_time: Timestamp) -> DispatchResult; // Deposit `_value` BNC for `addr` and lock until `_unlock_time`
	fn _increase_amount(addr: &AccountId, value: Balance) -> DispatchResult; // Deposit `_value` additional BNC for `addr` without modifying the unlock time
	fn _increase_unlock_time(addr: &AccountId, _unlock_time: Timestamp) -> DispatchResult; // Extend the unlock time for `addr` to `_unlock_time`
}

impl<T: Config> VeMintingInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>
	for Pallet<T>
{
	fn _create_lock(
		addr: &AccountIdOf<T>,
		_value: BalanceOf<T>,
		_unlock_time: Timestamp,
	) -> DispatchResult {
		let ve_config = Self::ve_configs();
		let _locked: LockedBalance<BalanceOf<T>> = Self::locked(addr);
		let unlock_time: Timestamp = (_unlock_time / ve_config.week) * ve_config.week;

		let current_timestamp: Timestamp =
			sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
		ensure!(
			unlock_time > ve_config.min_time.saturating_add(current_timestamp),
			Error::<T>::Expired
		);
		ensure!(
			unlock_time <= ve_config.max_time.saturating_add(current_timestamp),
			Error::<T>::Expired
		);
		ensure!(_locked.amount == BalanceOf::<T>::zero(), Error::<T>::Expired); // Withdraw old tokens first
		ensure!(_value >= ve_config.min_mint, Error::<T>::Expired); // need non-zero value

		Self::_deposit_for(addr, _value, unlock_time, _locked)
	}

	fn _increase_unlock_time(addr: &AccountIdOf<T>, _unlock_time: Timestamp) -> DispatchResult {
		let ve_config = Self::ve_configs();
		let _locked: LockedBalance<BalanceOf<T>> = Self::locked(addr);
		let unlock_time: Timestamp = (_unlock_time / ve_config.week) * ve_config.week;

		let current_timestamp: Timestamp =
			sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
		ensure!(_locked.end > current_timestamp, Error::<T>::Expired);
		ensure!(unlock_time >= ve_config.min_time.saturating_add(_locked.end), Error::<T>::Expired);
		ensure!(
			unlock_time <= ve_config.max_time.saturating_add(current_timestamp),
			Error::<T>::Expired
		);
		ensure!(_locked.amount > BalanceOf::<T>::zero(), Error::<T>::Expired); // Withdraw old tokens first

		Self::_deposit_for(addr, BalanceOf::<T>::zero(), unlock_time, _locked)
	}

	fn _increase_amount(addr: &AccountIdOf<T>, value: BalanceOf<T>) -> DispatchResult {
		let _locked: LockedBalance<BalanceOf<T>> = Self::locked(addr);
		Self::_deposit_for(addr, value, 0, _locked)
	}

	fn deposit_for(addr: &AccountIdOf<T>, value: BalanceOf<T>) -> DispatchResult {
		let _locked: LockedBalance<BalanceOf<T>> = Self::locked(addr);
		Self::_deposit_for(addr, value, 0, _locked)
	}

	fn _withdraw(addr: &AccountIdOf<T>) -> DispatchResult {
		let mut _locked = Self::locked(addr);
		let current_timestamp: Timestamp =
			sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
		ensure!(current_timestamp > _locked.end, Error::<T>::Expired);
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

	fn balance_of(
		addr: &AccountIdOf<T>,
		time: Option<Timestamp>,
	) -> Result<BalanceOf<T>, DispatchError> {
		// if let Some(_t) = time {
		// 	_t
		// } else {
		// 	sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis()
		// };
		let _t = match time {
			Some(_t) => _t,
			None => sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis(),
		};
		// let _t: Timestamp =
		// 	sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
		let u_epoch = Self::user_point_epoch(addr);
		if u_epoch == U256::zero() {
			return Ok(Zero::zero());
		} else {
			let mut last_point: Point<BalanceOf<T>, BlockNumberFor<T>> =
				Self::user_point_history(addr, u_epoch);
			// log::debug!("{:?}::{:?}::{:?}", _t, last_point.ts, last_point.bias);

			last_point.bias -= last_point
				.slope
				.saturating_mul((_t.saturating_sub(last_point.ts)).saturated_into());
			// .ok_or(ArithmeticError::Overflow)?;
			if last_point.bias < Zero::zero() {
				last_point.bias = Zero::zero();
			}
			let ve_config = Self::ve_configs();
			Ok(last_point.fxs_amt + (Self::ve_configs().vote_weight_multiplier * last_point.bias))
		}
	}

	fn balance_of_at(
		addr: &AccountIdOf<T>,
		_block: BlockNumberFor<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let current_block_number: BlockNumberFor<T> =
			frame_system::Pallet::<T>::block_number().into();
		ensure!(_block <= current_block_number, Error::<T>::Expired);
		let current_timestamp: Timestamp =
			sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();

		// Binary search
		let mut _min = U256::zero();
		let mut _max = Self::user_point_epoch(addr);
		for i in 0..128 {
			if _min >= _max {
				break;
			}
			let _mid = (_min + _max + 1) / 2;
			// let mut last_point: Point<BalanceOf<T>, BlockNumberFor<T>> =
			// 	Self::user_point_history(addr, _mid);

			if Self::user_point_history(addr, _mid).blk <= _block {
				_min = _mid
			} else {
				_max = _mid - 1
			}
		}

		let mut upoint: Point<BalanceOf<T>, BlockNumberFor<T>> =
			Self::user_point_history(addr, _min);

		let max_epoch: U256 = Self::epoch();
		let _epoch: U256 = Self::find_block_epoch(_block, max_epoch);
		let point_0: Point<BalanceOf<T>, BlockNumberFor<T>> = Self::point_history(_epoch);
		let mut d_block = Zero::zero();
		let mut d_t = Zero::zero();
		if _epoch < max_epoch {
			let point_1 = Self::point_history(_epoch + 1);
			d_block = point_1.blk - point_0.blk;
			d_t = point_1.ts - point_0.ts;
		} else {
			d_block = current_block_number - point_0.blk;
			d_t = current_timestamp - point_0.ts;
		}

		let mut block_time = point_0.ts;
		if d_block != Zero::zero() {
			block_time += d_t
				.saturating_mul((_block - point_0.blk).saturated_into())
				.saturating_div(d_block.saturated_into());
			// (_block - point_0.blk) / d_block
		}
		upoint.bias -= upoint.slope.saturating_mul((block_time - upoint.ts).saturated_into()); //  * (block_time - upoint.ts);

		if (upoint.bias >= Zero::zero()) || (upoint.fxs_amt >= Zero::zero()) {
			Ok(upoint.fxs_amt + (Self::ve_configs().vote_weight_multiplier * upoint.bias))
		} else {
			Ok(Zero::zero())
		}
	}

	fn find_block_epoch(_block: BlockNumberFor<T>, max_epoch: U256) -> U256 {
		let mut _min = U256::zero();
		let mut _max = max_epoch;
		for i in 0..128 {
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

	fn total_supply(t: Timestamp) -> BalanceOf<T> {
		let g_epoch: U256 = Self::epoch();
		let last_point = Self::point_history(g_epoch);
		Self::supply_at(last_point, t)
	}

	fn supply_at(point: Point<BalanceOf<T>, BlockNumberFor<T>>, t: Timestamp) -> BalanceOf<T> {
		let ve_config = Self::ve_configs();

		let mut last_point = point;
		let mut t_i: Timestamp = (last_point.ts / ve_config.week) * ve_config.week;
		for i in 0..255 {
			t_i += ve_config.week;
			let mut d_slope = Zero::zero();
			if t_i > t {
				t_i = t
			} else {
				d_slope = Self::slope_changes(t_i)
			}

			last_point.bias = U256::from(last_point.bias.saturated_into::<u128>())
				.checked_sub(
					U256::from(last_point.slope.saturated_into::<u128>())
						.saturating_mul(U256::from((t_i - last_point.ts).saturated_into::<u128>())),
				)
				.unwrap_or_default()
				.as_u128()
				.unique_saturated_into();

			if t_i == t {
				break;
			}
			last_point.slope += (d_slope as u128).saturated_into();
			last_point.ts = t_i
		}

		if last_point.bias < Zero::zero() {
			last_point.bias = Zero::zero()
		}
		last_point.fxs_amt + Self::ve_configs().vote_weight_multiplier * last_point.bias
		// last_point.bias
	}
}

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

pub trait VeMintingInterface<AccountId, CurrencyId, Balance> {
	fn deposit_for(addr: &AccountIdOf<T>, value: BalanceOf<T>) -> DispatchResult {}
	fn withdraw(addr: &AccountIdOf<T>) -> DispatchResult {}
	fn balanceOf(addr: &AccountIdOf<T>, _t: Timestamp) -> BalanceOf<T> {}
	fn balanceOfAt(addr: &AccountIdOf<T>, block: BlockNumberFor<T>) -> BalanceOf<T> {}
	fn totalSupply(t: Timestamp) -> BalanceOf<T> {}
	fn supply_at(point: Point<BalanceOf<T>, BlockNumberFor<T>>, t: Timestamp) -> BalanceOf<T> {}
}

// impl<T: Config> VeMintingInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>> for Pallet<T>
// {}

impl<T: Config> VeMintingInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn deposit_for(addr: &AccountIdOf<T>, value: BalanceOf<T>) -> DispatchResult {
		let _locked: LockedBalance<BalanceOf<T>> = Self::locked(addr);
		Self::_deposit_for(addr, value, 0, _locked)
	}

	fn withdraw(addr: &AccountIdOf<T>) -> DispatchResult {
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

	fn balanceOf(addr: &AccountIdOf<T>, _t: Timestamp) -> BalanceOf<T> {
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

	fn balanceOfAt(addr: &AccountIdOf<T>, _block: BlockNumberFor<T>) -> BalanceOf<T> {
		let current_block_number: BlockNumberFor<T> =
			frame_system::Pallet::<T>::block_number().into();
		ensure!(_block < current_block_number, Error::<T>::NotExpire);
		let current_timestamp: Timestamp =
			sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();

		// Binary search
		let mut _min = Zero::zero();
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
		let point_0 = Self::point_history(_epoch);
		let d_block = Zero::zero();
		let d_t = Zero::zero();
		if _epoch < max_epoch {
			let point_1 = Self::point_history(_epoch + 1);
			d_block = point_1.blk - point_0.blk;
			d_t = point_1.ts - point_0.ts;
		} else {
			d_block = current_block_number - point_0.blk;
			d_t = current_timestamp - point_0.ts;
		}

		let block_time = point_0.ts;
		if d_block != 0 {
			block_time += d_t * (_block - point_0.blk) / d_block
		}
		upoint.bias -= upoint.slope * (block_time - upoint.ts);
		upoint.bias
	}

	fn find_block_epoch(_block: BlockNumberFor<T>, max_epoch: U256) -> U256 {
		let mut _min = Zero::zero();
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

	fn totalSupply(t: Timestamp) -> BalanceOf<T> {
		let g_epoch: U256 = Self::epoch();
		let last_point = Self::point_history(g_epoch);
		supply_at(last_point, t)
	}

	fn supply_at(point: Point<BalanceOf<T>, BlockNumberFor<T>>, t: Timestamp) -> BalanceOf<T> {
		let mut last_point = point;
		let mut t_i: Timestamp = (last_point.ts / ve_config.WEEK) * ve_config.WEEK;
		for i in 0..255 {
			t_i += ve_config.WEEK;
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
			last_point.slope += d_slope;
			last_point.ts = t_i
		}

		if last_point.bias < Zero::zero() {
			last_point.bias = Zero::zero()
		}
		last_point.bias
	}
}

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

use codec::{FullCodec, HasCompact};
use frame_support::pallet_prelude::*;
use node_primitives::{BlockNumber, CurrencyId};
use orml_traits::RewardHandler;
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime::{
	traits::{
		AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, Saturating, UniqueSaturatedInto,
		Zero, *,
	},
	ArithmeticError, FixedPointOperand, RuntimeDebug, SaturatedConversion,
};
use sp_std::{borrow::ToOwned, collections::btree_map::BTreeMap, fmt::Debug, prelude::*};

use crate::*;

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct GaugePoolInfo<BalanceOf: HasCompact, CurrencyIdOf: Ord, BlockNumberFor> {
	pid: PoolId,
	token: CurrencyIdOf,
	gauge_amount: BalanceOf,
	gauge_time_factor: u128,
	gauge_state: GaugeState,
	gauge_start_block: BlockNumberFor,
	gauge_last_block: BlockNumberFor,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub enum GaugeState {
	Unbond,
	Bonded,
}

impl<BalanceOf, CurrencyIdOf, BlockNumberFor> Default
	for GaugePoolInfo<BalanceOf, CurrencyIdOf, BlockNumberFor>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord + Default,
	BlockNumberFor: Default,
{
	fn default() -> Self {
		Self {
			pid: Default::default(),
			token: Default::default(),
			gauge_amount: Default::default(),
			gauge_time_factor: Default::default(),
			gauge_start_block: Default::default(),
			gauge_last_block: Default::default(),
			gauge_state: GaugeState::Unbond,
		}
	}
}

impl<BalanceOf, CurrencyIdOf, BlockNumberFor> GaugePoolInfo<BalanceOf, CurrencyIdOf, BlockNumberFor>
where
	BalanceOf: Default + HasCompact,
	CurrencyIdOf: Ord + Default,
	BlockNumberFor: Clone,
{
	fn new(pid: PoolId, token: CurrencyIdOf, current_block_number: BlockNumberFor) -> Self {
		Self {
			pid,
			token,
			gauge_amount: Default::default(),
			gauge_time_factor: Default::default(),
			gauge_start_block: current_block_number.clone(),
			gauge_last_block: current_block_number,
			gauge_state: GaugeState::Bonded,
		}
	}
}

impl<T: Config> Pallet<T>
where
	BlockNumberFor<T>: Into<u128>,
	BalanceOf<T>: Into<u128>,
{
	pub fn create_gauge_pool(
		pid: PoolId,
		pool_info: &mut PoolInfo<BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>>,
		gauge_token: CurrencyIdOf<T>,
	) -> DispatchResult {
		let gid = Self::gauge_pool_next_id();
		pool_info.gauge = Some(gid);
		let current_block_number = frame_system::Pallet::<T>::block_number();
		let mut gauge_pool_info = GaugePoolInfo::new(pid, gauge_token, current_block_number);

		GaugePoolInfos::<T>::insert(gid, &gauge_pool_info);
		GaugePoolNextId::<T>::mutate(|id| -> DispatchResult {
			*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}

	pub fn gauge_add(
		who: AccountIdOf<T>,
		pool: PoolId,
		gauge_value: BalanceOf<T>,
		gauge: PoolId,
	) -> DispatchResult {
		let current_block_number = frame_system::Pallet::<T>::block_number();
		GaugePoolInfos::<T>::mutate(gauge, |gauge_info| -> DispatchResult {
			let interval_block = current_block_number - gauge_info.gauge_last_block;
			let total_time_factor: u128 = interval_block
				.into()
				.checked_mul(gauge_info.gauge_amount.into())
				.ok_or(ArithmeticError::Overflow)?;

			gauge_info.gauge_last_block = current_block_number;
			gauge_info.gauge_time_factor = gauge_info
				.gauge_time_factor
				.checked_add(total_time_factor)
				.ok_or(ArithmeticError::Overflow)?;
			gauge_info.gauge_amount = gauge_info
				.gauge_amount
				.checked_add(&gauge_value)
				.ok_or(ArithmeticError::Overflow)?;
			SharesAndWithdrawnRewards::<T>::mutate(pool, who, |share_info| -> DispatchResult {
				let user_interval_block = current_block_number - share_info.gauge_last_block;
				// let time_factor: u128 = user_interval_block.into() *
				// share_info.gauge_amount.into();
				let time_factor: u128 = user_interval_block
					.into()
					.checked_mul(share_info.gauge_amount.into())
					.ok_or(ArithmeticError::Overflow)?;

				share_info.gauge_last_block = current_block_number;
				share_info.gauge_time_factor = share_info
					.gauge_time_factor
					.checked_add(time_factor)
					.ok_or(ArithmeticError::Overflow)?;
				share_info.gauge_amount = share_info
					.gauge_amount
					.checked_add(&gauge_value)
					.ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;
			Ok(())
		});
		Ok(())
	}

	pub fn gauge_remove(
		who: AccountIdOf<T>,
		pool: PoolId,
		gauge_value: BalanceOf<T>,
		gauge: PoolId,
	) -> DispatchResult {
		let current_block_number = frame_system::Pallet::<T>::block_number();
		GaugePoolInfos::<T>::mutate(gauge, |gauge_info| -> DispatchResult {
			let interval_block = current_block_number - gauge_info.gauge_last_block;
			let total_time_factor: u128 = interval_block
				.into()
				.checked_mul(gauge_info.gauge_amount.into())
				.ok_or(ArithmeticError::Overflow)?;

			gauge_info.gauge_last_block = current_block_number;
			gauge_info.gauge_time_factor = gauge_info
				.gauge_time_factor
				.checked_add(total_time_factor)
				.ok_or(ArithmeticError::Overflow)?;
			gauge_info.gauge_amount = gauge_info
				.gauge_amount
				.checked_sub(&gauge_value)
				.ok_or(ArithmeticError::Overflow)?;
			SharesAndWithdrawnRewards::<T>::mutate(pool, who, |share_info| -> DispatchResult {
				let user_interval_block = current_block_number - share_info.gauge_last_block;
				let time_factor: u128 = user_interval_block
					.into()
					.checked_mul(share_info.gauge_amount.into())
					.ok_or(ArithmeticError::Overflow)?;

				share_info.gauge_last_block = current_block_number;
				share_info.gauge_time_factor = share_info
					.gauge_time_factor
					.checked_add(time_factor)
					.ok_or(ArithmeticError::Overflow)?;
				share_info.gauge_amount = share_info
					.gauge_amount
					.checked_sub(&gauge_value)
					.ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;
			Ok(())
		});
		Ok(())
	}

	pub fn gauge_cal_rewards(
		gauge_amount: BalanceOf<T>,
		gauge_last_block: BlockNumberFor<T>,
	) -> DispatchResult {
		// SharesAndWithdrawnRewards::<T>::mutate(pool, who, |share_info| {});
		// GaugePoolInfos::<T>::mutate(gauge, |gauge_info| {});
		Ok(())
	}
}

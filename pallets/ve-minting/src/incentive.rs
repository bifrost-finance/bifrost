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

use crate::{traits::VeMintingInterface, *};
use frame_system::pallet_prelude::*;
pub use pallet::*;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct IncentiveConfig<Balance> {
	rewardRate: Balance,
	rewardPerTokenStored: Balance,
	rewardsDuration: Timestamp,
	periodFinish: Timestamp,
	lastUpdateTime: Timestamp,
}

impl<T: Config> Pallet<T> {
	pub fn lastTimeRewardApplicable() -> Timestamp {
		let current_timestamp: Timestamp =
			sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
		if current_timestamp < Self::incentive_configs().periodFinish {
			current_timestamp
		} else {
			Self::incentive_configs().periodFinish
		}
	}

	pub fn rewardPerToken() -> BalanceOf<T> {
		let current_timestamp: Timestamp =
			sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
		let _totalSupply = Self::totalSupply(current_timestamp);
		if _totalSupply == BalanceOf::<T>::zero() {
			return Self::incentive_configs().rewardPerTokenStored;
		}
		return Self::incentive_configs().rewardPerTokenStored.saturating_add(
			Self::lastTimeRewardApplicable()
				.saturated_into::<BalanceOf<T>>()
				.saturating_sub(
					Self::incentive_configs().lastUpdateTime.saturated_into::<BalanceOf<T>>(),
				)
				.saturating_mul(Self::incentive_configs().rewardRate)
				// .mul(1e18)
				.checked_div(&_totalSupply)
				.unwrap_or_default(), // .ok_or(Error::<T>::CalculationOverflow)?,
		);
	}

	pub fn earned(addr: &AccountIdOf<T>) -> Result<BalanceOf<T>, DispatchError> {
		let current_timestamp: Timestamp =
			sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
		Ok(Self::balanceOf(addr, current_timestamp)?
			.saturating_mul(
				Self::rewardPerToken().saturating_sub(Self::user_reward_per_token_paid(addr)),
			)
			// .div(1e18)
			.saturating_add(Self::rewards(addr)))
	}

	pub fn updateReward(addr: &AccountIdOf<T>) -> DispatchResult {
		let rewardPerTokenStored = Self::rewardPerToken();
		IncentiveConfigs::<T>::mutate(|item| {
			item.rewardPerTokenStored = rewardPerTokenStored;
			item.lastUpdateTime = Self::lastTimeRewardApplicable();
		});
		// let lastUpdateTime = lastTimeRewardApplicable();
		// if (account != address(0)) {
		// rewards[account] = earned(account);
		Rewards::<T>::insert(addr, Self::earned(addr)?);
		UserRewardPerTokenPaid::<T>::insert(addr, rewardPerTokenStored);
		// Self::user_reward_per_token_paid(addr) = rewardPerTokenStored;
		// }
		Ok(())
	}
}

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

	pub fn updateReward(addr: Option<&AccountIdOf<T>>) -> DispatchResult {
		let rewardPerTokenStored = Self::rewardPerToken();
		IncentiveConfigs::<T>::mutate(|item| {
			item.rewardPerTokenStored = rewardPerTokenStored;
			item.lastUpdateTime = Self::lastTimeRewardApplicable();
		});
		if let Some(address) = addr {
			Rewards::<T>::insert(address, Self::earned(&address)?);
			UserRewardPerTokenPaid::<T>::insert(address, rewardPerTokenStored);
		}
		Ok(())
	}

	// pub fn staking(addr: &AccountIdOf<T>, reward: BalanceOf<T>) -> DispatchResult {
	// 	Self::updateReward(Some(addr))
	// }

	pub fn getReward(addr: &AccountIdOf<T>) -> DispatchResult {
		Self::updateReward(Some(addr))?;
		let reward = Self::rewards(addr);
		if reward > BalanceOf::<T>::zero() {
			T::Currency::transfer(
				&T::VeMintingPalletId::get().into_account_truncating(),
				addr,
				reward,
				ExistenceRequirement::KeepAlive,
			)?;
			Rewards::<T>::remove(addr);
		}
		Ok(())
	}

	// Motion
	pub fn notifyRewardAmount(addr: &AccountIdOf<T>, reward: BalanceOf<T>) -> DispatchResult {
		Self::updateReward(None)?;
		let mut conf = Self::incentive_configs();
		let current_timestamp: Timestamp =
			sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
		// let mut rewardRate;
		if current_timestamp >= conf.periodFinish {
			conf.rewardRate = reward
				.checked_div(&conf.rewardsDuration.saturated_into::<BalanceOf<T>>())
				.ok_or(Error::<T>::CalculationOverflow)?;
		} else {
			let remaining = conf
				.periodFinish
				.saturating_sub(current_timestamp)
				.saturated_into::<BalanceOf<T>>();
			let leftover: BalanceOf<T> = remaining.saturating_mul(conf.rewardRate);
			conf.rewardRate = reward
				.saturating_add(leftover)
				.checked_div(&conf.rewardsDuration.saturated_into::<BalanceOf<T>>())
				.ok_or(Error::<T>::CalculationOverflow)?;
		}
		let balance = Self::balanceOf(addr, current_timestamp)?;
		ensure!(
			conf.rewardRate <=
				balance
					.checked_div(&conf.rewardsDuration.saturated_into::<BalanceOf<T>>())
					.ok_or(Error::<T>::CalculationOverflow)?,
			Error::<T>::NotExpire
		);
		conf.lastUpdateTime = current_timestamp;
		conf.periodFinish = current_timestamp.saturating_add(conf.rewardsDuration);

		IncentiveConfigs::<T>::set(conf);
		Ok(())
	}
}

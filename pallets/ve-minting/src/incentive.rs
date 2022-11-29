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

use crate::*;
use frame_system::pallet_prelude::*;
pub use pallet::*;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct IncentiveConfigs<Balance> {
	rewardRate: Balance,
	rewardPerTokenStored: Balance,
	rewardsDuration: Timestamp,
	periodFinish: Timestamp,
	lastUpdateTime: Timestamp,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::storage]
	#[pallet::getter(fn incentive_configs)]
	pub type IncentiveConfigs<T: Config> =
		StorageValue<_, IncentiveConfigs<BalanceOf<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_reward_per_token_paid)]
	pub type UserRewardPerTokenPaid<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn rewards)]
	pub type Rewards<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, BalanceOf<T>, ValueQuery>;

	impl<T: Config> Pallet<T> {
		pub fn lastTimeRewardApplicable() -> Timestamp {
			let current_timestamp: Timestamp =
				sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
			if current_timestamp < periodFinish {
				current_timestamp
			} else {
				Self::incentive_configs().periodFinish
			}
		}

		pub fn rewardPerToken() -> BalanceOf<T> {
			let current_timestamp: Timestamp =
				sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
			let _totalSupply = Self::totalSupply(current_timestamp);
			if (_totalSupply == 0) {
				return Self::incentive_configs().rewardPerTokenStored;
			}
			return Self::incentive_configs().rewardPerTokenStored.add(
				Self::lastTimeRewardApplicable()
					.sub(Self::incentive_configs().lastUpdateTime)
					.mul(Self::incentive_configs().rewardRate)
					// .mul(1e18)
					.div(_totalSupply),
			);
		}

		pub fn earned(addr: &AccountId) -> BalanceOf<T> {
			return Self::balanceOf(addr)
				.mul(Self::rewardPerToken().sub(Self::user_reward_per_token_paid(addr)))
				// .div(1e18)
				.add(Self::rewards(addr));
		}

		pub fn updateReward(addr: &AccountId) {
			let rewardPerTokenStored = rewardPerToken();
			IncentiveConfigs::<T>::mutate(|item| {
				item.rewardPerTokenStored = rewardPerTokenStored;
				item.lastUpdateTime = lastTimeRewardApplicable();
			});
			// let lastUpdateTime = lastTimeRewardApplicable();
			// if (account != address(0)) {
			// rewards[account] = earned(account);
			Rewards::<T>::insert(addr, Self::earned(addr));
			UserRewardPerTokenPaid::<T>::insert(addr, rewardPerTokenStored);
			// Self::user_reward_per_token_paid(addr) = rewardPerTokenStored;
			// }
		}
	}
}

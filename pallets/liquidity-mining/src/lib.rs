// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::{SaturatedConversion, Saturating, Zero},
	traits::EnsureOrigin,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, LeasePeriod, ParaId};
use orml_traits::{MultiCurrency, MultiLockableCurrency, MultiReservableCurrency};
pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

// TODO: 讨论
// - Pool创建方式(Anyone or Council?) & 奖励注入方式
// - 函数的参数配置
// - 奖励如何释放
//  - 释放周期
//  - 释放比例

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct PoolInfo<T: Config> {
	creator: AccountIdOf<T>,
	liquidity_pair: (CurrencyId, CurrencyId),
	total_release_time: u32,
	min_staked_amount_to_start: BalanceOf<T>,
	after_block_to_start: BlockNumberFor<T>,
	r#type: PoolType,

	already_released_time: u32,
	rewards: Vec<RewardData<T>>,
	state: PoolState<T>,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, Debug)]
pub enum PoolType {
	Mining,
	Farming,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq)]
pub enum PoolState<T: Config> {
	Idle,
	Activated,
	Ongoing(BlockNumberFor<T>),
	Dead,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct StakeData<T: Config> {
	pid: PoolId,
	amount_staked: BalanceOf<T>,
	// TODO: The rewarded
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct RewardData<T: Config> {
	token: CurrencyId,
	total: BalanceOf<T>,
	released: BalanceOf<T>,
	claimed: BalanceOf<T>,
}

impl<T: Config> core::fmt::Debug for RewardData<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_tuple("").field(&self.token).field(&self.total).finish()
	}
}

#[allow(type_alias_bounds)]
type AccountIdOf<T: Config> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

type PoolId = u128;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Origin for anyone able to create/activate/kill the liquidity-pool.
		type ControlOrigin: EnsureOrigin<Self::Origin>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		#[pallet::constant]
		type RelayChainToken: Get<CurrencyId>;

		#[pallet::constant]
		type ReleaseCycle: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type MinDeposit: Get<BalanceOf<T>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		// TODO
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		// TODO: PoolCreated
	// TODO: PoolActivated
	// TODO: PoolKilled
	// TODO: UserStaked
	// TODO: UserRedeemed
	// TODO: UserClaimed
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_mining_pool(
			origin: OriginFor<T>,
			liquidity_pair: (CurrencyId, CurrencyId),
			main_reward: (CurrencyId, BalanceOf<T>),
			option_rewards: [(CurrencyId, BalanceOf<T>); 4],
			#[pallet::compact] total_release_time: u32,
			#[pallet::compact] min_staked_amount_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			let creator = ensure_signed(origin)?;

			// TODO: Check the stakeds

			// TODO: Check the rewards

			// TODO: Check the duration

			// TODO: Check the start-up condition

			// TODO: Construct the PoolInfo

			todo!()
		}

		#[pallet::weight(1_000)]
		pub fn create_farming_pool(
			origin: OriginFor<T>,
			index: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
			main_reward: (CurrencyId, BalanceOf<T>),
			option_reward: [(CurrencyId, BalanceOf<T>); 4],
			#[pallet::compact] total_release_time: u32,
			#[pallet::compact] min_staked_amount_to_start: BalanceOf<T>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			let creator = ensure_signed(origin)?;

			todo!()
		}

		#[pallet::weight(1_000)]
		pub fn activate_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			// TODO: Query the `PoolInfo` by pid

			// TODO: Check the state of `PoolInfo`

			// TODO: Check the balance of rewards

			// TODO: Change the state of `PoolInfo`

			todo!()
		}

		#[pallet::weight(1_000)]
		pub fn kill_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
			let _ = T::ControlOrigin::ensure_origin(origin)?;

			// TODO: Query the `PoolInfo` by pid

			// TODO: Check the state of `PoolInfo`

			// TODO: Change the state of `PoolInfo`

			todo!()
		}

		#[pallet::weight(1_000)]
		pub fn deposit(origin: OriginFor<T>, value: BalanceOf<T>) -> DispatchResultWithPostInfo {
			todo!()
		}

		#[pallet::weight(1_000)]
		pub fn redeem(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			todo!()
		}

		#[pallet::weight(1_000)]
		pub fn claim(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			todo!()
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			// TODO: Check whether pool-activated is meet the startup condition

			// TODO: Check whether pool-ongoing reach the release reward time

			// TODO: Check whether pool-ongoing reach the end time
		}

		fn on_initialize(_n: BlockNumberFor<T>) -> frame_support::weights::Weight {
			// TODO estimate weight
			Zero::zero()
		}
	}
}

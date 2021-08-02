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
use orml_traits::{MultiCurrency, MultiLockableCurrency};
use node_primitives::{CurrencyId, LeasePeriod};
pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct PoolInfo<T: Config> {
    // TODO: LpToken Type
    rewards_config: (RewardData<T>, Option<RewardData<T>>, Option<RewardData<T>>),
    duration: BlockNumberFor<T>,
    // TODO: More conditions?
    threshold: BalanceOf<T>,
    r#type: PoolType,

    // TODO: The rewarded
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
    amount: BalanceOf<T>,
}

impl<T: Config> core::fmt::Debug for RewardData<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("")
            .field(&self.token)
            .field(&self.amount)
            .finish()
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
        + MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
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
        pub fn create_pool(
            origin: OriginFor<T>,
            stakeds: (CurrencyId, Option<CurrencyId>),
            rewards: (RewardData<T>, Option<RewardData<T>>, Option<RewardData<T>>),
            #[pallet::compact] duration: BlockNumberFor<T>,
            #[pallet::compact] threshold: BalanceOf<T>,
            pool_type: PoolType,
        ) -> DispatchResultWithPostInfo {
            let _ = T::ControlOrigin::ensure_origin(origin)?;

            // TODO: Check the stakeds

            // TODO: Check the rewards

            // TODO: Check the duration

            // TODO: Check the start-up condition

            // TODO: Construct the PoolInfo

            todo!()
        }

        #[pallet::weight(1_000)]
        pub fn activate_pool(origin: OriginFor<T>, pid: PoolId) -> DispatchResultWithPostInfo {
            let _ = T::ControlOrigin::ensure_origin(origin)?;

            // TODO: Query the `PoolInfo` by pid

            // TODO: Check the state of `PoolInfo`

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
        pub fn stake(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
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
}
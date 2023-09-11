// Copyright 2021-2022 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;

pub mod v3 {
    use super::*;
    use crate::{pallet::StorageVersion, Config, Weight};
    use frame_support::{log, traits::Get};

    pub const DEFAULT_LIQUIDATE_INCENTIVE_RESERVED_FACTOR: Ratio = Ratio::from_percent(3);
    pub const DEFAULT_LIQUIDATION_OFFSET: Ratio = Ratio::from_percent(10);

    #[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
    #[derive(Clone, PartialEq, Eq, codec::Decode, codec::Encode, RuntimeDebug, TypeInfo)]
    pub struct V2Market<Balance> {
        /// The collateral utilization ratio
        pub collateral_factor: Ratio,
        /// Fraction of interest currently set aside for reserves
        pub reserve_factor: Ratio,
        /// The percent, ranging from 0% to 100%, of a liquidatable account's
        /// borrow that can be repaid in a single liquidate transaction.
        pub close_factor: Ratio,
        /// Liquidation incentive ratio
        pub liquidate_incentive: Rate,
        /// Current interest rate model being used
        pub rate_model: InterestRateModel,
        /// Current market state
        pub state: MarketState,
        /// Upper bound of supplying
        pub supply_cap: Balance,
        /// Upper bound of borrowing
        pub borrow_cap: Balance,
        /// Ptoken asset id
        pub ptoken_id: CurrencyId,
    }
    #[frame_support::storage_alias]
    type MarketRewardSpeed<T: Config> =
        StorageMap<crate::Pallet<T>, Blake2_128Concat, AssetIdOf<T>, BalanceOf<T>>;

    #[frame_support::storage_alias]
    type RewardAccured<T: Config> = StorageMap<
        crate::Pallet<T>,
        Blake2_128Concat,
        <T as frame_system::Config>::AccountId,
        BalanceOf<T>,
    >;

    #[frame_support::storage_alias]
    type LastAccruedTimestamp<T: Config> = StorageValue<crate::Pallet<T>, Timestamp, ValueQuery>;

    #[cfg(feature = "try-runtime")]
    pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
        #[frame_support::storage_alias]
        type Markets<T: Config> =
            StorageMap<crate::Pallet<T>, Blake2_128Concat, AssetIdOf<T>, V2Market<BalanceOf<T>>>;
        frame_support::ensure!(
            StorageVersion::<T>::get() == crate::Versions::V2,
            "must upgrade linearly"
        );
        Markets::<T>::iter().for_each(|(asset_id, _)| {
            log::info!("market {:#?} need to migrate", asset_id,);
        });
        let reward_speed_count = MarketRewardSpeed::<T>::iter().count();
        log::info!(
            "total {:#?} reward speed items need to migrate",
            reward_speed_count
        );

        let last_accrued_timestamp = LastAccruedTimestamp::<T>::get();
        log::info!(
            "LastAccruedTimestamp: {:#?} is about to move.",
            last_accrued_timestamp
        );

        let old_name_items_count = RewardAccured::<T>::iter().count();
        let new_name_items_count = RewardAccrued::<T>::iter().count();
        log::info!(
            "old_name_items_count: {:#?}, new_name_items_count: {:#?}.",
            old_name_items_count,
            new_name_items_count,
        );

        log::info!("👜 loans v3 migration passes PRE migrate checks ✅",);

        Ok(())
    }

    /// Migration to sorted [`SortedListProvider`].
    pub fn migrate<T: Config>() -> Weight {
        if StorageVersion::<T>::get() == crate::Versions::V2 {
            log::info!("migrating loans to Versions::V3",);

            Markets::<T>::translate::<V2Market<BalanceOf<T>>, _>(|_key, market| {
                Some(Market {
                    borrow_cap: market.borrow_cap,
                    supply_cap: market.supply_cap,
                    collateral_factor: market.collateral_factor,
                    liquidation_threshold: (market.collateral_factor
                        + market.collateral_factor * DEFAULT_LIQUIDATION_OFFSET),
                    reserve_factor: market.reserve_factor,
                    close_factor: market.close_factor,
                    liquidate_incentive_reserved_factor:
                        DEFAULT_LIQUIDATE_INCENTIVE_RESERVED_FACTOR,
                    liquidate_incentive: market.liquidate_incentive,
                    rate_model: market.rate_model,
                    state: market.state,
                    ptoken_id: market.ptoken_id,
                })
            });

            MarketRewardSpeed::<T>::iter().for_each(|(asset_id, reward_speed)| {
                RewardSupplySpeed::<T>::insert(asset_id, reward_speed);
                RewardBorrowSpeed::<T>::insert(asset_id, reward_speed);
            });

            //remove old data.
            let _ = MarketRewardSpeed::<T>::clear(u32::max_value(), None);
            LastAccruedTimestamp::<T>::kill();

            StorageVersion::<T>::put(crate::Versions::V3);
            log::info!("👜 completed loans migration to Versions::V3",);

            T::BlockWeights::get().max_block
        } else {
            T::DbWeight::get().reads(1)
        }
    }

    #[cfg(feature = "try-runtime")]
    pub fn post_migrate<T: Config>() -> Result<(), &'static str> {
        frame_support::ensure!(
            StorageVersion::<T>::get() == crate::Versions::V3,
            "must upgrade to V3"
        );
        Markets::<T>::iter().for_each(|(asset_id, market)| {
            log::info!(
                "market {:#?}, collateral_factor {:?}, liquidation_threshold {:?}, liquidate_incentive_reserved_factor {:?}",
                asset_id,
                market.collateral_factor,
                market.liquidation_threshold,
                market.liquidate_incentive_reserved_factor
            );
        });
        RewardSupplySpeed::<T>::iter().for_each(|(asset_id, supply_reward_speed)| {
            let borrow_reward_speed = RewardBorrowSpeed::<T>::get(asset_id);
            log::info!(
                "market {:#?}, supply_reward_speed {:?}, borrow_reward_speed {:?}",
                asset_id,
                supply_reward_speed,
                borrow_reward_speed
            );
        });

        let reward_speed_count = MarketRewardSpeed::<T>::iter().count();
        log::info!(
            "total {:#?} reward speed items remains after migrate",
            reward_speed_count
        );

        let last_accrued_timestamp = LastAccruedTimestamp::<T>::get();
        log::info!(
            "LastAccruedTimestamp: {:#?} after migrate.",
            last_accrued_timestamp
        );

        let old_name_items_count = RewardAccured::<T>::iter().count();
        let new_name_items_count = RewardAccrued::<T>::iter().count();
        log::info!(
            "old_name_items_count: {:#?}, new_name_items_count: {:#?}.",
            old_name_items_count,
            new_name_items_count,
        );

        log::info!("👜 loans v3 migration passes POST migrate checks ✅",);

        Ok(())
    }
}

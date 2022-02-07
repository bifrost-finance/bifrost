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
#![cfg_attr(not(feature = "std"), no_std)]

pub mod v2 {
	use frame_support::traits::{OnRuntimeUpgrade, PalletInfo};

	use crate::*;

	type PoolInfoOld<T, I> =
		deprecated::PoolInfo<AccountIdOf<T>, BalanceOf<T, I>, BlockNumberFor<T>>;
	type PoolInfoNew<T, I> = PoolInfo<AccountIdOf<T>, BalanceOf<T, I>, BlockNumberFor<T>>;
	type DepositDataOld<T, I> = deprecated::DepositData<BalanceOf<T, I>, BlockNumberFor<T>>;
	type DepositDataNew<T, I> = DepositData<BalanceOf<T, I>, BlockNumberFor<T>>;

	pub(crate) mod deprecated {
		use super::PalletInfo;
		use crate::*;

		#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
		pub(crate) struct PoolInfo<AccountIdOf, BalanceOf, BlockNumberOf>
		where
			AccountIdOf: Clone,
			BalanceOf: AtLeast32BitUnsigned + Copy,
			BlockNumberOf: AtLeast32BitUnsigned + Copy,
		{
			/// Id of the liquidity-pool
			pub(crate) pool_id: PoolId,
			/// The keeper of the liquidity-pool
			pub(crate) keeper: AccountIdOf,
			/// The man who charges the rewards to the pool
			pub(crate) investor: Option<AccountIdOf>,
			/// The trading-pair supported by the liquidity-pool
			pub(crate) trading_pair: (CurrencyId, CurrencyId),
			/// The length of time the liquidity-pool releases rewards
			pub(crate) duration: BlockNumberOf,
			/// The liquidity-pool type
			pub(crate) r#type: PoolType,

			/// The First Condition
			///
			/// When starts the liquidity-pool, the amount deposited in the liquidity-pool
			/// should be greater than the value.
			pub(crate) min_deposit_to_start: BalanceOf,
			/// The Second Condition
			///
			/// When starts the liquidity-pool, the current block should be greater than the value.
			pub(crate) after_block_to_start: BlockNumberOf,

			/// The total amount deposited in the liquidity-pool
			pub(crate) deposit: BalanceOf,

			/// The reward infos about the liquidity-pool
			pub(crate) rewards: BTreeMap<CurrencyId, RewardData<BalanceOf>>,
			/// The block of the last update of the rewards
			pub(crate) update_b: BlockNumberOf,
			/// The liquidity-pool state
			pub(crate) state: PoolState,
			/// The block number when the liquidity-pool startup
			pub(crate) block_startup: Option<BlockNumberOf>,
			/// The block number when the liquidity-pool retired
			pub(crate) block_retired: Option<BlockNumberOf>,
		}

		#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
		pub(crate) struct DepositData<BalanceOf, BlockNumberOf>
		where
			BalanceOf: AtLeast32BitUnsigned + Copy,
			BlockNumberOf: AtLeast32BitUnsigned + Copy,
		{
			/// The amount of trading-pair deposited in the liquidity-pool
			pub(crate) deposit: BalanceOf,
			/// The average gain in pico by 1 pico deposited from the startup of the
			/// liquidity-pool, updated when the `DepositData`'s owner deposits/redeems/claims from
			/// the liquidity-pool.
			///
			/// - Arg0: The average gain in pico by 1 pico deposited from the startup of the
			///   liquidity-pool
			/// - Arg1: The block number updated lastest
			pub(crate) gain_avgs: BTreeMap<CurrencyId, FixedU128>,
			pub(crate) update_b: BlockNumberOf,
		}

		pub(crate) struct TotalPoolInfosPrefix<T, I>(PhantomData<(T, I)>);
		impl<T: Config<I>, I: 'static> frame_support::traits::StorageInstance
			for TotalPoolInfosPrefix<T, I>
		{
			fn pallet_prefix() -> &'static str {
				T::PalletInfo::name::<Pallet<T, I>>().unwrap_or("none")
			}
			const STORAGE_PREFIX: &'static str = "TotalPoolInfos";
		}

		pub(crate) struct TotalDepositDataPrefix<T, I>(PhantomData<(T, I)>);
		impl<T: Config<I>, I: 'static> frame_support::traits::StorageInstance
			for TotalDepositDataPrefix<T, I>
		{
			fn pallet_prefix() -> &'static str {
				T::PalletInfo::name::<Pallet<T, I>>().unwrap_or("none")
			}
			const STORAGE_PREFIX: &'static str = "TotalDepositData";
		}

		#[allow(type_alias_bounds)]
		pub(crate) type TotalPoolInfos<T: Config<I>, I: 'static = ()> = StorageMap<
			TotalPoolInfosPrefix<T, I>,
			Twox64Concat,
			PoolId,
			self::PoolInfo<AccountIdOf<T>, BalanceOf<T, I>, BlockNumberFor<T>>,
		>;

		#[allow(type_alias_bounds)]
		pub(crate) type TotalDepositData<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
			TotalDepositDataPrefix<T, I>,
			Blake2_128Concat,
			PoolId,
			Blake2_128Concat,
			AccountIdOf<T>,
			self::DepositData<BalanceOf<T, I>, BlockNumberFor<T>>,
		>;
	}

	pub struct Upgrade<T, I>(PhantomData<(T, I)>);

	impl<T: Config<I>, I: 'static> OnRuntimeUpgrade for Upgrade<T, I> {
		fn on_runtime_upgrade() -> Weight {
			let pallet_name = T::PalletInfo::name::<Pallet<T, I>>().unwrap_or("none");
			log::info!("{} on processing", pallet_name);

			if Pallet::<T, I>::storage_version() == StorageVersion::V1_0_0 {
				let tp_nums = deprecated::TotalPoolInfos::<T, I>::iter().count() as u32;
				let td_nums = deprecated::TotalDepositData::<T, I>::iter().count() as u32;

				log::info!(" >>> update `PoolInfo` storage: Migrating {} pool", tp_nums);

				TotalPoolInfos::<T, I>::translate::<PoolInfoOld<T, I>, _>(|id, pool| {
					log::info!("    migrated pool-info for {}", id);

					Some(PoolInfoNew::<T, I> {
						pool_id: pool.pool_id,
						keeper: pool.keeper,
						investor: pool.investor,
						trading_pair: pool.trading_pair,
						duration: pool.duration,
						r#type: pool.r#type,

						min_deposit_to_start: pool.min_deposit_to_start,
						after_block_to_start: pool.after_block_to_start,

						deposit: pool.deposit,

						rewards: pool.rewards,
						update_b: pool.update_b,
						state: pool.state,
						block_startup: pool.block_startup,
						block_retired: pool.block_retired,

						redeem_limit_time: Zero::zero(),
						unlock_limit_nums: 0,
						pending_unlock_nums: 0,
					})
				});

				log::info!(" >>> update `DepositData` storage: Migrating {} user", td_nums);

				TotalDepositData::<T, I>::translate::<DepositDataOld<T, I>, _>(|id, user, data| {
					log::info!("    migrated deposit-data for {}: {:?}", id, user);

					Some(DepositDataNew::<T, I> {
						deposit: data.deposit,
						gain_avgs: data.gain_avgs,
						update_b: data.update_b,

						pending_unlocks: Default::default(),
					})
				});

				PalletVersion::<T, I>::put(StorageVersion::V2_0_0);

				log::info!(" >>> migration completed!");

				let total_nums = tp_nums + td_nums;
				T::DbWeight::get().reads_writes(total_nums as Weight + 1, total_nums as Weight + 1)
			} else {
				log::info!(" >>> unused migration!");
				0
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<(), &'static str> {
			use frame_support::traits::OnRuntimeUpgradeHelpersExt;

			let pallet_name = T::PalletInfo::name::<Pallet<T, I>>().unwrap_or("none");

			ensure!(
				Pallet::<T, I>::storage_version() == StorageVersion::V1_0_0,
				"❌ liquidity-mining upgrade to V2_0_0: not right version",
			);

			let tp_nums_old = deprecated::TotalPoolInfos::<T, I>::iter().count() as u32;
			let td_nums_old = deprecated::TotalDepositData::<T, I>::iter().count() as u32;
			Self::set_temp_storage((tp_nums_old, td_nums_old), pallet_name);

			log::info!(
				"✅ liquidity-mining({}) upgrade to V2_0_0: pass PRE migrate checks",
				pallet_name
			);

			Ok(())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade() -> Result<(), &'static str> {
			use frame_support::traits::OnRuntimeUpgradeHelpersExt;

			let pallet_name = T::PalletInfo::name::<Pallet<T, I>>().unwrap_or("none");

			let (tp_nums_old, td_nums_old) =
				Self::get_temp_storage::<(u32, u32)>(pallet_name).unwrap();
			let (tp_nums_new, td_nums_new) = (
				TotalPoolInfos::<T, I>::iter().count() as u32,
				TotalDepositData::<T, I>::iter().count() as u32,
			);

			ensure!(
				tp_nums_old == tp_nums_new,
				"❌ liquidity-mining upgrade to V2_0_0: pool quantity does not match"
			);

			ensure!(
				td_nums_old == td_nums_new,
				"❌ liquidity-mining upgrade to V2_0_0: user quantity does not match"
			);

			log::info!(
				"✅ liquidity-mining({}) upgrade to V2_0_0: pass POST migrate checks",
				pallet_name
			);

			Ok(())
		}
	}
}

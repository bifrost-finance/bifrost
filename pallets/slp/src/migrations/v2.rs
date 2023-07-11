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
use frame_support::traits::OnRuntimeUpgrade;

const LOG_TARGET: &str = "SLP::migration";

pub struct SlpMigration<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for SlpMigration<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// Check the storage version
		let onchain_version = Pallet::<T>::on_chain_storage_version();
		if onchain_version < 2 {
			// Transform storage values
			// We transform the storage values from the old into the new format.
			log::info!(target: LOG_TARGET, "Start to migrate Validators storage...");
			Validators::<T>::translate(|k: CurrencyId, value: Vec<MultiLocation>| {
				log::info!(target: LOG_TARGET, "Migrated to boundedvec for {:?}...", k);

				let target_bounded_vec: BoundedVec<MultiLocation, T::MaxLengthLimit>;

				if value.len() != 0 {
					target_bounded_vec = BoundedVec::try_from(value).unwrap();
				} else {
					target_bounded_vec = BoundedVec::<MultiLocation, T::MaxLengthLimit>::default();
				}

				Some(target_bounded_vec)
			});

			log::info!(target: LOG_TARGET, "Start to migrate ValidatorsByDelegator storage...");
			//migrate the value type of ValidatorsByDelegator
			ValidatorsByDelegator::<T>::translate(|key1, key2, value: Vec<MultiLocation>| {
				log::info!(
					target: LOG_TARGET,
					"Migrated to boundedvec for {:?} - {:?}...",
					key1,
					key2
				);

				let target_bounded_vec: BoundedVec<MultiLocation, T::MaxLengthLimit>;

				if value.len() != 0 {
					target_bounded_vec = BoundedVec::try_from(value).unwrap();
				} else {
					target_bounded_vec = BoundedVec::<MultiLocation, T::MaxLengthLimit>::default();
				}

				Some(target_bounded_vec)
			});

			log::info!(target: LOG_TARGET, "Start to migrate ValidatorBoostList storage...");
			//migrate the value type of ValidatorBoostList
			ValidatorBoostList::<T>::translate(
				|k: CurrencyId, value: Vec<(MultiLocation, BlockNumberFor<T>)>| {
					log::info!(target: LOG_TARGET, "Migrated to boundedvec for {:?}...", k);

					let target_bounded_vec: BoundedVec<
						(MultiLocation, BlockNumberFor<T>),
						T::MaxLengthLimit,
					>;

					if value.len() != 0 {
						target_bounded_vec = BoundedVec::try_from(value).unwrap();
					} else {
						target_bounded_vec = BoundedVec::<
							(MultiLocation, BlockNumberFor<T>),
							T::MaxLengthLimit,
						>::default();
					}

					Some(target_bounded_vec)
				},
			);

			// Update the storage version
			StorageVersion::new(2).put::<Pallet<T>>();

			// Return the consumed weight
			let count = Validators::<T>::iter().count() +
				ValidatorsByDelegator::<T>::iter().count() +
				ValidatorBoostList::<T>::iter().count();
			Weight::from(T::DbWeight::get().reads_writes(count as u64 + 1, count as u64 + 1))
		} else {
			// We don't do anything here.
			Weight::zero()
		}
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
		let validator_cnt = Validators::<T>::iter().count();
		// print out the pre-migrate storage count
		log::info!(target: LOG_TARGET, "Validators pre-migrate storage count: {:?}", validator_cnt);

		let validator_by_delegator_cnt = ValidatorsByDelegator::<T>::iter().count();
		log::info!(
			target: LOG_TARGET,
			"ValidatorsByDelegator pre-migrate storage count: {:?}",
			validator_by_delegator_cnt
		);

		let validator_boost_list_cnt = ValidatorBoostList::<T>::iter().count();
		log::info!(
			target: LOG_TARGET,
			"ValidatorBoostList pre-migrate storage count: {:?}",
			validator_boost_list_cnt
		);

		let cnt = (
			validator_cnt as u32,
			validator_by_delegator_cnt as u32,
			validator_boost_list_cnt as u32,
		);
		Ok(cnt.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(cnt: Vec<u8>) -> Result<(), &'static str> {
		let (validator_cnt_old, validator_by_delegator_cnt_old, validator_boost_list_cnt_old) =
			<(u32, u32, u32)>::decode(&mut &cnt[..]).map_err(|_| "Invalid data")?;

		let validator_cnt_new = Validators::<T>::iter().count();
		// print out the post-migrate storage count
		log::info!(
			target: LOG_TARGET,
			"Validators post-migrate storage count: {:?}",
			Validators::<T>::iter().count()
		);
		ensure!(
			validator_cnt_new as u32 == validator_cnt_old,
			"Validators post-migrate storage count not match"
		);

		let validator_by_delegator_cnt_new = ValidatorsByDelegator::<T>::iter().count();
		log::info!(
			target: LOG_TARGET,
			"ValidatorsByDelegator post-migrate storage count: {:?}",
			ValidatorsByDelegator::<T>::iter().count()
		);
		ensure!(
			validator_by_delegator_cnt_new as u32 == validator_by_delegator_cnt_old,
			"ValidatorsByDelegator post-migrate storage count not match"
		);

		let validator_boost_list_cnt_new = ValidatorBoostList::<T>::iter().count();
		log::info!(
			target: LOG_TARGET,
			"ValidatorBoostList post-migrate storage count: {:?}",
			ValidatorBoostList::<T>::iter().count()
		);
		ensure!(
			validator_boost_list_cnt_new as u32 == validator_boost_list_cnt_old,
			"ValidatorBoostList post-migrate storage count not match"
		);

		Ok(())
	}
}

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

const LOG_TARGET: &str = "SLP::migration";

// contains checks and transforms storage to V2 format
pub fn migrate_to_v2<T: Config>() -> Weight {
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
			log::info!(target: LOG_TARGET, "Migrated to boundedvec for {:?} - {:?}...", key1, key2);

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

pub fn pre_migrate<T: Config>() {
	// print out the pre-migrate storage count
	log::info!(
		target: LOG_TARGET,
		"Validators pre-migrate storage count: {:?}",
		Validators::<T>::iter().count()
	);
	log::info!(
		target: LOG_TARGET,
		"ValidatorsByDelegator pre-migrate storage count: {:?}",
		ValidatorsByDelegator::<T>::iter().count()
	);
	log::info!(
		target: LOG_TARGET,
		"ValidatorBoostList pre-migrate storage count: {:?}",
		ValidatorBoostList::<T>::iter().count()
	);
}

pub fn post_migrate<T: Config>() {
	// print out the post-migrate storage count
	log::info!(
		target: LOG_TARGET,
		"Validators post-migrate storage count: {:?}",
		Validators::<T>::iter().count()
	);
	log::info!(
		target: LOG_TARGET,
		"ValidatorsByDelegator post-migrate storage count: {:?}",
		ValidatorsByDelegator::<T>::iter().count()
	);
	log::info!(
		target: LOG_TARGET,
		"ValidatorBoostList post-migrate storage count: {:?}",
		ValidatorBoostList::<T>::iter().count()
	);
}

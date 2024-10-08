// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

const LOG_TARGET: &str = "asset-registry::migration";

pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// Check the storage version
		let in_code_version = Pallet::<T>::in_code_storage_version();
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		// Transform storage values
		// We transform the storage values from the old into the new format.
		if on_chain_version == 0 && in_code_version == 1 {
			let mut count = 0;

			log::info!(target: LOG_TARGET, "Start to migrate RegisterWhiteList storage...");
			CurrencyIdToLocations::<T>::translate::<xcm::v3::Location, _>(
				|k: CurrencyId, value: xcm::v3::Location| {
					log::info!(target: LOG_TARGET, "CurrencyIdToLocations Migrated to xcm::v4::Location for {:?}...", k);
					let v4_location = xcm::v4::Location::try_from(value).unwrap();

					count += 1;
					Some(v4_location)
				},
			);

			log::info!(target: LOG_TARGET, "Start to migrate LocationToCurrencyIds storage...");
			let migrated_items: Vec<_> = LocationToCurrencyIds::<T>::drain()
				.map(|(v3_location, value)| {
					log::info!(target: LOG_TARGET, "LocationToCurrencyIds Migrated to xcm::v4::Location for {:?}...", value);
					let v4_location = xcm::v4::Location::try_from(v3_location).unwrap();

					count += 1;
					(v4_location, value)
				})
				.collect();
			for (v4_location, value) in migrated_items {
				LocationToCurrencyIds::<T>::insert(v4_location, value);
			}

			// Update the storage version
			in_code_version.put::<Pallet<T>>();

			// Return the consumed weight
			Weight::from(T::DbWeight::get().reads_writes(count as u64 + 1, count as u64 + 1))
		} else {
			// We don't do anything here.
			Weight::zero()
		}
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		ensure!(Pallet::<T>::on_chain_storage_version() == 0, "must upgrade linearly");
		ensure!(Pallet::<T>::in_code_storage_version() == 1, "must upgrade linearly");
		let currency_id_to_locations_count = CurrencyIdToLocations::<T>::iter().count();
		log::info!(target: LOG_TARGET, "CurrencyIdToLocations pre-migrate storage count: {:?}", currency_id_to_locations_count);

		let location_to_currency_ids_count = LocationToCurrencyIds::<T>::iter().count();
		log::info!(target: LOG_TARGET, "LocationToCurrencyIds pre-migrate storage count: {:?}", location_to_currency_ids_count);

		let combined_data =
			(currency_id_to_locations_count as u64, location_to_currency_ids_count as u64);

		Ok(combined_data.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(cnt: Vec<u8>) -> Result<(), TryRuntimeError> {
		let in_code_version = Pallet::<T>::in_code_storage_version();
		let on_chain_version = Pallet::<T>::on_chain_storage_version();
		ensure!(in_code_version == 1, "must_upgrade");
		ensure!(
			in_code_version == on_chain_version,
			"after migration, the in_code_version and on_chain_version should be the same"
		);

		let (old_currency_id_to_locations_count, old_location_to_currency_ids_count): (u64, u64) =
			Decode::decode(&mut cnt.as_slice()).expect(
				"the state parameter should be something that was generated by pre_upgrade",
			);

		let new_currency_id_to_locations_count = CurrencyIdToLocations::<T>::iter().count();
		log::info!(
			target: LOG_TARGET,
			"CurrencyIdToLocations post-migrate storage count: {:?}",
			new_currency_id_to_locations_count
		);

		let new_location_to_currency_ids_count = LocationToCurrencyIds::<T>::iter().count();
		log::info!(
			target: LOG_TARGET,
			"LocationToCurrencyIds post-migrate storage count: {:?}",
			new_location_to_currency_ids_count
		);

		ensure!(
			new_currency_id_to_locations_count as u64 == old_currency_id_to_locations_count,
			"Post-migration CurrencyIdToLocations count does not match pre-migration count"
		);
		ensure!(
			new_location_to_currency_ids_count as u64 == old_location_to_currency_ids_count,
			"Post-migration LocationToCurrencyIds count does not match pre-migration count"
		);

		Ok(())
	}
}

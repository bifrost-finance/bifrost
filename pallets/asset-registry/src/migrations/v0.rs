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
use bifrost_primitives::{CurrencyId, BNC};
use frame_support::traits::{Get, OnRuntimeUpgrade};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
use xcm::opaque::v3::Junctions::X1;

const LOG_TARGET: &str = "asset-registry::migration";

pub fn update_blp_metadata<T: Config>(pool_count: u32) -> Weight {
	for pool_id in 0..pool_count {
		if let Some(old_metadata) = CurrencyMetadatas::<T>::get(CurrencyId::BLP(pool_id)) {
			let name = scale_info::prelude::format!("Bifrost Stable Pool Token {}", pool_id)
				.as_bytes()
				.to_vec();
			let symbol = scale_info::prelude::format!("BLP{}", pool_id).as_bytes().to_vec();
			CurrencyMetadatas::<T>::insert(
				CurrencyId::BLP(pool_id),
				&AssetMetadata { name, symbol, ..old_metadata },
			)
		}
	}

	T::DbWeight::get().reads(pool_count.into()) + T::DbWeight::get().writes(pool_count.into())
}

const BNC_LOCATION: xcm::v3::Location = xcm::v3::Location {
	parents: 0,
	interior: X1(xcm::v3::Junction::GeneralKey {
		length: 2,
		data: [
			0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			0, 0, 0,
		],
	}),
};

pub struct InsertBNCMetadata<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for InsertBNCMetadata<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!(target: LOG_TARGET, "Start to insert BNC Metadata...");
		CurrencyMetadatas::<T>::insert(
			BNC,
			&AssetMetadata {
				name: b"Bifrost Native Token".to_vec(),
				symbol: b"BNC".to_vec(),
				decimals: 12,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(10_000_000_000u128),
			},
		);

		match Location::try_from(BNC_LOCATION) {
			Ok(location) => {
				CurrencyIdToLocations::<T>::insert(BNC, location.clone());
				LocationToCurrencyIds::<T>::insert(location, BNC);

				Weight::from(T::DbWeight::get().reads_writes(3 as u64 + 1, 3 as u64 + 1))
			},
			Err(_) => {
				log::error!(target: LOG_TARGET, "Conversion failed from BNC_LOCATION to v4_bnc_location.");
				Weight::from(T::DbWeight::get().reads_writes(1 as u64 + 1, 1 as u64 + 1))
			},
		}
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let bnc_location = Location::try_from(BNC_LOCATION).unwrap();
		assert!(LocationToCurrencyIds::<T>::get(bnc_location).is_none());

		Ok(sp_std::vec![])
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_cnt: Vec<u8>) -> Result<(), TryRuntimeError> {
		let metadata = CurrencyMetadatas::<T>::get(BNC);
		assert_eq!(
			metadata,
			Some(AssetMetadata {
				name: b"Bifrost Native Token".to_vec(),
				symbol: b"BNC".to_vec(),
				decimals: 12,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(10_000_000_000u128),
			})
		);
		log::info!(
			target: LOG_TARGET,
			"InsertBNCMetadata post-migrate storage: {:?}",
			metadata
		);

		let bnc_location = Location::try_from(BNC_LOCATION).unwrap();

		let location = CurrencyIdToLocations::<T>::get(BNC);
		assert_eq!(location, Some(bnc_location.clone()));

		log::info!(
			target: LOG_TARGET,
			"InsertBNCMetadata post-migrate storage: {:?}",
			location
		);

		let currency = LocationToCurrencyIds::<T>::get(bnc_location);
		assert_eq!(currency, Some(BNC));
		log::info!(
			target: LOG_TARGET,
			"InsertBNCMetadata post-migrate storage: {:?}",
			currency
		);

		Ok(())
	}
}

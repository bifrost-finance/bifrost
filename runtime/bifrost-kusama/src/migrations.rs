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

//! A set of constant values used in Bifrost runtime.

use bifrost_asset_registry::{AssetMetadatas, Config, CurrencyIdToLocations, NextForeignAssetId};
use frame_support::{pallet_prelude::Weight, traits::Get};
use node_primitives::{AssetIds, CurrencyId};

pub struct AssetRegistryMigration<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> frame_support::traits::OnRuntimeUpgrade for AssetRegistryMigration<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut len = Weight::default();

		NextForeignAssetId::<T>::kill();
		for id in 0..4u32 {
			AssetMetadatas::<T>::remove(AssetIds::ForeignAssetId(id));
			CurrencyIdToLocations::<T>::remove(CurrencyId::ForeignAsset(id));
			len += 2
		}

		<T as frame_system::Config>::DbWeight::get().reads_writes(len, len)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		assert!(NextForeignAssetId::<T>::get() == 4, "NextForeignAssetId == 4");

		assert!(
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(0)).is_some(),
			"ForeignAssetId(0) not exist"
		);
		assert!(
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(1)).is_some(),
			"ForeignAssetId(1) not exist"
		);
		assert!(
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(2)).is_some(),
			"ForeignAssetId(2) not exist"
		);
		assert!(
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(3)).is_some(),
			"ForeignAssetId(3) not exist"
		);

		assert!(
			CurrencyIdToLocations::<T>::get(CurrencyId::ForeignAsset(1)).is_some(),
			"ForeignAssetId(1) not exist"
		);
		assert!(
			CurrencyIdToLocations::<T>::get(CurrencyId::ForeignAsset(2)).is_some(),
			"ForeignAssetId(2) not exist"
		);
		assert!(
			CurrencyIdToLocations::<T>::get(CurrencyId::ForeignAsset(3)).is_some(),
			"ForeignAssetId(3) not exist"
		);

		log::info!(
			"try-runtime::pre_upgrade NextForeignAssetId value: {:?}",
			NextForeignAssetId::<T>::get()
		);
		log::info!(
			"try-runtime::pre_upgrade AssetMetadatas ForeignAssetId(0): {:?}",
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(0)).is_some()
		);
		log::info!(
			"try-runtime::pre_upgrade CurrencyIdToLocations ForeignAssetId(0): {:?}",
			CurrencyIdToLocations::<T>::get(CurrencyId::ForeignAsset(0)).is_some()
		);
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		assert!(NextForeignAssetId::<T>::get() == 0, "NextForeignAssetId == 0");

		assert!(
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(0)).is_none(),
			"ForeignAssetId(0) exist"
		);
		assert!(
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(1)).is_none(),
			"ForeignAssetId(1) exist"
		);
		assert!(
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(2)).is_none(),
			"ForeignAssetId(2) exist"
		);
		assert!(
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(3)).is_none(),
			"ForeignAssetId(3) exist"
		);

		assert!(
			CurrencyIdToLocations::<T>::get(CurrencyId::ForeignAsset(1)).is_none(),
			"ForeignAssetId(1) exist"
		);
		assert!(
			CurrencyIdToLocations::<T>::get(CurrencyId::ForeignAsset(2)).is_none(),
			"ForeignAssetId(2) exist"
		);
		assert!(
			CurrencyIdToLocations::<T>::get(CurrencyId::ForeignAsset(3)).is_none(),
			"ForeignAssetId(3) exist"
		);

		log::info!(
			"try-runtime::post_upgrade NextForeignAssetId value: {:?}",
			NextForeignAssetId::<T>::get()
		);
		log::info!(
			"try-runtime::post_upgrade AssetMetadatas ForeignAssetId(0): {:?}",
			AssetMetadatas::<T>::get(AssetIds::ForeignAssetId(0)).is_some()
		);
		log::info!(
			"try-runtime::post_upgrade CurrencyIdToLocations ForeignAssetId(0): {:?}",
			CurrencyIdToLocations::<T>::get(CurrencyId::ForeignAsset(0)).is_some()
		);
		Ok(())
	}
}

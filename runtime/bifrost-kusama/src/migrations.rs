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

use bifrost_asset_registry::*;
use frame_support::{pallet_prelude::Weight, traits::Get};

pub struct AssetRegistryMigration<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> frame_support::traits::OnRuntimeUpgrade for AssetRegistryMigration<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		assert!(NextTokenId::<T>::get() == 0u8, "NextTokenId == 0");

		log::info!("try-runtime::pre_upgrade NextTokenId value: {:?}", NextTokenId::<T>::get());
		Ok(())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let mut len = Weight::default();
		let result = bifrost_asset_registry::Pallet::<T>::get_next_token_id();
		log::info!("try-runtime:: NextTokenId++ result: {:?}", result);
		len += 1;
		<T as frame_system::Config>::DbWeight::get().reads_writes(len, len)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		assert!(NextTokenId::<T>::get() == 1u8, "NextTokenId == 1");

		log::info!("try-runtime::pre_upgrade NextTokenId value: {:?}", NextTokenId::<T>::get());
		Ok(())
	}
}

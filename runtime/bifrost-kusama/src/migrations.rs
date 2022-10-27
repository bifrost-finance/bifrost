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

use crate::AssetRegistry;
use bifrost_asset_registry::Config;
use frame_support::{pallet_prelude::Weight, traits::Get};
use sp_std::vec;

pub struct AssetRegistryMigration<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> frame_support::traits::OnRuntimeUpgrade for AssetRegistryMigration<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		use node_primitives::CurrencyId::*;
		use xcm::latest::prelude::*;

		let mut len = Weight::default();

		// Token
		let items = vec![(
			Token2(AssetRegistry::next_token_id()),
			"Tether USD",
			"USDT",
			6u8,
			1_000_u128,
			MultiLocation::new(1, X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984))),
			0u128,
		)];
		for (currency_id, metadata, location, weight) in items.iter().map(
			|(currency_id, name, symbol, decimals, minimal_balance, location, weight)| {
				(
					currency_id,
					bifrost_asset_registry::AssetMetadata {
						name: name.as_bytes().to_vec(),
						symbol: symbol.as_bytes().to_vec(),
						decimals: *decimals,
						minimal_balance: *minimal_balance,
					},
					location,
					weight,
				)
			},
		) {
			AssetRegistry::do_remove_multilocation(location);
			AssetRegistry::do_register_metadata(*currency_id, &metadata).expect("Token register");
			AssetRegistry::do_register_multilocation(*currency_id, &location)
				.expect("MultiLocation register");
			AssetRegistry::do_register_weight(*currency_id, *weight).expect("Weight register");
		}
		len += (items.len() * 4) as Weight;

		<T as frame_system::Config>::DbWeight::get().reads_writes(len, len)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		log::info!(
			"try-runtime::pre_upgrade currency_metadatas count: {:?}",
			bifrost_asset_registry::CurrencyMetadatas::<T>::iter().count()
		);

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		let count = bifrost_asset_registry::CurrencyMetadatas::<T>::iter().count();
		log::info!("try-runtime::post_upgrade currency_metadatas count: {:?}", count);

		Ok(())
	}
}

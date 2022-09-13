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

use crate::{AssetIdMaps, AssetRegistry};
use bifrost_asset_registry::Config;
use frame_support::{pallet_prelude::Weight, traits::Get};
use node_primitives::CurrencyIdRegister;
use sp_std::vec;

pub struct AssetRegistryMigration<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> frame_support::traits::OnRuntimeUpgrade for AssetRegistryMigration<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		use bifrost_runtime_common::{cent, micro, milli, millicent};
		use node_primitives::{CurrencyId::*, TokenInfo, TokenSymbol::*};

		let mut len = Weight::default();

		// Token
		let items = vec![
			(Native(BNC), 10 * milli(Native(BNC))),
			(Stable(KUSD), 10 * millicent(Stable(KUSD))),
			(Token(KSM), 10 * millicent(Token(KSM))),
			(Token(ZLK), 1 * micro(Token(ZLK))),
			(Token(KAR), 10 * millicent(Token(KAR))),
			(Token(RMRK), 1 * micro(Token(RMRK))),
			(Token(PHA), 4 * cent(Token(PHA))),
			(Token(MOVR), 1 * micro(Token(MOVR))),
		];
		for (currency_id, metadata) in items.iter().map(|(currency_id, minimal_balance)| {
			(
				currency_id,
				bifrost_asset_registry::AssetMetadata {
					name: currency_id.name().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
					symbol: currency_id.symbol().map(|s| s.as_bytes().to_vec()).unwrap_or_default(),
					decimals: currency_id.decimals().unwrap_or_default(),
					minimal_balance: *minimal_balance,
				},
			)
		}) {
			AssetRegistry::do_register_metadata(*currency_id, &metadata).expect("Token register");
		}
		len += items.len() as Weight;

		// vToken
		AssetIdMaps::<T>::register_vtoken_metadata(KSM).expect("VToken register");
		AssetIdMaps::<T>::register_vtoken_metadata(MOVR).expect("VToken register");
		// vsToken
		AssetIdMaps::<T>::register_vstoken_metadata(KSM).expect("VSToken register");
		len += 3 as Weight;

		// vsBond
		let items = vec![
			(BNC, 2001u32, 13u32, 20u32),
			(KSM, 2011, 19, 26),
			(KSM, 2085, 15, 22),
			(KSM, 2087, 17, 24),
			(KSM, 2088, 15, 22),
			(KSM, 2090, 15, 22),
			(KSM, 2092, 15, 22),
			(KSM, 2095, 17, 24),
			(KSM, 2096, 17, 24),
			(KSM, 2100, 18, 25),
			(KSM, 2101, 18, 25),
			(KSM, 2102, 19, 26),
			(KSM, 2102, 21, 28),
			(KSM, 2102, 20, 27),
			(KSM, 2106, 19, 26),
			(KSM, 2114, 20, 27),
			(KSM, 2118, 22, 29),
			(KSM, 2119, 22, 29),
			(KSM, 2121, 22, 29),
			(KSM, 2124, 23, 30),
			(KSM, 2125, 23, 30),
			(KSM, 2127, 23, 30),
			(KSM, 2129, 24, 31),
		];
		for (symbol, para_id, first_slot, last_slot) in items.iter() {
			AssetIdMaps::<T>::register_vsbond_metadata(*symbol, *para_id, *first_slot, *last_slot)
				.expect("VSBond register");
		}
		len += items.len() as Weight;

		<T as frame_system::Config>::DbWeight::get().reads_writes(len, len)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		Ok(())
	}
}

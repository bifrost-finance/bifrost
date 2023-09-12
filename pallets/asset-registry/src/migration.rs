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

#![cfg_attr(not(feature = "std"), no_std)]

use super::{AssetMetadata, Config, CurrencyMetadatas, Weight};
use frame_support::traits::Get;
use primitives::CurrencyId;

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

	T::DbWeight::get().reads(pool_count) + T::DbWeight::get().writes(pool_count)
}

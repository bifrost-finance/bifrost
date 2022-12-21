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

// #![cfg_attr(not(feature = "std"), no_std)]

use super::{Config, Weight};
use crate::{CurrencyIdToLocations, LocationToCurrencyIds, MultiLocation};
use frame_support::traits::Get;
use primitives::CurrencyId;
use xcm::opaque::latest::{Junction, Junction::Parachain, Junctions::Here};

pub fn update_currency_multilocations<T: Config>() -> Weight {
	CurrencyIdToLocations::<T>::translate::<MultiLocation, _>(|currency_id, location| {
		let new_location = match currency_id {
			CurrencyId::VToken(_) |
			CurrencyId::VSToken(_) |
			CurrencyId::VToken2(_) |
			CurrencyId::Native(_) => {
				let lct = non_chain_part(location.clone());
				LocationToCurrencyIds::<T>::remove(location.clone());
				LocationToCurrencyIds::<T>::insert(lct.clone(), currency_id);
				lct
			},
			_ => location,
		};

		Some(new_location)
	});

	let entry_count = CurrencyIdToLocations::<T>::iter().count() as u64;

	T::DbWeight::get().reads(entry_count * 2) + T::DbWeight::get().writes(entry_count * 3)
}

fn non_chain_part(location: MultiLocation) -> MultiLocation {
	let mut junctions = location.interior().clone();
	while is_chain_junction(junctions.first()) {
		let _ = junctions.take_first();
	}

	if junctions != Here {
		MultiLocation::new(0, junctions)
	} else {
		location
	}
}

fn is_chain_junction(junction: Option<&Junction>) -> bool {
	matches!(junction, Some(Parachain(_)))
}

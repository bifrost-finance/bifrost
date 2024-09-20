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

use bifrost_asset_registry::AssetIdMaps;
use bifrost_primitives::{CurrencyId, CurrencyIdMapping};
use cumulus_primitives_core::ParaId;
use frame_support::traits::Get;
use sp_runtime::traits::Convert;
use sp_std::{marker::PhantomData, prelude::*};
use xcm::{
	latest::{AssetId, Location},
	prelude::Fungible,
	v4::Asset,
};

pub struct CurrencyIdConvert<T, R>(PhantomData<(T, R)>);
/// Convert CurrencyId to Location
impl<T: Get<ParaId>, R: bifrost_asset_registry::Config> Convert<CurrencyId, Option<Location>>
	for CurrencyIdConvert<T, R>
{
	fn convert(id: CurrencyId) -> Option<Location> {
		AssetIdMaps::<R>::get_location(id)
	}
}
/// Convert Location to CurrencyId
impl<T: Get<ParaId>, R: bifrost_asset_registry::Config> Convert<Location, Option<CurrencyId>>
	for CurrencyIdConvert<T, R>
{
	fn convert(location: Location) -> Option<CurrencyId> {
		AssetIdMaps::<R>::get_currency_id(location.clone())
	}
}
/// Convert Asset to CurrencyId
impl<T: Get<ParaId>, R: bifrost_asset_registry::Config> Convert<Asset, Option<CurrencyId>>
	for CurrencyIdConvert<T, R>
{
	fn convert(asset: Asset) -> Option<CurrencyId> {
		if let Asset { id: AssetId(id), fun: Fungible(_) } = asset {
			Self::convert(id)
		} else {
			None
		}
	}
}

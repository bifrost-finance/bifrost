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

use bifrost_asset_registry::AssetIdMaps;
use codec::{Decode, Encode};
pub use cumulus_primitives_core::ParaId;
use frame_support::{
	sp_runtime::traits::{CheckedConversion, Convert},
	traits::Get,
};
use node_primitives::{AccountId, CurrencyId, CurrencyIdMapping, TokenSymbol};
use orml_traits::location::Reserve;
use sp_std::{convert::TryFrom, marker::PhantomData, prelude::*};
use xcm::latest::prelude::*;
use xcm_executor::traits::{FilterAssetLocation, MatchesFungible};
use xcm_interface::traits::parachains;

use super::Runtime;

/// Bifrost Asset Matcher
pub struct BifrostAssetMatcher<CurrencyId, CurrencyIdConvert>(
	PhantomData<(CurrencyId, CurrencyIdConvert)>,
);

impl<CurrencyId, CurrencyIdConvert, Amount> MatchesFungible<Amount>
	for BifrostAssetMatcher<CurrencyId, CurrencyIdConvert>
where
	CurrencyIdConvert: Convert<MultiLocation, Option<CurrencyId>>,
	Amount: TryFrom<u128>,
{
	fn matches_fungible(a: &MultiAsset) -> Option<Amount> {
		if let (Fungible(ref amount), Concrete(ref location)) = (&a.fun, &a.id) {
			if CurrencyIdConvert::convert(location.clone()).is_some() {
				return CheckedConversion::checked_from(*amount);
			}
		}
		None
	}
}

/// A `FilterAssetLocation` implementation. Filters multi native assets whose
/// reserve is same with `origin`.
pub struct MultiNativeAsset<ReserveProvider>(PhantomData<ReserveProvider>);
impl<ReserveProvider> FilterAssetLocation for MultiNativeAsset<ReserveProvider>
where
	ReserveProvider: Reserve,
{
	fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		if let Some(ref reserve) = ReserveProvider::reserve(asset) {
			if reserve == origin {
				return true;
			}
		}
		false
	}
}

fn native_currency_location(id: CurrencyId, para_id: ParaId) -> MultiLocation {
	MultiLocation::new(
		1,
		X2(Parachain(para_id.into()), GeneralKey((id.encode()).try_into().unwrap())),
	)
}

impl<T: Get<ParaId>> Convert<MultiAsset, Option<CurrencyId>> for BifrostCurrencyIdConvert<T> {
	fn convert(asset: MultiAsset) -> Option<CurrencyId> {
		if let MultiAsset { id: Concrete(id), fun: Fungible(_) } = asset {
			Self::convert(id)
		} else {
			None
		}
	}
}

pub struct BifrostAccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for BifrostAccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(AccountId32 { network: NetworkId::Any, id: account.into() }).into()
	}
}

pub struct BifrostCurrencyIdConvert<T>(sp_std::marker::PhantomData<T>);
impl<T: Get<ParaId>> Convert<CurrencyId, Option<MultiLocation>> for BifrostCurrencyIdConvert<T> {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		use CurrencyId::*;
		use TokenSymbol::*;

		if let Some(id) = AssetIdMaps::<Runtime>::get_multi_location(id) {
			return Some(id);
		}

		match id {
			Token(KSM) => Some(MultiLocation::parent()),
			Native(ASG) | Native(BNC) | VSToken(KSM) | Token(ZLK) =>
				Some(native_currency_location(id, T::get())),
			// Karura currencyId types
			Token(KAR) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					GeneralKey((parachains::karura::KAR_KEY.to_vec()).try_into().unwrap()),
				),
			)),
			Stable(KUSD) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					GeneralKey((parachains::karura::KUSD_KEY.to_vec()).try_into().unwrap()),
				),
			)),
			Token(RMRK) => Some(MultiLocation::new(
				1,
				X3(
					Parachain(parachains::Statemine::ID),
					PalletInstance(parachains::Statemine::PALLET_ID),
					GeneralIndex(parachains::Statemine::RMRK_ID as u128),
				),
			)),
			// Phala Native token
			Token(PHA) => Some(MultiLocation::new(1, X1(Parachain(parachains::phala::ID)))),
			// Moonriver Native token
			Token(MOVR) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::moonriver::ID),
					PalletInstance(parachains::moonriver::PALLET_ID.into()),
				),
			)),
			_ => None,
		}
	}
}

impl<T: Get<ParaId>> Convert<MultiLocation, Option<CurrencyId>> for BifrostCurrencyIdConvert<T> {
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		use CurrencyId::*;
		use TokenSymbol::*;

		if location == MultiLocation::parent() {
			return Some(Token(KSM));
		}

		if let Some(currency_id) = AssetIdMaps::<Runtime>::get_currency_id(location.clone()) {
			return Some(currency_id);
		}

		match location {
			MultiLocation { parents, interior } if parents == 1 => match interior {
				X2(Parachain(id), GeneralKey(key)) if ParaId::from(id) == T::get() => {
					// decode the general key
					if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
						match currency_id {
							Native(ASG) | Native(BNC) | VSToken(KSM) | Token(ZLK) =>
								Some(currency_id),
							_ => None,
						}
					} else {
						None
					}
				},
				X2(Parachain(id), GeneralKey(key)) if id == parachains::karura::ID => {
					if key == parachains::karura::KAR_KEY.to_vec() {
						Some(Token(KAR))
					} else if key == parachains::karura::KUSD_KEY.to_vec() {
						Some(Stable(KUSD))
					} else {
						None
					}
				},
				X2(Parachain(id), GeneralIndex(key)) if id == parachains::Statemine::ID => {
					if key == parachains::Statemine::RMRK_ID as u128 {
						Some(Token(RMRK))
					} else {
						None
					}
				},
				X3(Parachain(id), PalletInstance(index), GeneralIndex(key))
					if (id == parachains::Statemine::ID &&
						index == parachains::Statemine::PALLET_ID) =>
					if key == parachains::Statemine::RMRK_ID as u128 {
						Some(Token(RMRK))
					} else {
						None
					},
				X1(Parachain(id)) if id == parachains::phala::ID => Some(Token(PHA)),
				X2(Parachain(id), PalletInstance(index))
					if ((id == parachains::moonriver::ID) &&
						(index == parachains::moonriver::PALLET_ID)) =>
					Some(Token(MOVR)),
				_ => None,
			},
			MultiLocation { parents, interior } if parents == 0 => match interior {
				X1(GeneralKey(key)) => {
					// decode the general key
					if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
						match currency_id {
							Native(ASG) | Native(BNC) | VToken(KSM) | VSToken(KSM) | Token(ZLK) =>
								Some(currency_id),
							_ => None,
						}
					} else {
						None
					}
				},
				_ => None,
			},
			_ => None,
		}
	}
}

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
use node_primitives::{
	AccountId, AssetIdMapping, CurrencyId, CurrencyId::ForeignAsset, TokenSymbol,
};
use orml_traits::location::Reserve;
use sp_std::{convert::TryFrom, marker::PhantomData};
use xcm::latest::prelude::*;
use xcm_builder::NativeAsset;
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

/// Bifrost Filtered Assets
pub struct BifrostFilterAsset;

impl FilterAssetLocation for BifrostFilterAsset {
	fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		if let Some(ref reserve) = asset.reserve() {
			if reserve == origin {
				return true;
			}
		}
		false
	}
}

pub type BifrostFilteredAssets = (NativeAsset, BifrostFilterAsset);

fn native_currency_location(id: CurrencyId, para_id: ParaId) -> MultiLocation {
	MultiLocation::new(1, X2(Parachain(para_id.into()), GeneralKey(id.encode())))
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
		use CurrencyId::{Native, Stable, Token, VSToken};
		match id {
			Token(TokenSymbol::KSM) => Some(MultiLocation::parent()),
			Native(TokenSymbol::ASG) |
			Native(TokenSymbol::BNC) |
			VSToken(TokenSymbol::KSM) |
			Token(TokenSymbol::ZLK) => Some(native_currency_location(id, T::get())),
			// Karura currencyId types
			Token(TokenSymbol::KAR) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					GeneralKey(parachains::karura::KAR_KEY.to_vec()),
				),
			)),
			Stable(TokenSymbol::KUSD) => Some(MultiLocation::new(
				1,
				X2(
					Parachain(parachains::karura::ID),
					GeneralKey(parachains::karura::KUSD_KEY.to_vec()),
				),
			)),
			Token(TokenSymbol::RMRK) => Some(MultiLocation::new(
				1,
				X3(
					Parachain(parachains::Statemine::ID),
					PalletInstance(parachains::Statemine::PALLET_ID),
					GeneralIndex(parachains::Statemine::RMRK_ID as u128),
				),
			)),
			// Phala Native token
			Token(TokenSymbol::PHA) =>
				Some(MultiLocation::new(1, X1(Parachain(parachains::phala::ID)))),
			ForeignAsset(foreign_asset_id) =>
				AssetIdMaps::<Runtime>::get_multi_location(foreign_asset_id),
			_ => None,
		}
	}
}

impl<T: Get<ParaId>> Convert<MultiLocation, Option<CurrencyId>> for BifrostCurrencyIdConvert<T> {
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		use CurrencyId::{Native, Stable, Token, VSToken};
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
							Native(TokenSymbol::ASG) |
							Native(TokenSymbol::BNC) |
							VSToken(TokenSymbol::KSM) |
							Token(TokenSymbol::ZLK) => Some(currency_id),
							_ => None,
						}
					} else {
						None
					}
				},
				X2(Parachain(id), GeneralKey(key)) if id == parachains::karura::ID => {
					if key == parachains::karura::KAR_KEY.to_vec() {
						Some(Token(TokenSymbol::KAR))
					} else if key == parachains::karura::KUSD_KEY.to_vec() {
						Some(Stable(TokenSymbol::KUSD))
					} else {
						None
					}
				},
				X2(Parachain(id), GeneralIndex(key)) if id == parachains::Statemine::ID => {
					if key == parachains::Statemine::RMRK_ID as u128 {
						Some(Token(TokenSymbol::RMRK))
					} else {
						None
					}
				},
				X3(Parachain(id), PalletInstance(index), GeneralIndex(key))
					if (id == parachains::Statemine::ID &&
						index == parachains::Statemine::PALLET_ID) =>
					if key == parachains::Statemine::RMRK_ID as u128 {
						Some(Token(TokenSymbol::RMRK))
					} else {
						None
					},
				X1(Parachain(id)) if id == parachains::phala::ID => Some(Token(TokenSymbol::PHA)),
				_ => None,
			},
			MultiLocation { parents, interior } if parents == 0 => match interior {
				X1(GeneralKey(key)) => {
					// decode the general key
					if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
						match currency_id {
							Native(TokenSymbol::ASG) |
							Native(TokenSymbol::BNC) |
							VSToken(TokenSymbol::KSM) |
							Token(TokenSymbol::ZLK) => Some(currency_id),
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

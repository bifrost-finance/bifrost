// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use codec::{Decode, Encode};
pub use cumulus_primitives_core::ParaId;
use frame_support::{
	sp_runtime::traits::{CheckedConversion, Convert},
	traits::{Contains, Get},
	weights::Weight,
};
use node_primitives::{AccountId, CurrencyId, TokenSymbol, TokenInfo};
use polkadot_parachain::primitives::Sibling;
use sp_std::{convert::TryFrom, marker::PhantomData};
use xcm::v0::Junction;
pub use xcm::v0::{
	Junction::{AccountId32, GeneralKey, Parachain, Parent},
	MultiAsset,
	MultiLocation::{self, X1, X2, X3},
	NetworkId, Xcm,
};
use xcm_builder::{AccountId32Aliases, NativeAsset, ParentIsDefault, SiblingParachainConvertsVia};
use xcm_executor::traits::{FilterAssetLocation, MatchesFungible, ShouldExecute};

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
		if let MultiAsset::ConcreteFungible { id, amount } = a {
			if CurrencyIdConvert::convert(id.clone()).is_some() {
				return CheckedConversion::checked_from(*amount);
			}
		}
		None
	}
}

/// Bifrost Location Convert
pub type BifrostLocationConvert = (
	// The parent (Relay-chain) origin converts to the default `AccountId`.
	ParentIsDefault<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<NetworkId, AccountId>,
);

/// Bifrost Xcm Transact Filter
pub struct BifrostXcmTransactFilter<T>(PhantomData<T>);

impl<T: Contains<MultiLocation>> ShouldExecute for BifrostXcmTransactFilter<T> {
	fn should_execute<Call>(
		_origin: &MultiLocation,
		_top_level: bool,
		message: &Xcm<Call>,
		_shallow_weight: Weight,
		_weight_credit: &mut Weight,
	) -> Result<(), ()> {
		match message {
			Xcm::Transact { origin_type: _, require_weight_at_most: _, call: _ } => Ok(()),
			_ => Err(()),
		}
	}
}

/// Bifrost Filtered Assets
pub struct BifrostFilterAsset;

impl FilterAssetLocation for BifrostFilterAsset {
	fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		match asset {
			MultiAsset::ConcreteFungible { .. } => match origin {
				X1(Junction::Plurality { .. }) => true,
				X1(Junction::AccountId32 { .. }) => true,
				X1(Junction::Parent { .. }) => true,
				X1(Junction::Parachain { .. }) => true,
				X2(Junction::Parachain { .. }, _) => true,
				X2(Junction::Parent { .. }, _) => true,
				_ => false,
			},
			_ => false,
		}
	}
}

pub type BifrostFilteredAssets = (NativeAsset, BifrostFilterAsset);

fn native_currency_location(id: CurrencyId, para_id: ParaId) -> MultiLocation {
	X3(Parent, Parachain(para_id.into()), GeneralKey(id.encode()))
}

pub struct BifrostCurrencyIdConvert<T>(sp_std::marker::PhantomData<T>);
impl<T: Get<ParaId>> Convert<CurrencyId, Option<MultiLocation>> for BifrostCurrencyIdConvert<T> {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		use CurrencyId::{Native, Token};
		match id {
			Token(TokenSymbol::KSM) => Some(X1(Parent)),
			Native(TokenSymbol::ASG) | Native(TokenSymbol::BNC) =>
				Some(native_currency_location(id, T::get())),
			// Karura currencyId types
			Token(TokenSymbol::KAR) =>
				Some(X3(Parent, Parachain(2000), GeneralKey([0,128].to_vec()))),
			_ => None,
		}
	}
}
impl<T: Get<ParaId>> Convert<MultiLocation, Option<CurrencyId>> for BifrostCurrencyIdConvert<T> {
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		use CurrencyId::{Native, Token};
		use TokenSymbol::*;
		match location {
			X1(Parent) => Some(Token(KSM)),
			X3(Parent, Junction::Parachain(id), GeneralKey(key)) => {
				// check `currency_id` is cross-chain asset
				if ParaId::from(id) == T::get() {
					// decode the general key
					if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
						match currency_id {
							Token(TokenSymbol::ASG) | Native(TokenSymbol::ASG) =>  Some(Native(TokenSymbol::ASG)),
							Token(TokenSymbol::BNC) | Native(TokenSymbol::BNC) =>  Some(Native(TokenSymbol::BNC)),
							_ => None,
						}
					} else {
						None
					}
					// Kurara CurrencyId types
				} else if id == 2000 {
					let kar_vec = [0, 128].to_vec();
					if key == kar_vec {
						Some(Token(TokenSymbol::KAR))
					} else {
						None
					}
				} else {
					None
				}
			},
			_ => None,
		}
			
	}
}
impl<T: Get<ParaId>> Convert<MultiAsset, Option<CurrencyId>> for BifrostCurrencyIdConvert<T> {
	fn convert(asset: MultiAsset) -> Option<CurrencyId> {
		if let MultiAsset::ConcreteFungible { id, amount: _ } = asset {
			Self::convert(id)
		} else {
			None
		}
	}
}

pub struct BifrostAccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for BifrostAccountIdToMultiLocation {
	fn convert(account: AccountId) -> MultiLocation {
		X1(AccountId32 { network: NetworkId::Any, id: account.into() })
	}
}

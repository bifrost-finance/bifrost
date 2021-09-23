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
	sp_runtime::traits::{CheckedConversion, Convert, Zero},
	traits::{Contains, Get},
	weights::Weight,
};
use node_primitives::{AccountId, CurrencyId, TokenSymbol};
use polkadot_parachain::primitives::Sibling;
use sp_std::{convert::TryFrom, marker::PhantomData};
use xcm::v0::Junction;
pub use xcm::v0::{
	Error as XcmError,
	Junction::{AccountId32, GeneralKey, Parachain, Parent},
	MultiAsset,
	MultiLocation::{self, X1, X2, X3},
	NetworkId, Xcm,
};
use xcm_builder::{AccountId32Aliases, NativeAsset, ParentIsDefault, SiblingParachainConvertsVia};
use xcm_executor::{
	traits::{FilterAssetLocation, MatchesFungible, ShouldExecute, WeightTrader},
	Assets,
};

use crate::constants::parachains;

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
		use CurrencyId::{Native, Stable, Token, VSToken};
		match id {
			Token(TokenSymbol::KSM) => Some(X1(Parent)),
			Native(TokenSymbol::ASG) | Native(TokenSymbol::BNC) | VSToken(TokenSymbol::KSM) =>
				Some(native_currency_location(id, T::get())),
			// Karura currencyId types
			Token(TokenSymbol::KAR) => Some(X3(
				Parent,
				Parachain(parachains::karura::ID),
				GeneralKey(parachains::karura::KAR_KEY.to_vec()),
			)),
			Stable(TokenSymbol::KUSD) => Some(X3(
				Parent,
				Parachain(parachains::karura::ID),
				GeneralKey(parachains::karura::KUSD_KEY.to_vec()),
			)),
			_ => None,
		}
	}
}
impl<T: Get<ParaId>> Convert<MultiLocation, Option<CurrencyId>> for BifrostCurrencyIdConvert<T> {
	fn convert(location: MultiLocation) -> Option<CurrencyId> {
		use CurrencyId::{Native, Stable, Token, VSToken};
		use TokenSymbol::*;
		match location {
			X1(Parent) => Some(Token(KSM)),
			X3(Parent, Junction::Parachain(id), GeneralKey(key)) => {
				// check `currency_id` is cross-chain asset
				if ParaId::from(id) == T::get() {
					// decode the general key
					if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
						match currency_id {
							Native(TokenSymbol::ASG) |
							Native(TokenSymbol::BNC) |
							VSToken(TokenSymbol::KSM) => Some(currency_id),
							_ => None,
						}
					} else {
						None
					}
				// Kurara CurrencyId types
				} else if id == parachains::karura::ID {
					if key == parachains::karura::KAR_KEY.to_vec() {
						Some(Token(TokenSymbol::KAR))
					} else if key == parachains::karura::KUSD_KEY.to_vec() {
						Some(Stable(TokenSymbol::KUSD))
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

// The implementation of multiple fee trader
pub struct MultiWeightTraders<KsmTrader, BncTrader, KarTrader, KusdTrader, VsksmTrader> {
	ksm_trader: KsmTrader,
	bnc_trader: BncTrader,
	kar_trader: KarTrader,
	kusd_trader: KusdTrader,
	vsksm_trader: VsksmTrader,
}

impl<
		KsmTrader: WeightTrader,
		BncTrader: WeightTrader,
		KarTrader: WeightTrader,
		KusdTrader: WeightTrader,
		VsksmTrader: WeightTrader,
	> WeightTrader for MultiWeightTraders<KsmTrader, BncTrader, KarTrader, KusdTrader, VsksmTrader>
{
	fn new() -> Self {
		Self {
			ksm_trader: KsmTrader::new(),
			bnc_trader: BncTrader::new(),
			kar_trader: KarTrader::new(),
			kusd_trader: KusdTrader::new(),
			vsksm_trader: VsksmTrader::new(),
			// dummy_trader: DummyTrader::new(),
		}
	}
	fn buy_weight(&mut self, weight: Weight, payment: Assets) -> Result<Assets, XcmError> {
		if let Ok(assets) = self.ksm_trader.buy_weight(weight, payment.clone()) {
			return Ok(assets);
		}

		if let Ok(assets) = self.bnc_trader.buy_weight(weight, payment.clone()) {
			return Ok(assets);
		}

		if let Ok(assets) = self.kar_trader.buy_weight(weight, payment.clone()) {
			return Ok(assets);
		}

		if let Ok(assets) = self.kusd_trader.buy_weight(weight, payment.clone()) {
			return Ok(assets);
		}

		if let Ok(assets) = self.vsksm_trader.buy_weight(weight, payment) {
			return Ok(assets);
		}

		// if let Ok(asset) = self.dummy_trader.buy_weight(weight, payment) {
		// 	return Ok(assets)
		// }

		Err(XcmError::TooExpensive)
	}
	fn refund_weight(&mut self, weight: Weight) -> MultiAsset {
		let ksm = self.ksm_trader.refund_weight(weight);
		match ksm {
			MultiAsset::ConcreteFungible { amount, .. } if !amount.is_zero() => return ksm,
			_ => {},
		}

		let bnc = self.bnc_trader.refund_weight(weight);
		match bnc {
			MultiAsset::ConcreteFungible { amount, .. } if !amount.is_zero() => return bnc,
			_ => {},
		}

		let kar = self.kar_trader.refund_weight(weight);
		match kar {
			MultiAsset::ConcreteFungible { amount, .. } if !amount.is_zero() => return kar,
			_ => {},
		}

		let kusd = self.kusd_trader.refund_weight(weight);
		match kusd {
			MultiAsset::ConcreteFungible { amount, .. } if !amount.is_zero() => return kusd,
			_ => {},
		}

		let vsksm = self.kusd_trader.refund_weight(weight);
		match vsksm {
			MultiAsset::ConcreteFungible { amount, .. } if !amount.is_zero() => return vsksm,
			_ => {},
		}

		// let dummy = self.dummy_trader.refund_weight(weight);
		// match dummy {
		// 	MultiAsset::ConcreteFungible { amount, .. } if !amount.is_zero() => return dummy,
		// 	_ => {},
		// }

		MultiAsset::None
	}
}

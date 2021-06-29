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

//! # XCM Support Module.
//!
//! ## Overview
//!
//! The XCM support module provides supporting traits, types and
//! implementations, to support cross-chain message(XCM) integration with ORML
//! modules.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

#[allow(unused_must_use)]
use codec::FullCodec;
use frame_support::{dispatch::Weight, traits::Contains};
use polkadot_parachain::primitives::Sibling;
use sp_runtime::traits::{CheckedConversion, Convert, MaybeSerializeDeserialize};
use sp_std::{
	cmp::{Eq, PartialEq},
	convert::TryFrom,
	fmt::Debug,
	marker::PhantomData,
	prelude::*,
};
use xcm::{
	v0::{
		prelude::{XcmError, XcmResult},
		Junction, MultiAsset, MultiLocation,
		MultiLocation::{Null, X1, X2},
		NetworkId, OriginKind, SendXcm, Xcm,
	},
	DoubleEncoded,
};
use xcm_builder::{AccountId32Aliases, NativeAsset, ParentIsDefault, SiblingParachainConvertsVia};
use xcm_executor::traits::{
	Convert as xcmConvert, FilterAssetLocation, MatchesFungible, ShouldExecute, TransactAsset,
};

use node_primitives::{traits::BifrostXcmExecutor, AccountId, CurrencyId, TokenSymbol};

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

/// Bifrost Currency Convert
pub struct BifrostCurrencyIdConvert;

impl Convert<MultiLocation, Option<CurrencyId>> for BifrostCurrencyIdConvert {
	fn convert(l: MultiLocation) -> Option<CurrencyId> {
		#[allow(unused_variables)]
		{
			let para_id: u32;
			match l {
				X2(Junction::Parent, Junction::Parachain(para_id)) => {
					Some(CurrencyId::Token(TokenSymbol::KSM))
				}
				_ => None,
			}
		}
	}
}

impl xcmConvert<MultiAsset, Option<CurrencyId>> for BifrostCurrencyIdConvert {
	fn convert(a: MultiAsset) -> Result<Option<CurrencyId>, MultiAsset> {
		if let MultiAsset::ConcreteFungible { id, amount: _ } = a {
			return Ok(<Self as Convert<MultiLocation, Option<CurrencyId>>>::convert(id));
		}
		Err(MultiAsset::None)
	}
}

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
			Xcm::Transact {
				origin_type: _,
				require_weight_at_most: _,
				call: _,
			} => Ok(()),
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
				Null | X1(Junction::Plurality { .. }) => true,
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

/// The `TransactAsset` implementation, to handle `MultiAsset` deposit/withdraw.
///
/// If the asset is known, deposit/withdraw will be handled by `MultiCurrency`,
/// else by `UnknownAsset` if unknown.
pub struct BifrostCurrencyAdapter<
	MultiCurrency,
	Matcher,
	AccountId,
	AccountIdConvert,
	CurrencyId,
	CurrencyIdConvert,
>(
	PhantomData<(
		MultiCurrency,
		Matcher,
		AccountId,
		AccountIdConvert,
		CurrencyId,
		CurrencyIdConvert,
	)>,
);

impl<
		MultiCurrency: orml_traits::MultiCurrency<AccountId, CurrencyId = CurrencyId>,
		Matcher: MatchesFungible<MultiCurrency::Balance>,
		AccountId: sp_std::fmt::Debug + sp_std::clone::Clone,
		AccountIdConvert: xcmConvert<MultiLocation, AccountId>,
		CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug,
		CurrencyIdConvert: xcmConvert<MultiAsset, Option<CurrencyId>>,
	> TransactAsset
	for BifrostCurrencyAdapter<
		MultiCurrency,
		Matcher,
		AccountId,
		AccountIdConvert,
		CurrencyId,
		CurrencyIdConvert,
	>
{
	fn deposit_asset(asset: &MultiAsset, location: &MultiLocation) -> XcmResult {
		match (
			AccountIdConvert::convert(location.clone()),
			CurrencyIdConvert::convert(asset.clone()),
			Matcher::matches_fungible(&asset),
		) {
			// known asset
			(who, currency_id, Some(amount)) => {
				MultiCurrency::deposit(currency_id.unwrap().unwrap(), &who.unwrap(), amount)
					.map_err(|e| XcmError::FailedToTransactAsset(e.into()))
			}
			_ => Err(XcmError::AssetNotFound),
		}
	}

	fn withdraw_asset(
		asset: &MultiAsset,
		location: &MultiLocation,
	) -> Result<xcm_executor::Assets, XcmError> {
		match (
			AccountIdConvert::convert(location.clone()),
			CurrencyIdConvert::convert(asset.clone()),
			Matcher::matches_fungible(&asset),
		) {
			// known asset
			(who, currency_id, Some(amount)) => {
				MultiCurrency::withdraw(currency_id.unwrap().unwrap(), &who.unwrap(), amount)
					.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
				Ok(xcm_executor::Assets::new())
			}
			_ => Err(XcmError::AssetNotFound),
		}
	}
}

pub struct BifrostXcmAdaptor<XcmSender>(PhantomData<XcmSender>);

impl<XcmSender: SendXcm> BifrostXcmExecutor for BifrostXcmAdaptor<XcmSender> {
	fn ump_transact(_origin: MultiLocation, call: DoubleEncoded<()>) -> XcmResult {
		let message = Xcm::Transact {
			origin_type: OriginKind::SovereignAccount,
			require_weight_at_most: u64::MAX,
			call,
		};

		XcmSender::send_xcm(MultiLocation::X1(Junction::Parent), message)
	}

	fn ump_transfer_asset(
		origin: MultiLocation,
		dest: MultiLocation,
		amount: u128,
		relay: bool,
	) -> XcmResult {
		let mut message = Xcm::TransferAsset {
			assets: vec![MultiAsset::ConcreteFungible {
				id: MultiLocation::Null,
				amount,
			}],
			dest,
		};

		if relay {
			message = Xcm::<()>::RelayedFrom {
				who: origin,
				message: Box::new(message),
			};
		}

		XcmSender::send_xcm(MultiLocation::X1(Junction::Parent), message)
	}
}

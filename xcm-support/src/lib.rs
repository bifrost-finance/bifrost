//! # XCM Support Module.
//!
//! ## Overview
//!
//! The XCM support module provides supporting traits, types and
//! implementations, to support cross-chain message(XCM) integration with ORML
//! modules.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::dispatch::{Weight};
use sp_runtime::traits::{CheckedConversion, Convert};
use sp_std::{convert::TryFrom, marker::PhantomData, prelude::*};
use codec::FullCodec;
use sp_runtime::traits::{MaybeSerializeDeserialize};
use sp_std::{
	cmp::{Eq, PartialEq},
	fmt::Debug
};

use xcm::v0::{NetworkId, Xcm, MultiLocation, MultiAsset, Junction};
use xcm_executor::traits::{FilterAssetLocation, MatchesFungible, ShouldExecute, Convert as xcmConvert, TransactAsset};
use frame_support::traits::{Contains};
use node_primitives::{CurrencyId, TokenSymbol, AccountId};
use polkadot_parachain::primitives::Sibling;

use xcm::v0::{
	MultiLocation::{X1,X2,Null},
};
use xcm_builder::{ParentIsDefault, SiblingParachainConvertsVia, AccountId32Aliases, NativeAsset};
use xcm::v0::prelude::{XcmResult, XcmError};

/// Bifrost Asset Matcher
pub struct BifrostAssetMatcher<CurrencyId, CurrencyIdConvert>(PhantomData<(CurrencyId, CurrencyIdConvert)>);
impl<CurrencyId, CurrencyIdConvert, Amount> MatchesFungible<Amount> for BifrostAssetMatcher<CurrencyId, CurrencyIdConvert>
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
		#[allow(unused_variables)] {
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
			Xcm::Transact { origin_type: _ , require_weight_at_most: _, call: _ } => Ok(()),
			_ => Err(())
		}
	}
}


/// Bifrost Filtered Assets
pub struct BifrostFilterAsset;
impl FilterAssetLocation for BifrostFilterAsset {
	fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		match asset {
			MultiAsset::ConcreteFungible {..} => {
				match origin {
					Null | X1(Junction::Plurality { .. }) => true,
					X1(Junction::AccountId32 { .. }) => true,
					X1(Junction::Parent { .. }) => true,
					X1(Junction::Parachain { .. }) => true,
					X2(Junction::Parachain{..}, _ ) => true,
					X2(Junction::Parent{..}, _ ) => true,
					_ => false
				}
			},
			_ => false
		}
	}
}

pub type BifrostFilteredAssets = (
	NativeAsset,
	BifrostFilterAsset,
);

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
				#[allow(unused_must_use)] {
					MultiCurrency::deposit(currency_id.unwrap().unwrap(), &who.unwrap(), amount).map_err(|e| XcmError::FailedToTransactAsset(e.into()))
				}
			}
			_ => Err(XcmError::AssetNotFound)
		}
	}

	fn withdraw_asset(asset: &MultiAsset, location: &MultiLocation) -> Result<xcm_executor::Assets, XcmError>  {
		match (
			AccountIdConvert::convert(location.clone()),
			CurrencyIdConvert::convert(asset.clone()),
			Matcher::matches_fungible(&asset),
		) {
			// known asset
			(who, currency_id, Some(amount)) => {
				#[allow(unused_must_use)] {
					MultiCurrency::withdraw(currency_id.unwrap().unwrap(), &who.unwrap(), amount).map_err(|e| XcmError::FailedToTransactAsset(e.into()));
					Ok(xcm_executor::Assets::new())
				}
			}
			_ => Err(XcmError::AssetNotFound)
		}
	}
}

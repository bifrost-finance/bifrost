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

use codec::{Encode, FullCodec};
pub use cumulus_primitives_core::{self, ParaId};
pub use frame_support::{traits::Get, weights::Weight};
pub use paste;
use sp_runtime::traits::{Convert, MaybeSerializeDeserialize, SaturatedConversion};
pub use sp_std::{cell::RefCell, marker::PhantomData};
use sp_std::{
	cmp::{Eq, PartialEq},
	fmt::Debug,
	prelude::*,
	vec,
};
pub use xcm::VersionedXcm;
use xcm::{
	v0::{
		Error as XcmError, Junction, MultiAsset, MultiLocation, OriginKind, Result as XcmResult,
		SendXcm, Xcm,
	},
	DoubleEncoded,
};
use xcm_executor::traits::{Convert as xcmConvert, MatchesFungible, TransactAsset};
pub use xcm_executor::XcmExecutor;
mod calls;
mod traits;
pub use calls::*;
use frame_support::weights::WeightToFeePolynomial;
use node_primitives::MessageId;
pub use node_primitives::XcmBaseWeight;
pub use traits::{BifrostXcmExecutor, HandleDmpMessage, HandleUmpMessage, HandleXcmpMessage};
#[allow(unused_imports)]
use xcm::v0::{
	prelude::{Parachain, Parent, X1, X2},
	Order,
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// Asset transaction errors.
enum Error {
	/// Failed to match fungible.
	FailedToMatchFungible,
	/// `MultiLocation` to `AccountId` Conversion failed.
	AccountIdConversionFailed,
	/// `CurrencyId` conversion failed.
	CurrencyIdConversionFailed,
}

impl From<Error> for XcmError {
	fn from(e: Error) -> Self {
		match e {
			Error::FailedToMatchFungible =>
				XcmError::FailedToTransactAsset("FailedToMatchFungible"),
			Error::AccountIdConversionFailed =>
				XcmError::FailedToTransactAsset("AccountIdConversionFailed"),
			Error::CurrencyIdConversionFailed =>
				XcmError::FailedToTransactAsset("CurrencyIdConversionFailed"),
		}
	}
}

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
		CurrencyIdConvert: Convert<MultiAsset, Option<CurrencyId>>,
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
			AccountIdConvert::convert_ref(location.clone()),
			CurrencyIdConvert::convert(asset.clone()),
			Matcher::matches_fungible(&asset),
		) {
			// known asset
			(Ok(who), Some(currency_id), Some(amount)) =>
				MultiCurrency::deposit(currency_id, &who, amount)
					.map_err(|e| XcmError::FailedToTransactAsset(e.into())),
			_ => Err(XcmError::AssetNotFound),
		}
	}

	fn withdraw_asset(
		asset: &MultiAsset,
		location: &MultiLocation,
	) -> Result<xcm_executor::Assets, XcmError> {
		let who = AccountIdConvert::convert_ref(location)
			.map_err(|_| XcmError::from(Error::AccountIdConversionFailed))?;
		let currency_id = CurrencyIdConvert::convert(asset.clone())
			.ok_or_else(|| XcmError::from(Error::CurrencyIdConversionFailed))?;
		let amount: MultiCurrency::Balance = Matcher::matches_fungible(&asset)
			.ok_or_else(|| XcmError::from(Error::FailedToMatchFungible))?
			.saturated_into();
		MultiCurrency::withdraw(currency_id, &who, amount)
			.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;
		Ok(asset.clone().into())
	}
}

pub struct BifrostXcmAdaptor<XcmSender, BaseXcmWeight, WeightToFee>(
	PhantomData<(XcmSender, BaseXcmWeight, WeightToFee)>,
);

impl<
		XcmSender: SendXcm,
		BaseXcmWeight: Get<u64>,
		WeightToFee: WeightToFeePolynomial<Balance = u128>,
	> BifrostXcmExecutor for BifrostXcmAdaptor<XcmSender, BaseXcmWeight, WeightToFee>
{
	fn transact_weight(weight: u64, nonce: u32) -> u64 {
		return weight + 4 * BaseXcmWeight::get() + nonce as u64;
	}

	fn transact_id(data: &[u8]) -> MessageId {
		return sp_io::hashing::blake2_256(&data[..]);
	}

	fn ump_transact(
		origin: MultiLocation,
		call: DoubleEncoded<()>,
		weight: u64,
		relay: bool,
		nonce: u32,
	) -> Result<MessageId, XcmError> {
		let mut message = Xcm::WithdrawAsset {
			assets: vec![MultiAsset::ConcreteFungible {
				id: MultiLocation::Null,
				amount: WeightToFee::calc(&Self::transact_weight(weight, nonce)),
			}],
			effects: vec![Order::BuyExecution {
				fees: MultiAsset::All,
				weight: weight + 2 * BaseXcmWeight::get() + nonce as u64,
				debt: 2 * BaseXcmWeight::get(),
				halt_on_error: true,
				xcm: vec![Xcm::Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: u64::MAX,
					call,
				}],
			}],
		};

		if relay {
			message = Xcm::<()>::RelayedFrom { who: origin, message: Box::new(message.clone()) };
		}

		let data = VersionedXcm::<()>::from(message.clone()).encode();

		let id = Self::transact_id(&data);

		XcmSender::send_xcm(MultiLocation::X1(Junction::Parent), message)
			.map_err(|_e| XcmError::Undefined)?;

		Ok(id)
	}

	fn ump_transacts(
		origin: MultiLocation,
		calls: Vec<DoubleEncoded<()>>,
		weight: u64,
		relay: bool,
	) -> Result<MessageId, XcmError> {
		let transacts = calls
			.iter()
			.map(|call| Xcm::Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: u64::MAX,
				call: call.clone(),
			})
			.collect();
		let mut message = Xcm::WithdrawAsset {
			assets: vec![MultiAsset::ConcreteFungible {
				id: MultiLocation::Null,
				amount: WeightToFee::calc(&Self::transact_weight(weight, 0)),
			}],
			effects: vec![Order::BuyExecution {
				fees: MultiAsset::All,
				weight: weight + 4 * BaseXcmWeight::get(),
				debt: 2 * BaseXcmWeight::get(),
				halt_on_error: true,
				xcm: transacts,
			}],
		};

		if relay {
			message = Xcm::<()>::RelayedFrom { who: origin, message: Box::new(message) };
		}

		let data = VersionedXcm::<()>::from(message.clone()).encode();

		let id = Self::transact_id(&data);

		XcmSender::send_xcm(MultiLocation::X1(Junction::Parent), message)?;

		Ok(id)
	}

	fn ump_transfer_asset(
		origin: MultiLocation,
		dest: MultiLocation,
		amount: u128,
		relay: bool,
		nonce: u32,
	) -> Result<MessageId, XcmError> {
		let mut message = Xcm::WithdrawAsset {
			assets: vec![MultiAsset::ConcreteFungible { id: MultiLocation::Null, amount }],
			effects: vec![
				Order::BuyExecution {
					fees: MultiAsset::All,
					weight: nonce as u64,
					debt: 3 * BaseXcmWeight::get(),
					halt_on_error: false,
					xcm: vec![],
				},
				Order::DepositAsset { assets: vec![MultiAsset::All], dest },
			],
		};

		if relay {
			message = Xcm::<()>::RelayedFrom { who: origin, message: Box::new(message) };
		}

		let data = VersionedXcm::<()>::from(message.clone()).encode();

		let id = Self::transact_id(&data);

		XcmSender::send_xcm(MultiLocation::X1(Junction::Parent), message)?;

		Ok(id)
	}
}

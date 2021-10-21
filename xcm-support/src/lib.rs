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

use codec::Encode;
pub use frame_support::{traits::Get, weights::Weight};
pub use paste;
pub use sp_std::{cell::RefCell, marker::PhantomData};
use sp_std::{prelude::*, vec};
pub use xcm::VersionedXcm;
use xcm::{latest::prelude::*, DoubleEncoded};
mod calls;
mod traits;
pub use calls::*;
use cumulus_primitives_core::ParaId;
use frame_support::{sp_runtime::traits::AccountIdConversion, weights::WeightToFeePolynomial};
pub use node_primitives::XcmBaseWeight;
use node_primitives::{AccountId, MessageId};
pub use traits::BifrostXcmExecutor;

/// Asset transaction errors.
#[allow(dead_code)]
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

pub struct BifrostXcmAdaptor<XcmSender, BaseXcmWeight, WeightToFee, SelfParaId>(
	PhantomData<(XcmSender, BaseXcmWeight, WeightToFee, SelfParaId)>,
);

impl<
		XcmSender: SendXcm,
		BaseXcmWeight: Get<u64>,
		WeightToFee: WeightToFeePolynomial<Balance = u128>,
		SelfParaId: Get<u32>,
	> BifrostXcmExecutor for BifrostXcmAdaptor<XcmSender, BaseXcmWeight, WeightToFee, SelfParaId>
{
	fn transact_weight(weight: u64, nonce: u32) -> u64 {
		return weight + 4 * BaseXcmWeight::get() + nonce as u64;
	}

	fn transact_id(data: &[u8]) -> MessageId {
		return sp_io::hashing::blake2_256(&data[..]);
	}

	fn ump_transact(
		_origin: MultiLocation,
		call: DoubleEncoded<()>,
		weight: u64,
		_relay: bool,
		nonce: u32,
	) -> Result<MessageId, XcmError> {
		let sovereign_account: AccountId = ParaId::from(SelfParaId::get()).into_account();

		let asset: MultiAsset = MultiAsset {
			id: Concrete(MultiLocation::here()),
			fun: Fungible(WeightToFee::calc(&Self::transact_weight(weight, nonce))),
		};

		let message = Xcm(vec![
			WithdrawAsset(asset.clone().into()),
			BuyExecution {
				fees: asset,
				weight_limit: WeightLimit::Limited(Self::transact_weight(weight, nonce)),
			},
			Instruction::Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: weight,
				call,
			},
			DepositAsset {
				assets: All.into(),
				max_assets: 1,
				beneficiary: X1(Junction::AccountId32 {
					network: NetworkId::Any,
					id: sovereign_account.into(),
				})
				.into(),
			},
		]);

		let data = VersionedXcm::<()>::from(message.clone()).encode();

		let id = sp_io::hashing::blake2_256(&data[..]);

		XcmSender::send_xcm(MultiLocation::parent(), message)
			.map_err(|_e| XcmError::Unimplemented)?;

		Ok(id)
	}

	fn ump_transfer_asset(
		_origin: MultiLocation,
		dest: MultiLocation,
		amount: u128,
		_relay: bool,
		nonce: u32,
	) -> Result<MessageId, XcmError> {
		let asset = MultiAsset { id: Concrete(MultiLocation::here()), fun: Fungible(amount) };
		let message = Xcm(vec![
			WithdrawAsset(asset.clone().into()),
			BuyExecution { fees: asset, weight_limit: Some(nonce as u64).into() },
			DepositAsset { assets: Wild(WildMultiAsset::All), max_assets: 1, beneficiary: dest },
		]);

		let data = VersionedXcm::<()>::from(message.clone()).encode();

		let id = Self::transact_id(&data);

		XcmSender::send_xcm(MultiLocation::parent(), message)?;

		Ok(id)
	}
}

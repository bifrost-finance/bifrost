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
pub use cumulus_primitives_core::{self, ParaId};
pub use frame_support::{traits::Get, weights::Weight};
pub use paste;
pub use sp_std::{cell::RefCell, marker::PhantomData};
use sp_std::{prelude::*, vec};
pub use xcm::VersionedXcm;
use xcm::{latest::prelude::*, DoubleEncoded};
pub use xcm_executor::XcmExecutor;
mod calls;
mod traits;
pub use calls::*;
use frame_support::weights::WeightToFeePolynomial;
use node_primitives::MessageId;
pub use node_primitives::XcmBaseWeight;
pub use traits::{BifrostXcmExecutor, HandleDmpMessage, HandleUmpMessage, HandleXcmpMessage};
#[allow(unused_imports)]
use xcm::opaque::v0::prelude::XcmResult;
#[allow(unused_imports)]
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

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
			assets: vec![MultiAsset {
				id: Concrete(MultiLocation::here()),
				fun: Fungible(WeightToFee::calc(&Self::transact_weight(weight, nonce))),
			}]
			.into(),
			effects: vec![Order::BuyExecution {
				fees: MultiAsset {
					id: Concrete(MultiLocation::here()),
					fun: Fungible(WeightToFee::calc(&Self::transact_weight(weight, nonce))),
				},
				weight: weight + 2 * BaseXcmWeight::get() + nonce as u64,
				debt: 2 * BaseXcmWeight::get(),
				halt_on_error: true,
				instructions: vec![Xcm::Transact {
					origin_type: OriginKind::SovereignAccount,
					require_weight_at_most: u64::MAX,
					call,
				}],
			}],
		};

		if relay {
			message = Xcm::<()>::RelayedFrom {
				who: origin.interior().clone(),
				message: Box::new(message.clone()),
			};
		}

		let data = VersionedXcm::<()>::from(message.clone()).encode();

		let id = sp_io::hashing::blake2_256(&data[..]);

		XcmSender::send_xcm(MultiLocation::parent(), message).map_err(|_e| XcmError::Undefined)?;

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
			assets: vec![MultiAsset {
				id: Concrete(MultiLocation::here()),
				fun: Fungible(WeightToFee::calc(&Self::transact_weight(weight, 0))),
			}]
			.into(),
			effects: vec![Order::BuyExecution {
				fees: MultiAsset {
					id: Concrete(MultiLocation::here()),
					fun: Fungible(WeightToFee::calc(&Self::transact_weight(weight, 0))),
				},
				weight: weight + 4 * BaseXcmWeight::get(),
				debt: 2 * BaseXcmWeight::get(),
				halt_on_error: true,
				instructions: transacts,
			}],
		};

		if relay {
			message = Xcm::<()>::RelayedFrom {
				who: origin.interior().clone(),
				message: Box::new(message),
			};
		}

		let data = VersionedXcm::<()>::from(message.clone()).encode();

		let id = Self::transact_id(&data);

		XcmSender::send_xcm(MultiLocation::parent(), message)?;

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
			assets: vec![MultiAsset { id: Concrete(MultiLocation::here()), fun: Fungible(amount) }]
				.into(),
			effects: vec![
				Order::BuyExecution {
					fees: MultiAsset { id: Concrete(MultiLocation::here()), fun: Fungible(amount) },
					weight: nonce as u64,
					debt: 3 * BaseXcmWeight::get(),
					halt_on_error: false,
					instructions: vec![],
				},
				DepositAsset {
					assets: Wild(WildMultiAsset::All),
					max_assets: 1,
					beneficiary: dest,
				},
			],
		};

		if relay {
			message = Xcm::<()>::RelayedFrom {
				who: origin.interior().clone(),
				message: Box::new(message),
			};
		}

		let data = VersionedXcm::<()>::from(message.clone()).encode();

		let id = Self::transact_id(&data);

		XcmSender::send_xcm(MultiLocation::parent(), message)?;

		Ok(id)
	}
}

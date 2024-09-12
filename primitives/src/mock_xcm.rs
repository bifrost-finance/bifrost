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

use crate::{AccountId, Balance, CurrencyId};
use orml_traits::{xcm_transfer::Transferred, XcmTransfer};
use sp_runtime::DispatchError;
use sp_std::vec::Vec;
use xcm::{
	latest::Asset,
	prelude::{ExecuteXcm, Fungible, Outcome, PreparedMessage, SendResult, Xcm, XcmResult},
	v4::{AssetId, Assets, Location, SendError, SendXcm, Weight, WeightLimit, XcmHash},
};
use xcm_executor::traits::{AssetTransferError, TransferType, XcmAssetTransfers};

pub struct MockXcmRouter;
impl SendXcm for MockXcmRouter {
	type Ticket = ();
	fn validate(_dest: &mut Option<Location>, _msg: &mut Option<Xcm<()>>) -> SendResult<()> {
		Ok(((), Assets::new()))
	}
	fn deliver(_: ()) -> Result<XcmHash, SendError> {
		Ok([0; 32])
	}
}

pub struct MockXcmTransfer;
impl XcmTransfer<AccountId, Balance, CurrencyId> for MockXcmTransfer {
	fn transfer(
		who: AccountId,
		_currency_id: CurrencyId,
		amount: Balance,
		dest: Location,
		_dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError> {
		Ok(Transferred {
			sender: who,
			assets: Default::default(),
			fee: Asset { id: AssetId(Location::here()), fun: Fungible(amount) },
			dest,
		})
	}

	fn transfer_multiasset(
		_who: AccountId,
		_asset: Asset,
		_dest: Location,
		_dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError> {
		unimplemented!()
	}

	fn transfer_with_fee(
		_who: AccountId,
		_currency_id: CurrencyId,
		_amount: Balance,
		_fee: Balance,
		_dest: Location,
		_dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError> {
		unimplemented!()
	}

	fn transfer_multiasset_with_fee(
		_who: AccountId,
		_asset: Asset,
		_fee: Asset,
		_dest: Location,
		_dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError> {
		unimplemented!()
	}

	fn transfer_multicurrencies(
		_who: AccountId,
		_currencies: Vec<(CurrencyId, Balance)>,
		_fee_item: u32,
		_dest: Location,
		_dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError> {
		unimplemented!()
	}

	fn transfer_multiassets(
		_who: AccountId,
		_assets: Assets,
		_fee: Asset,
		_dest: Location,
		_dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError> {
		unimplemented!()
	}
}

pub struct Weightless;
impl PreparedMessage for Weightless {
	fn weight_of(&self) -> Weight {
		Weight::default()
	}
}

pub struct MockXcmExecutor;
impl<Call> ExecuteXcm<Call> for MockXcmExecutor {
	type Prepared = Weightless;

	fn prepare(_message: Xcm<Call>) -> Result<Self::Prepared, Xcm<Call>> {
		Ok(Weightless)
	}

	fn execute(
		_origin: impl Into<Location>,
		_pre: Self::Prepared,
		_hash: &mut XcmHash,
		_weight_credit: Weight,
	) -> Outcome {
		Outcome::Complete { used: Weight::default() }
	}

	fn charge_fees(_location: impl Into<Location>, _fees: Assets) -> XcmResult {
		Ok(())
	}
}

impl XcmAssetTransfers for MockXcmExecutor {
	type IsReserve = ();
	type IsTeleporter = ();
	type AssetTransactor = ();

	fn determine_for(_asset: &Asset, _dest: &Location) -> Result<TransferType, AssetTransferError> {
		Ok(TransferType::DestinationReserve)
	}
}

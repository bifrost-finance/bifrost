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

#![cfg(test)]

// mod hrmp_channels;
mod reserve_transfer;

pub use codec::Encode;
pub use frame_support::{
	assert_err, assert_ok,
	pallet_prelude::Weight,
	sp_runtime::{AccountId32, DispatchError, DispatchResult},
	traits::fungibles::Inspect,
};
use integration_tests_common::BifrostPolkadot;
pub use integration_tests_common::{
	constants::{
		asset_hub_polkadot::ED as ASSET_HUB_POLKADOT_ED, polkadot::ED as POLKADOT_ED,
		PROOF_SIZE_THRESHOLD, REF_TIME_THRESHOLD, XCM_V3,
	},
	xcm_helpers::{xcm_transact_paid_execution, xcm_transact_unpaid_execution},
	AssetHubPolkadot, AssetHubPolkadotPallet, AssetHubPolkadotReceiver, AssetHubPolkadotSender,
	BifrostPolkadotReceiver, PenpalPolkadotA, PenpalPolkadotAPallet, PenpalPolkadotAReceiver,
	PenpalPolkadotB, PenpalPolkadotBPallet, Polkadot, PolkadotPallet, PolkadotReceiver,
	PolkadotSender,
};
pub use parachains_common::{AccountId, Balance};
pub use xcm::{
	prelude::{AccountId32 as AccountId32Junction, *},
	v3::{Error, NetworkId::Polkadot as PolkadotId},
};
pub use xcm_emulator::{
	assert_expected_events, bx, helpers::weight_within_threshold, Chain, Parachain as Para,
	RelayChain as Relay, Test, TestArgs, TestContext, TestExt,
};

pub const ASSET_ID: u32 = 1;
pub const ASSET_MIN_BALANCE: u128 = 1000;
// `Assets` pallet index
pub const ASSETS_PALLET_ID: u8 = 50;

pub type RelayToSystemParaTest = Test<Polkadot, AssetHubPolkadot>;
pub type SystemParaToRelayTest = Test<AssetHubPolkadot, Polkadot>;
pub type SystemParaToParaTest = Test<AssetHubPolkadot, BifrostPolkadot>;

/// Returns a `TestArgs` instance to de used for the Relay Chain accross integraton tests
pub fn relay_test_args(amount: Balance) -> TestArgs {
	TestArgs {
		dest: Polkadot::child_location_of(AssetHubPolkadot::para_id()),
		beneficiary: AccountId32Junction {
			network: None,
			id: AssetHubPolkadotReceiver::get().into(),
		}
		.into(),
		amount,
		assets: (Here, amount).into(),
		asset_id: None,
		fee_asset_item: 0,
		weight_limit: WeightLimit::Unlimited,
	}
}

/// Returns a `TestArgs` instance to de used for the System Parachain accross integraton tests
pub fn system_para_test_args(
	dest: MultiLocation,
	beneficiary_id: AccountId32,
	amount: Balance,
	assets: MultiAssets,
	asset_id: Option<u32>,
) -> TestArgs {
	TestArgs {
		dest,
		beneficiary: AccountId32Junction { network: None, id: beneficiary_id.into() }.into(),
		amount,
		assets,
		asset_id,
		fee_asset_item: 0,
		weight_limit: WeightLimit::Unlimited,
	}
}

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

use frame_support::{
	pallet_prelude::Weight,
	sp_runtime::{AccountId32, DispatchResult},
};
use integration_tests_common::{
	constants::asset_hub_polkadot::ED as ASSET_HUB_POLKADOT_ED, AssetHubPolkadot,
	AssetHubPolkadotPallet, AssetHubPolkadotSender, BifrostPolkadot, BifrostPolkadotReceiver,
};
use parachains_common::Balance;
use xcm::prelude::{AccountId32 as AccountId32Junction, *};
use xcm_emulator::{
	assert_expected_events, bx, Chain, Parachain as Para, Test, TestArgs, TestContext,
};

pub const ASSET_ID: u32 = 1;
pub const ASSET_MIN_BALANCE: u128 = 1000;
// `Assets` pallet index
pub const ASSETS_PALLET_ID: u8 = 50;

// pub type RelayToSystemParaTest = Test<Polkadot, BifrostPolkadot>;
// pub type SystemParaToRelayTest = Test<AssetHubPolkadot, Polkadot>;
pub type SystemParaToParaTest = Test<AssetHubPolkadot, BifrostPolkadot>;

/// Returns a `TestArgs` instance to de used for the Relay Chain accross integraton tests
// pub fn relay_test_args(amount: Balance) -> TestArgs {
// 	TestArgs {
// 		dest: Polkadot::child_location_of(BifrostPolkadot::para_id()),
// 		beneficiary: AccountId32Junction {
// 			network: None,
// 			id: BifrostPolkadotReceiver::get().into(),
// 		}
// 		.into(),
// 		amount,
// 		assets: (Here, amount).into(),
// 		asset_id: None,
// 		fee_asset_item: 0,
// 		weight_limit: WeightLimit::Unlimited,
// 	}
// }

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

fn system_para_to_para_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

	AssetHubPolkadot::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(
		676_119_000,
		6196,
	)));

	assert_expected_events!(
		AssetHubPolkadot,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereing account
			RuntimeEvent::Balances(
				pallet_balances::Event::Transfer { from, to, amount }
			) => {
				from: *from == t.sender.account_id,
				to: *to == AssetHubPolkadot::sovereign_account_id_of(
					t.args.dest
				),
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn system_para_to_para_assets_assertions(t: SystemParaToParaTest) {
	type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

	AssetHubPolkadot::assert_xcm_pallet_attempted_complete(Some(Weight::from_parts(
		676_119_000,
		6196,
	)));

	assert_expected_events!(
		AssetHubPolkadot,
		vec![
			// Amount to reserve transfer is transferred to Parachain's Sovereing account
			RuntimeEvent::Assets(
				pallet_assets::Event::Transferred { asset_id, from, to, amount }
			) => {
				asset_id: *asset_id == ASSET_ID,
				from: *from == t.sender.account_id,
				to: *to == AssetHubPolkadot::sovereign_account_id_of(
					t.args.dest
				),
				amount: *amount == t.args.amount,
			},
		]
	);
}

fn system_para_to_para_limited_reserve_transfer_assets(t: SystemParaToParaTest) -> DispatchResult {
	<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::limited_reserve_transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
		t.args.weight_limit,
	)
}

fn system_para_to_para_reserve_transfer_assets(t: SystemParaToParaTest) -> DispatchResult {
	<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::reserve_transfer_assets(
		t.signed_origin,
		bx!(t.args.dest.into()),
		bx!(t.args.beneficiary.into()),
		bx!(t.args.assets.into()),
		t.args.fee_asset_item,
	)
}

/// Reserve Transfers of native asset from AssetHub to BifrostPolkadot should work
#[test]
fn reserve_transfer_native_asset_from_polkadot_to_bifrost_polkadot() {
	// Init values for System Parachain
	let destination = AssetHubPolkadot::sibling_location_of(BifrostPolkadot::para_id());
	let beneficiary_id = BifrostPolkadotReceiver::get();
	let amount_to_send: Balance = ASSET_HUB_POLKADOT_ED * 1000;
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: AssetHubPolkadotSender::get(),
		receiver: BifrostPolkadotReceiver::get(),
		args: system_para_test_args(destination, beneficiary_id, amount_to_send, assets, None),
	};

	let mut test = SystemParaToParaTest::new(test_args);

	let sender_balance_before = test.sender.balance;

	test.set_assertion::<AssetHubPolkadot>(system_para_to_para_assertions);
	test.set_dispatchable::<AssetHubPolkadot>(system_para_to_para_limited_reserve_transfer_assets);
	test.assert();

	let sender_balance_after = test.sender.balance;

	assert_eq!(sender_balance_before - amount_to_send, sender_balance_after);
}

/// Reserve Transfers of native asset from AssetHub to BifrostPolkadot should work
#[test]
fn reserve_transfer_native_asset_from_asset_hub_to_bifrost_polkadot() {
	// Init values for System Parachain
	let destination = AssetHubPolkadot::sibling_location_of(BifrostPolkadot::para_id());
	let beneficiary_id = BifrostPolkadotReceiver::get();
	let amount_to_send: Balance = ASSET_HUB_POLKADOT_ED * 1000;
	let assets = (Parent, amount_to_send).into();

	let test_args = TestContext {
		sender: AssetHubPolkadotSender::get(),
		receiver: BifrostPolkadotReceiver::get(),
		args: system_para_test_args(destination, beneficiary_id, amount_to_send, assets, None),
	};

	let mut test = SystemParaToParaTest::new(test_args);

	let sender_balance_before = test.sender.balance;

	test.set_assertion::<AssetHubPolkadot>(system_para_to_para_assertions);
	test.set_dispatchable::<AssetHubPolkadot>(system_para_to_para_reserve_transfer_assets);
	test.assert();

	let sender_balance_after = test.sender.balance;

	assert_eq!(sender_balance_before - amount_to_send, sender_balance_after);
}

/// Limited Reserve Transfers of a local asset from AssetHub to BifrostPolkadot should work
#[test]
fn limited_reserve_transfer_asset_from_asset_hub_to_bifrost_polkadot() {
	// Force create asset from Relay Chain and mint assets for System Parachain's sender account
	AssetHubPolkadot::force_create_and_mint_asset(
		ASSET_ID,
		ASSET_MIN_BALANCE,
		true,
		AssetHubPolkadotSender::get(),
		ASSET_MIN_BALANCE * 1000000,
	);

	// Init values for System Parachain
	let destination = AssetHubPolkadot::sibling_location_of(BifrostPolkadot::para_id());
	let beneficiary_id = BifrostPolkadotReceiver::get();
	let amount_to_send = ASSET_MIN_BALANCE * 1000;
	let assets =
		(X2(PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())), amount_to_send)
			.into();

	let system_para_test_args = TestContext {
		sender: AssetHubPolkadotSender::get(),
		receiver: BifrostPolkadotReceiver::get(),
		args: system_para_test_args(destination, beneficiary_id, amount_to_send, assets, None),
	};

	let mut system_para_test = SystemParaToParaTest::new(system_para_test_args);

	system_para_test.set_assertion::<AssetHubPolkadot>(system_para_to_para_assets_assertions);
	system_para_test
		.set_dispatchable::<AssetHubPolkadot>(system_para_to_para_limited_reserve_transfer_assets);
	system_para_test.assert();
}

/// Reserve Transfers of a local asset from AssetHub to BifrostPolkadot should work
#[test]
fn reserve_transfer_asset_from_asset_hub_to_bifrost_polkadot() {
	// Force create asset from Relay Chain and mint assets for System Parachain's sender account
	AssetHubPolkadot::force_create_and_mint_asset(
		ASSET_ID,
		ASSET_MIN_BALANCE,
		true,
		AssetHubPolkadotSender::get(),
		ASSET_MIN_BALANCE * 1000000,
	);

	// Init values for System Parachain
	let destination = AssetHubPolkadot::sibling_location_of(BifrostPolkadot::para_id());
	let beneficiary_id = BifrostPolkadotReceiver::get();
	let amount_to_send = ASSET_MIN_BALANCE * 1000;
	let assets =
		(X2(PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())), amount_to_send)
			.into();

	let system_para_test_args = TestContext {
		sender: AssetHubPolkadotSender::get(),
		receiver: BifrostPolkadotReceiver::get(),
		args: system_para_test_args(destination, beneficiary_id, amount_to_send, assets, None),
	};

	let mut system_para_test = SystemParaToParaTest::new(system_para_test_args);

	system_para_test.set_assertion::<AssetHubPolkadot>(system_para_to_para_assets_assertions);
	system_para_test
		.set_dispatchable::<AssetHubPolkadot>(system_para_to_para_reserve_transfer_assets);
	system_para_test.assert();
}

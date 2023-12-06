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

use bifrost_asset_registry::AssetMetadata;
use bifrost_kusama_runtime::{
	AssetRegistry, RelayCurrencyId, RuntimeEvent, RuntimeOrigin, System,
	XcmDestWeightAndFeeHandler, XcmInterface,
};
use bifrost_primitives::{CurrencyId, XcmOperationType as XcmOperation};
use frame_support::{assert_ok, traits::Currency};
use integration_tests_common::{
	impls::Outcome::Complete, AssetHubKusama, AssetHubKusamaAlice, BifrostKusama,
	BifrostKusamaAlice,
};
use polkadot_parachain_primitives::primitives::Sibling;
use sp_runtime::{traits::AccountIdConversion, MultiAddress};
use xcm::{
	v3::{prelude::*, Weight},
	VersionedMultiAssets, VersionedMultiLocation,
};
use xcm_emulator::{bx, Parachain, TestExt};

const USDT: u128 = 1_000_000;

#[test]
fn cross_usdt() {
	BifrostKusama::execute_with(|| {
		let metadata = AssetMetadata {
			name: b"USDT".to_vec(),
			symbol: b"USDT".to_vec(),
			decimals: 6,
			minimal_balance: 10,
		};

		assert_ok!(AssetRegistry::register_token_metadata(
			RuntimeOrigin::root(),
			bx!(metadata.clone())
		));

		let location = VersionedMultiLocation::V3(MultiLocation {
			parents: 1,
			interior: X3(Parachain(1000), PalletInstance(50), GeneralIndex(1984)),
		});

		assert_ok!(AssetRegistry::register_multilocation(
			RuntimeOrigin::root(),
			CurrencyId::Token2(0),
			bx!(location.clone()),
			Weight::zero()
		));
	});

	AssetHubKusama::execute_with(|| {
		use asset_hub_kusama_runtime::{
			Assets, Balances, Runtime, RuntimeEvent, RuntimeOrigin, System,
		};

		let sibling_account = Sibling::from(2001).into_account_truncating();

		// need to have some KSM to be able to receive user assets
		Balances::make_free_balance_be(&sibling_account, 10 * 1_000_000_000_000u128);

		assert_ok!(Assets::create(
			RuntimeOrigin::signed(AssetHubKusamaAlice::get()),
			codec::Compact(1984),
			MultiAddress::Id(AssetHubKusamaAlice::get()),
			10
		));

		assert_ok!(Assets::set_metadata(
			RuntimeOrigin::signed(AssetHubKusamaAlice::get()),
			codec::Compact(1984),
			b"USDT".to_vec(),
			b"USDT".to_vec(),
			6
		));

		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(AssetHubKusamaAlice::get()),
			codec::Compact(1984),
			MultiAddress::Id(AssetHubKusamaAlice::get()),
			100 * USDT
		));
		assert_eq!(Assets::balance(1984, AssetHubKusamaAlice::get()), 100 * USDT);

		let assets = MultiAssets::from(vec![MultiAsset::from((
			Concrete(MultiLocation::new(0, X2(PalletInstance(50), GeneralIndex(1984)))),
			Fungibility::from(10 * USDT),
		))]);

		assert_ok!(pallet_xcm::Pallet::<Runtime>::limited_reserve_transfer_assets(
			RuntimeOrigin::signed(AssetHubKusamaAlice::get()),
			bx!(AssetHubKusama::sibling_location_of(BifrostKusama::para_id()).into()),
			bx!(AccountId32 { network: None, id: BifrostKusamaAlice::get().into() }.into()),
			Box::new(VersionedMultiAssets::V3(assets)),
			0,
			WeightLimit::Unlimited,
		));
		assert!(System::events().iter().any(|r| matches!(
			r.event,
			RuntimeEvent::Assets(pallet_assets::Event::Transferred {
				asset_id: 1984,
				from: _,
				to: _,
				amount: 10_000_000
			})
		)));
		assert!(System::events().iter().any(|r| matches!(
			&r.event,
			RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Attempted { outcome: Complete(_) })
		)));
		System::reset_events();
	});

	BifrostKusama::execute_with(|| {
		System::events().iter().for_each(|r| println!("Bifrost >>> {:?}", r.event));
		assert!(System::events().iter().any(|r| matches!(
			&r.event,
			RuntimeEvent::Tokens(orml_tokens::Event::Deposited {
				currency_id: CurrencyId::Token2(0),
				who: _,
				amount: 10_000_000
			})
		)));

		System::reset_events();

		assert_ok!(XcmInterface::set_xcm_dest_weight_and_fee(
			RelayCurrencyId::get(),
			XcmOperation::StatemineTransfer,
			Some((Weight::from_parts(400_000_000_000, 10_000), 4_000_000_000)),
		));

		// Alice transfers 5 statemine asset to Bob
		// TODO: Failed to process XCMP-XCM message, caused by Barrier
		// assert_ok!(XcmInterface::transfer_statemine_assets(
		// 	RuntimeOrigin::signed(BifrostKusamaAlice::get()),
		// 	5 * USDT,
		// 	1984,
		// 	Some(BifrostKusamaBob::get())
		// ));
	});

	// AsserHubKusama::execute_with(|| {
	// 		use statemine_runtime::*;
	// 		println!("{:?}", System::events());
	//
	// 		// assert Bob has 5 statemine asset
	// 		assert_eq!(Assets::balance(1984, AccountId::from(BOB)), 5 * USDT);
	//
	// 		assert!(System::events().iter().any(|r| matches!(
	// 			r.event,
	// 			RuntimeEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success {
	// 				message_hash: _,
	// 				weight: _
	// 			})
	// 		)));
	// 	});
}

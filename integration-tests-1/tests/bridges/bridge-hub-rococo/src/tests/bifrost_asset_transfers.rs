// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;
use bifrost_primitives::{DOT, KSM, BNC};
use frame_support::BoundedVec;
use bifrost_primitives::CurrencyId::Token2;
use frame_support::pallet_prelude::Encode;

fn send_asset_from_bifrost_polkadot_to_bifrost_kusama(id: MultiLocation, amount: u128) {
	let signed_origin =
		<BifrostPolkadot as Chain>::RuntimeOrigin::signed(BifrostPolkadotSender::get().into());
	let bifrost_kusama_para_id = BifrostKusama::para_id().into();
	let destination = MultiLocation {
		parents: 2,
		interior: X2(GlobalConsensus(NetworkId::Westend), Parachain(bifrost_kusama_para_id)),
	};
	let beneficiary_id = BifrostKusamaReceiver::get();
	let beneficiary: MultiLocation =
		AccountId32Junction { network: None, id: beneficiary_id.into() }.into();
	let assets: MultiAssets = (id, amount).into();
	let fee_asset_item = 0;

	// fund the AHR's SA on BHR for paying bridge transport fees
	let ahr_as_seen_by_bhr = BridgeHubRococo::sibling_location_of(BifrostPolkadot::para_id());
	let sov_ahr_on_bhr = BridgeHubRococo::sovereign_account_id_of(ahr_as_seen_by_bhr);
	BridgeHubRococo::fund_accounts(vec![(sov_ahr_on_bhr.into(), 10_000_000_000_000u128)]);

	BifrostPolkadot::execute_with(|| {
		assert_ok!(
			<BifrostPolkadot as BifrostPolkadotPallet>::PolkadotXcm::limited_reserve_transfer_assets(
				signed_origin,
				bx!(destination.into()),
				bx!(beneficiary.into()),
				bx!(assets.into()),
				fee_asset_item,
				WeightLimit::Unlimited,
			)
		);

		type RuntimeEvent = <BifrostPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			BifrostPolkadot,
			vec![
				// pay for bridge fees
				RuntimeEvent::Balances(pallet_balances::Event::Withdraw { .. }) => {},
					// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	BridgeHubRococo::execute_with(|| {
		type RuntimeEvent = <BridgeHubRococo as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubRococo,
			vec![
				// pay for bridge fees
				RuntimeEvent::Balances(pallet_balances::Event::Withdraw { .. }) => {},
				// message exported
				RuntimeEvent::BridgeWestendMessages(
					pallet_bridge_messages::Event::MessageAccepted { .. }
				) => {},
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});
	BridgeHubWestend::execute_with(|| {
		type RuntimeEvent = <BridgeHubWestend as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubWestend,
			vec![
				// message dispatched successfully
				RuntimeEvent::XcmpQueue(
					cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	});
}

#[test]
fn send_dot_from_bifrost_polkadot_to_bifrost_kusama() {
	let prefund_amount = 10_000_000_000_000u128;
	let dot_at_bifrost_polkadot: MultiLocation = Parent.into();
	let dot_at_bifrost_kusama =
		MultiLocation { parents: 2, interior: X1(GlobalConsensus(NetworkId::Rococo)) };
	let owner: AccountId = BifrostKusama::account_id_of(ALICE);
	BifrostKusama::execute_with(|| {
		type AssetRegistry = <BifrostKusama as BifrostKusamaPallet>::AssetRegistry;
		let sudo_origin = <BifrostKusama as Chain>::RuntimeOrigin::root();
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), KSM, bx!(Parent.into()), Weight::default()));
		println!("{}", AssetRegistry::next_token_id());
		assert_ok!(AssetRegistry::register_token_metadata(sudo_origin.clone(), bx!(bifrost_asset_registry::pallet::AssetMetadata {
			name: b"wDOT".to_vec(),
			symbol: b"wDOT".to_vec(),
			decimals: 10u8,
			minimal_balance: 1u128
		})));
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), Token2(0), bx!(dot_at_bifrost_kusama.into()), Weight::default()));
	});
	let sov_ahw_on_ahr = BifrostPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Westend,
		BifrostKusama::para_id(),
	);

	let (dot_in_reserve_on_ahr_before, sender_dot_before) = BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		let sudo_origin = <BifrostPolkadot as Chain>::RuntimeOrigin::root();
		// assert_ok!(Tokens::set_balance(sudo_origin.clone(), sov_ahw_on_ahr.clone().into(), DOT, ASSET_HUB_ROCOCO_ED * 1_000_000, 0u128));
		assert_ok!(Tokens::set_balance(sudo_origin, BifrostPolkadotSender::get().into(), DOT, ASSET_HUB_ROCOCO_ED * 1_000_000, 0u128));
		(
			Tokens::accounts(sov_ahw_on_ahr.clone(), DOT).free,
			Tokens::accounts(BifrostPolkadotSender::get(), DOT).free,
		)
	});

	println!("dot_in_reserve_on_ahr_before: {:?}", dot_in_reserve_on_ahr_before);
	println!("sender_dot_before: {:?}", sender_dot_before);

	let receiver_dot_before = BifrostKusama::execute_with(|| {
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		Tokens::accounts(BifrostPolkadotReceiver::get(), Token2(0)).free
	});
	println!("receiver_dot_before: {:?}", receiver_dot_before);

	let amount = ASSET_HUB_ROCOCO_ED * 1_000;
	println!("send amount: {:?}", amount);
	send_asset_from_bifrost_polkadot_to_bifrost_kusama(dot_at_bifrost_polkadot, amount);
	BifrostKusama::execute_with(|| {
		type RuntimeEvent = <BifrostKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			BifrostKusama,
			vec![
				// issue ROCs on AHW
				// RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
				// 	asset_id: *asset_id == dot_at_bifrost_polkadot,
				// 	owner: *owner == BifrostKusamaReceiver::get(),
				// },
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let (dot_in_reserve_on_ahr_after, sender_dot_after) = BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		let sudo_origin = <BifrostPolkadot as Chain>::RuntimeOrigin::root();
		(
			Tokens::accounts(sov_ahw_on_ahr.clone(), DOT).free,
			Tokens::accounts(BifrostPolkadotSender::get(), DOT).free,
		)
	});

	println!("dot_in_reserve_on_ahr_after: {:?}", dot_in_reserve_on_ahr_after);
	println!("sender_dot_after: {:?}", sender_dot_after);

	let receiver_dot_after = BifrostKusama::execute_with(|| {
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		Tokens::accounts(BifrostPolkadotReceiver::get(), Token2(0)).free
	});
	println!("receiver_dot_after: {:?}", receiver_dot_after);

	// Sender's balance is reduced
	assert!(sender_dot_before > sender_dot_after);
	// Receiver's balance is increased
	assert!(receiver_dot_after > receiver_dot_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(dot_in_reserve_on_ahr_after, dot_in_reserve_on_ahr_before + amount);
}

#[test]
fn send_vksm_from_bifrost_polkadot_to_bifrost_kusama() {
	let prefund_amount = 10_000_000_000_000u128;
	let ksm_at_bifrost_polkadot =
		MultiLocation { parents: 2, interior: X1(GlobalConsensus(NetworkId::Westend)) };
	let owner: AccountId = BifrostKusama::account_id_of(ALICE);

	BifrostPolkadot::execute_with(|| {
		type AssetRegistry = <BifrostPolkadot as BifrostPolkadotPallet>::AssetRegistry;
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		let sudo_origin = <BifrostPolkadot as Chain>::RuntimeOrigin::root();
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), DOT, bx!(Parent.into()), Weight::default()));
		println!("{}", AssetRegistry::next_token_id());
		assert_ok!(AssetRegistry::register_token_metadata(sudo_origin.clone(),bx!(bifrost_asset_registry::pallet::AssetMetadata {
			name: b"wKSM".to_vec(),
			symbol: b"wKSM".to_vec(),
			decimals: 12u8,
			minimal_balance: 1u128
		})));
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), Token2(2), bx!(ksm_at_bifrost_polkadot.into()), Weight::default()));

		assert_ok!(Tokens::set_balance(sudo_origin.clone(), BifrostPolkadotSender::get().into(), DOT, prefund_amount, 0u128));
		assert_ok!(Tokens::set_balance(sudo_origin.clone(), BifrostPolkadotSender::get().into(), Token2(2), prefund_amount, 0u128));
	});

	// fund the AHR's SA on AHW with the WND tokens held in reserve
	let sov_ahr_on_ahw = BifrostKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Rococo,
		BifrostPolkadot::para_id(),
	);
	BifrostKusama::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		let sudo_origin = <BifrostPolkadot as Chain>::RuntimeOrigin::root();
		assert_ok!(Tokens::set_balance(sudo_origin, sov_ahr_on_ahw.clone().into(), KSM, prefund_amount, 0u128));
	});

	let (ksm_in_reserve_on_ahw_before, receiver_ksm_before) = BifrostKusama::execute_with(|| {
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		(
			Tokens::accounts(sov_ahr_on_ahw.clone(), KSM).free,
			Tokens::accounts(BifrostKusamaReceiver::get(), KSM).free,
		)
	});

	let sender_ksm_before = BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
			Tokens::accounts(BifrostPolkadotSender::get(), Token2(2)).free
	});
	assert_eq!(ksm_in_reserve_on_ahw_before, prefund_amount);
	assert_eq!(sender_ksm_before, prefund_amount);

	println!("ksm_in_reserve_on_ahw_before: {:?}", ksm_in_reserve_on_ahw_before);
	println!("sender_ksm_before: {:?}", sender_ksm_before);
	println!("receiver_ksm_before: {:?}", receiver_ksm_before);


	let amount_to_send = ASSET_HUB_WESTEND_ED * 1_000;
	send_asset_from_bifrost_polkadot_to_bifrost_kusama(ksm_at_bifrost_polkadot, amount_to_send);
	BifrostKusama::execute_with(|| {
		type RuntimeEvent = <BifrostKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			BifrostKusama,
			vec![
				// WND is withdrawn from AHR's SA on AHW
				// RuntimeEvent::Balances(
				// 	pallet_balances::Event::Withdraw { who, amount }
				// ) => {
				// 	who: *who == sov_ahr_on_ahw,
				// 	amount: *amount == amount_to_send,
				// },
				// // WNDs deposited to beneficiary
				// RuntimeEvent::Balances(pallet_balances::Event::Deposit { who, .. }) => {
				// 	who: *who == BifrostKusamaReceiver::get(),
				// },
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let (ksm_in_reserve_on_ahw_after, receiver_ksm_after) = BifrostKusama::execute_with(|| {
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		(
			Tokens::accounts(sov_ahr_on_ahw.clone(), KSM).free,
			Tokens::accounts(BifrostKusamaReceiver::get(), KSM).free,
		)
	});

	let sender_ksm_after = BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		Tokens::accounts(BifrostPolkadotSender::get(), Token2(2)).free
	});
	println!("ksm_in_reserve_on_ahw_after: {:?}", ksm_in_reserve_on_ahw_after);
	println!("sender_ksm_after: {:?}", sender_ksm_after);
	println!("receiver_ksm_after: {:?}", receiver_ksm_after);

	// Sender's balance is reduced
	assert!(sender_ksm_before > sender_ksm_after);
	// Receiver's balance is increased
	assert!(receiver_ksm_after > receiver_ksm_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(ksm_in_reserve_on_ahw_after, ksm_in_reserve_on_ahw_before - amount_to_send);
}

#[test]
fn send_bnc_from_bifrost_polkadot_to_bifrost_kusama() {
	let prefund_amount = 10_000_000_000_000u128;
	let bnc_at_bifrost_polkadot =
		MultiLocation {
			parents: 0,
			interior: X1(Junction::from(BoundedVec::try_from(BNC.encode()).unwrap())),
		};
	let owner: AccountId = BifrostKusama::account_id_of(ALICE);

	BifrostPolkadot::execute_with(|| {
		type AssetRegistry = <BifrostPolkadot as BifrostPolkadotPallet>::AssetRegistry;
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		let sudo_origin = <BifrostPolkadot as Chain>::RuntimeOrigin::root();
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), DOT, bx!(Parent.into()), Weight::default()));
		assert_ok!(Tokens::set_balance(sudo_origin.clone(), BifrostPolkadotSender::get().into(), DOT, prefund_amount, 0u128));
	});

	// fund the AHR's SA on AHW with the WND tokens held in reserve
	let sov_ahw_on_ahr = BifrostPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Westend,
		BifrostKusama::para_id(),
	);

	let bnc_in_reserve_on_ahw_before =
		<BifrostPolkadot as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	let sender_bnc_before =
		<BifrostPolkadot as Chain>::account_data_of(BifrostPolkadotSender::get()).free;
	let receiver_bnc_before =
		<BifrostKusama as Chain>::account_data_of(BifrostKusamaReceiver::get()).free;

	println!("bnc_in_reserve_on_ahw_before: {:?}", bnc_in_reserve_on_ahw_before);
	println!("sender_bnc_before: {:?}", sender_bnc_before);
	println!("receiver_bnc_before: {:?}", receiver_bnc_before);


	let amount_to_send = ASSET_HUB_WESTEND_ED * 1_000;
	send_asset_from_bifrost_polkadot_to_bifrost_kusama(bnc_at_bifrost_polkadot, amount_to_send);
	BifrostKusama::execute_with(|| {
		type RuntimeEvent = <BifrostKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			BifrostKusama,
			vec![
				// WND is withdrawn from AHR's SA on AHW
				// RuntimeEvent::Balances(
				// 	pallet_balances::Event::Withdraw { who, amount }
				// ) => {
				// 	who: *who == sov_ahw_on_ahr,
				// 	amount: *amount == amount_to_send,
				// },
				// // WNDs deposited to beneficiary
				// RuntimeEvent::Balances(pallet_balances::Event::Deposit { who, .. }) => {
				// 	who: *who == BifrostKusamaReceiver::get(),
				// },
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let bnc_in_reserve_on_ahw_after =
		<BifrostPolkadot as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	let sender_bnc_after =
		<BifrostPolkadot as Chain>::account_data_of(BifrostPolkadotSender::get()).free;
	let receiver_bnc_after =
		<BifrostKusama as Chain>::account_data_of(BifrostKusamaReceiver::get()).free;

	println!("bnc_in_reserve_on_ahw_after: {:?}", bnc_in_reserve_on_ahw_after);
	println!("sender_bnc_after: {:?}", sender_bnc_after);
	println!("receiver_bnc_after: {:?}", receiver_bnc_after);

	// Sender's balance is reduced
	assert!(sender_bnc_before > sender_bnc_after);
	// Receiver's balance is increased
	assert!(receiver_bnc_after > receiver_bnc_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(bnc_in_reserve_on_ahw_after, bnc_in_reserve_on_ahw_before + amount_to_send);
}

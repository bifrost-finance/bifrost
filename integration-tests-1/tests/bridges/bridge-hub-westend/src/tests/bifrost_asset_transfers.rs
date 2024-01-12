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
use bifrost_primitives::{KSM, DOT};
use bifrost_primitives::BNC;
use frame_support::BoundedVec;
use bifrost_primitives::CurrencyId::Token2;
use frame_support::pallet_prelude::Encode;
use bifrost_primitives::currency::VMOVR;

fn send_asset_from_bifrost_kusama_to_bifrost_polkadot(id: MultiLocation, amount: u128) {
	let signed_origin =
		<BifrostKusama as Chain>::RuntimeOrigin::signed(BifrostKusamaSender::get().into());
	let bifrost_polkadot_para_id = BifrostPolkadot::para_id().into();
	let destination = MultiLocation {
		parents: 2,
		interior: X2(GlobalConsensus(NetworkId::Rococo), Parachain(bifrost_polkadot_para_id)),
	};
	let beneficiary_id = BifrostPolkadotReceiver::get();
	let beneficiary: MultiLocation =
		AccountId32Junction { network: None, id: beneficiary_id.into() }.into();
	let assets: MultiAssets = (id, amount).into();
	let fee_asset_item = 0;

	// fund the AHW's SA on BHW for paying bridge transport fees
	let ahw_as_seen_by_bhw = BridgeHubWestend::sibling_location_of(BifrostKusama::para_id());
	let sov_ahw_on_bhw = BridgeHubWestend::sovereign_account_id_of(ahw_as_seen_by_bhw);
	BridgeHubWestend::fund_accounts(vec![(sov_ahw_on_bhw.into(), 10_000_000_000_000u128)]);

	BifrostKusama::execute_with(|| {
		assert_ok!(
			<BifrostKusama as BifrostKusamaPallet>::PolkadotXcm::limited_reserve_transfer_assets(
				signed_origin,
				bx!(destination.into()),
				bx!(beneficiary.into()),
				bx!(assets.into()),
				fee_asset_item,
				WeightLimit::Unlimited,
			)
		);
	});

	BridgeHubWestend::execute_with(|| {
		type RuntimeEvent = <BridgeHubWestend as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubWestend,
			vec![
				// pay for bridge fees
				RuntimeEvent::Balances(pallet_balances::Event::Withdraw { .. }) => {},
				// message exported
				RuntimeEvent::BridgeRococoMessages(
					pallet_bridge_messages::Event::MessageAccepted { .. }
				) => {},
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
				// message dispatched successfully
				RuntimeEvent::XcmpQueue(
					cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	});
}

#[test]
fn send_wnds_from_bifrost_kusama_to_bifrost_polkadot() {
	let wnd_at_bifrost_kusama: MultiLocation = Parent.into();
	let wnd_at_bifrost_polkadot =
		MultiLocation { parents: 2, interior: X1(GlobalConsensus(NetworkId::Westend)) };

	BifrostPolkadot::execute_with(|| {
		type AssetRegistry = <BifrostPolkadot as BifrostPolkadotPallet>::AssetRegistry;
		let sudo_origin = <BifrostPolkadot as Chain>::RuntimeOrigin::root();
		assert_ok!(AssetRegistry::register_token_metadata(sudo_origin.clone(), bx!(bifrost_asset_registry::pallet::AssetMetadata {
			name: b"wKSM".to_vec(),
			symbol: b"wKSM".to_vec(),
			decimals: 12u8,
			minimal_balance: 1u128
		})));
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), Token2(2), bx!(wnd_at_bifrost_polkadot.into()), Weight::default()));
	});

	let sov_ahw_on_ahr = BifrostKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Rococo,
		BifrostPolkadot::para_id(),
	);

	let (wnds_in_reserve_on_ahw_before, sender_wnds_before) = BifrostKusama::execute_with(|| {
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		let sudo_origin = <BifrostKusama as Chain>::RuntimeOrigin::root();
		assert_ok!(Tokens::set_balance(sudo_origin, BifrostKusamaSender::get().into(), KSM, 1_000_000_000_000_000u128, 0u128));
		(
			Tokens::accounts(sov_ahw_on_ahr.clone(), KSM).free,
			Tokens::accounts(BifrostKusamaSender::get(), KSM).free,
		)
	});
	println!("wnds_in_reserve_on_ahw_before: {:?}", wnds_in_reserve_on_ahw_before);
	println!("sender_wnds_before: {:?}", sender_wnds_before);

	let receiver_wnds_before = BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		Tokens::accounts(BifrostPolkadotReceiver::get(), KSM).free
	});
	println!("receiver_wnds_before: {:?}", receiver_wnds_before);

	let amount = 1_00_000_000_000_000u128;
	println!("send amount: {:?}", amount);
	send_asset_from_bifrost_kusama_to_bifrost_polkadot(wnd_at_bifrost_kusama, amount);
	BifrostPolkadot::execute_with(|| {
		type RuntimeEvent = <BifrostPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			BifrostPolkadot,
			vec![
				// issue WNDs on AHR
				// RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
				// 	asset_id: *asset_id == wnd_at_bifrost_polkadot,
				// 	owner: *owner == BifrostPolkadotReceiver::get(),
				// },
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let (wnds_in_reserve_on_ahw_after, sender_wnds_after) = BifrostKusama::execute_with(|| {
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		(
			Tokens::accounts(sov_ahw_on_ahr.clone(), KSM).free,
			Tokens::accounts(BifrostKusamaSender::get(), KSM).free,
		)
	});
	println!("wnds_in_reserve_on_ahw_after: {:?}", wnds_in_reserve_on_ahw_after);
	println!("sender_wnds_after: {:?}", sender_wnds_after);

	let receiver_wnds_after = BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
			Tokens::accounts(BifrostPolkadotReceiver::get(), Token2(2)).free
	});
	println!("receiver_wnds_after: {:?}", receiver_wnds_after);

	// Sender's balance is reduced
	assert!(sender_wnds_before > sender_wnds_after);
	// Receiver's balance is increased
	assert!(receiver_wnds_after > receiver_wnds_before);
	// Reserve balance is increased by sent amount
	assert_eq!(wnds_in_reserve_on_ahw_after, wnds_in_reserve_on_ahw_before + amount);
}

#[test]
fn send_rocs_from_bifrost_kusama_to_bifrost_polkadot() {
	let prefund_amount = 10_000_000_000_000u128;
	let dot_at_bifrost_polkadot =
		MultiLocation { parents: 2, interior: X1(GlobalConsensus(NetworkId::Rococo)) };

	BifrostKusama::execute_with(|| {
		type AssetRegistry = <BifrostKusama as BifrostKusamaPallet>::AssetRegistry;
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		let sudo_origin = <BifrostKusama as Chain>::RuntimeOrigin::root();
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), KSM, bx!(Parent.into()), Weight::default()));
		assert_ok!(AssetRegistry::register_token_metadata(sudo_origin.clone(),bx!(bifrost_asset_registry::pallet::AssetMetadata {
			name: b"wDOT".to_vec(),
			symbol: b"wDOT".to_vec(),
			decimals: 10u8,
			minimal_balance: 1u128
		})));
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), Token2(0), bx!(dot_at_bifrost_polkadot.into()), Weight::default()));

		assert_ok!(Tokens::set_balance(sudo_origin.clone(), BifrostKusamaSender::get().into(), KSM, prefund_amount, 0u128));
		assert_ok!(Tokens::set_balance(sudo_origin.clone(), BifrostKusamaSender::get().into(), Token2(0), prefund_amount, 0u128));
	});

	// fund the AHR's SA on AHW with the WND tokens held in reserve
	let sov_ahw_on_ahr = BifrostPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Westend,
		BifrostKusama::para_id(),
	);
	BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		let sudo_origin = <BifrostPolkadot as Chain>::RuntimeOrigin::root();
		assert_ok!(Tokens::set_balance(sudo_origin, sov_ahw_on_ahr.clone().into(), DOT, prefund_amount, 0u128));
	});

	let (dot_in_reserve_on_ahw_before, receiver_dot_before) = BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		(
			Tokens::accounts(sov_ahw_on_ahr.clone(), DOT).free,
			Tokens::accounts(BifrostPolkadotReceiver::get(), DOT).free,
		)
	});

	let sender_dot_before = BifrostKusama::execute_with(|| {
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		Tokens::accounts(BifrostKusamaSender::get(), Token2(0)).free
	});
	assert_eq!(dot_in_reserve_on_ahw_before, prefund_amount);
	assert_eq!(sender_dot_before, prefund_amount);

	println!("dot_in_reserve_on_ahw_before: {:?}", dot_in_reserve_on_ahw_before);
	println!("sender_dot_before: {:?}", sender_dot_before);
	println!("receiver_dot_before: {:?}", receiver_dot_before);


	let amount_to_send = ASSET_HUB_WESTEND_ED * 1_000;
	send_asset_from_bifrost_kusama_to_bifrost_polkadot(dot_at_bifrost_polkadot, amount_to_send);
	BifrostPolkadot::execute_with(|| {
		type RuntimeEvent = <BifrostPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			BifrostPolkadot,
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
				// 	who: *who == BifrostPolkadotReceiver::get(),
				// },
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let (dot_in_reserve_on_ahw_after, receiver_dot_after) = BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		(
			Tokens::accounts(sov_ahw_on_ahr.clone(), DOT).free,
			Tokens::accounts(BifrostPolkadotReceiver::get(), DOT).free,
		)
	});

	let sender_dot_after = BifrostKusama::execute_with(|| {
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		Tokens::accounts(BifrostKusamaSender::get(), Token2(0)).free
	});
	println!("dot_in_reserve_on_ahw_after: {:?}", dot_in_reserve_on_ahw_after);
	println!("sender_dot_after: {:?}", sender_dot_after);
	println!("receiver_dot_after: {:?}", receiver_dot_after);

	// Sender's balance is reduced
	assert!(sender_dot_before > sender_dot_after);
	// Receiver's balance is increased
	assert!(receiver_dot_after > receiver_dot_before);
	// Reserve balance is reduced by sent amount
	assert_eq!(dot_in_reserve_on_ahw_after, dot_in_reserve_on_ahw_before - amount_to_send);
}

#[test]
fn send_bnc_from_bifrost_kusama_to_bifrost_polkadot() {
	let prefund_amount = 10_000_000_000_000u128;
	let bnc_at_bifrost_kusama =
		MultiLocation {
			parents: 0,
			interior: X1(Junction::from(BoundedVec::try_from(BNC.encode()).unwrap())),
		};

	BifrostKusama::execute_with(|| {
		type AssetRegistry = <BifrostKusama as BifrostKusamaPallet>::AssetRegistry;
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		let sudo_origin = <BifrostKusama as Chain>::RuntimeOrigin::root();
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), KSM, bx!(Parent.into()), Weight::default()));
		assert_ok!(Tokens::set_balance(sudo_origin.clone(), BifrostKusamaSender::get().into(), KSM, prefund_amount, 0u128));
	});

	// fund the AHR's SA on AHW with the WND tokens held in reserve
	let sov_ahr_on_ahw = BifrostKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Rococo,
		BifrostPolkadot::para_id(),
	);

	let bnc_in_reserve_on_ahw_before =
		<BifrostKusama as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	let sender_bnc_before =
		<BifrostKusama as Chain>::account_data_of(BifrostKusamaSender::get()).free;
	let receiver_bnc_before =
		<BifrostPolkadot as Chain>::account_data_of(BifrostPolkadotReceiver::get()).free;

	println!("bnc_in_reserve_on_ahw_before: {:?}", bnc_in_reserve_on_ahw_before);
	println!("sender_bnc_before: {:?}", sender_bnc_before);
	println!("receiver_bnc_before: {:?}", receiver_bnc_before);


	let amount_to_send = ASSET_HUB_WESTEND_ED * 1_000;
	send_asset_from_bifrost_kusama_to_bifrost_polkadot(bnc_at_bifrost_kusama, amount_to_send);
	BifrostPolkadot::execute_with(|| {
		type RuntimeEvent = <BifrostPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			BifrostPolkadot,
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

	let bnc_in_reserve_on_ahw_after =
		<BifrostKusama as Chain>::account_data_of(sov_ahr_on_ahw.clone()).free;
	let sender_bnc_after =
		<BifrostKusama as Chain>::account_data_of(BifrostKusamaSender::get()).free;
	let receiver_bnc_after =
		<BifrostPolkadot as Chain>::account_data_of(BifrostPolkadotReceiver::get()).free;

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

#[test]
fn send_vmovr_from_bifrost_kusama_to_bifrost_polkadot() {
	let vmovr_at_bifrost_kusama: MultiLocation = MultiLocation {
		parents: 0,
		interior: X1(Junction::from(BoundedVec::try_from(VMOVR.encode()).unwrap())),
	};
	let vmovr_at_bifrost_polkadot =
		MultiLocation {
			parents: 2,
			interior: X3(GlobalConsensus(NetworkId::Westend), Parachain(2001), Junction::from(BoundedVec::try_from(VMOVR.encode()).unwrap())),
		};

	BifrostPolkadot::execute_with(|| {
		type AssetRegistry = <BifrostPolkadot as BifrostPolkadotPallet>::AssetRegistry;
		let sudo_origin = <BifrostPolkadot as Chain>::RuntimeOrigin::root();
		assert_ok!(AssetRegistry::register_token_metadata(sudo_origin.clone(), bx!(bifrost_asset_registry::pallet::AssetMetadata {
			name: b"w vMOVR".to_vec(),
			symbol: b"w vMOVR".to_vec(),
			decimals: 18u8,
			minimal_balance: 1u128
		})));
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), Token2(2), bx!(vmovr_at_bifrost_polkadot.into()), Weight::default()));
	});

	let sov_ahw_on_ahr = BifrostKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Rococo,
		BifrostPolkadot::para_id(),
	);

	let (wnds_in_reserve_on_ahw_before, sender_wnds_before) = BifrostKusama::execute_with(|| {
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		type AssetRegistry = <BifrostKusama as BifrostKusamaPallet>::AssetRegistry;
		let sudo_origin = <BifrostKusama as Chain>::RuntimeOrigin::root();
		assert_ok!(AssetRegistry::register_multilocation(sudo_origin.clone(), VMOVR, bx!(vmovr_at_bifrost_kusama.into()), Weight::default()));
		assert_ok!(Tokens::set_balance(sudo_origin.clone(), BifrostKusamaSender::get().into(), KSM, 1_000_000_000_000_000u128, 0u128));
		assert_ok!(Tokens::set_balance(sudo_origin.clone(), BifrostKusamaSender::get().into(), VMOVR, 100_000_000_000_000_000_000u128, 0u128));
		(
			Tokens::accounts(sov_ahw_on_ahr.clone(), VMOVR).free,
			Tokens::accounts(BifrostKusamaSender::get(), VMOVR).free,
		)
	});
	println!("wnds_in_reserve_on_ahw_before: {:?}", wnds_in_reserve_on_ahw_before);
	println!("sender_wnds_before: {:?}", sender_wnds_before);

	let receiver_wnds_before = BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		Tokens::accounts(BifrostPolkadotReceiver::get(), Token2(2)).free
	});
	println!("receiver_wnds_before: {:?}", receiver_wnds_before);

	let amount = 1_00_000_000_000_000u128;
	println!("send amount: {:?}", amount);
	send_asset_from_bifrost_kusama_to_bifrost_polkadot(vmovr_at_bifrost_kusama, amount);
	BifrostPolkadot::execute_with(|| {
		type RuntimeEvent = <BifrostPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			BifrostPolkadot,
			vec![
				// issue WNDs on AHR
				// RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { asset_id, owner, .. }) => {
				// 	asset_id: *asset_id == vmovr_at_bifrost_polkadot,
				// 	owner: *owner == BifrostPolkadotReceiver::get(),
				// },
				// message processed successfully
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});

	let (wnds_in_reserve_on_ahw_after, sender_wnds_after) = BifrostKusama::execute_with(|| {
		type Tokens = <BifrostKusama as BifrostKusamaPallet>::Tokens;
		(
			Tokens::accounts(sov_ahw_on_ahr.clone(), VMOVR).free,
			Tokens::accounts(BifrostKusamaSender::get(), VMOVR).free,
		)
	});
	println!("wnds_in_reserve_on_ahw_after: {:?}", wnds_in_reserve_on_ahw_after);
	println!("sender_wnds_after: {:?}", sender_wnds_after);

	let receiver_wnds_after = BifrostPolkadot::execute_with(|| {
		type Tokens = <BifrostPolkadot as BifrostPolkadotPallet>::Tokens;
		Tokens::accounts(BifrostPolkadotReceiver::get(), Token2(2)).free
	});
	println!("receiver_wnds_after: {:?}", receiver_wnds_after);

	// Sender's balance is reduced
	assert!(sender_wnds_before > sender_wnds_after);
	// Receiver's balance is increased
	assert!(receiver_wnds_after > receiver_wnds_before);
	// Reserve balance is increased by sent amount
	assert_eq!(wnds_in_reserve_on_ahw_after, wnds_in_reserve_on_ahw_before + amount);
}

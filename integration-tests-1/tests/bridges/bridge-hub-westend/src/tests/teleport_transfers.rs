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
		<BifrostKusama as Chain>::RuntimeOrigin::signed(BifrostKusamaSender::get().into());
	let bifrost_kusama_para_id = BifrostPolkadot::para_id().into();
	let destination = MultiLocation {
		parents: 2,
		interior: X2(GlobalConsensus(NetworkId::Rococo), Parachain(bifrost_kusama_para_id)),
	};
	let beneficiary_id = BifrostPolkadotReceiver::get();
	let beneficiary: MultiLocation =
		AccountId32Junction { network: None, id: beneficiary_id.into() }.into();
	let assets: MultiAssets = (id, amount).into();
	let fee_asset_item = 0;

	// fund the AHR's SA on BHR for paying bridge transport fees
	let ahr_as_seen_by_bhr = BridgeHubWestend::sibling_location_of(BifrostKusama::para_id());
	let sov_ahr_on_bhr = BridgeHubWestend::sovereign_account_id_of(ahr_as_seen_by_bhr);
	BridgeHubWestend::fund_accounts(vec![(sov_ahr_on_bhr.into(), 10_000_000_000_000u128)]);

	let bnc_fee: MultiAssets = (MultiLocation {
		parents: 0,
		interior: X1(Junction::from(BoundedVec::try_from(BNC.encode()).unwrap())),
	}, 2_000_000_000u128).into();

	BifrostKusama::execute_with(|| {
		assert_ok!(
			pallet_xcm::Pallet::<bifrost_kusama_runtime::Runtime>::execute(
				signed_origin,
				bx!(VersionedXcm::V3(Xcm(vec![
					WithdrawAsset(assets.clone()),
					SetFeesMode { jit_withdraw: true },
					// Burn the native asset.
					BurnAsset(assets.clone()),

					WithdrawAsset(bnc_fee.clone()),
				]))),
				 Weight::from_parts(20000000000u64, 200000u64)
			)
		);

		assert_ok!(
			pallet_xcm::Pallet::<bifrost_kusama_runtime::Runtime>::send_xcm(
				Here,
				destination,
				// bx!(beneficiary.into()),
				// bx!(assets.into()),
				// fee_asset_item,
				// WeightLimit::Unlimited,
				Xcm(vec![
					ReceiveTeleportedAsset(assets.clone().into()),
					// WithdrawAsset(bnc_fee.clone().into()),
					ClearOrigin,
					// BuyExecution { fees: bnc_fee, weight_limit: Unlimited },
					DepositAsset { assets: Wild(AllCounted(2)), beneficiary }
				])
			)
		);

		type Balances = <BifrostKusama as BifrostKusamaPallet>::Balances;
		println!("After Balances: {:?}", Balances::total_issuance());
		// 671098572891136
		// 670098572891136
		// type RuntimeEvent = <BifrostKusama as Chain>::RuntimeEvent;
		// assert_expected_events!(
		// 	BifrostKusama,
		// 	vec![
		// 		// pay for bridge fees
		// 		// RuntimeEvent::Balances(pallet_balances::Event::Withdraw { .. }) => {},
		// 			// message processed successfully
		// 		RuntimeEvent::MessageQueue(
		// 			pallet_message_queue::Event::Processed { success: true, .. }
		// 		) => {},
		// 	]
		// );
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
fn teleport_bnc_2() {
	let prefund_amount = 10_000_000_000_000u128;
	let bnc_at_bifrost_polkadot =
		MultiLocation {
			parents: 0,
			interior: X1(Junction::from(BoundedVec::try_from(BNC.encode()).unwrap())),
		};
	let owner: AccountId = BifrostPolkadot::account_id_of(ALICE);

	// fund the AHR's SA on AHW with the WND tokens held in reserve
	let sov_ahw_on_ahr = BifrostKusama::sovereign_account_of_parachain_on_other_global_consensus(
		NetworkId::Rococo,
		BifrostPolkadot::para_id(),
	);

	let bnc_in_reserve_on_ahw_before =
		<BifrostKusama as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
	let sender_bnc_before =
		<BifrostKusama as Chain>::account_data_of(BifrostKusamaSender::get()).free;
	let receiver_bnc_before =
		<BifrostPolkadot as Chain>::account_data_of(BifrostPolkadotReceiver::get()).free;

	println!("bnc_in_reserve_on_ahw_before: {:?}", bnc_in_reserve_on_ahw_before);
	println!("sender_bnc_before: {:?}", sender_bnc_before);
	println!("receiver_bnc_before: {:?}", receiver_bnc_before);
	// 4096000000000
	// 5096000000000

	// 55924047740928
	// 54924047740928

	//sender_bnc_before: 4096000000000
	// receiver_bnc_before: 55924047740928

	//sender_bnc_after: 3094000000000
	// receiver_bnc_after: 55924047740928

	let amount_to_send = ASSET_HUB_WESTEND_ED * 1_000;
	send_asset_from_bifrost_polkadot_to_bifrost_kusama(bnc_at_bifrost_polkadot, amount_to_send);
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
					pallet_message_queue::Event::Processed { success: false, .. }
				) => {},
			]
		);
	});

	let bnc_in_reserve_on_ahw_after =
		<BifrostKusama as Chain>::account_data_of(sov_ahw_on_ahr.clone()).free;
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
	// assert_eq!(bnc_in_reserve_on_ahw_after, bnc_in_reserve_on_ahw_before + amount_to_send);
}

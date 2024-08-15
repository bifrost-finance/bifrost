use sp_runtime::BoundedVec;
use parity_scale_codec::Encode;
use bifrost_primitives::BNC;
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
use crate::tests::*;

#[test]
fn send_bnc_from_asset_hub_westend_to_asset_hub_rococo() {
	let destination = Location::new(
		2,
		[GlobalConsensus(NetworkId::Rococo), Parachain(BifrostPolkadot::para_id().into())],
	);
	let bnc_at_bifrost_kusama: Location = Location::new(0, [Junction::from(BoundedVec::try_from(BNC.encode()).unwrap())]);
	let amount = ASSET_HUB_WESTEND_ED * 1_000;

	let signed_origin =
		<BifrostKusama as Chain>::RuntimeOrigin::signed(BifrostKusamaSender::get().into());

	let beneficiary: Location =
		AccountId32Junction { network: None, id: BifrostKusamaReceiver::get().into() }.into();

	let assets: Assets = (bnc_at_bifrost_kusama.clone(), amount).into();
	let fee_asset_item = 0;

	BifrostKusama::force_xcm_version(destination.clone(), XCM_VERSION);

	// fund the AHW's SA on BHW for paying bridge transport fees
	BridgeHubWestend::fund_para_sovereign(BifrostKusama::para_id(), 10_000_000_000_000u128);


	assert_ok!(BifrostKusama::execute_with(|| {
		<BifrostKusama as BifrostKusamaPallet>::PolkadotXcm::limited_teleport_assets(
			signed_origin,
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			fee_asset_item,
			WeightLimit::Unlimited,
		)
	}));


}
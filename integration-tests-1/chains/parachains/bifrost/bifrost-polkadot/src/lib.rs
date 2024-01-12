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

pub mod genesis;

// Substrate
use frame_support::traits::OnInitialize;

// Cumulus
use emulated_integration_tests_common::{
	impl_accounts_helpers_for_parachain, impl_assert_events_helpers_for_parachain,
	impl_assets_helpers_for_parachain, impl_foreign_assets_helpers_for_parachain, impls::Parachain,
	xcm_emulator::decl_test_parachains,
};
use rococo_emulated_chain::Rococo;

// BifrostPolkadot Parachain declaration
decl_test_parachains! {
	pub struct BifrostPolkadot {
		genesis = genesis::genesis(),
		on_init = {
			bifrost_polkadot_runtime::AuraExt::on_initialize(1);
		},
		runtime = bifrost_polkadot_runtime,
		core = {
			XcmpMessageHandler: bifrost_polkadot_runtime::XcmpQueue,
			LocationToAccountId: bifrost_polkadot_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: bifrost_polkadot_runtime::ParachainInfo,
		},
		pallets = {
			PolkadotXcm: bifrost_polkadot_runtime::PolkadotXcm,
			Tokens: bifrost_polkadot_runtime::Tokens,
			ForeignAssets: bifrost_polkadot_runtime::ForeignAssets,
			AssetRegistry: bifrost_polkadot_runtime::AssetRegistry,
			// AssetConversion: bifrost_polkadot_runtime::AssetConversion,
			Balances: bifrost_polkadot_runtime::Balances,
		}
	},
}

// BifrostPolkadot implementation
impl_accounts_helpers_for_parachain!(BifrostPolkadot);
impl_assert_events_helpers_for_parachain!(BifrostPolkadot, false);
// impl_assets_helpers_for_parachain!(BifrostPolkadot, Rococo);
impl_foreign_assets_helpers_for_parachain!(BifrostPolkadot, Rococo);

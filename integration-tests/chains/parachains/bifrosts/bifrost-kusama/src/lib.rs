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
	impl_assets_helpers_for_parachain, impl_assets_helpers_for_system_parachain,
	impl_foreign_assets_helpers_for_parachain, impl_xcm_helpers_for_parachain, impls::Parachain,
	xcm_emulator::decl_test_parachains,
};
use westend_emulated_chain::Westend;

// BifrostKusama Parachain declaration
decl_test_parachains! {
	pub struct BifrostKusama {
		genesis = genesis::genesis(),
		on_init = {
			bifrost_kusama_runtime::AuraExt::on_initialize(1);
		},
		runtime = bifrost_kusama_runtime,
		core = {
			XcmpMessageHandler: bifrost_kusama_runtime::XcmpQueue,
			LocationToAccountId: bifrost_kusama_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: bifrost_kusama_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			PolkadotXcm: bifrost_kusama_runtime::PolkadotXcm,
			Tokens: bifrost_kusama_runtime::Tokens,
			AssetRegistry: bifrost_kusama_runtime::AssetRegistry,
			Balances: bifrost_kusama_runtime::Balances,
		}
	},
}

// BifrostKusama implementation
impl_accounts_helpers_for_parachain!(BifrostKusama);
impl_assert_events_helpers_for_parachain!(BifrostKusama);
// impl_assets_helpers_for_system_parachain!(BifrostKusama, Westend);
// impl_assets_helpers_for_parachain!(BifrostKusama);
// impl_foreign_assets_helpers_for_parachain!(BifrostKusama, xcm::v3::Location);
impl_xcm_helpers_for_parachain!(BifrostKusama);

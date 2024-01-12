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

// Substrate
use sp_core::storage::Storage;

// Cumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, collators, SAFE_XCM_VERSION,
};
use parachains_common::Balance;
use bifrost_polkadot_runtime::constants::currency::DOLLARS;
use bifrost_primitives::{BNC, DOT, GLMR, VBNC, VDOT};
use bifrost_primitives::currency::VGLMR;

pub const PARA_ID: u32 = 2030;
pub const ED: Balance = parachains_common::rococo::currency::EXISTENTIAL_DEPOSIT;

pub fn genesis() -> Storage {
	let genesis_config = bifrost_polkadot_runtime::RuntimeGenesisConfig {
		system: bifrost_polkadot_runtime::SystemConfig::default(),
		balances: bifrost_polkadot_runtime::BalancesConfig {
			balances: accounts::init_balances()
				.iter()
				.cloned()
				.map(|k| (k, ED * 4096 * 4096))
				.collect(),
		},
		parachain_info: bifrost_polkadot_runtime::ParachainInfoConfig {
			parachain_id: PARA_ID.into(),
			..Default::default()
		},
		asset_registry: bifrost_polkadot_runtime::AssetRegistryConfig {
			currency: vec![
				(BNC, DOLLARS / 100, None),
				(DOT, DOLLARS / 10_000, None),
				(GLMR, DOLLARS / 1000_000, None),
			],
			vcurrency: vec![VBNC, VDOT, VGLMR],
			vsbond: vec![],
			..Default::default()
		},
		collator_selection: bifrost_polkadot_runtime::CollatorSelectionConfig {
			invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ED * 16,
			..Default::default()
		},
		session: bifrost_polkadot_runtime::SessionConfig {
			keys: collators::invulnerables()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                                    // account id
						acc,                                            // validator id
						bifrost_polkadot_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		polkadot_xcm: bifrost_polkadot_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		..Default::default()
	};

	build_genesis_storage(
		&genesis_config,
		bifrost_polkadot_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
	)
}

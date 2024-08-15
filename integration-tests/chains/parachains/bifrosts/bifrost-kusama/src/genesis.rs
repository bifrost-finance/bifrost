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
use frame_support::parameter_types;
use sp_core::{sr25519, storage::Storage};

// Cumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, collators, get_account_id_from_seed,
	PenpalSiblingSovereignAccount, PenpalTeleportableAssetLocation, RESERVABLE_ASSET_ID,
	SAFE_XCM_VERSION,
};
use parachains_common::{AccountId, Balance};
use bifrost_kusama_runtime::CurrencyId::{Token, Token2};
use bifrost_kusama_runtime::TokenSymbol::KSM;
use emulated_integration_tests_common::accounts::ALICE;

pub const PARA_ID: u32 = 2001;
pub const ED: Balance = testnet_parachains_constants::westend::currency::EXISTENTIAL_DEPOSIT;

parameter_types! {
	pub AssetHubWestendAssetOwner: AccountId = get_account_id_from_seed::<sr25519::Public>("Alice");
}

pub fn genesis() -> Storage {
	let genesis_config = bifrost_kusama_runtime::RuntimeGenesisConfig {
		system: bifrost_kusama_runtime::SystemConfig::default(),
		balances: bifrost_kusama_runtime::BalancesConfig {
			balances: accounts::init_balances().iter().cloned().map(|k| (k, 1000_000_000_000_000_000)).collect(),
		},
		tokens: bifrost_kusama_runtime::TokensConfig { balances:
		vec![
			(get_account_id_from_seed::<sr25519::Public>(ALICE),
			Token(KSM),
			1_000_000_000_000_000_000u128)
		]},
		parachain_info: bifrost_kusama_runtime::ParachainInfoConfig {
			parachain_id: PARA_ID.into(),
			..Default::default()
		},
		session: bifrost_kusama_runtime::SessionConfig {
			keys: collators::invulnerables()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                                     // account id
						acc,                                             // validator id
						bifrost_kusama_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		polkadot_xcm: bifrost_kusama_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(3),
			..Default::default()
		},
		..Default::default()
	};

	build_genesis_storage(
		&genesis_config,
		bifrost_kusama_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
	)
}

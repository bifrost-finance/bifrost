// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use std::path::PathBuf;
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use bifrost_runtime::{AccountId, AuraId, Balance, AuraConfig, BalancesConfig, GenesisConfig, IndicesConfig, SudoConfig, SystemConfig, ParachainInfoConfig, VestingConfig, WASM_BINARY, BlockNumber};
use super::TELEMETRY_URL;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};

use crate::chain_spec::{RelayExtensions, get_account_id_from_seed, get_from_seed};
use bifrost_runtime::constants::currency::DOLLARS;

const DEFAULT_PROTOCOL_ID: &str = "bifrost";

/// Specialized `ChainSpec` for the bifrost runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

pub fn bifrost_genesis(
	initial_authorities: Vec<AuraId>,
	root_key: AccountId,
	balances: Vec<(AccountId, Balance)>,
	vestings: Vec<(AccountId, BlockNumber, BlockNumber, Balance)>,
	id: ParaId,
) -> GenesisConfig {
	GenesisConfig {
		frame_system: SystemConfig {
			code: WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: BalancesConfig {
			balances
		},
		pallet_indices: IndicesConfig {
			indices: vec![],
		},
		pallet_sudo: SudoConfig {
			key: root_key.clone(),
		},
		parachain_info: ParachainInfoConfig { parachain_id: id },
		// pallet_collator_selection: statemine_runtime::CollatorSelectionConfig {
		// 	invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
		// 	candidacy_bond: STATEMINE_ED * 16,
		// 	..Default::default()
		// },
		// pallet_session: statemine_runtime::SessionConfig {
		// 	keys: initial_authorities.iter().cloned().map(|(acc, aura)| (
		// 		acc.clone(), // account id
		// 		acc.clone(), // validator id
		// 		statemine_session_keys(aura), // session keys
		// 	)).collect()
		// },
		pallet_aura: AuraConfig {
			authorities: initial_authorities,
		},
		cumulus_pallet_aura_ext: Default::default(),
		cumulus_pallet_parachain_system: Default::default(),
		pallet_vesting:  VestingConfig {
			vesting: vestings
		},
	}
}

fn development_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts: Vec<AccountId> = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice")
	];
	const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

	bifrost_genesis(
		vec![get_from_seed::<AuraId>("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
		endowed_accounts.iter().cloned().map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT/4)).collect(),
		id,
	)
}

pub fn development_config(id: ParaId) -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		"Bifrost Development",
		"bifrost_dev",
		ChainType::Development,
		move || development_config_genesis(id),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		RelayExtensions {
			relay_chain: "kusama-dev".into(),
			para_id: id.into(),
		},
	))
}

fn local_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts: Vec<AccountId> = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
	];
	const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

	bifrost_genesis(
		vec![
			get_from_seed::<AuraId>("Alice"),
			get_from_seed::<AuraId>("Bob"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
		endowed_accounts.iter().cloned().map(|x| (x, 0u32, 100u32, ENDOWMENT/4)).collect(),
		id,
	)
}

pub fn local_testnet_config(id: ParaId) -> Result<ChainSpec, String> {

	Ok(ChainSpec::from_genesis(
		"Bifrost Local Testnet",
		"bifrost_local_testnet",
		ChainType::Local,
		move || local_config_genesis(id),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		RelayExtensions {
			relay_chain: "kusama-local".into(),
			para_id: id.into(),
		},
	))
}

pub fn chainspec_config(id: ParaId) -> ChainSpec {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "BNC".into());
	properties.insert("tokenDecimals".into(), 12.into());

	ChainSpec::from_genesis(
		"Bifrost",
		"bifrost",
		ChainType::Live,
		move || {
			bifrost_config_genesis(id)
		},
		vec![],
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties),
		RelayExtensions {
			relay_chain: "kusama".into(),
			para_id: id.into(),
		},
	)
}

fn bifrost_config_genesis(id: ParaId) -> GenesisConfig {
	let initial_authorities: Vec<AuraId> = vec![
		 // 5H6pFYqLatuQbnLLzKFUazX1VXjmqhnJQT6hVWVz67kaT94z
		 hex!["dec92f12684928aa042297f6d8927930b82d9ef28b1dfa1974e6a88c51c6ee75"].unchecked_into(),
		 // 5DPiyVYRVUghxtYz5qPcUMAci5GPnL9sBYawqmDFp2YH76hh
		 hex!["3abda893fc4ce0c3d465ea434cf513bed824f1c2b564cf38003a72c47fda7147"].unchecked_into(),
		 // 5HgpFg4DXfg2GZ5gKcRAtarF168y9SAi5zeAP7JRig2NW5Br
		 hex!["f8b788ebec50ba10e2676c6d59842dd1127b7701977d7daf3172016ac0d4632e"].unchecked_into(),
		 // 5EtBGed7DkcURQSc3NAfQqVz6wcxgkj8wQBh6JsrjDSuvmQL
		 hex!["7cad48689d421015bb3b449a365fdbd2a2d3070df2d42f8077d8f714d88ad200"].unchecked_into(),
		 // 5DLHpKfdUCki9xYYYKCrWCVE6PfX2U1gLG7f6sGj9uHyS9MC
		 hex!["381f3b88a3bc9872c7137f8bfbd24ae039bfa5845cba51ffa2ad8e4d03d1af1a"].unchecked_into(),
	 ];

	let root_key: AccountId = hex![
		// cjAZA391BNi2S1Je7PNGHiX4UoJh3SbknQSDQ7qh3g4Aa9H
		"2c64a40ec236d0a0823065791946f6254c4577c6110f512614bd6ece1a3fa22b"
	].into();

	let balances_configs: Vec<BalancesConfig> =
		super::config_from_json_files(PathBuf::from("./res/genesis_config/balances/"))
			.unwrap();

	let vesting_configs: Vec<VestingConfig> =
		super::config_from_json_files(PathBuf::from("./res/genesis_config/vesting/"))
			.unwrap();

	bifrost_genesis(
		initial_authorities,
		root_key,
		balances_configs.into_iter().flat_map(|bc| bc.balances).collect(),
		vesting_configs.into_iter().flat_map(|vc| vc.vesting).collect(),
		id,
	)
}

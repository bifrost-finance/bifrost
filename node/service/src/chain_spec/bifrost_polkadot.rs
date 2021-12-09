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

use bifrost_polkadot_runtime::{
	constants::currency::DOLLARS, AccountId, AuraId, Balance, BalancesConfig, BlockNumber,
	CollatorSelectionConfig, GenesisConfig, IndicesConfig, ParachainInfoConfig, PolkadotXcmConfig,
	SessionConfig, SudoConfig, SystemConfig, VestingConfig, WASM_BINARY,
};
use cumulus_primitives_core::ParaId;
use frame_benchmarking::{account, whitelisted_caller};
use hex_literal::hex;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::traits::Zero;

use super::TELEMETRY_URL;
use crate::chain_spec::{get_account_id_from_seed, get_from_seed, RelayExtensions};

const DEFAULT_PROTOCOL_ID: &str = "bifrost";

/// Specialized `ChainSpec` for the bifrost-polkadot runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

pub fn bifrost_polkadot_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	balances: Vec<(AccountId, Balance)>,
	vestings: Vec<(AccountId, BlockNumber, BlockNumber, Balance)>,
	id: ParaId,
) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			code: WASM_BINARY.expect("WASM binary was not build, please build it!").to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig { balances },
		indices: IndicesConfig { indices: vec![] },
		treasury: Default::default(),
		parachain_info: ParachainInfoConfig { parachain_id: id },
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: Zero::zero(),
			..Default::default()
		},
		session: SessionConfig {
			keys: invulnerables
				.iter()
				.cloned()
				.map(|(acc, aura)| {
					(
						acc.clone(),                                    // account id
						acc.clone(),                                    // validator id
						bifrost_polkadot_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		vesting: VestingConfig { vesting: vestings },
		polkadot_xcm: PolkadotXcmConfig { safe_xcm_version: Some(2) },
		sudo: SudoConfig { key: root_key.clone() },
	}
}

fn development_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		whitelisted_caller(), // Benchmarking whitelist_account
	];
	let balances = endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect();
	let vestings = endowed_accounts
		.iter()
		.cloned()
		.map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT / 4))
		.collect();

	bifrost_polkadot_genesis(
		vec![(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_from_seed::<AuraId>("Alice"),
		)],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		balances,
		vestings,
		id,
	)
}

pub fn development_config(id: ParaId) -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		"Bifrost Polkadot Development",
		"bifrost_polkadot_dev",
		ChainType::Development,
		move || development_config_genesis(id),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		RelayExtensions { relay_chain: "polkadot-dev".into(), para_id: id.into() },
	))
}

fn local_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
		get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
		get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		whitelisted_caller(), // Benchmarking whitelist_account
		account("bechmarking_account_1", 0, 0), /* Benchmarking account_1, used for interacting
		                       * with whitelistted_caller */
	];
	let balances = endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect();
	let vestings = endowed_accounts
		.iter()
		.cloned()
		.map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT / 4))
		.collect();

	bifrost_polkadot_genesis(
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<AuraId>("Alice"),
			),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), get_from_seed::<AuraId>("Bob")),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		balances,
		vestings,
		id,
	)
}

pub fn local_testnet_config(id: ParaId) -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		"Bifrost Polkadot Local Testnet",
		"bifrost_polkadot_local_testnet",
		ChainType::Local,
		move || local_config_genesis(id),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		RelayExtensions { relay_chain: "polkadot-local".into(), para_id: id.into() },
	))
}

pub fn chainspec_config(id: ParaId) -> ChainSpec {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "BNC".into());
	properties.insert("tokenDecimals".into(), 12.into());

	ChainSpec::from_genesis(
		"Bifrost Polkadot",
		"bifrost_polkadot",
		ChainType::Live,
		move || bifrost_polkadot_config_genesis(id),
		vec![],
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties),
		RelayExtensions { relay_chain: "polkadot".into(), para_id: id.into() },
	)
}

fn bifrost_polkadot_config_genesis(id: ParaId) -> GenesisConfig {
	let invulnerables: Vec<(AccountId, AuraId)> = vec![
		(
			// dpEZwz5nHxEjQXcm3sjy6NTz83EGcBRXMBSyuuWSguiVGJB
			hex!["5c7e9ccd1045cac7f8c5c77a79c87f44019d1dda4f5032713bda89c5d73cb36b"].into(),
			hex!["5c7e9ccd1045cac7f8c5c77a79c87f44019d1dda4f5032713bda89c5d73cb36b"]
				.unchecked_into(),
		),
		(
			// duNwrtscWpfuTzRkjtt431kUj1gsfwbPi1bzdQL4cmk9QAa
			hex!["606b0aad375ae1715fbe6a07315136a8e9c1c84a91230f6a0c296c2953581335"].into(),
			hex!["606b0aad375ae1715fbe6a07315136a8e9c1c84a91230f6a0c296c2953581335"]
				.unchecked_into(),
		),
		(
			// gPQG97HPe54fJpLoFePwm3fxdJaU2VV71hYbqd4RJcNeQfe
			hex!["ce42cea2dd0d4ac87ccdd5f0f2e1010955467f5a37587cf6af8ee2b4ba781034"].into(),
			hex!["ce42cea2dd0d4ac87ccdd5f0f2e1010955467f5a37587cf6af8ee2b4ba781034"]
				.unchecked_into(),
		),
		(
			// frYfsZhdVuG6Ap6AyJQLSHVqtKmUyqxo6ohnrmGTDk2neXK
			hex!["b6ba81e73bd39203e006fc99cc1e41976745de2ea2007bf62ed7c9a48ccc5b1d"].into(),
			hex!["b6ba81e73bd39203e006fc99cc1e41976745de2ea2007bf62ed7c9a48ccc5b1d"]
				.unchecked_into(),
		),
	];

	let root_key: AccountId = hex![
		// cjAZA391BNi2S1Je7PNGHiX4UoJh3SbknQSDQ7qh3g4Aa9H
		"2c64a40ec236d0a0823065791946f6254c4577c6110f512614bd6ece1a3fa22b"
	]
	.into();

	let balances = vec![(root_key.clone(), 10_000 * DOLLARS)];

	bifrost_polkadot_genesis(
		invulnerables,
		root_key,
		balances,
		// vesting_configs.into_iter().flat_map(|vc| vc.vesting).collect(),
		vec![], // temporarily set vesting as blank
		id,
	)
}

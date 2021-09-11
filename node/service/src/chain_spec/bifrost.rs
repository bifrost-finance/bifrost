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

use std::{
	fs::{read_dir, File},
	path::PathBuf,
};

use bifrost_runtime::{
	constants::currency::DOLLARS, AccountId, AuraId, Balance, BalancesConfig, BancorConfig,
	BlockNumber, CollatorSelectionConfig, CouncilConfig, DemocracyConfig, GenesisConfig,
	IndicesConfig, ParachainInfoConfig, SessionConfig, SudoConfig, SystemConfig,
	TechnicalCommitteeConfig, TokensConfig, VestingConfig, WASM_BINARY,
};
use cumulus_primitives_core::ParaId;
use frame_benchmarking::{account, whitelisted_caller};
use hex_literal::hex;
use node_primitives::{CurrencyId, TokenSymbol};
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::de::DeserializeOwned;
use serde_json as json;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::traits::Zero;

use super::TELEMETRY_URL;
use crate::chain_spec::{get_account_id_from_seed, get_from_seed, RelayExtensions};

const DEFAULT_PROTOCOL_ID: &str = "bifrost";

/// Specialized `ChainSpec` for the bifrost runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

pub fn bifrost_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	balances: Vec<(AccountId, Balance)>,
	vestings: Vec<(AccountId, BlockNumber, BlockNumber, Balance)>,
	id: ParaId,
	tokens: Vec<(AccountId, CurrencyId, Balance)>,
) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			code: WASM_BINARY.expect("WASM binary was not build, please build it!").to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig { balances },
		indices: IndicesConfig { indices: vec![] },
		democracy: DemocracyConfig::default(),
		council: CouncilConfig { members: vec![], phantom: Default::default() },
		technical_committee: TechnicalCommitteeConfig {
			members: vec![],
			phantom: Default::default(),
		},
		council_membership: Default::default(),
		technical_membership: Default::default(),
		treasury: Default::default(),
		phragmen_election: Default::default(),
		sudo: SudoConfig { key: root_key.clone() },
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
						acc.clone(),                           // account id
						acc.clone(),                           // validator id
						bifrost_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		vesting: VestingConfig { vesting: vestings },
		tokens: TokensConfig { balances: tokens },
		bancor: BancorConfig {
			bancor_pools: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 10_000 * DOLLARS),
				(CurrencyId::Token(TokenSymbol::KSM), 1_000_000 * DOLLARS),
			],
		},
	}
}

fn development_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		whitelisted_caller(), // Benchmarking whitelist_account
	];
	let balances = endowed_accounts
		.iter()
		.chain(faucet_accounts().iter())
		.cloned()
		.map(|x| (x, ENDOWMENT))
		.collect();
	let vestings = endowed_accounts
		.iter()
		.cloned()
		.map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT / 4))
		.collect();
	let tokens = endowed_accounts
		.iter()
		.chain(faucet_accounts().iter())
		.flat_map(|x| {
			vec![
				(x.clone(), CurrencyId::Stable(TokenSymbol::KUSD), ENDOWMENT * 10_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT * 4_000_000),
				(x.clone(), CurrencyId::VSToken(TokenSymbol::DOT), ENDOWMENT * 4_000_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::ETH), ENDOWMENT),
				(x.clone(), CurrencyId::Token(TokenSymbol::KSM), ENDOWMENT),
			]
		})
		.collect();

	bifrost_genesis(
		vec![(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_from_seed::<AuraId>("Alice"),
		)],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		balances,
		vestings,
		id,
		tokens,
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
		RelayExtensions { relay_chain: "kusama-dev".into(), para_id: id.into() },
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
	let balances = endowed_accounts
		.iter()
		.chain(faucet_accounts().iter())
		.cloned()
		.map(|x| (x, ENDOWMENT))
		.collect();
	let vestings = endowed_accounts
		.iter()
		.cloned()
		.map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT / 4))
		.collect();
	let tokens = endowed_accounts
		.iter()
		.chain(faucet_accounts().iter())
		.flat_map(|x| {
			vec![
				(x.clone(), CurrencyId::Stable(TokenSymbol::KUSD), ENDOWMENT * 10_000),
				(x.clone(), CurrencyId::VSToken(TokenSymbol::DOT), ENDOWMENT * 4_000_000),
				(x.clone(), CurrencyId::VToken(TokenSymbol::DOT), ENDOWMENT * 4_000_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT * 4_000_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::ETH), ENDOWMENT * 4_000_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::KSM), ENDOWMENT * 4_000_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::ASG), ENDOWMENT * 4_000_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::KUSD), ENDOWMENT * 4_000_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::BNC), ENDOWMENT * 4_000_000),
				(
					x.clone(),
					CurrencyId::VSBond(TokenSymbol::KSM, 3000, 13, 20),
					ENDOWMENT * 4_000_000,
				),
			]
		})
		.collect();

	bifrost_genesis(
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
		tokens,
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
		RelayExtensions { relay_chain: "kusama-local".into(), para_id: id.into() },
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
		move || bifrost_config_genesis(id),
		vec![],
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties),
		RelayExtensions { relay_chain: "kusama".into(), para_id: id.into() },
	)
}

fn bifrost_config_genesis(id: ParaId) -> GenesisConfig {
	let invulnerables: Vec<(AccountId, AuraId)> = vec![
		(
			// eunwjK45qDugPXhnjxGUcMbifgdtgefzoW7PgMMpr39AXwh
			hex!["8cf80f0bafcd0a3d80ca61cb688e4400e275b39d3411b4299b47e712e9dab809"].into(),
			hex!["8cf80f0bafcd0a3d80ca61cb688e4400e275b39d3411b4299b47e712e9dab809"]
				.unchecked_into(),
		),
		(
			// dBkoWVdQCccH1xNAeR1Y4vrETt3a4j4iU8Ct2ewY1FUjasL
			hex!["40ac4effe39181731a8feb8a8ee0780e177bdd0d752b09c8fd71047e67189022"].into(),
			hex!["40ac4effe39181731a8feb8a8ee0780e177bdd0d752b09c8fd71047e67189022"]
				.unchecked_into(),
		),
		(
			// dwrEwfj2RFU4DS6EiTCfmxMpQ1sAsaHykftzwoptFe4a8aH
			hex!["624d6a004c72a1abcf93131e185515ebe1410e43a301fe1f25d20d8da345376e"].into(),
			hex!["624d6a004c72a1abcf93131e185515ebe1410e43a301fe1f25d20d8da345376e"]
				.unchecked_into(),
		),
		(
			// fAjW6bwT4GKgW88sjZfNLRr5hWyMM9T9ZwqHYkFiSxw4Yhp
			hex!["985d2738e512909c81289e6055e60a6824818964535ecfbf10e4d69017084756"].into(),
			hex!["985d2738e512909c81289e6055e60a6824818964535ecfbf10e4d69017084756"]
				.unchecked_into(),
		),
	];

	let root_key: AccountId = hex![
		// cjAZA391BNi2S1Je7PNGHiX4UoJh3SbknQSDQ7qh3g4Aa9H
		"2c64a40ec236d0a0823065791946f6254c4577c6110f512614bd6ece1a3fa22b"
	]
	.into();

	let exe_dir = {
		let mut exe_dir = std::env::current_exe().unwrap();
		exe_dir.pop();

		exe_dir
	};

	let balances_configs: Vec<BalancesConfig> =
		config_from_json_files(exe_dir.join("res/genesis_config/balances")).unwrap();

	let mut total_issuance: Balance = Zero::zero();
	let balances = balances_configs
		.into_iter()
		.flat_map(|bc| bc.balances)
		.fold(BTreeMap::<AccountId, Balance>::new(), |mut acc, (account_id, amount)| {
			if let Some(balance) = acc.get_mut(&account_id) {
				*balance = balance
					.checked_add(amount)
					.expect("balance cannot overflow when building genesis");
			} else {
				acc.insert(account_id.clone(), amount);
			}

			total_issuance = total_issuance
				.checked_add(amount)
				.expect("total insurance cannot overflow when building genesis");
			acc
		})
		.into_iter()
		.collect();

	assert_eq!(total_issuance, 32_000_000 * DOLLARS, "total issuance must be equal to 320 million");

	let vesting_configs: Vec<VestingConfig> =
		config_from_json_files(exe_dir.join("res/genesis_config/vesting")).unwrap();

	use sp_core::sp_std::collections::btree_map::BTreeMap;
	bifrost_genesis(
		invulnerables,
		root_key,
		balances,
		vesting_configs.into_iter().flat_map(|vc| vc.vesting).collect(),
		id,
		vec![],
	)
}

fn config_from_json_file<T: DeserializeOwned>(path: PathBuf) -> Result<T, String> {
	let file = File::open(&path).map_err(|e| format!("Error opening genesis config: {}", e))?;

	let config =
		json::from_reader(file).map_err(|e| format!("Error parsing config file: {}", e))?;

	Ok(config)
}

fn config_from_json_files<T: DeserializeOwned>(dir: PathBuf) -> Result<Vec<T>, String> {
	let mut configs = vec![];

	let iter = read_dir(&dir).map_err(|e| format!("Error opening directory: {}", e))?;
	for entry in iter {
		let path = entry.map_err(|e| format!("{}", e))?.path();

		if !path.is_dir() {
			configs.push(config_from_json_file(path)?);
		}
	}

	Ok(configs)
}

pub fn faucet_accounts() -> Vec<AccountId> {
	vec![
		hex!["ce6072037670ca8e974fd571eae4f215a58d0bf823b998f619c3f87a911c3541"].into(), /* asgard sudo account */
		hex!["a2d57b8e781327bd2853b36e6f290bd8beeaa850971c9b0789ec4969f8beb01b"].into(), /* bifrost-faucet */
		hex!["a272fa6e2282767b61a299e81023d44ef583c640fef99b0bafe216399775cd17"].into(),
		hex!["56f6e7bb0826cd128672ad3a03016533834123c319adc635c6db595c6f72272e"].into(),
		hex!["7e9005c247601a0d0e967f68b03f6e39e402a735ec65c20e4965c6d94a22e42f"].into(),
		hex!["f2449dfbb431a5f9e8dc7468e5f3521baff4c0125edcda746f38df5295d5fb28"].into(),
		hex!["aaa565b52ea12bf3c8d7abb79411976bccd8054c5581922acc0165ad88640f09"].into(),
		hex!["8afadc065940f22f73b745aab694b1b20cafea3d4e52adad844f581614fbdd00"].into(),
		hex!["0831325e2b4953f247db9df3f6452becbf23d8f7f806c0396ad853cb3c284d06"].into(),
		hex!["7ea84934a575487fb02c44e01f4488c2f242cdbf48052630780dcd8ac567950c"].into(),
		hex!["ee05492a82cb982392aad78f7e6f6fff56eaee4988fd9961ebb84e177dd6526d"].into(), /* bifrost-faucet */
		hex!["7435653321694ee115e8cea8c8e117c0b6703b6fb91298b6df5adeef7679a46f"].into(), /* danny testing account */
		hex!["263c78393f33b23cd23f3211726b2316e950910749d20c1552ea6972091a645e"].into(), /* jianbo testing account */
		hex!["803feefeab8e5c81c3d268038b6c494d3018714fc8c5d08cf027111fd8114b06"].into(), /* tieqiao testing account */
		hex!["8898ffd2cb04fb751655ede7bc0081b6b6ebe13cd0bdee5bbb9273e6dcc9b91c"].into(), /* tyrone testing account */
	]
}

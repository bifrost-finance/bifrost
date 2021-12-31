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

use bifrost_kusama_runtime::{
	AccountId, AuraId, Balance, BalancesConfig, BlockNumber, CollatorSelectionConfig,
	CouncilConfig, CouncilMembershipConfig, DemocracyConfig, GenesisConfig, IndicesConfig,
	ParachainInfoConfig, PolkadotXcmConfig, SS58Prefix, SalpConfig, SalpLiteConfig, SessionConfig,
	SystemConfig, TechnicalCommitteeConfig, TechnicalMembershipConfig, TokensConfig, VestingConfig,
	WASM_BINARY,
};
use bifrost_runtime_common::dollar;
use cumulus_primitives_core::ParaId;
use frame_benchmarking::{account, whitelisted_caller};
use hex_literal::hex;
use node_primitives::{CurrencyId, TokenInfo, TokenSymbol};
use sc_chain_spec::Properties;
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

#[allow(non_snake_case)]
pub fn ENDOWMENT() -> u128 {
	1_000_000 * dollar(CurrencyId::Native(TokenSymbol::BNC))
}

pub const PARA_ID: u32 = 2001;

fn bifrost_kusama_properties() -> Properties {
	let mut properties = sc_chain_spec::Properties::new();
	let mut token_symbol: Vec<String> = vec![];
	let mut token_decimals: Vec<u32> = vec![];
	[
		// native token
		CurrencyId::Native(TokenSymbol::BNC),
		// stable token
		CurrencyId::Stable(TokenSymbol::KUSD),
		// token
		CurrencyId::Token(TokenSymbol::DOT),
		CurrencyId::Token(TokenSymbol::KSM),
		CurrencyId::Token(TokenSymbol::KAR),
		CurrencyId::Token(TokenSymbol::ZLK),
		CurrencyId::Token(TokenSymbol::PHA),
	]
	.iter()
	.for_each(|token| {
		token_symbol.push(token.symbol().to_string());
		token_decimals.push(token.decimals() as u32);
	});

	properties.insert("tokenSymbol".into(), token_symbol.into());
	properties.insert("tokenDecimals".into(), token_decimals.into());
	properties.insert("ss58Format".into(), SS58Prefix::get().into());

	properties
}

pub fn bifrost_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	balances: Vec<(AccountId, Balance)>,
	vestings: Vec<(AccountId, BlockNumber, BlockNumber, Balance)>,
	id: ParaId,
	tokens: Vec<(AccountId, CurrencyId, Balance)>,
	council_membership: Vec<AccountId>,
	technical_committee_membership: Vec<AccountId>,
	salp_multisig_key: AccountId,
	salp_lite_multisig_key_salp: AccountId,
) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			code: WASM_BINARY.expect("WASM binary was not build, please build it!").to_vec(),
		},
		balances: BalancesConfig { balances },
		indices: IndicesConfig { indices: vec![] },
		democracy: DemocracyConfig::default(),
		council_membership: CouncilMembershipConfig {
			members: council_membership,
			phantom: Default::default(),
		},
		technical_membership: TechnicalMembershipConfig {
			members: technical_committee_membership,
			phantom: Default::default(),
		},
		council: CouncilConfig { members: vec![], phantom: Default::default() },
		technical_committee: TechnicalCommitteeConfig {
			members: vec![],
			phantom: Default::default(),
		},
		treasury: Default::default(),
		phragmen_election: Default::default(),
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
						acc.clone(),                                  // account id
						acc.clone(),                                  // validator id
						bifrost_kusama_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		vesting: VestingConfig { vesting: vestings },
		tokens: TokensConfig { balances: tokens },
		polkadot_xcm: PolkadotXcmConfig { safe_xcm_version: Some(2) },
		salp: SalpConfig { initial_multisig_account: Some(salp_multisig_key) },
		salp_lite: SalpLiteConfig { initial_multisig_account: Some(salp_lite_multisig_key_salp) },
	}
}

fn development_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		whitelisted_caller(), // Benchmarking whitelist_account
	];
	let balances = endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT())).collect();
	let vestings = endowed_accounts
		.iter()
		.cloned()
		.map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT() / 4))
		.collect();
	let tokens = endowed_accounts
		.iter()
		.flat_map(|x| {
			vec![
				(x.clone(), CurrencyId::Stable(TokenSymbol::KUSD), ENDOWMENT() * 10_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::KAR), ENDOWMENT() * 10_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::KSM), ENDOWMENT()),
				(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT()),
				(x.clone(), CurrencyId::VSToken(TokenSymbol::DOT), ENDOWMENT()),
			]
		})
		.collect();

	let council_membership = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let technical_committee_membership = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];

	let salp_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();

	let salp_lite_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();

	bifrost_genesis(
		vec![(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_from_seed::<AuraId>("Alice"),
		)],
		balances,
		vestings,
		id,
		tokens,
		council_membership,
		technical_committee_membership,
		salp_multisig,
		salp_lite_multisig,
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
		Some(bifrost_kusama_properties()),
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
	let balances = endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT())).collect();
	let vestings = endowed_accounts
		.iter()
		.cloned()
		.map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT() / 4))
		.collect();
	let tokens = endowed_accounts
		.iter()
		.flat_map(|x| {
			vec![
				(x.clone(), CurrencyId::Stable(TokenSymbol::KUSD), ENDOWMENT() * 4_000_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::KSM), ENDOWMENT() * 4_000_000),
				(x.clone(), CurrencyId::Native(TokenSymbol::BNC), ENDOWMENT() * 4_000_000),
				(x.clone(), CurrencyId::VSToken(TokenSymbol::KSM), ENDOWMENT() * 4_000_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT() * 4_000_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::KAR), ENDOWMENT() * 4_000_000),
				(x.clone(), CurrencyId::VSToken(TokenSymbol::DOT), ENDOWMENT() * 4_000_000),
				(
					x.clone(),
					CurrencyId::VSBond(TokenSymbol::BNC, 2001, 13, 20),
					ENDOWMENT() * 4_000_000,
				),
				(
					x.clone(),
					CurrencyId::VSBond(TokenSymbol::KSM, 3000, 13, 20),
					ENDOWMENT() * 4_000_000,
				),
			]
		})
		.collect();

	let council_membership = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let technical_committee_membership = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let salp_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();
	let salp_lite_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();

	bifrost_genesis(
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<AuraId>("Alice"),
			),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), get_from_seed::<AuraId>("Bob")),
		],
		balances,
		vestings,
		id,
		tokens,
		council_membership,
		technical_committee_membership,
		salp_multisig,
		salp_lite_multisig,
	)
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		"Bifrost Local Testnet",
		"bifrost_local_testnet",
		ChainType::Local,
		move || local_config_genesis(PARA_ID.into()),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		Some(bifrost_kusama_properties()),
		RelayExtensions { relay_chain: "kusama-local".into(), para_id: PARA_ID },
	))
}

pub fn chainspec_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Bifrost",
		"bifrost",
		ChainType::Live,
		move || bifrost_config_genesis(PARA_ID.into()),
		vec![],
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		Some(DEFAULT_PROTOCOL_ID),
		Some(bifrost_kusama_properties()),
		RelayExtensions { relay_chain: "kusama".into(), para_id: PARA_ID },
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

	assert_eq!(
		total_issuance,
		32_000_000 * dollar(CurrencyId::Native(TokenSymbol::BNC)),
		"total issuance must be equal to 320 million"
	);

	let vesting_configs: Vec<VestingConfig> =
		config_from_json_files(exe_dir.join("res/genesis_config/vesting")).unwrap();

	let salp_multisig: AccountId =
		hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();
	let salp_lite_multisig: AccountId =
		hex!["e4f78719c654cd8e8ac1375c447b7a80f9476cfe6505ea401c4b15bd6b967c93"].into();

	use sp_core::sp_std::collections::btree_map::BTreeMap;
	bifrost_genesis(
		invulnerables,
		balances,
		vesting_configs.into_iter().flat_map(|vc| vc.vesting).collect(),
		id,
		vec![], // tokens
		vec![], // council membership
		vec![], // technical committee membership
		salp_multisig,
		salp_lite_multisig,
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

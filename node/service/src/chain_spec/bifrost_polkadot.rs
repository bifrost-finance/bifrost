// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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
	constants::currency::DOLLARS, AccountId, Balance, BalancesConfig, BlockNumber,
	CollatorSelectionConfig, GenesisConfig, IndicesConfig, ParachainInfoConfig, PolkadotXcmConfig,
	SS58Prefix, SessionConfig, SudoConfig, SystemConfig, TokensConfig, VestingConfig, WASM_BINARY,
};
use bifrost_runtime_common::{dollar, AuraId};
use cumulus_primitives_core::ParaId;
use frame_benchmarking::{account, whitelisted_caller};
use hex_literal::hex;
use node_primitives::{CurrencyId, TokenInfo, TokenSymbol};
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::traits::Zero;

use super::TELEMETRY_URL;
use crate::chain_spec::{get_account_id_from_seed, get_from_seed, RelayExtensions};

const DEFAULT_PROTOCOL_ID: &str = "bifrost_polkadot";

/// Specialized `ChainSpec` for the bifrost-polkadot runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

#[allow(non_snake_case)]
pub fn ENDOWMENT() -> u128 {
	1_000_000 * dollar(CurrencyId::Native(TokenSymbol::BNC))
}

pub const PARA_ID: u32 = 2030;

fn bifrost_polkadot_properties() -> Properties {
	let mut properties = sc_chain_spec::Properties::new();
	let mut token_symbol: Vec<String> = vec![];
	let mut token_decimals: Vec<u32> = vec![];
	[
		// native token
		CurrencyId::Native(TokenSymbol::BNC),
	]
	.iter()
	.for_each(|token| {
		token_symbol.push(token.symbol().expect("Token symbol expected").to_string());
		token_decimals.push(token.decimals().expect("Token decimals expected") as u32);
	});

	properties.insert("tokenSymbol".into(), token_symbol.into());
	properties.insert("tokenDecimals".into(), token_decimals.into());
	properties.insert("ss58Format".into(), SS58Prefix::get().into());

	properties
}

pub fn bifrost_polkadot_genesis(
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
		},
		balances: BalancesConfig { balances },
		indices: IndicesConfig { indices: vec![] },
		democracy: Default::default(),
		council_membership: Default::default(),
		technical_membership: Default::default(),
		council: Default::default(),
		technical_committee: Default::default(),
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
						acc.clone(),                                    // account id
						acc,                                            // validator id
						bifrost_polkadot_runtime::SessionKeys { aura }, // session keys
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
		sudo: SudoConfig { key: Some(root_key) },
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
		.map(|x| (x, 0u32, 100u32, ENDOWMENT() / 4))
		.collect();
	let tokens = endowed_accounts
		.iter()
		.flat_map(|x| vec![(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT())])
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
		tokens,
	)
}

pub fn development_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		"Bifrost Polkadot Development",
		"bifrost_polkadot_dev",
		ChainType::Development,
		move || development_config_genesis(PARA_ID.into()),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Some(bifrost_polkadot_properties()),
		RelayExtensions { relay_chain: "polkadot-dev".into(), para_id: PARA_ID },
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
		.map(|x| (x, 0u32, 100u32, ENDOWMENT() / 4))
		.collect();
	let tokens = endowed_accounts
		.iter()
		.flat_map(|x| {
			vec![(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT() * 4_000_000)]
		})
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
		tokens,
	)
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		"Bifrost Polkadot Local Testnet",
		"bifrost_polkadot_local_testnet",
		ChainType::Local,
		move || local_config_genesis(PARA_ID.into()),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Some(bifrost_polkadot_properties()),
		RelayExtensions { relay_chain: "polkadot-local".into(), para_id: PARA_ID },
	))
}

pub fn chainspec_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Bifrost Polkadot",
		"bifrost_polkadot",
		ChainType::Live,
		move || bifrost_polkadot_config_genesis(PARA_ID.into()),
		vec![],
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Some(bifrost_polkadot_properties()),
		RelayExtensions { relay_chain: "polkadot".into(), para_id: PARA_ID },
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

	let balances = vec![(root_key.clone(), 1000 * DOLLARS)];

	bifrost_polkadot_genesis(invulnerables, root_key, balances, vec![], id, vec![])
}

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

use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use asgard_runtime::{
	AccountId, AuraId,
	constants::{currency::DOLLARS, time::DAYS},
	AuraConfig, AssetsConfig, BalancesConfig, GenesisConfig, IndicesConfig, MinterRewardConfig,
	SudoConfig, SystemConfig, VoucherConfig, VtokenMintConfig, CouncilConfig, TechnicalCommitteeConfig,
	DemocracyConfig, ParachainInfoConfig, WASM_BINARY,
};
use super::TELEMETRY_URL;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::Permill;

use crate::chain_spec::{RelayExtensions, get_account_id_from_seed, testnet_accounts, get_from_seed, initialize_all_vouchers};
use node_primitives::{CurrencyId, TokenSymbol};

const DEFAULT_PROTOCOL_ID: &str = "asgard";

/// Specialized `ChainSpec` for the asgard runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

/// Helper function to create asgard GenesisConfig for testing
pub fn asgard_genesis(
	initial_authorities: Vec<AuraId>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
	id: ParaId,
) -> GenesisConfig {
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);
	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

	GenesisConfig {
		frame_system: SystemConfig {
			code: WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: BalancesConfig {
			balances: endowed_accounts.iter()
				.chain(super::faucet_accounts().iter())
				.cloned()
				.map(|x| (x, ENDOWMENT))
				.collect()
		},
		pallet_indices: IndicesConfig {
			indices: vec![],
		},
		pallet_democracy: DemocracyConfig::default(),
		pallet_collective_Instance1: CouncilConfig::default(),
		pallet_collective_Instance2: TechnicalCommitteeConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		},
		pallet_sudo: SudoConfig {
			key: root_key.clone(),
		},
		bifrost_voucher: {
			if let Some(vouchers) = initialize_all_vouchers() {
				VoucherConfig { voucher: vouchers }
			} else {
				Default::default()
			}
		},
		orml_tokens: AssetsConfig {
			endowed_accounts: endowed_accounts
				.iter()
				.chain(super::faucet_accounts().iter())
				.flat_map(|x| {
					vec![
						(x.clone(), CurrencyId::Stable(TokenSymbol::AUSD), ENDOWMENT * 10_000),
						(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT),
						(x.clone(), CurrencyId::Token(TokenSymbol::ETH), ENDOWMENT),
						(x.clone(), CurrencyId::Token(TokenSymbol::KSM), ENDOWMENT),
					]
				})
				.collect(),
		},
		bifrost_minter_reward: MinterRewardConfig {
			wegiths: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 1 * 1),
				(CurrencyId::Token(TokenSymbol::ETH), 1 * 1),
				(CurrencyId::Token(TokenSymbol::KSM), 1 * 3),
			],
			reward_by_one_block: 5 * DOLLARS / 100,
			round_index: 1,
			storage_version: Default::default(),
		},
		bifrost_vtoken_mint: VtokenMintConfig {
			pools: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 1000 * DOLLARS),
				(CurrencyId::VToken(TokenSymbol::DOT), 1000 * DOLLARS),
				(CurrencyId::Token(TokenSymbol::ETH), 1000 * DOLLARS),
				(CurrencyId::VToken(TokenSymbol::ETH), 1000 * DOLLARS),
				(CurrencyId::Token(TokenSymbol::KSM), 1000 * DOLLARS),
				(CurrencyId::VToken(TokenSymbol::KSM), 1000 * DOLLARS),
			],
			staking_lock_period: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 28 * DAYS),
				(CurrencyId::Token(TokenSymbol::ETH), 14 * DAYS),
				(CurrencyId::Token(TokenSymbol::KSM), 7 * DAYS)
			],
			rate_of_interest_each_block: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 019_025_875_190), // 100000.0 * 0.148/(365*24*600)
				(CurrencyId::Token(TokenSymbol::ETH), 009_512_937_595), // 50000.0 * 0.082/(365*24*600)
				(CurrencyId::Token(TokenSymbol::KSM), 000_285_388_127) // 10000.0 * 0.15/(365*24*600)
			],
			yield_rate: vec![
				(CurrencyId::Token(TokenSymbol::DOT), Permill::from_perthousand(148)),// 14.8%
				(CurrencyId::Token(TokenSymbol::ETH), Permill::from_perthousand(82)), // 8.2%
				(CurrencyId::Token(TokenSymbol::KSM), Permill::from_perthousand(150)) // 15.0%
			],
		},
		parachain_info: ParachainInfoConfig { parachain_id: id },
		pallet_aura: AuraConfig {
			authorities: initial_authorities,
		},
		cumulus_pallet_aura_ext: Default::default(),
	}
}

fn development_config_genesis(id: ParaId) -> GenesisConfig {
	asgard_genesis(
		vec![get_from_seed::<AuraId>("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
		id,
	)
}

pub fn development_config(id: ParaId) -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		move || development_config_genesis(id),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		RelayExtensions {
			relay_chain: "westend-dev".into(),
			para_id: id.into(),
		},
	))
}

fn local_config_genesis(id: ParaId) -> GenesisConfig {
	asgard_genesis(
		vec![
			get_from_seed::<AuraId>("Alice"),
			get_from_seed::<AuraId>("Bob"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
		id,
	)
}

pub fn local_testnet_config(id: ParaId) -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		"Asgard Local Testnet",
		"asgard_local_testnet",
		ChainType::Local,
		move || local_config_genesis(id),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		RelayExtensions {
			relay_chain: "westend-local".into(),
			para_id: id.into(),
		},
	))
}

pub fn chainspec_config(id: ParaId) -> ChainSpec {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "ASG".into());
	properties.insert("tokenDecimals".into(), 12.into());

	ChainSpec::from_genesis(
		"Bifrost Asgard CC4",
		"asgard_testnet",
		ChainType::Custom("Asgard Testnet".into()),
		move || {
			asgard_config_genesis(id)
		},
		vec![
			"/ip4/150.109.71.108/tcp/30333/p2p/12D3KooWC8djq1tWHepURWiiQ1FAcPE7L1AfJvSpDb8TQAi1izKv".parse().expect("failed to parse multiaddress."),
			"/ip4/119.28.73.187/tcp/30333/p2p/12D3KooWGK87fM2pNQyLn23R1GkBvQCYqnSdKdMsrJGNRqT22wU8".parse().expect("failed to parse multiaddress."),
			"/ip4/150.109.194.40/tcp/30333/p2p/12D3KooWLt3w5tadCR5Fc7ZvjciLy7iKJ2ZHq6qp4UVmUUHyCJuX".parse().expect("failed to parse multiaddress."),
			"/ip4/124.156.223.229/tcp/30333/p2p/12D3KooWMduQkmRVzpwxJuN6MQT4ex1iP9YquzL4h5K9Ru8qMXtQ".parse().expect("failed to parse multiaddress."),
			"/ip4/124.156.217.80/tcp/30333/p2p/12D3KooWLAHZyqMa9TQ1fR7aDRRKfWt857yFMT3k2ckK9mhYT9qR".parse().expect("failed to parse multiaddress.")
		],
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties),
		RelayExtensions {
			relay_chain: "westend".into(),
			para_id: id.into(),
		},
	)
}

fn asgard_config_genesis(id: ParaId) -> GenesisConfig {
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

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = hex![
		// 5GjJNWYS6f2UQ9aiLexuB8qgjG8fRs2Ax4nHin1z1engpnNt
		"ce6072037670ca8e974fd571eae4f215a58d0bf823b998f619c3f87a911c3541"
	].into();

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	asgard_genesis(
		initial_authorities,
		root_key,
		Some(endowed_accounts),
		id,
	)
}

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

use std::collections::BTreeMap;

use asgard_runtime::{
	constants::{currency::DOLLARS, time::DAYS},
	AccountId, AuraId, Balance, BalancesConfig, BancorConfig, BlockNumber, CollatorSelectionConfig,
	CouncilConfig, DemocracyConfig, GenesisConfig, IndicesConfig, MinterRewardConfig,
	ParachainInfoConfig, SessionConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig,
	TokensConfig, VestingConfig, VoucherConfig, VtokenMintConfig, WASM_BINARY,
};
use cumulus_primitives_core::ParaId;
use hex_literal::hex;
use node_primitives::{CurrencyId, TokenSymbol};
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::{traits::Zero, Permill};

use super::TELEMETRY_URL;
use crate::chain_spec::{
	get_account_id_from_seed, get_from_seed, testnet_accounts, RelayExtensions,
};

const DEFAULT_PROTOCOL_ID: &str = "asgard";

/// Specialized `ChainSpec` for the asgard runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

/// Helper function to create asgard GenesisConfig for testing
pub fn asgard_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	id: ParaId,
	balances: Vec<(AccountId, Balance)>,
	vestings: Vec<(AccountId, BlockNumber, BlockNumber, Balance)>,
	vouchers: Vec<(AccountId, Balance)>,
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
		elections: Default::default(),
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
						acc.clone(),                          // account id
						acc.clone(),                          // validator id
						asgard_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		vesting: VestingConfig { vesting: vestings },
		voucher: VoucherConfig { voucher: vouchers },
		tokens: TokensConfig { balances: tokens },
		bancor: BancorConfig {
			bancor_pools: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 10_000 * DOLLARS),
				(CurrencyId::Token(TokenSymbol::KSM), 1_000_000 * DOLLARS),
			],
		},
		minter_reward: MinterRewardConfig {
			wegiths: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 1 * 1),
				(CurrencyId::Token(TokenSymbol::ETH), 1 * 1),
				(CurrencyId::Token(TokenSymbol::KSM), 1 * 3),
			],
			reward_by_one_block: 5 * DOLLARS / 100,
			round_index: 1,
			storage_version: Default::default(),
		},
		vtoken_mint: VtokenMintConfig {
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
				(CurrencyId::Token(TokenSymbol::KSM), 7 * DAYS),
			],
			rate_of_interest_each_block: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 019_025_875_190), // 100000.0 * 0.148/(365*24*600)
				(CurrencyId::Token(TokenSymbol::ETH), 009_512_937_595), // 50000.0 * 0.082/(365*24*600)
				(CurrencyId::Token(TokenSymbol::KSM), 000_285_388_127), // 10000.0 * 0.15/(365*24*600)
			],
			yield_rate: vec![
				(CurrencyId::Token(TokenSymbol::DOT), Permill::from_perthousand(148)), // 14.8%
				(CurrencyId::Token(TokenSymbol::ETH), Permill::from_perthousand(82)),  // 8.2%
				(CurrencyId::Token(TokenSymbol::KSM), Permill::from_perthousand(150)), // 15.0%
			],
		},
	}
}

#[allow(dead_code)]
fn initialize_all_vouchers() -> Vec<(AccountId, Balance)> {
	use std::collections::HashSet;

	let exe_dir = {
		let mut exe_dir = std::env::current_exe().unwrap();
		exe_dir.pop();

		exe_dir
	};

	let balances_configs: Vec<BalancesConfig> =
		super::config_from_json_files(exe_dir.join("res/genesis_config/balances")).unwrap();

	let vouchers: Vec<(node_primitives::AccountId, node_primitives::Balance)> = balances_configs
		.into_iter()
		.flat_map(|bc| bc.balances)
		.map(|v| (v.0.clone(), v.1))
		.into_iter()
		.collect();

	let set = vouchers.iter().map(|v| v.0.clone()).collect::<HashSet<_>>();
	let mut final_vouchers = Vec::new();
	for i in set.iter() {
		let mut sum = 0;
		for j in vouchers.iter() {
			if *i == j.0 {
				sum += j.1;
			}
		}
		final_vouchers.push((i.clone(), sum));
	}

	final_vouchers
}

fn development_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let balances = endowed_accounts
		.iter()
		.chain(super::faucet_accounts().iter())
		.cloned()
		.map(|x| (x, ENDOWMENT))
		.collect();
	let vestings = endowed_accounts
		.iter()
		.cloned()
		.map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT / 4))
		.collect();
	let vouchers = vec![];
	let tokens = endowed_accounts
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
		.collect();

	asgard_genesis(
		vec![(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_from_seed::<AuraId>("Alice"),
		)],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		id,
		balances,
		vestings,
		vouchers,
		tokens,
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
		RelayExtensions { relay_chain: "westend-dev".into(), para_id: id.into() },
	))
}

fn local_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts = testnet_accounts();
	let balances = endowed_accounts
		.iter()
		.chain(super::faucet_accounts().iter())
		.cloned()
		.map(|x| (x, ENDOWMENT))
		.collect();
	let vestings = endowed_accounts
		.iter()
		.cloned()
		.map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT / 4))
		.collect();
	let vouchers = vec![];
	let tokens = endowed_accounts
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
		.collect();

	asgard_genesis(
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<AuraId>("Alice"),
			),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), get_from_seed::<AuraId>("Bob")),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		id,
		balances,
		vestings,
		vouchers,
		tokens,
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
		RelayExtensions { relay_chain: "westend-local".into(), para_id: id.into() },
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
		move || asgard_config_genesis(id),
		vec![],
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties),
		RelayExtensions { relay_chain: "westend".into(), para_id: id.into() },
	)
}

fn asgard_config_genesis(id: ParaId) -> GenesisConfig {
	let invulnerables: Vec<(AccountId, AuraId)> = vec![
		(
			hex!["20b8de78cf83088dd5d8f1e05aeb7122635e5f00015e4cf03e961fe8cc7b9935"].into(),
			hex!["20b8de78cf83088dd5d8f1e05aeb7122635e5f00015e4cf03e961fe8cc7b9935"]
				.unchecked_into(),
		),
		(
			hex!["0c5192dccfcab3a676d74d3aab838f4d1e6b4f490cf15703424c382c6a72401d"].into(),
			hex!["0c5192dccfcab3a676d74d3aab838f4d1e6b4f490cf15703424c382c6a72401d"]
				.unchecked_into(),
		),
		(
			hex!["3c7e936535c17ff1ab4c72e4d8bf7672fd8488e5a30a1b3305c959ee7f794f28"].into(),
			hex!["3c7e936535c17ff1ab4c72e4d8bf7672fd8488e5a30a1b3305c959ee7f794f28"]
				.unchecked_into(),
		),
		(
			hex!["eee4ed9bb0a1a72aa966a1a21c403835b5edac59de296be19bd8b2ad31d03f3b"].into(),
			hex!["eee4ed9bb0a1a72aa966a1a21c403835b5edac59de296be19bd8b2ad31d03f3b"]
				.unchecked_into(),
		),
	];

	let root_key: AccountId = hex![
		// 5GjJNWYS6f2UQ9aiLexuB8qgjG8fRs2Ax4nHin1z1engpnNt
		"ce6072037670ca8e974fd571eae4f215a58d0bf823b998f619c3f87a911c3541"
	]
	.into();

	let exe_dir = {
		let mut exe_dir = std::env::current_exe().unwrap();
		exe_dir.pop();

		exe_dir
	};

	let balances_configs: Vec<BalancesConfig> =
		super::config_from_json_files(exe_dir.join("res/genesis_config/balances")).unwrap();

	let mut total_issuance: Balance = Zero::zero();
	let balances = balances_configs
		.into_iter()
		.flat_map(|bc| bc.balances)
		.chain(super::faucet_accounts().iter().map(|x| (x, ENDOWMENT)))
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
		super::config_from_json_files(exe_dir.join("res/genesis_config/vesting")).unwrap();
	let vouchers = initialize_all_vouchers();
	let tokens = vec![];

	asgard_genesis(
		invulnerables,
		root_key,
		id,
		balances,
		vesting_configs.into_iter().flat_map(|vc| vc.vesting).collect(),
		vouchers,
		tokens,
	)
}

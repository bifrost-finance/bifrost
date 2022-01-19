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

use asgard_runtime::{
	AccountId, AuraId, Balance, BalancesConfig, BancorConfig, BlockNumber, CouncilConfig,
	DefaultBlocksPerRound, DemocracyConfig, GenesisConfig, IndicesConfig, InflationInfo,
	MinterRewardConfig, ParachainInfoConfig, ParachainStakingConfig, PolkadotXcmConfig, Range,
	SS58Prefix, SalpConfig, SalpLiteConfig, SessionConfig, SudoConfig, SystemConfig,
	TechnicalCommitteeConfig, TokensConfig, VestingConfig, VtokenMintConfig, WASM_BINARY,
};
use bifrost_runtime_common::{constants::time::*, dollar};
use cumulus_primitives_core::ParaId;
use frame_benchmarking::{account, whitelisted_caller};
use hex_literal::hex;
use node_primitives::{CurrencyId, TokenInfo, TokenSymbol};
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};

use super::TELEMETRY_URL;
use crate::chain_spec::{get_account_id_from_seed, get_from_seed, RelayExtensions};

const DEFAULT_PROTOCOL_ID: &str = "asgard";

use sp_runtime::Perbill;

/// Specialized `ChainSpec` for the asgard runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

#[allow(non_snake_case)]
pub fn ENDOWMENT() -> u128 {
	10_000 * dollar(CurrencyId::Native(TokenSymbol::ASG))
}

pub const PARA_ID: u32 = 2121;

fn asgard_properties() -> Properties {
	let mut properties = sc_chain_spec::Properties::new();
	let mut token_symbol: Vec<String> = vec![];
	let mut token_decimals: Vec<u32> = vec![];
	[
		// native token
		CurrencyId::Native(TokenSymbol::ASG),
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

pub fn inflation_config() -> InflationInfo<Balance> {
	fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
		use parachain_staking::inflation::{perbill_annual_to_perbill_round, BLOCKS_PER_YEAR};
		perbill_annual_to_perbill_round(
			annual,
			// rounds per year
			BLOCKS_PER_YEAR / DefaultBlocksPerRound::get(),
		)
	}
	let annual = Range {
		min: Perbill::from_percent(4),
		ideal: Perbill::from_percent(5),
		max: Perbill::from_percent(5),
	};
	InflationInfo {
		// staking expectations
		expect: Range {
			min: 100_000 * dollar(CurrencyId::Native(TokenSymbol::ASG)),
			ideal: 200_000 * dollar(CurrencyId::Native(TokenSymbol::ASG)),
			max: 500_000 * dollar(CurrencyId::Native(TokenSymbol::ASG)),
		},
		// annual inflation
		annual,
		round: to_round_inflation(annual),
	}
}

/// Helper function to create asgard GenesisConfig for testing
pub fn asgard_genesis(
	candidates: Vec<(AccountId, AuraId, Balance)>,
	delegations: Vec<(AccountId, AccountId, Balance)>,
	root_key: AccountId,
	id: ParaId,
	balances: Vec<(AccountId, Balance)>,
	vestings: Vec<(AccountId, BlockNumber, BlockNumber, Balance)>,
	tokens: Vec<(AccountId, CurrencyId, Balance)>,
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
		parachain_system: Default::default(),
		session: SessionConfig {
			keys: candidates
				.iter()
				.cloned()
				.map(|(acc, aura, _)| {
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
		vesting: VestingConfig { vesting: vestings },
		tokens: TokensConfig { balances: tokens },
		bancor: BancorConfig {
			bancor_pools: vec![
				(
					CurrencyId::Token(TokenSymbol::DOT),
					10_000 * dollar(CurrencyId::Token(TokenSymbol::DOT)),
				),
				(
					CurrencyId::Token(TokenSymbol::KSM),
					1_000_000 * dollar(CurrencyId::Token(TokenSymbol::KSM)),
				),
			],
		},
		minter_reward: MinterRewardConfig {
			currency_weights: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 1 * 1),
				(CurrencyId::Token(TokenSymbol::ETH), 1 * 1),
				(CurrencyId::Token(TokenSymbol::KSM), 1 * 3),
			],
			reward_per_block: 5 * dollar(CurrencyId::Native(TokenSymbol::ASG)) / 100,
			cycle_index: 1,
		},
		vtoken_mint: VtokenMintConfig {
			pools: vec![
				(
					CurrencyId::Token(TokenSymbol::DOT),
					1000 * dollar(CurrencyId::Token(TokenSymbol::DOT)),
				),
				(
					CurrencyId::VToken(TokenSymbol::DOT),
					1000 * dollar(CurrencyId::VToken(TokenSymbol::DOT)),
				),
				(
					CurrencyId::Token(TokenSymbol::ETH),
					1000 * dollar(CurrencyId::Token(TokenSymbol::ETH)),
				),
				(
					CurrencyId::VToken(TokenSymbol::ETH),
					1000 * dollar(CurrencyId::VToken(TokenSymbol::ETH)),
				),
				(
					CurrencyId::Token(TokenSymbol::KSM),
					1000 * dollar(CurrencyId::Token(TokenSymbol::KSM)),
				),
				(
					CurrencyId::VToken(TokenSymbol::KSM),
					1000 * dollar(CurrencyId::VToken(TokenSymbol::KSM)),
				),
			],
			staking_lock_period: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 28 * DAYS),
				(CurrencyId::Token(TokenSymbol::ETH), 14 * DAYS),
				(CurrencyId::Token(TokenSymbol::KSM), 7 * DAYS),
			],
		},
		polkadot_xcm: PolkadotXcmConfig { safe_xcm_version: Some(2) },
		parachain_staking: ParachainStakingConfig {
			candidates: candidates
				.iter()
				.cloned()
				.map(|(account, _, bond)| (account, bond))
				.collect(),
			delegations,
			inflation_config: inflation_config(),
		},
		salp: SalpConfig { initial_multisig_account: Some(salp_multisig_key) },
		salp_lite: SalpLiteConfig { initial_multisig_account: Some(salp_lite_multisig_key_salp) },
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
		.map(|x| (x, ENDOWMENT()))
		.collect();
	let vestings = endowed_accounts
		.iter()
		.cloned()
		.map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT() / 4))
		.collect();
	let tokens = endowed_accounts
		.iter()
		.chain(faucet_accounts().iter())
		.flat_map(|x| {
			vec![
				(x.clone(), CurrencyId::Stable(TokenSymbol::KUSD), ENDOWMENT()),
				(x.clone(), CurrencyId::Token(TokenSymbol::KSM), ENDOWMENT()),
			]
		})
		.collect();
	let salp_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();
	let salp_lite_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();

	asgard_genesis(
		vec![(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_from_seed::<AuraId>("Alice"),
			ENDOWMENT(),
		)],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		id,
		balances,
		vestings,
		tokens,
		salp_multisig,
		salp_lite_multisig,
	)
}

pub fn development_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		move || development_config_genesis(PARA_ID.into()),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		Some(asgard_properties()),
		RelayExtensions { relay_chain: "westend-dev".into(), para_id: PARA_ID },
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
		.map(|x| (x, ENDOWMENT()))
		.collect();
	let vesting_configs: Vec<VestingConfig> = vec![];
	let tokens = endowed_accounts
		.iter()
		.chain(faucet_accounts().iter())
		.flat_map(|x| {
			vec![
				(x.clone(), CurrencyId::Native(TokenSymbol::ASG), ENDOWMENT()),
				(x.clone(), CurrencyId::Stable(TokenSymbol::KUSD), ENDOWMENT()),
				(x.clone(), CurrencyId::Token(TokenSymbol::KSM), ENDOWMENT()),
				(x.clone(), CurrencyId::VSToken(TokenSymbol::KSM), ENDOWMENT()),
				(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT()),
				(x.clone(), CurrencyId::VSToken(TokenSymbol::DOT), ENDOWMENT()),
				(x.clone(), CurrencyId::VSBond(TokenSymbol::ASG, 2121, 13, 20), ENDOWMENT()),
			]
		})
		.collect();

	let salp_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();
	let salp_lite_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();

	asgard_genesis(
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<AuraId>("Alice"),
				ENDOWMENT(),
			),
			(
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_from_seed::<AuraId>("Bob"),
				ENDOWMENT(),
			),
		],
		vec![],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		id,
		balances,
		vesting_configs.into_iter().flat_map(|vc| vc.vesting).collect(),
		tokens,
		salp_multisig,
		salp_lite_multisig,
	)
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
	Ok(ChainSpec::from_genesis(
		"Asgard Local Testnet",
		"asgard_local_testnet",
		ChainType::Local,
		move || local_config_genesis(PARA_ID.into()),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		Some(asgard_properties()),
		RelayExtensions { relay_chain: "westend-local".into(), para_id: PARA_ID },
	))
}

pub fn chainspec_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Bifrost Asgard CC4",
		"asgard_testnet",
		ChainType::Custom("Asgard Testnet".into()),
		move || asgard_config_genesis(PARA_ID.into()),
		vec![],
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		Some(DEFAULT_PROTOCOL_ID),
		Some(asgard_properties()),
		RelayExtensions { relay_chain: "westend".into(), para_id: PARA_ID },
	)
}

fn asgard_config_genesis(id: ParaId) -> GenesisConfig {
	let invulnerables: Vec<(AccountId, AuraId, Balance)> = vec![
		(
			hex!["16deb8ffe47267721605dbd3049c09e3015a70bb188e89d11bcb81cd0103642f"].into(),
			hex!["16deb8ffe47267721605dbd3049c09e3015a70bb188e89d11bcb81cd0103642f"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			hex!["72f66e5ae3711acf3d2d13d13a53d238000664d2416a0f6faa6ee6b33477180b"].into(),
			hex!["72f66e5ae3711acf3d2d13d13a53d238000664d2416a0f6faa6ee6b33477180b"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			hex!["d481ed743d3c54fbda92e0c364ae49f73b34d5c396295849783944cbb138ad29"].into(),
			hex!["d481ed743d3c54fbda92e0c364ae49f73b34d5c396295849783944cbb138ad29"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			hex!["5ca75e09f976da25ca776b748dd5b206d51db697f19bf71d0f93a21a4c983706"].into(),
			hex!["5ca75e09f976da25ca776b748dd5b206d51db697f19bf71d0f93a21a4c983706"]
				.unchecked_into(),
			ENDOWMENT(),
		),
	];

	let root_key: AccountId =
		hex!["30a74e1548ed3a26575ee1f1ea28e3c2d55cfdaeb539ae7aa06778ca7f75c935"].into();

	let balances = faucet_accounts()
		.into_iter()
		.map(|x| (x, ENDOWMENT()))
		.collect::<Vec<(AccountId, Balance)>>();

	let vesting_configs: Vec<VestingConfig> = vec![];
	let endowed_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
	];
	let tokens = endowed_accounts
		.iter()
		.chain(faucet_accounts().iter())
		.flat_map(|x| {
			vec![
				(x.clone(), CurrencyId::Native(TokenSymbol::ASG), ENDOWMENT()),
				(x.clone(), CurrencyId::Stable(TokenSymbol::KUSD), ENDOWMENT()),
				(x.clone(), CurrencyId::Token(TokenSymbol::KSM), ENDOWMENT()),
				(x.clone(), CurrencyId::VSToken(TokenSymbol::KSM), ENDOWMENT()),
				(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT()),
				(x.clone(), CurrencyId::VSToken(TokenSymbol::DOT), ENDOWMENT()),
				(x.clone(), CurrencyId::VSBond(TokenSymbol::ASG, 2121, 13, 20), ENDOWMENT()),
			]
		})
		.collect();
	let salp_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();
	let salp_lite_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();

	asgard_genesis(
		invulnerables,
		vec![],
		root_key,
		id,
		balances,
		vesting_configs.into_iter().flat_map(|vc| vc.vesting).collect(),
		tokens,
		salp_multisig,
		salp_lite_multisig,
	)
}

pub fn faucet_accounts() -> Vec<AccountId> {
	vec![
		hex!["30a74e1548ed3a26575ee1f1ea28e3c2d55cfdaeb539ae7aa06778ca7f75c935"].into(), /* asgard sudo account */
		hex!["16deb8ffe47267721605dbd3049c09e3015a70bb188e89d11bcb81cd0103642f"].into(),
		hex!["72f66e5ae3711acf3d2d13d13a53d238000664d2416a0f6faa6ee6b33477180b"].into(),
		hex!["d481ed743d3c54fbda92e0c364ae49f73b34d5c396295849783944cbb138ad29"].into(),
		hex!["5ca75e09f976da25ca776b748dd5b206d51db697f19bf71d0f93a21a4c983706"].into(),
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

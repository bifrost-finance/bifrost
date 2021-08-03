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
use dev_runtime::{
	constants::{currency::DOLLARS, time::DAYS},
	AccountId, AuraConfig, AuraId, Balance, BalancesConfig, BancorConfig, BlockNumber,
	CouncilConfig, DemocracyConfig, GenesisConfig, GrandpaConfig, IndicesConfig,
	MinterRewardConfig, ParachainInfoConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig,
	TokensConfig, VestingConfig, VtokenMintConfig, WASM_BINARY,
};
use hex_literal::hex;
use node_primitives::{CurrencyId, TokenSymbol};
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::sr25519;
use sp_finality_grandpa::AuthorityId as GrandpaId;

use super::TELEMETRY_URL;
use crate::chain_spec::{get_account_id_from_seed, get_from_seed, RelayExtensions};

const DEFAULT_PROTOCOL_ID: &str = "asgard-dev";

/// Specialized `ChainSpec` for the asgard runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

/// Helper function to create asgard GenesisConfig for testing
pub fn dev_genesis(
	invulnerables: Vec<(AccountId, AuraId, GrandpaId)>,
	root_key: AccountId,
	id: ParaId,
	balances: Vec<(AccountId, Balance)>,
	vestings: Vec<(AccountId, BlockNumber, BlockNumber, Balance)>,
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
		aura: AuraConfig { authorities: invulnerables.iter().map(|x| (x.1.clone())).collect() },
		grandpa: GrandpaConfig {
			authorities: invulnerables.iter().map(|x| (x.2.clone(), 1)).collect(),
		},
		parachain_system: Default::default(),
		vesting: VestingConfig { vesting: vestings },
		tokens: TokensConfig { balances: tokens },
		bancor: BancorConfig {
			bancor_pools: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 10_000 * DOLLARS),
				(CurrencyId::Token(TokenSymbol::KSM), 1_000_000 * DOLLARS),
			],
		},
		minter_reward: MinterRewardConfig {
			currency_weights: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 1 * 1),
				(CurrencyId::Token(TokenSymbol::ETH), 1 * 1),
				(CurrencyId::Token(TokenSymbol::KSM), 1 * 3),
			],
			reward_per_block: 5 * DOLLARS / 100,
			cycle_index: 1,
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
		},
	}
}

fn development_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
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
				(x.clone(), CurrencyId::Stable(TokenSymbol::AUSD), ENDOWMENT * 10_000),
				(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT),
				(x.clone(), CurrencyId::Token(TokenSymbol::ETH), ENDOWMENT),
				(x.clone(), CurrencyId::Token(TokenSymbol::KSM), ENDOWMENT),
			]
		})
		.collect();

	dev_genesis(
		vec![(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_from_seed::<AuraId>("Alice"),
			get_from_seed::<GrandpaId>("Alice"),
		)],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		id,
		balances,
		vestings,
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

pub fn chainspec_config(id: ParaId) -> ChainSpec {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "ASG".into());
	properties.insert("tokenDecimals".into(), 12.into());

	ChainSpec::from_genesis(
		"Bifrost Dev",
		"dev_testnet",
		ChainType::Custom("Dev Testnet".into()),
		move || development_config_genesis(id),
		vec![],
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties),
		RelayExtensions { relay_chain: "westend".into(), para_id: id.into() },
	)
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

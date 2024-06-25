// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

use crate::chain_spec::{get_account_id_from_seed, get_from_seed, RelayExtensions};
use bifrost_polkadot_runtime::{
	constants::currency::DOLLARS, AccountId, Balance, BlockNumber, DefaultBlocksPerRound,
	InflationInfo, Range, RuntimeGenesisConfig, SS58Prefix,
};
use bifrost_primitives::{CurrencyId, CurrencyId::*, TokenInfo, TokenSymbol, DOT_TOKEN_ID};
use bifrost_runtime_common::AuraId;
use cumulus_primitives_core::ParaId;
use frame_benchmarking::{account, whitelisted_caller};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::Perbill;

const DEFAULT_PROTOCOL_ID: &str = "bifrost_polkadot";

/// Specialized `ChainSpec` for the bifrost-polkadot runtime.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, RelayExtensions>;

#[allow(non_snake_case)]
pub fn ENDOWMENT() -> u128 {
	1_000_000 * DOLLARS
}

pub const PARA_ID: u32 = 2030;

pub fn inflation_config() -> InflationInfo<Balance> {
	fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
		use bifrost_parachain_staking::inflation::{
			perbill_annual_to_perbill_round, BLOCKS_PER_YEAR,
		};
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
		expect: Range { min: 100_000 * DOLLARS, ideal: 200_000 * DOLLARS, max: 500_000 * DOLLARS },
		// annual inflation
		annual,
		round: to_round_inflation(annual),
	}
}

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
	candidates: Vec<(AccountId, AuraId, Balance)>,
	delegations: Vec<(AccountId, AccountId, Balance)>,
	balances: Vec<(AccountId, Balance)>,
	vestings: Vec<(AccountId, BlockNumber, BlockNumber, Balance)>,
	id: ParaId,
	council_membership: Vec<AccountId>,
	technical_committee_membership: Vec<AccountId>,
	salp_multisig_key: AccountId,
	asset_registry: (
		Vec<(CurrencyId, Balance, Option<(String, String, u8)>)>,
		Vec<CurrencyId>,
		Vec<(CurrencyId, u32, u32, u32)>,
	),
	oracle_membership: Vec<AccountId>,
) -> serde_json::Value {
	serde_json::json!( {
		"balances": {
			"balances": balances
		},
		"councilMembership": {
			"members": council_membership
		},
		"oracleMembership": {
			"members": oracle_membership
		},
		"technicalCommittee": {
			"members": technical_committee_membership
		},
		"parachainInfo": {
			"parachainId": id
		},
		"session": {
			"keys": candidates
				.iter()
				.cloned()
				.map(|(acc, aura,_)| {
					(
						acc.clone(),                                    // account id
						acc,                                            // validator id
						bifrost_polkadot_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect::<Vec<_>>(),
		},
		"vesting": {
			"vesting": vestings
		},
		"assetRegistry": {
			"currency": asset_registry.0,
			"vcurrency": asset_registry.1,
			"vsbond": asset_registry.2
		},
		"polkadotXcm": {
			"safeXcmVersion": 3
		},
		"salp": { "initialMultisigAccount": Some(salp_multisig_key) },
		"parachainStaking": {
			"candidates": candidates
				.iter()
				.cloned()
				.map(|(account, _, bond)| (account, bond))
				.collect::<Vec<_>>(),
			"delegations": delegations,
			"inflationConfig": inflation_config(),
		},
	})
}

pub fn local_testnet_config() -> ChainSpec {
	let endowed_accounts = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		whitelisted_caller(), // Benchmarking whitelist_account
		account("bechmarking_account_1", 0, 0),
	];
	let balances = endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT())).collect();
	let council_membership = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let technical_committee_membership = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let oracle_membership = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let salp_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();
	let currency = vec![
		(Native(TokenSymbol::BNC), DOLLARS / 100, None),
		(
			Token2(DOT_TOKEN_ID),
			DOLLARS / 1000_000,
			Some((String::from("Polkadot DOT"), String::from("DOT"), 10u8)),
		),
	];
	let vcurrency = vec![VSToken2(DOT_TOKEN_ID), VToken(TokenSymbol::BNC), VToken2(DOT_TOKEN_ID)];

	ChainSpec::builder(
		bifrost_polkadot_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		RelayExtensions { relay_chain: "polkadot-local".into(), para_id: PARA_ID },
	)
	.with_name("Bifrost Polkadot Local Testnet")
	.with_id("bifrost_polkadot_local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_patch(bifrost_polkadot_genesis(
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
		balances,
		vec![],
		PARA_ID.into(),
		council_membership,
		technical_committee_membership,
		salp_multisig,
		(currency, vcurrency, vec![]),
		oracle_membership,
	))
	.with_properties(bifrost_polkadot_properties())
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build()
}

pub fn paseo_config() -> ChainSpec {
	let candidates: Vec<(AccountId, AuraId, Balance)> = vec![
		(
			// e2s2dTSWe9kHebF2FCbPGbXftDT7fY5AMDfib3j86zSi3v7
			hex!["66204aeda74f07f77a4b6945681296763706f98d0f8aebb1b9ccdf6e9b7ac13f"].into(),
			hex!["66204aeda74f07f77a4b6945681296763706f98d0f8aebb1b9ccdf6e9b7ac13f"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			// fFjUFbokagaDRQUDzVhDcMZQaDwQvvha74RMZnyoSWNpiBQ
			hex!["9c2d45edb30d4bf0c285d6809e28c55e871f10578c5a3ea62da152d03761d266"].into(),
			hex!["9c2d45edb30d4bf0c285d6809e28c55e871f10578c5a3ea62da152d03761d266"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			// fBAbVJAsbWsKTedTVYGrBB3Usm6Vx635z1N9PX2tZ2boT37
			hex!["98b19fa5a3e98f693b7440de07b4744834ff0072cb704f1c6e33791953ac4924"].into(),
			hex!["98b19fa5a3e98f693b7440de07b4744834ff0072cb704f1c6e33791953ac4924"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			// c9eHvgbxTFzijvY3AnAKiRTHhi2hzS5SLCPzCkb4jP79MLu
			hex!["12d3ab675d6503279133898efe246a63fdc8be685cc3f7bce079aac064108a7a"].into(),
			hex!["12d3ab675d6503279133898efe246a63fdc8be685cc3f7bce079aac064108a7a"]
				.unchecked_into(),
			ENDOWMENT(),
		),
	];

	let endowed_accounts: Vec<AccountId> = vec![
		// dDWnEWnx3GUgfugXh9mZtgj4CvJdmd8naYkWYCZGxjfb1Cz
		hex!["420398e0150cd9d417fb8fd4027b75bd42717262e6eac97c55f2f8f84e8ffb3f"].into(),
		// e2s2dTSWe9kHebF2FCbPGbXftDT7fY5AMDfib3j86zSi3v7
		hex!["66204aeda74f07f77a4b6945681296763706f98d0f8aebb1b9ccdf6e9b7ac13f"].into(),
		// fFjUFbokagaDRQUDzVhDcMZQaDwQvvha74RMZnyoSWNpiBQ
		hex!["9c2d45edb30d4bf0c285d6809e28c55e871f10578c5a3ea62da152d03761d266"].into(),
		// fBAbVJAsbWsKTedTVYGrBB3Usm6Vx635z1N9PX2tZ2boT37
		hex!["98b19fa5a3e98f693b7440de07b4744834ff0072cb704f1c6e33791953ac4924"].into(),
		// c9eHvgbxTFzijvY3AnAKiRTHhi2hzS5SLCPzCkb4jP79MLu
		hex!["12d3ab675d6503279133898efe246a63fdc8be685cc3f7bce079aac064108a7a"].into(),
	];
	let balances = endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT())).collect();

	let salp_multisig: AccountId =
		hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();

	let council_membership = vec![
		// dDWnEWnx3GUgfugXh9mZtgj4CvJdmd8naYkWYCZGxjfb1Cz
		hex!["420398e0150cd9d417fb8fd4027b75bd42717262e6eac97c55f2f8f84e8ffb3f"].into(),
	];
	let technical_committee_membership = vec![
		// dDWnEWnx3GUgfugXh9mZtgj4CvJdmd8naYkWYCZGxjfb1Cz
		hex!["420398e0150cd9d417fb8fd4027b75bd42717262e6eac97c55f2f8f84e8ffb3f"].into(),
	];
	let oracle_membership = vec![
		// dDWnEWnx3GUgfugXh9mZtgj4CvJdmd8naYkWYCZGxjfb1Cz
		hex!["420398e0150cd9d417fb8fd4027b75bd42717262e6eac97c55f2f8f84e8ffb3f"].into(),
	];

	ChainSpec::builder(
		bifrost_polkadot_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		RelayExtensions { relay_chain: "paseo".into(), para_id: PARA_ID },
	)
	.with_name("Bifrost Paseo")
	.with_id("bifrost_paseo")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_patch(bifrost_polkadot_genesis(
		candidates,
		vec![],
		balances,
		vec![],
		PARA_ID.into(),
		council_membership,
		technical_committee_membership,
		salp_multisig,
		(vec![], vec![], vec![]),
		oracle_membership,
	))
	.with_properties(bifrost_polkadot_properties())
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build()
}
pub fn chainspec_config() -> ChainSpec {
	let candidates: Vec<(AccountId, AuraId, Balance)> = vec![
		(
			// dpEZwz5nHxEjQXcm3sjy6NTz83EGcBRXMBSyuuWSguiVGJB
			hex!["5c7e9ccd1045cac7f8c5c77a79c87f44019d1dda4f5032713bda89c5d73cb36b"].into(),
			hex!["5c7e9ccd1045cac7f8c5c77a79c87f44019d1dda4f5032713bda89c5d73cb36b"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			// duNwrtscWpfuTzRkjtt431kUj1gsfwbPi1bzdQL4cmk9QAa
			hex!["606b0aad375ae1715fbe6a07315136a8e9c1c84a91230f6a0c296c2953581335"].into(),
			hex!["606b0aad375ae1715fbe6a07315136a8e9c1c84a91230f6a0c296c2953581335"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			// gPQG97HPe54fJpLoFePwm3fxdJaU2VV71hYbqd4RJcNeQfe
			hex!["ce42cea2dd0d4ac87ccdd5f0f2e1010955467f5a37587cf6af8ee2b4ba781034"].into(),
			hex!["ce42cea2dd0d4ac87ccdd5f0f2e1010955467f5a37587cf6af8ee2b4ba781034"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			// frYfsZhdVuG6Ap6AyJQLSHVqtKmUyqxo6ohnrmGTDk2neXK
			hex!["b6ba81e73bd39203e006fc99cc1e41976745de2ea2007bf62ed7c9a48ccc5b1d"].into(),
			hex!["b6ba81e73bd39203e006fc99cc1e41976745de2ea2007bf62ed7c9a48ccc5b1d"]
				.unchecked_into(),
			ENDOWMENT(),
		),
	];

	let salp_multisig: AccountId =
		hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();

	ChainSpec::builder(
		bifrost_polkadot_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		RelayExtensions { relay_chain: "polkadot".into(), para_id: PARA_ID },
	)
	.with_name("Bifrost Polkadot")
	.with_id("bifrost_polkadot")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_patch(bifrost_polkadot_genesis(
		candidates,
		vec![],
		vec![],
		vec![],
		PARA_ID.into(),
		vec![],
		vec![],
		salp_multisig,
		(vec![], vec![], vec![]),
		vec![],
	))
	.with_properties(bifrost_polkadot_properties())
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build()
}

pub fn testnet_config() -> ChainSpec {
	let candidates: Vec<(AccountId, AuraId, Balance)> = vec![
		(
			// cxXb4sZAnwMRchQVRoMUkfFGF52Li6YqZRcuQd8XnaEQXKC
			hex!["3695a262bb68cf0c23e1f849bf036993ae9ceb7b6d1dd300cc0d6777ec8de520"].into(),
			hex!["3695a262bb68cf0c23e1f849bf036993ae9ceb7b6d1dd300cc0d6777ec8de520"]
				.unchecked_into(),
			ENDOWMENT() / 4,
		),
		(
			// cjQkMaTsHJm27cEcAwN1ghF6UpwmHLyq5pY12qxiMHDNwKv
			hex!["2c946b45e851b62582b6a2b8b304e3cda8392c8ec6da550b4426b7c60833956c"].into(),
			hex!["2c946b45e851b62582b6a2b8b304e3cda8392c8ec6da550b4426b7c60833956c"]
				.unchecked_into(),
			ENDOWMENT() / 4,
		),
		(
			// bjGo3QQWnf2HpBTrRC4YRDYqyeJmzsZgxr8mvbtLfkqXvYM
			hex!["003d693ed5403785569498d899593bb74119bacca39d226dd37853e21190ca65"].into(),
			hex!["003d693ed5403785569498d899593bb74119bacca39d226dd37853e21190ca65"]
				.unchecked_into(),
			ENDOWMENT() / 4,
		),
		(
			// eBghkPPpcM5aiQsN7jPFG2dGtiaJb3PazebaiDyDXhT6aw1
			hex!["6cdabe150eac14ba3700ce14fda9d5d595857690f40031cda41bcf775004d426"].into(),
			hex!["6cdabe150eac14ba3700ce14fda9d5d595857690f40031cda41bcf775004d426"]
				.unchecked_into(),
			ENDOWMENT() / 4,
		),
	];

	let endowed_accounts: Vec<AccountId> = vec![
		// gfF1Z4KNG7735MgtDQkbJTw7Z3P2W9p7Qquf3eqiy54zrt9
		hex!["da57971eb1a094247cc83f5b3775058bf82df535f214fecac2d1840df646252c"].into(),
		// cxXb4sZAnwMRchQVRoMUkfFGF52Li6YqZRcuQd8XnaEQXKC
		hex!["3695a262bb68cf0c23e1f849bf036993ae9ceb7b6d1dd300cc0d6777ec8de520"].into(),
		// cjQkMaTsHJm27cEcAwN1ghF6UpwmHLyq5pY12qxiMHDNwKv
		hex!["2c946b45e851b62582b6a2b8b304e3cda8392c8ec6da550b4426b7c60833956c"].into(),
		// bjGo3QQWnf2HpBTrRC4YRDYqyeJmzsZgxr8mvbtLfkqXvYM
		hex!["003d693ed5403785569498d899593bb74119bacca39d226dd37853e21190ca65"].into(),
		// eBghkPPpcM5aiQsN7jPFG2dGtiaJb3PazebaiDyDXhT6aw1
		hex!["6cdabe150eac14ba3700ce14fda9d5d595857690f40031cda41bcf775004d426"].into(),
	];
	let balances = endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT() * 100)).collect();

	let salp_multisig: AccountId =
		hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();

	ChainSpec::builder(
		bifrost_polkadot_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		RelayExtensions { relay_chain: "polkadot".into(), para_id: PARA_ID },
	)
	.with_name("Bifrost Polkadot Testnet")
	.with_id("bifrost_polkadot_testnet")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_patch(bifrost_polkadot_genesis(
		candidates,
		vec![],
		balances,
		vec![],
		PARA_ID.into(),
		vec![],
		vec![],
		salp_multisig,
		(vec![], vec![], vec![]),
		vec![],
	))
	.with_properties(bifrost_polkadot_properties())
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build()
}

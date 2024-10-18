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
use bifrost_kusama_runtime::{
	constants::currency::DOLLARS, AccountId, Balance, BalancesConfig, BlockNumber, InflationInfo,
	Range, SS58Prefix, VestingConfig,
};
use bifrost_primitives::{
	BifrostKusamaChainId, CurrencyId, CurrencyId::*, TokenInfo, TokenSymbol::*,
};
use bifrost_runtime_common::{constants::time::HOURS, AuraId};
use cumulus_primitives_core::ParaId;
use frame_benchmarking::{account, whitelisted_caller};
use hex_literal::hex;
use sc_chain_spec::Properties;
use sc_service::ChainType;
use serde::de::DeserializeOwned;
use serde_json as json;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::{traits::Zero, Perbill, Percent};
use std::{
	collections::BTreeMap,
	fs::{read_dir, File},
	path::PathBuf,
};

const DEFAULT_PROTOCOL_ID: &str = "bifrost";

/// Specialized `ChainSpec` for the bifrost runtime.
pub type ChainSpec = sc_service::GenericChainSpec<RelayExtensions>;

#[allow(non_snake_case)]
pub fn ENDOWMENT() -> u128 {
	1_000_000 * DOLLARS
}

const COLLATOR_COMMISSION: Perbill = Perbill::from_percent(10);
const PARACHAIN_BOND_RESERVE_PERCENT: Percent = Percent::from_percent(0);
const BLOCKS_PER_ROUND: u32 = 2 * HOURS;

pub fn inflation_config() -> InflationInfo<Balance> {
	fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
		use bifrost_parachain_staking::inflation::{
			perbill_annual_to_perbill_round, BLOCKS_PER_YEAR,
		};
		perbill_annual_to_perbill_round(
			annual,
			// rounds per year
			BLOCKS_PER_YEAR / BLOCKS_PER_ROUND,
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

fn bifrost_kusama_properties() -> Properties {
	let mut properties = Properties::new();
	let mut token_symbol: Vec<String> = vec![];
	let mut token_decimals: Vec<u32> = vec![];
	[
		// native token
		Native(BNC),
		// stable token
		Stable(KUSD),
		// token
		Token(DOT),
		Token(KSM),
		Token(KAR),
		Token(ZLK),
		Token(PHA),
		Token(RMRK),
		Token(MOVR),
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

pub fn bifrost_genesis(
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
				.map(|(acc, aura, _)| {
					(
						acc.clone(),                                  // account id
						acc,                                          // validator id
						bifrost_kusama_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect::<Vec<_>>(),
		},
		"polkadotXcm": {
			"safeXcmVersion": 3
		},
		"vesting": {
			"vesting": vestings
		},
		"assetRegistry": {
			"currency": asset_registry.0,
			"vcurrency": asset_registry.1,
			"vsbond": asset_registry.2
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
			"collatorCommission": COLLATOR_COMMISSION,
			"parachainBondReservePercent": PARACHAIN_BOND_RESERVE_PERCENT,
			"blocksPerRound": BLOCKS_PER_ROUND,
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
		// dKwyFv7RL79j1pAukZnZtAmxwG6a3USBmjZyFCLRSbghdiV
		hex!["46ebddef8cd9bb167dc30878d7113b7e168e6f0646beffd77d69d39bad76b47a"].into(),
		// eCSrvbA5gGNQr7VZ48fkCX5vkt1H16F8Np9g2hYssRXHZJF
		hex!["6d6f646c62662f7374616b650000000000000000000000000000000000000000"].into(),
	];
	let balances = endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT())).collect();
	let vestings = endowed_accounts
		.iter()
		.cloned()
		.map(|x| (x, 0u32, 100u32, ENDOWMENT() / 4))
		.collect();

	let council_membership = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let technical_committee_membership = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let oracle_membership = vec![get_account_id_from_seed::<sr25519::Public>("Alice")];
	let salp_multisig: AccountId =
		hex!["49daa32c7287890f38b7e1a8cd2961723d36d20baa0bf3b82e0c4bdda93b1c0a"].into();

	// Token
	let currency = vec![
		(Native(BNC), DOLLARS / 100, None),
		(Stable(KUSD), DOLLARS / 10_000, None),
		(Token(KSM), DOLLARS / 10_000, None),
		(Token(ZLK), DOLLARS / 1000_000, None),
		(Token(MOVR), DOLLARS / 1000_000, None),
	];
	let vcurrency = vec![VToken(BNC), VToken(KSM), VToken(MOVR)];

	// vsBond
	let vsbond = vec![];

	ChainSpec::builder(
		bifrost_kusama_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		RelayExtensions {
			relay_chain: "kusama-local".into(),
			para_id: BifrostKusamaChainId::get(),
			evm_since: 1,
		},
	)
	.with_name("Bifrost Local Testnet")
	.with_id("bifrost_local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_patch(bifrost_genesis(
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<AuraId>("Alice"),
				ENDOWMENT() / 4,
			),
			(
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_from_seed::<AuraId>("Bob"),
				ENDOWMENT() / 4,
			),
		],
		vec![],
		balances,
		vestings,
		BifrostKusamaChainId::get().into(),
		council_membership,
		technical_committee_membership,
		salp_multisig,
		(currency, vcurrency, vsbond),
		oracle_membership,
	))
	.with_properties(bifrost_kusama_properties())
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build()
}

pub fn chainspec_config() -> ChainSpec {
	let invulnerables: Vec<(AccountId, AuraId, Balance)> = vec![
		(
			// eunwjK45qDugPXhnjxGUcMbifgdtgefzoW7PgMMpr39AXwh
			hex!["8cf80f0bafcd0a3d80ca61cb688e4400e275b39d3411b4299b47e712e9dab809"].into(),
			hex!["8cf80f0bafcd0a3d80ca61cb688e4400e275b39d3411b4299b47e712e9dab809"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			// dBkoWVdQCccH1xNAeR1Y4vrETt3a4j4iU8Ct2ewY1FUjasL
			hex!["40ac4effe39181731a8feb8a8ee0780e177bdd0d752b09c8fd71047e67189022"].into(),
			hex!["40ac4effe39181731a8feb8a8ee0780e177bdd0d752b09c8fd71047e67189022"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			// dwrEwfj2RFU4DS6EiTCfmxMpQ1sAsaHykftzwoptFe4a8aH
			hex!["624d6a004c72a1abcf93131e185515ebe1410e43a301fe1f25d20d8da345376e"].into(),
			hex!["624d6a004c72a1abcf93131e185515ebe1410e43a301fe1f25d20d8da345376e"]
				.unchecked_into(),
			ENDOWMENT(),
		),
		(
			// fAjW6bwT4GKgW88sjZfNLRr5hWyMM9T9ZwqHYkFiSxw4Yhp
			hex!["985d2738e512909c81289e6055e60a6824818964535ecfbf10e4d69017084756"].into(),
			hex!["985d2738e512909c81289e6055e60a6824818964535ecfbf10e4d69017084756"]
				.unchecked_into(),
			ENDOWMENT(),
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

	assert_eq!(total_issuance, 32_000_000 * DOLLARS, "total issuance must be equal to 320 million");

	let vesting_configs: Vec<VestingConfig> =
		config_from_json_files(exe_dir.join("res/genesis_config/vesting")).unwrap();

	let salp_multisig: AccountId =
		hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();

	ChainSpec::builder(
		bifrost_kusama_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		RelayExtensions {
			relay_chain: "kusama".into(),
			para_id: BifrostKusamaChainId::get(),
			evm_since: 1,
		},
	)
	.with_name("Bifrost")
	.with_id("bifrost")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_patch(bifrost_genesis(
		invulnerables,
		vec![],
		balances,
		vesting_configs.into_iter().flat_map(|vc| vc.vesting).collect(),
		BifrostKusamaChainId::get().into(),
		vec![], // council membership
		vec![], // technical committee membership
		salp_multisig,
		(vec![], vec![], vec![]),
		vec![],
	))
	.with_properties(bifrost_kusama_properties())
	.with_protocol_id(DEFAULT_PROTOCOL_ID)
	.build()
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

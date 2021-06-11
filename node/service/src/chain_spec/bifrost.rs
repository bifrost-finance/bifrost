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
use bifrost_runtime::{
	AccountId, AuraId, Balance, BlockNumber,
	BalancesConfig, CollatorSelectionConfig, GenesisConfig, IndicesConfig,
	SessionConfig, SudoConfig, SystemConfig, ParachainInfoConfig, VestingConfig, WASM_BINARY,
};
use super::TELEMETRY_URL;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::traits::Zero;

use crate::chain_spec::{RelayExtensions, get_account_id_from_seed, get_from_seed};
use bifrost_runtime::constants::currency::DOLLARS;

const DEFAULT_PROTOCOL_ID: &str = "bifrost";

/// Specialized `ChainSpec` for the bifrost runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

pub fn bifrost_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	root_key: AccountId,
	balances: Vec<(AccountId, Balance)>,
	vestings: Vec<(AccountId, BlockNumber, BlockNumber, Balance)>,
	id: ParaId,
) -> GenesisConfig {
	GenesisConfig {
		frame_system: SystemConfig {
			code: WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: BalancesConfig {
			balances
		},
		pallet_indices: IndicesConfig {
			indices: vec![],
		},
		pallet_sudo: SudoConfig {
			key: root_key.clone(),
		},
		parachain_info: ParachainInfoConfig { parachain_id: id },
		pallet_collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: Zero::zero(),
			..Default::default()
		},
		pallet_session: SessionConfig {
			keys: invulnerables.iter().cloned().map(|(acc, aura)| (
				acc.clone(), // account id
				acc.clone(), // validator id
				bifrost_runtime::SessionKeys { aura }, // session keys
			)).collect()
		},
		pallet_aura: Default::default(),
		cumulus_pallet_aura_ext: Default::default(),
		cumulus_pallet_parachain_system: Default::default(),
		pallet_vesting:  VestingConfig {
			vesting: vestings
		},
	}
}

fn development_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts: Vec<AccountId> = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice")
	];
	const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

	bifrost_genesis(
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<AuraId>("Alice"),
			),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
		endowed_accounts.iter().cloned().map(|x| (x.clone(), 0u32, 100u32, ENDOWMENT/4)).collect(),
		id,
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
		RelayExtensions {
			relay_chain: "kusama-dev".into(),
			para_id: id.into(),
		},
	))
}

fn local_config_genesis(id: ParaId) -> GenesisConfig {
	let endowed_accounts: Vec<AccountId> = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
	];
	const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

	bifrost_genesis(
		vec![
			(
				 get_account_id_from_seed::<sr25519::Public>("Alice"),
				 get_from_seed::<AuraId>("Alice"),
			 ),
			 (
				 get_account_id_from_seed::<sr25519::Public>("Bob"),
				 get_from_seed::<AuraId>("Bob"),
			 ),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		endowed_accounts.iter().cloned().map(|x| (x, ENDOWMENT)).collect(),
		endowed_accounts.iter().cloned().map(|x| (x, 0u32, 100u32, ENDOWMENT/4)).collect(),
		id,
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
		RelayExtensions {
			relay_chain: "kusama-local".into(),
			para_id: id.into(),
		},
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
		move || {
			bifrost_config_genesis(id)
		},
		vec![],
		TelemetryEndpoints::new(vec![(TELEMETRY_URL.into(), 0)]).ok(),
		Some(DEFAULT_PROTOCOL_ID),
		Some(properties),
		RelayExtensions {
			relay_chain: "kusama".into(),
			para_id: id.into(),
		},
	)
}

fn bifrost_config_genesis(id: ParaId) -> GenesisConfig {
	let invulnerables: Vec<(AccountId, AuraId)> = vec![
		(
			// eunwjK45qDugPXhnjxGUcMbifgdtgefzoW7PgMMpr39AXwh
		 	hex!["8cf80f0bafcd0a3d80ca61cb688e4400e275b39d3411b4299b47e712e9dab809"].into(),
		 	hex!["8cf80f0bafcd0a3d80ca61cb688e4400e275b39d3411b4299b47e712e9dab809"].unchecked_into(),
		),
		(
			// dBkoWVdQCccH1xNAeR1Y4vrETt3a4j4iU8Ct2ewY1FUjasL
			hex!["40ac4effe39181731a8feb8a8ee0780e177bdd0d752b09c8fd71047e67189022"].into(),
			hex!["40ac4effe39181731a8feb8a8ee0780e177bdd0d752b09c8fd71047e67189022"].unchecked_into(),
		),
		(
			// dwrEwfj2RFU4DS6EiTCfmxMpQ1sAsaHykftzwoptFe4a8aH
			hex!["624d6a004c72a1abcf93131e185515ebe1410e43a301fe1f25d20d8da345376e"].into(),
			hex!["624d6a004c72a1abcf93131e185515ebe1410e43a301fe1f25d20d8da345376e"].unchecked_into(),
		),
		(
			// fAjW6bwT4GKgW88sjZfNLRr5hWyMM9T9ZwqHYkFiSxw4Yhp
			hex!["985d2738e512909c81289e6055e60a6824818964535ecfbf10e4d69017084756"].into(),
			hex!["985d2738e512909c81289e6055e60a6824818964535ecfbf10e4d69017084756"].unchecked_into(),
		),
	 ];

	let root_key: AccountId = hex![
		// cjAZA391BNi2S1Je7PNGHiX4UoJh3SbknQSDQ7qh3g4Aa9H
		"2c64a40ec236d0a0823065791946f6254c4577c6110f512614bd6ece1a3fa22b"
	].into();

	let exe_dir = {
		let mut exe_dir = std::env::current_exe().unwrap();
		exe_dir.pop();

		exe_dir
	};

	let balances_configs: Vec<BalancesConfig> =
		super::config_from_json_files(exe_dir.join("res/genesis_config/balances"))
			.unwrap();

	let mut total_issuance: Balance = Zero::zero();
	let balances = balances_configs
		.into_iter()
		.flat_map(|bc| bc.balances)
		.fold(BTreeMap::<AccountId, Balance>::new(), |mut acc, (account_id, amount)| {
			if let Some(balance) = acc.get_mut(&account_id) {
				*balance = balance.checked_add(amount).expect("balance cannot overflow when building genesis");
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
		32_000_000 * DOLLARS,
		"total issuance must be equal to 320 million"
	);

	let vesting_configs: Vec<VestingConfig> =
		super::config_from_json_files(exe_dir.join("res/genesis_config/vesting"))
			.unwrap();

	use sp_core::sp_std::collections::btree_map::BTreeMap;
	bifrost_genesis(
		invulnerables,
		root_key,
		balances,
		vesting_configs.into_iter().flat_map(|vc| vc.vesting).collect(),
		id,
	)
}

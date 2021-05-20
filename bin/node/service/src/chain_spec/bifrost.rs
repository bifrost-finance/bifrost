// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

use hex_literal::hex;
use sc_chain_spec::ChainType;
use sc_telemetry::TelemetryEndpoints;
use sp_core::{crypto::UncheckedInto, sr25519};
use cumulus_primitives_core::ParaId;
use bifrost_runtime::{
	AccountId, AuraId,
	constants::{currency::DOLLARS},
	AuraConfig, BalancesConfig, GenesisConfig, IndicesConfig, SudoConfig, SystemConfig,
	ParachainInfoConfig, WASM_BINARY, wasm_binary_unwrap,
};
use crate::chain_spec::{RelayExtensions, get_account_id_from_seed, testnet_accounts, get_from_seed};

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const DEFAULT_PROTOCOL_ID: &str = "bifrost";

/// The `ChainSpec` parametrized for the bifrost runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

pub fn config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../../res/bifrost.json")[..])
}

fn staging_testnet_config_genesis(id: ParaId) -> GenesisConfig {
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
	// and
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

	let initial_authorities: Vec<AuraId> = vec![
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
	];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = hex![
		// 5Ff3iXP75ruzroPWRP2FYBHWnmGGBSb63857BgnzCoXNxfPo
		"9ee5e5bdc0ec239eb164f865ecc345ce4c88e76ee002e0f7e318097347471809"
	].into();

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	testnet_genesis(
		initial_authorities,
		root_key,
		Some(endowed_accounts),
		id,
	)
}

pub fn staging_testnet_config(id: ParaId) -> ChainSpec {
	let boot_nodes = vec![];
	ChainSpec::from_genesis(
		"Bifrost PC1 Staging Testnet",
		"bifrost_pc1_staging_testnet",
		ChainType::Live,
		move || {
			staging_testnet_config_genesis(id)
		},
		boot_nodes,
		Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Staging telemetry url is valid; qed")),
		None,
		None,
		RelayExtensions {
			relay_chain: "westend-dev".into(),
			para_id: id.into(),
		},
	)
}

/// Helper function to create bifrost GenesisConfig for testing
pub fn testnet_genesis(
	initial_authorities: Vec<AuraId>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
	id: ParaId,
) -> GenesisConfig {
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);

	const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

	GenesisConfig {
		frame_system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
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
		pallet_sudo: SudoConfig {
			key: root_key.clone(),
		},
		parachain_info: ParachainInfoConfig { parachain_id: id },
		pallet_aura: AuraConfig {
			authorities: initial_authorities,
		},
		cumulus_pallet_aura_ext: Default::default(),
	}
}

fn development_config_genesis(_wasm_binary: &[u8], id: ParaId) -> GenesisConfig {
	testnet_genesis(
		vec![get_from_seed::<AuraId>("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
		id,
	)
}

/// Bifrost PC1 development config (single validator Alice)
pub fn development_config(id: ParaId) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or("Bifrost PC1 development wasm not available")?;

	Ok(ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		move || development_config_genesis(wasm_binary, id),
		vec![],
		Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Bifrost PC1 Testnet telemetry url is valid; qed")),
		Some(DEFAULT_PROTOCOL_ID),
		None,
		RelayExtensions {
			relay_chain: "westend-dev".into(),
			para_id: id.into(),
		},
	))
}

fn local_testnet_genesis(_wasm_binary: &[u8], id: ParaId) -> GenesisConfig {
	testnet_genesis(
		vec![
			get_from_seed::<AuraId>("Alice"),
			get_from_seed::<AuraId>("Bob"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
		id,
	)
}

/// Bifrost PC1 local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config(id: ParaId) -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or("Bifrost PC1 development wasm not available")?;

	Ok(ChainSpec::from_genesis(
		"Bifrost PC1 Local Testnet",
		"rococo_pc1_local_testnet",
		ChainType::Local,
		move || local_testnet_genesis(wasm_binary, id),
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

pub fn chainspec_config(id: ParaId) -> ChainSpec {
	let properties = {
		let mut props = serde_json::Map::new();

		props.insert(
			"ss58Format".to_owned(),
			serde_json::value::to_value(6u8).expect("The ss58Format cannot be convert to json value.")
		);
		props.insert(
			"tokenDecimals".to_owned(),
			serde_json::value::to_value(12u8).expect("The tokenDecimals cannot be convert to json value.")
		);
		props.insert(
			"tokenSymbol".to_owned(),
			serde_json::value::to_value("BNC".to_owned()).expect("The tokenSymbol cannot be convert to json value.")
		);
		Some(props)
	};
	let protocol_id = Some("bifrost");

	ChainSpec::from_genesis(
		"Bifrost",
		"bifrost_mainnet",
		ChainType::Custom("Bifrost Mainnet".into()),
		move || {
			bifrost_config_genesis(id)
		},
		vec![
			// "/dns/bifrost-1.testnet.liebi.com/tcp/30333/p2p/12D3KooWNM2rAjo2FqUgtQ2nnQ7nNxQntB9ssHS5TryvTVMpMKxa".parse().expect("failed to parse multiaddress.")
		],
		Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Bifrost PC1 Testnet telemetry url is valid; qed")),
		protocol_id,
		properties,
		RelayExtensions {
			relay_chain: "westend-dev".into(),
			para_id: id.into(),
		},
	)
}

fn bifrost_config_genesis(id: ParaId) -> GenesisConfig {
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

	testnet_genesis(
		initial_authorities,
		root_key,
		Some(endowed_accounts),
		id,
	)
} 

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

use crate::chain_spec::{
	authority_keys_from_seed, get_account_id_from_seed, testnet_accounts, AuthorityDiscoveryId,
	BabeId, Extensions, GrandpaId,
};
use bifrost_runtime::{
	constants::currency::DOLLARS, wasm_binary_unwrap, AuthorityDiscoveryConfig, BabeConfig,
	BalancesConfig, CouncilConfig, DemocracyConfig, GenesisConfig, GrandpaConfig, ImOnlineConfig,
	IndicesConfig, PoaManagerConfig, SessionConfig, SessionKeys, SudoConfig, SystemConfig,
	TechnicalCommitteeConfig, VestingConfig, WASM_BINARY,
};
use hex_literal::hex;
use node_primitives::AccountId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sc_chain_spec::ChainType;
use sp_core::{crypto::UncheckedInto, sr25519};
use telemetry::TelemetryEndpoints;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const DEFAULT_PROTOCOL_ID: &str = "bnc";

/// The `ChainSpec` parametrised for the bifrost runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

pub fn config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../../res/bifrost.json")[..])
}

fn session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys {
		babe,
		grandpa,
		im_online,
		authority_discovery,
	}
}

fn staging_testnet_config_genesis() -> GenesisConfig {
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
	// and
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

	let initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)> = vec![
		(
			// 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
			hex!["9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"].into(),
			// 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
			hex!["781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"].into(),
			// 5Fb9ayurnxnaXj56CjmyQLBiadfRCqUbL2VWNbbe1nZU6wiC
			hex!["9becad03e6dcac03cee07edebca5475314861492cdfc96a2144a67bbe9699332"]
				.unchecked_into(),
			// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
			hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"]
				.unchecked_into(),
			// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
			hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"]
				.unchecked_into(),
			// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
			hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"]
				.unchecked_into(),
		),
		(
			// 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
			hex!["68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"].into(),
			// 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
			hex!["c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"].into(),
			// 5EockCXN6YkiNCDjpqqnbcqd4ad35nU4RmA1ikM4YeRN4WcE
			hex!["7932cff431e748892fa48e10c63c17d30f80ca42e4de3921e641249cd7fa3c2f"]
				.unchecked_into(),
			// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
			hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"]
				.unchecked_into(),
			// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
			hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"]
				.unchecked_into(),
			// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
			hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"]
				.unchecked_into(),
		),
		(
			// 5DyVtKWPidondEu8iHZgi6Ffv9yrJJ1NDNLom3X9cTDi98qp
			hex!["547ff0ab649283a7ae01dbc2eb73932eba2fb09075e9485ff369082a2ff38d65"].into(),
			// 5FeD54vGVNpFX3PndHPXJ2MDakc462vBCD5mgtWRnWYCpZU9
			hex!["9e42241d7cd91d001773b0b616d523dd80e13c6c2cab860b1234ef1b9ffc1526"].into(),
			// 5E1jLYfLdUQKrFrtqoKgFrRvxM3oQPMbf6DfcsrugZZ5Bn8d
			hex!["5633b70b80a6c8bb16270f82cca6d56b27ed7b76c8fd5af2986a25a4788ce440"]
				.unchecked_into(),
			// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
			hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"]
				.unchecked_into(),
			// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
			hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"]
				.unchecked_into(),
			// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
			hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"]
				.unchecked_into(),
		),
		(
			// 5HYZnKWe5FVZQ33ZRJK1rG3WaLMztxWrrNDb1JRwaHHVWyP9
			hex!["f26cdb14b5aec7b2789fd5ca80f979cef3761897ae1f37ffb3e154cbcc1c2663"].into(),
			// 5EPQdAQ39WQNLCRjWsCk5jErsCitHiY5ZmjfWzzbXDoAoYbn
			hex!["66bc1e5d275da50b72b15de072a2468a5ad414919ca9054d2695767cf650012f"].into(),
			// 5DMa31Hd5u1dwoRKgC4uvqyrdK45RHv3CpwvpUC1EzuwDit4
			hex!["3919132b851ef0fd2dae42a7e734fe547af5a6b809006100f48944d7fae8e8ef"]
				.unchecked_into(),
			// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
			hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"]
				.unchecked_into(),
			// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
			hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"]
				.unchecked_into(),
			// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
			hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"]
				.unchecked_into(),
		),
	];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = hex![
		// 5Ff3iXP75ruzroPWRP2FYBHWnmGGBSb63857BgnzCoXNxfPo
		"9ee5e5bdc0ec239eb164f865ecc345ce4c88e76ee002e0f7e318097347471809"
	]
	.into();

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	load_genesis_config_from_json_or_default(initial_authorities, root_key, endowed_accounts)
}

// TODO: Too verbose, which should be splited.
fn load_genesis_config_from_json_or_default(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,
	mut endowed_accounts: Vec<AccountId>,
) -> GenesisConfig {
	use serde_json::Value;
	use sp_core::sp_std::collections::btree_map::BTreeMap;
	use sp_core::sp_std::iter::FromIterator;

	const NAME: &str = "bifrost_mainnet.json";
	const PATH: &str = "./genesis_config/bifrost_mainnet.json";

	// Balances default value.
	const ENDOWMENT: u128 = 1_000_000 * DOLLARS;
	// Vesting default value.
	const LOCKED: u32 = 0;
	const PER_BLOCK: u32 = 10_000;
	const STARTING_BLOCK: u128 = ENDOWMENT / 2;

	// TODO: (Post Build)Copy res/genesis_config to target directory after build.

	#[cfg(feature = "std")]
	let json_str: String = {
		if let Ok(str) = read_json_from_file(PATH) {
			str
		} else {
			log::info!("CANNOT load {}, use default genesis config.", PATH);
			String::new()
		}
	};
	#[cfg(not(feature = "std"))]
	let json_str: &str = include_str!("../../res/genesis_config/bifrost_mainnet.json");

	// ASK: Why endowed_accounts need to be modified?
	let num_endowed_accounts = endowed_accounts.len();
	initial_authorities.iter().for_each(|(account_id, ..)| {
		if !endowed_accounts.contains(account_id) {
			endowed_accounts.push(account_id.clone())
		}
	});

	// Init `BalancesConfig` and `VestingConfig` by default.
	let mut balances_config = BalancesConfig {
		balances: endowed_accounts
			.iter()
			.cloned()
			.map(|account_id| (account_id, ENDOWMENT))
			.collect(),
	};
	let mut vesting_config = VestingConfig {
		vesting: endowed_accounts
			.iter()
			.cloned()
			.map(|account_id| (account_id, LOCKED, PER_BLOCK, STARTING_BLOCK))
			.collect(),
	};

	// Modify genesis config by json.
	if let Ok(genesis_config) = serde_json::from_str::<Value>(&json_str) {
		// Endowed accounts btree_map.
		let mut ec_map = BTreeMap::from_iter(
			endowed_accounts
				.iter()
				.cloned()
				.map(|account_id| (account_id, false)),
		);

		// Modify `BalancesConfig` if has `"palletBalances": BalancesConfig` pair in json.
		if let Some(bv) = genesis_config.get("palletBalances") {
			if let Ok(mut bc) = serde_json::from_value::<BalancesConfig>(bv.clone()) {
				balances_config = Default::default();

				for (account_id, balance) in bc.balances.drain(..) {
					if ec_map.contains_key(&account_id) {
						// NORMAL: account_id in endowed_accounts and in json.
						if ec_map[&account_id] != true {
							*ec_map.get_mut(&account_id).unwrap() = true;
							balances_config.balances.push((account_id, balance));
						}
						// WARNING: duplicate account_id.
						else {
							log::info!(
								"Duplicate account_id({}) in `palletBalances` in json.",
								account_id
							);
						}
					}
					// WARNING: account_id not in endowed_accounts.
					else {
						log::info!(
							"Extra account_id({}) in `palletBalances` in json.",
							account_id
						);
					}
				}

				for account_id in ec_map
					.iter()
					.filter(|(_, is_set)| !**is_set)
					.map(|(account_id, _)| account_id)
					.cloned()
				{
					// WARNING: missing account_id.
					log::info!(
						"Missing account_id({}) in `palletBalances` in json.",
						&account_id
					);
					balances_config.balances.push((account_id, ENDOWMENT));
				}
			} else {
				log::info!(
					"CANNOT convert json in {} to `BalancesConfig`, use default balances config.",
					NAME
				);
			}
		} else {
			log::info!(
				"Not find `palletBalances` in {}, use default balances config.",
				NAME
			);
		}

		// Reset ec_map.
		ec_map.iter_mut().for_each(|(_, is_set)| *is_set = false);

		// Modify `VestingConfig` if has `"palletVesting": VestingConfig` pair in json.
		if let Some(vv) = genesis_config.get("palletVesting") {
			if let Ok(mut vc) = serde_json::from_value::<VestingConfig>(vv.clone()) {
				vesting_config = Default::default();

				for (account_id, locked, per_block, starting_block) in vc.vesting.drain(..) {
					if ec_map.contains_key(&account_id) {
						// NORMAL: account_id in endowed_accounts and in json.
						if ec_map[&account_id] != true {
							*ec_map.get_mut(&account_id).unwrap() = true;
							vesting_config.vesting.push((
								account_id,
								locked,
								per_block,
								starting_block,
							));
						}
						// WARNING: duplicate account_id.
						else {
							log::info!(
								"Duplicate account_id({}) in `palletVesting` in json.",
								account_id
							);
						}
					}
					// WARNING: account_id not in endowed_accounts.
					else {
						log::info!(
							"Extra account_id({}) in `palletVesting` in json.",
							account_id
						);
					}
				}

				for account_id in ec_map
					.iter()
					.filter(|(_, is_set)| !**is_set)
					.map(|(account_id, _)| account_id)
					.cloned()
				{
					// WARNING: missing account_id.
					log::info!(
						"Missing account_id({}) in `palletVesting` in json.",
						&account_id
					);
					vesting_config
						.vesting
						.push((account_id, LOCKED, PER_BLOCK, STARTING_BLOCK));
				}
			} else {
				log::info!(
					"CANNOT convert json in {} to `VestingConfig`, use default vesting config.",
					NAME
				);
			}
		} else {
			log::info!(
				"Not find `palletVesting` in {}, use default vesting config.",
				NAME
			);
		}
	} else {
		log::info!("CANNOT parse {}, use default genesis config.", NAME);
	}

	// TODO: Too verbose.
	GenesisConfig {
		frame_system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: balances_config,
		pallet_indices: IndicesConfig { indices: vec![] },
		pallet_session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
		pallet_im_online: ImOnlineConfig { keys: vec![] },
		pallet_democracy: DemocracyConfig::default(),
		pallet_collective_Instance1: CouncilConfig::default(),
		pallet_collective_Instance2: TechnicalCommitteeConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		},
		pallet_sudo: SudoConfig {
			key: root_key.clone(),
		},
		pallet_babe: BabeConfig {
			authorities: vec![],
		},
		pallet_authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
		pallet_grandpa: GrandpaConfig {
			authorities: vec![],
		},
		pallet_membership_Instance1: Default::default(),
		pallet_treasury: Default::default(),
		pallet_vesting: vesting_config,
		brml_poa_manager: PoaManagerConfig {
			initial_validators: initial_authorities
				.iter()
				.map(|x| x.0.clone())
				.collect::<Vec<_>>(),
		},
	}
}

#[cfg(feature = "std")]
fn read_json_from_file(path: &str) -> Result<String, Box<std::io::Error>> {
	use std::fs::File;
	use std::io::Read;

	let mut file = File::open(path)?;
	let mut json_str = String::new();
	file.read_to_string(&mut json_str)?;

	Ok(json_str)
}

pub fn staging_testnet_config() -> ChainSpec {
	let boot_nodes = vec![];
	ChainSpec::from_genesis(
		"Bifrost Staging Testnet",
		"bifrost_staging_testnet",
		ChainType::Live,
		staging_testnet_config_genesis,
		boot_nodes,
		Some(
			TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
				.expect("Staging telemetry url is valid; qed"),
		),
		None,
		None,
		Default::default(),
	)
}

/// Helper function to create bifrost GenesisConfig for testing
pub fn testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		GrandpaId,
		BabeId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> GenesisConfig {
	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);
	let num_endowed_accounts = endowed_accounts.len();

	initial_authorities.iter().for_each(|x| {
		if !endowed_accounts.contains(&x.0) {
			endowed_accounts.push(x.0.clone())
		}
	});

	const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

	GenesisConfig {
		frame_system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|x| (x, ENDOWMENT))
				.collect(),
		},
		pallet_indices: IndicesConfig { indices: vec![] },
		pallet_session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
					)
				})
				.collect::<Vec<_>>(),
		},
		pallet_im_online: ImOnlineConfig { keys: vec![] },
		pallet_democracy: DemocracyConfig::default(),
		pallet_collective_Instance1: CouncilConfig::default(),
		pallet_collective_Instance2: TechnicalCommitteeConfig {
			members: endowed_accounts
				.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		},
		pallet_sudo: SudoConfig {
			key: root_key.clone(),
		},
		pallet_babe: BabeConfig {
			authorities: vec![],
		},
		pallet_authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
		pallet_grandpa: GrandpaConfig {
			authorities: vec![],
		},
		pallet_membership_Instance1: Default::default(),
		pallet_treasury: Default::default(),
		pallet_vesting: VestingConfig {
			vesting: endowed_accounts
				.iter()
				.map(|account_id| (account_id.clone(), 0, 10000, ENDOWMENT / 2))
				.collect::<Vec<_>>(),
		},
		brml_poa_manager: PoaManagerConfig {
			initial_validators: initial_authorities
				.iter()
				.map(|x| x.0.clone())
				.collect::<Vec<_>>(),
		},
	}
}

fn development_config_genesis(_wasm_binary: &[u8]) -> GenesisConfig {
	testnet_genesis(
		vec![authority_keys_from_seed("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Bifrost development config (single validator Alice)
pub fn development_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or("Bifrost development wasm not available")?;

	let properties = {
		let mut props = serde_json::Map::new();

		props.insert(
			"ss58Format".to_owned(),
			serde_json::value::to_value(6u8)
				.expect("The ss58Format cannot be convert to json value."),
		);
		props.insert(
			"tokenDecimals".to_owned(),
			serde_json::value::to_value(12u8)
				.expect("The tokenDecimals cannot be convert to json value."),
		);
		props.insert(
			"tokenSymbol".to_owned(),
			serde_json::value::to_value("BNC".to_owned())
				.expect("The tokenSymbol cannot be convert to json value."),
		);
		Some(props)
	};
	let protocol_id = Some("bifrost");

	Ok(ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		move || development_config_genesis(wasm_binary),
		vec![],
		None,
		protocol_id,
		properties,
		Default::default(),
	))
}

fn local_testnet_genesis(_wasm_binary: &[u8]) -> GenesisConfig {
	testnet_genesis(
		vec![authority_keys_from_seed("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Bifrost local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> Result<ChainSpec, String> {
	let wasm_binary = WASM_BINARY.ok_or("Bifrost development wasm not available")?;

	Ok(ChainSpec::from_genesis(
		"Bifrost Local Testnet",
		"bifrost_local_testnet",
		ChainType::Local,
		move || local_testnet_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Default::default(),
	))
}

pub fn chainspec_config() -> ChainSpec {
	let properties = {
		let mut props = serde_json::Map::new();

		props.insert(
			"ss58Format".to_owned(),
			serde_json::value::to_value(6u8)
				.expect("The ss58Format cannot be convert to json value."),
		);
		props.insert(
			"tokenDecimals".to_owned(),
			serde_json::value::to_value(12u8)
				.expect("The tokenDecimals cannot be convert to json value."),
		);
		props.insert(
			"tokenSymbol".to_owned(),
			serde_json::value::to_value("BNC".to_owned())
				.expect("The tokenSymbol cannot be convert to json value."),
		);
		Some(props)
	};
	let protocol_id = Some("bifrost");

	ChainSpec::from_genesis(
        "Bifrost",
        "bifrost_mainnet",
        ChainType::Custom("Bifrost Mainnet".into()),
        staging_testnet_config_genesis,
        vec![
            "/dns/n1.testnet.liebi.com/tcp/30333/p2p/12D3KooWHjmfpAdrjL7EvZ7Zkk4pFmkqKDLL5JDENc7oJdeboxJJ".parse().expect("failed to parse multiaddress."),
            "/dns/n2.testnet.liebi.com/tcp/30333/p2p/12D3KooWPbTeqZHdyTdqY14Zu2t6FVKmUkzTZc3y5GjyJ6ybbmSB".parse().expect("failed to parse multiaddress."),
            "/dns/n3.testnet.liebi.com/tcp/30333/p2p/12D3KooWLt3w5tadCR5Fc7ZvjciLy7iKJ2ZHq6qp4UVmUUHyCJuX".parse().expect("failed to parse multiaddress."),
            "/dns/n4.testnet.liebi.com/tcp/30333/p2p/12D3KooWMduQkmRVzpwxJuN6MQT4ex1iP9YquzL4h5K9Ru8qMXtQ".parse().expect("failed to parse multiaddress."),
            "/dns/n5.testnet.liebi.com/tcp/30333/p2p/12D3KooWLAHZyqMa9TQ1fR7aDRRKfWt857yFMT3k2ckK9mhYT9qR".parse().expect("failed to parse multiaddress.")
        ],
        Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
            .expect("Asgard Testnet telemetry url is valid; qed")),
        protocol_id,
        properties,
        Default::default(),
    )
}

// Copyright 2019-2020 Liebi Technologies.
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
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::Perbill;
use telemetry::TelemetryEndpoints;
use node_primitives::{AccountId, ConvertPool, TokenType, Token};
use bifrost_runtime::{
	constants::currency::{BNCS as BNC, DOLLARS},
	AssetsConfig, AuthorityDiscoveryConfig, BabeConfig, BalancesConfig,
	BridgeEosConfig,
	BridgeIostConfig,
	ConvertConfig, CouncilConfig, DemocracyConfig, ElectionsConfig,
	GenesisConfig, GrandpaConfig, ImOnlineConfig, IndicesConfig, SessionConfig, SessionKeys,
	SocietyConfig, StakingConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig, VoucherConfig,
	StakerStatus, WASM_BINARY, wasm_binary_unwrap,
};
use crate::chain_spec::{
	Extensions, BabeId, GrandpaId, ImOnlineId, AuthorityDiscoveryId,
	authority_keys_from_seed, get_account_id_from_seed, initialize_all_vouchers, testnet_accounts
};

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

	let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> = vec![(
		// 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
		hex!["9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"].into(),
		// 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
		hex!["781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"].into(),
		// 5Fb9ayurnxnaXj56CjmyQLBiadfRCqUbL2VWNbbe1nZU6wiC
		hex!["9becad03e6dcac03cee07edebca5475314861492cdfc96a2144a67bbe9699332"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
	),(
		// 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
		hex!["68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"].into(),
		// 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
		hex!["c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"].into(),
		// 5EockCXN6YkiNCDjpqqnbcqd4ad35nU4RmA1ikM4YeRN4WcE
		hex!["7932cff431e748892fa48e10c63c17d30f80ca42e4de3921e641249cd7fa3c2f"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
	),(
		// 5DyVtKWPidondEu8iHZgi6Ffv9yrJJ1NDNLom3X9cTDi98qp
		hex!["547ff0ab649283a7ae01dbc2eb73932eba2fb09075e9485ff369082a2ff38d65"].into(),
		// 5FeD54vGVNpFX3PndHPXJ2MDakc462vBCD5mgtWRnWYCpZU9
		hex!["9e42241d7cd91d001773b0b616d523dd80e13c6c2cab860b1234ef1b9ffc1526"].into(),
		// 5E1jLYfLdUQKrFrtqoKgFrRvxM3oQPMbf6DfcsrugZZ5Bn8d
		hex!["5633b70b80a6c8bb16270f82cca6d56b27ed7b76c8fd5af2986a25a4788ce440"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
	),(
		// 5HYZnKWe5FVZQ33ZRJK1rG3WaLMztxWrrNDb1JRwaHHVWyP9
		hex!["f26cdb14b5aec7b2789fd5ca80f979cef3761897ae1f37ffb3e154cbcc1c2663"].into(),
		// 5EPQdAQ39WQNLCRjWsCk5jErsCitHiY5ZmjfWzzbXDoAoYbn
		hex!["66bc1e5d275da50b72b15de072a2468a5ad414919ca9054d2695767cf650012f"].into(),
		// 5DMa31Hd5u1dwoRKgC4uvqyrdK45RHv3CpwvpUC1EzuwDit4
		hex!["3919132b851ef0fd2dae42a7e734fe547af5a6b809006100f48944d7fae8e8ef"].unchecked_into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
	)];

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
	)
}

pub fn staging_testnet_config() -> ChainSpec {
	let boot_nodes = vec![];
	ChainSpec::from_genesis(
		"Bifrost Staging Testnet",
		"bifrost_staging_testnet",
		ChainType::Live,
		staging_testnet_config_genesis,
		boot_nodes,
		Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Staging telemetry url is valid; qed")),
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

	initial_authorities.iter().for_each(|x|
		if !endowed_accounts.contains(&x.0) {
			endowed_accounts.push(x.0.clone())
		}
	);

	const ENDOWMENT: u128 = 1_000_000 * BNC;
	const STASH: u128 = ENDOWMENT / 1000;

	GenesisConfig {
		frame_system: Some(SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_balances: Some(BalancesConfig {
			balances: endowed_accounts.iter().cloned()
				.map(|x| (x, ENDOWMENT))
				.collect()
		}),
		pallet_indices: Some(IndicesConfig {
			indices: vec![],
		}),
		pallet_session: Some(SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.0.clone(), session_keys(
					x.2.clone(),
					x.3.clone(),
					x.4.clone(),
					x.5.clone(),
				))
			}).collect::<Vec<_>>(),
		}),
		pallet_staking: Some(StakingConfig {
			validator_count: 30,
			minimum_validator_count: 3,
			stakers: initial_authorities.iter().map(|x| { // we need last three addresses
				(x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)
			}).collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone())
				.chain(endowed_accounts.iter().cloned()).collect::<Vec<_>>(),
			slash_reward_fraction: Perbill::from_percent(10),
			.. Default::default()
		}),
		pallet_democracy: Some(DemocracyConfig::default()),
		pallet_elections_phragmen: Some(ElectionsConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		}),
		pallet_collective_Instance1: Some(CouncilConfig::default()),
		pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		}),
		pallet_sudo: Some(SudoConfig {
			key: root_key.clone(),
		}),
		pallet_babe: Some(BabeConfig {
			authorities: vec![],
		}),
		pallet_im_online: Some(ImOnlineConfig {
			keys: vec![],
		}),
		pallet_authority_discovery: Some(AuthorityDiscoveryConfig {
			keys: vec![],
		}),
		pallet_grandpa: Some(GrandpaConfig {
			authorities: vec![],
		}),
		pallet_membership_Instance1: Some(Default::default()),
		pallet_treasury: Some(Default::default()),
		pallet_society: Some(SocietyConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			pot: 0,
			max_members: 999,
		}),
		pallet_vesting: Some(Default::default()),
		brml_assets: Some(AssetsConfig {
			account_assets: vec![],
			token_details: vec![
				(0, Token::new(b"BNC".to_vec(), 12, 0, TokenType::Native)),
				(1, Token::new(b"aUSD".to_vec(), 18, 0, TokenType::Stable)),
				(2, Token::new(b"DOT".to_vec(), 12, 0, TokenType::Token)),
				(4, Token::new(b"KSM".to_vec(), 12, 0, TokenType::Token)),
				(6, Token::new(b"EOS".to_vec(), 4, 0, TokenType::Token)),
				(8, Token::new(b"IOST".to_vec(), 8, 0, TokenType::Token)),
			],
		}),
		brml_convert: Some(ConvertConfig {
			convert_price: vec![
				(2, DOLLARS / 100), // DOT
				(4, DOLLARS / 100), // KSM
				(6, DOLLARS / 100), // EOS
				(8, DOLLARS / 100), // IOST
			], // initialize convert price as token = 100 * vtoken
			pool: vec![
				(2, ConvertPool::new(1, 100)), // DOT
				(4, ConvertPool::new(1, 100)), // KSM
				(6, ConvertPool::new(1, 100)), // EOS
				(8, ConvertPool::new(1, 100)), // IOST
			],
		}),
		brml_bridge_eos: Some(BridgeEosConfig {
			bridge_contract_account: (b"bifrostcross".to_vec(), 2),
			notary_keys: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			// alice and bob have the privilege to sign cross transaction
			cross_chain_privilege: [(root_key.clone(), true)].iter().cloned().collect::<Vec<_>>(),
			all_crosschain_privilege: Vec::new(),
			cross_trade_eos_limit: 50 * DOLLARS, // 50 EOS as limit
			eos_asset_id: 6,
		}),
		brml_bridge_iost: Some(BridgeIostConfig {
			bridge_contract_account: (b"bifrost".to_vec(), 1),
			notary_keys: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			// alice and bob have the privilege to sign cross transaction
			cross_chain_privilege: [(root_key.clone(), true)].iter().cloned().collect::<Vec<_>>(),
			all_crosschain_privilege: Vec::new(),
			iost_asset_id: 8,
		}),
		brml_voucher: {
			if let Some(vouchers) = initialize_all_vouchers() {
				Some(VoucherConfig { voucher: vouchers })
			} else {
				None
			}
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
		vec![
			authority_keys_from_seed("Alice"),
			authority_keys_from_seed("Bob"),
		],
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

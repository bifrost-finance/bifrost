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
use sp_core::{crypto::UncheckedInto, sr25519};
use telemetry::TelemetryEndpoints;
use cumulus_primitives_core::ParaId;
use node_primitives::{AccountId, VtokenPool, TokenType, Token, TokenSymbol, CurrencyId};
use rococo_runtime::{
	constants::currency::DOLLARS,
	AssetsConfig, BalancesConfig, VtokenMintConfig,
	GenesisConfig, IndicesConfig, SudoConfig, SystemConfig, TokensConfig, VoucherConfig,
	ParachainInfoConfig, WASM_BINARY, wasm_binary_unwrap,
};
use pallet_im_online::sr25519::{AuthorityId as ImOnlineId};
use crate::chain_spec::{
	RelayExtensions, BabeId, GrandpaId, AuthorityDiscoveryId,
	authority_keys_from_seed, get_account_id_from_seed, initialize_all_vouchers, testnet_accounts
};

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const DEFAULT_PROTOCOL_ID: &str = "roc";

/// The `ChainSpec` parametrized for the rococo runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, RelayExtensions>;

pub fn config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../../res/rococo.json")[..])
}

fn staging_testnet_config_genesis(id: ParaId) -> GenesisConfig {
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

/// Helper function to create rococo GenesisConfig for testing
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
	id: ParaId,
) -> GenesisConfig {
	let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);

	initial_authorities.iter().for_each(|x|
		if !endowed_accounts.contains(&x.0) {
			endowed_accounts.push(x.0.clone())
		}
	);

	const ENDOWMENT: u128 = 1_000_000 * DOLLARS;

	GenesisConfig {
		frame_system: SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		},
		pallet_balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned()
				.map(|x| (x, ENDOWMENT))
				.collect()
		},
		pallet_indices: IndicesConfig {
			indices: vec![],
		},
		pallet_sudo: SudoConfig {
			key: root_key.clone(),
		},
		brml_assets: AssetsConfig {
			account_assets: vec![],
			token_details: vec![
				(0, Token::new(b"BNC".to_vec(), 12, 0, TokenType::Native)),
				(1, Token::new(b"aUSD".to_vec(), 18, 0, TokenType::Stable)),
				(2, Token::new(b"DOT".to_vec(), 12, 0, TokenType::Token)),
				(4, Token::new(b"KSM".to_vec(), 12, 0, TokenType::Token)),
			],
		},
		brml_vtoken_mint: VtokenMintConfig {
			mint_price: vec![
				(2, DOLLARS / 100), // DOT
				(4, DOLLARS / 100), // KSM
			], // initialize convert price as token = 100 * vtoken
			pool: vec![
				(2, VtokenPool::new(1, 100)), // DOT
				(4, VtokenPool::new(1, 100)), // KSM
			],
		},
		brml_voucher: {
			if let Some(vouchers) = initialize_all_vouchers() {
				VoucherConfig { voucher: vouchers }
			} else {
				Default::default()
			}
		},
		orml_tokens: TokensConfig {
			endowed_accounts: endowed_accounts
				.iter()
				.flat_map(|x| {
					vec![
						(x.clone(), CurrencyId::Token(TokenSymbol::BNC), ENDOWMENT),
						(x.clone(), CurrencyId::Token(TokenSymbol::AUSD), ENDOWMENT),
						(x.clone(), CurrencyId::Token(TokenSymbol::DOT), ENDOWMENT),
						(x.clone(), CurrencyId::Token(TokenSymbol::KSM), ENDOWMENT),
					]
				})
				.collect(),
		},
		parachain_info: ParachainInfoConfig { parachain_id: id },
	}
}

fn development_config_genesis(_wasm_binary: &[u8], id: ParaId) -> GenesisConfig {
	testnet_genesis(
		vec![authority_keys_from_seed("Alice")],
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
			authority_keys_from_seed("Alice"),
			authority_keys_from_seed("Bob"),
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
			serde_json::value::to_value("ROC".to_owned()).expect("The tokenSymbol cannot be convert to json value.")
		);
		Some(props)
	};
	let protocol_id = Some("rococo");

	ChainSpec::from_genesis(
		"Bifrost PC1",
		"bifrost_pc1_testnet",
		ChainType::Custom("Bifrost PC1 Testnet".into()),
		move || {
			staging_testnet_config_genesis(id)
		},
		vec![],
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

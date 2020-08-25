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

//! Bifrost chain configurations.

use sc_chain_spec::ChainSpecExtension;
use sp_core::{Pair, Public, crypto::UncheckedInto, sr25519};
use serde::{Serialize, Deserialize};
use node_runtime::{
	AuthorityDiscoveryConfig, BabeConfig, BalancesConfig, CouncilConfig, DemocracyConfig, ElectionsConfig,
	GrandpaConfig, ImOnlineConfig, SessionConfig, SessionKeys, StakerStatus, StakingConfig,
	IndicesConfig, SocietyConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig, wasm_binary_unwrap,
	AssetsConfig, BridgeEosConfig, VoucherConfig, SwapConfig, ConvertConfig,
};
use node_runtime::Block;
use node_runtime::constants::currency::*;
use sc_service::ChainType;
use hex_literal::hex;
use sc_telemetry::TelemetryEndpoints;
use grandpa_primitives::{AuthorityId as GrandpaId};
use sp_consensus_babe::{AuthorityId as BabeId};
use pallet_im_online::sr25519::{AuthorityId as ImOnlineId};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_runtime::{Perbill, traits::{Verify, IdentifyAccount}};

//pub use node_primitives::{AccountId, AccountAsset, Balance, Cost, Income, Signature, TokenSymbol, ConvertPool};
pub use node_primitives::{AccountId, AccountAsset, Balance, Cost, Income, Signature, TokenSymbol, ConvertPool};
pub use node_runtime::GenesisConfig;

type AccountPublic = <Signature as Verify>::Signer;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<Block>,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<
	GenesisConfig,
	Extensions,
>;
/// Flaming Fir testnet generator
pub fn flaming_fir_config() -> Result<ChainSpec, String> {
	ChainSpec::from_json_bytes(&include_bytes!("../res/flaming-fir.json")[..])
}

fn session_keys(
	grandpa: GrandpaId,
	babe: BabeId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
	SessionKeys { grandpa, babe, im_online, authority_discovery }
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

/// Staging testnet config.
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

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(seed: &str) -> (
	AccountId,
	AccountId,
	GrandpaId,
	BabeId,
	ImOnlineId,
	AuthorityDiscoveryId,
) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

/// Helper function to create GenesisConfig for testing
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
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
		vec![
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
		]
	});
	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: Balance = 10_000 * DOLLARS;
	const STASH: Balance = 100 * DOLLARS;

	GenesisConfig {
		frame_system: Some(SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_balances: Some(BalancesConfig {
			balances: endowed_accounts.iter().cloned()
				.map(|k| (k, ENDOWMENT))
				.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
				.collect(),
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
			validator_count: initial_authorities.len() as u32 * 2,
			minimum_validator_count: initial_authorities.len() as u32,
			stakers: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)
			}).collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
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
			next_asset_id: 7u32, // start from 7, [0..6] has been reserved
			token_details: vec![],
			prices: vec![],
		}),
		brml_convert: Some(ConvertConfig {
			convert_price: vec![
				(TokenSymbol::DOT, DOLLARS / 100),
				(TokenSymbol::KSM, DOLLARS / 100),
				(TokenSymbol::EOS, DOLLARS / 100),
			], // initialize convert price as token = 100 * vtoken
//			pool: vec![
//				(TokenSymbol::DOT, ConvertPool::new(1, 100)),
//				(TokenSymbol::KSM, ConvertPool::new(1, 100)),
//				(TokenSymbol::EOS, ConvertPool::new(1, 100)),
//			],
		}),
		brml_bridge_eos: Some(BridgeEosConfig {
			bridge_contract_account: (b"bifrostcross".to_vec(), 2),
			notary_keys: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			// alice and bob have the privilege to sign cross transaction
			cross_chain_privilege: [(root_key.clone(), true)].iter().cloned().collect::<Vec<_>>(),
			all_crosschain_privilege: Vec::new(),
		}),
		brml_voucher: {
			if let Some(vouchers) = initialize_all_vouchers() {
				Some(VoucherConfig { voucher: vouchers })
			} else {
				None
			}
		},
		brml_swap: initialize_swap_module(root_key),
	}
}

fn initialize_swap_module(sudo: AccountId) -> Option<SwapConfig> {
	/*
	This list is each token for aUSD.
	Accroding to the weight to calculate how many token will be added to the pool.
	For example, if aUSD has 10000 in the pool, DOT has to be added 10000 / (300 * dot_amount) = 15 / 15 =>
	so dot_amount = 10000 / 300 = 33.3333
	aUSD 10000
	DOT 300 aUSD
	vDOT 3 aUSD
	KSM 8.6 aUSD
	vKSM 0.086 aUSD
	EOS 2.62 aUSD
	vEOS 0.0262 aUSD
	*/
	let all_pool_token = 1000 * DOLLARS;
	let count_of_supported_tokens = 7u8;
	let global_pool = {
		let pool = vec![
			(TokenSymbol::aUSD, 10000 * DOLLARS, 15),
			(TokenSymbol::DOT, (33.333_333_333_333f64 * DOLLARS as f64) as Balance, 15), // 33.333_333_333_333
			(TokenSymbol::vDOT, (2222.222222222222f64 * DOLLARS as f64) as Balance, 10), // 2222.222222222222
			(TokenSymbol::KSM, (1550.3875968992247f64 * DOLLARS as f64) as Balance, 20), // 1550.3875968992247
			(TokenSymbol::vKSM, (155038.75968992253f64 * DOLLARS as f64) as Balance, 20), // 155038.7596899225
			(TokenSymbol::EOS, (2544.529262086514f64 * DOLLARS as f64) as Balance, 10), // 2544.529262086514
			(TokenSymbol::vEOS, (254452.9262086514f64 * DOLLARS as f64) as Balance, 10), // 254452.9262086514
		];
		(pool, 0)
	};
	let user_pool = {
		let pool = vec![
			(TokenSymbol::aUSD, 10000 * DOLLARS),
			(TokenSymbol::DOT, (33.333_333_333_333f64 * DOLLARS as f64) as Balance),
			(TokenSymbol::vDOT, (2222.222222222222f64 * DOLLARS as f64) as Balance),
			(TokenSymbol::KSM, (1550.3875968992247f64 * DOLLARS as f64) as Balance),
			(TokenSymbol::vKSM, (155038.75968992253f64 * DOLLARS as f64) as Balance),
			(TokenSymbol::EOS, (2544.529262086514f64 * DOLLARS as f64) as Balance),
			(TokenSymbol::vEOS, (254452.9262086514f64 * DOLLARS as f64) as Balance),
		];
		vec![(sudo, (pool, all_pool_token))]
	};
	let swap_fee = 150;
	let exit_fee = 0;
	let total_weight = global_pool.0.iter().map(|p| p.2).collect();

	Some(SwapConfig {
		all_pool_token,
		count_of_supported_tokens,
		global_pool,
		user_pool,
		swap_fee,
		exit_fee,
		total_weight
	})
}

fn development_config_genesis() -> GenesisConfig {
	testnet_genesis(
		vec![
			authority_keys_from_seed("Alice"),
			authority_keys_from_seed("Bob"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
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
			serde_json::value::to_value("ASG".to_owned()).expect("The tokenSymbol cannot be convert to json value.")
		);
		Some(props)
	};
	let protocol_id = Some("bifrost-test");

	ChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		development_config_genesis,
		vec![],
		None,
		protocol_id,
		properties,
		Default::default(),
	)
}

fn local_testnet_genesis() -> GenesisConfig {
	testnet_genesis(
		vec![
			authority_keys_from_seed("Alice"),
			authority_keys_from_seed("Bob"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
	ChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		local_testnet_genesis,
		vec![],
		None,
		None,
		None,
		Default::default(),
	)
}

/// Helper function to create GenesisConfig for bifrost
pub fn bifrost_genesis(
	initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
) -> GenesisConfig {
	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: Balance = 10_000 * DOLLARS;
	const STASH: Balance = 10_000 * DOLLARS;

	GenesisConfig {
		frame_system: Some(SystemConfig {
			code: wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_balances: Some(BalancesConfig {
			balances: initial_authorities.iter()
				.map(|k| (k.0.clone(), ENDOWMENT))
				.chain(endowed_accounts.iter().cloned().map(|x| (x, STASH / 100)))
				.collect(),
		}),
		pallet_indices: Some(IndicesConfig {
			indices: initial_authorities.iter().map(|x| x.0.clone())
				.chain(endowed_accounts.iter().cloned()).
				enumerate().map(|accts| (accts.0 as u32, accts.1))
				.collect::<Vec<_>>(),
		}),
		pallet_session: Some(SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.0.clone(), session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()))
			}).collect::<Vec<_>>(),
		}),
		pallet_staking: Some(StakingConfig {
			validator_count: 30,
			minimum_validator_count: 3,
			stakers: initial_authorities[2..5].iter().map(|x| { // we need last three addresses
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
				.map(|member| (member, STASH / 100))
				.collect(),
		}),
		pallet_collective_Instance1: Some(CouncilConfig::default()),
		pallet_collective_Instance2: Some(TechnicalCommitteeConfig {
			members: initial_authorities.iter().map(|x| x.0.clone()).collect(),
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
			members: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			pot: 0,
			max_members: 999,
		}),
		pallet_vesting: Some(Default::default()),
		brml_assets: Some(AssetsConfig {
			account_assets: initialize_assets(),
			next_asset_id: 7u32, // start from 7, [0..6] has been reserved
			token_details: vec![],
			prices: vec![],
		}),
		brml_convert: Some(ConvertConfig {
			convert_price: vec![
				(TokenSymbol::DOT, DOLLARS / 100),
				(TokenSymbol::KSM, DOLLARS / 100),
				(TokenSymbol::EOS, DOLLARS / 100),
			], // initialize convert price as token = 100 * vtoken
//			pool: vec![
//				(TokenSymbol::DOT, ConvertPool::new(1, 100)),
//				(TokenSymbol::KSM, ConvertPool::new(1, 100)),
//				(TokenSymbol::EOS, ConvertPool::new(1, 100)),
//			],
		}),
		brml_bridge_eos: Some(BridgeEosConfig {
			bridge_contract_account: (b"bifrostcross".to_vec(), 3), // this eos account needs 3 signer to sign a trade
			notary_keys: initial_authorities[2..5].iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			// root_key has the privilege to sign cross transaction
			cross_chain_privilege: [(root_key.clone(), true)].iter().cloned().collect::<Vec<_>>(),
			all_crosschain_privilege: Vec::new(),
		}),
		brml_voucher: {
			if let Some(vouchers) = initialize_all_vouchers() {
				Some(VoucherConfig { voucher: vouchers })
			} else {
				None
			}
		},
		brml_swap: initialize_swap_module(root_key),
	}
}

fn initialize_all_vouchers() -> Option<Vec<(node_primitives::AccountId, node_primitives::Balance)>> {
	use std::collections::HashSet;

	let path = std::path::Path::join(
		&std::env::current_dir().ok()?,
		"bnc_vouchers.json"
	);

	if !path.exists() { return None; }
	let file = std::fs::File::open(path).ok()?;
	let reader = std::io::BufReader::new(file);

	let vouchers_str: Vec<(String, String)> = serde_json::from_reader(reader).ok()?;
	let vouchers: Vec<(node_primitives::AccountId, node_primitives::Balance)> = vouchers_str.iter().map(|v| {
		(parse_address(&v.0), v.1.parse().expect("Balance is invalid."))
	}).collect();

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

	Some(final_vouchers)
}

fn parse_address(address: impl AsRef<str>) -> AccountId {
	let decoded_ss58 = bs58::decode(address.as_ref()).into_vec().expect("decode account id failure");
	let mut data = [0u8; 32];
	data.copy_from_slice(&decoded_ss58[1..33]);

	node_primitives::AccountId::from(data)
}

/// initialize assets for specific bifrost accounts
fn initialize_assets() -> Vec<((TokenSymbol, AccountId), AccountAsset<Balance, Cost, Income>)> {
	let assets = vec![
		(
			(TokenSymbol::DOT, parse_address("5CDWwkPsyc37XdB9N5QpZosgrcqcKA48Lpb81KjDZ89W9GPm")),
			AccountAsset { balance: 5_000_000 * DOLLARS, ..Default::default() }
//			AccountAsset { balance: 5_000_000 * DOLLARS, available: 5_000_000 * DOLLARS, ..Default::default() }
		),
		(
			(TokenSymbol::KSM, parse_address("5DAQaLpQjAZKuX4F77Lb69e5qb3GtaKVLF1mdiYt5SAhXeLC")),
			AccountAsset { balance: 5_000_000 * DOLLARS, ..Default::default() }
//			AccountAsset { balance: 5_000_000 * DOLLARS, available: 5_000_000 * DOLLARS, ..Default::default() }
		),
	];
	assets
}

/// Configure genesis for bifrost test
fn bifrost_config_genesis() -> GenesisConfig {
	let initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId, ImOnlineId, AuthorityDiscoveryId)> = vec![(
		 // 5CSpDMTeczUJoZ14BuoJTAXJzF2FnWj7gwAsfredQKdvzkGL
		 hex!["10dccc17a745f12b6026fb8e8c73544ad6d0e67f1e39106a899094bcc707e034"].into(),
		 // 5CSpDMTeczUJoZ14BuoJTAXJzF2FnWj7gwAsfredQKdvzkGL
		 hex!["10dccc17a745f12b6026fb8e8c73544ad6d0e67f1e39106a899094bcc707e034"].into(),
		 // 5EZ7Ed8PNharn8PqUDVDpriCNMk54hGyXfNh3eV5z6cwBgj4
		 hex!["6e2209923b84e44d774cf692a7f4b2f67ffcddbfd1dfbf7bd6f1fb7d769aaf9d"].unchecked_into(),
		 // 5H6pFYqLatuQbnLLzKFUazX1VXjmqhnJQT6hVWVz67kaT94z
		 hex!["dec92f12684928aa042297f6d8927930b82d9ef28b1dfa1974e6a88c51c6ee75"].unchecked_into(),
		 // 5H6pFYqLatuQbnLLzKFUazX1VXjmqhnJQT6hVWVz67kaT94z
		 hex!["dec92f12684928aa042297f6d8927930b82d9ef28b1dfa1974e6a88c51c6ee75"].unchecked_into(),
		 // 5H6pFYqLatuQbnLLzKFUazX1VXjmqhnJQT6hVWVz67kaT94z
		 hex!["dec92f12684928aa042297f6d8927930b82d9ef28b1dfa1974e6a88c51c6ee75"].unchecked_into(),
	 ),(
		 // 5GCAXGqMdfzFbcGXfFWZKGJQb7NdAsx21mFSwQhmBdJVw4Mm
		 hex!["b6a16a837cad7ad3cadfd6eb3661488fdb4c419805ffd66ab1d5bdc2e7449a60"].into(),
		 // 5GCAXGqMdfzFbcGXfFWZKGJQb7NdAsx21mFSwQhmBdJVw4Mm
		 hex!["b6a16a837cad7ad3cadfd6eb3661488fdb4c419805ffd66ab1d5bdc2e7449a60"].into(),
		 // 5G2PmFJC4HDjUSZxkp18TbdyzCdbLoCVykaJJhKnUfsR2JCo
		 hex!["af2d88f20650a6048f6d67d26d27636eb7d08ee8d44d2c535946d83257d12e26"].unchecked_into(),
		 // 5DPiyVYRVUghxtYz5qPcUMAci5GPnL9sBYawqmDFp2YH76hh
		 hex!["3abda893fc4ce0c3d465ea434cf513bed824f1c2b564cf38003a72c47fda7147"].unchecked_into(),
		 // 5DPiyVYRVUghxtYz5qPcUMAci5GPnL9sBYawqmDFp2YH76hh
		 hex!["3abda893fc4ce0c3d465ea434cf513bed824f1c2b564cf38003a72c47fda7147"].unchecked_into(),
		 // 5DPiyVYRVUghxtYz5qPcUMAci5GPnL9sBYawqmDFp2YH76hh
		 hex!["3abda893fc4ce0c3d465ea434cf513bed824f1c2b564cf38003a72c47fda7147"].unchecked_into(),
	 ),(
		 // 5GGtX6U97Kb8qiCkzQhqDaspLkghK9zz42X9g8K98gubV5Zi
		 hex!["ba3bc59c52d7eaffef1a38d38ad41cca00936fcd471d1773aed308355b91600a"].into(),
		 // 5GGtX6U97Kb8qiCkzQhqDaspLkghK9zz42X9g8K98gubV5Zi
		 hex!["ba3bc59c52d7eaffef1a38d38ad41cca00936fcd471d1773aed308355b91600a"].into(),
		 // 5HXm38QXLsYNvEDXcNNLXES6r1mUUTqYaz1fmsGMyFdQ97Do
		 hex!["f1cf7fc925e4c35ab308234b51f72052a9354c71937c481bd8d128626d144de1"].unchecked_into(),
		 // 5HgpFg4DXfg2GZ5gKcRAtarF168y9SAi5zeAP7JRig2NW5Br
		 hex!["f8b788ebec50ba10e2676c6d59842dd1127b7701977d7daf3172016ac0d4632e"].unchecked_into(),
		 // 5HgpFg4DXfg2GZ5gKcRAtarF168y9SAi5zeAP7JRig2NW5Br
		 hex!["f8b788ebec50ba10e2676c6d59842dd1127b7701977d7daf3172016ac0d4632e"].unchecked_into(),
		 // 5HgpFg4DXfg2GZ5gKcRAtarF168y9SAi5zeAP7JRig2NW5Br
		 hex!["f8b788ebec50ba10e2676c6d59842dd1127b7701977d7daf3172016ac0d4632e"].unchecked_into(),
	 ),(
		 // 5D2DxHgaHafNc4cu6gi98NDwHrdRowkL1sRydtXTHbL5nDr1
		 hex!["2a57cff9e91f5ee1fedf20061cf5dec7a24ff468dd35b0c6db9a6a7639405d2f"].into(),
		 // 5D2DxHgaHafNc4cu6gi98NDwHrdRowkL1sRydtXTHbL5nDr1
		 hex!["2a57cff9e91f5ee1fedf20061cf5dec7a24ff468dd35b0c6db9a6a7639405d2f"].into(),
		 // 5GDf8Ut6d9a9qvSbLvUK7LTW5PSBH6JrBgzvsgzyhjKVMJrJ
		 hex!["b7c4f618fcc05c2f3eefeb0454ebb932f10712beca6fa4193e89ebd624133f66"].unchecked_into(),
		 // 5EtBGed7DkcURQSc3NAfQqVz6wcxgkj8wQBh6JsrjDSuvmQL
		 hex!["7cad48689d421015bb3b449a365fdbd2a2d3070df2d42f8077d8f714d88ad200"].unchecked_into(),
		 // 5EtBGed7DkcURQSc3NAfQqVz6wcxgkj8wQBh6JsrjDSuvmQL
		 hex!["7cad48689d421015bb3b449a365fdbd2a2d3070df2d42f8077d8f714d88ad200"].unchecked_into(),
		 // 5EtBGed7DkcURQSc3NAfQqVz6wcxgkj8wQBh6JsrjDSuvmQL
		 hex!["7cad48689d421015bb3b449a365fdbd2a2d3070df2d42f8077d8f714d88ad200"].unchecked_into(),
	 ), (
		 // 5GyAB9Jia3nWUMxZ34n8TNgXFaJyhXG1n5ttkuE4N7oNvPzp
		 hex!["d8f24a7af34e86ccfec746ce9546a22eee95587a448a221c880e5a3bb447d835"].into(),
		 // 5GyAB9Jia3nWUMxZ34n8TNgXFaJyhXG1n5ttkuE4N7oNvPzp
		 hex!["d8f24a7af34e86ccfec746ce9546a22eee95587a448a221c880e5a3bb447d835"].into(),
		 // 5CowAQJr7Cx6WZxV3yNRL5QrhT68sVYj94QazzQuQm8BgV2n
		 hex!["20f857f37fff5fbff14292b565bdc6aaa6a5e4e3d04d2a01b2f501dc9bda0b14"].unchecked_into(),
		 // 5DLHpKfdUCki9xYYYKCrWCVE6PfX2U1gLG7f6sGj9uHyS9MC
		 hex!["381f3b88a3bc9872c7137f8bfbd24ae039bfa5845cba51ffa2ad8e4d03d1af1a"].unchecked_into(),
		 // 5DLHpKfdUCki9xYYYKCrWCVE6PfX2U1gLG7f6sGj9uHyS9MC
		 hex!["381f3b88a3bc9872c7137f8bfbd24ae039bfa5845cba51ffa2ad8e4d03d1af1a"].unchecked_into(),
		 // 5DLHpKfdUCki9xYYYKCrWCVE6PfX2U1gLG7f6sGj9uHyS9MC
		 hex!["381f3b88a3bc9872c7137f8bfbd24ae039bfa5845cba51ffa2ad8e4d03d1af1a"].unchecked_into(),
	 )];

	// generated with secret: subkey inspect "$secret"/fir
	let root_key: AccountId = hex![
		// 5GjJNWYS6f2UQ9aiLexuB8qgjG8fRs2Ax4nHin1z1engpnNt
		"ce6072037670ca8e974fd571eae4f215a58d0bf823b998f619c3f87a911c3541"
	].into();

	let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

	bifrost_genesis(
		initial_authorities,
		root_key,
		endowed_accounts,
	)
}

/// Adapt local test as asgard test, create chain spec use the command: birost-node build-spec --chain=local > chain.json
pub fn bifrost_chainspec_config() -> ChainSpec {
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
			serde_json::value::to_value("ASG".to_owned()).expect("The tokenSymbol cannot be convert to json value.")
		);
		Some(props)
	};
	let protocol_id = Some("bifrost");

	ChainSpec::from_genesis(
		"Bifrost Asgard CC2",
		"bifrost_testnet",
		ChainType::Custom("Asgard Testnet".into()),
		bifrost_config_genesis,
		vec![
			"/dns/n1.testnet.liebi.com/tcp/30333/p2p/12D3KooWHjmfpAdrjL7EvZ7Zkk4pFmkqKDLL5JDENc7oJdeboxJJ".parse().expect("failed to parse multiaddress."),
			"/dns/n2.testnet.liebi.com/tcp/30333/p2p/12D3KooWBMjifHHUZxbQaQZS9t5jMmTDtZbugAtJ8TG9RuX4umEY".parse().expect("failed to parse multiaddress."),
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

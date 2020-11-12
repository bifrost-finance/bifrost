// Copyright 2017-2020 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Polkadot chain configurations.

// use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
// use babe_primitives::AuthorityId as BabeId;
use hex_literal::hex;
use sc_chain_spec::{ChainSpecExtension, ChainType};
use serde::{Deserialize, Serialize};
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::{traits::IdentifyAccount, Perbill};
use telemetry::TelemetryEndpoints;

use grandpa_primitives::{AuthorityId as GrandpaId};
use babe_primitives::{AuthorityId as BabeId};
use pallet_im_online::sr25519::{AuthorityId as ImOnlineId};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;

use asgard_runtime as asgard;
use bifrost_runtime as bifrost;
use rococo_runtime as rococo;
use asgard::constants::currency::BNCS as ASG;
use bifrost::constants::currency::BNCS as BNC;
use rococo_runtime::constants::currency::BNCS as ROC;

pub use node_primitives::{AccountId, AccountAsset, Balance, Cost, Income, Signature, TokenSymbol, ConvertPool};
use sp_runtime::traits::{Verify};
use asgard_runtime::constants::currency::DOLLARS;

type AccountPublic = <Signature as Verify>::Signer;

const BIFROST_STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const ASGARD_STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const ROCOCO_STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const DEFAULT_PROTOCOL_ID: &str = "bnc";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
	/// Block numbers with known hashes.
	pub fork_blocks: sc_client_api::ForkBlocks<node_primitives::Block>,
	/// Known bad block hashes.
	pub bad_blocks: sc_client_api::BadBlocks<node_primitives::Block>,
}

/// The `ChainSpec` parametrised for the bifrost runtime.
pub type BifrostChainSpec = sc_service::GenericChainSpec<bifrost::GenesisConfig, Extensions>;

/// The `ChainSpec` parametrised for the asgard runtime.
pub type AsgardChainSpec = sc_service::GenericChainSpec<asgard::GenesisConfig, Extensions>;

/// The `ChainSpec` parametrized for the rococo runtime.
pub type RococoChainSpec = sc_service::GenericChainSpec<rococo::GenesisConfig, Extensions>;

pub fn bifrost_config() -> Result<BifrostChainSpec, String> {
	BifrostChainSpec::from_json_bytes(&include_bytes!("../res/bifrost.json")[..])
}

pub fn asgard_config() -> Result<AsgardChainSpec, String> {
	AsgardChainSpec::from_json_bytes(&include_bytes!("../res/asgard.json")[..])
}

pub fn rococo_config() -> Result<BifrostChainSpec, String> {
	BifrostChainSpec::from_json_bytes(&include_bytes!("../res/rococo.json")[..])
}

fn bifrost_session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> bifrost::SessionKeys {
	bifrost::SessionKeys {
		babe,
		grandpa,
		im_online,
		authority_discovery,
	}
}

fn asgard_session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId,
) -> asgard::SessionKeys {
	asgard::SessionKeys {
		babe,
		grandpa,
		im_online,
		authority_discovery,
	}
}

fn rococo_session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	im_online: ImOnlineId,
	authority_discovery: AuthorityDiscoveryId
) -> rococo_runtime::SessionKeys {
	rococo_runtime::SessionKeys {
		babe,
		grandpa,
		im_online,
		authority_discovery,
	}
}

fn bifrost_staging_testnet_config_genesis() -> bifrost::GenesisConfig {
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
	// and
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

	let initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId, AuthorityDiscoveryId)> = vec![(
		// 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
		hex!["9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"].into(),
		// 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
		hex!["781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"].into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5Fb9ayurnxnaXj56CjmyQLBiadfRCqUbL2VWNbbe1nZU6wiC
		hex!["9becad03e6dcac03cee07edebca5475314861492cdfc96a2144a67bbe9699332"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
	),(
		// 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
		hex!["68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"].into(),
		// 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
		hex!["c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"].into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5EockCXN6YkiNCDjpqqnbcqd4ad35nU4RmA1ikM4YeRN4WcE
		hex!["7932cff431e748892fa48e10c63c17d30f80ca42e4de3921e641249cd7fa3c2f"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
	),(
		// 5DyVtKWPidondEu8iHZgi6Ffv9yrJJ1NDNLom3X9cTDi98qp
		hex!["547ff0ab649283a7ae01dbc2eb73932eba2fb09075e9485ff369082a2ff38d65"].into(),
		// 5FeD54vGVNpFX3PndHPXJ2MDakc462vBCD5mgtWRnWYCpZU9
		hex!["9e42241d7cd91d001773b0b616d523dd80e13c6c2cab860b1234ef1b9ffc1526"].into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5E1jLYfLdUQKrFrtqoKgFrRvxM3oQPMbf6DfcsrugZZ5Bn8d
		hex!["5633b70b80a6c8bb16270f82cca6d56b27ed7b76c8fd5af2986a25a4788ce440"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
	),(
		// 5HYZnKWe5FVZQ33ZRJK1rG3WaLMztxWrrNDb1JRwaHHVWyP9
		hex!["f26cdb14b5aec7b2789fd5ca80f979cef3761897ae1f37ffb3e154cbcc1c2663"].into(),
		// 5EPQdAQ39WQNLCRjWsCk5jErsCitHiY5ZmjfWzzbXDoAoYbn
		hex!["66bc1e5d275da50b72b15de072a2468a5ad414919ca9054d2695767cf650012f"].into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
		// 5DMa31Hd5u1dwoRKgC4uvqyrdK45RHv3CpwvpUC1EzuwDit4
		hex!["3919132b851ef0fd2dae42a7e734fe547af5a6b809006100f48944d7fae8e8ef"].unchecked_into(),
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

	bifrost_testnet_genesis(
		initial_authorities,
		root_key,
		Some(endowed_accounts),
	)
}

fn asgard_staging_testnet_config_genesis() -> asgard::GenesisConfig {
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
	// and
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

	let initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId, AuthorityDiscoveryId)> = vec![(
		// 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
		hex!["9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"].into(),
		// 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
		hex!["781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"].into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5Fb9ayurnxnaXj56CjmyQLBiadfRCqUbL2VWNbbe1nZU6wiC
		hex!["9becad03e6dcac03cee07edebca5475314861492cdfc96a2144a67bbe9699332"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
		// 5EZaeQ8djPcq9pheJUhgerXQZt9YaHnMJpiHMRhwQeinqUW8
		hex!["6e7e4eb42cbd2e0ab4cae8708ce5509580b8c04d11f6758dbf686d50fe9f9106"].unchecked_into(),
	),(
		// 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
		hex!["68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"].into(),
		// 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
		hex!["c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"].into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5EockCXN6YkiNCDjpqqnbcqd4ad35nU4RmA1ikM4YeRN4WcE
		hex!["7932cff431e748892fa48e10c63c17d30f80ca42e4de3921e641249cd7fa3c2f"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
		// 5DhLtiaQd1L1LU9jaNeeu9HJkP6eyg3BwXA7iNMzKm7qqruQ
		hex!["482dbd7297a39fa145c570552249c2ca9dd47e281f0c500c971b59c9dcdcd82e"].unchecked_into(),
	),(
		// 5DyVtKWPidondEu8iHZgi6Ffv9yrJJ1NDNLom3X9cTDi98qp
		hex!["547ff0ab649283a7ae01dbc2eb73932eba2fb09075e9485ff369082a2ff38d65"].into(),
		// 5FeD54vGVNpFX3PndHPXJ2MDakc462vBCD5mgtWRnWYCpZU9
		hex!["9e42241d7cd91d001773b0b616d523dd80e13c6c2cab860b1234ef1b9ffc1526"].into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5E1jLYfLdUQKrFrtqoKgFrRvxM3oQPMbf6DfcsrugZZ5Bn8d
		hex!["5633b70b80a6c8bb16270f82cca6d56b27ed7b76c8fd5af2986a25a4788ce440"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
		// 5DhKqkHRkndJu8vq7pi2Q5S3DfftWJHGxbEUNH43b46qNspH
		hex!["482a3389a6cf42d8ed83888cfd920fec738ea30f97e44699ada7323f08c3380a"].unchecked_into(),
	),(
		// 5HYZnKWe5FVZQ33ZRJK1rG3WaLMztxWrrNDb1JRwaHHVWyP9
		hex!["f26cdb14b5aec7b2789fd5ca80f979cef3761897ae1f37ffb3e154cbcc1c2663"].into(),
		// 5EPQdAQ39WQNLCRjWsCk5jErsCitHiY5ZmjfWzzbXDoAoYbn
		hex!["66bc1e5d275da50b72b15de072a2468a5ad414919ca9054d2695767cf650012f"].into(),
		// 5C4vDQxA8LTck2xJEy4Yg1hM9qjDt4LvTQaMo4Y8ne43aU6x
		hex!["00299981a2b92f878baaf5dbeba5c18d4e70f2a1fcd9c61b32ea18daf38f4378"].unchecked_into(),
		// 5DMa31Hd5u1dwoRKgC4uvqyrdK45RHv3CpwvpUC1EzuwDit4
		hex!["3919132b851ef0fd2dae42a7e734fe547af5a6b809006100f48944d7fae8e8ef"].unchecked_into(),
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

	asgard_testnet_genesis(
		initial_authorities,
		root_key,
		Some(endowed_accounts),
	)
}

pub fn bifrost_staging_testnet_config() -> BifrostChainSpec {
	let boot_nodes = vec![];
	BifrostChainSpec::from_genesis(
		"Bifrost Staging Testnet",
		"bifrost_staging_testnet",
		ChainType::Live,
		bifrost_staging_testnet_config_genesis,
		boot_nodes,
		Some(TelemetryEndpoints::new(vec![(BIFROST_STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Staging telemetry url is valid; qed")),
		None,
		None,
		Default::default(),
	)
}

pub fn asgard_staging_testnet_config() -> AsgardChainSpec {
	let boot_nodes = vec![];
	AsgardChainSpec::from_genesis(
		"Asgard Staging Testnet",
		"asgard_staging_testnet",
		ChainType::Live,
		asgard_staging_testnet_config_genesis,
		boot_nodes,
		Some(TelemetryEndpoints::new(vec![(ASGARD_STAGING_TELEMETRY_URL.to_string(), 0)])
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
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(
	seed: &str,
) -> (
	AccountId,
	AccountId,
	BabeId,
	GrandpaId,
	ImOnlineId,
	AuthorityDiscoveryId,
) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<ImOnlineId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

fn testnet_accounts() -> Vec<AccountId> {
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
}

/// Helper function to create bifrost GenesisConfig for testing
pub fn bifrost_testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> bifrost::GenesisConfig {
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);
	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: u128 = 1_000_000 * BNC;
	const STASH: u128 = 100 * BNC;

	bifrost::GenesisConfig {
		frame_system: Some(bifrost::SystemConfig {
			code: bifrost::wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_balances: Some(bifrost::BalancesConfig {
			balances: endowed_accounts.iter().cloned()
				.map(|k| (k, ENDOWMENT))
				.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
				.collect(),
		}),
		pallet_indices: Some(bifrost::IndicesConfig {
			indices: vec![],
		}),
		pallet_session: Some(bifrost::SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.0.clone(), bifrost_session_keys(
					x.2.clone(),
					x.3.clone(),
					x.4.clone(),
					x.5.clone(),
				))
			}).collect::<Vec<_>>(),
		}),
		pallet_staking: Some(bifrost::StakingConfig {
			validator_count: initial_authorities.len() as u32 * 2,
			minimum_validator_count: initial_authorities.len() as u32,
			stakers: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.1.clone(), STASH, bifrost::StakerStatus::Validator)
			}).collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			.. Default::default()
		}),
		pallet_democracy: Some(bifrost::DemocracyConfig::default()),
		pallet_elections_phragmen: Some(bifrost::ElectionsConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		}),
		pallet_collective_Instance1: Some(bifrost::CouncilConfig::default()),
		pallet_collective_Instance2: Some(bifrost::TechnicalCommitteeConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		}),
		pallet_sudo: Some(bifrost::SudoConfig {
			key: root_key.clone(),
		}),
		pallet_babe: Some(bifrost::BabeConfig {
			authorities: vec![],
		}),
		pallet_im_online: Some(bifrost::ImOnlineConfig {
			keys: vec![],
		}),
		pallet_authority_discovery: Some(bifrost::AuthorityDiscoveryConfig {
			keys: vec![],
		}),
		pallet_grandpa: Some(bifrost::GrandpaConfig {
			authorities: vec![],
		}),
		pallet_membership_Instance1: Some(Default::default()),
		pallet_treasury: Some(Default::default()),
		pallet_society: Some(bifrost::SocietyConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			pot: 0,
			max_members: 999,
		}),
		pallet_vesting: Some(Default::default()),
		brml_assets: Some(bifrost::AssetsConfig {
			account_assets: vec![],
			next_asset_id: 9u32, // start from 7, [0..6] has been reserved
			token_details: vec![],
			prices: vec![],
		}),
		brml_convert: Some(bifrost::ConvertConfig {
			convert_price: vec![
				(TokenSymbol::DOT, DOLLARS / 100),
				(TokenSymbol::KSM, DOLLARS / 100),
				(TokenSymbol::EOS, DOLLARS / 100),
				(TokenSymbol::IOST, DOLLARS / 100),
			], // initialize convert price as token = 100 * vtoken
			pool: vec![
				(TokenSymbol::DOT, ConvertPool::new(1, 100)),
				(TokenSymbol::KSM, ConvertPool::new(1, 100)),
				(TokenSymbol::EOS, ConvertPool::new(1, 100)),
				(TokenSymbol::IOST, ConvertPool::new(1, 100)),
			],
		}),
		brml_bridge_eos: Some(bifrost::BridgeEosConfig {
			bridge_contract_account: (b"bifrostcross".to_vec(), 2),
			notary_keys: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			// alice and bob have the privilege to sign cross transaction
			cross_chain_privilege: [(root_key.clone(), true)].iter().cloned().collect::<Vec<_>>(),
			all_crosschain_privilege: Vec::new(),
			cross_trade_eos_limit: 50 * DOLLARS, // 50 EOS as limit
		}),
		brml_bridge_iost: Some(bifrost::BridgeIostConfig {
			bridge_contract_account: (b"lispczz4".to_vec(), 1),
			notary_keys: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			// alice and bob have the privilege to sign cross transaction
			cross_chain_privilege: [(root_key.clone(), true)].iter().cloned().collect::<Vec<_>>(),
			all_crosschain_privilege: Vec::new(),
		}),
		brml_voucher: {
			if let Some(vouchers) = initialize_all_vouchers() {
				Some(bifrost::VoucherConfig { voucher: vouchers })
			} else {
				None
			}
		},
		// brml_swap: initialize_swap_module(root_key),
		brml_swap: None,
	}
}

/// Helper function to create asgard GenesisConfig for testing
pub fn asgard_testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ImOnlineId,
		AuthorityDiscoveryId,
	)>,
	root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> asgard::GenesisConfig {
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);
	let num_endowed_accounts = endowed_accounts.len();

	const ENDOWMENT: u128 = 1_000_000 * ASG;
	const STASH: u128 = 100 * ASG;

	asgard::GenesisConfig {
		frame_system: Some(asgard::SystemConfig {
			code: asgard::wasm_binary_unwrap().to_vec(),
			changes_trie_config: Default::default(),
		}),
		pallet_balances: Some(asgard::BalancesConfig {
			balances: endowed_accounts.iter().cloned()
				.map(|k| (k, ENDOWMENT))
				.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
				.collect(),
		}),
		pallet_indices: Some(asgard::IndicesConfig {
			indices: vec![],
		}),
		pallet_session: Some(asgard::SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.0.clone(), asgard_session_keys(
					x.2.clone(),
					x.3.clone(),
					x.4.clone(),
					x.5.clone(),
				))
			}).collect::<Vec<_>>(),
		}),
		pallet_staking: Some(asgard::StakingConfig {
			validator_count: initial_authorities.len() as u32 * 2,
			minimum_validator_count: initial_authorities.len() as u32,
			stakers: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.1.clone(), STASH, asgard::StakerStatus::Validator)
			}).collect(),
			invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
			slash_reward_fraction: Perbill::from_percent(10),
			.. Default::default()
		}),
		pallet_democracy: Some(asgard::DemocracyConfig::default()),
		pallet_elections_phragmen: Some(asgard::ElectionsConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.map(|member| (member, STASH))
				.collect(),
		}),
		pallet_collective_Instance1: Some(asgard::CouncilConfig::default()),
		pallet_collective_Instance2: Some(asgard::TechnicalCommitteeConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			phantom: Default::default(),
		}),
		pallet_sudo: Some(asgard::SudoConfig {
			key: root_key.clone(),
		}),
		pallet_babe: Some(asgard::BabeConfig {
			authorities: vec![],
		}),
		pallet_im_online: Some(asgard::ImOnlineConfig {
			keys: vec![],
		}),
		pallet_authority_discovery: Some(asgard::AuthorityDiscoveryConfig {
			keys: vec![],
		}),
		pallet_grandpa: Some(asgard::GrandpaConfig {
			authorities: vec![],
		}),
		pallet_membership_Instance1: Some(Default::default()),
		pallet_treasury: Some(Default::default()),
		pallet_society: Some(asgard::SocietyConfig {
			members: endowed_accounts.iter()
				.take((num_endowed_accounts + 1) / 2)
				.cloned()
				.collect(),
			pot: 0,
			max_members: 999,
		}),
		pallet_vesting: Some(Default::default()),
		brml_assets: Some(asgard::AssetsConfig {
			account_assets: vec![],
			next_asset_id: 9u32, // start from 7, [0..6] has been reserved
			token_details: vec![],
			prices: vec![],
		}),
		brml_convert: Some(asgard::ConvertConfig {
			convert_price: vec![
				(TokenSymbol::DOT, DOLLARS / 100),
				(TokenSymbol::KSM, DOLLARS / 100),
				(TokenSymbol::EOS, DOLLARS / 100),
				(TokenSymbol::IOST, DOLLARS / 100),
			], // initialize convert price as token = 100 * vtoken
			pool: vec![
				(TokenSymbol::DOT, ConvertPool::new(1, 100)),
				(TokenSymbol::KSM, ConvertPool::new(1, 100)),
				(TokenSymbol::EOS, ConvertPool::new(1, 100)),
				(TokenSymbol::IOST, ConvertPool::new(1, 100)),
			],
		}),
		brml_bridge_eos: Some(asgard::BridgeEosConfig {
			bridge_contract_account: (b"bifrostcross".to_vec(), 2),
			notary_keys: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			// alice and bob have the privilege to sign cross transaction
			cross_chain_privilege: [(root_key.clone(), true)].iter().cloned().collect::<Vec<_>>(),
			all_crosschain_privilege: Vec::new(),
			cross_trade_eos_limit: 50 * DOLLARS, // 50 EOS as limit
		}),
		brml_bridge_iost: Some(asgard::BridgeIostConfig {
			bridge_contract_account: (b"lispczz4".to_vec(), 1),
			notary_keys: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			// alice and bob have the privilege to sign cross transaction
			cross_chain_privilege: [(root_key.clone(), true)].iter().cloned().collect::<Vec<_>>(),
			all_crosschain_privilege: Vec::new(),
		}),
		brml_voucher: {
			if let Some(vouchers) = initialize_all_vouchers() {
				Some(asgard::VoucherConfig { voucher: vouchers })
			} else {
				None
			}
		},
		// brml_swap: initialize_swap_module(root_key),
		brml_swap: None,
	}
}

fn bifrost_development_config_genesis(_wasm_binary: &[u8]) -> bifrost::GenesisConfig {
	bifrost_testnet_genesis(
		vec![get_authority_keys_from_seed("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

fn asgard_development_config_genesis(_wasm_binary: &[u8]) -> asgard::GenesisConfig {
	asgard_testnet_genesis(
		vec![get_authority_keys_from_seed("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Bifrost development config (single validator Alice)
pub fn bifrost_development_config() -> Result<BifrostChainSpec, String> {
	let wasm_binary = bifrost::WASM_BINARY.ok_or("Bifrost development wasm not available")?;

	Ok(BifrostChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		move || bifrost_development_config_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Default::default(),
	))
}

/// Asgard development config (single validator Alice)
pub fn asgard_development_config() -> Result<AsgardChainSpec, String> {
	let wasm_binary = asgard::WASM_BINARY.ok_or("Asgard development wasm not available")?;

	Ok(AsgardChainSpec::from_genesis(
		"Development",
		"dev",
		ChainType::Development,
		move || asgard_development_config_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Default::default(),
	))
}

fn bifrost_local_testnet_genesis(_wasm_binary: &[u8]) -> bifrost::GenesisConfig {
	bifrost_testnet_genesis(
		vec![
			get_authority_keys_from_seed("Alice"),
			get_authority_keys_from_seed("Bob"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

fn asgard_local_testnet_genesis(_wasm_binary: &[u8]) -> asgard::GenesisConfig {
	asgard_testnet_genesis(
		vec![
			get_authority_keys_from_seed("Alice"),
			get_authority_keys_from_seed("Bob"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Bifrost local testnet config (multivalidator Alice + Bob)
pub fn bifrost_local_testnet_config() -> Result<BifrostChainSpec, String> {
	let wasm_binary = bifrost::WASM_BINARY.ok_or("Bifrost development wasm not available")?;

	Ok(BifrostChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		move || bifrost_local_testnet_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Default::default(),
	))
}

/// Asgard local testnet config (multivalidator Alice + Bob)
pub fn asgard_local_testnet_config() -> Result<AsgardChainSpec, String> {
	let wasm_binary = asgard::WASM_BINARY.ok_or("Asgard development wasm not available")?;

	Ok(AsgardChainSpec::from_genesis(
		"Local Testnet",
		"local_testnet",
		ChainType::Local,
		move || asgard_local_testnet_genesis(wasm_binary),
		vec![],
		None,
		Some(DEFAULT_PROTOCOL_ID),
		None,
		Default::default(),
	))
}

pub fn bifrost_chainspec_config() -> BifrostChainSpec {
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

	BifrostChainSpec::from_genesis(
		"Bifrost",
		"bifrost_mainnet",
		ChainType::Custom("Bifrost Mainnet".into()),
		bifrost_staging_testnet_config_genesis,
		vec![
			"/dns/n1.testnet.liebi.com/tcp/30333/p2p/12D3KooWHjmfpAdrjL7EvZ7Zkk4pFmkqKDLL5JDENc7oJdeboxJJ".parse().expect("failed to parse multiaddress."),
			"/dns/n2.testnet.liebi.com/tcp/30333/p2p/12D3KooWBMjifHHUZxbQaQZS9t5jMmTDtZbugAtJ8TG9RuX4umEY".parse().expect("failed to parse multiaddress."),
			"/dns/n3.testnet.liebi.com/tcp/30333/p2p/12D3KooWLt3w5tadCR5Fc7ZvjciLy7iKJ2ZHq6qp4UVmUUHyCJuX".parse().expect("failed to parse multiaddress."),
			"/dns/n4.testnet.liebi.com/tcp/30333/p2p/12D3KooWMduQkmRVzpwxJuN6MQT4ex1iP9YquzL4h5K9Ru8qMXtQ".parse().expect("failed to parse multiaddress."),
			"/dns/n5.testnet.liebi.com/tcp/30333/p2p/12D3KooWLAHZyqMa9TQ1fR7aDRRKfWt857yFMT3k2ckK9mhYT9qR".parse().expect("failed to parse multiaddress.")
		],
		Some(TelemetryEndpoints::new(vec![(BIFROST_STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Asgard Testnet telemetry url is valid; qed")),
		protocol_id,
		properties,
		Default::default(),
	)
}

pub fn asgard_chainspec_config() -> AsgardChainSpec {
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
	let protocol_id = Some("asgard");

	AsgardChainSpec::from_genesis(
		"Bifrost Asgard CC3",
		"bifrost_testnet",
		ChainType::Custom("Asgard Testnet".into()),
		asgard_staging_testnet_config_genesis,
		vec![
			"/dns/n1.testnet.liebi.com/tcp/30333/p2p/12D3KooWHjmfpAdrjL7EvZ7Zkk4pFmkqKDLL5JDENc7oJdeboxJJ".parse().expect("failed to parse multiaddress."),
			"/dns/n2.testnet.liebi.com/tcp/30333/p2p/12D3KooWBMjifHHUZxbQaQZS9t5jMmTDtZbugAtJ8TG9RuX4umEY".parse().expect("failed to parse multiaddress."),
			"/dns/n3.testnet.liebi.com/tcp/30333/p2p/12D3KooWLt3w5tadCR5Fc7ZvjciLy7iKJ2ZHq6qp4UVmUUHyCJuX".parse().expect("failed to parse multiaddress."),
			"/dns/n4.testnet.liebi.com/tcp/30333/p2p/12D3KooWMduQkmRVzpwxJuN6MQT4ex1iP9YquzL4h5K9Ru8qMXtQ".parse().expect("failed to parse multiaddress."),
			"/dns/n5.testnet.liebi.com/tcp/30333/p2p/12D3KooWLAHZyqMa9TQ1fR7aDRRKfWt857yFMT3k2ckK9mhYT9qR".parse().expect("failed to parse multiaddress.")
		],
		Some(TelemetryEndpoints::new(vec![(ASGARD_STAGING_TELEMETRY_URL.to_string(), 0)])
			.expect("Asgard Testnet telemetry url is valid; qed")),
		protocol_id,
		properties,
		Default::default(),
	)
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

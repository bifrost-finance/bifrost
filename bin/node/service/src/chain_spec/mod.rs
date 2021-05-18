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

#[cfg(feature = "with-asgard-runtime")]
pub mod asgard;
#[cfg(feature = "with-bifrost-runtime")]
pub mod bifrost;
#[cfg(feature = "with-rococo-runtime")]
pub mod rococo;

use hex_literal::hex;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use pallet_im_online::sr25519::{AuthorityId as ImOnlineId};
use grandpa_primitives::{AuthorityId as GrandpaId};
use babe_primitives::{AuthorityId as BabeId};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;

pub use node_primitives::{AccountId, AccountAsset, Balance, Signature, VtokenPool};

type AccountPublic = <Signature as Verify>::Signer;

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

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct RelayExtensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl RelayExtensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn parse_address(address: impl AsRef<str>) -> AccountId {
	let decoded_ss58 = bs58::decode(address.as_ref()).into_vec().expect("decode account id failure");
	let mut data = [0u8; 32];
	data.copy_from_slice(&decoded_ss58[1..33]);

	node_primitives::AccountId::from(data)
}

pub fn faucet_accounts() -> Vec<AccountId> {
	vec![
		hex!["a2d57b8e781327bd2853b36e6f290bd8beeaa850971c9b0789ec4969f8beb01b"].into(), // bifrost-faucet
		hex!["a272fa6e2282767b61a299e81023d44ef583c640fef99b0bafe216399775cd17"].into(),
		hex!["56f6e7bb0826cd128672ad3a03016533834123c319adc635c6db595c6f72272e"].into(),
		hex!["7e9005c247601a0d0e967f68b03f6e39e402a735ec65c20e4965c6d94a22e42f"].into(),
		hex!["f2449dfbb431a5f9e8dc7468e5f3521baff4c0125edcda746f38df5295d5fb28"].into(),
		hex!["aaa565b52ea12bf3c8d7abb79411976bccd8054c5581922acc0165ad88640f09"].into(),
		hex!["8afadc065940f22f73b745aab694b1b20cafea3d4e52adad844f581614fbdd00"].into(),
		hex!["0831325e2b4953f247db9df3f6452becbf23d8f7f806c0396ad853cb3c284d06"].into(),
		hex!["7ea84934a575487fb02c44e01f4488c2f242cdbf48052630780dcd8ac567950c"].into(),
		hex!["ee05492a82cb982392aad78f7e6f6fff56eaee4988fd9961ebb84e177dd6526d"].into(), // bifrost-faucet
		hex!["7435653321694ee115e8cea8c8e117c0b6703b6fb91298b6df5adeef7679a46f"].into(), // danny testing account
		hex!["263c78393f33b23cd23f3211726b2316e950910749d20c1552ea6972091a645e"].into(), // jianbo testing account
		hex!["803feefeab8e5c81c3d268038b6c494d3018714fc8c5d08cf027111fd8114b06"].into(), // tieqiao testing account
		hex!["8898ffd2cb04fb751655ede7bc0081b6b6ebe13cd0bdee5bbb9273e6dcc9b91c"].into(), // tyrone testing account
	]
}
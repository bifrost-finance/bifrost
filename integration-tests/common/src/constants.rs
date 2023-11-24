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

// Substrate
use beefy_primitives::ecdsa_crypto::AuthorityId as BeefyId;
use grandpa::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, storage::Storage, Pair, Public};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	BuildStorage, MultiSignature, Perbill,
};

// Cumulus
use bifrost_kusama_runtime::{
	constants::currency::DOLLARS, DefaultBlocksPerRound, InflationInfo, Range,
};
use bifrost_primitives::DOT;
use parachains_common::{AccountId, AssetHubPolkadotAuraId, AuraId, Balance, BlockNumber};
use polkadot_parachain_primitives::primitives::{HeadData, ValidationCode};
use polkadot_primitives::{AssignmentId, ValidatorId};
use polkadot_runtime_parachains::{
	configuration::HostConfiguration,
	paras::{ParaGenesisArgs, ParaKind},
};
use polkadot_service::chain_spec::get_authority_keys_from_seed_no_beefy;
use xcm;

pub const XCM_V2: u32 = 3;
pub const XCM_V3: u32 = 2;
pub const REF_TIME_THRESHOLD: u64 = 33;
pub const PROOF_SIZE_THRESHOLD: u64 = 33;

type AccountPublic = <MultiSignature as Verify>::Signer;

/// Helper function to generate a crypto pair from seed
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed.
fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub mod accounts {
	use super::*;
	use sp_runtime::traits::AccountIdConversion;
	pub const ALICE: &str = "Alice";
	pub const BOB: &str = "Bob";
	pub const CHARLIE: &str = "Charlie";
	pub const DAVE: &str = "Dave";
	pub const EVE: &str = "Eve";
	pub const FERDIE: &str = "Ferdei";
	pub const ALICE_STASH: &str = "Alice//stash";
	pub const BOB_STASH: &str = "Bob//stash";

	pub fn init_balances() -> Vec<AccountId> {
		vec![
			get_account_id_from_seed::<sr25519::Public>(ALICE),
			get_account_id_from_seed::<sr25519::Public>(BOB),
			get_account_id_from_seed::<sr25519::Public>(CHARLIE),
			get_account_id_from_seed::<sr25519::Public>(DAVE),
			get_account_id_from_seed::<sr25519::Public>(EVE),
			get_account_id_from_seed::<sr25519::Public>(ALICE_STASH),
			get_account_id_from_seed::<sr25519::Public>(BOB_STASH),
			bifrost_kusama_runtime::TreasuryPalletId::get().into_account_truncating(),
		]
	}
}

pub mod collators {
	use super::*;

	pub fn invulnerables_asset_hub_polkadot() -> Vec<(AccountId, AssetHubPolkadotAuraId)> {
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<AssetHubPolkadotAuraId>("Alice"),
			),
			(
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_from_seed::<AssetHubPolkadotAuraId>("Bob"),
			),
		]
	}

	pub fn invulnerables() -> Vec<(AccountId, AuraId)> {
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<AuraId>("Alice"),
			),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), get_from_seed::<AuraId>("Bob")),
		]
	}

	pub fn candidates() -> Vec<(AccountId, Balance)> {
		vec![
			(get_account_id_from_seed::<sr25519::Public>("Alice"), 10000 * 1_000_000_000_000u128),
			(get_account_id_from_seed::<sr25519::Public>("Bob"), 10000 * 1_000_000_000_000u128),
		]
	}

	pub fn delegations() -> Vec<(AccountId, AccountId, Balance)> {
		vec![
			(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				5000 * 1_000_000_000_000u128,
			),
			(
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				5000 * 1_000_000_000_000u128,
			),
		]
	}

	pub fn inflation_config() -> InflationInfo<Balance> {
		fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
			use parachain_staking::inflation::{perbill_annual_to_perbill_round, BLOCKS_PER_YEAR};
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
			expect: Range {
				min: 100_000 * DOLLARS,
				ideal: 200_000 * DOLLARS,
				max: 500_000 * DOLLARS,
			},
			// annual inflation
			annual,
			round: to_round_inflation(annual),
		}
	}
}

pub mod validators {
	use super::*;

	pub fn initial_authorities() -> Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ImOnlineId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
	)> {
		vec![get_authority_keys_from_seed_no_beefy("Alice")]
	}
}

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
// Polkadot
pub mod polkadot {
	use super::*;
	pub const ED: Balance = polkadot_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
	const STASH: u128 = 100 * polkadot_runtime_constants::currency::UNITS;

	pub fn get_host_config() -> HostConfiguration<BlockNumber> {
		HostConfiguration {
			max_upward_queue_count: 10,
			max_upward_queue_size: 51200,
			max_upward_message_size: 51200,
			max_upward_message_num_per_candidate: 10,
			max_downward_message_size: 51200,
			hrmp_sender_deposit: 100_000_000_000,
			hrmp_recipient_deposit: 100_000_000_000,
			hrmp_channel_max_capacity: 1000,
			hrmp_channel_max_message_size: 102400,
			hrmp_channel_max_total_size: 102400,
			hrmp_max_parachain_outbound_channels: 30,
			hrmp_max_parachain_inbound_channels: 30,
			..Default::default()
		}
	}

	fn session_keys(
		babe: BabeId,
		grandpa: GrandpaId,
		im_online: ImOnlineId,
		para_validator: ValidatorId,
		para_assignment: AssignmentId,
		authority_discovery: AuthorityDiscoveryId,
	) -> polkadot_runtime::SessionKeys {
		polkadot_runtime::SessionKeys {
			babe,
			grandpa,
			im_online,
			para_validator,
			para_assignment,
			authority_discovery,
		}
	}

	pub fn genesis() -> Storage {
		let genesis_config = polkadot_runtime::RuntimeGenesisConfig {
			system: polkadot_runtime::SystemConfig {
				code: polkadot_runtime::WASM_BINARY.unwrap().to_vec(),
				..Default::default()
			},
			balances: polkadot_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.cloned()
					.map(|k| (k, ED * 4096))
					.collect(),
			},
			session: polkadot_runtime::SessionConfig {
				keys: validators::initial_authorities()
					.iter()
					.map(|x| {
						(
							x.0.clone(),
							x.0.clone(),
							polkadot::session_keys(
								x.2.clone(),
								x.3.clone(),
								x.4.clone(),
								x.5.clone(),
								x.6.clone(),
								x.7.clone(),
							),
						)
					})
					.collect::<Vec<_>>(),
			},
			staking: polkadot_runtime::StakingConfig {
				validator_count: validators::initial_authorities().len() as u32,
				minimum_validator_count: 1,
				stakers: validators::initial_authorities()
					.iter()
					.map(|x| {
						(x.0.clone(), x.1.clone(), STASH, polkadot_runtime::StakerStatus::Validator)
					})
					.collect(),
				invulnerables: validators::initial_authorities()
					.iter()
					.map(|x| x.0.clone())
					.collect(),
				force_era: pallet_staking::Forcing::ForceNone,
				slash_reward_fraction: Perbill::from_percent(10),
				..Default::default()
			},
			babe: polkadot_runtime::BabeConfig {
				authorities: Default::default(),
				epoch_config: Some(polkadot_runtime::BABE_GENESIS_EPOCH_CONFIG),
				..Default::default()
			},
			configuration: polkadot_runtime::ConfigurationConfig { config: get_host_config() },
			paras: polkadot_runtime::ParasConfig {
				paras: vec![(
					bifrost_polkadot::PARA_ID.into(),
					ParaGenesisArgs {
						genesis_head: HeadData::default(),
						validation_code: ValidationCode(
							bifrost_polkadot_runtime::WASM_BINARY.unwrap().to_vec(),
						),
						para_kind: ParaKind::Parachain,
					},
				)],
				..Default::default()
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Kusama
pub mod kusama {
	use super::*;
	pub const ED: Balance = kusama_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
	use kusama_runtime_constants::currency::UNITS as KSM;
	const ENDOWMENT: u128 = 1_000_000 * KSM;
	const STASH: u128 = 100 * KSM;

	pub fn get_host_config() -> HostConfiguration<BlockNumber> {
		HostConfiguration {
			max_upward_queue_count: 10,
			max_upward_queue_size: 51200,
			max_upward_message_size: 51200,
			max_upward_message_num_per_candidate: 10,
			max_downward_message_size: 51200,
			hrmp_sender_deposit: 5_000_000_000_000,
			hrmp_recipient_deposit: 5_000_000_000_000,
			hrmp_channel_max_capacity: 1000,
			hrmp_channel_max_message_size: 102400,
			hrmp_channel_max_total_size: 102400,
			hrmp_max_parachain_outbound_channels: 30,
			hrmp_max_parachain_inbound_channels: 30,
			..Default::default()
		}
	}

	fn session_keys(
		babe: BabeId,
		grandpa: GrandpaId,
		im_online: ImOnlineId,
		para_validator: ValidatorId,
		para_assignment: AssignmentId,
		authority_discovery: AuthorityDiscoveryId,
		beefy: BeefyId,
	) -> kusama_runtime::SessionKeys {
		kusama_runtime::SessionKeys {
			babe,
			grandpa,
			im_online,
			para_validator,
			para_assignment,
			authority_discovery,
			beefy,
		}
	}

	pub fn genesis() -> Storage {
		let genesis_config = kusama_runtime::RuntimeGenesisConfig {
			system: kusama_runtime::SystemConfig {
				code: kusama_runtime::WASM_BINARY.unwrap().to_vec(),
				..Default::default()
			},
			balances: kusama_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.map(|k: &AccountId| (k.clone(), ENDOWMENT))
					.collect(),
			},
			session: kusama_runtime::SessionConfig {
				keys: validators::initial_authorities()
					.iter()
					.map(|x| {
						(
							x.0.clone(),
							x.0.clone(),
							kusama::session_keys(
								x.2.clone(),
								x.3.clone(),
								x.4.clone(),
								x.5.clone(),
								x.6.clone(),
								x.7.clone(),
								get_from_seed::<BeefyId>("Alice"),
							),
						)
					})
					.collect::<Vec<_>>(),
			},
			staking: kusama_runtime::StakingConfig {
				validator_count: validators::initial_authorities().len() as u32,
				minimum_validator_count: 1,
				stakers: validators::initial_authorities()
					.iter()
					.map(|x| {
						(x.0.clone(), x.1.clone(), STASH, kusama_runtime::StakerStatus::Validator)
					})
					.collect(),
				invulnerables: validators::initial_authorities()
					.iter()
					.map(|x| x.0.clone())
					.collect(),
				force_era: pallet_staking::Forcing::NotForcing,
				slash_reward_fraction: Perbill::from_percent(10),
				..Default::default()
			},
			babe: kusama_runtime::BabeConfig {
				authorities: Default::default(),
				epoch_config: Some(kusama_runtime::BABE_GENESIS_EPOCH_CONFIG),
				..Default::default()
			},
			configuration: kusama_runtime::ConfigurationConfig { config: get_host_config() },
			paras: kusama_runtime::ParasConfig {
				paras: vec![(
					bifrost_kusama::PARA_ID.into(),
					ParaGenesisArgs {
						genesis_head: HeadData::default(),
						validation_code: ValidationCode(
							bifrost_kusama_runtime::WASM_BINARY.unwrap().to_vec(),
						),
						para_kind: ParaKind::Parachain,
					},
				)],
				..Default::default()
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Asset Hub Polkadot
pub mod bifrost_polkadot {
	use super::*;
	use crate::BOB;
	pub const PARA_ID: u32 = 2030;
	pub const ED: Balance = 10_000_000_000;

	pub fn genesis() -> Storage {
		let genesis_config = bifrost_polkadot_runtime::RuntimeGenesisConfig {
			system: bifrost_polkadot_runtime::SystemConfig {
				code: bifrost_polkadot_runtime::WASM_BINARY
					.expect("WASM binary was not build, please build it!")
					.to_vec(),
				..Default::default()
			},
			balances: bifrost_polkadot_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.cloned()
					.map(|k| (k, ED * 4096))
					.collect(),
			},
			parachain_info: bifrost_polkadot_runtime::ParachainInfoConfig {
				parachain_id: PARA_ID.into(),
				..Default::default()
			},
			collator_selection: bifrost_polkadot_runtime::CollatorSelectionConfig {
				invulnerables: collators::invulnerables_asset_hub_polkadot()
					.iter()
					.cloned()
					.map(|(acc, _)| acc)
					.collect(),
				candidacy_bond: ED * 16,
				..Default::default()
			},
			session: bifrost_polkadot_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                                    // account id
							acc,                                            // validator id
							bifrost_polkadot_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			polkadot_xcm: bifrost_polkadot_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			tokens: bifrost_polkadot_runtime::TokensConfig {
				balances: vec![(
					get_account_id_from_seed::<sr25519::Public>(BOB),
					DOT,
					10 * 10_000_000_000u128,
				)],
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Asset Hub Polkadot
pub mod bifrost_kusama {
	use super::*;
	use crate::ALICE;
	use bifrost_primitives::KSM;

	pub const PARA_ID: u32 = 2001;
	pub const ED: Balance = 1_000_000_000_000;

	pub fn genesis() -> Storage {
		let genesis_config = bifrost_kusama_runtime::RuntimeGenesisConfig {
			system: bifrost_kusama_runtime::SystemConfig {
				code: bifrost_kusama_runtime::WASM_BINARY
					.expect("WASM binary was not build, please build it!")
					.to_vec(),
				..Default::default()
			},
			balances: bifrost_kusama_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.cloned()
					.map(|k| (k, ED * 10_000_000))
					.collect(),
			},
			parachain_info: bifrost_kusama_runtime::ParachainInfoConfig {
				parachain_id: PARA_ID.into(),
				..Default::default()
			},
			session: bifrost_kusama_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                                  // account id
							acc,                                          // validator id
							bifrost_kusama_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			// parachain_staking: bifrost_kusama_runtime::ParachainStakingConfig {
			// 	candidates: collators::candidates(),
			// 	delegations: collators::delegations(),
			// 	inflation_config: collators::inflation_config(),
			// },
			polkadot_xcm: bifrost_kusama_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			tokens: bifrost_kusama_runtime::TokensConfig {
				balances: vec![(
					get_account_id_from_seed::<sr25519::Public>(ALICE),
					KSM,
					10 * 1_000_000_000_000u128,
				)],
			},
			asset_registry: bifrost_kusama_runtime::AssetRegistryConfig {
				currency: vec![(KSM, 10_000_000, None)],
				vcurrency: vec![],
				vsbond: vec![],
				phantom: Default::default(),
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

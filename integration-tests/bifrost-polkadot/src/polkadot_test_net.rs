// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

//! Relay chain and parachains emulation.

use bifrost_runtime_common::dollar;
use cumulus_primitives_core::ParaId;
use frame_support::{assert_ok, traits::GenesisBuild};
use polkadot_primitives::v2::{BlockNumber, MAX_CODE_SIZE, MAX_POV_SIZE};
use polkadot_runtime_parachains::configuration::HostConfiguration;
use sp_runtime::traits::AccountIdConversion;
use xcm_emulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain, TestExt};

use crate::polkadot_integration_tests::*;

pub const DECIMAL_18: u128 = 1_000_000_000_000_000_000;
pub const DECIMAL_10: u128 = 10_000_000_000;

pub const DOT_TOKEN_ID: u8 = 0;
pub const GLMR_TOKEN_ID: u8 = 1;
pub const DOT_MINIMAL_BALANCE: u128 = 1_000_000;
pub const GLMR_MINIMAL_BALANCE: u128 = 1_000_000_000_000;

decl_test_relay_chain! {
	pub struct PolkadotNet {
		Runtime = polkadot_runtime::Runtime,
		XcmConfig = polkadot_runtime::xcm_config::XcmConfig,
		new_ext = polkadot_ext(),
	}
}

decl_test_parachain! {
	pub struct Bifrost {
		Runtime = Runtime,
		Origin = Origin,
		XcmpMessageHandler = bifrost_polkadot_runtime ::XcmpQueue,
		DmpMessageHandler = bifrost_polkadot_runtime::DmpQueue,
		new_ext = para_ext(2010),
	}
}

decl_test_parachain! {
	pub struct Sibling {
		Runtime = Runtime,
		Origin = Origin,
		XcmpMessageHandler = bifrost_polkadot_runtime ::XcmpQueue,
		DmpMessageHandler = bifrost_polkadot_runtime::DmpQueue,
		new_ext = para_ext(2000),
	}
}

decl_test_network! {
	pub struct TestNet {
		relay_chain = PolkadotNet,
		parachains = vec![
			(2010, Bifrost),
			(2000, Sibling),
		],
	}
}

fn default_parachains_host_configuration() -> HostConfiguration<BlockNumber> {
	HostConfiguration {
		minimum_validation_upgrade_delay: 5,
		validation_upgrade_cooldown: 5u32,
		validation_upgrade_delay: 5,
		code_retention_period: 1200,
		max_code_size: MAX_CODE_SIZE,
		max_pov_size: MAX_POV_SIZE,
		max_head_data_size: 32 * 1024,
		group_rotation_frequency: 20,
		chain_availability_period: 4,
		thread_availability_period: 4,
		max_upward_queue_count: 8,
		max_upward_queue_size: 1024 * 1024,
		max_downward_message_size: 1024,
		ump_service_total_weight: 4 * 1_000_000_000,
		max_upward_message_size: 1024 * 50,
		max_upward_message_num_per_candidate: 5,
		hrmp_sender_deposit: 0,
		hrmp_recipient_deposit: 0,
		hrmp_channel_max_capacity: 8,
		hrmp_channel_max_total_size: 8 * 1024,
		hrmp_max_parachain_inbound_channels: 4,
		hrmp_max_parathread_inbound_channels: 4,
		hrmp_channel_max_message_size: 1024 * 1024,
		hrmp_max_parachain_outbound_channels: 4,
		hrmp_max_parathread_outbound_channels: 4,
		hrmp_max_message_num_per_candidate: 5,
		dispute_period: 6,
		no_show_slots: 2,
		n_delay_tranches: 25,
		needed_approvals: 2,
		relay_vrf_modulo_samples: 2,
		zeroth_delay_tranche_width: 0,
		..Default::default()
	}
}

pub fn polkadot_ext() -> sp_io::TestExternalities {
	use polkadot_runtime::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![
			(AccountId::from(ALICE), 100 * dollar::<bifrost_polkadot_runtime::Runtime>(DOT)),
			(
				ParaId::from(2010u32).into_account_truncating(),
				2 * dollar::<bifrost_polkadot_runtime::Runtime>(DOT),
			),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	polkadot_runtime_parachains::configuration::GenesisConfig::<Runtime> {
		config: default_parachains_host_configuration(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	<pallet_xcm::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
		&pallet_xcm::GenesisConfig { safe_xcm_version: Some(2) },
		&mut t,
	)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn para_ext(parachain_id: u32) -> sp_io::TestExternalities {
	ExtBuilder::default()
		.balances(vec![(AccountId::from(ALICE), RelayCurrencyId::get(), 10 * 10_000_000_000)])
		.parachain_id(parachain_id)
		.build()
}

pub fn register_token2_asset() {
	Bifrost::execute_with(|| {
		// Token
		let items = vec![
			(
				CurrencyId::Token2(DOT_TOKEN_ID),
				b"Polkadot DOT".to_vec(),
				b"DOT".to_vec(),
				10u8,
				DOT_MINIMAL_BALANCE,
			),
			(
				CurrencyId::Token2(GLMR_TOKEN_ID),
				b"Moonbeam Native Token".to_vec(),
				b"GLMR".to_vec(),
				18u8,
				GLMR_MINIMAL_BALANCE,
			),
		];
		for (currency_id, metadata) in
			items.iter().map(|(currency_id, name, symbol, decimals, minimal_balance)| {
				(
					currency_id,
					bifrost_asset_registry::AssetMetadata {
						name: (*name.clone()).to_vec(),
						symbol: (*symbol.clone()).to_vec(),
						decimals: *decimals,
						minimal_balance: *minimal_balance,
					},
				)
			}) {
			assert_ok!(AssetRegistry::do_register_metadata(*currency_id, &metadata));
		}
	});
}

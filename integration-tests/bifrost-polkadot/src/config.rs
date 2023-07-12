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

use frame_support::{traits::GenesisBuild, weights::Weight};
use polkadot_primitives::{BlockNumber, MAX_CODE_SIZE, MAX_POV_SIZE};
use polkadot_runtime_parachains::configuration::HostConfiguration;
use xcm_emulator::{decl_test_network, decl_test_parachain, decl_test_relay_chain};

pub use codec::Encode;
pub use node_primitives::*;
pub use orml_traits::{Change, GetByKey, MultiCurrency};
pub use sp_runtime::{
	traits::{BadOrigin, Convert, Zero},
	BuildStorage, DispatchError, DispatchResult, FixedPointNumber, MultiAddress,
};

pub const ALICE: [u8; 32] = [0u8; 32];
pub const BOB: [u8; 32] = [1u8; 32];

pub use bifrost_imports::*;

mod bifrost_imports {
	pub use bifrost_polkadot_runtime::{
		create_x2_multilocation, AccountId, AssetRegistry, Balance, Balances, BifrostCrowdloanId,
		BlockNumber, Currencies, CurrencyId, ExistentialDeposit, NativeCurrencyId, OriginCaller,
		ParachainInfo, ParachainSystem, Proxy, RelayCurrencyId, Runtime, RuntimeCall, RuntimeEvent,
		RuntimeOrigin, Salp, Scheduler, Session, SlotLength, Slp, System, Tokens, TreasuryPalletId,
		Utility, Vesting, XTokens,
	};
	pub use frame_support::parameter_types;
	pub use sp_runtime::traits::AccountIdConversion;
}

pub const GLMR_DECIMALS: u128 = 1_000_000_000_000_000_000;
pub const DOT_DECIMALS: u128 = 10_000_000_000;

pub const DOT_TOKEN_ID: u8 = 0;
pub const GLMR_TOKEN_ID: u8 = 1;

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
		RuntimeOrigin = RuntimeOrigin,
		XcmpMessageHandler = bifrost_polkadot_runtime ::XcmpQueue,
		DmpMessageHandler = bifrost_polkadot_runtime::DmpQueue,
		new_ext = para_ext(),
	}
}

decl_test_network! {
	pub struct TestNet {
		relay_chain = PolkadotNet,
		parachains = vec![
			(2030, Bifrost),
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
		ump_service_total_weight: Weight::from_parts(4 * 1_000_000_000, 0),
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

// Polkadot initial configuration
pub fn polkadot_ext() -> sp_io::TestExternalities {
	use polkadot_runtime::{Runtime, System};

	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(AccountId::from(ALICE), 100 * DOT_DECIMALS)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	polkadot_runtime_parachains::configuration::GenesisConfig::<Runtime> {
		config: default_parachains_host_configuration(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	<pallet_xcm::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
		&pallet_xcm::GenesisConfig { safe_xcm_version: Some(3) },
		&mut t,
	)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

// Bifrost initial configuration
pub fn para_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

	bifrost_asset_registry::GenesisConfig::<Runtime> {
		currency: vec![
			(
				CurrencyId::Token2(DOT_TOKEN_ID),
				DOT_DECIMALS / 1000,
				Some((String::from("Polkadot DOT"), String::from("DOT"), 10u8)),
			),
			(
				CurrencyId::Token2(GLMR_TOKEN_ID),
				GLMR_DECIMALS / 1000_000,
				Some((String::from("Moonbeam Native Token"), String::from("GLMR"), 18u8)),
			),
		],
		vcurrency: vec![CurrencyId::VToken2(DOT_TOKEN_ID)],
		vsbond: vec![],
		phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();
	orml_tokens::GenesisConfig::<Runtime> {
		balances: vec![(
			AccountId::from(ALICE),
			CurrencyId::Token2(DOT_TOKEN_ID),
			10 * DOT_DECIMALS,
		)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	pallet_membership::GenesisConfig::<Runtime, pallet_membership::Instance1> {
		members: Default::default(),
		phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	<parachain_info::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
		&parachain_info::GenesisConfig { parachain_id: 2030.into() },
		&mut t,
	)
	.unwrap();

	<pallet_xcm::GenesisConfig as GenesisBuild<Runtime>>::assimilate_storage(
		&pallet_xcm::GenesisConfig { safe_xcm_version: Some(3) },
		&mut t,
	)
	.unwrap();

	<bifrost_salp::GenesisConfig<Runtime> as GenesisBuild<Runtime>>::assimilate_storage(
		&bifrost_salp::GenesisConfig { initial_multisig_account: Some(AccountId::new(ALICE)) },
		&mut t,
	)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

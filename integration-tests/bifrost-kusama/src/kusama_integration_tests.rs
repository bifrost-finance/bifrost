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

pub use codec::Encode;
use frame_support::{traits::GenesisBuild, weights::constants::*};
pub use node_primitives::*;
pub use orml_traits::{Change, GetByKey, MultiCurrency};
pub use sp_runtime::{
	traits::{AccountIdConversion, BadOrigin, Convert, Zero},
	BuildStorage, DispatchError, DispatchResult, FixedPointNumber, MultiAddress,
};

pub const ALICE: [u8; 32] =
	hex_literal::hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"];
pub const BOB: [u8; 32] =
	hex_literal::hex!["8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"];
pub const KUSAMA_ALICE_STASH_ACCOUNT: [u8; 32] =
	hex_literal::hex!["be5ddb1579b72e84524fc29e78609e3caf42e85aa118ebfe0b0ad404b5bdd25f"];
pub const KUSAMA_BOB_STASH_ACCOUNT: [u8; 32] =
	hex_literal::hex!["fe65717dad0447d715f660a0a58411de509b42e6efb8375f562f58a554d5860e"];
// pub const CATHI: [u8; 32] = [2u8; 32];

pub const KSM_DECIMALS: u128 = 1000_000_000_000;
pub const BNC_DECIMALS: u128 = 1000_000_000_000;
//pub const CONTRIBUTON_INDEX: MessageId = [0; 32];
const SECONDS_PER_YEAR: u32 = 31557600;
const SECONDS_PER_BLOCK: u32 = 12;
pub const BLOCKS_PER_YEAR: u32 = SECONDS_PER_YEAR / SECONDS_PER_BLOCK;

pub use bifrost_imports::*;
use bifrost_kusama_runtime::{ExistentialDeposit, NativeCurrencyId};

mod bifrost_imports {
	pub use bifrost_kusama_runtime::{
		create_x2_multilocation, AccountId, AssetRegistry, Balance, Balances, BifrostCrowdloanId,
		BlockNumber, Currencies, CurrencyId, ExistentialDeposit, ExistentialDeposits,
		NativeCurrencyId, OriginCaller, ParachainInfo, ParachainSystem, Proxy, RelayCurrencyId,
		Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, Salp, Scheduler, Session, SlotLength,
		Slp, SlpEntrancePalletId, System, Tokens, TreasuryPalletId, Utility, Vesting, XTokens,
		XcmConfig, XcmInterface,
	};
	pub use bifrost_runtime_common::dollar;
	pub use frame_support::parameter_types;
	pub use sp_runtime::traits::AccountIdConversion;
}

pub fn get_all_module_accounts() -> Vec<AccountId> {
	vec![BifrostCrowdloanId::get().into_account_truncating()]
}

pub struct ExtBuilder {
	balances: Vec<(AccountId, CurrencyId, Balance)>,
	parachain_id: u32,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { balances: vec![], parachain_id: 2001 }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, balances: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	#[allow(dead_code)]
	pub fn parachain_id(mut self, parachain_id: u32) -> Self {
		self.parachain_id = parachain_id;
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		let native_currency_id = NativeCurrencyId::get();
		let existential_deposit = ExistentialDeposit::get();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self
				.balances
				.clone()
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == native_currency_id)
				.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
				.chain(get_all_module_accounts().iter().map(|x| (x.clone(), existential_deposit)))
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			balances: self
				.balances
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id != native_currency_id)
				.collect::<Vec<_>>(),
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
			&parachain_info::GenesisConfig { parachain_id: self.parachain_id.into() },
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

		<bifrost_asset_registry::GenesisConfig<Runtime> as GenesisBuild<Runtime>>::assimilate_storage(
			&bifrost_asset_registry::GenesisConfig {
				currency: vec![
					(CurrencyId::Token(TokenSymbol::DOT), 100_000_000, None),
					(CurrencyId::Token(TokenSymbol::KSM), 10_000_000, None),
				],
				vcurrency: vec![],
				vsbond: vec![],
				phantom: Default::default()
			},
			&mut t,
		)
			.unwrap();

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

#[test]
fn sanity_check_weight_per_time_constants_are_as_expected() {
	assert_eq!(WEIGHT_REF_TIME_PER_SECOND, 1_000_000_000_000);
	assert_eq!(WEIGHT_REF_TIME_PER_MILLIS, WEIGHT_REF_TIME_PER_SECOND / 1000);
	assert_eq!(WEIGHT_REF_TIME_PER_MICROS, WEIGHT_REF_TIME_PER_MILLIS / 1000);
	assert_eq!(WEIGHT_REF_TIME_PER_NANOS, WEIGHT_REF_TIME_PER_MICROS / 1000);
}

#[test]
fn parachain_subaccounts_are_unique() {
	sp_io::TestExternalities::default().execute_with(|| {
		let parachain: AccountId = ParachainInfo::parachain_id().into_account_truncating();
		assert_eq!(
			parachain,
			hex_literal::hex!["7061726164000000000000000000000000000000000000000000000000000000"]
				.into()
		);
	});
}

// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

#![cfg(test)]

use crate as bancor;

use frame_support::{construct_runtime, parameter_types, traits::GenesisBuild};
use node_primitives::{CurrencyId, TokenSymbol, Balance};
use sp_core::H256;
use sp_runtime::{
	testing::Header,AccountId32,
	traits::{BlakeTwo256, IdentityLookup},
};

pub type AccountId = AccountId32;
pub const VSDOT_BASE_SUPPLY: Balance = 10_000;
pub const VSKSM_BASE_SUPPLY: Balance = 1_000_000;
pub const DOT: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
pub const VSDOT: CurrencyId = CurrencyId::VSToken(TokenSymbol::DOT);
pub const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
pub const VSKSM: CurrencyId = CurrencyId::VSToken(TokenSymbol::KSM);
pub const ALICE: AccountId = AccountId32::new([0u8; 32]);
pub const BOB: AccountId = AccountId32::new([1u8; 32]);

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Assets: orml_tokens::{Pallet, Call, Config<T>, Storage, Event<T>},
		Bancor: bancor::{Pallet, Call, Config<T>, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Call = Call;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

parameter_types! {
	pub const MaxLocks: u32 = 999;
}

impl orml_tokens::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type Amount = i128;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = MaxLocks;
}

parameter_types! {
	pub const InterventionPercentage: u128 = 75;
}

impl bancor::Config for Test {
	type Event = Event;
	type MultiCurrenciesHandler = Assets;
	type InterventionPercentage = InterventionPercentage;
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn one_thousand_for_alice_n_bob(self) -> Self {
		self.balances(vec![
			(ALICE, KSM, 1_000),
			(ALICE, DOT, 1_000),
			(ALICE, VSKSM, 1_000),
			(ALICE, VSDOT, 1_000),
			(BOB, KSM, 1_000),
			(BOB, DOT, 1_000),
			(BOB, VSKSM, 1_000),
			(BOB, VSDOT, 1_000),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.unwrap();

		orml_tokens::GenesisConfig::<Test> {
			endowed_accounts: self.endowed_accounts
		}
		.assimilate_storage(&mut t)
		.unwrap();

		crate::GenesisConfig::<Test> {
			bancor_pools: vec![
				(CurrencyId::Token(TokenSymbol::DOT), VSDOT_BASE_SUPPLY),
				(CurrencyId::Token(TokenSymbol::KSM), VSKSM_BASE_SUPPLY),
			],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}

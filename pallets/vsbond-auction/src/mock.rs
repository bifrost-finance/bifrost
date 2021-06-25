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

use crate as vsbond_auction;

use crate::{AccountIdOf, BalanceOf, CurrencyIdOf};
use frame_support::{construct_runtime, parameter_types, traits::GenesisBuild};
use node_primitives::{Amount, AssetId, Balance};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		OrmlAssets: orml_tokens::{Pallet, Call, Storage, Event<T>},
		VSBondAuction: vsbond_auction::{Pallet, Call, Storage, Event<T>},
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
	type AccountId = u64;
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
	pub ExistentialDeposits: |_currency_id: AssetId| -> Balance {
		0
	};
}

parameter_types! {
	pub const MaxLocks: u32 = 999;
}

impl orml_tokens::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = AssetId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = MaxLocks;
}

parameter_types! {
	pub const MaxInTradeOrderNum: u32 = 5;
}

impl vsbond_auction::Config for Test {
	type Event = Event;

	type MultiCurrency = orml_tokens::Pallet<Self>;
	type MaximumOrderInTrade = MaxInTradeOrderNum;
}

// mockup runtime
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut fs_gc = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	orml_tokens::GenesisConfig::<Test> {
		endowed_accounts: vec![
			(ACCOUNT_ALICE, CURRENCY_OWNED_BY_ALICE, BALANCE_OWNED),
			(ACCOUNT_BRUCE, CURRENCY_OWNED_BY_BRUCE, BALANCE_OWNED),
		],
	}
	.assimilate_storage(&mut fs_gc)
	.unwrap();

	fs_gc.into()
}

pub(crate) const ACCOUNT_ALICE: AccountIdOf<Test> = 1;
pub(crate) const ACCOUNT_BRUCE: AccountIdOf<Test> = 2;
pub(crate) const CURRENCY_OWNED_BY_ALICE: CurrencyIdOf<Test> = 1;
pub(crate) const CURRENCY_OWNED_BY_BRUCE: CurrencyIdOf<Test> = 2;
pub(crate) const BALANCE_OWNED: BalanceOf<Test> = 1_000;
pub(crate) const BALANCE_EXCEEDED: BalanceOf<Test> = BALANCE_OWNED + 1;

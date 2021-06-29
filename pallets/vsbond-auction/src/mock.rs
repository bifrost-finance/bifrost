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
use vsbond_auction::*;

use frame_support::{construct_runtime, parameter_types, traits::GenesisBuild};
use node_primitives::{Amount, Balance, CurrencyId, TokenSymbol};
use sp_core::H256;
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;
type BlockNumber = u32;

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
	pub const BlockHashCount: BlockNumber = 250;
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
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Call = Call;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
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
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = MaxLocks;
}

parameter_types! {
	pub const InvoicingCurrency: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const MaximumOrderInTrade: u32 = 5;
	pub const MinimumSupply: Balance = 0;
}

impl vsbond_auction::Config for Test {
	type Event = Event;

	type InvoicingCurrency = InvoicingCurrency;
	type MaximumOrderInTrade = MaximumOrderInTrade;
	type MinimumSupply = MinimumSupply;
	type MultiCurrency = orml_tokens::Pallet<Self>;
}

// mockup runtime
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut fs_gc = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();

	orml_tokens::GenesisConfig::<Test> {
		endowed_accounts: vec![
			(ACCOUNT_ALICE, TOKEN, BALANCE_TOKEN),
			(ACCOUNT_ALICE, VSBOND, BALANCE_VSBOND),
			(ACCOUNT_BRUCE, TOKEN, BALANCE_TOKEN),
			(ACCOUNT_BRUCE, VSBOND, BALANCE_VSBOND),
		],
	}
	.assimilate_storage(&mut fs_gc)
	.unwrap();

	fs_gc.into()
}

pub(crate) const TOKEN: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
pub(crate) const PARA_ID: ParaId = 3000;
pub(crate) const FIRST_SLOT: LeasePeriod = 0;
pub(crate) const LAST_SLOT: LeasePeriod = 100;
pub(crate) const VSBOND: CurrencyId =
	CurrencyId::VSBond(TokenSymbol::KSM, PARA_ID, FIRST_SLOT, LAST_SLOT);
pub(crate) const ACCOUNT_ALICE: AccountId = 1;
pub(crate) const ACCOUNT_BRUCE: AccountId = 2;
pub(crate) const BALANCE_VSBOND: Balance = 1_000;
pub(crate) const BALANCE_TOKEN: Balance = 1_000;
pub(crate) const UNIT_PRICE: Balance = 1;

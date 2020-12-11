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

//! Test utilities

#![cfg(test)]

use frame_support::{impl_outer_origin, impl_outer_event, parameter_types, traits::{OnInitialize, OnFinalize}};
use sp_core::H256;
use sp_runtime::{traits::{BlakeTwo256, IdentityLookup}, testing::Header};
use super::*;

impl_outer_origin! {
	pub enum Origin for Test {}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = ();
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
}

parameter_types! {
	pub const SettlementPeriod: u64 = 24 * 60 * 10;
}

impl Config for Test {
	type Event = TestEvent;
	type Balance = u128;
	type AssetId = u32;
	type Price = u64;
	type Convert = u128;
	type AssetRedeem = ();
	type FetchConvertPrice = ();
	type WeightInfo = ();
}

mod assets {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Test {
		system<T>,
		assets<T>,
	}
}

pub type Assets = Module<Test>;
pub type AssetsError = Error<Test>;
pub type System = system::Module<Test>;

// simulate block production
#[allow(dead_code)]
pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		Assets::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		Assets::on_initialize(System::block_number());
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

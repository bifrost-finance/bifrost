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
use sp_runtime::{Perbill, traits::{BlakeTwo256, IdentityLookup}, testing::Header};
use super::*;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_event! {
	pub enum TestEvent for Test {
		system<T>,
		brml_convert,
		assets<T>,
	}
}

mod brml_convert {
	pub use crate::Event;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl assets::Trait for Test {
	type Event = TestEvent;
	type Balance = u64;
	type AssetId = u32;
	type Price = u64;
	type Cost = u64;
	type Income = u64;
	type Convert = u64;
	type AssetRedeem = ();
	type FetchConvertPrice = ();
}

impl system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type ModuleToIndex = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
}

parameter_types! {
	pub const ConvertDuration: u64 = 24 * 60 * 10;
}

impl crate::Trait for Test {
	type ConvertPrice = u64;
	type RatePerBlock = u64;
	type Event = TestEvent;
	type AssetTrait = Assets;
	type Balance = u64;
	type AssetId = u32;
	type Cost = u64;
	type Income = u64;
	type ConvertDuration = ConvertDuration;
}

pub type Convert = crate::Module<Test>;
pub type System = system::Module<Test>;
pub type Assets = assets::Module<Test>;

pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		Convert::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		Convert::on_initialize(System::block_number());
	}
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

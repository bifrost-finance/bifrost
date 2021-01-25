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

use frame_support::{impl_outer_origin, parameter_types};
use sp_core::H256;
use sp_runtime::{Perbill, traits::{BlakeTwo256, IdentityLookup}, testing::Header};
use super::*;

impl_outer_origin! {
	pub enum Origin for Test {}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl system::Config for Test {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type ModuleToIndex = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}

impl brml_assets::Config for Test {
	type Event = ();
	type Balance = u64;
	type AssetId = u32;
	type ClearingHandler = Settlement;
	type AssetRedeem = ();
}

parameter_types! {
	pub const SettlementPeriod: u64 = 24 * 60 * 10;
}

impl Config for Test {
	type Event = ();
	type SettlementId = u32;
	type SettlementPeriod = SettlementPeriod;
	type Duration = u128;
	type AssetIssue = Assets;
}

pub type Settlement = Module<Test>;
pub type Assets = brml_assets::Module<Test>;
pub type System = system::Module<Test>;

pub fn new_test_ext() -> runtime_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

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

use frame_system as system;
use frame_support::{impl_outer_origin, impl_outer_event, parameter_types};
use sp_core::H256;
use sp_runtime::{traits::{BlakeTwo256, IdentityLookup}, testing::Header};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_event! {
	pub enum TestEvent for Test {
		system<T>,
	}
}


parameter_types! {
	pub const BlockHashCount: u64 = 250;
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
	type Event = TestEvent;
	type Version = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type PalletInfo = ();
	type BlockHashCount = ();
	type MaximumBlockWeight = ();
	type MaximumExtrinsicWeight = ();
	type MaximumBlockLength = ();
	type AvailableBlockRatio = ();
}

parameter_types! {
	pub const ConvertDuration: u64 = 24 * 60 * 10;
}

impl crate::Config for Test {
	type AssetId = u32;
	type Balance = u64;
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

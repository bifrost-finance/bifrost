// Copyright 2019 Liebi Technologies.
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

use frame_support::{impl_outer_origin, impl_outer_dispatch, impl_outer_event, parameter_types};
use substrate_primitives::H256;
use sr_primitives::{Perbill, traits::{BlakeTwo256, IdentityLookup}, testing::{Header, TestXt}};
use super::*;

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		bridge::Bridge,
	}
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;
type SubmitTransaction = system::offchain::TransactionSubmitter<(), Call, Extrinsic>;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

impl system::Trait for Test {
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}

parameter_types! {
	pub const SettlementPeriod: u64 = 24 * 60 * 10;
}

impl Trait for Test {
	type Event = TestEvent;
	type Balance = u64;
	type AssetId = u32;
	type AssetIssue = ();
	type Call = Call;
	type SubmitTransaction = SubmitTransaction;
}

mod bridge {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Test {
		bridge,
	}
}

pub type Bridge = Module<Test>;
pub type System = system::Module<Test>;

pub fn new_test_ext() -> sr_io::TestExternalities {
	let t = system::GenesisConfig::default().build_storage::<Test>().unwrap();
	t.into()
}

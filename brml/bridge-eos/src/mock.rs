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

#![cfg(test)]

use support::{impl_outer_origin, impl_outer_dispatch, impl_outer_event, parameter_types};
use sp_core::{
	H256,
	Perbill,
	testing::{Header, TestXt},
	traits::{BlakeTwo256, IdentityLookup, OnInitialize, OnFinalize},
};
use super::*;

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		bridge_eos::BridgeEos,
	}
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;
type SubmitTransaction = system::offchain::TransactionSubmitter<(), Call, Extrinsic>;

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_event! {
	pub enum TestEvent for Test {
		bridge_eos,
	}
}

mod bridge_eos {
	pub use crate::Event;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 4 * 1024 * 1024;
	pub const MaximumBlockLength: u32 = 4 * 1024 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
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
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}

impl crate::Trait for Test {
	type Event = TestEvent;
	type Balance = u64;
	type Precision = u32;
	type BridgeAssetFrom = ();
	type Call = Call;
	type SubmitTransaction = SubmitTransaction;
}

pub type BridgeEos = crate::Module<Test>;
pub type System = system::Module<Test>;

// simulate block production
pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		BridgeEos::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		BridgeEos::on_initialize(System::block_number());
	}
}

// mockup runtime
pub(crate) fn new_test_ext() -> sr_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

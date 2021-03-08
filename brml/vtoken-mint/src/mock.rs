// Copyright 2019-2021 Liebi Technologies.
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
use frame_support::{impl_outer_origin, impl_outer_event, parameter_types, traits::{OnInitialize, OnFinalize}};
use sp_core::H256;
use sp_runtime::{traits::{BlakeTwo256, IdentityLookup}, testing::Header};

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_event! {
	pub enum TestEvent for Test {
		system<T>,
		brml_vtoken_mint,
		assets<T>,
	}
}

mod brml_vtoken_mint {
	pub use crate::Event;
}

impl assets::Config for Test {
	type Event = TestEvent;
	type Balance = u64;
	type AssetId = u32;
	type Price = u64;
	type VtokenMint = u64;
	type AssetRedeem = ();
	type FetchVtokenMintPrice = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
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
	type Version = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type PalletInfo = ();
	type SS58Prefix = ();
}

parameter_types! {
	pub const VtokenMintDuration: u64 = 24 * 60 * 10;
}

impl crate::Config for Test {
	type MintPrice = u64;
	type Event = TestEvent;
	type AssetTrait = Assets;
	type Balance = u64;
	type AssetId = u32;
	type VtokenMintDuration = VtokenMintDuration;
	type WeightInfo = ();
}

pub type VtokenMint = crate::Module<Test>;
pub type System = system::Module<Test>;
pub type Assets = assets::Module<Test>;

pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 1 {
			System::on_finalize(System::block_number());
		}
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		VtokenMint::on_initialize(System::block_number());
	}
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

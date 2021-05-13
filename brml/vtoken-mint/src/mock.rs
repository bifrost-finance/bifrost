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

use crate as pallet_vtoken_mint;
use frame_support::{construct_runtime, parameter_types, traits::{OnInitialize, OnFinalize}};
use sp_core::H256;
use sp_runtime::{traits::{BlakeTwo256, IdentityLookup}, testing::Header};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Assets: assets::{Module, Call, Storage, Event<T>},
		VtokenMint: pallet_vtoken_mint::{Module, Call, Config<T>, Storage, Event},
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
}

impl assets::Config for Test {
	type Event = Event;
	type Balance = u64;
	type AssetId = u32;
	type Price = u64;
	type VtokenMint = u64;
	type AssetRedeem = ();
	type FetchVtokenMintPrice = ();
	type WeightInfo = ();
}


parameter_types! {
	pub const VtokenMintDuration: u64 = 24 * 60 * 10;
}

impl crate::Config for Test {
	type MintPrice = u64;
	type Event = Event;
	type AssetTrait = Assets;
	type Balance = u64;
	type AssetId = u32;
	type VtokenMintDuration = VtokenMintDuration;
	type WeightInfo = ();
}

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
	frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

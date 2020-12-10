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

#![cfg(test)]

use super::*;
use crate as bid;
use frame_support::{
	impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types,
	traits::{OnFinalize, OnInitialize},
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Permill,Perbill
};
use node_primitives::{Balance, AssetId, BlockNumber};

use frame_system as system;

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		brml_bid::Bid,
	}
}

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Test;

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_event! {
	pub enum TestEvent for Test {
		system<T>, // the alias of the package/crate
		bid<T>,
		assets<T>,
	}
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 4 * 1024 * 1024;
	pub const MaximumBlockLength: u32 = 4 * 1024 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
	pub const UncleGenerations: u32 = 5;
}

impl system::Trait for Test {
	//配置各个type的类型，再加上上面定义好的常量。类型+常量
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
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
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type BaseCallFilter = ();
	type MaximumExtrinsicWeight = MaximumBlockWeight;
	type SystemWeightInfo = ();
	type PalletInfo = ();
}

parameter_types! {
	pub const TokenOrderROIListLength: u8 = 200u8;
	pub const MinimumVotes: u64 = 100;
	pub const MaximumVotes: u64 = 50_000;
	pub const BlocksPerYear: BlockNumber = 60 * 60 * 24 * 365 / 6;
	pub const MaxProposalNumberForBidder: u32 = 5;
}

impl crate::Trait for Test {
	type Event = TestEvent;

	type AssetId = AssetId;
	type Cost = Balance;
	type Income = Balance;
	type AssetTrait = Assets;
	type BiddingOrderId = u64;
	type EraId = u64;
	type Balance = Balance;
	type TokenOrderROIListLength = TokenOrderROIListLength ;
	type MinimumVotes = MinimumVotes;
	type MaximumVotes = MaximumVotes;
	type BlocksPerYear = BlocksPerYear;
	type MaxProposalNumberForBidder = MaxProposalNumberForBidder;
}

impl assets::Trait for Test {
	type Event = TestEvent;
	type Balance = Balance;
	type AssetId = AssetId;
	type Price = Balance;
	type Convert = Balance;
	type AssetRedeem = ();
	type FetchConvertPrice = ();
	type WeightInfo = ();
}

pub type Bid = bid::Module<Test>;  // package/crate name
pub type System = frame_system::Module<Test>;
pub type Assets = assets::Module<Test>;

// simulate block production
pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		Bid::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		Bid::on_initialize(System::block_number());
	}
}

// mockup runtime
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap()
		.into()
}

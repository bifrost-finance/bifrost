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

#![cfg(test)]

// use super::*;
use crate as pallet_bid;
use frame_support::{parameter_types, construct_runtime, traits::{OnFinalize, OnInitialize}};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use node_primitives::{Balance, AssetId, BlockNumber};

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
		Bid: pallet_bid::{Module, Call, Storage, Event<T>},
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
	type Balance = Balance;
	type AssetId = AssetId;
	type Price = Balance;
	type VtokenMint = Balance;
	type AssetRedeem = ();
	type FetchVtokenMintPrice = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const TokenOrderROIListLength: u8 = 200u8;
	pub const MinimumVotes: u64 = 100;
	pub const MaximumVotes: u64 = 50_000;
	pub const BlocksPerYear: BlockNumber = 60 * 60 * 24 * 365 / 6;
	pub const MaxProposalNumberForBidder: u32 = 5;
	pub const ROIPermillPrecision: u32 = 100;
}

impl crate::Config for Test {
	type Event = Event;
	type AssetId = AssetId;
	type AssetTrait = Assets;
	type BiddingOrderId = u64;
	type EraId = u64;
	type Balance = Balance;
	type TokenOrderROIListLength = TokenOrderROIListLength ;
	type MinimumVotes = MinimumVotes;
	type MaximumVotes = MaximumVotes;
	type BlocksPerYear = BlocksPerYear;
	type MaxProposalNumberForBidder = MaxProposalNumberForBidder;
	type ROIPermillPrecision = ROIPermillPrecision;
}


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
	frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap()
		.into()
}

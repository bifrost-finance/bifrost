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

use super::*;
use crate as swap;
use frame_support::{
	impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types,
	traits::{OnFinalize, OnInitialize},
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

use frame_system as system;

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		brml_swap::Swap,
	}
}

#[derive(Clone, Eq, PartialEq, Debug, Default)]
pub struct Test;

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_event! {
	pub enum TestEvent for Test {
		system<T>,
		swap<T>,
		assets<T>,
	}
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
	type Index = u64;
	type Call = Call;
	type BlockNumber = u64;
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
	type SS58Prefix = ();
}

parameter_types! {
	pub const MaximumSwapInRatio: u64 = 2;
	pub const MinimumPassedInPoolTokenShares: u64 = 2;
	pub const MinimumSwapFee: u64 = 1; // 0.001%
	pub const MaximumSwapFee: u64 = 10_000; // 10%
	pub const FeePrecision: u64 = 10_000;
	pub const WeightPrecision: u64 = 100_000;
	pub const BNCAssetId: u32 = 0;
	pub const InitialPoolSupply: u64 = 1_000;

	pub const NumberOfSupportedTokens: u8 = 8;
	pub const BonusClaimAgeDenominator: u64 = 14_400;
	pub const MaximumPassedInPoolTokenShares: u64 = 1_000_000;
}

impl crate::Config for Test {
	type Event = TestEvent;
	type Fee = u64;
	type AssetId = u32;
	type PoolId = u32;
	type Balance = u64;
	type AssetTrait = Assets;
	type PoolWeight = u64;
	type MaximumSwapInRatio = MaximumSwapInRatio;
	type MinimumPassedInPoolTokenShares = MinimumPassedInPoolTokenShares;
	type MinimumSwapFee = MinimumSwapFee;
	type MaximumSwapFee = MaximumSwapFee;
	type FeePrecision = FeePrecision;
	type WeightPrecision = WeightPrecision;
	type BNCAssetId = BNCAssetId;
	type InitialPoolSupply = InitialPoolSupply;
	type NumberOfSupportedTokens = NumberOfSupportedTokens;
	type BonusClaimAgeDenominator = BonusClaimAgeDenominator;
	type MaximumPassedInPoolTokenShares = MaximumPassedInPoolTokenShares;
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

pub type Swap = swap::Module<Test>;
pub type System = frame_system::Module<Test>;
pub type Assets = assets::Module<Test>;

// simulate block production
pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		Swap::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		Swap::on_initialize(System::block_number());
	}
}

// mockup runtime
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap()
		.into()
}

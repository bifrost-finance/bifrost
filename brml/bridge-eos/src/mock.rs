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

use codec::Decode;
use frame_support::{
	impl_outer_origin, impl_outer_dispatch, impl_outer_event, parameter_types, ConsensusEngineId,
	traits::{OnInitialize, OnFinalize, FindAuthor}
};
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, IdentityLookup},
};
use super::*;

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		bridge_eos::BridgeEos,
	}
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;

#[derive(Clone, Eq, PartialEq, Encode, Decode)]
pub struct Test;

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_event! {
	pub enum TestEvent for Test {
		system<T>,
		bridge_eos<T>,
		assets<T>,
		convert,
	}
}

mod bridge_eos {
	pub use crate::Event;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
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
	type Version = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type PalletInfo = ();
}


pub const TEST_ID: ConsensusEngineId = [1, 2, 3, 4];

pub struct AuthorGiven;

impl FindAuthor<u64> for AuthorGiven {
	fn find_author<'a, I>(digests: I) -> Option<u64>
		where I: 'a + IntoIterator<Item=(ConsensusEngineId, &'a [u8])>
	{
		for (id, data) in digests {
			if id == TEST_ID {
				return u64::decode(&mut &data[..]).ok();
			}
		}

		None
	}
}

parameter_types! {
	pub const UncleGenerations: u64 = 5;
}

impl pallet_authorship::Config for Test {
	type FindAuthor = AuthorGiven;
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = ();
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test where
	Call: From<LocalCall>,
{
	type Extrinsic = Extrinsic;
	type OverarchingCall = Call;
}

impl From<u64> for sr25519::AuthorityId {
	fn from(_: u64) -> Self {
		Default::default()
	}
}

impl crate::Config for Test {
	type AuthorityId = sr25519::AuthorityId;
	type Event = TestEvent;
	type Balance = u64;
	type AssetId = u32;
	type Precision = u32;
	type BridgeAssetFrom = ();
	type Call = Call;
	type AssetTrait = Assets;
	type FetchConvertPool = Convert;
	type WeightInfo = ();
}

impl assets::Config for Test {
	type Event = TestEvent;
	type Balance = u64;
	type AssetId = u32;
	type Price = u64;
	type Convert = u64;
	type AssetRedeem = ();
	type FetchConvertPrice = Convert;
	type WeightInfo = ();
}

parameter_types! {
	pub const ConvertDuration: u64 = 24 * 60 * 10;
}

impl convert::Config for Test {
	type ConvertPrice = u64;
	type RatePerBlock = u64;
	type Event = TestEvent;
	type AssetTrait = Assets;
	type Balance = u64;
	type AssetId = u32;
	type ConvertDuration = ConvertDuration;
	type WeightInfo = ();
}

pub type BridgeEos = crate::Module<Test>;
pub type Authorship = pallet_authorship::Module<Test>;
pub type System = frame_system::Module<Test>;
pub type Assets = assets::Module<Test>;
pub type Convert = convert::Module<Test>;

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
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	GenesisConfig::<Test> {
		bridge_contract_account: (b"bifrostcross".to_vec(), 2),
		notary_keys: vec![1u64, 2u64],
		cross_chain_privilege: vec![(1u64, true)],
		all_crosschain_privilege: Vec::new(),
		cross_trade_eos_limit: 50,
		eos_asset_id: 6,
	}.assimilate_storage(&mut t).unwrap();
	t.into()
}

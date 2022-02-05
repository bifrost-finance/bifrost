// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg(test)]

use codec::Decode;
use frame_support::{
	construct_runtime, parameter_types,
	traits::{FindAuthor, OnFinalize, OnInitialize},
	ConsensusEngineId, PalletId,
};
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, IdentityLookup},
};

use super::*;
use crate as pallet_bridge_eos;
pub type AccountId = u64;
pub type BlockNumber = u64;
pub type Amount = i128;
pub type Balance = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		VtokenMint: vtoken_mint::{Pallet, Call, Config<T>, Storage, Event<T>},
		Authorship: pallet_authorship::{Pallet, Call, Storage},
		BridgeEos: pallet_bridge_eos::{Pallet, Call, Config<T>, Storage, Event<T>},

		Currencies: orml_currencies::{Pallet, Call, Storage, Event<T>},
		Assets: orml_tokens::{Pallet, Call, Storage, Event<T>, Config<T>},
		PalletBalances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		MinterReward: minter_reward::{Pallet, Storage, Event<T>},

	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type Call = Call;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
}

pub type AdaptedBasicCurrency =
	orml_currencies::BasicCurrencyAdapter<Test, PalletBalances, Amount, BlockNumber>;

impl orml_currencies::Config for Test {
	type Event = Event;
	type MultiCurrency = Assets;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type MaxLocks = ();
	type WeightInfo = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}
impl orml_tokens::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type Amount = i128;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = orml_tokens::TransferDust<Test, ()>;
}

pub const TEST_ID: ConsensusEngineId = [1, 2, 3, 4];

pub struct AuthorGiven;

impl FindAuthor<u64> for AuthorGiven {
	fn find_author<'a, I>(digests: I) -> Option<u64>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
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

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
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
	type Event = Event;
	type Balance = u64;
	type Precision = u32;
	type Call = Call;
	type CurrenciesHandler = Currencies;
	type VtokenPoolHandler = VtokenMint;
	type WeightInfo = ();
}

parameter_types! {
	// 3 hours(1800 blocks) as an era
	pub const VtokenMintDuration: u32 = 3 * 60 * 1;
	pub const StakingPalletId: PalletId = PalletId(*b"staking ");
}

impl vtoken_mint::Config for Test {
	type Event = Event;
	type MultiCurrency = Assets;
	type PalletId = StakingPalletId;
	type MinterReward = MinterReward;
	type DEXOperations = ();
	type RandomnessSource = RandomnessCollectiveFlip;
	type WeightInfo = ();
}

parameter_types! {
	pub const TwoYear: u32 = 1 * 365 * 2;
	pub const RewardPeriod: u32 = 50;
	pub const MaximumExtendedPeriod: u32 = 500;
	pub const ShareWeightPalletId: PalletId = PalletId(*b"weight  ");
}

impl minter_reward::Config for Test {
	type Event = Event;
	type MultiCurrency = Assets;
	type TwoYear = TwoYear;
	type PalletId = ShareWeightPalletId;
	type RewardPeriod = RewardPeriod;
	type MaximumExtendedPeriod = MaximumExtendedPeriod;
	// type DEXOperations = ZenlinkProtocol;
	type DEXOperations = ();
	type ShareWeight = Balance;
}

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
	pallet_bridge_eos::GenesisConfig::<Test> {
		bridge_contract_account: (b"bifrostcross".to_vec(), 2),
		notary_keys: vec![1u64, 2u64],
		cross_chain_privilege: vec![(1u64, true)],
		all_crosschain_privilege: Vec::new(),
		cross_trade_eos_limit: 50,
		eos_asset_id: CurrencyId::Token(TokenSymbol::EOS),
	}
	.assimilate_storage(&mut t)
	.unwrap();
	t.into()
}

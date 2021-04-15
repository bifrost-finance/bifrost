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
#![allow(non_upper_case_globals)]

use crate::{self as vtoken_mint};
use frame_support::{parameter_types, traits::GenesisBuild};
use node_primitives::{Balance, CurrencyId, TokenSymbol};
use sp_core::H256;
use sp_runtime::{
	testing::Header, AccountId32, ModuleId, Permill,
	traits::{BlakeTwo256, IdentityLookup, Zero},
};

pub type AccountId = AccountId32;
pub const BNC: CurrencyId = CurrencyId::Token(TokenSymbol::ASG);
pub const aUSD: CurrencyId = CurrencyId::Token(TokenSymbol::aUSD);
pub const DOT: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
pub const vDOT: CurrencyId = CurrencyId::Token(TokenSymbol::vDOT);
pub const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
pub const vKSM: CurrencyId = CurrencyId::Token(TokenSymbol::vKSM);
pub const ALICE: AccountId = AccountId32::new([0u8; 32]);
pub const BOB: AccountId = AccountId32::new([1u8; 32]);
pub const CENTS: Balance = 1_000_000_000_000 / 100;

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Assets: orml_tokens::{Module, Call, Storage, Event<T>, Config<T>},
		PalletBalances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		VtokenMint: vtoken_mint::{Module, Call, Storage, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},
		MinterReward: brml_minter_reward::{Module, Storage, Event<T>},
	}
);

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}
impl frame_system::Config for Runtime {
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
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Module<Runtime>;
	type MaxLocks = ();
	type WeightInfo = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		0
	};
}
impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = i128;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = orml_tokens::TransferDust<Runtime, ()>;
}

parameter_types! {
	pub const TwoYear: u32 = 1 * 365 * 2;
	pub const RewardPeriod: u32 = 50;
	pub const MaximumExtendedPeriod: u32 = 500;
	pub const ShareWeightModuleId: ModuleId = ModuleId(*b"weight  ");
}

impl brml_minter_reward::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Assets;
	type TwoYear = TwoYear;
	type ModuleId = ShareWeightModuleId;
	type RewardPeriod = RewardPeriod;
	type MaximumExtendedPeriod = MaximumExtendedPeriod;
	// type DEXOperations = ZenlinkProtocol;
	type DEXOperations = ();
	type ShareWeight = Balance;
}

parameter_types! {
	// 3 hours(1800 blocks) as an era
	pub const VtokenMintDuration: u32 = 3 * 60 * 1;
	pub const StakingModuleId: ModuleId = ModuleId(*b"staking ");
}
orml_traits::parameter_type_with_key! {
	pub RateOfInterestEachBlock: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			&CurrencyId::Token(TokenSymbol::DOT) => 1 * CENTS,
			&CurrencyId::Token(TokenSymbol::ETH) => 7 * CENTS,
			_ => Zero::zero(),
		}
	};
}
impl crate::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Assets;
	type ModuleId = StakingModuleId;
	type MinterReward = MinterReward;
	type DEXOperations = ();
	type RandomnessSource = RandomnessCollectiveFlip;
	type WeightInfo = ();
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			endowed_accounts: vec![],
		}
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn one_hundred_for_alice_n_bob(self) -> Self {
		self.balances(vec![
			(ALICE, BNC, 100),
			(BOB, BNC, 100),
			(ALICE, DOT, 100),
			(ALICE, vDOT, 400),
			(BOB, DOT, 100),
			(BOB, KSM, 100),
		])
	}

	pub fn zero_for_alice_n_bob(self) -> Self {
		self.balances(vec![
			(ALICE, BNC, 100),
			(BOB, BNC, 100),
			(ALICE, DOT, 0),
			(ALICE, vDOT, 100),
			(BOB, DOT, 0),
			(BOB, KSM, 100),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self
				.endowed_accounts
				.clone()
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == BNC)
				.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			endowed_accounts: self.endowed_accounts
		}
		.assimilate_storage(&mut t)
		.unwrap();

		crate::GenesisConfig::<Runtime> {
			pools: vec![],
			staking_lock_period: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 28 * 1),
				(CurrencyId::Token(TokenSymbol::ETH), 14 * 1)
			],
			rate_of_interest_each_block: vec![
				(CurrencyId::Token(TokenSymbol::DOT), 019_025_875_190), // 100000.0 * 0.148/(365*24*600)
				(CurrencyId::Token(TokenSymbol::ETH), 009_512_937_595) // 50000.0 * 0.082/(365*24*600)
			],
			yield_rate: vec![
				(CurrencyId::Token(TokenSymbol::DOT), Permill::from_perthousand(148)),// 14.8%
				(CurrencyId::Token(TokenSymbol::ETH), Permill::from_perthousand(82)) // 8.2%
			]
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}

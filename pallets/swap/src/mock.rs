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

use frame_support::{
	parameter_types,
	traits::{OnFinalize, OnInitialize},
};
use node_primitives::{CurrencyId, TokenSymbol};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

use super::*;
use crate as pallet_swap;
pub type BlockNumber = u64;
pub type Amount = i128;
pub type Balance = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Storage, Event<T>},
		Assets: orml_tokens::{Pallet, Storage, Event<T>},
		Balances: pallet_balances ::{Pallet, Call, Storage, Event<T>},
		Swap: pallet_swap::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}
impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<u64>;
	type AccountId = u64;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockNumber = u64;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = Header;
	type Index = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
}

pub type AdaptedBasicCurrency =
	orml_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Test {
	type Event = Event;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type MultiCurrency = Assets;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = u64;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

impl orml_tokens::Config for Test {
	type Amount = i128;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ();
	type OnDust = orml_tokens::TransferDust<Test, ()>;
	type WeightInfo = ();
}

parameter_types! {
	pub const MaximumSwapInRatio: u8 = 2;
	pub const MinimumPassedInPoolTokenShares: u64 = 2;
	pub const MinimumSwapFee: u64 = 1; // 0.001%
	pub const MaximumSwapFee: u64 = 10_000; // 10%
	pub const FeePrecision: u64 = 100_000;
	pub const WeightPrecision: u64 = 100_000;
	pub const BNCAssetId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
	pub const InitialPoolSupply: u64 = 1_000;

	pub const NumberOfSupportedTokens: u8 = 8;
	pub const BonusClaimAgeDenominator: u64 = 14_400;
	pub const MaximumPassedInPoolTokenShares: u64 = 1_000_000;
}

impl crate::Config for Test {
	type BNCAssetId = BNCAssetId;
	type Balance = u64;
	type BonusClaimAgeDenominator = BonusClaimAgeDenominator;
	type CurrenciesHandler = Currencies;
	type Event = Event;
	type FeePrecision = FeePrecision;
	type InitialPoolSupply = InitialPoolSupply;
	type MaximumPassedInPoolTokenShares = MaximumPassedInPoolTokenShares;
	type MaximumSwapFee = MaximumSwapFee;
	type MaximumSwapInRatio = MaximumSwapInRatio;
	type MinimumPassedInPoolTokenShares = MinimumPassedInPoolTokenShares;
	type MinimumSwapFee = MinimumSwapFee;
	type NumberOfSupportedTokens = NumberOfSupportedTokens;
	type PoolId = u32;
	type PoolToken = u64;
	type PoolWeight = u64;
	type SwapFee = u64;
	type WeightPrecision = WeightPrecision;
}

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
	frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

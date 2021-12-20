// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::{account, whitelisted_caller};
use frame_support::{
	construct_runtime, parameter_types,
	traits::{GenesisBuild, Nothing},
	PalletId,
};
use node_primitives::{Amount, Balance, CurrencyId, TokenSymbol};
use sp_core::H256;
use sp_runtime::{
	generic,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
};

use crate as vsbond_auction;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;
type BlockNumber = u32;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>},
		Auction: vsbond_auction::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for Test {
	type AccountData = ();
	type AccountId = AccountId;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockNumber = BlockNumber;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
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
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

parameter_types! {
	pub const MaxLocks: u32 = 999;
}

impl orml_tokens::Config for Test {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = Nothing;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type OnDust = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const InvoicingCurrency: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const MaximumOrderInTrade: u32 = 5;
	pub const MinimumSupply: Balance = 0;
	pub const VsbondAuctionPalletId: PalletId = PalletId(*b"bf/vsbnd");
	pub BifrostTreasuryAccount: AccountId = PalletId(*b"bf/trsry").into_account();
}

impl vsbond_auction::Config for Test {
	type Event = Event;
	type InvoicingCurrency = InvoicingCurrency;
	type MaximumOrderInTrade = MaximumOrderInTrade;
	type MinimumAmount = MinimumSupply;
	type MultiCurrency = orml_tokens::Pallet<Self>;
	type WeightInfo = ();
	type PalletId = VsbondAuctionPalletId;
	type TreasuryAccount = BifrostTreasuryAccount;
	type ControlOrigin = ALICE;
}

// mockup runtime
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut fs_gc = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	#[cfg(feature = "runtime-benchmarks")]
	let whitelist_caller: AccountId = whitelisted_caller();
	#[cfg(feature = "runtime-benchmarks")]
	let benchmarking_account_1: AccountId = account("bechmarking_account_1", 0, 0);

	orml_tokens::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, TOKEN, 100),
			(ALICE, VSBOND, 100),
			(BRUCE, TOKEN, 100),
			(BRUCE, VSBOND, 100),
			(ALICE, SPECIAL_VSBOND, 100),
			(BRUCE, SPECIAL_VSBOND, 100),
			#[cfg(feature = "runtime-benchmarks")]
			(whitelist_caller.clone(), TOKEN, 100_000_000_000_000),
			#[cfg(feature = "runtime-benchmarks")]
			(whitelist_caller.clone(), VSBOND, 100_000_000_000_000),
			#[cfg(feature = "runtime-benchmarks")]
			(benchmarking_account_1.clone(), TOKEN, 100_000_000_000_000),
			#[cfg(feature = "runtime-benchmarks")]
			(benchmarking_account_1.clone(), VSBOND, 100_000_000_000_000),
		],
	}
	.assimilate_storage(&mut fs_gc)
	.unwrap();

	fs_gc.into()
}

pub(crate) const ALICE: AccountId = 1;
pub(crate) const BRUCE: AccountId = 2;
pub(crate) const TOKEN: CurrencyId = InvoicingCurrency::get();
pub(crate) const TOKEN_SYMBOL: TokenSymbol = TokenSymbol::KSM;
pub(crate) const VSBOND: CurrencyId = CurrencyId::VSBond(TOKEN_SYMBOL, 3000, 13, 20);
pub(crate) const SPECIAL_VSBOND: CurrencyId = CurrencyId::VSBond(TokenSymbol::BNC, 2001, 13, 20);

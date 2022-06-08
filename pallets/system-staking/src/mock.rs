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
use crate as pallet_system_staking;
use frame_system::EnsureRoot;
use crate::{Config};
use frame_support::{
	construct_runtime, parameter_types,
	traits::{Everything},
	weights::Weight,
	PalletId,
};
use node_primitives::{Amount, CurrencyId, TokenSymbol};
use sp_core::H256;
use sp_io;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;

pub const BNC: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
pub const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Currencies: orml_currencies::{Pallet, Call},
		// Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>},
		SystemStaking: pallet_system_staking::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
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
	type BlockWeights = ();
	type BlockLength = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
}
impl pallet_balances::Config for Test {
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 4];
	type MaxLocks = ();
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

pub type BifrostToken = orml_currencies::BasicCurrencyAdapter<Test, Balances, Amount, u64>;

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = BNC;
	pub const RelayCurrencyId: CurrencyId = KSM;
}

impl orml_currencies::Config for Test {
	type GetNativeCurrencyId = NativeCurrencyId;
	// type MultiCurrency = Tokens;
	type MultiCurrency = ();
	type NativeCurrency = BifrostToken;
	type WeightInfo = ();
}

// orml_traits::parameter_type_with_key! {
// 	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
// 		0
// 	};
// }

// impl orml_tokens::Config for Runtime {
// 	type Amount = Amount;
// 	type Balance = Balance;
// 	type CurrencyId = CurrencyId;
// 	type DustRemovalWhitelist = Nothing;
// 	type Event = Event;
// 	type ExistentialDeposits = ExistentialDeposits;
// 	type MaxLocks = MaxLocks;
// 	type MaxReserves = ();
// 	type OnDust = ();
// 	type ReserveIdentifier = [u8; 8];
// 	type WeightInfo = ();
// }

parameter_types! {
	pub const TreasuryAccount: AccountId = 1u64.into();
	pub const ThePalletId: PalletId = PalletId(*b"/systems");
}

impl Config for Test {
	type Event = Event;
	type MultiCurrency = Currencies;
	type EnsureConfirmAsGovernance = EnsureRoot<AccountId>;
	type WeightInfo =  pallet_system_staking::weights::SubstrateWeight<Test>;
	type FarmingInfo = ();
	type VtokenMintingInterface = ();
	type TreasuryAccount = TreasuryAccount;
	type PalletId = ThePalletId;
}

pub(crate) struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		Self
	}
}

impl ExtBuilder {
	pub(crate) fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Test>()
			.expect("Frame system builds valid default genesis config");

		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

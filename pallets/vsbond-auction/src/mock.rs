// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

use bifrost_primitives::{Amount, Balance, CurrencyId, TokenSymbol, VsbondAuctionPalletId, KSM};
#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::{account, whitelisted_caller};
use frame_support::{
	construct_runtime, derive_impl, ord_parameter_types, parameter_types, traits::Contains,
	PalletId,
};
use frame_system::EnsureSignedBy;
use sp_core::H256;
use sp_runtime::{
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	BuildStorage,
};

use crate as vsbond_auction;

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;
type BlockNumber = u32;

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Tokens: orml_tokens,
		Auction: vsbond_auction,
	}
);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountData = ();
	type AccountId = AccountId;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockWeights = ();
	type RuntimeCall = RuntimeCall;
	type DbWeight = ();
	type RuntimeEvent = RuntimeEvent;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Nonce = u32;
	type Block = Block;
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type RuntimeOrigin = RuntimeOrigin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		10
	};
}

pub struct DustRemovalWhitelist;
impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(a: &AccountId) -> bool {
		BifrostTreasuryAccount::get().eq(a) ||
			AccountIdConversion::<AccountId>::into_account_truncating(
				&VsbondAuctionPalletId::get(),
			)
			.eq(a)
	}
}

parameter_types! {
	pub const MaxLocks: u32 = 999;
}

impl orml_tokens::Config for Test {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = DustRemovalWhitelist;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type CurrencyHooks = ();
}

parameter_types! {
	pub const InvoicingCurrency: CurrencyId = KSM;
	pub const MaximumOrderInTrade: u32 = 5;
	pub const MinimumSupply: Balance = 0;
	pub BifrostTreasuryAccount: AccountId = PalletId(*b"bf/trsry").into_account_truncating();
}

ord_parameter_types! {
	pub const One: AccountId = 1;
}

impl vsbond_auction::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type InvoicingCurrency = InvoicingCurrency;
	type MaximumOrderInTrade = MaximumOrderInTrade;
	type MinimumAmount = MinimumSupply;
	type MultiCurrency = orml_tokens::Pallet<Self>;
	type WeightInfo = ();
	type PalletId = VsbondAuctionPalletId;
	type TreasuryAccount = BifrostTreasuryAccount;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
}

// mockup runtime
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut fs_gc = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
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
			(CHARLIE, TOKEN, 100),
			(ALICE, SPECIAL_VSBOND, 100),
			(BRUCE, SPECIAL_VSBOND, 100),
			(DAVE, VSBOND, 100),
			#[cfg(feature = "runtime-benchmarks")]
			(whitelist_caller, TOKEN, 100_000_000_000_000),
			#[cfg(feature = "runtime-benchmarks")]
			(whitelist_caller, VSBOND, 100_000_000_000_000),
			#[cfg(feature = "runtime-benchmarks")]
			(benchmarking_account_1, TOKEN, 100_000_000_000_000),
			#[cfg(feature = "runtime-benchmarks")]
			(benchmarking_account_1, VSBOND, 100_000_000_000_000),
		],
	}
	.assimilate_storage(&mut fs_gc)
	.unwrap();

	fs_gc.into()
}

pub(crate) const ALICE: AccountId = 1;
pub(crate) const BRUCE: AccountId = 2;
pub(crate) const CHARLIE: AccountId = 3;
pub(crate) const DAVE: AccountId = 4;
pub(crate) const TOKEN: CurrencyId = InvoicingCurrency::get();
pub(crate) const TOKEN_SYMBOL: TokenSymbol = TokenSymbol::KSM;
pub(crate) const VSBOND: CurrencyId = CurrencyId::VSBond(TOKEN_SYMBOL, 3000, 13, 20);
pub(crate) const SPECIAL_VSBOND: CurrencyId = CurrencyId::VSBond(TokenSymbol::BNC, 2001, 13, 20);

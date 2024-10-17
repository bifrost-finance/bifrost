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

#![cfg(test)]
#![allow(non_upper_case_globals)]

use frame_support::{
	parameter_types,
	traits::{Nothing, OnFinalize, OnInitialize},
	PalletId,
};
use bifrost_primitives::{
	currency::{BNC, KSM, VSKSM},
	CurrencyId, TokenSymbol,
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup},
	AccountId32,
};

use crate as bifrost_lightening_redeem;

pub type BlockNumber = u64;
pub type Balance = u64;

pub type AccountId = AccountId32;

pub const vsBond: CurrencyId = CurrencyId::VSBond(TokenSymbol::BNC, 2001, 13, 20);
pub const ALICE: AccountId = AccountId32::new([0u8; 32]);
pub const BOB: AccountId = AccountId32::new([1u8; 32]);
pub const CHARLIE: AccountId = AccountId32::new([3u8; 32]);

frame_support::construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		Tokens: orml_tokens,
		Council: pallet_collective::<Instance1>,
		LighteningRedeem: bifrost_lightening_redeem
	}
);

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}
impl frame_system::Config for Runtime {
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
		0
	};
}
parameter_types! {
	pub DustAccount: AccountId = PalletId(*b"orml/dst").into_account_truncating();
	pub const MaxLocks: u32 = 100;
}
impl orml_tokens::Config for Runtime {
	type Amount = i128;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = Nothing;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type CurrencyHooks = ();
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 2 * 7_200;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type RuntimeEvent = RuntimeEvent;
	type MaxMembers = CouncilMaxMembers;
	type MaxProposals = CouncilMaxProposals;
	type MotionDuration = CouncilMotionDuration;
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

impl bifrost_lightening_redeem::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Tokens;
	type ControlOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
	type PalletId = LighteningRedeemPalletId;
	type WeightInfo = ();
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { endowed_accounts: vec![] }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn one_hundred_for_alice_n_bob(self) -> Self {
		self.balances(vec![
			(ALICE, KSM, 100),
			(ALICE, vsBond, 100),
			(ALICE, VSKSM, 100),
			(BOB, vsBond, 100),
			(BOB, VSKSM, 100),
		])
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub fn one_hundred_precision_for_each_currency_type_for_whitelist_account(self) -> Self {
		use frame_benchmarking::whitelisted_caller;
		use sp_runtime::traits::AccountIdConversion;
		let whitelist_caller: AccountId = whitelisted_caller();
		let pool_account: AccountId = LighteningRedeemPalletId::get().into_account_truncating();

		self.balances(vec![
			(whitelist_caller.clone(), KSM, 100_000_000_000_000),
			(whitelist_caller.clone(), VSKSM, 100_000_000_000_000),
			(whitelist_caller.clone(), vsBond, 100_000_000_000_000),
			(pool_account.clone(), KSM, 100_000_000_000_000),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			balances: self
				.endowed_accounts
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id != BNC)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}

// simulate block production
pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		LighteningRedeem::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		LighteningRedeem::on_initialize(System::block_number());
	}
}

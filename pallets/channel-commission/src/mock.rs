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

use crate::mock::sp_api_hidden_includes_construct_runtime::hidden_include::traits::OnInitialize;
use bifrost_primitives::{
	currency::{ASG, BNC, KSM},
	CommissionPalletId, CurrencyId,
};
use frame_support::{derive_impl, ord_parameter_types, parameter_types, traits::Nothing, PalletId};
use frame_system::EnsureSignedBy;
use sp_core::ConstU32;
use sp_runtime::{traits::AccountIdConversion, AccountId32, BuildStorage};

use crate as bifrost_channel_commission;

pub type BlockNumber = u64;
pub type Amount = i128;
pub type Balance = u64;

pub type AccountId = AccountId32;

pub const ALICE: AccountId = AccountId32::new([0u8; 32]);
pub const BOB: AccountId = AccountId32::new([1u8; 32]);
pub const CHARLIE: AccountId = AccountId32::new([2u8; 32]);

frame_support::construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		Tokens: orml_tokens,
		Balances: pallet_balances,
		Currencies: bifrost_currencies,
		ChannelCommission: bifrost_channel_commission
	}
);

type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type AccountId = AccountId;
	type AccountData = pallet_balances::AccountData<Balance>;
	type Block = Block;
	type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = ASG;
}

pub type AdaptedBasicCurrency =
	bifrost_currencies::BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Runtime {
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Runtime {
	type AccountStore = frame_system::Pallet<Runtime>;
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
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

ord_parameter_types! {
	pub const One: AccountId = ALICE;
}

parameter_types! {
	pub const ClearingDuration: u32 = 100;
	pub const NameLengthLimit: u32 = 20;
	pub BifrostCommissionReceiver: AccountId = AccountId32::new([7u8; 32]);
}

impl bifrost_channel_commission::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
	type CommissionPalletId = CommissionPalletId;
	type BifrostCommissionReceiver = BifrostCommissionReceiver;
	type WeightInfo = ();
	type ClearingDuration = ClearingDuration;
	type NameLengthLimit = NameLengthLimit;
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
		let commission_account: AccountId32 =
			<Runtime as crate::Config>::CommissionPalletId::get().into_account_truncating();

		self.balances(vec![
			(ALICE, BNC, 100),
			(BOB, BNC, 100),
			(CHARLIE, BNC, 100),
			(ALICE, KSM, 100),
			(BOB, KSM, 100),
			(commission_account.clone(), CurrencyId::Token2(0), 11000),
			(commission_account, CurrencyId::VToken2(0), 11000),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

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
		System::set_block_number(System::block_number() + 1);
		ChannelCommission::on_initialize(System::block_number());
	}
}

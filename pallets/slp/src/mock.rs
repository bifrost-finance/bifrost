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

// Ensure we're `no_std` when compiling for Wasm.

#![cfg(test)]

use codec::{Decode, Encode};
pub use cumulus_primitives_core::ParaId;
use frame_support::{
	construct_runtime,
	dispatch::DispatchResult,
	ord_parameter_types,
	pallet_prelude::Get,
	parameter_types,
	traits::{GenesisBuild, Nothing, OnFinalize, OnInitialize},
	weights::Weight,
};
use frame_system::EnsureSignedBy;
use node_primitives::{Amount, Balance, CurrencyId, TokenSymbol};
use orml_traits::XcmTransfer;
use sp_core::{blake2_256, H256};
pub use sp_runtime::{testing::Header, Perbill};
use sp_runtime::{
	traits::{AccountIdConversion, Convert, IdentityLookup, TrailingZeroInput},
	AccountId32,
};
use sp_std::{boxed::Box, vec::Vec};
use xcm::latest::prelude::*;

use crate as bifrost_slp;
use crate::{Config, TimeUnit, VtokenMintingOperator};

pub type AccountId = AccountId32;
pub type Block = frame_system::mocking::MockBlock<Runtime>;
pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;

pub const EXIT_ACCOUNT: AccountId = AccountId32::new([5u8; 32]);
pub const ENTRANCE_ACCOUNT: AccountId = AccountId32::new([6u8; 32]);

pub const ALICE: AccountId = AccountId32::new([1u8; 32]);
pub const BOB: AccountId = AccountId32::new([2u8; 32]);
pub const CHARLIE: AccountId = AccountId32::new([3u8; 32]);

pub const BNC: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
pub const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
pub const VKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>},
		Slp: bifrost_slp::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = BNC;
	pub const RelayCurrencyId: CurrencyId = KSM;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = frame_support::traits::Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 0;
	pub const MaxLocks: u32 = 999_999;
	pub const MaxReserves: u32 = 999_999;
}

impl pallet_balances::Config for Runtime {
	type AccountStore = System;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

impl orml_tokens::Config for Runtime {
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

pub type BifrostToken = orml_currencies::BasicCurrencyAdapter<Runtime, Balances, Amount, u64>;

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BifrostToken;
	type WeightInfo = ();
}

ord_parameter_types! {
	pub const One: AccountId = AccountId32::new([1u8; 32]);
}

pub struct SubAccountIndexMultiLocationConvertor;
impl Convert<u16, MultiLocation> for SubAccountIndexMultiLocationConvertor {
	fn convert(sub_account_index: u16) -> MultiLocation {
		MultiLocation::new(
			1,
			X1(Junction::AccountId32 {
				network: NetworkId::Any,
				// id: Utility::derivative_account_id(
				// 	ParaId::from(2001u32).into_account(),
				// 	sub_account_index,
				// )
				// .into(),
				id: Self::derivative_account_id(
					ParaId::from(2001u32).into_account(),
					sub_account_index,
				)
				.into(),
			}),
		)
	}
}

// Mock Utility::derivative_account_id function.
impl SubAccountIndexMultiLocationConvertor {
	pub fn derivative_account_id(who: AccountId, index: u16) -> AccountId {
		let entropy = (b"modlpy/utilisuba", who, index).using_encoded(blake2_256);
		Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
			.expect("infinite length input; no invalid inputs for type; qed")
	}
}

pub struct ParachainId;
impl Get<ParaId> for ParachainId {
	fn get() -> ParaId {
		2001.into()
	}
}

impl Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
	type WeightInfo = ();
	type VtokenMinting = MockVtokenMintingOperator;
	type AccountConverter = SubAccountIndexMultiLocationConvertor;
	type ParachainId = ParachainId;
	type XcmSender = ();
	type XcmExecutor = ();
}

pub struct MockVtokenMintingOperator;
impl VtokenMintingOperator<CurrencyId, Balance, AccountId, TimeUnit> for MockVtokenMintingOperator {
	fn increase_token_pool(_currency_id: CurrencyId, _token_amount: Balance) -> DispatchResult {
		Ok(())
	}

	fn decrease_token_pool(_currency_id: CurrencyId, _token_amount: Balance) -> DispatchResult {
		Ok(())
	}

	fn update_ongoing_time_unit(_currency_id: CurrencyId, _time_unit: TimeUnit) -> DispatchResult {
		Ok(())
	}

	fn get_ongoing_time_unit(_currency_id: CurrencyId) -> Option<TimeUnit> {
		Some(TimeUnit::Era(2))
	}

	fn get_unlock_records(
		_currency_id: CurrencyId,
		_time_unit: TimeUnit,
	) -> Option<(Balance, Vec<u32>)> {
		None
	}

	fn deduct_unlock_amount(
		_currency_id: CurrencyId,
		_index: u32,
		_deduct_amount: Balance,
	) -> DispatchResult {
		Ok(())
	}

	fn get_entrance_and_exit_accounts() -> (AccountId, AccountId) {
		(ENTRANCE_ACCOUNT, EXIT_ACCOUNT)
	}

	fn get_token_unlock_ledger(
		_currency_id: CurrencyId,
		_index: u32,
	) -> Option<(AccountId, Balance, TimeUnit)> {
		None
	}

	fn increase_token_to_add(_currency_id: CurrencyId, _value: Balance) -> DispatchResult {
		Ok(())
	}

	fn decrease_token_to_add(_currency_id: CurrencyId, _value: Balance) -> DispatchResult {
		Ok(())
	}

	fn increase_token_to_deduct(_currency_id: CurrencyId, _value: Balance) -> DispatchResult {
		Ok(())
	}

	fn decrease_token_to_deduct(_currency_id: CurrencyId, _value: Balance) -> DispatchResult {
		Ok(())
	}
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
		self.balances(vec![(ALICE, KSM, 100), (ALICE, VKSM, 100), (BOB, VKSM, 100)])
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub fn one_hundred_precision_for_each_currency_type_for_whitelist_account(self) -> Self {
		use frame_benchmarking::whitelisted_caller;
		use sp_runtime::traits::AccountIdConversion;
		let whitelist_caller: AccountId = whitelisted_caller();
		let pool_account: AccountId = LighteningRedeemPalletId::get().into_account();

		self.balances(vec![
			(whitelist_caller.clone(), KSM, 100_000_000_000_000),
			(whitelist_caller.clone(), VKSM, 100_000_000_000_000),
			(pool_account.clone(), KSM, 100_000_000_000_000),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			balances: self
				.endowed_accounts
				.clone()
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
		Slp::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		Slp::on_initialize(System::block_number());
	}
}

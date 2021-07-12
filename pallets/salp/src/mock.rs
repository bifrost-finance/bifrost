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

// Ensure we're `no_std` when compiling for Wasm.

use frame_support::{construct_runtime, parameter_types, traits::GenesisBuild, PalletId};
use node_primitives::{Amount, Balance, CurrencyId, TokenSymbol};
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentityLookup},
};
use xcm::{
	v0::{prelude::XcmResult, MultiLocation, NetworkId},
	DoubleEncoded,
};
use xcm_builder::{EnsureXcmOrigin, SignedToAccountId32};
use xcm_support::BifrostXcmExecutor;

use crate as salp;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Signature = sp_runtime::MultiSignature;
pub(crate) type AccountId = <<Signature as sp_runtime::traits::Verify>::Signer as sp_runtime::traits::IdentifyAccount>::AccountId;
type BlockNumber = u32;
type Index = u32;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>},
		Bancor: bifrost_bancor::{Pallet, Call, Config<T>, Storage, Event<T>},
		Salp: salp::{Pallet, Call, Storage, Event<T>},
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
	type BaseCallFilter = ();
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
	type Index = Index;
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
	pub const MaxLocks: u32 = 999_999_999;
}

impl orml_tokens::Config for Test {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type OnDust = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const InterventionPercentage: Balance = 75;
}

impl bifrost_bancor::Config for Test {
	type Event = Event;
	type InterventionPercentage = InterventionPercentage;
	type MultiCurrenciesHandler = Tokens;
}

// TODO: Impl bifrost_xcm_executor::Config

parameter_types! {
	pub const SubmissionDeposit: u32 = 1;
	pub const MinContribution: Balance = 1 * DOLLARS;
	pub const BifrostCrowdloanId: PalletId = PalletId(*b"bf/salp#");
	pub const RemoveKeysLimit: u32 = 50;
	pub const TokenType: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const VSBondValidPeriod: BlockNumber = 30 * DAYS;
	pub const ReleaseCycle: BlockNumber = 1 * DAYS;
	pub const LeasePeriod: BlockNumber = 6 * WEEKS;
	pub const ReleaseRatio: Percent = Percent::from_percent(50);
	pub const SlotLength: BlockNumber = 8u32 as BlockNumber;
	pub const DepositTokenType: CurrencyId = CurrencyId::Token(TokenSymbol::ASG);
}

parameter_types! {
	pub const AnyNetwork: NetworkId = NetworkId::Any;
}

type LocalOriginToLocation = (SignedToAccountId32<Origin, AccountId, AnyNetwork>,);

impl salp::Config for Test {
	type BancorPool = Bancor;
	type BifrostXcmExecutor = MockXcmExecutor;
	type DepositToken = DepositTokenType;
	type Event = Event;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type LeasePeriod = LeasePeriod;
	type MinContribution = MinContribution;
	type MultiCurrency = Tokens;
	type PalletId = BifrostCrowdloanId;
	type ReleaseCycle = ReleaseCycle;
	type ReleaseRatio = ReleaseRatio;
	type RelyChainToken = TokenType;
	type RemoveKeysLimit = RemoveKeysLimit;
	type SlotLength = SlotLength;
	type SubmissionDeposit = SubmissionDeposit;
	type VSBondValidPeriod = VSBondValidPeriod;
	type WeightInfo = salp::TestWeightInfo;
}

// To control the result returned by `MockXcmExecutor`
pub(crate) static mut MOCK_XCM_RESULT: (bool, bool) = (true, true);

// Mock XcmExecutor
pub struct MockXcmExecutor;

impl BifrostXcmExecutor for MockXcmExecutor {
	fn ump_transact(_origin: MultiLocation, _call: DoubleEncoded<()>) -> XcmResult {
		let result = unsafe { MOCK_XCM_RESULT.0 };

		match result {
			true => Ok(()),
			false => Err(xcm::v0::Error::Undefined),
		}
	}

	fn ump_transfer_asset(
		_origin: MultiLocation,
		_dest: MultiLocation,
		_amount: u128,
		_relay: bool,
	) -> XcmResult {
		let result = unsafe { MOCK_XCM_RESULT.1 };

		match result {
			true => Ok(()),
			false => Err(xcm::v0::Error::Undefined),
		}
	}
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	orml_tokens::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, CurrencyId::Token(TokenSymbol::ASG), 20),
			(ALICE, CurrencyId::Token(TokenSymbol::BNC), 20),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

pub const DOLLARS: Balance = 1_000_000_000_000;
// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60 / (12 as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;

pub(crate) const ALICE: AccountId = AccountId::new([0u8; 32]);
pub(crate) const BRUCE: AccountId = AccountId::new([1u8; 32]);
pub(crate) const CATHI: AccountId = AccountId::new([2u8; 32]);

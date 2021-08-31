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

use frame_support::{
	construct_runtime, parameter_types,
	traits::{EnsureOrigin, GenesisBuild},
	weights::{WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial},
	PalletId,
};
use frame_system::RawOrigin;
use node_primitives::{
	Amount, Balance, CurrencyId, MessageId, ParachainTransactProxyType, ParachainTransactType,
	TokenSymbol, TransferOriginType,
};
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentityLookup},
};
use xcm::{
	v0::{MultiLocation, NetworkId},
	DoubleEncoded,
};
use xcm_builder::{EnsureXcmOrigin, SignedToAccountId32};
use xcm_support::{BifrostXcmExecutor, Weight};

use crate as salp;
use crate::WeightInfo;

pub(crate) type AccountId = <<Signature as sp_runtime::traits::Verify>::Signer as sp_runtime::traits::IdentifyAccount>::AccountId;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u32;
pub(crate) type Index = u32;
pub(crate) type Signature = sp_runtime::MultiSignature;
pub(crate) type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>},
		Bancor: bifrost_bancor::{Pallet, Call, Config<T>, Storage, Event<T>},
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>},
		Salp: salp::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
	pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const StableCurrencyId: CurrencyId = CurrencyId::Stable(TokenSymbol::KUSD);
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<Balance>;
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

parameter_types! {
	pub const ExistentialDeposit: u128 = 0;
	pub const TransferFee: u128 = 0;
	pub const CreationFee: u128 = 0;
	pub const TransactionByteFee: u128 = 0;
	pub const MaxLocks: u32 = 999_999;
	pub const MaxReserves: u32 = 999_999;
}

impl pallet_balances::Config for Test {
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
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
}

parameter_types! {
	pub const DepositBase: Balance = 0;
	pub const DepositFactor: Balance = 0;
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Test {
	type Call = Call;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type Event = Event;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Test>;
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

impl orml_tokens::Config for Test {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = ();
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type OnDust = ();
	type WeightInfo = ();
}

pub type BifrostToken = orml_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Test {
	type Event = Event;
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BifrostToken;
	type WeightInfo = ();
}

parameter_types! {
	pub const InterventionPercentage: Percent = Percent::from_percent(75);
	pub const DailyReleasePercentage: Percent = Percent::from_percent(5);
}

impl bifrost_bancor::Config for Test {
	type Event = Event;
	type InterventionPercentage = InterventionPercentage;
	type DailyReleasePercentage = DailyReleasePercentage;
	type MultiCurrency = Tokens;
	type WeightInfo = ();
}

pub fn create_x2_parachain_multilocation(_index: u16) -> MultiLocation {
	MultiLocation::X2(
		Junction::Parent,
		Junction::AccountId32 { network: NetworkId::Any, id: ALICE.into() },
	)
}

parameter_types! {
	pub const SubmissionDeposit: u32 = 1;
	pub const MinContribution: Balance = 10;
	pub const BifrostCrowdloanId: PalletId = PalletId(*b"bf/salp#");
	pub const RemoveKeysLimit: u32 = 50;
	pub const SlotLength: BlockNumber = 8u32 as BlockNumber;
	pub const LeasePeriod: BlockNumber = 6 * WEEKS;
	pub const VSBondValidPeriod: BlockNumber = 30 * DAYS;
	pub const ReleaseCycle: BlockNumber = 1 * DAYS;
	pub const ReleaseRatio: Percent = Percent::from_percent(50);
	pub const DepositTokenType: CurrencyId = CurrencyId::Token(TokenSymbol::ASG);
	pub const XcmTransferOrigin: TransferOriginType = TransferOriginType::FromRelayChain;
	pub BaseXcmWeight:u64 = 1_000_000_000 as u64;
	pub ContributionWeight:u64 = 1_000_000_000 as u64;
	pub WithdrawWeight:u64 = 1_000_000_000 as u64;
	pub AddProxyWeight:u64 = 1_000_000_000 as u64;
	pub RemoveProxyWeight:u64 = 1_000_000_000 as u64;
	pub const SelfParaId: u32 = 2001;
	pub PrimaryAccount: AccountId = ALICE;
	pub ConfirmMuitiSigAccount: AccountId = Multisig::multi_account_id(&vec![
		ALICE,
		BRUCE,
		CATHI
	],2);
	pub RelaychainSovereignSubAccount: MultiLocation = create_x2_parachain_multilocation(0 as u16);
	pub SalpTransactProxyType: ParachainTransactProxyType = ParachainTransactProxyType::Derived;
	pub SalpTransactType: ParachainTransactType = ParachainTransactType::Xcm;
}

parameter_types! {
	pub const AnyNetwork: NetworkId = NetworkId::Any;
}

type LocalOriginToLocation = (SignedToAccountId32<Origin, AccountId, AnyNetwork>,);

pub struct EnsureConfirmAsMultiSig;
impl EnsureOrigin<Origin> for EnsureConfirmAsMultiSig {
	type Success = AccountId;

	fn try_origin(o: Origin) -> Result<Self::Success, Origin> {
		Into::<Result<RawOrigin<AccountId>, Origin>>::into(o).and_then(|o| match o {
			RawOrigin::Signed(who) =>
				if who == PrimaryAccount::get() || who == ConfirmMuitiSigAccount::get() {
					Ok(who)
				} else {
					Err(Origin::from(Some(who)))
				},
			r => Err(Origin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> Origin {
		Origin::from(RawOrigin::Signed(ConfirmMuitiSigAccount::get()))
	}
}

use frame_support::dispatch::DispatchResult;
use orml_traits::XcmTransfer;
use smallvec::smallvec;
pub use sp_runtime::Perbill;
use xcm::{
	opaque::v0::MultiAsset,
	v0::{prelude::XcmError, Junction},
};

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(90u32, 100u32),
			coeff_integer: 1,
		}]
	}
}

pub struct MockXTokens;

impl XcmTransfer<AccountId, Balance, CurrencyId> for MockXTokens {
	fn transfer(
		_who: AccountId,
		_currency_id: CurrencyId,
		_amount: Balance,
		_dest: MultiLocation,
		_dest_weight: Weight,
	) -> DispatchResult {
		Ok(())
	}

	fn transfer_multi_asset(
		_who: AccountId,
		_asset: MultiAsset,
		_dest: MultiLocation,
		_dest_weight: Weight,
	) -> DispatchResult {
		Ok(())
	}
}

impl salp::Config for Test {
	type BancorPool = Bancor;
	type BifrostXcmExecutor = MockXcmExecutor;
	type DepositToken = NativeCurrencyId;
	type Event = Event;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type LeasePeriod = LeasePeriod;
	type MinContribution = MinContribution;
	type MultiCurrency = Tokens;
	type PalletId = BifrostCrowdloanId;
	type RelayChainToken = RelayCurrencyId;
	type ReleaseCycle = ReleaseCycle;
	type ReleaseRatio = ReleaseRatio;
	type RemoveKeysLimit = RemoveKeysLimit;
	type SlotLength = SlotLength;
	type SubmissionDeposit = SubmissionDeposit;
	type VSBondValidPeriod = VSBondValidPeriod;
	type XcmTransferOrigin = XcmTransferOrigin;
	type WeightInfo = SalpWeightInfo;
	type SelfParaId = SelfParaId;
	type BaseXcmWeight = BaseXcmWeight;
	type ContributionWeight = ContributionWeight;
	type WithdrawWeight = WithdrawWeight;
	type EnsureConfirmAsMultiSig = EnsureConfirmAsMultiSig;
	type WeightToFee = WeightToFee;
	type AddProxyWeight = AddProxyWeight;
	type RemoveProxyWeight = RemoveProxyWeight;
	type XcmTransfer = MockXTokens;
	type SovereignSubAccountLocation = RelaychainSovereignSubAccount;
	type TransactProxyType = SalpTransactProxyType;
	type TransactType = SalpTransactType;
}

pub struct SalpWeightInfo;
impl WeightInfo for SalpWeightInfo {
	fn create() -> Weight {
		0
	}

	fn contribute() -> Weight {
		0
	}

	fn unlock() -> Weight {
		0
	}

	fn withdraw() -> Weight {
		0
	}

	fn redeem() -> Weight {
		0
	}

	fn refund() -> Weight {
		0
	}

	fn dissolve(_n: u32) -> Weight {
		0
	}

	fn on_initialize(_n: u32) -> Weight {
		0
	}
}

// To control the result returned by `MockXcmExecutor`
pub(crate) static mut MOCK_XCM_RESULT: (bool, bool) = (true, true);

// Mock XcmExecutor
pub struct MockXcmExecutor;

impl BifrostXcmExecutor for MockXcmExecutor {
	fn transact_weight(_: u64, _: u32) -> u64 {
		return 0;
	}

	fn transact_id(_data: &[u8]) -> MessageId {
		return [0; 32];
	}

	fn ump_transact(
		_origin: MultiLocation,
		_call: DoubleEncoded<()>,
		_weight: u64,
		_relayer: bool,
		_nonce: u32,
	) -> Result<[u8; 32], XcmError> {
		let result = unsafe { MOCK_XCM_RESULT.0 };

		match result {
			true => Ok([0; 32]),
			false => Err(xcm::v0::Error::Undefined),
		}
	}

	fn ump_transacts(
		_origin: MultiLocation,
		_call: Vec<DoubleEncoded<()>>,
		_weight: u64,
		_relayer: bool,
	) -> Result<MessageId, XcmError> {
		let result = unsafe { MOCK_XCM_RESULT.0 };

		match result {
			true => Ok([0; 32]),
			false => Err(xcm::v0::Error::Undefined),
		}
	}

	fn ump_transfer_asset(
		_origin: MultiLocation,
		_dest: MultiLocation,
		_amount: u128,
		_relay: bool,
		_nonce: u32,
	) -> Result<MessageId, XcmError> {
		let result = unsafe { MOCK_XCM_RESULT.1 };

		match result {
			true => Ok([0; 32]),
			false => Err(xcm::v0::Error::Undefined),
		}
	}
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	orml_tokens::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, NativeCurrencyId::get(), INIT_BALANCE),
			(ALICE, RelayCurrencyId::get(), INIT_BALANCE),
			(BRUCE, NativeCurrencyId::get(), INIT_BALANCE),
			(BRUCE, RelayCurrencyId::get(), INIT_BALANCE),
			(CATHI, NativeCurrencyId::get(), INIT_BALANCE),
			(CATHI, RelayCurrencyId::get(), INIT_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60 / (12 as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;

pub(crate) const ALICE: AccountId = AccountId::new([0u8; 32]);
pub(crate) const BRUCE: AccountId = AccountId::new([1u8; 32]);
pub(crate) const CATHI: AccountId = AccountId::new([2u8; 32]);
pub(crate) const CONTRIBUTON_INDEX: MessageId = [0; 32];

pub(crate) const INIT_BALANCE: Balance = 100_000;

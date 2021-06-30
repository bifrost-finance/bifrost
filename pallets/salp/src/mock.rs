#![cfg(test)]

use frame_support::{construct_runtime, parameter_types, PalletId};
use node_primitives::{
	traits::BifrostXcmExecutor, Amount, Balance, CurrencyId, Moment, TokenSymbol,
};
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
use crate as salp;

pub const BNCS: Balance = 1_000_000_000_000;
pub const DOLLARS: Balance = BNCS;
pub const MILLISECS_PER_BLOCK: Moment = 12000;
pub const SECS_PER_BLOCK: Moment = MILLISECS_PER_BLOCK / 1000;
// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60 / (SECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type Signature = sp_runtime::MultiSignature;
pub type AccountId = <<Signature as sp_runtime::traits::Verify>::Signer as sp_runtime::traits::IdentifyAccount>::AccountId;
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
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
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
	pub const SubmissionDeposit: Balance = 100 * DOLLARS;
	pub const MinContribution: Balance = 1 * DOLLARS;
	pub const BifrostCrowdloanId: PalletId = PalletId(*b"bf/salp#");
	pub const RemoveKeysLimit: u32 = 500;
	pub const TokenType: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const VSBondValidPeriod: BlockNumber = 30 * DAYS;
	pub const ReleaseCycle: BlockNumber = 1 * DAYS;
	pub const LeasePeriod: BlockNumber = 6 * WEEKS;
	pub const ReleaseRatio: Percent = Percent::from_percent(50);
	pub const SlotLength: BlockNumber = 8u32 as BlockNumber;
}

parameter_types! {
	pub const AnyNetwork: NetworkId = NetworkId::Any;
}

type LocalOriginToLocation = (SignedToAccountId32<Origin, AccountId, AnyNetwork>,);

impl salp::Config for Test {
	type BancorPool = Bancor;
	type BifrostXcmExecutor = MockXcmExecutor;
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
}

// Mock XcmExecutor
pub struct MockXcmExecutor;

// To control the result returned by `MockXcmExecutor`
pub(crate) static mut MOCK_XCM_RESULT: (bool, bool) = (true, true);

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
	let fs_gc = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	// orml_tokens::GenesisConfig::<Test> {
	// 	balances: vec![
	// 		(ACCOUNT_ALICE, TOKEN, BALANCE_TOKEN),
	// 		(ACCOUNT_ALICE, VSBOND, BALANCE_VSBOND),
	// 		(ACCOUNT_BRUCE, TOKEN, BALANCE_TOKEN),
	// 		(ACCOUNT_BRUCE, VSBOND, BALANCE_VSBOND),
	// 	],
	// }
	// 	.assimilate_storage(&mut fs_gc)
	// 	.unwrap();

	fs_gc.into()
}

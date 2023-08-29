use crate as oracle;
use crate::{Config, Error};
use frame_support::{
	parameter_types,
	traits::{ConstU32, Everything, GenesisBuild},
	BoundedVec,
};
use mocktopus::mocking::clear_mocks;
use orml_traits::parameter_type_with_key;
pub use primitives::{CurrencyId::Token, TokenSymbol::*, BNC};
use sp_arithmetic::FixedU128;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		// substrate pallets
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Tokens: orml_tokens::{Pallet, Storage, Config<T>, Event<T>},

		// Operational
		// Security: security::{Pallet, Call, Storage, Event<T>},
		Oracle: oracle::{Pallet, Call, Config<T>, Storage, Event<T>},
		// Staking: staking::{Pallet, Storage, Event<T>},
		// Currency: currency::{Pallet},
	}
);

pub type AccountId = u64;
pub type Balance = u128;
pub type BlockNumber = u64;
pub type UnsignedFixedPoint = FixedU128;
// pub type SignedFixedPoint = FixedI128;
// pub type SignedInner = i128;
pub type CurrencyId = primitives::CurrencyId;
pub type Moment = u64;
pub type Index = u64;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = Index;
	type BlockNumber = BlockNumber;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub const DEFAULT_COLLATERAL_CURRENCY: CurrencyId = Token(DOT);
pub const DEFAULT_NATIVE_CURRENCY: CurrencyId = BNC;
// pub const DEFAULT_WRAPPED_CURRENCY: CurrencyId = Token(IBTC);

parameter_types! {
	pub const GetCollateralCurrencyId: CurrencyId = DEFAULT_COLLATERAL_CURRENCY;
	pub const GetNativeCurrencyId: CurrencyId = DEFAULT_NATIVE_CURRENCY;
	// pub const GetWrappedCurrencyId: CurrencyId = DEFAULT_WRAPPED_CURRENCY;
	pub const MaxLocks: u32 = 50;
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

impl orml_tokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = i128;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = ();
	type MaxLocks = MaxLocks;
	type DustRemovalWhitelist = Everything;
	type MaxReserves = ConstU32<0>; // we don't use named reserves
	type ReserveIdentifier = (); // we don't use named reserves
}

// pub struct CurrencyConvert;
// impl currency::CurrencyConversion<currency::Amount<Test>, CurrencyId> for CurrencyConvert {
// 	fn convert(
// 		_amount: &currency::Amount<Test>,
// 		_to: CurrencyId,
// 	) -> Result<currency::Amount<Test>, sp_runtime::DispatchError> {
// 		unimplemented!()
// 	}
// }

// impl currency::Config for Test {
// 	type SignedInner = SignedInner;
// 	type SignedFixedPoint = SignedFixedPoint;
// 	type UnsignedFixedPoint = UnsignedFixedPoint;
// 	type Balance = Balance;
// 	type GetNativeCurrencyId = GetNativeCurrencyId;
// 	type GetRelayChainCurrencyId = GetCollateralCurrencyId;
// 	type GetWrappedCurrencyId = GetWrappedCurrencyId;
// 	type CurrencyConversion = CurrencyConvert;
// }

// impl OnExchangeRateChange<CurrencyId> for () {
// 	fn on_exchange_rate_change(currency_id: &CurrencyId) {
// 		// todo: propagate error
// 		let _ = currency_id;
// 	}
// }

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type OnExchangeRateChange = ();
	type WeightInfo = ();
	type MaxNameLength = ConstU32<255>;
	type UnsignedFixedPoint = UnsignedFixedPoint;
}

parameter_types! {
	pub const MinimumPeriod: Moment = 5;
}

impl pallet_timestamp::Config for Test {
	type Moment = Moment;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

// impl security::Config for Test {
// 	type RuntimeEvent = RuntimeEvent;
// 	type WeightInfo = ();
// }

// impl staking::Config for Test {
// 	type RuntimeEvent = RuntimeEvent;
// 	type SignedFixedPoint = SignedFixedPoint;
// 	type SignedInner = SignedInner;
// 	type CurrencyId = CurrencyId;
// 	type GetNativeCurrencyId = GetNativeCurrencyId;
// }

pub type TestEvent = RuntimeEvent;
pub type TestError = Error<Test>;

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build() -> sp_io::TestExternalities {
		let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		oracle::GenesisConfig::<Test> {
			authorized_oracles: vec![(
				0,
				BoundedVec::try_from("test".as_bytes().to_vec()).unwrap(),
			)],
			max_delay: 0,
		}
		.assimilate_storage(&mut storage)
		.unwrap();

		sp_io::TestExternalities::from(storage)
	}
}

pub fn run_test<T>(test: T)
where
	T: FnOnce(),
{
	clear_mocks();
	ExtBuilder::build().execute_with(|| {
		// Security::set_active_block_number(1);
		System::set_block_number(1);
		test();
	});
}

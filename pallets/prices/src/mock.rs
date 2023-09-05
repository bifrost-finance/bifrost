// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Mocks for the prices module.

use super::*;
use frame_support::{
	construct_runtime, ord_parameter_types, parameter_types,
	traits::{AsEnsureOriginWithArg, Everything, SortedMembers},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned, EnsureSignedBy};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, FixedPointNumber};

pub use primitives::tokens::{CDOT_7_14, CKSM_20_27, DOT, KSM, LP_DOT_CDOT_7_14, SDOT, SKSM};

pub type AccountId = u128;
pub type BlockNumber = u64;
pub const ALICE: AccountId = 1;
pub const CHARLIE: AccountId = 2;

pub const PRICE_ONE: u128 = 1_000_000_000_000_000_000;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub type TimeStampedPrice = orml_oracle::TimestampedValue<Price, Moment>;
pub struct MockDataProvider;
impl DataProvider<CurrencyId, TimeStampedPrice> for MockDataProvider {
	fn get(asset_id: &CurrencyId) -> Option<TimeStampedPrice> {
		match *asset_id {
			DOT =>
				Some(TimeStampedPrice { value: Price::saturating_from_integer(100), timestamp: 0 }),
			KSM =>
				Some(TimeStampedPrice { value: Price::saturating_from_integer(500), timestamp: 0 }),
			_ => None,
		}
	}
}

impl DataProviderExtended<CurrencyId, TimeStampedPrice> for MockDataProvider {
	fn get_no_op(asset_id: &CurrencyId) -> Option<TimeStampedPrice> {
		match *asset_id {
			DOT =>
				Some(TimeStampedPrice { value: Price::saturating_from_integer(100), timestamp: 0 }),
			KSM =>
				Some(TimeStampedPrice { value: Price::saturating_from_integer(500), timestamp: 0 }),
			_ => None,
		}
	}

	fn get_all_values() -> Vec<(CurrencyId, Option<TimeStampedPrice>)> {
		vec![]
	}
}

impl DataFeeder<CurrencyId, TimeStampedPrice, AccountId> for MockDataProvider {
	fn feed_value(_: AccountId, _: CurrencyId, _: TimeStampedPrice) -> sp_runtime::DispatchResult {
		Ok(())
	}
}

pub struct LiquidStakingExchangeRateProvider;
impl ExchangeRateProvider<CurrencyId> for LiquidStakingExchangeRateProvider {
	fn get_exchange_rate(_: &CurrencyId) -> Option<Rate> {
		Some(Rate::saturating_from_rational(150, 100))
	}
}

pub struct TokenExchangeRateProvider;
impl VaultTokenExchangeRateProvider<CurrencyId> for TokenExchangeRateProvider {
	fn get_exchange_rate(_: &CurrencyId, _: Rate) -> Option<Rate> {
		Some(Rate::saturating_from_rational(100, 150))
	}
}

ord_parameter_types! {
	pub const One: AccountId = 1;
}

pub struct Decimal;
#[allow(non_upper_case_globals)]
impl DecimalProvider<CurrencyId> for Decimal {
	fn get_decimal(asset_id: &CurrencyId) -> Option<u8> {
		match *asset_id {
			DOT | SDOT => Some(10),
			KSM | SKSM => Some(12),
			CKSM_20_27 => Some(12),
			CDOT_7_14 => Some(10),
			LP_DOT_CDOT_7_14 => Some(12),
			LC_DOT => Some(10),
			_ => None,
		}
	}
}

pub struct LiquidStaking;
impl LiquidStakingCurrenciesProvider<CurrencyId> for LiquidStaking {
	fn get_staking_currency() -> Option<CurrencyId> {
		Some(DOT)
	}
	fn get_liquid_currency() -> Option<CurrencyId> {
		Some(SDOT)
	}
}

pub struct TokenCurrenciesFilter;
impl VaultTokenCurrenciesFilter<CurrencyId> for TokenCurrenciesFilter {
	fn contains(asset_id: &CurrencyId) -> bool {
		asset_id == &CDOT_7_14
	}
}

pub struct VaultLoansRateProvider;
impl LoansMarketDataProvider<CurrencyId, Balance> for VaultLoansRateProvider {
	fn get_full_interest_rate(_asset_id: CurrencyId) -> Option<Rate> {
		Some(Rate::from_inner(450_000_000_000_000_000))
	}

	fn get_market_info(_: CurrencyId) -> Result<MarketInfo, sp_runtime::DispatchError> {
		Ok(Default::default())
	}
	fn get_market_status(
		_: CurrencyId,
	) -> Result<MarketStatus<Balance>, sp_runtime::DispatchError> {
		Ok(Default::default())
	}
}

parameter_types! {
	pub const RelayCurrency: CurrencyId = DOT;
	pub const NativeCurrencyId: CurrencyId = 1;
}

impl pallet_currency_adapter::Config for Test {
	type Assets = Assets;
	type Balances = Balances;
	type GetNativeCurrencyId = NativeCurrencyId;
	type LockOrigin = EnsureRoot<AccountId>;
}

// pallet-balances configuration
parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

// pallet-assets configuration
parameter_types! {
	pub const AssetDeposit: u64 = 1;
	pub const ApprovalDeposit: u64 = 1;
	pub const AssetAccountDeposit: u64 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u64 = 1;
	pub const MetadataDepositPerByte: u64 = 1;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = CurrencyId;
	type AssetIdParameter = codec::Compact<CurrencyId>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type AssetAccountDeposit = AssetAccountDeposit;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	type CallbackHandle = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
}

// AMM instance initialization
parameter_types! {
	pub const AMMPalletId: PalletId = PalletId(*b"par/ammp");
	// pub const DefaultLpFee: Ratio = Ratio::from_rational(25u32, 10000u32);        // 0.25%
	// pub const DefaultProtocolFee: Ratio = Ratio::from_rational(5u32, 10000u32);
	pub  DefaultLpFee: Ratio = Ratio::from_rational(25u32, 10000u32);         // 0.3%
	pub const MinimumLiquidity: u128 = 1_000u128;
	pub const LockAccountId: AccountId = ALICE;
	pub const MaxLengthRoute: u8 = 10;
}

pub struct AliceCreatePoolOrigin;
impl SortedMembers<AccountId> for AliceCreatePoolOrigin {
	fn sorted_members() -> Vec<AccountId> {
		vec![ALICE]
	}
}

impl pallet_amm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Assets = CurrencyAdapter;
	type PalletId = AMMPalletId;
	type LockAccountId = LockAccountId;
	type AMMWeightInfo = ();
	type CreatePoolOrigin = EnsureSignedBy<AliceCreatePoolOrigin, AccountId>;
	type ProtocolFeeUpdateOrigin = EnsureSignedBy<AliceCreatePoolOrigin, AccountId>;
	type LpFee = DefaultLpFee;
	type MinimumLiquidity = MinimumLiquidity;
	type MaxLengthRoute = MaxLengthRoute;
	type GetNativeCurrencyId = NativeCurrencyId;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Source = MockDataProvider;
	type FeederOrigin = EnsureSignedBy<One, AccountId>;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
	type LiquidStakingCurrenciesProvider = LiquidStaking;
	type LiquidStakingExchangeRateProvider = LiquidStakingExchangeRateProvider;
	type VaultTokenCurrenciesFilter = TokenCurrenciesFilter;
	type VaultTokenExchangeRateProvider = TokenExchangeRateProvider;
	type VaultLoansRateProvider = VaultLoansRateProvider;
	type RelayCurrency = RelayCurrency;
	type Decimal = Decimal;
	type AMM = DefaultAMM;
	type Assets = CurrencyAdapter;
	type WeightInfo = ();
}

pub type Amount = i128;
pub type AdaptedBasicCurrency =
	bifrost_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Test {
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type MultiCurrency = Assets;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		DefaultAMM: pallet_amm::{Pallet, Call, Storage, Event<T>},
		// CurrencyAdapter: pallet_currency_adapter::{Pallet, Call},
		Currencies: bifrost_currencies::{Pallet, Call},
		Prices: crate::{Pallet, Storage, Call, Event<T>},
	}
);

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(ALICE, 100_000_000), (CHARLIE, 100_000_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		Assets::force_create(RuntimeOrigin::root(), tokens::DOT.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), tokens::SDOT.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), tokens::CDOT_7_14.into(), ALICE, true, 1)
			.unwrap();
		Assets::force_create(
			RuntimeOrigin::root(),
			tokens::LP_DOT_CDOT_7_14.into(),
			ALICE,
			true,
			1,
		)
		.unwrap();

		Assets::mint(RuntimeOrigin::signed(ALICE), tokens::DOT.into(), ALICE, 1000 * PRICE_ONE)
			.unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), tokens::SDOT.into(), ALICE, 1000 * PRICE_ONE)
			.unwrap();
		Assets::mint(
			RuntimeOrigin::signed(ALICE),
			tokens::CDOT_7_14.into(),
			ALICE,
			1000 * PRICE_ONE,
		)
		.unwrap();

		Prices::set_foreign_asset(RuntimeOrigin::signed(ALICE), tokens::LC_DOT, CDOT_7_14).unwrap();
	});

	ext
}

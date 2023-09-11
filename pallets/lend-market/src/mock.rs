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

pub use super::*;

use frame_support::{
    construct_runtime, parameter_types,
    traits::SortedMembers,
    traits::{AsEnsureOriginWithArg, Everything},
    PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned, EnsureSignedBy};
use orml_traits::{DataFeeder, DataProvider, DataProviderExtended};
use pallet_traits::{
    DecimalProvider, ExchangeRateProvider, LiquidStakingCurrenciesProvider,
    VaultTokenCurrenciesFilter, VaultTokenExchangeRateProvider,
};
use primitives::{
    tokens::{CDOT_6_13, PCDOT_6_13},
    *,
};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32};
use sp_std::vec::Vec;
use std::{cell::RefCell, collections::HashMap};

pub use primitives::tokens::{DOT, HKO, KSM, PDOT, PHKO, PKSM, PUSDT, SDOT, SKSM, USDT};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
        Loans: crate::{Pallet, Storage, Call, Event<T>},
        Prices: pallet_prices::{Pallet, Storage, Call, Event<T>},
        TimestampPallet: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        DefaultAMM: pallet_amm::{Pallet, Call, Storage, Event<T>},
        CurrencyAdapter: pallet_currency_adapter::{Pallet, Call},
    }
);

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
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub type AccountId = AccountId32;
pub type BlockNumber = u64;

pub const ALICE: AccountId = AccountId32::new([1u8; 32]);
pub const BOB: AccountId = AccountId32::new([2u8; 32]);
pub const CHARLIE: AccountId = AccountId32::new([3u8; 32]);
pub const DAVE: AccountId = AccountId32::new([4u8; 32]);
pub const EVE: AccountId = AccountId32::new([5u8; 32]);

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
    type MaxLocks = MaxLocks;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

// pallet-price is using for benchmark compilation
pub type TimeStampedPrice = orml_oracle::TimestampedValue<Price, Moment>;
pub struct MockDataProvider;
impl DataProvider<CurrencyId, TimeStampedPrice> for MockDataProvider {
    fn get(_asset_id: &CurrencyId) -> Option<TimeStampedPrice> {
        Some(TimeStampedPrice {
            value: Price::saturating_from_integer(100),
            timestamp: 0,
        })
    }
}

impl DataProviderExtended<CurrencyId, TimeStampedPrice> for MockDataProvider {
    fn get_no_op(_key: &CurrencyId) -> Option<TimeStampedPrice> {
        None
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

pub struct Decimal;
impl DecimalProvider<CurrencyId> for Decimal {
    fn get_decimal(asset_id: &CurrencyId) -> Option<u8> {
        match *asset_id {
            KSM | SKSM => Some(12),
            HKO => Some(12),
            USDT => Some(6),
            _ => None,
        }
    }
}

pub struct LiquidStaking;
impl LiquidStakingCurrenciesProvider<CurrencyId> for LiquidStaking {
    fn get_staking_currency() -> Option<CurrencyId> {
        Some(KSM)
    }
    fn get_liquid_currency() -> Option<CurrencyId> {
        Some(SKSM)
    }
}

impl ExchangeRateProvider<CurrencyId> for LiquidStaking {
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

pub struct TokenCurrenciesFilter;
impl VaultTokenCurrenciesFilter<CurrencyId> for TokenCurrenciesFilter {
    fn contains(_asset_id: &CurrencyId) -> bool {
        return false;
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
    pub const RelayCurrency: CurrencyId = KSM;
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

impl pallet_prices::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Source = MockDataProvider;
    type FeederOrigin = EnsureRoot<AccountId>;
    type UpdateOrigin = EnsureRoot<AccountId>;
    type LiquidStakingExchangeRateProvider = LiquidStaking;
    type LiquidStakingCurrenciesProvider = LiquidStaking;
    type VaultTokenCurrenciesFilter = TokenCurrenciesFilter;
    type VaultTokenExchangeRateProvider = TokenExchangeRateProvider;
    type VaultLoansRateProvider = VaultLoansRateProvider;
    type RelayCurrency = RelayCurrency;
    type Decimal = Decimal;
    type AMM = DefaultAMM;
    type Assets = CurrencyAdapter;
    type WeightInfo = ();
}

pub struct MockPriceFeeder;

impl MockPriceFeeder {
    thread_local! {
        pub static PRICES: RefCell<HashMap<CurrencyId, Option<PriceDetail>>> = {
            RefCell::new(
                vec![HKO, DOT, KSM, USDT, SKSM, SDOT, CDOT_6_13]
                    .iter()
                    .map(|&x| (x, Some((Price::saturating_from_integer(1), 1))))
                    .collect()
            )
        };
    }

    pub fn set_price(asset_id: CurrencyId, price: Price) {
        Self::PRICES.with(|prices| {
            prices.borrow_mut().insert(asset_id, Some((price, 1u64)));
        });
    }

    pub fn reset() {
        Self::PRICES.with(|prices| {
            for (_, val) in prices.borrow_mut().iter_mut() {
                *val = Some((Price::saturating_from_integer(1), 1u64));
            }
        })
    }
}

impl PriceFeeder for MockPriceFeeder {
    fn get_price(asset_id: &CurrencyId) -> Option<PriceDetail> {
        Self::PRICES.with(|prices| *prices.borrow().get(asset_id).unwrap())
    }
}

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

parameter_types! {
    pub const LoansPalletId: PalletId = PalletId(*b"par/loan");
    pub const RewardAssetId: CurrencyId = HKO;
    pub const LiquidationFreeAssetId: CurrencyId = DOT;
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type PriceFeeder = MockPriceFeeder;
    type PalletId = LoansPalletId;
    type ReserveOrigin = EnsureRoot<AccountId>;
    type UpdateOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
    type UnixTime = TimestampPallet;
    type Assets = CurrencyAdapter;
    type RewardAssetId = RewardAssetId;
    type LiquidationFreeAssetId = LiquidationFreeAssetId;
}

parameter_types! {
    pub const NativeCurrencyId: CurrencyId = HKO;
}

impl pallet_currency_adapter::Config for Test {
    type Assets = Assets;
    type Balances = Balances;
    type GetNativeCurrencyId = NativeCurrencyId;
    type LockOrigin = EnsureRoot<AccountId>;
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
    let t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        // Init assets
        Balances::set_balance(RuntimeOrigin::root(), DAVE, unit(1000), unit(0)).unwrap();
        Assets::force_create(RuntimeOrigin::root(), DOT.into(), ALICE, true, 1).unwrap();
        Assets::force_create(RuntimeOrigin::root(), KSM.into(), ALICE, true, 1).unwrap();
        Assets::force_create(RuntimeOrigin::root(), USDT.into(), ALICE, true, 1).unwrap();
        Assets::force_create(RuntimeOrigin::root(), SDOT.into(), ALICE, true, 1).unwrap();
        Assets::force_create(RuntimeOrigin::root(), CDOT_6_13.into(), ALICE, true, 1).unwrap();

        Assets::mint(RuntimeOrigin::signed(ALICE), KSM.into(), ALICE, unit(1000)).unwrap();
        Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), ALICE, unit(1000)).unwrap();
        Assets::mint(RuntimeOrigin::signed(ALICE), USDT.into(), ALICE, unit(1000)).unwrap();
        Assets::mint(
            RuntimeOrigin::signed(ALICE),
            CDOT_6_13.into(),
            ALICE,
            unit(1000),
        )
        .unwrap();
        Assets::mint(RuntimeOrigin::signed(ALICE), KSM.into(), BOB, unit(1000)).unwrap();
        Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), BOB, unit(1000)).unwrap();
        Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), DAVE, unit(1000)).unwrap();
        Assets::mint(RuntimeOrigin::signed(ALICE), USDT.into(), DAVE, unit(1000)).unwrap();

        // Init Markets
        Loans::add_market(RuntimeOrigin::root(), HKO, market_mock(PHKO)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), HKO).unwrap();
        Loans::add_market(RuntimeOrigin::root(), KSM, market_mock(PKSM)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), KSM).unwrap();
        Loans::add_market(RuntimeOrigin::root(), DOT, market_mock(PDOT)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), DOT).unwrap();
        Loans::add_market(RuntimeOrigin::root(), USDT, market_mock(PUSDT)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), USDT).unwrap();
        Loans::add_market(RuntimeOrigin::root(), CDOT_6_13, market_mock(PCDOT_6_13)).unwrap();
        Loans::activate_market(RuntimeOrigin::root(), CDOT_6_13).unwrap();

        Loans::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![CDOT_6_13]).unwrap();

        System::set_block_number(0);
        TimestampPallet::set_timestamp(6000);
    });
    ext
}

/// Progress to the given block, and then finalize the block.
pub(crate) fn _run_to_block(n: BlockNumber) {
    Loans::on_finalize(System::block_number());
    for b in (System::block_number() + 1)..=n {
        System::set_block_number(b);
        Loans::on_initialize(b);
        TimestampPallet::set_timestamp(6000 * b);
        if b != n {
            Loans::on_finalize(b);
        }
    }
}

pub fn almost_equal(target: u128, value: u128) -> bool {
    let target = target as i128;
    let value = value as i128;
    let diff = (target - value).abs() as u128;
    diff < micro_unit(1)
}

pub fn accrue_interest_per_block(asset_id: CurrencyId, block_delta_secs: u64, run_to_block: u64) {
    for i in 1..run_to_block {
        TimestampPallet::set_timestamp(6000 + (block_delta_secs * 1000 * i));
        Loans::accrue_interest(asset_id).unwrap();
    }
}

pub fn unit(d: u128) -> u128 {
    d.saturating_mul(10_u128.pow(12))
}

pub fn milli_unit(d: u128) -> u128 {
    d.saturating_mul(10_u128.pow(9))
}

pub fn micro_unit(d: u128) -> u128 {
    d.saturating_mul(10_u128.pow(6))
}

pub fn million_unit(d: u128) -> u128 {
    unit(d) * 10_u128.pow(6)
}

pub const fn market_mock(ptoken_id: u32) -> Market<Balance> {
    Market {
        close_factor: Ratio::from_percent(50),
        collateral_factor: Ratio::from_percent(50),
        liquidation_threshold: Ratio::from_percent(55),
        liquidate_incentive: Rate::from_inner(Rate::DIV / 100 * 110),
        liquidate_incentive_reserved_factor: Ratio::from_percent(3),
        state: MarketState::Pending,
        rate_model: InterestRateModel::Jump(JumpModel {
            base_rate: Rate::from_inner(Rate::DIV / 100 * 2),
            jump_rate: Rate::from_inner(Rate::DIV / 100 * 10),
            full_rate: Rate::from_inner(Rate::DIV / 100 * 32),
            jump_utilization: Ratio::from_percent(80),
        }),
        reserve_factor: Ratio::from_percent(15),
        supply_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        borrow_cap: 1_000_000_000_000_000_000_000u128, // set to 1B
        ptoken_id,
    }
}

pub const MARKET_MOCK: Market<Balance> = market_mock(1200);

pub const ACTIVE_MARKET_MOCK: Market<Balance> = {
    let mut market = MARKET_MOCK;
    market.state = MarketState::Active;
    market
};

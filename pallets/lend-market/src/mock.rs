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
	traits::{AsEnsureOriginWithArg, Everything, Nothing, SortedMembers},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned, EnsureSignedBy};
pub use node_primitives::*;
use orml_traits::{DataFeeder, DataProvider, DataProviderExtended};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, AccountId32};
use sp_std::vec::Vec;
use std::{
	cell::RefCell,
	collections::HashMap,
	hash::{Hash, Hasher},
};

pub use node_primitives::{Price, BNC, DOT, DOT_U, KSM, VBNC, VDOT, VKSM, VSKSM};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub const LDOT: CurrencyId = CurrencyId::Token2(10);
pub const LKSM: CurrencyId = CurrencyId::Token2(11);
pub const LUSDT: CurrencyId = CurrencyId::Token2(12);
pub const PHA: CurrencyId = CurrencyId::Token2(5);
pub const VPHA: CurrencyId = CurrencyId::VToken2(5);

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>},
		Currencies: bifrost_currencies::{Pallet, Call},
		AssetRegistry: bifrost_asset_registry,
		Loans: crate::{Pallet, Storage, Call, Event<T>},
		TimestampPallet: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
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

parameter_types! {
	pub const StableCurrencyId: CurrencyId = CurrencyId::Stable(TokenSymbol::KUSD);
	pub const PolkadotCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
}

impl pallet_balances::Config for Test {
	type AccountStore = frame_system::Pallet<Test>;
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxHolds = ConstU32<0>;
	type MaxFreezes = ConstU32<0>;
}

impl bifrost_asset_registry::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureSignedBy<AliceCreatePoolOrigin, AccountId>;
	type WeightInfo = ();
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			&CurrencyId::Native(TokenSymbol::BNC) => 0,
			&CurrencyId::Token(TokenSymbol::KSM) => 0,
			&CurrencyId::VToken(TokenSymbol::KSM) => 0,
			&DOT => 0,
			&VDOT => 0,
			&VBNC => 0,
			&CurrencyId::BLP(_) => 0,
			_ => 0
		}
	};
}

impl orml_tokens::Config for Test {
	type Amount = i128;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = Nothing;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type CurrencyHooks = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = BNC;
}

// pub type BlockNumber = u64;
pub type Amount = i128;
pub type AdaptedBasicCurrency =
	bifrost_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Test {
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

// pallet-price is using for benchmark compilation
pub type TimeStampedPrice = orml_oracle::TimestampedValue<Price, Moment>;
pub struct MockDataProvider;
impl DataProvider<CurrencyId, TimeStampedPrice> for MockDataProvider {
	fn get(_asset_id: &CurrencyId) -> Option<TimeStampedPrice> {
		Some(TimeStampedPrice { value: Price::saturating_from_integer(100), timestamp: 0 })
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

parameter_types! {
	pub const RelayCurrency: CurrencyId = KSM;
}

pub struct AliceCreatePoolOrigin;
impl SortedMembers<AccountId> for AliceCreatePoolOrigin {
	fn sorted_members() -> Vec<AccountId> {
		vec![ALICE]
	}
}

pub struct MockPriceFeeder;
#[derive(Encode, Decode, Clone, Copy, RuntimeDebug)]
pub struct CurrencyIdWrap(CurrencyId);

impl Hash for CurrencyIdWrap {
	fn hash<H: Hasher>(&self, state: &mut H) {
		state.write_u8(1);
	}
}

impl PartialEq for CurrencyIdWrap {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl Eq for CurrencyIdWrap {}

impl MockPriceFeeder {
	thread_local! {
		pub static PRICES: RefCell<HashMap<CurrencyIdWrap, Option<PriceDetail>>> = {
			RefCell::new(
				vec![BNC, DOT, KSM, DOT_U, VKSM, VDOT, PHA]
					.iter()
					.map(|&x| (CurrencyIdWrap(x), Some((Price::saturating_from_integer(1), 1))))
					.collect()
			)
		};
	}

	pub fn set_price(asset_id: CurrencyId, price: Price) {
		Self::PRICES.with(|prices| {
			prices.borrow_mut().insert(CurrencyIdWrap(asset_id), Some((price, 1u64)));
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
		Self::PRICES.with(|prices| *prices.borrow().get(&CurrencyIdWrap(*asset_id)).unwrap())
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
	type AssetIdParameter = CurrencyId;
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
	pub const RewardAssetId: CurrencyId = BNC;
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
	type Assets = Currencies;
	type RewardAssetId = RewardAssetId;
	type LiquidationFreeAssetId = LiquidationFreeAssetId;
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	bifrost_asset_registry::GenesisConfig::<Test> {
		currency: vec![
			// (CurrencyId::Token(TokenSymbol::DOT), 100_000_000, None),
			(CurrencyId::Token(TokenSymbol::KSM), 1, None),
			(CurrencyId::Native(TokenSymbol::BNC), 1, None),
			(DOT, 1, None),
			(ASTR, 1, None),
			(GLMR, 1, None),
			(DOT_U, 1, None),
			(PHA, 1, None),
			(VPHA, 1, None),
		],
		vcurrency: vec![VDOT],
		vsbond: vec![],
		phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let endowed_accounts: Vec<(AccountId, CurrencyId, Balance)> = vec![
		// (ALICE, BNC, 1_000_000_000_000_000),
		(ALICE, KSM, 1_000_000_000_000_000),
		(ALICE, DOT, 1_000_000_000_000_000),
		(ALICE, PHA, 1_000_000_000_000_000),
		(ALICE, DOT_U, 1_000_000_000_000_000),
		(BOB, KSM, 1_000_000_000_000_000),
		(BOB, DOT, 1_000_000_000_000_000),
		(DAVE, DOT, 1_000_000_000_000_000),
		(DAVE, DOT_U, 1_000_000_000_000_000),
	];
	pallet_balances::GenesisConfig::<Test> {
		balances: endowed_accounts
			.clone()
			.into_iter()
			.filter(|(_, currency_id, _)| *currency_id == BNC)
			.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
			.collect::<Vec<_>>(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	orml_tokens::GenesisConfig::<Test> {
		balances: endowed_accounts
			.clone()
			.into_iter()
			.filter(|(_, currency_id, _)| *currency_id != BNC)
			.collect::<Vec<_>>(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		// Init assets
		Balances::force_set_balance(RuntimeOrigin::root(), DAVE, unit(1000)).unwrap();
		Assets::force_create(RuntimeOrigin::root(), DOT.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), KSM.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), DOT_U.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), VDOT.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), PHA.into(), ALICE, true, 1).unwrap();

		Assets::mint(RuntimeOrigin::signed(ALICE), KSM.into(), ALICE, unit(1000)).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), ALICE, unit(1000)).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), DOT_U.into(), ALICE, unit(1000)).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), PHA.into(), ALICE, unit(1000)).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), KSM.into(), BOB, unit(1000)).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), BOB, unit(1000)).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), DAVE, unit(1000)).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), DOT_U.into(), DAVE, unit(1000)).unwrap();

		// Init Markets
		Loans::add_market(RuntimeOrigin::root(), BNC, market_mock(VBNC)).unwrap();
		Loans::activate_market(RuntimeOrigin::root(), BNC).unwrap();
		Loans::add_market(RuntimeOrigin::root(), KSM, market_mock(LKSM)).unwrap();
		Loans::activate_market(RuntimeOrigin::root(), KSM).unwrap();
		Loans::add_market(RuntimeOrigin::root(), DOT, market_mock(LDOT)).unwrap();
		Loans::activate_market(RuntimeOrigin::root(), DOT).unwrap();
		Loans::add_market(RuntimeOrigin::root(), DOT_U, market_mock(LUSDT)).unwrap();
		Loans::activate_market(RuntimeOrigin::root(), DOT_U).unwrap();
		Loans::add_market(RuntimeOrigin::root(), PHA, market_mock(VPHA)).unwrap();
		Loans::activate_market(RuntimeOrigin::root(), PHA).unwrap();

		Loans::update_liquidation_free_collateral(RuntimeOrigin::root(), vec![PHA]).unwrap();

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

pub const fn market_mock(lend_token_id: CurrencyId) -> Market<Balance> {
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
		lend_token_id,
	}
}

pub const MARKET_MOCK: Market<Balance> = market_mock(CurrencyId::Token2(9));

pub const ACTIVE_MARKET_MOCK: Market<Balance> = {
	let mut market = MARKET_MOCK;
	market.state = MarketState::Active;
	market
};

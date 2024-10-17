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
pub use super::*;

use crate as leverage_staking;
use bifrost_asset_registry::AssetIdMaps;
pub use bifrost_primitives::{
	currency::*, Balance, CurrencyId, CurrencyIdMapping, SlpOperator, SlpxOperator, TokenSymbol,
};
use bifrost_primitives::{
	BifrostEntranceAccount, BifrostExitAccount, IncentivePoolAccount, LendMarketPalletId, Moment,
	MoonbeamChainId, OraclePriceProvider, Price, PriceDetail, Ratio, StableAssetPalletId,
};
use bifrost_runtime_common::milli;
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{ConstU128, ConstU32, Everything, Nothing},
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use lend_market::{InterestRateModel, JumpModel, Market, MarketState};
use orml_traits::{
	location::RelativeReserveProvider, parameter_type_with_key, DataFeeder, DataProvider,
	DataProviderExtended,
};
use sp_runtime::{traits::IdentityLookup, BuildStorage, FixedPointNumber};
use std::{
	cell::RefCell,
	collections::HashMap,
	hash::{Hash, Hasher},
};
use xcm::{prelude::*, v3::Weight};
use xcm_builder::{FixedWeightBounds, FrameTransactionalProcessor};
use xcm_executor::XcmExecutor;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test{
		System: frame_system,
		Tokens: orml_tokens,
		Currencies: bifrost_currencies::{Pallet, Call},
		Balances: pallet_balances,
		XTokens: orml_xtokens::{Pallet, Call, Event<T>},
		PolkadotXcm: pallet_xcm,
		AssetRegistry: bifrost_asset_registry,
		StableAsset: bifrost_stable_asset::{Pallet, Storage, Event<T>},
		StablePool: bifrost_stable_pool,
		VtokenMinting: bifrost_vtoken_minting::{Pallet, Call, Storage, Event<T>},
		LendMarket: lend_market::{Pallet, Storage, Call, Event<T>},
		TimestampPallet: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		LeverageStaking: leverage_staking::{Pallet, Storage, Call, Event<T>},
		Prices: pallet_prices::{Pallet, Storage, Call, Event<T>},
		// PolkadotXcm: pallet_xcm,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = u128;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = BNC;
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		env_logger::try_init().unwrap_or(());

		match currency_id {
			&CurrencyId::Native(TokenSymbol::BNC) => 10 * milli::<Test>(NativeCurrencyId::get()),   // 0.01 BNC
			&CurrencyId::Token(TokenSymbol::KSM) => 0,
			&CurrencyId::VToken(TokenSymbol::KSM) => 0,
			&DOT => 0,
			&VDOT => 0,
			&VBNC => 0,
			&CurrencyId::BLP(_) => 0,
			_ => bifrost_asset_registry::AssetIdMaps::<Test>::get_currency_metadata(*currency_id)
				.map_or(Balance::max_value(), |metatata| metatata.minimal_balance)
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

pub type BlockNumber = u64;
pub type Amount = i128;
pub type AdaptedBasicCurrency =
	bifrost_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Test {
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

parameter_types! {
	// One XCM operation is 200_000_000 XcmWeight, cross-chain transfer ~= 2x of transfer = 3_000_000_000
	pub UnitWeightCost: Weight = Weight::from_parts(200_000_000, 0);
	pub const MaxInstructions: u32 = 100;
	pub UniversalLocation: InteriorLocation = Parachain(2001).into();
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	type AssetTransactor = ();
	type AssetTrap = PolkadotXcm;
	type Barrier = ();
	type RuntimeCall = RuntimeCall;
	type IsReserve = ();
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type OriginConverter = ();
	type ResponseHandler = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type Trader = ();
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmSender = ();
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = ConstU32<64>;
	type FeeManager = ();
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type AssetLocker = ();
	type AssetExchanger = ();
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = ();
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: xcm::v4::Location| -> Option<u128> {
		Some(u128::MAX)
	};
}

parameter_types! {
	pub SelfRelativeLocation: xcm::v4::Location = xcm::v4::Location::here();
	// pub const BaseXcmWeight: Weight = Weight::from_ref_time(1000_000_000u64);
	pub const MaxAssetsForTransfer: usize = 2;
	// pub UniversalLocation: InteriorLocation = Parachain(2001).into();
}

impl orml_xtokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = ();
	type AccountIdToLocation = ();
	type UniversalLocation = UniversalLocation;
	type SelfLocation = SelfRelativeLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = ();
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type LocationsFilter = Everything;
	type ReserveProvider = RelativeReserveProvider;
	type RateLimiter = ();
	type RateLimiterId = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
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
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

ord_parameter_types! {
	pub const One: u128 = 1;
}
impl bifrost_asset_registry::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureSignedBy<One, u128>;
	type WeightInfo = ();
}

pub struct EnsurePoolAssetId;
impl bifrost_stable_asset::traits::ValidateAssetId<CurrencyId> for EnsurePoolAssetId {
	fn validate(_: CurrencyId) -> bool {
		true
	}
}

impl bifrost_stable_asset::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = CurrencyId;
	type Balance = Balance;
	type Assets = Currencies;
	type PalletId = StableAssetPalletId;
	type AtLeast64BitUnsigned = u128;
	type FeePrecision = ConstU128<10_000_000_000>;
	type APrecision = ConstU128<100>;
	type PoolAssetLimit = ConstU32<5>;
	type SwapExactOverAmount = ConstU128<100>;
	type WeightInfo = ();
	type ListingOrigin = EnsureSignedBy<One, u128>;
	type EnsurePoolAssetId = EnsurePoolAssetId;
}

impl bifrost_stable_pool::Config for Test {
	type WeightInfo = ();
	type ControlOrigin = EnsureRoot<u128>;
	type CurrencyId = CurrencyId;
	type MultiCurrency = Currencies;
	type StableAsset = StableAsset;
	type VtokenMinting = VtokenMinting;
	type CurrencyIdConversion = AssetIdMaps<Test>;
	type CurrencyIdRegister = AssetIdMaps<Test>;
}

impl leverage_staking::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type ControlOrigin = EnsureRoot<u128>;
	type VtokenMinting = VtokenMinting;
	type LendMarket = LendMarket;
	type StablePoolHandler = StablePool;
	type CurrencyIdConversion = AssetIdMaps<Test>;
}

parameter_types! {
	pub const MaximumUnlockIdOfUser: u32 = 1_000;
	pub const MaximumUnlockIdOfTimeUnit: u32 = 1_000;
}

pub struct SlpxInterface;
impl SlpxOperator<Balance> for SlpxInterface {
	fn get_moonbeam_transfer_to_fee() -> Balance {
		Default::default()
	}
}

ord_parameter_types! {
	pub const RelayCurrencyId: CurrencyId = KSM;
}

impl bifrost_vtoken_minting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Tokens;
	type ControlOrigin = EnsureSignedBy<One, u128>;
	type MaximumUnlockIdOfUser = MaximumUnlockIdOfUser;
	type MaximumUnlockIdOfTimeUnit = MaximumUnlockIdOfTimeUnit;
	type EntranceAccount = BifrostEntranceAccount;
	type ExitAccount = BifrostExitAccount;
	type FeeAccount = One;
	type RedeemFeeAccount = One;
	type RelayChainToken = RelayCurrencyId;
	type WeightInfo = ();
	type OnRedeemSuccess = ();
	type XcmTransfer = XTokens;
	type MoonbeamChainId = MoonbeamChainId;
	type BifrostSlpx = SlpxInterface;
	type ChannelCommission = ();
	type MaxLockRecords = ConstU32<100>;
	type IncentivePoolAccount = IncentivePoolAccount;
	type BbBNC = ();
}

pub struct Slp;
// Functions to be called by other pallets.
impl SlpOperator<CurrencyId> for Slp {
	fn all_delegation_requests_occupied(_currency_id: CurrencyId) -> bool {
		true
	}
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

pub struct MockOraclePriceProvider;
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

impl DataFeeder<CurrencyId, TimeStampedPrice, u128> for MockDataProvider {
	fn feed_value(
		_: Option<u128>,
		_: CurrencyId,
		_: TimeStampedPrice,
	) -> sp_runtime::DispatchResult {
		Ok(())
	}
}

impl MockOraclePriceProvider {
	thread_local! {
		pub static PRICES: RefCell<HashMap<CurrencyIdWrap, Option<PriceDetail>>> = {
			RefCell::new(
				vec![BNC, DOT, KSM, DOT_U, VKSM, VDOT]
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

impl OraclePriceProvider for MockOraclePriceProvider {
	fn get_price(asset_id: &CurrencyId) -> Option<PriceDetail> {
		Self::PRICES.with(|prices| *prices.borrow().get(&CurrencyIdWrap(*asset_id)).unwrap())
	}

	fn get_amount_by_prices(
		_currency_in: &CurrencyId,
		_amount_in: Balance,
		_currency_in_price: Price,
		_currency_out: &CurrencyId,
		_currency_out_price: Price,
	) -> Option<Balance> {
		todo!()
	}

	fn get_oracle_amount_by_currency_and_amount_in(
		_currency_in: &CurrencyId,
		_amount_in: Balance,
		_currency_out: &CurrencyId,
	) -> Option<(Balance, Price, Price)> {
		todo!()
	}
}

parameter_types! {
	pub const RewardAssetId: CurrencyId = BNC;
	pub const LiquidationFreeAssetId: CurrencyId = DOT;
	pub const MaxLengthLimit: u32 = 500;
}

impl lend_market::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type OraclePriceProvider = MockOraclePriceProvider;
	type PalletId = LendMarketPalletId;
	type ReserveOrigin = EnsureRoot<u128>;
	type UpdateOrigin = EnsureRoot<u128>;
	type WeightInfo = ();
	type UnixTime = TimestampPallet;
	type Assets = Currencies;
	type RewardAssetId = RewardAssetId;
	type LiquidationFreeAssetId = LiquidationFreeAssetId;
	type MaxLengthLimit = MaxLengthLimit;
}

impl pallet_prices::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Source = MockDataProvider;
	type FeederOrigin = EnsureRoot<u128>;
	type UpdateOrigin = EnsureRoot<u128>;
	type RelayCurrency = RelayCurrencyId;
	type Assets = Currencies;
	type CurrencyIdConvert = AssetIdMaps<Test>;
	type WeightInfo = ();
}

impl pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, ()>;
	type UniversalLocation = UniversalLocation;
	type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, ()>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = ();
	type XcmTeleportFilter = Nothing;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = ConstU32<2>;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = ConstU32<8>;
	type WeightInfo = pallet_xcm::TestWeightInfo; // TODO: config after polkadot impl WeightInfo for ()
	type AdminOrigin = EnsureSignedBy<One, u128>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(u128, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { endowed_accounts: vec![] }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(u128, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn new_test_ext(self) -> Self {
		self.balances(vec![
			(0, DOT, unit(1_000_000_000_000)),
			(1, BNC, unit(1)),
			(1, DOT, unit(1000)),
			(3, DOT, unit(1000)),
		])
	}

	// Build genesis storage according to the mock runtime.
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();

		bifrost_asset_registry::GenesisConfig::<Test> {
			currency: vec![
				(CurrencyId::Token(TokenSymbol::KSM), 10_000_000, None),
				(CurrencyId::Native(TokenSymbol::BNC), 10_000_000, None),
				(DOT, 1_000_000, None),
				(ASTR, 10_000_000, None),
				(GLMR, 10_000_000, None),
			],
			vcurrency: vec![VDOT],
			vsbond: vec![],
			phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		pallet_balances::GenesisConfig::<Test> {
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

		orml_tokens::GenesisConfig::<Test> {
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

pub fn unit(d: u128) -> u128 {
	d.saturating_mul(10_u128.pow(12))
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

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
pub use super::*;

use crate as leverage_staking;
use bifrost_asset_registry::AssetIdMaps;
use bifrost_runtime_common::milli;
use frame_support::{
	ord_parameter_types, parameter_types,
	traits::{ConstU128, ConstU16, ConstU32, ConstU64, Everything, GenesisBuild, Nothing},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use lend_market::{JumpModel, Market, MarketState};
pub use node_primitives::{
	AccountId, Balance, CurrencyId, CurrencyIdMapping, SlpOperator, SlpxOperator, TokenSymbol,
	ASTR, BNC, DOT, DOT_TOKEN_ID, GLMR, VBNC, VDOT, *,
};
use orml_traits::{location::RelativeReserveProvider, parameter_type_with_key};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	FixedPointNumber,
};
use std::{
	cell::RefCell,
	collections::HashMap,
	hash::{Hash, Hasher},
};
use xcm::{
	prelude::*,
	v3::{MultiLocation, Weight},
};
use xcm_builder::FixedWeightBounds;
use xcm_executor::XcmExecutor;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Tokens: orml_tokens,
		Currencies: bifrost_currencies::{Pallet, Call},
		Balances: pallet_balances,
		XTokens: orml_xtokens::{Pallet, Call, Event<T>},
		PolkadotXcm: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin, Config},
		AssetRegistry: bifrost_asset_registry,
		StableAsset: nutsfinance_stable_asset::{Pallet, Storage, Event<T>},
		// StablePool: bifrost_stable_pool,
		VtokenMinting: bifrost_vtoken_minting::{Pallet, Call, Storage, Event<T>},
		LendMarket: lend_market::{Pallet, Storage, Call, Event<T>},
		TimestampPallet: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		LeverageStaking: leverage_staking::{Pallet, Storage, Call, Event<T>},
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u128;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
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
	pub UniversalLocation: InteriorMultiLocation = X1(Parachain(2001));
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
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(u128::MAX)
	};
}

parameter_types! {
	pub SelfRelativeLocation: MultiLocation = MultiLocation::here();
	// pub const BaseXcmWeight: Weight = Weight::from_ref_time(1000_000_000u64);
	pub const MaxAssetsForTransfer: usize = 2;
	// pub UniversalLocation: InteriorMultiLocation = X1(Parachain(2001));
}

impl orml_xtokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = ();
	type AccountIdToMultiLocation = ();
	type UniversalLocation = UniversalLocation;
	type SelfLocation = SelfRelativeLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = ();
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type MultiLocationsFilter = Everything;
	type ReserveProvider = RelativeReserveProvider;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	// pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
	// pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const StableCurrencyId: CurrencyId = CurrencyId::Stable(TokenSymbol::KUSD);
	// pub SelfParaId: u32 = ParachainInfo::parachain_id().into();
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
impl nutsfinance_stable_asset::traits::ValidateAssetId<CurrencyId> for EnsurePoolAssetId {
	fn validate(_: CurrencyId) -> bool {
		true
	}
}
parameter_types! {
	pub const StableAssetPalletId: PalletId = PalletId(*b"nuts/sta");
}

impl nutsfinance_stable_asset::Config for Test {
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

parameter_types! {
	pub const LeverageStakingPalletId: PalletId = PalletId(*b"bf/levst");
}

impl leverage_staking::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type ControlOrigin = EnsureRoot<u128>;
	type VtokenMinting = VtokenMinting;
	type LendMarket = LendMarket;
	type CurrencyIdConversion = AssetIdMaps<Test>;
	type CurrencyIdRegister = AssetIdMaps<Test>;
	type PalletId = LeverageStakingPalletId;
}

parameter_types! {
	pub const MaximumUnlockIdOfUser: u32 = 1_000;
	pub const MaximumUnlockIdOfTimeUnit: u32 = 1_000;
	pub BifrostEntranceAccount: PalletId = PalletId(*b"bf/vtkin");
	pub BifrostExitAccount: PalletId = PalletId(*b"bf/vtout");
	// pub BifrostFeeAccount: AccountId = 1.into();
}

pub struct SlpxInterface;
impl SlpxOperator<Balance> for SlpxInterface {
	fn get_moonbeam_transfer_to_fee() -> Balance {
		Default::default()
	}
}

ord_parameter_types! {
	pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
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
	type BifrostSlp = Slp;
	type RelayChainToken = RelayCurrencyId;
	type CurrencyIdConversion = AssetIdMaps<Test>;
	type CurrencyIdRegister = AssetIdMaps<Test>;
	type WeightInfo = ();
	type OnRedeemSuccess = ();
	type XcmTransfer = XTokens;
	type AstarParachainId = ConstU32<2007>;
	type MoonbeamParachainId = ConstU32<2023>;
	type BifrostSlpx = SlpxInterface;
	type HydradxParachainId = ConstU32<2034>;
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

impl PriceFeeder for MockPriceFeeder {
	fn get_price(asset_id: &CurrencyId) -> Option<PriceDetail> {
		Self::PRICES.with(|prices| *prices.borrow().get(&CurrencyIdWrap(*asset_id)).unwrap())
	}
}

parameter_types! {
	pub const LendMarketPalletId: PalletId = PalletId(*b"bf/ldmkt");
	pub const RewardAssetId: CurrencyId = BNC;
	pub const LiquidationFreeAssetId: CurrencyId = DOT;
}

impl lend_market::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PriceFeeder = MockPriceFeeder;
	type PalletId = LendMarketPalletId;
	type ReserveOrigin = EnsureRoot<u128>;
	type UpdateOrigin = EnsureRoot<u128>;
	type WeightInfo = ();
	type UnixTime = TimestampPallet;
	type Assets = Currencies;
	type RewardAssetId = RewardAssetId;
	type LiquidationFreeAssetId = LiquidationFreeAssetId;
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
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
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
	type AdminOrigin = EnsureSignedBy<One, u128>;
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
			(1, BNC, 1_000_000_000_000),
			// (1, VDOT, 100_000_000),
			(1, DOT, unit(1000)),
			// (2, VDOT, 100_000_000_000_000),
			(3, DOT, 200_000_000),
			(4, DOT, 100_000_000),
			(6, BNC, 100_000_000_000_000),
		])
	}

	// Build genesis storage according to the mock runtime.
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into();

		bifrost_asset_registry::GenesisConfig::<Test> {
			currency: vec![
				// (CurrencyId::Token(TokenSymbol::DOT), 100_000_000, None),
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
		// .into()

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

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
use crate as bifrost_stable_pool;
use bifrost_asset_registry::AssetIdMaps;
pub use bifrost_primitives::{
	currency::{MOVR, VMOVR},
	Balance, CurrencyId, CurrencyIdMapping, SlpOperator, SlpxOperator, TokenSymbol, ASTR, BNC, DOT,
	GLMR, VBNC, VDOT,
};
use bifrost_primitives::{
	BifrostEntranceAccount, BifrostExitAccount, IncentivePoolAccount, MoonbeamChainId,
	StableAssetPalletId, KSM, KUSD,
};
use bifrost_runtime_common::milli;
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{ConstU128, ConstU32, Everything, Nothing},
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use orml_traits::{location::RelativeReserveProvider, parameter_type_with_key};
use sp_runtime::{traits::IdentityLookup, BuildStorage};
use xcm::{prelude::*, v3::Weight};
use xcm_builder::{FixedWeightBounds, FrameTransactionalProcessor};
use xcm_executor::XcmExecutor;

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Tokens: orml_tokens,
		Currencies: bifrost_currencies,
		Balances: pallet_balances,
		XTokens: orml_xtokens,
		PolkadotXcm: pallet_xcm,
		AssetRegistry: bifrost_asset_registry,
		StableAsset: bifrost_stable_asset,
		StablePool: bifrost_stable_pool,
		VtokenMinting: bifrost_vtoken_minting,
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
	pub const StableCurrencyId: CurrencyId = KUSD;
	pub const PolkadotCurrencyId: CurrencyId = DOT;
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
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
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

pub fn million_unit(d: u128) -> u128 {
	d.saturating_mul(10_u128.pow(18))
}

pub fn unit(d: u128) -> u128 {
	d.saturating_mul(10_u128.pow(12))
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(u128, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn new_test_ext(self) -> Self {
		self.balances(vec![
			(0, BNC, unit(1000)),
			(0, MOVR, million_unit(1_000_000)),
			(0, VMOVR, million_unit(1_000_000)),
			(1, BNC, 1_000_000_000_000),
			(1, DOT, 100_000_000_000_000),
			(3, DOT, 200_000_000),
			(4, DOT, 100_000_000),
			(6, BNC, 100_000_000_000_000),
		])
	}

	// Build genesis storage according to the mock runtime.
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();

		bifrost_asset_registry::GenesisConfig::<Test> {
			currency: vec![
				// (CurrencyId::Token(TokenSymbol::DOT), 100_000_000, None),
				(CurrencyId::Token(TokenSymbol::KSM), 10_000_000, None),
				(CurrencyId::Native(TokenSymbol::BNC), 10_000_000, None),
				(DOT, 1_000_000, None),
				(ASTR, 10_000_000, None),
				(GLMR, 10_000_000, None),
				(MOVR, 10_000_000, None),
			],
			vcurrency: vec![VDOT, VMOVR],
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

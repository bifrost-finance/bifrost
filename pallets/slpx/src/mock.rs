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
#![cfg(test)]

use crate as slpx;
use bifrost_asset_registry::AssetIdMaps;
pub use bifrost_primitives::{
	CurrencyId, CurrencyIdMapping, MockXcmExecutor, SlpxOperator, TokenSymbol, BNC, KSM,
};
use bifrost_primitives::{MockXcmTransfer, MoonbeamChainId, SlpOperator};
use cumulus_primitives_core::ParaId;
use frame_support::{
	construct_runtime, derive_impl, ord_parameter_types,
	pallet_prelude::*,
	parameter_types,
	traits::{Everything, Nothing},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use hex_literal::hex;
use orml_traits::parameter_type_with_key;
use sp_runtime::{
	traits::{Convert, IdentityLookup},
	AccountId32, BuildStorage,
};
use sp_std::vec;
pub use xcm::latest::prelude::*;
use xcm::{latest::Location, opaque::latest::Junction::Parachain};
use xcm_builder::FrameTransactionalProcessor;
pub use xcm_builder::{EnsureXcmOrigin, FixedWeightBounds};

pub type Balance = u128;
pub type Amount = i128;
pub type BlockNumber = u64;
pub type AccountId = AccountId32;

pub const ALICE: AccountId = AccountId32::new([1u8; 32]);
pub const BOB: AccountId = AccountId32::new([2u8; 32]);

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
  pub enum Test {
	System: frame_system,
	Balances: pallet_balances,
	Tokens: orml_tokens,
	Currencies: bifrost_currencies,
	AssetRegistry: bifrost_asset_registry,
	VtokenMinting: bifrost_vtoken_minting,
	Slpx: slpx,
	PolkadotXcm: pallet_xcm,
	ParachainInfo: parachain_info,
  }
);

// Pallet system configuration
parameter_types! {
  pub const BlockHashCount: u32 = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<Balance>;
}

// Pallet balances configuration
parameter_types! {
  pub const ExistentialDeposit: u128 = 10_000_000_000;
}

impl pallet_balances::Config for Test {
	type MaxReserves = ConstU32<2>;
	type ReserveIdentifier = [u8; 8];
	type MaxLocks = ();
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
}

pub type AdaptedBasicCurrency =
	bifrost_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Test {
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

// Pallet orml-tokens configuration
parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> u128 {
		match currency_id {
			&BNC => 10 * 1_000_000_000,
			&KSM => 10 * 1_000_000_000,
			_=> 10 * 1_000_000_000
		}
	};
}
pub type ReserveIdentifier = [u8; 8];
impl orml_tokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type Amount = i128;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = ();
	type MaxLocks = ();
	type DustRemovalWhitelist = Nothing;
	type ReserveIdentifier = ReserveIdentifier;
	type MaxReserves = ConstU32<100_000>;
}

// Pallet vtoken-minting configuration
parameter_types! {
	pub const MaximumUnlockIdOfUser: u32 = 10;
	pub const MaximumUnlockIdOfTimeUnit: u32 = 50;
	pub BifrostEntranceAccount: PalletId = PalletId(*b"bf/vtkin");
	pub BifrostExitAccount: PalletId = PalletId(*b"bf/vtout");
	pub BifrostFeeAccount: AccountId = hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();
	pub const RelayCurrencyId: CurrencyId = KSM;
	pub IncentivePoolAccount: PalletId = PalletId(*b"bf/inpoo");
}

ord_parameter_types! {
	pub const One: AccountId = ALICE;
}

pub struct SlpxInterface;
impl SlpxOperator<Balance> for SlpxInterface {
	fn get_moonbeam_transfer_to_fee() -> Balance {
		Default::default()
	}
}

pub struct MockSlp;

impl<CurrencyId> SlpOperator<CurrencyId> for MockSlp {
	fn all_delegation_requests_occupied(_: CurrencyId) -> bool {
		false
	}
}

impl bifrost_vtoken_minting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
	type MaximumUnlockIdOfUser = MaximumUnlockIdOfUser;
	type MaximumUnlockIdOfTimeUnit = MaximumUnlockIdOfTimeUnit;
	type EntranceAccount = BifrostEntranceAccount;
	type ExitAccount = BifrostExitAccount;
	type FeeAccount = BifrostFeeAccount;
	type RedeemFeeAccount = BifrostFeeAccount;
	type RelayChainToken = RelayCurrencyId;
	type CurrencyIdConversion = AssetIdMaps<Test>;
	type CurrencyIdRegister = AssetIdMaps<Test>;
	type BifrostSlp = MockSlp;
	type BifrostSlpx = SlpxInterface;
	type WeightInfo = ();
	type OnRedeemSuccess = ();
	type XcmTransfer = MockXcmTransfer;
	type MoonbeamChainId = MoonbeamChainId;
	type ChannelCommission = ();
	type MaxLockRecords = ConstU32<100>;
	type IncentivePoolAccount = IncentivePoolAccount;
	type BbBNC = ();
	type AssetIdMaps = AssetIdMaps<Test>;
}

parameter_types! {
	// One XCM operation is 200_000_000 XcmWeight, cross-chain transfer ~= 2x of transfer = 3_000_000_000
	pub UnitWeightCost: Weight = Weight::from_parts(200_000_000, 0);
	pub const MaxInstructions: u32 = 100;
	pub UniversalLocation: InteriorLocation = Parachain(2001).into();
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type AssetClaims = ();
	type AssetTransactor = ();
	type AssetTrap = ();
	type Barrier = ();
	type RuntimeCall = RuntimeCall;
	type IsReserve = ();
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type OriginConverter = ();
	type ResponseHandler = ();
	type SubscriptionService = ();
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
	type Aliasers = Nothing;
	type AssetExchanger = ();
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = ();
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: Location| -> Option<u128> {
		None
	};
}

parameter_types! {
	pub SelfRelativeLocation: Location = Location::here();
	pub const BaseXcmWeight: Weight = Weight::from_parts(1000_000_000u64, 0);
	pub const MaxAssetsForTransfer: usize = 2;
}

pub struct CurrencyIdConvert<T>(sp_std::marker::PhantomData<T>);
impl<T: Get<ParaId>> Convert<CurrencyId, Option<Location>> for CurrencyIdConvert<T> {
	fn convert(id: CurrencyId) -> Option<Location> {
		AssetIdMaps::<Test>::get_location(id)
	}
}

impl<T: Get<ParaId>> Convert<Location, Option<CurrencyId>> for CurrencyIdConvert<T> {
	fn convert(location: Location) -> Option<CurrencyId> {
		AssetIdMaps::<Test>::get_currency_id(location)
	}
}

impl parachain_info::Config for Test {}

impl bifrost_asset_registry::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
}

pub struct ParachainId;
impl Get<ParaId> for ParachainId {
	fn get() -> ParaId {
		2001.into()
	}
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<Location> = Some(Parent.into());
}

impl pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, ()>;
	type UniversalLocation = UniversalLocation;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, ()>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = MockXcmExecutor;
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = bifrost_primitives::MockXcmRouter;
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
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
}

impl slpx::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type ControlOrigin = EnsureRoot<AccountId>;
	type MultiCurrency = Currencies;
	type VtokenMintingInterface = VtokenMinting;
	type XcmTransfer = MockXcmTransfer;
	type XcmSender = ();
	type CurrencyIdConvert = AssetIdMaps<Test>;
	type TreasuryAccount = BifrostFeeAccount;
	type ParachainId = ParachainId;
	type WeightInfo = ();
	type MaxOrderSize = ConstU32<500>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(0));
	ext
}

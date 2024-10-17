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

use crate as slp_v2;
use bifrost_asset_registry::AssetIdMaps;
use bifrost_primitives::{
	currency::DOT, Amount, Balance, BifrostEntranceAccount, BifrostExitAccount, BifrostFeeAccount,
	BlockNumber, CommissionPalletId, CurrencyId, IncentivePoolAccount, MockXcmRouter,
	MockXcmTransfer, SlpOperator, SlpxOperator, BNC,
};
use frame_support::{
	derive_impl,
	pallet_prelude::{ConstU32, Get},
	parameter_types,
	traits::{Everything, Nothing},
};
use frame_system as system;
use frame_system::EnsureRoot;
use pallet_xcm::EnsureResponse;
use polkadot_parachain_primitives::primitives::Id as ParaId;
use sp_core::{crypto::AccountId32, ConstU64};
use sp_runtime::{
	traits::{BlockNumberProvider, IdentityLookup},
	BuildStorage,
};
use xcm::{
	prelude::Parachain,
	v4::{InteriorLocation, Weight},
};
use xcm_builder::{FixedWeightBounds, FrameTransactionalProcessor};
use xcm_executor::XcmExecutor;

pub type AccountId = AccountId32;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		AssetRegistry: bifrost_asset_registry,
		Currencies: bifrost_currencies,
		VtokenMinting: bifrost_vtoken_minting,
		Balances: pallet_balances,
		Tokens: orml_tokens,
		PolkadotXcm: pallet_xcm,
		SlpV2: slp_v2,
	}
);

parameter_types! {
	pub const SS58Prefix: u8 = 6;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<Balance>;
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
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
}

impl bifrost_asset_registry::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
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
	type DustRemovalWhitelist = Nothing;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type CurrencyHooks = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = BNC;
}

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
	pub UniversalLocation: InteriorLocation = Parachain(2030).into();
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

impl pallet_xcm::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ExecuteXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, ()>;
	type UniversalLocation = UniversalLocation;
	type SendXcmOrigin = xcm_builder::EnsureXcmOrigin<RuntimeOrigin, ()>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = MockXcmRouter;
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

pub struct ParachainId;
impl Get<ParaId> for ParachainId {
	fn get() -> ParaId {
		2030.into()
	}
}

// Pallet vtoken-minting configuration
parameter_types! {
	pub const MaximumUnlockIdOfUser: u32 = 10;
	pub const MaximumUnlockIdOfTimeUnit: u32 = 50;
	pub const RelayCurrencyId: CurrencyId = DOT;
}

pub struct SlpxInterface;
impl SlpxOperator<Balance> for SlpxInterface {
	fn get_moonbeam_transfer_to_fee() -> Balance {
		Default::default()
	}
}

pub struct MockSlp;
impl SlpOperator<CurrencyId> for MockSlp {
	fn all_delegation_requests_occupied(_currency_id: CurrencyId) -> bool {
		true
	}
}

impl bifrost_vtoken_minting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureRoot<AccountId>;
	type MaximumUnlockIdOfUser = MaximumUnlockIdOfUser;
	type MaximumUnlockIdOfTimeUnit = MaximumUnlockIdOfTimeUnit;
	type EntranceAccount = BifrostEntranceAccount;
	type ExitAccount = BifrostExitAccount;
	type FeeAccount = BifrostFeeAccount;
	type RedeemFeeAccount = BifrostFeeAccount;
	type RelayChainToken = RelayCurrencyId;
	type BifrostSlpx = SlpxInterface;
	type WeightInfo = ();
	type OnRedeemSuccess = ();
	type XcmTransfer = MockXcmTransfer;
	type MoonbeamChainId = ConstU32<2023>;
	type ChannelCommission = ();
	type MaxLockRecords = ConstU32<100>;
	type IncentivePoolAccount = IncentivePoolAccount;
	type BbBNC = ();
}

parameter_types! {
	pub static RelaychainBlockNumber: BlockNumber = 1;

}

pub struct RelaychainDataProvider;

impl RelaychainDataProvider {
	pub fn set_block_number(block: BlockNumber) {
		RelaychainBlockNumber::set(block);
	}
}

impl BlockNumberProvider for RelaychainDataProvider {
	type BlockNumber = BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		RelaychainBlockNumber::get()
	}
}

impl slp_v2::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type XcmSender = MockXcmRouter;
	type WeightInfo = ();
	type MultiCurrency = Tokens;
	type ControlOrigin = EnsureRoot<AccountId>;
	type ParachainId = ParachainId;
	type ResponseOrigin = EnsureResponse<Everything>;
	type QueryTimeout = ConstU64<100>;
	type VtokenMinting = VtokenMinting;
	type XcmTransfer = MockXcmTransfer;
	type CurrencyIdConversion = AssetIdMaps<Test>;
	type CommissionPalletId = CommissionPalletId;
	type RelaychainBlockNumberProvider = RelaychainDataProvider;
	type MaxValidators = ConstU32<256>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub(crate) fn last_event() -> RuntimeEvent {
	system::Pallet::<Test>::events().pop().expect("Event expected").event
}

pub(crate) fn expect_event<E: Into<RuntimeEvent>>(e: E) {
	assert_eq!(last_event(), e.into());
}

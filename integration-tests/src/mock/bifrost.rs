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

use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU32, Everything, Nothing},
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use frame_system::EnsureRoot;
use orml_traits::{location::RelativeReserveProvider, parameter_type_with_key};
use sp_runtime::{traits::IdentityLookup, AccountId32};
use sp_std::prelude::*;

use crate::mock::{mock_message_queue, Amount};
use bifrost_asset_registry::AssetIdMaps;
use bifrost_polkadot_runtime::{
	xcm_config::{BaseXcmWeight, BifrostAssetTransactor, MaxAssetsForTransfer, ParachainMinFee},
	BifrostTreasuryAccount, MaxLengthLimit, MaxRefundPerBlock, MaxTypeEntryPerBlock,
	NativeCurrencyId, SubAccountIndexMultiLocationConvertor, VtokenMinting, XcmInterface,
};
use bifrost_primitives::{
	AccountIdToLocation, CurrencyId, PolkadotUniversalLocation, SelfLocation,
};
use bifrost_runtime_common::currency_converter::CurrencyIdConvert;
use bifrost_slp::QueryResponseManager;
use pallet_xcm::{QueryStatus, XcmPassthrough};
use polkadot_parachain_primitives::primitives::Sibling;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowUnpaidExecutionFrom, EnsureXcmOrigin, FixedRateOfFungible,
	FixedWeightBounds, FrameTransactionalProcessor, NativeAsset, ParentIsPreset,
	SiblingParachainConvertsVia, SignedAccountId32AsNative, SignedToAccountId32,
	SovereignSignedViaLocation,
};
use xcm_executor::{traits::QueryHandler, Config, XcmExecutor};

pub type AccountId = AccountId32;
pub type Balance = u128;
pub type BlockNumber = u64;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

parameter_types! {
	pub ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
}

impl parachain_info::Config for Runtime {}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = ();
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type DustRemovalWhitelist = Everything;
}

parameter_types! {
	pub const XtokensRateLimiterId: u8 = 0;
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = CurrencyIdConvert<ParachainInfo, Runtime>;
	type AccountIdToLocation = AccountIdToLocation;
	type SelfLocation = SelfLocation;
	type LocationsFilter = Everything;
	type MinXcmFee = ParachainMinFee;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type UniversalLocation = PolkadotUniversalLocation;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type ReserveProvider = RelativeReserveProvider;
	type RateLimiter = ();
	type RateLimiterId = ();
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4), 0);
	pub const ReservedDmpWeight: Weight = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_div(4), 0);
}

parameter_types! {
	pub const KsmLocation: Location = Location::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
}

pub type LocationToAccountId = (
	ParentIsPreset<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	AccountId32Aliases<RelayNetwork, AccountId>,
);

pub type XcmOriginToCallOrigin = (
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	pub const UnitWeightCost: Weight = Weight::from_parts(1, 1);
	pub KsmPerSecondPerByte: (AssetId, u128, u128) = (AssetId(Parent.into()), 1, 1);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}

pub type XcmRouter = super::ParachainXcmRouter<MessageQueue>;
pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct XcmConfig;
impl Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	type AssetTransactor = BifrostAssetTransactor;
	type OriginConverter = XcmOriginToCallOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = ();
	type UniversalLocation = PolkadotUniversalLocation;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader = FixedRateOfFungible<KsmPerSecondPerByte, ()>;
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetLocker = ();
	type AssetExchanger = ();
	type AssetClaims = ();
	type SubscriptionService = ();
	type PalletInstancesInfo = ();
	type FeeManager = ();
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = ();
}

impl mock_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Everything;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = PolkadotUniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	type WeightInfo = pallet_xcm::TestWeightInfo;
	type AdminOrigin = EnsureRoot<AccountId>;
}

impl bifrost_currencies::Config for Runtime {
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency =
		bifrost_currencies::BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type WeightInfo = ();
}

// impl bifrost_xcm_interface::Config for Runtime {
//     type RuntimeEvent = RuntimeEvent;
//     type UpdateOrigin = EnsureRoot<Runtime>;
//     type MultiCurrency = Currencies;
//     type RelayNetwork = bifrost_polkadot_runtime::xcm_config::RelayNetwork;
//     type RelaychainCurrencyId = RelayCurrencyId;
//     type ParachainSovereignAccount = ParachainAccount;
//     type XcmExecutor = XcmExecutor<XcmConfig>;
//     type AccountIdToLocation = BifrostAccountIdToLocation;
//     type SalpHelper = Salp;
//     type ParachainId = SelfParaChainId;
//     type CallBackTimeOut = ConstU32<10>;
//     type CurrencyIdConvert = AssetIdMaps<Runtime>;
// }

pub struct SubstrateResponseManager;
impl QueryResponseManager<QueryId, Location, BlockNumber, RuntimeCall>
	for SubstrateResponseManager
{
	fn get_query_response_record(query_id: QueryId) -> bool {
		if let Some(QueryStatus::Ready { .. }) = PolkadotXcm::query(query_id) {
			true
		} else {
			false
		}
	}

	fn create_query_record(
		responder: Location,
		call_back: Option<RuntimeCall>,
		timeout: BlockNumber,
	) -> u64 {
		if let Some(call_back) = call_back {
			PolkadotXcm::new_notify_query(responder.clone(), call_back, timeout, Here)
		} else {
			PolkadotXcm::new_query(responder, timeout, Here)
		}
	}

	fn remove_query_record(query_id: bifrost_slp::QueryId) -> bool {
		// Temporarily banned. Querries from pallet_xcm cannot be removed unless it is in ready
		// status. And we are not allowed to mannually change query status.
		// So in the manual mode, it is not possible to remove the query at all.
		// PolkadotXcm::take_response(query_id).is_some()

		PolkadotXcm::take_response(query_id);
		true
	}
}

impl bifrost_slp::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
	type VtokenMinting = VtokenMinting;
	type AccountConverter = SubAccountIndexMultiLocationConvertor;
	type ParachainId = ParachainInfo;
	type SubstrateResponseManager = SubstrateResponseManager;
	type MaxTypeEntryPerBlock = MaxTypeEntryPerBlock;
	type MaxRefundPerBlock = MaxRefundPerBlock;
	type ParachainStaking = ();
	type XcmTransfer = XTokens;
	type MaxLengthLimit = MaxLengthLimit;
	type XcmWeightAndFeeHandler = XcmInterface;
	type ChannelCommission = ();
	type StablePoolHandler = ();
	type AssetIdMaps = AssetIdMaps<Runtime>;
	type TreasuryAccount = BifrostTreasuryAccount;
}

impl bifrost_asset_registry::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
}

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime
	{
		System: frame_system,
		ParachainInfo: parachain_info,
		Balances: pallet_balances,
		MessageQueue: mock_message_queue,
		PolkadotXcm: pallet_xcm,
		Tokens: orml_tokens,
		XTokens: orml_xtokens,
		Currencies: bifrost_currencies,
		Slp: bifrost_slp,
		AssetRegistry: bifrost_asset_registry
	}
);

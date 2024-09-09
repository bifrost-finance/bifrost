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

use super::*;
use bifrost_asset_registry::{AssetIdMaps, FixedRateOfAsset};
use bifrost_primitives::{
	AccountId, AccountIdToLocation, AssetHubLocation, AssetPrefixFrom, CurrencyId,
	CurrencyIdMapping, EthereumLocation, KusamaNetwork, KusamaUniversalLocation, NativeAssetFrom,
	SelfLocation, TokenSymbol,
};
pub use bifrost_xcm_interface::traits::{parachains, XcmBaseWeight};
pub use cumulus_primitives_core::ParaId;
use frame_support::{parameter_types, sp_runtime::traits::Convert, traits::Get};
use parity_scale_codec::Encode;
pub use polkadot_parachain_primitives::primitives::Sibling;
use sp_std::convert::TryFrom;
pub use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, DescribeAllTerminal, DescribeFamily, EnsureXcmOrigin,
	FixedRateOfFungible, FixedWeightBounds, HashedDescription, IsConcrete, ParentAsSuperuser,
	ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeRevenue,
	TakeWeightCredit,
};

// orml imports
use bifrost_currencies::BasicCurrencyAdapter;
use bifrost_runtime_common::{
	currency_adapter::{BifrostDropAssets, DepositToAlternative, MultiCurrencyAdapter},
	currency_converter::CurrencyIdConvert,
};
use cumulus_primitives_core::AggregateMessageOrigin;
use frame_support::traits::TransformOrigin;
use orml_traits::{currency::MutationHooks, location::RelativeReserveProvider};
pub use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use orml_xcm_support::{IsNativeConcrete, MultiNativeAsset};
use pallet_xcm::XcmPassthrough;
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use polkadot_runtime_common::xcm_sender::NoPriceForMessageDelivery;
use sp_core::bounded::BoundedVec;
use xcm::v4::{prelude::*, Location};
use xcm_builder::{FrameTransactionalProcessor, TrailingSetTopicAsId, WithComputedOrigin};

parameter_types! {
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	// Maximum weight assigned to MessageQueue pallet to execute messages when the block is on_initialize
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
	// Maximum weight assigned to MessageQueue pallet to execute messages when the block is on_idle
	pub MessageQueueIdleServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
	// XTokens pallet BaseXcmWeight, Actually weight for an XCM message is `T::BaseXcmWeight + T::Weigher::weight(&msg)`.
	pub const BaseXcmWeight: Weight = Weight::from_parts(1000_000_000u64, 0);
	// XTokens pallet supports maximum number of assets to be transferred at a time
	pub const MaxAssetsForTransfer: usize = 2;
	// One XCM operation is 200_000_000 weight, cross-chain transfer ~= 2x of transfer = 3_000_000_000
	pub const UnitWeightCost: Weight = Weight::from_parts(200_000_000, 0);
	// Maximum number of instructions that can be executed in one XCM message
	pub const MaxInstructions: u32 = 100;
}

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch RuntimeOrigin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<KusamaNetwork, AccountId>,
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
);

/// This is the type we use to convert an (incoming) XCM origin into a local `RuntimeOrigin`
/// instance, ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind`
/// which can biases the kind of local `RuntimeOrigin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `RuntimeOrigin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<KusamaNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

pub type Barrier = TrailingSetTopicAsId<(
	// Weight that is paid for may be consumed.
	TakeWeightCredit,
	// Expected responses are OK.
	AllowKnownQueryResponses<PolkadotXcm>,
	WithComputedOrigin<
		(
			// If the message is one that immediately attemps to pay for execution, then allow it.
			AllowTopLevelPaidExecutionFrom<Everything>,
			// Subscriptions for version tracking are OK.
			AllowSubscriptionsFrom<Everything>,
		),
		KusamaUniversalLocation,
		ConstU32<8>,
	>,
)>;

pub type BifrostAssetTransactor = MultiCurrencyAdapter<
	Currencies,
	UnknownTokens,
	IsNativeConcrete<CurrencyId, CurrencyIdConvert<ParachainInfo, Runtime>>,
	AccountId,
	LocationToAccountId,
	CurrencyId,
	CurrencyIdConvert<ParachainInfo, Runtime>,
	DepositToAlternative<BifrostTreasuryAccount, Currencies, CurrencyId, AccountId, Balance>,
>;

parameter_types! {
	pub KsmPerSecond: (AssetId, u128, u128) = (Location::parent().into(), ksm_per_second::<Runtime>(),0);
	pub VksmPerSecond: (AssetId, u128,u128) = (
		Location::new(
			0,
			[Junction::from(BoundedVec::try_from(CurrencyId::VToken(TokenSymbol::KSM).encode()).unwrap())],
		).into(),
		ksm_per_second::<Runtime>(),
		0
	);
	pub VsksmPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(CurrencyId::VSToken(TokenSymbol::KSM).encode()).unwrap())]
		).into(),
		ksm_per_second::<Runtime>(),
		0
	);
	pub VsksmNewPerSecond: (AssetId, u128,u128) = (
		Location::new(
			0,
			[Junction::from(BoundedVec::try_from(CurrencyId::VSToken(TokenSymbol::KSM).encode()).unwrap())]
		).into(),
		ksm_per_second::<Runtime>(),
		0
	);
	pub BncPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(NativeCurrencyId::get().encode()).unwrap())]
		).into(),
		// BNC:KSM = 80:1
		ksm_per_second::<Runtime>() * 80,
		0
	);
	pub BncNewPerSecond: (AssetId, u128,u128) = (
		Location::new(
			0,
			[Junction::from(BoundedVec::try_from(NativeCurrencyId::get().encode()).unwrap())]
		).into(),
		// BNC:KSM = 80:1
		ksm_per_second::<Runtime>() * 80,
		0
	);

	pub ZlkPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(SelfParaId::get()), Junction::from(BoundedVec::try_from(CurrencyId::Token(TokenSymbol::ZLK).encode()).unwrap())]
		).into(),
		// ZLK:KSM = 150:1
		//ZLK has a decimal of 18, while KSM is 12.
		ksm_per_second::<Runtime>() * 150 * 1_000_000,
		0
	);
	pub ZlkNewPerSecond: (AssetId, u128,u128) = (
		Location::new(
			0,
			[Junction::from(BoundedVec::try_from(CurrencyId::Token(TokenSymbol::ZLK).encode()).unwrap())]
		).into(),
		// ZLK:KSM = 150:1
		//ZLK has a decimal of 18, while KSM is 12.
		ksm_per_second::<Runtime>() * 150 * 1_000_000,
		0
	);
	pub KarPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::karura::ID), Junction::from(BoundedVec::try_from(parachains::karura::KAR_KEY.to_vec()).unwrap())]
		).into(),
		// KAR:KSM = 100:1
		ksm_per_second::<Runtime>() * 100,
		0
	);
	pub KusdPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::karura::ID), Junction::from(BoundedVec::try_from(parachains::karura::KUSD_KEY.to_vec()).unwrap())]
		).into(),
		// kUSD:KSM = 400:1
		ksm_per_second::<Runtime>() * 400,
		0
	);
	pub PhaPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::phala::ID)],
		).into(),
		// PHA:KSM = 400:1
		ksm_per_second::<Runtime>() * 400,
		0
	);
	pub RmrkPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::Statemine::ID), GeneralIndex(parachains::Statemine::RMRK_ID.into())]
		).into(),
		// rmrk:KSM = 10:1
		ksm_per_second::<Runtime>() * 10 / 100, //rmrk currency decimal as 10
		0
	);
	pub RmrkNewPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::Statemine::ID), PalletInstance(parachains::Statemine::PALLET_ID),GeneralIndex(parachains::Statemine::RMRK_ID.into())]
		).into(),
		// rmrk:KSM = 10:1
		ksm_per_second::<Runtime>() * 10 / 100, //rmrk currency decimal as 10
		0
	);
	pub MovrPerSecond: (AssetId, u128,u128) = (
		Location::new(
			1,
			[Parachain(parachains::moonriver::ID), PalletInstance(parachains::moonriver::PALLET_ID.into())]
		).into(),
		// MOVR:KSM = 2.67:1
		ksm_per_second::<Runtime>() * 267 * 10_000, //movr currency decimal as 18
		0
	);
	pub BasePerSecond: u128 = ksm_per_second::<Runtime>();
}

pub struct ToTreasury;
impl TakeRevenue for ToTreasury {
	fn take_revenue(revenue: Asset) {
		if let Asset { id: AssetId(location), fun: Fungible(amount) } = revenue {
			if let Some(currency_id) =
				CurrencyIdConvert::<ParachainInfo, Runtime>::convert(location)
			{
				let _ = Currencies::deposit(currency_id, &BifrostTreasuryAccount::get(), amount);
			}
		}
	}
}

pub type Trader = (
	FixedRateOfFungible<KsmPerSecond, ToTreasury>,
	FixedRateOfFungible<VksmPerSecond, ToTreasury>,
	FixedRateOfFungible<VsksmPerSecond, ToTreasury>,
	FixedRateOfFungible<VsksmNewPerSecond, ToTreasury>,
	FixedRateOfFungible<BncPerSecond, ToTreasury>,
	FixedRateOfFungible<BncNewPerSecond, ToTreasury>,
	FixedRateOfFungible<ZlkPerSecond, ToTreasury>,
	FixedRateOfFungible<ZlkNewPerSecond, ToTreasury>,
	FixedRateOfFungible<KarPerSecond, ToTreasury>,
	FixedRateOfFungible<KusdPerSecond, ToTreasury>,
	FixedRateOfFungible<PhaPerSecond, ToTreasury>,
	FixedRateOfFungible<RmrkPerSecond, ToTreasury>,
	FixedRateOfFungible<RmrkNewPerSecond, ToTreasury>,
	FixedRateOfFungible<MovrPerSecond, ToTreasury>,
	FixedRateOfAsset<Runtime, BasePerSecond, ToTreasury>,
);

/// A call filter for the XCM Transact instruction. This is a temporary measure until we properly
/// account for proof size weights.
///
/// Calls that are allowed through this filter must:
/// 1. Have a fixed weight;
/// 2. Cannot lead to another call being made;
/// 3. Have a defined proof size weight, e.g. no unbounded vecs in call parameters.
pub struct SafeCallFilter;
impl Contains<RuntimeCall> for SafeCallFilter {
	fn contains(call: &RuntimeCall) -> bool {
		#[cfg(feature = "runtime-benchmarks")]
		{
			if matches!(call, RuntimeCall::System(frame_system::Call::remark_with_event { .. })) {
				return true;
			}
		}

		match call {
			RuntimeCall::System(
				frame_system::Call::kill_prefix { .. } | frame_system::Call::set_heap_pages { .. },
			) |
			RuntimeCall::Timestamp(..) |
			RuntimeCall::Indices(..) |
			RuntimeCall::Balances(..) |
			RuntimeCall::ParachainStaking(..) |
			RuntimeCall::Session(pallet_session::Call::purge_keys { .. }) |
			RuntimeCall::Treasury(..) |
			RuntimeCall::Utility(pallet_utility::Call::as_derivative { .. }) |
			RuntimeCall::Identity(
				pallet_identity::Call::add_registrar { .. } |
				pallet_identity::Call::set_identity { .. } |
				pallet_identity::Call::clear_identity { .. } |
				pallet_identity::Call::request_judgement { .. } |
				pallet_identity::Call::cancel_request { .. } |
				pallet_identity::Call::set_fee { .. } |
				pallet_identity::Call::set_account_id { .. } |
				pallet_identity::Call::set_fields { .. } |
				pallet_identity::Call::provide_judgement { .. } |
				pallet_identity::Call::kill_identity { .. } |
				pallet_identity::Call::add_sub { .. } |
				pallet_identity::Call::rename_sub { .. } |
				pallet_identity::Call::remove_sub { .. } |
				pallet_identity::Call::quit_sub { .. },
			) |
			RuntimeCall::Vesting(..) |
			RuntimeCall::PolkadotXcm(pallet_xcm::Call::limited_reserve_transfer_assets { .. }) |
			RuntimeCall::Proxy(..) |
			RuntimeCall::Tokens(
				orml_tokens::Call::transfer { .. } |
				orml_tokens::Call::transfer_all { .. } |
				orml_tokens::Call::transfer_keep_alive { .. }
			) |
			// Bifrost moudule
			RuntimeCall::Farming(
				bifrost_farming::Call::claim { .. } |
				bifrost_farming::Call::deposit { .. } |
				bifrost_farming::Call::withdraw { .. } |
				bifrost_farming::Call::withdraw_claim { .. }
			) |
			RuntimeCall::Salp(
				bifrost_salp::Call::contribute { .. } |
				bifrost_salp::Call::batch_unlock { .. } |
				bifrost_salp::Call::redeem { .. } |
				bifrost_salp::Call::unlock { .. } |
				bifrost_salp::Call::unlock_by_vsbond { .. } |
				bifrost_salp::Call::unlock_vstoken { .. }
			) |
			RuntimeCall::VSBondAuction(
				bifrost_vsbond_auction::Call::clinch_order { .. } |
				bifrost_vsbond_auction::Call::create_order { .. } |
				bifrost_vsbond_auction::Call::partial_clinch_order { .. } |
				bifrost_vsbond_auction::Call::revoke_order { .. }
			) |
			RuntimeCall::VstokenConversion(
				bifrost_vstoken_conversion::Call::vsbond_convert_to_vstoken { .. } |
				bifrost_vstoken_conversion::Call::vstoken_convert_to_vsbond { .. }
			) |
			RuntimeCall::VtokenMinting(
				bifrost_vtoken_minting::Call::mint { .. } |
				bifrost_vtoken_minting::Call::rebond { .. } |
				bifrost_vtoken_minting::Call::rebond_by_unlock_id { .. } |
				bifrost_vtoken_minting::Call::redeem { .. }
			) |
			RuntimeCall::XcmInterface(
				bifrost_xcm_interface::Call::transfer_statemine_assets { .. }
			) |
			RuntimeCall::Slpx(..) |
			RuntimeCall::ZenlinkProtocol(
				zenlink_protocol::Call::add_liquidity { .. } |
				zenlink_protocol::Call::remove_liquidity { .. } |
				zenlink_protocol::Call::transfer { .. }
			) |
			RuntimeCall::ZenlinkStableAMM(
				zenlink_stable_amm::Call::remove_liquidity_one_currency { .. } |
				zenlink_stable_amm::Call::remove_pool_and_base_pool_liquidity_one_currency { .. } |
				zenlink_stable_amm::Call::swap { .. } |
				zenlink_stable_amm::Call::swap_pool_to_base { .. } |
				zenlink_stable_amm::Call::swap_meta_pool_underlying { .. } |
				zenlink_stable_amm::Call::withdraw_admin_fee { .. }
			) => true,
			_ => false,
		}
	}
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	type AssetTransactor = BifrostAssetTransactor;
	type AssetTrap = BifrostDropAssets<ToTreasury>;
	type Barrier = Barrier;
	type RuntimeCall = RuntimeCall;
	type IsReserve = (
		NativeAssetFrom<AssetHubLocation>,
		AssetPrefixFrom<EthereumLocation, AssetHubLocation>,
		MultiNativeAsset<RelativeReserveProvider>,
	);
	type IsTeleporter = ();
	type UniversalLocation = KusamaUniversalLocation;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type ResponseHandler = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type Trader = Trader;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmSender = XcmRouter;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = ConstU32<8>;
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = SafeCallFilter;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = ();
	type MessageExporter = ();
	type Aliasers = Nothing;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type XcmRecorder = ();
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
}

/// Local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, KusamaNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, ()>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<Location> = Some(Parent.into());
}

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type UniversalLocation = KusamaUniversalLocation;
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type XcmExecuteFilter = Nothing;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmExecutor = bifrost_primitives::MockXcmExecutor;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmReserveTransferFilter = Everything;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmRouter = bifrost_primitives::MockXcmRouter;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmRouter = XcmRouter;
	type XcmTeleportFilter = Nothing;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = ();
	type MaxLockers = ConstU32<8>;
	type WeightInfo = weights::pallet_xcm::WeightInfo<Runtime>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type ChannelInfo = ParachainSystem;
	type RuntimeEvent = RuntimeEvent;
	type VersionWrapper = PolkadotXcm;
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
	type MaxInboundSuspended = ConstU32<1_000>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Runtime>;
	type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
	type MaxActiveOutboundChannels = ConstU32<128>;
	type MaxPageSize = ConstU32<{ 103 * 1024 }>;
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_message_queue::weights::SubstrateWeight<Self>;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor =
		pallet_message_queue::mock_helpers::NoopMessageProcessor<AggregateMessageOrigin>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = ConstU32<{ 64 * 1024 }>;
	type MaxStale = ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = MessageQueueIdleServiceWeight;
}

// orml runtime start

impl bifrost_currencies::Config for Runtime {
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type WeightInfo = weights::bifrost_currencies::WeightInfo<Runtime>;
}

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			&CurrencyId::Native(TokenSymbol::BNC) => 10 * milli::<Runtime>(NativeCurrencyId::get()),   // 0.01 BNC
			&CurrencyId::Stable(TokenSymbol::KUSD) => 10 * millicent::<Runtime>(StableCurrencyId::get()),
			&CurrencyId::Token(TokenSymbol::KSM) => 10 * millicent::<Runtime>(RelayCurrencyId::get()),  // 0.0001 KSM
			&CurrencyId::Token(TokenSymbol::KAR) => 10 * millicent::<Runtime>(CurrencyId::Token(TokenSymbol::KAR)),
			&CurrencyId::Token(TokenSymbol::DOT) => 1 * cent::<Runtime>(PolkadotCurrencyId::get()),  // DOT has a decimals of 10e10, 0.01 DOT
			&CurrencyId::Token(TokenSymbol::ZLK) => 1 * micro::<Runtime>(CurrencyId::Token(TokenSymbol::ZLK)),	// ZLK has a decimals of 10e18
			&CurrencyId::Token(TokenSymbol::PHA) => 4 * cent::<Runtime>(CurrencyId::Token(TokenSymbol::PHA)),	// 0.04 PHA, PHA has a decimals of 10e12.
			&CurrencyId::VSToken(TokenSymbol::KSM) => 10 * millicent::<Runtime>(RelayCurrencyId::get()),
			&CurrencyId::VSToken(TokenSymbol::DOT) => 1 * cent::<Runtime>(PolkadotCurrencyId::get()),
			&CurrencyId::VSBond(TokenSymbol::BNC, ..) => 10 * millicent::<Runtime>(NativeCurrencyId::get()),
			&CurrencyId::VSBond(TokenSymbol::KSM, ..) => 10 * millicent::<Runtime>(RelayCurrencyId::get()),
			&CurrencyId::VSBond(TokenSymbol::DOT, ..) => 1 * cent::<Runtime>(PolkadotCurrencyId::get()),
			&CurrencyId::LPToken(..) => 1 * micro::<Runtime>(NativeCurrencyId::get()),
			&CurrencyId::StableLpToken(..) => 10 * millicent::<Runtime>(NativeCurrencyId::get()),
			&CurrencyId::VToken(TokenSymbol::KSM) => 10 * millicent::<Runtime>(RelayCurrencyId::get()),  // 0.0001 vKSM
			&CurrencyId::Token(TokenSymbol::RMRK) => 1 * micro::<Runtime>(CurrencyId::Token(TokenSymbol::RMRK)),
			&CurrencyId::Token(TokenSymbol::MOVR) => 1 * micro::<Runtime>(CurrencyId::Token(TokenSymbol::MOVR)),	// MOVR has a decimals of 10e18
			&CurrencyId::VToken(TokenSymbol::MOVR) => 1 * micro::<Runtime>(CurrencyId::Token(TokenSymbol::MOVR)),	// MOVR has a decimals of 10e18
			CurrencyId::ForeignAsset(foreign_asset_id) => {
				AssetIdMaps::<Runtime>::get_asset_metadata(AssetIds::ForeignAssetId(*foreign_asset_id)).
					map_or(Balance::max_value(), |metatata| metatata.minimal_balance)
			},
			_ => AssetIdMaps::<Runtime>::get_currency_metadata(*currency_id)
				.map_or(Balance::max_value(), |metatata| metatata.minimal_balance)
		}
	};
}

pub struct DustRemovalWhitelist;
impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(a: &AccountId) -> bool {
		let whitelist: Vec<AccountId> = vec![
			TreasuryPalletId::get().into_account_truncating(),
			BifrostCrowdloanId::get().into_account_truncating(),
			BifrostVsbondPalletId::get().into_account_truncating(),
			SlpEntrancePalletId::get().into_account_truncating(),
			SlpExitPalletId::get().into_account_truncating(),
			BuybackPalletId::get().into_account_truncating(),
			SystemMakerPalletId::get().into_account_truncating(),
			ZenklinkFeeAccount::get(),
			CommissionPalletId::get().into_account_truncating(),
			VsbondAuctionPalletId::get().into_account_truncating(),
			ParachainStakingPalletId::get().into_account_truncating(),
			SystemStakingPalletId::get().into_account_truncating(),
		];
		whitelist.contains(a) ||
			FarmingKeeperPalletId::get().check_sub_account::<PoolId>(a) ||
			FarmingRewardIssuerPalletId::get().check_sub_account::<PoolId>(a) ||
			FeeSharePalletId::get().check_sub_account::<DistributionId>(a)
	}
}

parameter_types! {
	pub BifrostTreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
	// gVLo8SqxQsm11cXpkFJnaqXhAd6qtxwi2DhxfUFE7pSiyoi
	pub ZenklinkFeeAccount: AccountId = hex!["d2ca9ceb400cc68dcf58de4871bd261406958fd17338d2d82ad2592db62e6a2a"].into();
}

pub struct CurrencyHooks;
impl MutationHooks<AccountId, CurrencyId, Balance> for CurrencyHooks {
	type OnDust = orml_tokens::TransferDust<Runtime, BifrostTreasuryAccount>;
	type OnSlash = ();
	type PreDeposit = ();
	type PostDeposit = ();
	type PreTransfer = ();
	type PostTransfer = ();
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}

impl orml_tokens::Config for Runtime {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = DustRemovalWhitelist;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = weights::orml_tokens::WeightInfo<Runtime>;
	type CurrencyHooks = CurrencyHooks;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: Location| -> Option<u128> {
		Some(u128::MAX)
	};
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = CurrencyIdConvert<ParachainInfo, Runtime>;
	type AccountIdToLocation = AccountIdToLocation;
	type UniversalLocation = KusamaUniversalLocation;
	type SelfLocation = SelfLocation;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmExecutor = bifrost_primitives::MockXcmExecutor;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type LocationsFilter = Everything;
	type ReserveProvider = RelativeReserveProvider;
	type RateLimiter = ();
	type RateLimiterId = ();
}

impl orml_unknown_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
}

impl orml_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type SovereignOrigin = MoreThanHalfCouncil;
}

parameter_types! {
	pub ParachainAccount: AccountId = ParachainInfo::get().into_account_truncating();

}

impl bifrost_xcm_interface::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type UpdateOrigin = TechAdminOrCouncil;
	type MultiCurrency = Currencies;
	type RelayNetwork = KusamaNetwork;
	type RelaychainCurrencyId = RelayCurrencyId;
	type ParachainSovereignAccount = ParachainAccount;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmExecutor = bifrost_primitives::MockXcmExecutor;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type AccountIdToLocation = AccountIdToLocation;
	type SalpHelper = Salp;
	type ParachainId = ParachainInfo;
	type CallBackTimeOut = ConstU32<10>;
	type CurrencyIdConvert = AssetIdMaps<Runtime>;
}

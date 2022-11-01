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

//! The Bifrost Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use core::convert::TryInto;

use bifrost_slp::QueryResponseManager;
// A few exports that help ease life for downstream crates.
use cumulus_pallet_parachain_system::RelayNumberStrictlyIncreases;
pub use frame_support::{
	construct_runtime, match_types, parameter_types,
	traits::{
		Contains, EqualPrivilegeOnly, Everything, Imbalance, InstanceFilter, IsInVec,
		LockIdentifier, NeverEnsureOrigin, Nothing, OnUnbalanced, Randomness,
	},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		ConstantMultiplier, DispatchClass, IdentityFee, Weight,
	},
	PalletId, RuntimeDebug, StorageValue,
};
use frame_system::limits::{BlockLength, BlockWeights};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use pallet_xcm::QueryStatus;
use sp_api::impl_runtime_apis;
use sp_core::OpaqueMetadata;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdConversion, BlakeTwo256, Block as BlockT, Zero},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, DispatchError, DispatchResult, Perbill, Permill,
};
use sp_std::{marker::PhantomData, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

/// Constant values used within the runtime.
pub mod constants;
use bifrost_asset_registry::AssetIdMaps;
use bifrost_flexible_fee::{
	fee_dealer::FeeDealer,
	misc_fees::{ExtraFeeMatcher, MiscFeeHandler, NameGetter},
};
use bifrost_runtime_common::{
	constants::time::*, dollar, milli, millicent, AuraId, CouncilCollective,
	EnsureRootOrAllTechnicalCommittee, MoreThanHalfCouncil, SlowAdjustingFeeUpdate,
	TechnicalCollective,
};
use bifrost_slp::QueryId;
use codec::{Decode, Encode, MaxEncodedLen};
use constants::currency::*;
use cumulus_primitives_core::ParaId as CumulusParaId;
use frame_support::{
	sp_runtime::traits::Convert,
	traits::{EitherOfDiverse, Get},
};
use frame_system::EnsureRoot;
use hex_literal::hex;
pub use node_primitives::{
	traits::{CheckSubAccount, FarmingInfo, VtokenMintingInterface, VtokenMintingOperator},
	AccountId, Amount, AssetIds, Balance, BlockNumber, CurrencyId, CurrencyIdMapping,
	DistributionId, ExtraFeeName, Moment, Nonce, ParaId, PoolId, RpcContributionStatus, TimeUnit,
	TokenSymbol, DOT_TOKEN_ID, GLMR_TOKEN_ID,
};
// orml imports
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use orml_xcm_support::{DepositToAlternative, MultiCurrencyAdapter};
use pallet_xcm::XcmPassthrough;
// XCM imports
use polkadot_parachain::primitives::Sibling;
use sp_arithmetic::Percent;
use sp_runtime::traits::ConvertInto;
use static_assertions::const_assert;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, CurrencyAdapter as XcmCurrencyAdapter, EnsureXcmOrigin,
	FixedRateOfFungible, FixedWeightBounds, IsConcrete, LocationInverter, ParentAsSuperuser,
	ParentIsPreset, RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeRevenue,
	TakeWeightCredit,
};
use xcm_executor::{Config, XcmExecutor};
pub use xcm_interface::traits::{parachains, XcmBaseWeight};
// zenlink imports
use zenlink_protocol::{
	make_x2_location, AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler,
	MultiAssetsHandler, PairInfo, ZenlinkMultiAssets,
};
// Weights used in the runtime.
mod weights;

mod xcm_config;

use xcm_config::{
	BifrostAccountIdToMultiLocation, BifrostAssetMatcher, BifrostCurrencyIdConvert,
	MultiNativeAsset,
};

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("bifrost_polkadot"),
	impl_name: create_runtime_str!("bifrost_polkadot"),
	authoring_version: 0,
	spec_version: 962,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 0,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for .5 seconds of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND / 2;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u8 = 6;
}

pub struct CallFilter;
impl Contains<Call> for CallFilter {
	fn contains(call: &Call) -> bool {
		let is_core_call =
			matches!(call, Call::System(_) | Call::Timestamp(_) | Call::ParachainSystem(_));
		if is_core_call {
			// always allow core call
			return true;
		}

		if bifrost_call_switchgear::OverallToggleFilter::<Runtime>::get_overall_toggle_status() {
			return false;
		}

		// temporarily ban PhragmenElection
		let is_temporarily_banned = matches!(call, Call::PhragmenElection(_));

		if is_temporarily_banned {
			return false;
		}

		let is_switched_off =
			bifrost_call_switchgear::SwitchOffTransactionFilter::<Runtime>::contains(call);
		if is_switched_off {
			// no switched off call
			return false;
		}

		// disable transfer
		let is_transfer = matches!(call, Call::Currencies(_) | Call::Tokens(_) | Call::Balances(_));
		if is_transfer {
			let is_disabled = match *call {
				// orml-currencies module
				Call::Currencies(orml_currencies::Call::transfer {
					dest: _,
					currency_id,
					amount: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&currency_id,
				),
				Call::Currencies(orml_currencies::Call::transfer_native_currency {
					dest: _,
					amount: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&NativeCurrencyId::get(),
				),
				// orml-tokens module
				Call::Tokens(orml_tokens::Call::transfer { dest: _, currency_id, amount: _ }) =>
					bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
						&currency_id,
					),
				Call::Tokens(orml_tokens::Call::transfer_all {
					dest: _,
					currency_id,
					keep_alive: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&currency_id,
				),
				Call::Tokens(orml_tokens::Call::transfer_keep_alive {
					dest: _,
					currency_id,
					amount: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&currency_id,
				),
				// Balances module
				Call::Balances(pallet_balances::Call::transfer { dest: _, value: _ }) =>
					bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
						&NativeCurrencyId::get(),
					),
				Call::Balances(pallet_balances::Call::transfer_keep_alive {
					dest: _,
					value: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&NativeCurrencyId::get(),
				),
				Call::Balances(pallet_balances::Call::transfer_all { dest: _, keep_alive: _ }) =>
					bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
						&NativeCurrencyId::get(),
					),
				_ => false,
			};

			if is_disabled {
				// no switched off call
				return false;
			}
		}

		true
	}
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
	pub const RelayCurrencyId: CurrencyId = CurrencyId::Token2(DOT_TOKEN_ID);
	pub SelfParaId: u32 = ParachainInfo::parachain_id().into();
}

parameter_types! {
	pub const TreasuryPalletId: PalletId = PalletId(*b"bf/trsry");
	pub const BifrostCrowdloanId: PalletId = PalletId(*b"bf/salp#");
	pub const MerkleDirtributorPalletId: PalletId = PalletId(*b"bf/mklds");
	pub const BifrostVsbondPalletId: PalletId = PalletId(*b"bf/salpb");
	pub const SlpEntrancePalletId: PalletId = PalletId(*b"bf/vtkin");
	pub const SlpExitPalletId: PalletId = PalletId(*b"bf/vtout");
	pub const FarmingKeeperPalletId: PalletId = PalletId(*b"bf/fmkpr");
	pub const FarmingRewardIssuerPalletId: PalletId = PalletId(*b"bf/fmrir");
	pub const BuybackPalletId: PalletId = PalletId(*b"bf/salpc");
	pub const SystemMakerPalletId: PalletId = PalletId(*b"bf/sysmk");
	pub const FeeSharePalletId: PalletId = PalletId(*b"bf/feesh");
}

impl frame_system::Config for Runtime {
	type AccountData = pallet_balances::AccountData<Balance>;
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	type BaseCallFilter = CallFilter;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	type BlockLength = RuntimeBlockLength;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	type BlockWeights = RuntimeBlockWeights;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	type DbWeight = RocksDbWeight;
	/// The ubiquitous event type.
	type Event = Event;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = Indices;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	type SS58Prefix = SS58Prefix;
	type SystemWeightInfo = ();
	/// Runtime version.
	type Version = Version;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	type MinimumPeriod = MinimumPeriod;
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = ();
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 10 * MILLIBNC;
	pub const TransferFee: Balance = 1 * MILLIBNC;
	pub const CreationFee: Balance = 1 * MILLIBNC;
	pub const TransactionByteFee: Balance = 16 * MICROBNC;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_utility::Config for Runtime {
	type Call = Call;
	type Event = Event;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const MaxProxies: u16 = 32;
	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
	pub const MaxPending: u16 = 32;
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	Any = 0,
	NonTransfer = 1,
	Governance = 2,
	CancelProxy = 3,
	IdentityJudgement = 4,
}

impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<Call> for ProxyType {
	fn filter(&self, c: &Call) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => matches!(
				c,
				Call::System(..) |
				Call::Scheduler(..) |
				Call::Preimage(_) |
				Call::Timestamp(..) |
				Call::Indices(pallet_indices::Call::claim{..}) |
				Call::Indices(pallet_indices::Call::free{..}) |
				Call::Indices(pallet_indices::Call::freeze{..}) |
				// Specifically omitting Indices `transfer`, `force_transfer`
				// Specifically omitting the entire Balances pallet
				Call::Authorship(..) |
				Call::Session(..) |
				Call::Democracy(..) |
				Call::Council(..) |
				Call::TechnicalCommittee(..) |
				Call::PhragmenElection(..) |
				Call::TechnicalMembership(..) |
				Call::Treasury(..) |
				Call::Bounties(..) |
				Call::Tips(..) |
				Call::Vesting(pallet_vesting::Call::vest{..}) |
				Call::Vesting(pallet_vesting::Call::vest_other{..}) |
				// Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
				Call::Utility(..) |
				Call::Proxy(..) |
				Call::Multisig(..)
			),
			ProxyType::Governance => matches!(
				c,
				Call::Democracy(..) |
					Call::Council(..) | Call::TechnicalCommittee(..) |
					Call::PhragmenElection(..) |
					Call::Treasury(..) | Call::Bounties(..) |
					Call::Tips(..) | Call::Utility(..)
			),
			ProxyType::CancelProxy => {
				matches!(c, Call::Proxy(pallet_proxy::Call::reject_announcement { .. }))
			},
			ProxyType::IdentityJudgement => matches!(
				c,
				Call::Identity(pallet_identity::Call::provide_judgement { .. }) | Call::Utility(..)
			),
		}
	}

	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
	type Call = Call;
	type CallHasher = BlakeTwo256;
	type Currency = Balances;
	type Event = Event;
	type MaxPending = MaxPending;
	type MaxProxies = MaxProxies;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type ProxyType = ProxyType;
	type WeightInfo = ();
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub PreimageBaseDeposit: Balance = deposit(2, 64);
	pub PreimageByteDeposit: Balance = deposit(0, 1);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type Event = Event;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type MaxSize = PreimageMaxSize;
	type BaseDeposit = PreimageBaseDeposit;
	type ByteDeposit = PreimageByteDeposit;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl pallet_scheduler::Config for Runtime {
	type Call = Call;
	type Event = Event;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type MaximumWeight = MaximumSchedulerWeight;
	type Origin = Origin;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type PalletsOrigin = OriginCaller;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
	type PreimageProvider = Preimage;
	type NoPreimagePostponement = NoPreimagePostponement;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
	type Call = Call;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type Event = Event;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = ();
}

parameter_types! {
	// Minimum 4 CENTS/byte
	pub const BasicDeposit: Balance = deposit(1, 258);
	pub const FieldDeposit: Balance = deposit(0, 66);
	pub const SubAccountDeposit: Balance = deposit(1, 53);
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type FieldDeposit = FieldDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxAdditionalFields = MaxAdditionalFields;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = MoreThanHalfCouncil;
	type RegistrarOrigin = MoreThanHalfCouncil;
	type WeightInfo = ();
}

parameter_types! {
	pub const IndexDeposit: Balance = 10 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type Event = Event;
	type WeightInfo = ();
}

impl pallet_balances::Config for Runtime {
	type AccountStore = System;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = Treasury;
	/// The ubiquitous event type.
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

impl pallet_collective::Config<CouncilCollective> for Runtime {
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type Event = Event;
	type MaxMembers = CouncilMaxMembers;
	type MaxProposals = CouncilMaxProposals;
	type MotionDuration = CouncilMotionDuration;
	type Origin = Origin;
	type Proposal = Call;
	type WeightInfo = ();
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 7 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type Event = Event;
	type MaxMembers = TechnicalMaxMembers;
	type MaxProposals = TechnicalMaxProposals;
	type MotionDuration = TechnicalMotionDuration;
	type Origin = Origin;
	type Proposal = Call;
	type WeightInfo = ();
}

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
	type AddOrigin = MoreThanHalfCouncil;
	type Event = Event;
	type MaxMembers = CouncilMaxMembers;
	type MembershipChanged = Council;
	type MembershipInitialized = Council;
	type PrimeOrigin = MoreThanHalfCouncil;
	type RemoveOrigin = MoreThanHalfCouncil;
	type ResetOrigin = MoreThanHalfCouncil;
	type SwapOrigin = MoreThanHalfCouncil;
	type WeightInfo = ();
}

impl pallet_membership::Config<pallet_membership::Instance2> for Runtime {
	type AddOrigin = MoreThanHalfCouncil;
	type Event = Event;
	type MaxMembers = TechnicalMaxMembers;
	type MembershipChanged = TechnicalCommittee;
	type MembershipInitialized = TechnicalCommittee;
	type PrimeOrigin = MoreThanHalfCouncil;
	type RemoveOrigin = MoreThanHalfCouncil;
	type ResetOrigin = MoreThanHalfCouncil;
	type SwapOrigin = MoreThanHalfCouncil;
	type WeightInfo = ();
}

parameter_types! {
	pub const CandidacyBond: Balance = 100 * DOLLARS;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub const VotingBondBase: Balance = deposit(1, 64);
	// additional data per vote is 32 bytes (account id).
	pub const VotingBondFactor: Balance = deposit(0, 32);
	/// Daily council elections
	pub const TermDuration: BlockNumber = 7 * DAYS;
	pub const DesiredMembers: u32 = 13;
	pub const DesiredRunnersUp: u32 = 20;
	pub const PhragmenElectionPalletId: LockIdentifier = *b"phrelect";
	pub const MaxVoters: u32 = 10 * 1000;
	pub const MaxCandidates: u32 = 1000;
}

// Make sure that there are no more than MaxMembers members elected via phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
	type CandidacyBond = CandidacyBond;
	type ChangeMembers = Council;
	type Currency = Balances;
	type CurrencyToVote = frame_support::traits::U128CurrencyToVote;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type Event = Event;
	type InitializeMembers = Council;
	type KickedMember = Treasury;
	type LoserCandidate = Treasury;
	type PalletId = PhragmenElectionPalletId;
	type TermDuration = TermDuration;
	type VotingBondBase = VotingBondBase;
	type VotingBondFactor = VotingBondFactor;
	type MaxCandidates = MaxCandidates;
	type MaxVoters = MaxVoters;
	type WeightInfo = ();
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 28 * DAYS;
	pub const VotingPeriod: BlockNumber = 28 * DAYS;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
	pub const MinimumDeposit: Balance = 100 * DOLLARS;
	pub const EnactmentPeriod: BlockNumber = 28 * DAYS;
	pub const CooloffPeriod: BlockNumber = 7 * DAYS;
	pub const InstantAllowed: bool = true;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>,
	>;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 3>;
	type CooloffPeriod = CooloffPeriod;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type Event = Event;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 1>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 1, 2>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 2, 3>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	type InstantAllowed = InstantAllowed;
	type InstantOrigin =
		pallet_collective::EnsureProportionAtLeast<AccountId, TechnicalCollective, 1, 1>;
	type LaunchPeriod = LaunchPeriod;
	type MaxProposals = MaxProposals;
	type MaxVotes = MaxVotes;
	type MinimumDeposit = MinimumDeposit;
	type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
	type PalletsOrigin = OriginCaller;
	type PreimageByteDeposit = PreimageByteDeposit;
	type Proposal = Call;
	type Scheduler = Scheduler;
	type Slash = Treasury;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
	type VotingPeriod = VotingPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 100 * DOLLARS;
	pub const ProposalBondMaximum: Balance = 500 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 24 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(1);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * DOLLARS;
	pub const DataDepositPerByte: Balance = 1 * CENTS;
	pub const MaxApprovals: u32 = 100;
}

type ApproveOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
>;

impl pallet_treasury::Config for Runtime {
	type ApproveOrigin = ApproveOrigin;
	type SpendOrigin = NeverEnsureOrigin<Balance>;
	type Burn = Burn;
	type BurnDestination = ();
	type Currency = Balances;
	type Event = Event;
	type MaxApprovals = MaxApprovals;
	type OnSlash = Treasury;
	type PalletId = TreasuryPalletId;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ProposalBondMaximum;
	type RejectOrigin = MoreThanHalfCouncil;
	type SpendFunds = Bounties;
	type SpendPeriod = SpendPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const BountyDepositBase: Balance = 1 * DOLLARS;
	pub const BountyDepositPayoutDelay: BlockNumber = 8 * DAYS;
	pub const BountyUpdatePeriod: BlockNumber = 90 * DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMin: Balance = 10 * DOLLARS;
	pub const CuratorDepositMax: Balance = 200 * DOLLARS;
	pub const BountyValueMinimum: Balance = 10 * DOLLARS;
}

impl pallet_bounties::Config for Runtime {
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type BountyValueMinimum = BountyValueMinimum;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type DataDepositPerByte = DataDepositPerByte;
	type Event = Event;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = ();
	type ChildBountyManager = ();
}

impl pallet_tips::Config for Runtime {
	type DataDepositPerByte = DataDepositPerByte;
	type Event = Event;
	type MaximumReasonLength = MaximumReasonLength;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type Tippers = PhragmenElection;
	type WeightInfo = ();
}

impl pallet_transaction_payment::Config for Runtime {
	type Event = Event;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type OnChargeTransaction = FlexibleFee;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
}

// culumus runtime start
parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type DmpMessageHandler = DmpQueue;
	type Event = Event;
	type OnSystemEvent = ();
	type OutboundXcmpMessageSource = XcmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type XcmpMessageHandler = XcmpQueue;
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const DotLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub SelfParaChainId: CumulusParaId = ParachainInfo::parachain_id();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = XcmCurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<DotLocation>,
	// Do a simple punn to convert an AccountId32 MultiLocation into a native chain account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports.
	(),
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, Origin>,
	// Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, Origin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<Origin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `Origin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

parameter_types! {
	// One XCM operation is 200_000_000 weight, cross-chain transfer ~= 2x of transfer = 3_000_000_000
	pub UnitWeightCost: Weight = 200_000_000;
	pub const MaxInstructions: u32 = 100;
}

match_types! {
	pub type ParentOrParentsExecutivePlurality: impl Contains<MultiLocation> = {
		MultiLocation { parents: 1, interior: Here } |
		MultiLocation { parents: 1, interior: X1(Plurality { id: BodyId::Executive, .. }) }
	};
}

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<Everything>,
	AllowKnownQueryResponses<PolkadotXcm>,
	AllowSubscriptionsFrom<Everything>,
);

pub type BifrostAssetTransactor = MultiCurrencyAdapter<
	Currencies,
	UnknownTokens,
	BifrostAssetMatcher<CurrencyId, BifrostCurrencyIdConvert<SelfParaChainId>>,
	AccountId,
	LocationToAccountId,
	CurrencyId,
	BifrostCurrencyIdConvert<SelfParaChainId>,
	DepositToAlternative<BifrostTreasuryAccount, Currencies, CurrencyId, AccountId, Balance>,
>;

parameter_types! {
	pub DotPerSecond: (AssetId, u128) = (MultiLocation::parent().into(), dot_per_second());
	pub BncPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), GeneralKey((NativeCurrencyId::get().encode()).try_into().unwrap()))
		).into(),
		// BNC:DOT = 80:1
		dot_per_second() * 80
	);
	pub BncNewPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			0,
			X1(GeneralKey((NativeCurrencyId::get().encode()).try_into().unwrap()))
		).into(),
		// BNC:DOT = 80:1
		dot_per_second() * 80
	);
	pub ZlkPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), GeneralKey((CurrencyId::Token(TokenSymbol::ZLK).encode()).try_into().unwrap()))
		).into(),
		// ZLK:KSM = 150:1
		dot_per_second() * 150 * 1_000_000
	);
	pub ZlkNewPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			0,
			X1(GeneralKey((CurrencyId::Token(TokenSymbol::ZLK).encode()).try_into().unwrap()))
		).into(),
		// ZLK:KSM = 150:1
		dot_per_second() * 150 * 1_000_000
	);
}

pub struct ToTreasury;
impl TakeRevenue for ToTreasury {
	fn take_revenue(revenue: MultiAsset) {
		if let MultiAsset { id: Concrete(location), fun: Fungible(amount) } = revenue {
			if let Some(currency_id) =
				BifrostCurrencyIdConvert::<SelfParaChainId>::convert(location)
			{
				let _ = Currencies::deposit(currency_id, &BifrostTreasuryAccount::get(), amount);
			}
		}
	}
}

pub type Trader = (
	FixedRateOfFungible<BncPerSecond, ToTreasury>,
	FixedRateOfFungible<BncNewPerSecond, ToTreasury>,
	FixedRateOfFungible<DotPerSecond, ToTreasury>,
);

pub struct XcmConfig;
impl Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	type AssetTransactor = BifrostAssetTransactor;
	type AssetTrap = PolkadotXcm;
	type Barrier = Barrier;
	type Call = Call;
	type IsReserve = MultiNativeAsset<AbsoluteReserveProvider>;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type ResponseHandler = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type Trader = Trader;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type XcmSender = XcmRouter;
}

/// Local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<Origin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type LocationInverter = LocationInverter<Ancestry>;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type XcmExecuteFilter = Nothing;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmReserveTransferFilter = Everything;
	type XcmRouter = XcmRouter;
	type XcmTeleportFilter = Nothing;
	type Origin = Origin;
	type Call = Call;
	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type ChannelInfo = ParachainSystem;
	type Event = Event;
	type VersionWrapper = PolkadotXcm;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = EnsureRoot<AccountId>;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	type WeightInfo = ();
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type Event = Event;
	type Keys = SessionKeys;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	// Essentially just Aura, but lets be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type SessionManager = CollatorSelection;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type WeightInfo = ();
}

parameter_types! {
	pub const UncleGenerations: u32 = 0;
}

impl pallet_authorship::Config for Runtime {
	type EventHandler = CollatorSelection;
	type FilterUncle = ();
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type UncleGenerations = UncleGenerations;
}

parameter_types! {
	pub const MaxAuthorities: u32 = 100_000;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const MinCandidates: u32 = 5;
	pub const SessionLength: BlockNumber = 6 * HOURS;
	pub const MaxInvulnerables: u32 = 100;
}

impl pallet_collator_selection::Config for Runtime {
	type Currency = Balances;
	type Event = Event;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type MaxCandidates = MaxCandidates;
	type MaxInvulnerables = MaxInvulnerables;
	type MinCandidates = MinCandidates;
	type PotId = PotId;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = ();
}

// culumus runtime end

impl pallet_vesting::Config for Runtime {
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type Event = Event;
	type MinVestedTransfer = ExistentialDeposit;
	type WeightInfo = ();
	const MAX_VESTING_SCHEDULES: u32 = 42;
}

// orml runtime start

impl orml_currencies::Config for Runtime {
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type WeightInfo = ();
}

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			&CurrencyId::Native(TokenSymbol::BNC) => 10 * milli::<Runtime>(NativeCurrencyId::get()),   // 0.01 BNC
			&CurrencyId::Token2(DOT_TOKEN_ID) => 1_000_000,  // DOT
			&CurrencyId::LPToken(..) => 10 * millicent::<Runtime>(NativeCurrencyId::get()),
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
		AccountIdConversion::<AccountId>::into_account_truncating(&TreasuryPalletId::get()).eq(a) ||
			AccountIdConversion::<AccountId>::into_account_truncating(&BifrostCrowdloanId::get())
				.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&BifrostVsbondPalletId::get(),
		)
		.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&SlpEntrancePalletId::get(),
		)
		.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(&SlpExitPalletId::get())
			.eq(a) || FarmingKeeperPalletId::get().check_sub_account::<PoolId>(a) ||
			FarmingRewardIssuerPalletId::get().check_sub_account::<PoolId>(a) ||
			AccountIdConversion::<AccountId>::into_account_truncating(&BuybackPalletId::get())
				.eq(a) || AccountIdConversion::<AccountId>::into_account_truncating(
			&SystemMakerPalletId::get(),
		)
		.eq(a) || FeeSharePalletId::get().check_sub_account::<DistributionId>(a)
	}
}

parameter_types! {
	pub BifrostTreasuryAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
}

impl orml_tokens::Config for Runtime {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = DustRemovalWhitelist;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type OnDust = orml_tokens::TransferDust<Runtime, BifrostTreasuryAccount>;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}

parameter_types! {
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::get().into())));
	pub RelayXcmBaseWeight: u64 = (100 * milli::<Runtime>(RelayCurrencyId::get())) as u64;
	pub const MaxAssetsForTransfer: usize = 2;
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(u128::MAX)
	};
}

impl orml_xtokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = BifrostCurrencyIdConvert<ParachainInfo>;
	type AccountIdToMultiLocation = BifrostAccountIdToMultiLocation;
	type LocationInverter = LocationInverter<Ancestry>;
	type SelfLocation = SelfLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type BaseXcmWeight = RelayXcmBaseWeight;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type MultiLocationsFilter = Everything;
	type ReserveProvider = AbsoluteReserveProvider;
}

impl orml_unknown_tokens::Config for Runtime {
	type Event = Event;
}

impl orml_xcm::Config for Runtime {
	type Event = Event;
	type SovereignOrigin = MoreThanHalfCouncil;
}

// orml runtime end

// Bifrost modules start

// Aggregate name getter to get fee names if the call needs to pay extra fees.
// If any call need to pay extra fees, it should be added as an item here.
// Used together with AggregateExtraFeeFilter below.
pub struct FeeNameGetter;
impl NameGetter<Call> for FeeNameGetter {
	fn get_name(c: &Call) -> ExtraFeeName {
		match *c {
			Call::Salp(bifrost_salp::Call::contribute { .. }) => ExtraFeeName::SalpContribute,
			Call::XcmInterface(xcm_interface::Call::transfer_statemine_assets { .. }) =>
				ExtraFeeName::StatemineTransfer,
			_ => ExtraFeeName::NoExtraFee,
		}
	}
}

// Aggregate filter to filter if the call needs to pay extra fees
// If any call need to pay extra fees, it should be added as an item here.
pub struct AggregateExtraFeeFilter;
impl Contains<Call> for AggregateExtraFeeFilter {
	fn contains(c: &Call) -> bool {
		match *c {
			Call::Salp(bifrost_salp::Call::contribute { .. }) => true,
			Call::XcmInterface(xcm_interface::Call::transfer_statemine_assets { .. }) => true,
			_ => false,
		}
	}
}

pub struct ContributeFeeFilter;
impl Contains<Call> for ContributeFeeFilter {
	fn contains(c: &Call) -> bool {
		match *c {
			Call::Salp(bifrost_salp::Call::contribute { .. }) => true,
			_ => false,
		}
	}
}

pub struct StatemineTransferFeeFilter;
impl Contains<Call> for StatemineTransferFeeFilter {
	fn contains(c: &Call) -> bool {
		match *c {
			Call::XcmInterface(xcm_interface::Call::transfer_statemine_assets { .. }) => true,
			_ => false,
		}
	}
}

parameter_types! {
	pub const AltFeeCurrencyExchangeRate: (u32, u32) = (1, 100);
	pub UmpContributeFee: Balance = UmpTransactFee::get();
	pub const MaximumAssetsInOrder: u8 = 20;
}

pub type MiscFeeHandlers = (
	MiscFeeHandler<Runtime, RelayCurrencyId, UmpContributeFee, ContributeFeeFilter>,
	MiscFeeHandler<Runtime, RelayCurrencyId, StatemineTransferFee, StatemineTransferFeeFilter>,
);

impl bifrost_flexible_fee::Config for Runtime {
	type Currency = Balances;
	type DexOperator = ZenlinkProtocol;
	type FeeDealer = FlexibleFee;
	type Event = Event;
	type MultiCurrency = Currencies;
	type TreasuryAccount = BifrostTreasuryAccount;
	type NativeCurrencyId = NativeCurrencyId;
	type AlternativeFeeCurrencyId = RelayCurrencyId;
	type AltFeeCurrencyExchangeRate = AltFeeCurrencyExchangeRate;
	type OnUnbalanced = Treasury;
	type WeightInfo = ();
	type ExtraFeeMatcher = ExtraFeeMatcher<Runtime, FeeNameGetter, AggregateExtraFeeFilter>;
	type MiscFeeHandler = MiscFeeHandlers;
	type ParachainId = ParachainInfo;
}

parameter_types! {
	pub BifrostParachainAccountId20: [u8; 20] = cumulus_primitives_core::ParaId::from(ParachainInfo::get()).into_account_truncating();
}

pub fn create_x2_multilocation(index: u16, currency_id: CurrencyId) -> MultiLocation {
	match currency_id {
		CurrencyId::Token2(GLMR_TOKEN_ID) => MultiLocation::new(
			1,
			X2(
				Parachain(parachains::moonbeam::ID.into()),
				AccountKey20 {
					network: NetworkId::Any,
					key: Slp::derivative_account_id_20(
						cumulus_primitives_core::ParaId::from(ParachainInfo::get())
							.into_account_truncating(),
						index,
					)
					.into(),
				},
			),
		),
		_ => MultiLocation::new(
			1,
			X1(AccountId32 {
				network: NetworkId::Any,
				id: Utility::derivative_account_id(
					ParachainInfo::get().into_account_truncating(),
					index,
				)
				.into(),
			}),
		),
	}
}

pub struct SubAccountIndexMultiLocationConvertor;
impl Convert<(u16, CurrencyId), MultiLocation> for SubAccountIndexMultiLocationConvertor {
	fn convert((sub_account_index, currency_id): (u16, CurrencyId)) -> MultiLocation {
		create_x2_multilocation(sub_account_index, currency_id)
	}
}

parameter_types! {
	pub MinContribution: Balance = dollar::<Runtime>(RelayCurrencyId::get()) * 5;
	pub const RemoveKeysLimit: u32 = 500;
	pub const VSBondValidPeriod: BlockNumber = 30 * DAYS;
	pub const ReleaseCycle: BlockNumber = 1 * DAYS;
	pub const LeasePeriod: BlockNumber = POLKA_LEASE_PERIOD;
	pub const ReleaseRatio: Percent = Percent::from_percent(50);
	pub const SlotLength: BlockNumber = 8u32 as BlockNumber;
	pub ConfirmMuitiSigAccount: AccountId = hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();
}

impl bifrost_salp::Config for Runtime {
	type BancorPool = ();
	type Event = Event;
	type LeasePeriod = LeasePeriod;
	type MinContribution = MinContribution;
	type MultiCurrency = Currencies;
	type PalletId = BifrostCrowdloanId;
	type RelayChainToken = RelayCurrencyId;
	type ReleaseCycle = ReleaseCycle;
	type ReleaseRatio = ReleaseRatio;
	type RemoveKeysLimit = RemoveKeysLimit;
	type SlotLength = SlotLength;
	type VSBondValidPeriod = VSBondValidPeriod;
	type WeightInfo = bifrost_salp::weights::BifrostWeight<Runtime>;
	type EnsureConfirmAsGovernance =
		EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type XcmInterface = XcmInterface;
	type TreasuryAccount = BifrostTreasuryAccount;
	type BuybackPalletId = BuybackPalletId;
	type DexOperator = ZenlinkProtocol;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type CurrencyIdRegister = AssetIdMaps<Runtime>;
	type ParachainId = ParachainInfo;
}

impl bifrost_call_switchgear::Config for Runtime {
	type Event = Event;
	type UpdateOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type WeightInfo = bifrost_call_switchgear::weights::BifrostWeight<Runtime>;
}

impl bifrost_asset_registry::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type RegisterOrigin = MoreThanHalfCouncil;
	type WeightInfo = bifrost_asset_registry::weights::BifrostWeight<Runtime>;
}

parameter_types! {
	pub ParachainAccount: AccountId = ParachainInfo::get().into_account_truncating();
	pub ContributionWeight:XcmBaseWeight = RelayXcmBaseWeight::get().into();
	pub UmpTransactFee: Balance = milli::<Runtime>(RelayCurrencyId::get()) * 100;
	pub StatemineTransferFee: Balance = milli::<Runtime>(RelayCurrencyId::get()) * 400;
	pub StatemineTransferWeight:XcmBaseWeight = (RelayXcmBaseWeight::get() * 400).into();
}

impl xcm_interface::Config for Runtime {
	type Event = Event;
	type UpdateOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type MultiCurrency = Currencies;
	type RelayNetwork = RelayNetwork;
	type RelaychainCurrencyId = RelayCurrencyId;
	type ParachainSovereignAccount = ParachainAccount;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type AccountIdToMultiLocation = BifrostAccountIdToMultiLocation;
	type StatemineTransferWeight = StatemineTransferWeight;
	type StatemineTransferFee = StatemineTransferFee;
	type ContributionWeight = ContributionWeight;
	type ContributionFee = UmpTransactFee;
}

parameter_types! {
	pub const MaxTypeEntryPerBlock: u32 = 10;
	pub const MaxRefundPerBlock: u32 = 10;
}

pub struct SubstrateResponseManager;
impl QueryResponseManager<QueryId, MultiLocation, BlockNumber> for SubstrateResponseManager {
	fn get_query_response_record(query_id: QueryId) -> bool {
		if let Some(QueryStatus::Ready { .. }) = PolkadotXcm::query(query_id) {
			true
		} else {
			false
		}
	}

	fn create_query_record(responder: &MultiLocation, timeout: BlockNumber) -> u64 {
		PolkadotXcm::new_query(responder.clone(), timeout)
		// for xcm v3 version see the following
		// PolkadotXcm::new_query(responder, timeout, Here)
	}

	fn remove_query_record(query_id: QueryId) -> bool {
		// Temporarily banned. Querries from pallet_xcm cannot be removed unless it is in ready
		// status. And we are not allowed to mannually change query status.
		// So in the manual mode, it is not possible to remove the query at all.
		// PolkadotXcm::take_response(query_id).is_some()

		PolkadotXcm::take_response(query_id);
		true
	}
}

impl bifrost_slp::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type ControlOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type WeightInfo = bifrost_slp::weights::BifrostWeight<Runtime>;
	type VtokenMinting = VtokenMinting;
	type AccountConverter = SubAccountIndexMultiLocationConvertor;
	type ParachainId = SelfParaChainId;
	type XcmRouter = XcmRouter;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type SubstrateResponseManager = SubstrateResponseManager;
	type MaxTypeEntryPerBlock = MaxTypeEntryPerBlock;
	type MaxRefundPerBlock = MaxRefundPerBlock;
	type OnRefund = ();
	type ParachainStaking = ();
}

parameter_types! {
	pub const RelayChainTokenSymbolDOT: TokenSymbol = TokenSymbol::DOT;
}

impl bifrost_vstoken_conversion::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type RelayCurrencyId = RelayCurrencyId;
	type TreasuryAccount = BifrostTreasuryAccount;
	type ControlOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type VsbondAccount = BifrostVsbondPalletId;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type WeightInfo = ();
}

impl bifrost_farming::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type ControlOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type TreasuryAccount = BifrostTreasuryAccount;
	type Keeper = FarmingKeeperPalletId;
	type RewardIssuer = FarmingRewardIssuerPalletId;
	type WeightInfo = bifrost_farming::weights::BifrostWeight<Runtime>;
}

impl bifrost_system_maker::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type ControlOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type WeightInfo = ();
	type DexOperator = ZenlinkProtocol;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type TreasuryAccount = BifrostTreasuryAccount;
	type RelayChainToken = RelayCurrencyId;
	type SystemMakerPalletId = SystemMakerPalletId;
	type ParachainId = ParachainInfo;
	type VtokenMintingInterface = VtokenMinting;
}

impl bifrost_fee_share::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type ControlOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type WeightInfo = ();
	type FeeSharePalletId = FeeSharePalletId;
}

// Bifrost modules end

// zenlink runtime start

parameter_types! {
	pub const StringLimit: u32 = 50;
}

impl merkle_distributor::Config for Runtime {
	type Event = Event;
	type CurrencyId = CurrencyId;
	type MultiCurrency = Currencies;
	type Balance = Balance;
	type MerkleDistributorId = u32;
	type PalletId = MerkleDirtributorPalletId;
	type StringLimit = StringLimit;
	type WeightInfo = ();
}

parameter_types! {
	pub const ZenlinkPalletId: PalletId = PalletId(*b"/zenlink");
	pub const GetExchangeFee: (u32, u32) = (3, 1000);   // 0.3%

	// xcm
	pub const AnyNetwork: NetworkId = NetworkId::Any;
	pub ZenlinkRegistedParaChains: Vec<(MultiLocation, u128)> = vec![
		// Bifrost local and live, 0.01 BNC
		(make_x2_location(2001), 10_000_000_000),
		// Phala local and live, 1 PHA
		(make_x2_location(2004), 1_000_000_000_000),
		// Plasm local and live, 0.0000000000001 SDN
		(make_x2_location(2007), 1_000_000),
		// Sherpax live, 0 KSX
		(make_x2_location(2013), 0),

		// Zenlink local 1 for test
		(make_x2_location(200), 1_000_000),
		// Zenlink local 2 for test
		(make_x2_location(300), 1_000_000),
	];
}

impl zenlink_protocol::Config for Runtime {
	type Conversion = ZenlinkLocationToAccountId;
	type Event = Event;
	type MultiAssetsHandler = MultiAssets;
	type PalletId = ZenlinkPalletId;
	type SelfParaId = SelfParaId;
	type TargetChains = ZenlinkRegistedParaChains;
	type XcmExecutor = ();
	type WeightInfo = ();
}

type MultiAssets = ZenlinkMultiAssets<ZenlinkProtocol, Balances, LocalAssetAdaptor<Currencies>>;

pub type ZenlinkLocationToAccountId = (
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<AnyNetwork, AccountId>,
);

parameter_types! {
	pub const MaximumUnlockIdOfUser: u32 = 10;
	pub const MaximumUnlockIdOfTimeUnit: u32 = 50;
	pub BifrostFeeAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
}

impl bifrost_vtoken_minting::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type ControlOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type MaximumUnlockIdOfUser = MaximumUnlockIdOfUser;
	type MaximumUnlockIdOfTimeUnit = MaximumUnlockIdOfTimeUnit;
	type EntranceAccount = SlpEntrancePalletId;
	type ExitAccount = SlpExitPalletId;
	type FeeAccount = BifrostFeeAccount;
	type BifrostSlp = Slp;
	type WeightInfo = ();
	type OnRedeemSuccess = ();
	type RelayChainToken = RelayCurrencyId;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type CurrencyIdRegister = AssetIdMaps<Runtime>;
}

// Below is the implementation of tokens manipulation functions other than native token.
pub struct LocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
{
	fn local_balance_of(asset_id: ZenlinkAssetId, who: &AccountId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::free_balance(currency_id, &who))
				.unwrap_or_default();
		}
		AssetBalance::default()
	}

	fn local_total_supply(asset_id: ZenlinkAssetId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			return TryInto::<AssetBalance>::try_into(Local::total_issuance(currency_id))
				.unwrap_or_default();
		}
		AssetBalance::default()
	}

	fn local_is_exists(asset_id: ZenlinkAssetId) -> bool {
		let currency_id: Result<CurrencyId, ()> = asset_id.try_into();
		match currency_id {
			Ok(_) => true,
			Err(_) => false,
		}
	}

	fn local_transfer(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		target: &AccountId,
		amount: AssetBalance,
	) -> DispatchResult {
		if let Ok(currency_id) = asset_id.try_into() {
			Local::transfer(
				currency_id,
				&origin,
				&target,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local transfer"))?,
			)
		} else {
			Err(DispatchError::Other("unknown asset in local transfer"))
		}
	}

	fn local_deposit(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(currency_id) = asset_id.try_into() {
			Local::deposit(
				currency_id,
				&origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local deposit"))?,
			)?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"));
		}

		Ok(amount)
	}

	fn local_withdraw(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(currency_id) = asset_id.try_into() {
			Local::withdraw(
				currency_id,
				&origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local withdraw"))?,
			)?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"));
		}

		Ok(amount)
	}
}

// zenlink runtime end

construct_runtime! {
	pub enum Runtime where
		Block = Block,
		NodeBlock = generic::Block<Header, sp_runtime::OpaqueExtrinsic>,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		// Basic stuff
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 1,
		Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>} = 2,
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>, ValidateUnsigned} = 5,
		ParachainInfo: parachain_info::{Pallet, Storage, Config} = 6,

		// Monetary stuff
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 10,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>} = 11,

		// Collator support. the order of these 4 are important and shall not change.
		Authorship: pallet_authorship::{Pallet, Call, Storage} = 20,
		CollatorSelection: pallet_collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>} = 21,
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 22,
		Aura: pallet_aura::{Pallet, Storage, Config<T>} = 23,
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config} = 24,

		// Governance stuff
		Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 30,
		Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 31,
		TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 32,
		PhragmenElection: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>} = 33,
		CouncilMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>} = 34,
		TechnicalMembership: pallet_membership::<Instance2>::{Pallet, Call, Storage, Event<T>, Config<T>} = 35,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 40,
		PolkadotXcm: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin, Config} = 41,
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 42,
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 43,

		// utilities
		Utility: pallet_utility::{Pallet, Call, Event} = 50,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 51,
		Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>} = 52,
		Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 53,
		Identity: pallet_identity::{Pallet, Call, Storage, Event<T>} = 54,

		// Vesting. Usable initially, but removed once all vesting is finished.
		Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 60,

		// Treasury stuff
		Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 61,
		Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 62,
		Tips: pallet_tips::{Pallet, Call, Storage, Event<T>} = 63,
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>} = 64,

		// Third party modules
		XTokens: orml_xtokens::{Pallet, Call, Event<T>} = 70,
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>, Config<T>} = 71,
		Currencies: orml_currencies::{Pallet, Call} = 72,
		UnknownTokens: orml_unknown_tokens::{Pallet, Storage, Event} = 73,
		OrmlXcm: orml_xcm::{Pallet, Call, Event<T>} = 74,
		ZenlinkProtocol: zenlink_protocol::{Pallet, Call, Storage, Event<T>} = 80,
		MerkleDistributor: merkle_distributor::{Pallet, Call, Storage, Event<T>} = 81,

		// Bifrost modules
		FlexibleFee: bifrost_flexible_fee::{Pallet, Call, Storage, Event<T>} = 100,
		Salp: bifrost_salp::{Pallet, Call, Storage, Event<T>, Config<T>} = 105,
		CallSwitchgear: bifrost_call_switchgear::{Pallet, Storage, Call, Event<T>} = 112,
		AssetRegistry: bifrost_asset_registry::{Pallet, Call, Storage, Event<T>, Config<T>} = 114,
		VtokenMinting: bifrost_vtoken_minting::{Pallet, Call, Storage, Event<T>} = 115,
		Slp: bifrost_slp::{Pallet, Call, Storage, Event<T>} = 116,
		XcmInterface: xcm_interface::{Pallet, Call, Storage, Event<T>} = 117,
		TokenConversion: bifrost_vstoken_conversion::{Pallet, Call, Storage, Event<T>} = 118,
		Farming: bifrost_farming::{Pallet, Call, Storage, Event<T>} = 119,
		SystemMaker: bifrost_system_maker::{Pallet, Call, Storage, Event<T>} = 121,
		FeeShare: bifrost_fee_share::{Pallet, Call, Storage, Event<T>} = 122,
	}
}

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = sp_runtime::MultiSignature;
/// Index of a transaction in the chain.
pub type Index = u32;
/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;
/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, AccountIndex>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
	frame_system::CheckNonZeroSender<Runtime>,
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	(),
>;

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[pallet_vesting, Vesting]
		[bifrost_call_switchgear, CallSwitchgear]
		[bifrost_vtoken_minting, VtokenMinting]
		[bifrost_slp, Slp]
		[bifrost_salp, Salp]
	);
}

impl_runtime_apis! {
	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(
			extrinsic: <Block as BlockT>::Extrinsic,
		) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}

		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
		}

		fn authorities() -> Vec<AuraId> {
			Aura::authorities().into_inner()
		}
	}

	impl bifrost_flexible_fee_rpc_runtime_api::FlexibleFeeRuntimeApi<Block, AccountId> for Runtime {
		fn get_fee_token_and_amount(who: AccountId, fee: Balance) -> (CurrencyId, Balance) {
			let rs = FlexibleFee::cal_fee_token_and_amount(&who, fee);
			match rs {
				Ok(val) => val,
				_ => (CurrencyId::Native(TokenSymbol::BNC), Zero::zero()),
			}
		}
	}

	// zenlink runtime outer apis
	impl zenlink_protocol_runtime_api::ZenlinkProtocolApi<Block, AccountId> for Runtime {

		fn get_balance(
			asset_id: ZenlinkAssetId,
			owner: AccountId
		) -> AssetBalance {
			<Runtime as zenlink_protocol::Config>::MultiAssetsHandler::balance_of(asset_id, &owner)
		}

		fn get_sovereigns_info(
			asset_id: ZenlinkAssetId
		) -> Vec<(u32, AccountId, AssetBalance)> {
			ZenlinkProtocol::get_sovereigns_info(&asset_id)
		}

		fn get_pair_by_asset_id(
			asset_0: ZenlinkAssetId,
			asset_1: ZenlinkAssetId
		) -> Option<PairInfo<AccountId, AssetBalance>> {
			ZenlinkProtocol::get_pair_by_asset_id(asset_0, asset_1)
		}

		fn get_amount_in_price(
			supply: AssetBalance,
			path: Vec<ZenlinkAssetId>
		) -> AssetBalance {
			ZenlinkProtocol::desired_in_amount(supply, path)
		}

		fn get_amount_out_price(
			supply: AssetBalance,
			path: Vec<ZenlinkAssetId>
		) -> AssetBalance {
			ZenlinkProtocol::supply_out_amount(supply, path)
		}

		fn get_estimate_lptoken(
			token_0: ZenlinkAssetId,
			token_1: ZenlinkAssetId,
			amount_0_desired: AssetBalance,
			amount_1_desired: AssetBalance,
			amount_0_min: AssetBalance,
			amount_1_min: AssetBalance,
		) -> AssetBalance{
			ZenlinkProtocol::get_estimate_lptoken(
				token_0,
				token_1,
				amount_0_desired,
				amount_1_desired,
				amount_0_min,
				amount_1_min
			)
		}
	}

	impl bifrost_salp_rpc_runtime_api::SalpRuntimeApi<Block, ParaId, AccountId> for Runtime {
		fn get_contribution(index: ParaId, who: AccountId) -> (Balance,RpcContributionStatus) {
			let rs = Salp::contribution_by_fund(index, &who);
			match rs {
				Ok((val,status)) => (val,status.to_rpc()),
				_ => (Zero::zero(),RpcContributionStatus::Idle),
			}
		}

		fn get_lite_contribution(_index: ParaId, _who: AccountId) -> (Balance,RpcContributionStatus) {
				(Zero::zero(),RpcContributionStatus::Idle)
		}
	}

	impl bifrost_farming_rpc_runtime_api::FarmingRuntimeApi<Block, AccountId, PoolId> for Runtime {
		fn get_farming_rewards(who: AccountId, pid: PoolId) -> Vec<(CurrencyId, Balance)> {
			Farming::get_farming_rewards(&who, pid).unwrap_or(Vec::new())
		}

		fn get_gauge_rewards(who: AccountId, pid: PoolId) -> Vec<(CurrencyId, Balance)> {
			Farming::get_gauge_rewards(&who, pid).unwrap_or(Vec::new())
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_benchmarking::{Benchmarking, BenchmarkList};
			use frame_support::traits::StorageInfoTrait;

			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			return (list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, TrackedStorageKey};

			impl frame_system_benchmarking::Config for Runtime {}

			let whitelist: Vec<TrackedStorageKey> = vec![
			// you can whitelist any storage keys you do not want to track here
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade() -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade bifrost-polkadot.");
			let weight = Executive::try_runtime_upgrade().unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}
		fn execute_block_no_check(block: Block) -> Weight {
			Executive::execute_block_no_check(block)
		}
	}
}

struct CheckInherents;

impl cumulus_pallet_parachain_system::CheckInherents<Block> for CheckInherents {
	fn check_inherents(
		block: &Block,
		relay_state_proof: &cumulus_pallet_parachain_system::RelayChainStateProof,
	) -> sp_inherents::CheckInherentsResult {
		let relay_chain_slot = relay_state_proof
			.read_slot()
			.expect("Could not read the relay chain slot from the proof");

		let inherent_data =
			cumulus_primitives_timestamp::InherentDataProvider::from_relay_chain_slot_and_duration(
				relay_chain_slot,
				sp_std::time::Duration::from_secs(6),
			)
			.create_inherent_data()
			.expect("Could not create the timestamp inherent data");

		inherent_data.check_extrinsics(&block)
	}
}

cumulus_pallet_parachain_system::register_validate_block! {
	Runtime = Runtime,
	BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
	CheckInherents = CheckInherents,
}

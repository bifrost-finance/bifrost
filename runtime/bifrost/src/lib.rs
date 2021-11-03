// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, match_type, parameter_types,
	traits::{Contains, Everything, InstanceFilter, IsInVec, Nothing, Randomness},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		DispatchClass, IdentityFee, Weight,
	},
	PalletId, RuntimeDebug, StorageValue,
};
use frame_system::limits::{BlockLength, BlockWeights};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use sp_api::impl_runtime_apis;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
	u32_trait::{_1, _2, _3, _4, _5},
	OpaqueMetadata,
};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdConversion, BlakeTwo256, Block as BlockT, UniqueSaturatedInto, Zero},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, DispatchError, DispatchResult, SaturatedConversion,
};
pub use sp_runtime::{Perbill, Permill};
use sp_std::{marker::PhantomData, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;

/// Constant values used within the runtime.
pub mod constants;
use bifrost_flexible_fee::{
	fee_dealer::{FeeDealer, FixedCurrencyFeeRate},
	misc_fees::{ExtraFeeMatcher, MiscFeeHandler, NameGetter},
};
use bifrost_runtime_common::{
	constants::parachains,
	xcm_impl::{
		BifrostAccountIdToMultiLocation, BifrostAssetMatcher, BifrostCurrencyIdConvert,
		BifrostFilteredAssets,
	},
	CouncilCollective, EnsureRootOrAllTechnicalCommittee, MoreThanHalfCouncil,
	SlowAdjustingFeeUpdate, TechnicalCollective,
};
use codec::{Decode, Encode, MaxEncodedLen};
use constants::{currency::*, time::*};
use cumulus_primitives_core::ParaId as CumulusParaId;
use frame_support::{
	sp_runtime::traits::Convert,
	traits::{EnsureOrigin, LockIdentifier, OnRuntimeUpgrade},
};
use frame_system::{EnsureOneOf, EnsureRoot, RawOrigin};
use hex_literal::hex;
pub use node_primitives::{
	traits::CheckSubAccount, AccountId, Amount, Balance, BlockNumber, CurrencyId, ExtraFeeName,
	Moment, Nonce, ParaId, ParachainDerivedProxyAccountType, ParachainTransactProxyType,
	ParachainTransactType, PoolId, RpcContributionStatus, TokenSymbol, TransferOriginType,
	XcmBaseWeight,
};
// orml imports
use orml_currencies::BasicCurrencyAdapter;
use orml_traits::MultiCurrency;
use orml_xcm_support::MultiCurrencyAdapter;
use pallet_xcm::XcmPassthrough;
// XCM imports
use polkadot_parachain::primitives::Sibling;
use sp_arithmetic::Percent;
use sp_runtime::traits::ConvertInto;
use xcm::latest::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, CurrencyAdapter, EnsureXcmOrigin, FixedRateOfFungible,
	FixedWeightBounds, IsConcrete, LocationInverter, ParentAsSuperuser, ParentIsDefault,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeRevenue,
	TakeWeightCredit,
};
use xcm_executor::{Config, XcmExecutor};
use xcm_support::{BifrostXcmAdaptor, Get};
// zenlink imports
use zenlink_protocol::{
	make_x2_location, AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler,
	MultiAssetsHandler, PairInfo, ZenlinkMultiAssets,
};
// Weights used in the runtime.
mod weights;

pub type SessionHandlers = ();

impl_opaque_keys! {
	pub struct SessionKeys {
		pub aura: Aura,
	}
}

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("bifrost"),
	impl_name: create_runtime_str!("bifrost"),
	authoring_version: 1,
	spec_version: 908,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
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
	pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const StableCurrencyId: CurrencyId = CurrencyId::Stable(TokenSymbol::KUSD);
	pub SelfParaId: u32 = ParachainInfo::parachain_id().into();
	pub const PolkadotCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
}

parameter_types! {
	pub const TreasuryPalletId: PalletId = PalletId(*b"bf/trsry");
	pub const BifrostCrowdloanId: PalletId = PalletId(*b"bf/salp#");
	pub const LiquidityMiningPalletId: PalletId = PalletId(*b"bf/lm###");
	pub const LighteningRedeemPalletId: PalletId = PalletId(*b"bf/ltnrd");
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
	type SystemWeightInfo = weights::frame_system::WeightInfo<Runtime>;
	/// Runtime version.
	type Version = Version;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	type MinimumPeriod = MinimumPeriod;
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = ();
	type WeightInfo = weights::pallet_timestamp::WeightInfo<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 10 * MILLIBNC;
	pub const TransferFee: Balance = 1 * MILLIBNC;
	pub const CreationFee: Balance = 1 * MILLIBNC;
	pub const TransactionByteFee: Balance = 1 * MICROBNC;
	pub const OperationalFeeMultiplier: u8 = 5;
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_utility::Config for Runtime {
	type Call = Call;
	type Event = Event;
	type WeightInfo = weights::pallet_utility::WeightInfo<Runtime>;
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
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type Call = Call;
	type Event = Event;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type MaximumWeight = MaximumSchedulerWeight;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type WeightInfo = weights::pallet_scheduler::WeightInfo<Runtime>;
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
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
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
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const IndexDeposit: Balance = 1 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type Event = Event;
	type WeightInfo = weights::pallet_indices::WeightInfo<Runtime>;
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
	type WeightInfo = weights::pallet_balances::WeightInfo<Runtime>;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 2 * DAYS;
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
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 2 * DAYS;
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
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
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
	type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
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
	type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const CandidacyBond: Balance = 100 * CENTS;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub const VotingBondBase: Balance = deposit(1, 64);
	// additional data per vote is 32 bytes (account id).
	pub const VotingBondFactor: Balance = deposit(0, 32);
	/// Daily council elections
	pub const TermDuration: BlockNumber = 24 * HOURS;
	pub const DesiredMembers: u32 = 7;
	pub const DesiredRunnersUp: u32 = 7;
	pub const PhragmenElectionPalletId: LockIdentifier = *b"phrelect";
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
	type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 7 * DAYS;
	pub const VotingPeriod: BlockNumber = 7 * DAYS;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
	pub const MinimumDeposit: Balance = 100 * DOLLARS;
	pub const EnactmentPeriod: BlockNumber = 2 * DAYS;
	pub const CooloffPeriod: BlockNumber = 7 * DAYS;
	pub const PreimageByteDeposit: Balance = 10 * MILLICENTS;
	pub const InstantAllowed: bool = true;
	pub const MaxVotes: u32 = 100;
	pub const MaxProposals: u32 = 100;
}

impl pallet_democracy::Config for Runtime {
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EnsureOneOf<
		AccountId,
		EnsureRoot<AccountId>,
		pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>,
	>;
	// To cancel a proposal which has been passed, 2/3 of the council must agree to it.
	type CancellationOrigin =
		pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, CouncilCollective>;
	type CooloffPeriod = CooloffPeriod;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type Event = Event;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin =
		pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin =
		pallet_collective::EnsureProportionAtLeast<_3, _4, AccountId, CouncilCollective>;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin =
		pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin =
		pallet_collective::EnsureProportionAtLeast<_2, _3, AccountId, TechnicalCollective>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	type InstantAllowed = InstantAllowed;
	type InstantOrigin =
		pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, TechnicalCollective>;
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
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 100 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(0);

	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub const TipReportDepositBase: Balance = 1 * DOLLARS;
	pub const DataDepositPerByte: Balance = 10 * CENTS;
	pub const BountyDepositBase: Balance = 1 * DOLLARS;
	pub const BountyDepositPayoutDelay: BlockNumber = 4 * DAYS;
	pub const BountyUpdatePeriod: BlockNumber = 90 * DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub const BountyValueMinimum: Balance = 10 * DOLLARS;
	pub const MaxApprovals: u32 = 100;
}

type ApproveOrigin = EnsureOneOf<
	AccountId,
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>,
>;

impl pallet_treasury::Config for Runtime {
	type ApproveOrigin = ApproveOrigin;
	type Burn = Burn;
	type BurnDestination = ();
	type Currency = Balances;
	type Event = Event;
	type MaxApprovals = MaxApprovals;
	type OnSlash = Treasury;
	type PalletId = TreasuryPalletId;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type RejectOrigin = MoreThanHalfCouncil;
	type SpendFunds = Bounties;
	type SpendPeriod = SpendPeriod;
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
}

impl pallet_bounties::Config for Runtime {
	type BountyCuratorDeposit = BountyCuratorDeposit;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type BountyValueMinimum = BountyValueMinimum;
	type DataDepositPerByte = DataDepositPerByte;
	type Event = Event;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = weights::pallet_bounties::WeightInfo<Runtime>;
}

impl pallet_tips::Config for Runtime {
	type DataDepositPerByte = DataDepositPerByte;
	type Event = Event;
	type MaximumReasonLength = MaximumReasonLength;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type Tippers = PhragmenElection;
	type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

impl pallet_transaction_payment::Config for Runtime {
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OnChargeTransaction = FlexibleFee;
	type TransactionByteFee = TransactionByteFee;
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
	type OnValidationData = ();
	type OutboundXcmpMessageSource = XcmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type XcmpMessageHandler = XcmpQueue;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl pallet_randomness_collective_flip::Config for Runtime {}

parameter_types! {
	pub const KsmLocation: MultiLocation = MultiLocation::parent();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub SelfParaChainId: CumulusParaId = ParachainInfo::parachain_id();
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

/// Type for specifying how a `MultiLocation` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the default `AccountId`.
	ParentIsDefault<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<KsmLocation>,
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
	// recognised.
	RelayChainAsNative<RelayChainOrigin, Origin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognised.
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

match_type! {
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
>;

parameter_types! {
	pub KsmPerSecond: (AssetId, u128) = (MultiLocation::parent().into(), ksm_per_second());
	pub VsksmPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), GeneralKey(CurrencyId::VSToken(TokenSymbol::KSM).encode()))
		).into(),
		ksm_per_second()
	);
	pub BncPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(SelfParaId::get()), GeneralKey(NativeCurrencyId::get().encode()))
		).into(),
		// BNC:KSM = 80:1
		ksm_per_second() * 80
	);
	pub KarPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::karura::ID), GeneralKey(parachains::karura::KAR_KEY.to_vec()))
		).into(),
		// KAR:KSM = 100:1
		ksm_per_second() * 100
	);
	pub KusdPerSecond: (AssetId, u128) = (
		MultiLocation::new(
			1,
			X2(Parachain(parachains::karura::ID), GeneralKey(parachains::karura::KUSD_KEY.to_vec()))
		).into(),
		// kUSD:KSM = 400:1
		ksm_per_second() * 400
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
	FixedRateOfFungible<KsmPerSecond, ToTreasury>,
	FixedRateOfFungible<VsksmPerSecond, ToTreasury>,
	FixedRateOfFungible<BncPerSecond, ToTreasury>,
	FixedRateOfFungible<KarPerSecond, ToTreasury>,
	FixedRateOfFungible<KusdPerSecond, ToTreasury>,
);

pub struct XcmConfig;
impl Config for XcmConfig {
	type AssetClaims = PolkadotXcm;
	type AssetTransactor = BifrostAssetTransactor;
	type AssetTrap = PolkadotXcm;
	type Barrier = Barrier;
	type Call = Call;
	type IsReserve = BifrostFilteredAssets;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type ResponseHandler = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type Trader = Trader;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type XcmSender = XcmRouter;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
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
	type XcmTeleportFilter = Everything;
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
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
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
	type EventHandler = (CollatorSelection,);
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
	pub const MaxCandidates: u32 = 1000;
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
	type WeightInfo = weights::pallet_vesting::WeightInfo<Runtime>;
}

// orml runtime start

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
	type WeightInfo = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			&CurrencyId::Native(TokenSymbol::BNC) => 10 * MILLIBNC,
			&CurrencyId::Stable(TokenSymbol::KUSD) => 10 * MILLICENTS,
			&CurrencyId::Token(TokenSymbol::KSM) => 10 * MILLICENTS,
			&CurrencyId::Token(TokenSymbol::KAR) => 10 * MILLICENTS,
			&CurrencyId::Token(TokenSymbol::DOT) => 10 * MILLICENTS,
			&CurrencyId::Token(TokenSymbol::ZLK) => 1_000_000_000_000,	// ZLK has a decimals of 10e18
			&CurrencyId::VSToken(TokenSymbol::KSM) => 10 * MILLICENTS,
			&CurrencyId::VSToken(TokenSymbol::DOT) => 10 * MILLICENTS,
			&CurrencyId::VSBond(TokenSymbol::BNC, ..) => 10 * MILLICENTS,
			&CurrencyId::VSBond(TokenSymbol::KSM, ..) => 10 * MILLICENTS,
			&CurrencyId::VSBond(TokenSymbol::DOT, ..) => 10 * MILLICENTS,
			&CurrencyId::LPToken(..) => 10 * MILLICENTS,
			_ => Balance::max_value() // unsupported
		}
	};
}

pub struct DustRemovalWhitelist;
impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(a: &AccountId) -> bool {
		AccountIdConversion::<AccountId>::into_account(&TreasuryPalletId::get()).eq(a) ||
			AccountIdConversion::<AccountId>::into_account(&BifrostCrowdloanId::get()).eq(a) ||
			AccountIdConversion::<AccountId>::into_account(&LighteningRedeemPalletId::get())
				.eq(a) || LiquidityMiningPalletId::get().check_sub_account::<PoolId>(a)
	}
}

parameter_types! {
	pub BifrostTreasuryAccount: AccountId = TreasuryPalletId::get().into_account();
}

impl orml_tokens::Config for Runtime {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = DustRemovalWhitelist;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type OnDust = orml_tokens::TransferDust<Runtime, BifrostTreasuryAccount>;
	type WeightInfo = weights::orml_tokens::WeightInfo<Runtime>;
}

parameter_types! {
	pub SelfLocation: MultiLocation = MultiLocation::new(1, X1(Parachain(ParachainInfo::get().into())));
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
	type BaseXcmWeight = XcmWeight;
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

parameter_types! {
	pub const AltFeeCurrencyExchangeRate: (u32, u32) = (1, 100);
	pub SalpWeightHolder: XcmBaseWeight = XcmBaseWeight::from(4 * XCM_WEIGHT) + ContributionWeight::get() + u64::pow(2, 24).into();
}

impl bifrost_flexible_fee::Config for Runtime {
	type Currency = Balances;
	type DexOperator = ZenlinkProtocol;
	// type FeeDealer = FlexibleFee;
	type FeeDealer = FixedCurrencyFeeRate<Runtime>;
	type Event = Event;
	type MultiCurrency = Currencies;
	type TreasuryAccount = BifrostTreasuryAccount;
	type NativeCurrencyId = NativeCurrencyId;
	type AlternativeFeeCurrencyId = RelayCurrencyId;
	type AltFeeCurrencyExchangeRate = AltFeeCurrencyExchangeRate;
	type OnUnbalanced = Treasury;
	type WeightInfo = weights::bifrost_flexible_fee::WeightInfo<Runtime>;
	type ExtraFeeMatcher = ExtraFeeMatcher<Runtime, FeeNameGetter, AggregateExtraFeeFilter>;
	type MiscFeeHandler = MiscFeeHandler<
		Runtime,
		RelayCurrencyId,
		WeightToFee,
		SalpWeightHolder,
		ContributeFeeFilter,
	>;
}

pub struct EnsureConfirmAsMultiSig;
impl EnsureOrigin<Origin> for EnsureConfirmAsMultiSig {
	type Success = AccountId;

	fn try_origin(o: Origin) -> Result<Self::Success, Origin> {
		Into::<Result<RawOrigin<AccountId>, Origin>>::into(o).and_then(|o| match o {
			RawOrigin::Signed(who) =>
				if who == ConfirmMuitiSigAccount::get() {
					Ok(who)
				} else {
					Err(Origin::from(Some(who)))
				},
			r => Err(Origin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> Origin {
		Origin::from(RawOrigin::Signed(Default::default()))
	}
}

pub fn create_x2_multilocation(index: u16) -> MultiLocation {
	MultiLocation::new(
		1,
		X1(AccountId32 {
			network: NetworkId::Any,
			id: Utility::derivative_account_id(ParachainInfo::get().into_account(), index).into(),
		}),
	)
}

parameter_types! {
	pub const MinContribution: Balance = DOLLARS / 10;
	pub const RemoveKeysLimit: u32 = 500;
	pub const VSBondValidPeriod: BlockNumber = 30 * DAYS;
	pub const ReleaseCycle: BlockNumber = 1 * DAYS;
	pub const LeasePeriod: BlockNumber = KUSAMA_LEASE_PERIOD;
	pub const ReleaseRatio: Percent = Percent::from_percent(50);
	pub const SlotLength: BlockNumber = 8u32 as BlockNumber;
	pub const XcmTransferOrigin: TransferOriginType = TransferOriginType::FromRelayChain;
	pub XcmWeight: XcmBaseWeight = XCM_WEIGHT.into();
	pub ContributionWeight:XcmBaseWeight = XCM_WEIGHT.into();
	pub AddProxyWeight:XcmBaseWeight = XCM_WEIGHT.into();
	pub ConfirmMuitiSigAccount: AccountId = hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();
	pub RelaychainSovereignSubAccount: MultiLocation = create_x2_multilocation(ParachainDerivedProxyAccountType::Salp as u16);
	pub SalpTransactType: ParachainTransactType = ParachainTransactType::Xcm;
	pub SalpProxyType: ParachainTransactProxyType = ParachainTransactProxyType::Derived;
}

impl bifrost_salp::Config for Runtime {
	type BancorPool = ();
	type BifrostXcmExecutor = BifrostXcmAdaptor<XcmRouter, XcmWeight, WeightToFee, SelfParaId>;
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
	type XcmTransferOrigin = XcmTransferOrigin;
	type WeightInfo = weights::bifrost_salp::WeightInfo<Runtime>;
	type SelfParaId = SelfParaId;
	type ContributionWeight = ContributionWeight;
	type BaseXcmWeight = XcmWeight;
	type EnsureConfirmAsMultiSig =
		EnsureOneOf<AccountId, MoreThanHalfCouncil, EnsureConfirmAsMultiSig>;
	type EnsureConfirmAsGovernance =
		EnsureOneOf<AccountId, MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type AddProxyWeight = AddProxyWeight;
	type XcmTransfer = XTokens;
	type SovereignSubAccountLocation = RelaychainSovereignSubAccount;
	type TransactProxyType = SalpProxyType;
	type TransactType = SalpTransactType;
	type RelayNetwork = RelayNetwork;
}

impl bifrost_salp_lite::Config for Runtime {
	type BancorPool = ();
	type Event = Event;
	type LeasePeriod = LeasePeriod;
	type MinContribution = MinContribution;
	type MultiCurrency = Currencies;
	type PalletId = BifrostCrowdloanId;
	type RelayChainToken = PolkadotCurrencyId;
	type ReleaseCycle = ReleaseCycle;
	type ReleaseRatio = ReleaseRatio;
	type BatchKeysLimit = RemoveKeysLimit;
	type SlotLength = SlotLength;
	type WeightInfo = weights::bifrost_salp_lite::WeightInfo<Runtime>;
	type EnsureConfirmAsMultiSig =
		EnsureOneOf<AccountId, MoreThanHalfCouncil, EnsureConfirmAsMultiSig>;
	type EnsureConfirmAsGovernance =
		EnsureOneOf<AccountId, MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
}

parameter_types! {
	pub const RelayChainTokenSymbol: TokenSymbol = TokenSymbol::KSM;
	pub const MaximumDepositInPool: Balance = 1_000_000_000 * DOLLARS;
	pub const MinimumDepositOfUser: Balance = 1_000_000;
	pub const MinimumRewardPerBlock: Balance = 1_000;
	pub const MinimumDuration: BlockNumber = HOURS;
	pub const MaximumOptionRewards: u32 = 7;
	pub const MaximumCharged: u32 = 32;
}

impl bifrost_liquidity_mining::Config for Runtime {
	type Event = Event;
	type ControlOrigin = MoreThanHalfCouncil;
	type MultiCurrency = Currencies;
	type RelayChainTokenSymbol = RelayChainTokenSymbol;
	type MaximumDepositInPool = MaximumDepositInPool;
	type MinimumDepositOfUser = MinimumDepositOfUser;
	type MinimumRewardPerBlock = MinimumRewardPerBlock;
	type MinimumDuration = MinimumDuration;
	type MaximumCharged = MaximumCharged;
	type MaximumOptionRewards = MaximumOptionRewards;
	type PalletId = LiquidityMiningPalletId;
	type WeightInfo = weights::bifrost_liquidity_mining::WeightInfo<Runtime>;
}

impl bifrost_token_issuer::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Currencies;
	type ControlOrigin =
		EnsureOneOf<AccountId, MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type WeightInfo = weights::bifrost_token_issuer::WeightInfo<Runtime>;
}

impl bifrost_lightening_redeem::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Tokens;
	type ControlOrigin =
		EnsureOneOf<AccountId, MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type PalletId = LighteningRedeemPalletId;
	type WeightInfo = weights::bifrost_lightening_redeem::WeightInfo<Runtime>;
}

impl bifrost_call_switchgear::Config for Runtime {
	type Event = Event;
	type UpdateOrigin =
		EnsureOneOf<AccountId, MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type WeightInfo = weights::bifrost_call_switchgear::WeightInfo<Runtime>;
}

// Bifrost modules end

// zenlink runtime start
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

// Below is the implementation of tokens manipulation functions other than native token.
pub struct LocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
{
	fn local_balance_of(asset_id: ZenlinkAssetId, who: &AccountId) -> AssetBalance {
		let currency_id: CurrencyId = asset_id.try_into().unwrap_or_default();
		Local::free_balance(currency_id, &who).saturated_into()
	}

	fn local_total_supply(asset_id: ZenlinkAssetId) -> AssetBalance {
		let currency_id: CurrencyId = asset_id.try_into().unwrap_or_default();
		Local::total_issuance(currency_id).saturated_into()
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
		let currency_id: CurrencyId = asset_id.try_into().unwrap_or_default();
		Local::transfer(currency_id, &origin, &target, amount.unique_saturated_into())?;

		Ok(())
	}

	fn local_deposit(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		let currency_id: CurrencyId = asset_id.try_into().unwrap_or_default();
		Local::deposit(currency_id, &origin, amount.unique_saturated_into())?;
		return Ok(amount);
	}

	fn local_withdraw(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		let currency_id: CurrencyId = asset_id.try_into().unwrap_or_default();
		Local::withdraw(currency_id, &origin, amount.unique_saturated_into())?;

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
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage} = 4,
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Config, Storage, Inherent, Event<T>, ValidateUnsigned} = 5,
		ParachainInfo: parachain_info::{Pallet, Storage, Config} = 6,

		// Monetary stuff
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 10,
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 11,

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
		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin, Config} = 41,
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin} = 42,
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

		// Third party modules
		XTokens: orml_xtokens::{Pallet, Call, Event<T>} = 70,
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>, Config<T>} = 71,
		Currencies: orml_currencies::{Pallet, Call, Event<T>} = 72,
		UnknownTokens: orml_unknown_tokens::{Pallet, Storage, Event} = 73,
		OrmlXcm: orml_xcm::{Pallet, Call, Event<T>} = 74,
		ZenlinkProtocol: zenlink_protocol::{Pallet, Call, Storage, Event<T>} = 80,

		// Bifrost modules
		FlexibleFee: bifrost_flexible_fee::{Pallet, Call, Storage, Event<T>} = 100,
		Salp: bifrost_salp::{Pallet, Call, Storage, Event<T>} = 105,
		LiquidityMining: bifrost_liquidity_mining::{Pallet, Call, Storage, Event<T>} = 108,
		TokenIssuer: bifrost_token_issuer::{Pallet, Call, Storage, Event<T>} = 109,
		LighteningRedeem: bifrost_lightening_redeem::{Pallet, Call, Storage, Event<T>} = 110,
		SalpLite: bifrost_salp_lite::{Pallet, Call, Storage, Event<T>} = 111,
		CallSwitchgear: bifrost_call_switchgear::{Pallet, Storage, Call, Event<T>} = 112,
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
	AllPallets,
	CustomOnRuntimeUpgrade,
>;

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
		fn collect_collation_info() -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info()
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
	}

	impl bifrost_liquidity_mining_rpc_runtime_api::LiquidityMiningRuntimeApi<Block, AccountId, PoolId> for Runtime {
		fn get_rewards(who: AccountId, pid: PoolId) -> Vec<(CurrencyId, Balance)> {
			LiquidityMining::rewards(who, pid).unwrap_or(Vec::new())
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			use frame_support::traits::StorageInfoTrait;
			use frame_benchmarking::{list_benchmark, Benchmarking, BenchmarkList};

			let mut list = Vec::<BenchmarkList>::new();

			list_benchmark!(list, extra, bifrost_flexible_fee, FlexibleFee);
			list_benchmark!(list, extra, bifrost_salp, Salp);
			// list_benchmark!(list, extra, bifrost_salp_lite, SalpLite);
			list_benchmark!(list, extra, bifrost_liquidity_mining, LiquidityMining);
			list_benchmark!(list, extra, bifrost_token_issuer, TokenIssuer);
			list_benchmark!(list, extra, bifrost_lightening_redeem, LighteningRedeem);
			list_benchmark!(list, extra, bifrost_call_switchgear, CallSwitchgear);

			let storage_info = AllPalletsWithSystem::storage_info();

			return (list, storage_info)
		}
		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};
			use frame_system_benchmarking::Pallet as SystemBench;

			impl frame_system_benchmarking::Config for Runtime {}

			let whitelist: Vec<TrackedStorageKey> = vec![
			// you can whitelist any storage keys you do not want to track here
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			// Adding the pallet you will perform the benchmarking
			add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
			add_benchmark!(params, batches, pallet_balances, Balances);
			add_benchmark!(params, batches, pallet_bounties, Bounties);
			add_benchmark!(params, batches, pallet_indices, Indices);
			add_benchmark!(params, batches, pallet_scheduler, Scheduler);
			add_benchmark!(params, batches, pallet_timestamp, Timestamp);
			add_benchmark!(params, batches, pallet_treasury, Treasury);
			add_benchmark!(params, batches, pallet_utility, Utility);
			add_benchmark!(params, batches, pallet_vesting, Vesting);

			add_benchmark!(params, batches, bifrost_flexible_fee, FlexibleFee);
			add_benchmark!(params, batches, bifrost_salp, Salp);
			// add_benchmark!(params, batches, bifrost_salp_lite, SalpLite);
			add_benchmark!(params, batches, bifrost_liquidity_mining, LiquidityMining);
			add_benchmark!(params, batches, bifrost_token_issuer, TokenIssuer);
			add_benchmark!(params, batches, bifrost_lightening_redeem, LighteningRedeem);
			add_benchmark!(params, batches, bifrost_call_switchgear, CallSwitchgear);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade() -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade bifrost.");
			let weight = Executive::try_runtime_upgrade().unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}
		fn execute_block_no_check(block: Block) -> Weight {
			Executive::execute_block_no_check(block)
		}
	}
}

pub struct CustomOnRuntimeUpgrade;
impl OnRuntimeUpgrade for CustomOnRuntimeUpgrade {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		#[allow(unused_imports)]
		use frame_support::{migration, Identity};

		log::info!("Bifrost `pre_upgrade`...");

		Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn on_runtime_upgrade() -> Weight {
		log::info!("Bifrost `on_runtime_upgrade`...");
		let _ = PolkadotXcm::force_default_xcm_version(Origin::root(), Some(2));
		log::info!("Bifrost `on_runtime_upgrade finished`");
		RocksDbWeight::get().writes(1)
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

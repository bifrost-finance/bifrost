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

//! The Bifrost Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 512.
#![recursion_limit = "512"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use bifrost_slp::{DerivativeAccountProvider, QueryResponseManager};
use core::convert::TryInto;
// A few exports that help ease life for downstream crates.
use cumulus_pallet_parachain_system::{RelayNumberStrictlyIncreases, RelaychainDataProvider};
pub use frame_support::{
	construct_runtime, match_types, parameter_types,
	traits::{
		ConstBool, ConstU128, ConstU32, ConstU64, ConstU8, Contains, EqualPrivilegeOnly,
		Everything, Imbalance, InstanceFilter, IsInVec, LockIdentifier, NeverEnsureOrigin, Nothing,
		OnUnbalanced, Randomness, WithdrawReasons,
	},
	weights::{
		constants::{
			BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND,
		},
		ConstantMultiplier, IdentityFee, Weight,
	},
	PalletId, StorageValue,
};
use frame_system::limits::{BlockLength, BlockWeights};
use orml_oracle::{DataFeeder, DataProvider, DataProviderExtended};
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use sp_api::impl_runtime_apis;
use sp_arithmetic::Percent;
use sp_core::{OpaqueMetadata, U256};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdConversion, BlakeTwo256, Block as BlockT, Zero},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, DispatchError, DispatchResult, Perbill, Permill, RuntimeDebug,
};
use sp_std::{marker::PhantomData, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

/// Constant values used within the runtime.
pub mod constants;
mod migration;
pub mod weights;
use bifrost_asset_registry::{AssetIdMaps, FixedRateOfAsset};
pub use bifrost_primitives::{
	traits::{
		CheckSubAccount, FarmingInfo, FeeGetter, VtokenMintingInterface, VtokenMintingOperator,
		XcmDestWeightAndFeeHandler,
	},
	AccountId, Amount, AssetIds, Balance, BlockNumber, CurrencyId, CurrencyIdMapping,
	DistributionId, ExtraFeeInfo, ExtraFeeName, Liquidity, Moment, Nonce, ParaId, PoolId, Price,
	Rate, Ratio, RpcContributionStatus, Shortfall, TimeUnit, TokenSymbol, DOT_TOKEN_ID,
	GLMR_TOKEN_ID,
};
use bifrost_runtime_common::{
	constants::time::*, dollar, micro, milli, AuraId, CouncilCollective,
	EnsureRootOrAllTechnicalCommittee, MoreThanHalfCouncil, SlowAdjustingFeeUpdate,
	TechnicalCollective,
};
use bifrost_slp::QueryId;
use bifrost_ve_minting::traits::VeMintingInterface;
use constants::currency::*;
use cumulus_primitives_core::ParaId as CumulusParaId;
use frame_support::{
	dispatch::DispatchClass,
	genesis_builder_helper::{build_config, create_default_config},
	sp_runtime::traits::{Convert, ConvertInto},
	traits::{
		fungible::HoldConsideration,
		tokens::{PayFromAccount, UnityAssetBalanceConversion},
		Currency, EitherOf, EitherOfDiverse, Get, LinearStoragePrice,
	},
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess, EnsureSigned};
use hex_literal::hex;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
// zenlink imports
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler, MultiAssetsHandler, PairInfo,
	PairLpGenerate, ZenlinkMultiAssets,
};
// xcm config
pub mod xcm_config;
use orml_traits::{currency::MutationHooks, location::RelativeReserveProvider};
use pallet_identity::legacy::IdentityInfo;
use pallet_xcm::{EnsureResponse, QueryStatus};
use polkadot_runtime_common::prod_or_fast;
use sp_runtime::traits::{IdentityLookup, Verify};
use static_assertions::const_assert;
use xcm::{v3::MultiLocation, v4::prelude::*};
pub use xcm_config::{
	parachains, BifrostCurrencyIdConvert, BifrostTreasuryAccount, MultiCurrency, SelfParaChainId,
};
use xcm_executor::{
	traits::{Properties, QueryHandler},
	XcmExecutor,
};

pub mod governance;
use crate::xcm_config::XcmRouter;
use governance::{
	custom_origins, CoreAdminOrCouncil, LiquidStaking, SALPAdmin, Spender, TechAdmin,
	TechAdminOrCouncil,
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
	spec_version: 12000,
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
/// We allow for 0.5 of a second of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
	cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

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
impl Contains<RuntimeCall> for CallFilter {
	fn contains(call: &RuntimeCall) -> bool {
		let is_core_call = matches!(
			call,
			RuntimeCall::System(_) | RuntimeCall::Timestamp(_) | RuntimeCall::ParachainSystem(_)
		);
		if is_core_call {
			// always allow core call
			return true;
		}

		if bifrost_call_switchgear::OverallToggleFilter::<Runtime>::get_overall_toggle_status() {
			return false;
		}

		// temporarily ban PhragmenElection
		let is_temporarily_banned = matches!(call, RuntimeCall::PhragmenElection(_));

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
		let is_transfer = matches!(
			call,
			RuntimeCall::Currencies(_) | RuntimeCall::Tokens(_) | RuntimeCall::Balances(_)
		);
		if is_transfer {
			let is_disabled = match *call {
				// bifrost-currencies module
				RuntimeCall::Currencies(bifrost_currencies::Call::transfer {
					dest: _,
					currency_id,
					amount: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&currency_id,
				),
				RuntimeCall::Currencies(bifrost_currencies::Call::transfer_native_currency {
					dest: _,
					amount: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&NativeCurrencyId::get(),
				),
				// orml-tokens module
				RuntimeCall::Tokens(orml_tokens::Call::transfer {
					dest: _,
					currency_id,
					amount: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&currency_id,
				),
				RuntimeCall::Tokens(orml_tokens::Call::transfer_all {
					dest: _,
					currency_id,
					keep_alive: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&currency_id,
				),
				RuntimeCall::Tokens(orml_tokens::Call::transfer_keep_alive {
					dest: _,
					currency_id,
					amount: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&currency_id,
				),
				// Balances module
				RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
					dest: _,
					value: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&NativeCurrencyId::get(),
				),
				RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
					dest: _,
					value: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
					&NativeCurrencyId::get(),
				),
				RuntimeCall::Balances(pallet_balances::Call::transfer_all {
					dest: _,
					keep_alive: _,
				}) => bifrost_call_switchgear::DisableTransfersFilter::<Runtime>::contains(
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
	pub const SystemStakingPalletId: PalletId = PalletId(*b"bf/sysst");
	pub const BuybackPalletId: PalletId = PalletId(*b"bf/salpc");
	pub const SystemMakerPalletId: PalletId = PalletId(*b"bf/sysmk");
	pub const FeeSharePalletId: PalletId = PalletId(*b"bf/feesh");
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
	pub const VeMintingPalletId: PalletId = PalletId(*b"bf/vemnt");
	pub const IncentivePalletId: PalletId = PalletId(*b"bf/veict");
	pub const FarmingBoostPalletId: PalletId = PalletId(*b"bf/fmbst");
	pub const LendMarketPalletId: PalletId = PalletId(*b"bf/ldmkt");
	pub const OraclePalletId: PalletId = PalletId(*b"bf/oracl");
	pub const StableAssetPalletId: PalletId = PalletId(*b"bf/stabl");
	pub const CommissionPalletId: PalletId = PalletId(*b"bf/comms");
	pub const CloudsPalletId: PalletId = PalletId(*b"bf/cloud");
	pub IncentivePoolAccount: PalletId = PalletId(*b"bf/inpoo");
	pub const FarmingGaugeRewardIssuerPalletId: PalletId = PalletId(*b"bf/fmgar");
	pub const BuyBackAccount: PalletId = PalletId(*b"bf/bybck");
	pub const LiquidityAccount: PalletId = PalletId(*b"bf/liqdt");
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
	type Nonce = Nonce;
	type BlockWeights = RuntimeBlockWeights;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	type DbWeight = RocksDbWeight;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	type Block = Block;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = Indices;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	type SS58Prefix = SS58Prefix;
	type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
	/// Runtime version.
	type Version = Version;
	type MaxConsumers = ConstU32<16>;
	type RuntimeTask = ();
}

impl pallet_timestamp::Config for Runtime {
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = Aura;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 10 * MILLIBNC;
	pub const TransferFee: Balance = 1 * MILLIBNC;
	pub const CreationFee: Balance = 1 * MILLIBNC;
	pub const TransactionByteFee: Balance = 16 * MICROBNC;
}

impl pallet_utility::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
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
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => matches!(
				c,
				RuntimeCall::System(..) |
				RuntimeCall::Scheduler(..) |
				RuntimeCall::Preimage(_) |
				RuntimeCall::Timestamp(..) |
				RuntimeCall::Indices(pallet_indices::Call::claim{..}) |
				RuntimeCall::Indices(pallet_indices::Call::free{..}) |
				RuntimeCall::Indices(pallet_indices::Call::freeze{..}) |
				// Specifically omitting Indices `transfer`, `force_transfer`
				// Specifically omitting the entire Balances pallet
				RuntimeCall::Session(..) |
				RuntimeCall::Democracy(..) |
				RuntimeCall::Council(..) |
				RuntimeCall::TechnicalCommittee(..) |
				RuntimeCall::PhragmenElection(..) |
				RuntimeCall::TechnicalMembership(..) |
				RuntimeCall::Treasury(..) |
				RuntimeCall::Vesting(bifrost_vesting::Call::vest{..}) |
				RuntimeCall::Vesting(bifrost_vesting::Call::vest_other{..}) |
				// Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
				RuntimeCall::Utility(..) |
				RuntimeCall::Proxy(..) |
				RuntimeCall::Multisig(..)
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Democracy(..) |
					RuntimeCall::Council(..) |
					RuntimeCall::TechnicalCommittee(..) |
					RuntimeCall::PhragmenElection(..) |
					RuntimeCall::Treasury(..) |
					RuntimeCall::Utility(..)
			),
			ProxyType::CancelProxy => {
				matches!(c, RuntimeCall::Proxy(pallet_proxy::Call::reject_announcement { .. }))
			},
			ProxyType::IdentityJudgement => matches!(
				c,
				RuntimeCall::Identity(pallet_identity::Call::provide_judgement { .. }) |
					RuntimeCall::Utility(..)
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
	type RuntimeCall = RuntimeCall;
	type CallHasher = BlakeTwo256;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type MaxPending = MaxPending;
	type MaxProxies = MaxProxies;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type ProxyType = ProxyType;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub PreimageBaseDeposit: Balance = deposit(2, 64);
	pub PreimageByteDeposit: Balance = deposit(0, 1);
	pub const PreimageHoldReason: RuntimeHoldReason = RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
	type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
	pub const NoPreimagePostponement: Option<u32> = Some(10);
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type MaximumWeight = MaximumSchedulerWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type PalletsOrigin = OriginCaller;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
	type Preimages = Preimage;
}

parameter_types! {
	// One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
	pub const DepositBase: Balance = deposit(1, 88);
	// Additional storage item size of 32 bytes.
	pub const DepositFactor: Balance = deposit(0, 32);
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type RuntimeEvent = RuntimeEvent;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// 1 entry, storing 258 bytes on-chain
	pub const BasicDeposit: Balance = deposit(1, 258);
	   // Additional bytes adds 0 entries, storing 1 byte on-chain
	pub const ByteDeposit: Balance = deposit(0, 1);
	// 1 entry, storing 53 bytes on-chain
	pub const SubAccountDeposit: Balance = deposit(1, 53);
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = MoreThanHalfCouncil;
	type RegistrarOrigin = MoreThanHalfCouncil;
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
	type ByteDeposit = ByteDeposit;
	type OffchainSignature = Signature;
	type SigningPublicKey = <Signature as Verify>::Signer;
	type UsernameAuthorityOrigin = EnsureRoot<Self::AccountId>;
	type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
	type MaxSuffixLength = ConstU32<7>;
	type MaxUsernameLength = ConstU32<32>;
}

parameter_types! {
	pub const IndexDeposit: Balance = 10 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

// pallet-treasury did not impl OnUnbalanced<Credit>, need an adapter to handle dust.
type CreditOf =
	frame_support::traits::fungible::Credit<<Runtime as frame_system::Config>::AccountId, Balances>;
pub struct DustRemovalAdapter;
impl OnUnbalanced<CreditOf> for DustRemovalAdapter {
	fn on_nonzero_unbalanced(amount: CreditOf) {
		let _ = <Balances as Currency<AccountId>>::deposit_creating(
			&TreasuryPalletId::get().into_account_truncating(),
			amount.peek(),
		);
	}
}

impl pallet_balances::Config for Runtime {
	type AccountStore = System;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = DustRemovalAdapter;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 7 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 100;
}

impl pallet_collective::Config<CouncilCollective> for Runtime {
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type RuntimeEvent = RuntimeEvent;
	type MaxMembers = CouncilMaxMembers;
	type MaxProposals = CouncilMaxProposals;
	type MotionDuration = CouncilMotionDuration;
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type MaxProposalWeight = MaxProposalWeight;
	type SetMembersOrigin = EnsureRoot<AccountId>;
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 7 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
	pub MaxProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

impl pallet_collective::Config<TechnicalCollective> for Runtime {
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type RuntimeEvent = RuntimeEvent;
	type MaxMembers = TechnicalMaxMembers;
	type MaxProposals = TechnicalMaxProposals;
	type MotionDuration = TechnicalMotionDuration;
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
	type MaxProposalWeight = MaxProposalWeight;
	type SetMembersOrigin = EnsureRoot<AccountId>;
}

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
	type AddOrigin = MoreThanHalfCouncil;
	type RuntimeEvent = RuntimeEvent;
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
	type RuntimeEvent = RuntimeEvent;
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
	pub const CandidacyBond: Balance = 10_000 * DOLLARS;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub const VotingBondBase: Balance = deposit(1, 64);
	// additional data per vote is 32 bytes (account id).
	pub const VotingBondFactor: Balance = deposit(0, 32);
	/// Daily council elections
	pub const TermDuration: BlockNumber = 7 * DAYS;
	pub const DesiredMembers: u32 = 3;
	pub const DesiredRunnersUp: u32 = 20;
	pub const PhragmenElectionPalletId: LockIdentifier = *b"phrelect";
	pub const MaxVoters: u32 = 512;
	 pub const MaxVotesPerVoter: u32 = 16;
	pub const MaxCandidates: u32 = 64;
}

// Make sure that there are no more than MaxMembers members elected via phragmen.
const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
	type CandidacyBond = CandidacyBond;
	type ChangeMembers = Council;
	type Currency = Balances;
	type CurrencyToVote = sp_staking::currency_to_vote::U128CurrencyToVote;
	type DesiredMembers = DesiredMembers;
	type DesiredRunnersUp = DesiredRunnersUp;
	type RuntimeEvent = RuntimeEvent;
	type InitializeMembers = Council;
	type KickedMember = Treasury;
	type LoserCandidate = Treasury;
	type PalletId = PhragmenElectionPalletId;
	type TermDuration = TermDuration;
	type VotingBondBase = VotingBondBase;
	type VotingBondFactor = VotingBondFactor;
	type MaxCandidates = MaxCandidates;
	type MaxVoters = MaxVoters;
	type MaxVotesPerVoter = MaxVotesPerVoter;
	type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
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
	type RuntimeEvent = RuntimeEvent;
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
	type PalletsOrigin = OriginCaller;
	type Scheduler = Scheduler;
	type Slash = Treasury;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type VoteLockingPeriod = EnactmentPeriod; // Same as EnactmentPeriod
	type VotingPeriod = VotingPeriod;
	type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
	type Preimages = Preimage;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
	type SubmitOrigin = EnsureSigned<AccountId>;
}

parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 100 * DOLLARS;
	pub const ProposalBondMaximum: Balance = 500 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(0);
	pub const TipReportDepositBase: Balance = 1 * DOLLARS;
	pub const DataDepositPerByte: Balance = 1 * CENTS;
	pub const MaxApprovals: u32 = 100;
	pub const MaxBalance: Balance = 800_000 * BNCS;
}

type ApproveOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
>;

impl pallet_treasury::Config for Runtime {
	type ApproveOrigin = ApproveOrigin;
	type SpendOrigin = EitherOf<EnsureRootWithSuccess<AccountId, MaxBalance>, Spender>;
	type Burn = Burn;
	type BurnDestination = ();
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type MaxApprovals = MaxApprovals;
	type AssetKind = ();
	type Beneficiary = AccountId;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = PayFromAccount<Balances, BifrostFeeAccount>;
	type BalanceConverter = UnityAssetBalanceConversion;
	type PayoutPeriod = PayoutSpendPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = ();
	type OnSlash = Treasury;
	type PalletId = TreasuryPalletId;
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ProposalBondMaximum;
	type RejectOrigin = MoreThanHalfCouncil;
	type SpendFunds = ();
	type SpendPeriod = SpendPeriod;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type OnChargeTransaction = FlexibleFee;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = WeightToFee;
}

// culumus runtime start
parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, xcm_config::RelayOrigin>;
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type OutboundXcmpMessageSource = XcmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type XcmpMessageHandler = XcmpQueue;
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
	type ConsensusHook = cumulus_pallet_parachain_system::ExpectParentIncluded;
	type WeightInfo = cumulus_pallet_parachain_system::weights::SubstrateWeight<Runtime>;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Keys = SessionKeys;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	// Essentially just Aura, but lets be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type SessionManager = CollatorSelection;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type EventHandler = CollatorSelection;
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const SessionLength: BlockNumber = 6 * HOURS;
	pub const MaxInvulnerables: u32 = 100;
}

impl pallet_collator_selection::Config for Runtime {
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type MaxCandidates = MaxCandidates;
	type MaxInvulnerables = MaxInvulnerables;
	type PotId = PotId;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = ();
	type MinEligibleCollators = ConstU32<5>;
}

// culumus runtime end

parameter_types! {
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl bifrost_vesting::Config for Runtime {
	type BlockNumberToBalance = ConvertInto;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type MinVestedTransfer = ExistentialDeposit;
	type WeightInfo = weights::bifrost_vesting::BifrostWeight<Runtime>;
	type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
	const MAX_VESTING_SCHEDULES: u32 = 28;
}

// Bifrost modules start

pub struct ExtraFeeMatcher;
impl FeeGetter<RuntimeCall> for ExtraFeeMatcher {
	fn get_fee_info(c: &RuntimeCall) -> ExtraFeeInfo {
		match *c {
			RuntimeCall::Salp(bifrost_salp::Call::contribute { .. }) => ExtraFeeInfo {
				extra_fee_name: ExtraFeeName::SalpContribute,
				extra_fee_currency: RelayCurrencyId::get(),
			},
			RuntimeCall::XcmInterface(bifrost_xcm_interface::Call::transfer_statemine_assets {
				..
			}) => ExtraFeeInfo {
				extra_fee_name: ExtraFeeName::StatemineTransfer,
				extra_fee_currency: RelayCurrencyId::get(),
			},
			RuntimeCall::VtokenVoting(bifrost_vtoken_voting::Call::vote { vtoken, .. }) =>
				ExtraFeeInfo {
					extra_fee_name: ExtraFeeName::VoteVtoken,
					extra_fee_currency: vtoken.to_token().unwrap_or(vtoken),
				},
			RuntimeCall::VtokenVoting(bifrost_vtoken_voting::Call::remove_delegator_vote {
				vtoken,
				..
			}) => ExtraFeeInfo {
				extra_fee_name: ExtraFeeName::VoteRemoveDelegatorVote,
				extra_fee_currency: vtoken.to_token().unwrap_or(vtoken),
			},
			_ => ExtraFeeInfo::default(),
		}
	}
}

parameter_types! {
	pub MaxFeeCurrencyOrderListLen: u32 = 50;
}

impl bifrost_flexible_fee::Config for Runtime {
	type Currency = Balances;
	type DexOperator = ZenlinkProtocol;
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type TreasuryAccount = BifrostTreasuryAccount;
	type MaxFeeCurrencyOrderListLen = MaxFeeCurrencyOrderListLen;
	type OnUnbalanced = Treasury;
	type WeightInfo = weights::bifrost_flexible_fee::BifrostWeight<Runtime>;
	type ExtraFeeMatcher = ExtraFeeMatcher;
	type ParachainId = ParachainInfo;
	type ControlOrigin = TechAdminOrCouncil;
	type XcmWeightAndFeeHandler = XcmInterface;
}

parameter_types! {
	pub BifrostParachainAccountId20: [u8; 20] = cumulus_primitives_core::ParaId::from(ParachainInfo::get()).into_account_truncating();
}

pub fn create_x2_multilocation(index: u16, currency_id: CurrencyId) -> MultiLocation {
	match currency_id {
		CurrencyId::Token2(GLMR_TOKEN_ID) => MultiLocation::new(
			1,
			xcm::v3::Junctions::X2(
				xcm::v3::Junction::Parachain(parachains::moonbeam::ID.into()),
				xcm::v3::Junction::AccountKey20 {
					network: None,
					key: Slp::derivative_account_id_20(
						polkadot_parachain_primitives::primitives::Sibling::from(
							ParachainInfo::get(),
						)
						.into_account_truncating(),
						index,
					)
					.into(),
				},
			),
		),
		// Only relay chain use the Bifrost para account with "para"
		CurrencyId::Token2(DOT_TOKEN_ID) => xcm::v3::Location::new(
			1,
			xcm::v3::Junctions::X1(xcm::v3::Junction::AccountId32 {
				network: None,
				id: Utility::derivative_account_id(
					ParachainInfo::get().into_account_truncating(),
					index,
				)
				.into(),
			}),
		),
		// Bifrost Polkadot Native token
		CurrencyId::Native(TokenSymbol::BNC) => xcm::v3::Location::new(
			0,
			xcm::v3::Junctions::X1(xcm::v3::Junction::AccountId32 {
				network: None,
				id: Utility::derivative_account_id(
					polkadot_parachain_primitives::primitives::Sibling::from(ParachainInfo::get())
						.into_account_truncating(),
					index,
				)
				.into(),
			}),
		),
		// Other sibling chains use the Bifrost para account with "sibl"
		_ => {
			// get parachain id
			if let Some(location) =
				BifrostCurrencyIdConvert::<SelfParaChainId>::convert(currency_id)
			{
				if let Some(Parachain(para_id)) = location.interior().first() {
					xcm::v3::Location::new(
						1,
						xcm::v3::Junctions::X2(
							xcm::v3::Junction::Parachain(*para_id),
							xcm::v3::Junction::AccountId32 {
								network: None,
								id: Utility::derivative_account_id(
									polkadot_parachain_primitives::primitives::Sibling::from(
										ParachainInfo::get(),
									)
									.into_account_truncating(),
									index,
								)
								.into(),
							},
						),
					)
				} else {
					xcm::v3::Location::default()
				}
			} else {
				xcm::v3::Location::default()
			}
		},
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
	pub const SalpLockId: LockIdentifier = *b"salplock";
	pub const BatchLimit: u32 = 50;
}

impl bifrost_salp::Config for Runtime {
	type BancorPool = ();
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
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
	type WeightInfo = weights::bifrost_salp::BifrostWeight<Runtime>;
	type EnsureConfirmAsGovernance = EitherOfDiverse<TechAdminOrCouncil, SALPAdmin>;
	type XcmInterface = XcmInterface;
	type TreasuryAccount = BifrostTreasuryAccount;
	type BuybackPalletId = BuybackPalletId;
	type DexOperator = ZenlinkProtocol;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type CurrencyIdRegister = AssetIdMaps<Runtime>;
	type ParachainId = ParachainInfo;
	type StablePool = StablePool;
	type VtokenMinting = VtokenMinting;
	type LockId = SalpLockId;
	type BatchLimit = BatchLimit;
}

impl bifrost_call_switchgear::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type UpdateOrigin = CoreAdminOrCouncil;
	type WeightInfo = weights::bifrost_call_switchgear::BifrostWeight<Runtime>;
}

impl bifrost_asset_registry::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EitherOfDiverse<MoreThanHalfCouncil, TechAdmin>;
	type WeightInfo = weights::bifrost_asset_registry::BifrostWeight<Runtime>;
}

parameter_types! {
	pub const MaxTypeEntryPerBlock: u32 = 10;
	pub const MaxRefundPerBlock: u32 = 10;
	pub const MaxLengthLimit: u32 = 500;
}

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
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type MultiCurrency = Currencies;
	type ControlOrigin = EitherOfDiverse<TechAdminOrCouncil, LiquidStaking>;
	type WeightInfo = weights::bifrost_slp::BifrostWeight<Runtime>;
	type VtokenMinting = VtokenMinting;
	type AccountConverter = SubAccountIndexMultiLocationConvertor;
	type ParachainId = SelfParaChainId;
	type SubstrateResponseManager = SubstrateResponseManager;
	type MaxTypeEntryPerBlock = MaxTypeEntryPerBlock;
	type MaxRefundPerBlock = MaxRefundPerBlock;
	type ParachainStaking = ();
	type XcmTransfer = XTokens;
	type MaxLengthLimit = MaxLengthLimit;
	type XcmWeightAndFeeHandler = XcmInterface;
	type ChannelCommission = ChannelCommission;
	type StablePoolHandler = StablePool;
	type AssetIdMaps = AssetIdMaps<Runtime>;
	type TreasuryAccount = BifrostTreasuryAccount;
}

parameter_types! {
	pub const RelayChainTokenSymbolDOT: TokenSymbol = TokenSymbol::DOT;
}

impl bifrost_vstoken_conversion::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type RelayCurrencyId = RelayCurrencyId;
	type TreasuryAccount = BifrostTreasuryAccount;
	type ControlOrigin = CoreAdminOrCouncil;
	type VsbondAccount = BifrostVsbondPalletId;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type WeightInfo = weights::bifrost_vstoken_conversion::BifrostWeight<Runtime>;
}

parameter_types! {
	pub const WhitelistMaximumLimit: u32 = 10;
}

impl bifrost_farming::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CurrencyId = CurrencyId;
	type MultiCurrency = Currencies;
	type ControlOrigin = TechAdminOrCouncil;
	type TreasuryAccount = BifrostTreasuryAccount;
	type Keeper = FarmingKeeperPalletId;
	type RewardIssuer = FarmingRewardIssuerPalletId;
	type WeightInfo = weights::bifrost_farming::BifrostWeight<Runtime>;
	type FarmingBoost = FarmingBoostPalletId;
	type VeMinting = VeMinting;
	type BlockNumberToBalance = ConvertInto;
	type WhitelistMaximumLimit = WhitelistMaximumLimit;
	type GaugeRewardIssuer = FarmingGaugeRewardIssuerPalletId;
}

parameter_types! {
	pub const BlocksPerRound: u32 = prod_or_fast!(1500, 50);
	pub const MaxTokenLen: u32 = 500;
	pub const MaxFarmingPoolIdLen: u32 = 100;
}

impl bifrost_system_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type EnsureConfirmAsGovernance = CoreAdminOrCouncil;
	type WeightInfo = weights::bifrost_system_staking::BifrostWeight<Runtime>;
	type FarmingInfo = Farming;
	type VtokenMintingInterface = VtokenMinting;
	type TreasuryAccount = BifrostTreasuryAccount;
	type PalletId = SystemStakingPalletId;
	type BlocksPerRound = BlocksPerRound;
	type MaxTokenLen = MaxTokenLen;
	type MaxFarmingPoolIdLen = MaxFarmingPoolIdLen;
}

impl bifrost_system_maker::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type WeightInfo = weights::bifrost_system_maker::BifrostWeight<Runtime>;
	type DexOperator = ZenlinkProtocol;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type TreasuryAccount = BifrostTreasuryAccount;
	type RelayChainToken = RelayCurrencyId;
	type SystemMakerPalletId = SystemMakerPalletId;
	type ParachainId = ParachainInfo;
	type VtokenMintingInterface = VtokenMinting;
}

impl bifrost_fee_share::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = CoreAdminOrCouncil;
	type WeightInfo = weights::bifrost_fee_share::BifrostWeight<Runtime>;
	type FeeSharePalletId = FeeSharePalletId;
}

impl bifrost_cross_in_out::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = TechAdminOrCouncil;
	type EntrancePalletId = SlpEntrancePalletId;
	type WeightInfo = weights::bifrost_cross_in_out::BifrostWeight<Runtime>;
	type MaxLengthLimit = MaxLengthLimit;
}

impl bifrost_slpx::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ControlOrigin = TechAdminOrCouncil;
	type MultiCurrency = Currencies;
	type DexOperator = ZenlinkProtocol;
	type VtokenMintingInterface = VtokenMinting;
	type StablePoolHandler = StablePool;
	type XcmTransfer = XTokens;
	type XcmSender = XcmRouter;
	type CurrencyIdConvert = AssetIdMaps<Runtime>;
	type TreasuryAccount = BifrostTreasuryAccount;
	type ParachainId = SelfParaChainId;
	type WeightInfo = weights::bifrost_slpx::BifrostWeight<Runtime>;
}

pub struct EnsurePoolAssetId;
impl bifrost_stable_asset::traits::ValidateAssetId<CurrencyId> for EnsurePoolAssetId {
	fn validate(_: CurrencyId) -> bool {
		true
	}
}

/// Configure the pallet bifrost_stable_asset in pallets/bifrost_stable_asset.
impl bifrost_stable_asset::Config for Runtime {
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
	type ListingOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type EnsurePoolAssetId = EnsurePoolAssetId;
}

impl bifrost_stable_pool::Config for Runtime {
	type WeightInfo = weights::bifrost_stable_pool::BifrostWeight<Runtime>;
	type ControlOrigin = TechAdminOrCouncil;
	type CurrencyId = CurrencyId;
	type MultiCurrency = Currencies;
	type StableAsset = StableAsset;
	type VtokenMinting = VtokenMinting;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type CurrencyIdRegister = AssetIdMaps<Runtime>;
}

parameter_types! {
	pub const QueryTimeout: BlockNumber = 100;
	pub const ReferendumCheckInterval: BlockNumber = 300;
}

pub struct DerivativeAccountTokenFilter;
impl Contains<CurrencyId> for DerivativeAccountTokenFilter {
	fn contains(token: &CurrencyId) -> bool {
		*token == RelayCurrencyId::get()
	}
}

impl bifrost_vtoken_voting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type MultiCurrency = Currencies;
	type ControlOrigin = CoreAdminOrCouncil;
	type ResponseOrigin = EnsureResponse<Everything>;
	type XcmDestWeightAndFee = XcmInterface;
	type DerivativeAccount = DerivativeAccountProvider<Runtime, DerivativeAccountTokenFilter>;
	type RelaychainBlockNumberProvider = RelaychainDataProvider<Runtime>;
	type VTokenSupplyProvider = VtokenMinting;
	type ParachainId = SelfParaChainId;
	type MaxVotes = ConstU32<256>;
	type QueryTimeout = QueryTimeout;
	type ReferendumCheckInterval = ReferendumCheckInterval;
	type WeightInfo = weights::bifrost_vtoken_voting::BifrostWeight<Runtime>;
}

// Bifrost modules end

// zenlink runtime start

parameter_types! {
	pub const StringLimit: u32 = 50;
}

impl merkle_distributor::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
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
}

impl zenlink_protocol::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiAssetsHandler = MultiAssets;
	type PalletId = ZenlinkPalletId;
	type SelfParaId = SelfParaId;
	type TargetChains = ();
	type WeightInfo = ();
	type AssetId = ZenlinkAssetId;
	type LpGenerate = PairLpGenerate<Self>;
}

type MultiAssets = ZenlinkMultiAssets<ZenlinkProtocol, Balances, LocalAssetAdaptor<Currencies>>;

pub struct OnRedeemSuccess;
impl bifrost_vtoken_minting::OnRedeemSuccess<AccountId, CurrencyId, Balance> for OnRedeemSuccess {
	fn on_redeem_success(token_id: CurrencyId, to: AccountId, token_amount: Balance) -> Weight {
		SystemStaking::on_redeem_success(token_id, to, token_amount)
	}

	fn on_redeemed(
		address: AccountId,
		token_id: CurrencyId,
		token_amount: Balance,
		vtoken_amount: Balance,
		fee: Balance,
	) -> Weight {
		SystemStaking::on_redeemed(address, token_id, token_amount, vtoken_amount, fee)
	}
}

parameter_types! {
	pub const MaximumUnlockIdOfUser: u32 = 10;
	pub const MaximumUnlockIdOfTimeUnit: u32 = 50;
	pub BifrostFeeAccount: AccountId = TreasuryPalletId::get().into_account_truncating();
}

impl bifrost_vtoken_minting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = TechAdminOrCouncil;
	type MaximumUnlockIdOfUser = MaximumUnlockIdOfUser;
	type MaximumUnlockIdOfTimeUnit = MaximumUnlockIdOfTimeUnit;
	type EntranceAccount = SlpEntrancePalletId;
	type ExitAccount = SlpExitPalletId;
	type FeeAccount = BifrostFeeAccount;
	type RedeemFeeAccount = BifrostFeeAccount;
	type BifrostSlp = Slp;
	type BifrostSlpx = Slpx;
	type WeightInfo = weights::bifrost_vtoken_minting::BifrostWeight<Runtime>;
	type OnRedeemSuccess = OnRedeemSuccess;
	type RelayChainToken = RelayCurrencyId;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type CurrencyIdRegister = AssetIdMaps<Runtime>;
	type XcmTransfer = XTokens;
	type AstarParachainId = ConstU32<2006>;
	type MoonbeamParachainId = ConstU32<2004>;
	type HydradxParachainId = ConstU32<2034>;
	type MantaParachainId = ConstU32<2104>;
	type InterlayParachainId = ConstU32<2032>;
	type ChannelCommission = ChannelCommission;
	type MaxLockRecords = ConstU32<100>;
	type IncentivePoolAccount = IncentivePoolAccount;
	type VeMinting = VeMinting;
	type AssetIdMaps = AssetIdMaps<Runtime>;
}

parameter_types! {
	pub const VeMintingTokenType: CurrencyId = CurrencyId::VToken(TokenSymbol::BNC);
	pub const Week: BlockNumber = prod_or_fast!(WEEKS, 10);
	pub const MaxBlock: BlockNumber = 4 * 365 * DAYS;
	pub const Multiplier: Balance = 10_u128.pow(12);
	pub const VoteWeightMultiplier: Balance = 1;
	pub const MaxPositions: u32 = 10;
	pub const MarkupRefreshLimit: u32 = 100;
}

impl bifrost_ve_minting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = TechAdminOrCouncil;
	type TokenType = VeMintingTokenType;
	type VeMintingPalletId = VeMintingPalletId;
	type IncentivePalletId = IncentivePalletId;
	type WeightInfo = weights::bifrost_ve_minting::BifrostWeight<Runtime>;
	type BlockNumberToBalance = ConvertInto;
	type Week = Week;
	type MaxBlock = MaxBlock;
	type Multiplier = Multiplier;
	type VoteWeightMultiplier = VoteWeightMultiplier;
	type MaxPositions = MaxPositions;
	type MarkupRefreshLimit = MarkupRefreshLimit;
}

parameter_types! {
	pub const MinimumCount: u32 = 3;
	pub const ExpiresIn: Moment = 1000 * 60 * 60; // 60 mins
	pub const MaxHasDispatchedSize: u32 = 100;
	pub OracleRootOperatorAccountId: AccountId = OraclePalletId::get().into_account_truncating();
}

type BifrostDataProvider = orml_oracle::Instance1;
impl orml_oracle::Config<BifrostDataProvider> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnNewData = ();
	type CombineData =
		orml_oracle::DefaultCombineData<Runtime, MinimumCount, ExpiresIn, BifrostDataProvider>;
	type Time = Timestamp;
	type OracleKey = CurrencyId;
	type OracleValue = Price;
	type RootOperatorAccountId = OracleRootOperatorAccountId;
	type MaxHasDispatchedSize = MaxHasDispatchedSize;
	type WeightInfo = weights::orml_oracle::WeightInfo<Runtime>;
	type Members = OracleMembership;
	type MaxFeedValues = ConstU32<100>;
}

pub type TimeStampedPrice = orml_oracle::TimestampedValue<Price, Moment>;
pub struct AggregatedDataProvider;
impl DataProvider<CurrencyId, TimeStampedPrice> for AggregatedDataProvider {
	fn get(key: &CurrencyId) -> Option<TimeStampedPrice> {
		Oracle::get(key)
	}
}

impl DataProviderExtended<CurrencyId, TimeStampedPrice> for AggregatedDataProvider {
	fn get_no_op(key: &CurrencyId) -> Option<TimeStampedPrice> {
		Oracle::get_no_op(key)
	}

	fn get_all_values() -> Vec<(CurrencyId, Option<TimeStampedPrice>)> {
		Oracle::get_all_values()
	}
}

impl DataFeeder<CurrencyId, TimeStampedPrice, AccountId> for AggregatedDataProvider {
	fn feed_value(_: Option<AccountId>, _: CurrencyId, _: TimeStampedPrice) -> DispatchResult {
		Err("Not supported".into())
	}
}

impl pallet_prices::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Source = AggregatedDataProvider;
	type FeederOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type UpdateOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type RelayCurrency = RelayCurrencyId;
	type CurrencyIdConvert = AssetIdMaps<Runtime>;
	type Assets = Currencies;
	type WeightInfo = pallet_prices::weights::SubstrateWeight<Runtime>;
}

impl lend_market::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = LendMarketPalletId;
	type PriceFeeder = Prices;
	type ReserveOrigin = TechAdminOrCouncil;
	type UpdateOrigin = TechAdminOrCouncil;
	type WeightInfo = lend_market::weights::BifrostWeight<Runtime>;
	type UnixTime = Timestamp;
	type Assets = Currencies;
	type RewardAssetId = NativeCurrencyId;
	type LiquidationFreeAssetId = RelayCurrencyId;
}

parameter_types! {
	pub const OracleMaxMembers: u32 = 100;
}

impl pallet_membership::Config<pallet_membership::Instance3> for Runtime {
	type AddOrigin = CoreAdminOrCouncil;
	type RuntimeEvent = RuntimeEvent;
	type MaxMembers = OracleMaxMembers;
	type MembershipInitialized = ();
	type MembershipChanged = ();
	type PrimeOrigin = CoreAdminOrCouncil;
	type RemoveOrigin = CoreAdminOrCouncil;
	type ResetOrigin = CoreAdminOrCouncil;
	type SwapOrigin = CoreAdminOrCouncil;
	type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

impl leverage_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = leverage_staking::weights::SubstrateWeight<Runtime>;
	type ControlOrigin = EnsureRoot<AccountId>;
	type VtokenMinting = VtokenMinting;
	type LendMarket = LendMarket;
	type StablePoolHandler = StablePool;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
}

parameter_types! {
	pub const ClearingDuration: u32 = prod_or_fast!(1 * DAYS, 10 * MINUTES);
	pub const NameLengthLimit: u32 = 20;
	pub BifrostCommissionReceiver: AccountId = TreasuryPalletId::get().into_account_truncating();
}

impl bifrost_channel_commission::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = EitherOfDiverse<CoreAdminOrCouncil, LiquidStaking>;
	type CommissionPalletId = CommissionPalletId;
	type BifrostCommissionReceiver = BifrostCommissionReceiver;
	type WeightInfo = weights::bifrost_channel_commission::BifrostWeight<Runtime>;
	type ClearingDuration = ClearingDuration;
	type NameLengthLimit = NameLengthLimit;
}

impl bifrost_clouds_convert::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type CloudsPalletId = CloudsPalletId;
	type VeMinting = VeMinting;
	type WeightInfo = weights::bifrost_clouds_convert::BifrostWeight<Runtime>;
	type LockedBlocks = MaxBlock;
}

impl bifrost_buy_back::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type WeightInfo = weights::bifrost_buy_back::BifrostWeight<Runtime>;
	type DexOperator = ZenlinkProtocol;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type TreasuryAccount = BifrostTreasuryAccount;
	type RelayChainToken = RelayCurrencyId;
	type BuyBackAccount = BuyBackAccount;
	type LiquidityAccount = LiquidityAccount;
	type ParachainId = ParachainInfo;
	type CurrencyIdRegister = AssetIdMaps<Runtime>;
	type VeMinting = VeMinting;
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
	pub enum Runtime {
		// Basic stuff
		System: frame_system = 0,
		Timestamp: pallet_timestamp = 1,
		Indices: pallet_indices = 2,
		ParachainSystem: cumulus_pallet_parachain_system = 5,
		ParachainInfo: parachain_info = 6,

		// Monetary stuff
		Balances: pallet_balances = 10,
		TransactionPayment: pallet_transaction_payment = 11,

		// Collator support. the order of these 4 are important and shall not change.
		Authorship: pallet_authorship = 20,
		CollatorSelection: pallet_collator_selection = 21,
		Session: pallet_session = 22,
		Aura: pallet_aura = 23,
		AuraExt: cumulus_pallet_aura_ext = 24,

		// Governance stuff
		Democracy: pallet_democracy = 30,
		Council: pallet_collective::<Instance1> = 31,
		TechnicalCommittee: pallet_collective::<Instance2> = 32,
		PhragmenElection: pallet_elections_phragmen = 33,
		CouncilMembership: pallet_membership::<Instance1> = 34,
		TechnicalMembership: pallet_membership::<Instance2> = 35,
		ConvictionVoting: pallet_conviction_voting = 36,
		Referenda: pallet_referenda = 37,
		Origins: custom_origins = 38,
		Whitelist: pallet_whitelist = 39,

		// XCM helpers.
		XcmpQueue: cumulus_pallet_xcmp_queue = 40,
		PolkadotXcm: pallet_xcm = 41,
		CumulusXcm: cumulus_pallet_xcm = 42,
		MessageQueue: pallet_message_queue = 44,

		// utilities
		Utility: pallet_utility = 50,
		Scheduler: pallet_scheduler = 51,
		Proxy: pallet_proxy = 52,
		Multisig: pallet_multisig = 53,
		Identity: pallet_identity = 54,

		// Vesting. Usable initially, but removed once all vesting is finished.
		Vesting: bifrost_vesting = 60,

		// Treasury stuff
		Treasury: pallet_treasury = 61,
		Preimage: pallet_preimage = 64,

		// Third party modules
		XTokens: orml_xtokens = 70,
		Tokens: orml_tokens = 71,
		Currencies: bifrost_currencies = 72,
		UnknownTokens: orml_unknown_tokens = 73,
		OrmlXcm: orml_xcm = 74,
		ZenlinkProtocol: zenlink_protocol = 80,
		MerkleDistributor: merkle_distributor = 81,

		// Bifrost modules
		FlexibleFee: bifrost_flexible_fee = 100,
		Salp: bifrost_salp = 105,
		CallSwitchgear: bifrost_call_switchgear = 112,
		AssetRegistry: bifrost_asset_registry = 114,
		VtokenMinting: bifrost_vtoken_minting = 115,
		Slp: bifrost_slp = 116,
		XcmInterface: bifrost_xcm_interface = 117,
		TokenConversion: bifrost_vstoken_conversion = 118,
		Farming: bifrost_farming = 119,
		SystemStaking: bifrost_system_staking = 120,
		SystemMaker: bifrost_system_maker = 121,
		FeeShare: bifrost_fee_share = 122,
		CrossInOut: bifrost_cross_in_out = 123,
		VeMinting: bifrost_ve_minting = 124,
		Slpx: bifrost_slpx = 125,
		FellowshipCollective: pallet_ranked_collective::<Instance1> = 126,
		FellowshipReferenda: pallet_referenda::<Instance2> = 127,
		StableAsset: bifrost_stable_asset exclude_parts { Call } = 128,
		StablePool: bifrost_stable_pool = 129,
		VtokenVoting: bifrost_vtoken_voting = 130,
		LendMarket: lend_market = 131,
		Prices: pallet_prices = 132,
		Oracle: orml_oracle::<Instance1> = 133,
		OracleMembership: pallet_membership::<Instance3> = 134,
		LeverageStaking: leverage_staking = 135,
		ChannelCommission: bifrost_channel_commission = 136,
		CloudsConvert: bifrost_clouds_convert = 137,
		BuyBack: bifrost_buy_back = 138,
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
pub type UncheckedExtrinsic =
	generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<RuntimeCall, SignedExtra>;

parameter_types! {
	pub const DmpQueuePalletName: &'static str = "DmpQueue";
}

/// All migrations that will run on the next runtime upgrade.
///
/// This contains the combined migrations of the last 10 releases. It allows to skip runtime
/// upgrades in case governance decides to do so. THE ORDER IS IMPORTANT.
pub type Migrations = migrations::Unreleased;

/// The runtime migrations per release.
pub mod migrations {
	#[allow(unused_imports)]
	use super::*;

	/// Unreleased migrations. Add new ones here:
	pub type Unreleased = ();
}

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
	Migrations,
>;

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
	define_benchmarks!(
		[bifrost_ve_minting, VeMinting]
		[bifrost_buy_back, BuyBack]
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
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}
		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}
		fn metadata_versions() -> sp_std::vec::Vec<u32> {
			Runtime::metadata_versions()
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
		fn get_fee_token_and_amount(who: AccountId, fee: Balance, utx: <Block as BlockT>::Extrinsic) -> (CurrencyId, Balance) {
			let call = utx.function;
			let rs = FlexibleFee::cal_fee_token_and_amount(&who, fee, &call);

			match rs {
				Ok(val) => val,
				_ => (CurrencyId::Native(TokenSymbol::BNC), Zero::zero()),
			}
		}
	}

	// zenlink runtime outer apis
	impl zenlink_protocol_runtime_api::ZenlinkProtocolApi<Block, AccountId, ZenlinkAssetId> for Runtime {

		fn get_balance(
			asset_id: ZenlinkAssetId,
			owner: AccountId
		) -> AssetBalance {
			<Runtime as zenlink_protocol::Config>::MultiAssetsHandler::balance_of(asset_id, &owner)
		}

		fn get_pair_by_asset_id(
			asset_0: ZenlinkAssetId,
			asset_1: ZenlinkAssetId
		) -> Option<PairInfo<AccountId, AssetBalance, ZenlinkAssetId>> {
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
		fn calculate_remove_liquidity(
			asset_0: ZenlinkAssetId,
			asset_1: ZenlinkAssetId,
			amount: AssetBalance,
		) -> Option<(AssetBalance, AssetBalance)>{
			ZenlinkProtocol::calculate_remove_liquidity(
				asset_0,
				asset_1,
				amount,
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

	impl bifrost_farming_rpc_runtime_api::FarmingRuntimeApi<Block, AccountId, PoolId, CurrencyId> for Runtime {
		fn get_farming_rewards(who: AccountId, pid: PoolId) -> Vec<(CurrencyId, Balance)> {
			Farming::get_farming_rewards(&who, pid).unwrap_or(Vec::new())
		}

		fn get_gauge_rewards(who: AccountId, pid: PoolId) -> Vec<(CurrencyId, Balance)> {
			Farming::get_gauge_rewards(&who, pid).unwrap_or(Vec::new())
		}
	}

	impl bifrost_ve_minting_rpc_runtime_api::VeMintingRuntimeApi<Block, AccountId> for Runtime {
		fn balance_of(
			who: AccountId,
			t: Option<bifrost_primitives::BlockNumber>,
		) -> Balance{
			VeMinting::balance_of(&who, t).unwrap_or(Zero::zero())
		}

		fn total_supply(
			t: bifrost_primitives::BlockNumber,
		) -> Balance{
			VeMinting::total_supply(t).unwrap_or(Zero::zero())
		}

		fn find_block_epoch(
			block: bifrost_primitives::BlockNumber,
			max_epoch: U256,
		) -> U256{
			VeMinting::find_block_epoch(block, max_epoch)
		}
	}

	impl lend_market_rpc_runtime_api::LendMarketApi<Block, AccountId, Balance> for Runtime {
		fn get_account_liquidity(account: AccountId) -> Result<(Liquidity, Shortfall, Liquidity, Shortfall), DispatchError> {
			LendMarket::get_account_liquidity(&account)
		}

		fn get_market_status(asset_id: CurrencyId) -> Result<(Rate, Rate, Rate, Ratio, Balance, Balance, sp_runtime::FixedU128), DispatchError> {
			LendMarket::get_market_status(asset_id)
		}

		fn get_liquidation_threshold_liquidity(account: AccountId) -> Result<(Liquidity, Shortfall, Liquidity, Shortfall), DispatchError> {
			LendMarket::get_account_liquidation_threshold_liquidity(&account)
		}
	}

	impl bifrost_stable_pool_rpc_runtime_api::StablePoolRuntimeApi<Block> for Runtime {
		fn get_swap_output(
			pool_id: u32,
			currency_id_in: u32,
			currency_id_out: u32,
			amount: Balance,
		) -> Balance {
			StablePool::get_swap_output(pool_id, currency_id_in, currency_id_out, amount).unwrap_or(Zero::zero())
		}

		fn add_liquidity_amount(
			pool_id: u32,
			amounts: Vec<Balance>,
		) -> Balance {
			StablePool::add_liquidity_amount(pool_id, amounts).unwrap_or(Zero::zero())
		}
	}

	impl bifrost_vtoken_minting_rpc_runtime_api::VtokenMintingRuntimeApi<Block, CurrencyId> for Runtime {
		fn get_exchange_rate(token_id: Option<CurrencyId>) -> Vec<(CurrencyId, U256)> {
			VtokenMinting::get_exchange_rate(token_id).unwrap_or(Vec::new())
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
			use frame_benchmarking::{Benchmarking, BenchmarkBatch};
			use frame_support::traits::TrackedStorageKey;

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
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			log::info!("try-runtime::on_runtime_upgrade bifrost.");
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}
		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check,signature_check, select).unwrap()
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn create_default_config() -> Vec<u8> {
			create_default_config::<RuntimeGenesisConfig>()
		}

		fn build_config(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_config::<RuntimeGenesisConfig>(config)
		}
	}
}

struct CheckInherents;
#[allow(deprecated)]
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

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
pub use bifrost_parachain_staking::{InflationInfo, Range};
pub use frame_support::{
	construct_runtime, match_types, parameter_types,
	traits::{
		ConstU128, ConstU32, ConstU64, ConstU8, Contains, EqualPrivilegeOnly, Everything,
		InstanceFilter, IsInVec, Nothing, Randomness, WithdrawReasons,
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
pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;
use sp_api::impl_runtime_apis;
use sp_arithmetic::Percent;
use sp_core::{ConstBool, OpaqueMetadata};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{
		AccountIdConversion, AccountIdLookup, BlakeTwo256, Block as BlockT, StaticLookup, Zero,
	},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, DispatchError, DispatchResult, Perbill, Permill, RuntimeDebug,
	SaturatedConversion,
};
use sp_std::{marker::PhantomData, prelude::*};
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;
/// Constant values used within the runtime.
pub mod constants;
mod migration;
pub mod weights;
use bifrost_asset_registry::AssetIdMaps;

pub use bifrost_primitives::{
	traits::{
		CheckSubAccount, FarmingInfo, FeeGetter, VtokenMintingInterface, VtokenMintingOperator,
		XcmDestWeightAndFeeHandler,
	},
	AccountId, Amount, AssetIds, Balance, BlockNumber, CurrencyId, CurrencyIdMapping,
	DistributionId, ExtraFeeInfo, ExtraFeeName, Liquidity, Moment, ParaId, PoolId, Price, Rate,
	Ratio, RpcContributionStatus, Shortfall, TimeUnit, TokenSymbol,
};
pub use bifrost_runtime_common::{
	cent, constants::time::*, dollar, micro, milli, millicent, AuraId, CouncilCollective,
	EnsureRootOrAllTechnicalCommittee, MoreThanHalfCouncil, SlowAdjustingFeeUpdate,
	TechnicalCollective,
};
use bifrost_slp::QueryId;
use constants::currency::*;
use cumulus_pallet_parachain_system::{RelayNumberStrictlyIncreases, RelaychainDataProvider};
use frame_support::{
	dispatch::DispatchClass,
	sp_runtime::traits::{Convert, ConvertInto},
	traits::{
		fungible::HoldConsideration,
		tokens::{PayFromAccount, UnityAssetBalanceConversion},
		Currency, EitherOf, EitherOfDiverse, Get, Imbalance, LinearStoragePrice, LockIdentifier,
		OnUnbalanced,
	},
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess, EnsureSigned};
use hex_literal::hex;
use orml_oracle::{DataFeeder, DataProvider, DataProviderExtended};
use pallet_identity::simple::IdentityInfo;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use polkadot_runtime_common::prod_or_fast;

// zenlink imports
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler, MultiAssetsHandler, PairInfo,
	PairLpGenerate, ZenlinkMultiAssets,
};
use zenlink_stable_amm::traits::{StableAmmApi, StablePoolLpCurrencyIdGenerate, ValidateCurrency};

// Governance configurations.
pub mod governance;
use governance::{
	custom_origins, CoreAdmin, CoreAdminOrCouncil, LiquidStaking, SALPAdmin, Spender, TechAdmin,
	TechAdminOrCouncil,
};

// xcm config
pub mod xcm_config;
use pallet_xcm::{EnsureResponse, QueryStatus};
use sp_runtime::traits::IdentityLookup;
use xcm::v3::prelude::*;
pub use xcm_config::{
	parachains, AccountId32Aliases, BifrostCurrencyIdConvert, BifrostTreasuryAccount,
	ExistentialDeposits, MultiCurrency, SelfParaChainId, Sibling, SiblingParachainConvertsVia,
	XcmConfig, XcmRouter,
};
use xcm_executor::{traits::QueryHandler, XcmExecutor};

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
	spec_version: 994,
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

pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
	fn contains(_c: &RuntimeCall) -> bool {
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
	pub const BifrostSalpLiteCrowdloanId: PalletId = PalletId(*b"bf/salpl");
	pub const LiquidityMiningPalletId: PalletId = PalletId(*b"bf/lm###");
	pub const LiquidityMiningDOTPalletId: PalletId = PalletId(*b"bf/lmdot");
	pub const LighteningRedeemPalletId: PalletId = PalletId(*b"bf/ltnrd");
	pub const MerkleDirtributorPalletId: PalletId = PalletId(*b"bf/mklds");
	pub const VsbondAuctionPalletId: PalletId = PalletId(*b"bf/vsbnd");
	pub const ParachainStakingPalletId: PalletId = PalletId(*b"bf/stake");
	pub const BifrostVsbondPalletId: PalletId = PalletId(*b"bf/salpb");
	pub const SlpEntrancePalletId: PalletId = PalletId(*b"bf/vtkin");
	pub const SlpExitPalletId: PalletId = PalletId(*b"bf/vtout");
	pub const StableAmmPalletId: PalletId = PalletId(*b"bf/stamm");
	pub const FarmingKeeperPalletId: PalletId = PalletId(*b"bf/fmkpr");
	pub const FarmingRewardIssuerPalletId: PalletId = PalletId(*b"bf/fmrir");
	pub const SystemStakingPalletId: PalletId = PalletId(*b"bf/sysst");
	pub const BuybackPalletId: PalletId = PalletId(*b"bf/salpc");
	pub const SystemMakerPalletId: PalletId = PalletId(*b"bf/sysmk");
	pub const FeeSharePalletId: PalletId = PalletId(*b"bf/feesh");
	pub CheckingAccount: AccountId = PolkadotXcm::check_account();
	pub const FarmingBoostPalletId: PalletId = PalletId(*b"bf/fmbst");
	pub const LendMarketPalletId: PalletId = PalletId(*b"bf/ldmkt");
	pub const OraclePalletId: PalletId = PalletId(*b"bf/oracl");
	pub const StableAssetPalletId: PalletId = PalletId(*b"bf/stabl");
	pub const CommissionPalletId: PalletId = PalletId(*b"bf/comms");
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
	type Block = Block;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	type DbWeight = RocksDbWeight;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
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
}

impl pallet_timestamp::Config for Runtime {
	type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = Aura;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub ExistentialDeposit: Balance = 10 * MILLIBNC;
	pub TransferFee: Balance = 1 * MILLIBNC;
	pub CreationFee: Balance = 1 * MILLIBNC;
	pub TransactionByteFee: Balance = 16 * MICROBNC;
}

impl pallet_utility::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub ProxyDepositBase: Balance = deposit::<Runtime>(1, 8);
	// Additional storage item size of 33 bytes.
	pub ProxyDepositFactor: Balance = deposit::<Runtime>(0, 33);
	pub const MaxProxies: u16 = 32;
	pub AnnouncementDepositBase: Balance = deposit::<Runtime>(1, 8);
	pub AnnouncementDepositFactor: Balance = deposit::<Runtime>(0, 66);
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
	Staking = 5,
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
				RuntimeCall::Bounties(..) |
				RuntimeCall::Tips(..) |
				RuntimeCall::ConvictionVoting(..) |
				RuntimeCall::Referenda(..) |
				RuntimeCall::FellowshipCollective(..) |
				RuntimeCall::FellowshipReferenda(..) |
				RuntimeCall::Whitelist(..) |
				RuntimeCall::Vesting(bifrost_vesting::Call::vest{..}) |
				RuntimeCall::Vesting(bifrost_vesting::Call::vest_other{..}) |
				// Specifically omitting Vesting `vested_transfer`, and `force_vested_transfer`
				RuntimeCall::Utility(..) |
				RuntimeCall::Proxy(..) |
				RuntimeCall::Multisig(..) |
				RuntimeCall::ParachainStaking(..)
			),
			ProxyType::Staking => {
				matches!(c, RuntimeCall::ParachainStaking(..) | RuntimeCall::Utility(..))
			},
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Democracy(..) |
						RuntimeCall::Council(..) | RuntimeCall::TechnicalCommittee(..) |
						RuntimeCall::PhragmenElection(..) |
						RuntimeCall::Treasury(..) |
						RuntimeCall::Bounties(..) |
						RuntimeCall::Tips(..) | RuntimeCall::Utility(..) |
						// OpenGov calls
						RuntimeCall::ConvictionVoting(..) |
						RuntimeCall::Referenda(..) |
						RuntimeCall::FellowshipCollective(..) |
						RuntimeCall::FellowshipReferenda(..) |
						RuntimeCall::Whitelist(..)
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
	pub PreimageBaseDeposit: Balance = deposit::<Runtime>(2, 64);
	pub PreimageByteDeposit: Balance = deposit::<Runtime>(0, 1);
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
	pub DepositBase: Balance = deposit::<Runtime>(1, 88);
	// Additional storage item size of 32 bytes.
	pub DepositFactor: Balance = deposit::<Runtime>(0, 32);
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
	// Minimum 4 CENTS/byte
	pub BasicDeposit: Balance = deposit::<Runtime>(1, 258);
	pub FieldDeposit: Balance = deposit::<Runtime>(0, 66);
	pub SubAccountDeposit: Balance = deposit::<Runtime>(1, 53);
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type FieldDeposit = FieldDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
	type MaxAdditionalFields = MaxAdditionalFields;
	type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = MoreThanHalfCouncil;
	type RegistrarOrigin = MoreThanHalfCouncil;
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub IndexDeposit: Balance = 1 * BNCS;
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
	type MaxHolds = ConstU32<1>;
	type MaxFreezes = ConstU32<0>;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 2 * DAYS;
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
	pub const TechnicalMotionDuration: BlockNumber = 2 * DAYS;
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
	pub CandidacyBond: Balance = 10_000 * BNCS;
	// 1 storage item created, key size is 32 bytes, value size is 16+16.
	pub VotingBondBase: Balance = deposit::<Runtime>(1, 64);
	// additional data per vote is 32 bytes (account id).
	pub VotingBondFactor: Balance = deposit::<Runtime>(0, 32);
	/// Daily council elections
	pub const TermDuration: BlockNumber = 24 * HOURS;
	pub const DesiredMembers: u32 = 3;
	pub const DesiredRunnersUp: u32 = 7;
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
	pub const LaunchPeriod: BlockNumber = 7 * DAYS;
	pub const VotingPeriod: BlockNumber = 7 * DAYS;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
	pub MinimumDeposit: Balance = 100 * BNCS;
	pub const EnactmentPeriod: BlockNumber = 2 * DAYS;
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
	pub ProposalBondMinimum: Balance = 100 * BNCS;
	pub ProposalBondMaximum: Balance = 500 * BNCS;
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const Burn: Permill = Permill::from_perthousand(0);
	pub const TipCountdown: BlockNumber = 1 * DAYS;
	pub const TipFindersFee: Percent = Percent::from_percent(20);
	pub TipReportDepositBase: Balance = 1 * BNCS;
	pub DataDepositPerByte: Balance = 10 * cent::<Runtime>(NativeCurrencyId::get());
	pub BountyDepositBase: Balance = 1 * BNCS;
	pub const BountyDepositPayoutDelay: BlockNumber = 4 * DAYS;
	pub const BountyUpdatePeriod: BlockNumber = 90 * DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;
	pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
	pub BountyValueMinimum: Balance = 10 * BNCS;
	pub const MaxApprovals: u32 = 100;

	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub CuratorDepositMin: Balance = 1 * BNCS;
	pub CuratorDepositMax: Balance = 100 * BNCS;
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
	type SpendFunds = Bounties;
	type SpendPeriod = SpendPeriod;
	type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
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
	type RuntimeEvent = RuntimeEvent;
	type MaximumReasonLength = MaximumReasonLength;
	type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
	type ChildBountyManager = ();
}

impl pallet_tips::Config for Runtime {
	type DataDepositPerByte = DataDepositPerByte;
	type RuntimeEvent = RuntimeEvent;
	type MaximumReasonLength = MaximumReasonLength;
	type TipCountdown = TipCountdown;
	type TipFindersFee = TipFindersFee;
	type TipReportDepositBase = TipReportDepositBase;
	type Tippers = PhragmenElection;
	type MaxTipAmount = ();
	type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

impl pallet_transaction_payment::Config for Runtime {
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type OnChargeTransaction = FlexibleFee;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightToFee = WeightToFee;
	type RuntimeEvent = RuntimeEvent;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	RuntimeCall: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: RuntimeCall,
		public: <Signature as sp_runtime::traits::Verify>::Signer,
		account: AccountId,
		nonce: Nonce,
	) -> Option<(
		RuntimeCall,
		<UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload,
	)> {
		// take the biggest period possible.
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let tip = 0;
		let extra: SignedExtra = (
			frame_system::CheckNonZeroSender::<Runtime>::new(),
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = AccountIdLookup::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature, extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	RuntimeCall: From<C>,
{
	type OverarchingCall = RuntimeCall;
	type Extrinsic = UncheckedExtrinsic;
}

// culumus runtime start
parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type DmpMessageHandler = DmpQueue;
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type OutboundXcmpMessageSource = XcmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type XcmpMessageHandler = XcmpQueue;
	type CheckAssociatedRelayNumber = RelayNumberStrictlyIncreases;
	type ConsensusHook = cumulus_pallet_parachain_system::ExpectParentIncluded;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	/// Minimum round length is 2 minutes (10 * 12 second block times)
	pub const MinBlocksPerRound: u32 = 10;
	/// Blocks per round
	pub const DefaultBlocksPerRound: u32 = prod_or_fast!(2 * HOURS, 10);
	/// Rounds before the collator leaving the candidates request can be executed
	pub const LeaveCandidatesDelay: u32 = 84;
	/// Rounds before the candidate bond increase/decrease can be executed
	pub const CandidateBondLessDelay: u32 = 84;
	/// Rounds before the delegator exit can be executed
	pub const LeaveDelegatorsDelay: u32 = 84;
	/// Rounds before the delegator revocation can be executed
	pub const RevokeDelegationDelay: u32 = 84;
	/// Rounds before the delegator bond increase/decrease can be executed
	pub const DelegationBondLessDelay: u32 = 84;
	/// Rounds before the reward is paid
	pub const RewardPaymentDelay: u32 = 2;
	/// Minimum collators selected per round, default at genesis and minimum forever after
	pub const MinSelectedCandidates: u32 = prod_or_fast!(16,6);
	/// Maximum top delegations per candidate
	pub const MaxTopDelegationsPerCandidate: u32 = 300;
	/// Maximum bottom delegations per candidate
	pub const MaxBottomDelegationsPerCandidate: u32 = 50;
	/// Maximum delegations per delegator
	pub const MaxDelegationsPerDelegator: u32 = 100;
	/// Default fixed percent a collator takes off the top of due rewards
	pub const DefaultCollatorCommission: Perbill = Perbill::from_percent(10);
	/// Default percent of inflation set aside for parachain bond every round
	pub const DefaultParachainBondReservePercent: Percent = Percent::from_percent(0);
	/// Minimum stake required to become a collator
	pub MinCollatorStk: u128 = 5000 * BNCS;
	/// Minimum stake required to be reserved to be a candidate
	pub MinCandidateStk: u128 = 5000 * BNCS;
	/// Minimum stake required to be reserved to be a delegator
	pub MinDelegatorStk: u128 = 50 * BNCS;
	pub AllowInflation: bool = false;
	pub ToMigrateInvulnables: Vec<AccountId> = prod_or_fast!(vec![
		hex!["8cf80f0bafcd0a3d80ca61cb688e4400e275b39d3411b4299b47e712e9dab809"].into(),
		hex!["40ac4effe39181731a8feb8a8ee0780e177bdd0d752b09c8fd71047e67189022"].into(),
		hex!["624d6a004c72a1abcf93131e185515ebe1410e43a301fe1f25d20d8da345376e"].into(),
		hex!["985d2738e512909c81289e6055e60a6824818964535ecfbf10e4d69017084756"].into(),
	],vec![
		hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"].into(),
		hex!["8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"].into(),
	]);
	pub PaymentInRound: u128 = 180 * BNCS;
	pub InitSeedStk: u128 = 5000 * BNCS;
}
impl bifrost_parachain_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MonetaryGovernanceOrigin =
		EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type MinBlocksPerRound = MinBlocksPerRound;
	type DefaultBlocksPerRound = DefaultBlocksPerRound;
	type LeaveCandidatesDelay = LeaveCandidatesDelay;
	type CandidateBondLessDelay = CandidateBondLessDelay;
	type LeaveDelegatorsDelay = LeaveDelegatorsDelay;
	type RevokeDelegationDelay = RevokeDelegationDelay;
	type DelegationBondLessDelay = DelegationBondLessDelay;
	type RewardPaymentDelay = RewardPaymentDelay;
	type MinSelectedCandidates = MinSelectedCandidates;
	type MaxTopDelegationsPerCandidate = MaxTopDelegationsPerCandidate;
	type MaxBottomDelegationsPerCandidate = MaxBottomDelegationsPerCandidate;
	type MaxDelegationsPerDelegator = MaxDelegationsPerDelegator;
	type DefaultCollatorCommission = DefaultCollatorCommission;
	type DefaultParachainBondReservePercent = DefaultParachainBondReservePercent;
	type MinCollatorStk = MinCollatorStk;
	type MinCandidateStk = MinCandidateStk;
	type MinDelegation = MinDelegatorStk;
	type MinDelegatorStk = MinDelegatorStk;
	type AllowInflation = AllowInflation;
	type PaymentInRound = PaymentInRound;
	type ToMigrateInvulnables = ToMigrateInvulnables;
	type PalletId = ParachainStakingPalletId;
	type InitSeedStk = InitSeedStk;
	type OnCollatorPayout = ();
	type OnNewRound = ();
	type WeightInfo = bifrost_parachain_staking::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Keys = SessionKeys;
	type NextSessionRotation = ParachainStaking;
	// Essentially just Aura, but lets be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type SessionManager = ParachainStaking;
	type ShouldEndSession = ParachainStaking;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = ConvertInto;
	type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type EventHandler = ParachainStaking;
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
	type AllowMultipleBlocksPerSlot = ConstBool<false>;
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
		// AccountKey20 format of Bifrost sibling para account
		CurrencyId::Token(TokenSymbol::MOVR) => MultiLocation::new(
			1,
			X2(
				Parachain(parachains::moonriver::ID.into()),
				AccountKey20 {
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
		CurrencyId::Token(TokenSymbol::KSM) => MultiLocation::new(
			1,
			X1(AccountId32 {
				network: None,
				id: Utility::derivative_account_id(
					ParachainInfo::get().into_account_truncating(),
					index,
				)
				.into(),
			}),
		),
		// Bifrost Kusama Native token
		CurrencyId::Native(TokenSymbol::BNC) => MultiLocation::new(
			0,
			X1(AccountId32 {
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
					MultiLocation::new(
						1,
						X2(
							Parachain(*para_id),
							AccountId32 {
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
					MultiLocation::default()
				}
			} else {
				MultiLocation::default()
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
	pub MinContribution: Balance = dollar::<Runtime>(RelayCurrencyId::get()) / 10;
	pub const RemoveKeysLimit: u32 = 500;
	pub const VSBondValidPeriod: BlockNumber = 30 * DAYS;
	pub const ReleaseCycle: BlockNumber = 1 * DAYS;
	pub const LeasePeriod: BlockNumber = KUSAMA_LEASE_PERIOD;
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

parameter_types! {
	pub const MaximumOrderInTrade: u32 = 1_000;
	pub const MinimumSupply: Balance = 0;
}

impl bifrost_vsbond_auction::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type InvoicingCurrency = RelayCurrencyId;
	type MaximumOrderInTrade = MaximumOrderInTrade;
	type MinimumAmount = MinimumSupply;
	type MultiCurrency = Currencies;
	type WeightInfo = weights::bifrost_vsbond_auction::BifrostWeight<Runtime>;
	type PalletId = VsbondAuctionPalletId;
	type TreasuryAccount = BifrostTreasuryAccount;
	type ControlOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
}

impl bifrost_token_issuer::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type WeightInfo = weights::bifrost_token_issuer::BifrostWeight<Runtime>;
	type MaxLengthLimit = MaxLengthLimit;
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
impl QueryResponseManager<QueryId, MultiLocation, BlockNumber, RuntimeCall>
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
		responder: &MultiLocation,
		call_back: Option<RuntimeCall>,
		timeout: BlockNumber,
	) -> u64 {
		// for xcm v3 version see the following
		// PolkadotXcm::new_query(responder, timeout, Here)
		if let Some(call_back) = call_back {
			PolkadotXcm::new_notify_query(*responder, call_back, timeout, Here)
		} else {
			PolkadotXcm::new_query(*responder, timeout, Here)
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

pub struct OnRefund;
impl bifrost_slp::OnRefund<AccountId, CurrencyId, Balance> for OnRefund {
	fn on_refund(token_id: CurrencyId, to: AccountId, token_amount: Balance) -> u64 {
		SystemStaking::on_refund(token_id, to, token_amount).ref_time()
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
	type BifrostSlpx = Slpx;
	type AccountConverter = SubAccountIndexMultiLocationConvertor;
	type ParachainId = SelfParaChainId;
	type SubstrateResponseManager = SubstrateResponseManager;
	type MaxTypeEntryPerBlock = MaxTypeEntryPerBlock;
	type MaxRefundPerBlock = MaxRefundPerBlock;
	type OnRefund = OnRefund;
	type ParachainStaking = ParachainStaking;
	type XcmTransfer = XTokens;
	type MaxLengthLimit = MaxLengthLimit;
	type XcmWeightAndFeeHandler = XcmInterface;
	type ChannelCommission = ChannelCommission;
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
	type MultiCurrency = Currencies;
	type CurrencyId = CurrencyId;
	type ControlOrigin = TechAdminOrCouncil;
	type TreasuryAccount = BifrostTreasuryAccount;
	type Keeper = FarmingKeeperPalletId;
	type RewardIssuer = FarmingRewardIssuerPalletId;
	type WeightInfo = weights::bifrost_farming::BifrostWeight<Runtime>;
	type FarmingBoost = FarmingBoostPalletId;
	type VeMinting = ();
	type BlockNumberToBalance = ConvertInto;
	type WhitelistMaximumLimit = WhitelistMaximumLimit;
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
	type ControlOrigin = EitherOfDiverse<CoreAdmin, MoreThanHalfCouncil>;
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

impl zenlink_stable_amm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type CurrencyId = CurrencyId;
	type MultiCurrency = Currencies;
	type PoolId = u32;
	type TimeProvider = Timestamp;
	type EnsurePoolAsset = StableAmmVerifyPoolAsset;
	type LpGenerate = PoolLpGenerate;
	type PoolCurrencySymbolLimit = StringLimit;
	type PalletId = StableAmmPalletId;
	type WeightInfo = ();
}

impl zenlink_swap_router::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StablePoolId = u32;
	type Balance = u128;
	type StableCurrencyId = CurrencyId;
	type NormalCurrencyId = ZenlinkAssetId;
	type NormalAmm = ZenlinkProtocol;
	type StableAMM = ZenlinkStableAMM;
	type WeightInfo = zenlink_swap_router::weights::SubstrateWeight<Runtime>;
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

pub struct StableAmmVerifyPoolAsset;

impl ValidateCurrency<CurrencyId> for StableAmmVerifyPoolAsset {
	fn validate_pooled_currency(_currencies: &[CurrencyId]) -> bool {
		true
	}

	fn validate_pool_lp_currency(_currency_id: CurrencyId) -> bool {
		if Currencies::total_issuance(_currency_id) > 0 {
			return false;
		}
		true
	}
}

pub struct PoolLpGenerate;

impl StablePoolLpCurrencyIdGenerate<CurrencyId, PoolId> for PoolLpGenerate {
	fn generate_by_pool_id(pool_id: PoolId) -> CurrencyId {
		CurrencyId::StableLpToken(pool_id)
	}
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
	type BifrostSlp = Slp;
	type BifrostSlpx = Slpx;
	type WeightInfo = weights::bifrost_vtoken_minting::BifrostWeight<Runtime>;
	type OnRedeemSuccess = OnRedeemSuccess;
	type RelayChainToken = RelayCurrencyId;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type CurrencyIdRegister = AssetIdMaps<Runtime>;
	type XcmTransfer = XTokens;
	type AstarParachainId = ConstU32<2007>;
	type MoonbeamParachainId = ConstU32<2023>;
	type HydradxParachainId = ConstU32<2034>;
	type MantaParachainId = ConstU32<2104>;
	type InterlayParachainId = ConstU32<2092>;
	type ChannelCommission = ChannelCommission;
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
	type ReserveOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
	type UpdateOrigin = EitherOfDiverse<MoreThanHalfCouncil, EnsureRootOrAllTechnicalCommittee>;
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
	type AddOrigin = MoreThanHalfCouncil;
	type RuntimeEvent = RuntimeEvent;
	type MaxMembers = OracleMaxMembers;
	type MembershipInitialized = ();
	type MembershipChanged = ();
	type PrimeOrigin = MoreThanHalfCouncil;
	type RemoveOrigin = MoreThanHalfCouncil;
	type ResetOrigin = MoreThanHalfCouncil;
	type SwapOrigin = MoreThanHalfCouncil;
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
	pub const ClearingDuration: u32 = prod_or_fast!(7 * DAYS, 10 * MINUTES);
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
		Session: pallet_session = 22,
		Aura: pallet_aura = 23,
		AuraExt: cumulus_pallet_aura_ext = 24,
		ParachainStaking: bifrost_parachain_staking = 25,

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
		DmpQueue: cumulus_pallet_dmp_queue = 43,

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
		Bounties: pallet_bounties = 62,
		Tips: pallet_tips = 63,
		Preimage: pallet_preimage = 64,

		// Third party modules
		XTokens: orml_xtokens = 70,
		Tokens: orml_tokens = 71,
		Currencies: bifrost_currencies = 72,
		UnknownTokens: orml_unknown_tokens = 73,
		OrmlXcm: orml_xcm = 74,
		ZenlinkProtocol: zenlink_protocol = 80,
		MerkleDistributor: merkle_distributor = 81,
		ZenlinkStableAMM: zenlink_stable_amm = 82,
		ZenlinkSwapRouter: zenlink_swap_router = 83,

		// Bifrost modules
		FlexibleFee: bifrost_flexible_fee = 100,
		Salp: bifrost_salp = 105,
		TokenIssuer: bifrost_token_issuer = 109,
		CallSwitchgear: bifrost_call_switchgear = 112,
		VSBondAuction: bifrost_vsbond_auction = 113,
		AssetRegistry: bifrost_asset_registry = 114,
		VtokenMinting: bifrost_vtoken_minting = 115,
		Slp: bifrost_slp = 116,
		XcmInterface: bifrost_xcm_interface = 117,
		VstokenConversion: bifrost_vstoken_conversion = 118,
		Farming: bifrost_farming = 119,
		SystemStaking: bifrost_system_staking = 120,
		SystemMaker: bifrost_system_maker = 121,
		FeeShare: bifrost_fee_share = 122,
		CrossInOut: bifrost_cross_in_out = 123,
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
	}
}

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = sp_runtime::MultiSignature;
/// Index of a transaction in the chain.
pub type Nonce = u32;
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

/// All migrations that will run on the next runtime upgrade.
///
/// This contains the combined migrations of the last 10 releases. It allows to skip runtime
/// upgrades in case governance decides to do so. THE ORDER IS IMPORTANT.
pub type Migrations = migrations::Unreleased;

/// The runtime migrations per release.
pub mod migrations {
	#![allow(unused_imports)]
	use super::*;

	/// Unreleased migrations. Add new ones here:
	pub type Unreleased = (
		crate::migration::v1::RestoreReferendaV1<crate::migration::ReferendaData, Runtime>,
		crate::migration::v1::RestoreReferendaV1<
			crate::migration::FellowshipReferendaData,
			Runtime,
			governance::fellowship::FellowshipReferendaInstance,
		>,
		bifrost_slpx::migration::BifrostKusamaAddCurrencyToSupportXcmFee<Runtime>,
	);
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
		[bifrost_asset_registry, AssetRegistry]
		[bifrost_call_switchgear, CallSwitchgear]
		[bifrost_cross_in_out, CrossInOut]
		[bifrost_farming, Farming]
		[bifrost_fee_share, FeeShare]
		[bifrost_flexible_fee, FlexibleFee]
		[bifrost_salp, Salp]
		[bifrost_slp, Slp]
		[bifrost_slpx, Slpx]
		[bifrost_stable_pool, StablePool]
		[bifrost_system_maker, SystemMaker]
		[bifrost_system_staking, SystemStaking]
		[bifrost_token_issuer, TokenIssuer]
		[bifrost_vsbond_auction, VSBondAuction]
		[bifrost_vstoken_conversion, VstokenConversion]
		[bifrost_vtoken_minting, VtokenMinting]
		[bifrost_vtoken_voting, VtokenVoting]
		[lend_market, LendMarket]
		[leverage_staking, LeverageStaking]
		// [bifrost_channel_commission, ChannelCommission]
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
		fn get_fee_token_and_amount(who: AccountId, fee: Balance,utx: <Block as BlockT>::Extrinsic) -> (CurrencyId, Balance) {
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

	impl zenlink_stable_amm_runtime_api::StableAmmApi<Block, CurrencyId, u128, AccountId, u32> for Runtime{
		fn get_virtual_price(pool_id: PoolId)->Balance{
			ZenlinkStableAMM::get_virtual_price(pool_id)
		}

		fn get_a(pool_id: PoolId)->Balance{
			ZenlinkStableAMM::get_a(pool_id)
		}

		fn get_a_precise(pool_id: PoolId)->Balance{
			ZenlinkStableAMM::get_a(pool_id) * 100
		}

		fn get_currencies(pool_id: PoolId)->Vec<CurrencyId>{
			ZenlinkStableAMM::get_currencies(pool_id)
		}

		fn get_currency(pool_id: PoolId, index: u32)->Option<CurrencyId>{
			ZenlinkStableAMM::get_currency(pool_id, index)
		}

		fn get_lp_currency(pool_id: PoolId)->Option<CurrencyId>{
			ZenlinkStableAMM::get_lp_currency(pool_id)
		}

		fn get_currency_precision_multipliers(pool_id: PoolId)->Vec<Balance>{
			ZenlinkStableAMM::get_currency_precision_multipliers(pool_id)
		}

		fn get_currency_balances(pool_id: PoolId)->Vec<Balance>{
			ZenlinkStableAMM::get_currency_balances(pool_id)
		}

		fn get_number_of_currencies(pool_id: PoolId)->u32{
			ZenlinkStableAMM::get_number_of_currencies(pool_id)
		}

		fn get_admin_balances(pool_id: PoolId)->Vec<Balance>{
			ZenlinkStableAMM::get_admin_balances(pool_id)
		}

		fn calculate_currency_amount(pool_id: PoolId, amounts:Vec<Balance>, deposit: bool)->Balance{
			ZenlinkStableAMM::stable_amm_calculate_currency_amount(pool_id, &amounts, deposit).unwrap_or_default()
		}

		fn calculate_swap(pool_id: PoolId, in_index: u32, out_index: u32, in_amount: Balance)->Balance{
			ZenlinkStableAMM::stable_amm_calculate_swap_amount(pool_id, in_index as usize, out_index as usize, in_amount).unwrap_or_default()
		}

		fn calculate_remove_liquidity(pool_id: PoolId, amount: Balance)->Vec<Balance>{
			ZenlinkStableAMM::stable_amm_calculate_remove_liquidity(pool_id, amount).unwrap_or_default()
		}

		fn calculate_remove_liquidity_one_currency(pool_id: PoolId, amount:Balance, index: u32)->Balance{
			ZenlinkStableAMM::stable_amm_calculate_remove_liquidity_one_currency(pool_id, amount, index).unwrap_or_default()
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
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
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

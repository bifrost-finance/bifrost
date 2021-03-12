// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

//! The Bifrost Node runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

use sp_std::{collections::btree_set::BTreeSet, prelude::*};
use frame_support::{
	construct_runtime, parameter_types, debug,
	weights::{
		Weight, IdentityFee,
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND}, DispatchClass,
	},
	traits::{Get, Randomness}
};
use frame_system::{
	EnsureRoot,
	limits::{BlockWeights, BlockLength}
};
use codec::{Encode};
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
pub use node_primitives::{AccountId, Signature};
use node_primitives::{
	AccountIndex, Balance, BlockNumber, Hash, Index, Moment,
	AssetId, SwapFee, PoolId, PoolWeight, PoolToken,
	BiddingOrderId, EraId, Amount, CurrencyId, TokenSymbol
};
use sp_api::impl_runtime_apis;
use sp_runtime::{
	Perbill, ApplyExtrinsicResult, Perquintill, FixedPointNumber,
	impl_opaque_keys, generic, create_runtime_str, ModuleId
};
use sp_runtime::transaction_validity::{TransactionValidity, TransactionSource, TransactionPriority};
use sp_runtime::traits::{
	self, BlakeTwo256, Block as BlockT, StaticLookup, SaturatedConversion, Convert, Zero
};
use sp_version::RuntimeVersion;
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use pallet_transaction_payment::{FeeDetails, RuntimeDispatchInfo};
pub use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use sp_inherents::{InherentData, CheckInherentsResult};
use static_assertions::const_assert;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
#[cfg(any(feature = "std", test))]
pub use pallet_balances::Call as BalancesCall;
#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall;

/// Constant values used within the runtime.
pub mod constants;
use constants::{time::*, currency::*};
use sp_runtime::generic::Era;

// XCM imports
use polkadot_parachain::primitives::Sibling;
use xcm::v0::{MultiLocation, NetworkId, Junction};
use xcm_builder::{
	ParentIsDefault, SiblingParachainConvertsVia, AccountId32Aliases, LocationInverter,
	SovereignSignedViaLocation, SiblingParachainAsNative,
	RelayChainAsNative, SignedAccountId32AsNative, ChildParachainConvertsVia
};
use xcm_executor::{Config, XcmExecutor};
use cumulus_primitives_core::{
	relay_chain::Balance as RelayChainBalance,
	ParaId
};

use orml_xcm_support::{
	CurrencyIdConverter, IsConcreteWithGeneralKey, MultiCurrencyAdapter, NativePalletAssetOr
};
use orml_traits::parameter_type_with_key;
use orml_currencies::BasicCurrencyAdapter;

// zenlink imports
use zenlink_protocol::{
	Origin as ZenlinkOrigin, ParaChainWhiteList, Transactor, PairInfo, AssetId as ZenlinkAssetId,
};

/// Weights for pallets used in the runtime.
mod weights;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect("Development wasm binary is not available. This means the client is \
						built with `SKIP_WASM_BUILD` flag and it is only usable for \
						production chains. Please rebuild with the flag disabled.")
}

/// Runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("bifrost-parachain"),
	impl_name: create_runtime_str!("bifrost-parachain"),
	authoring_version: 10,
	// Per convention: if the runtime behavior changes, increment spec_version
	// and set impl_version to 0. If only runtime
	// implementation changes and behavior does not, then leave spec_version as
	// is and increment impl_version.
	spec_version: 1,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

#[derive(codec::Encode, codec::Decode)]
pub enum XCMPMessage<XAccountId, XBalance> {
	/// Transfer tokens to the given account from the Parachain account.
	TransferToken(XAccountId, XBalance),
}

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

// type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

// pub struct DealWithFees;
// impl OnUnbalanced<NegativeImbalance> for DealWithFees {
// 	fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item=NegativeImbalance>) {
// 		if let Some(fees) = fees_then_tips.next() {
// 			// for fees, 80% to treasury, 20% to author
// 			let mut split = fees.ration(80, 20);
// 			if let Some(tips) = fees_then_tips.next() {
// 				// for tips, if any, 80% to treasury, 20% to author (though this can be anything)
// 				tips.ration_merge_into(80, 20, &mut split);
// 			}
// 			Treasury::on_unbalanced(split.0);
// 			Author::on_unbalanced(split.1);
// 		}
// 	}
// }

/// We assume that ~10% of the block weight is consumed by `on_initalize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
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

const_assert!(NORMAL_DISPATCH_RATIO.deconstruct() >= AVERAGE_ON_INITIALIZE_RATIO.deconstruct());

impl frame_system::Config for Runtime {
	type BaseCallFilter = ();
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type DbWeight = RocksDbWeight;
	type Origin = Origin;
	type Call = Call;
	type Index = Index;
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = Indices;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = Version;
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
	type SS58Prefix = SS58Prefix;
}

impl pallet_utility::Config for Runtime {
	type Event = Event;
	type Call = Call;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		RuntimeBlockWeights::get().max_block;
	pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
	type Event = Event;
	type Origin = Origin;
	type PalletsOrigin = OriginCaller;
	type Call = Call;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = MaxScheduledPerBlock;
	type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const IndexDeposit: Balance = 1 * DOLLARS;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = AccountIndex;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type Event = Event;
	type WeightInfo = pallet_indices::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * CENTS;
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type DustRemoval = ();
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Module<Runtime>;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}

impl pallet_transaction_payment::Config for Runtime {
	// type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate =
	TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	type Moment = Moment;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = ();
	type UncleGenerations = UncleGenerations;
	type FilterUncle = ();
	type EventHandler = ();
}

impl_opaque_keys! {
	pub struct SessionKeys {}
}

impl pallet_sudo::Config for Runtime {
	type Event = Event;
	type Call = Call;
}

parameter_types! {
	pub const SessionDuration: BlockNumber = EPOCH_DURATION_IN_SLOTS as _;
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	/// We prioritize im-online heartbeats over election solution submission.
	pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
	where
		Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as traits::Verify>::Signer,
		account: AccountId,
		nonce: Index,
	) -> Option<(Call, <UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload)> {
		let tip = 0;
		// take the biggest period possible.
		let period = BlockHashCount::get()
			.checked_next_power_of_two()
			.map(|c| c / 2)
			.unwrap_or(2) as u64;
		let current_block = System::block_number()
			.saturated_into::<u64>()
			// The `System::block_number` is initialized with `n+1`,
			// so the actual block number is `n`.
			.saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let extra = (
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				// debug::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload
			.using_encoded(|payload| {
				C::sign(payload, public)
			})?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature.into(), extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime where
	Call: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = Call;
}

parameter_types! {
	pub const BasicDeposit: Balance = 10 * DOLLARS;       // 258 bytes on-chain
	pub const FieldDeposit: Balance = 250 * CENTS;        // 66 bytes on-chain
	pub const SubAccountDeposit: Balance = 2 * DOLLARS;   // 53 bytes on-chain
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

// bifrost runtime start
// impl brml_assets::Config for Runtime {
// 	type Event = Event;
// 	type Balance = Balance;
// 	type AssetId = AssetId;
// 	type Price = Price;
// 	type VtokenMint = VtokenMintPrice;
// 	type AssetRedeem = ();
// 	type FetchVtokenMintPrice = VtokenMint;
// 	type WeightInfo = weights::pallet_assets::WeightInfo<Runtime>;
// }

impl brml_voucher::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type WeightInfo = weights::pallet_voucher::WeightInfo<Runtime>;
}

parameter_types! {
	// 3 hours(1800 blocks) as an era
	pub const VtokenMintDuration: BlockNumber = 3 * 60 * MINUTES;
}
parameter_type_with_key! {
	pub RateOfInterestEachBlock: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			&CurrencyId::Token(TokenSymbol::DOT) => 000_761_035_007,
			&CurrencyId::Token(TokenSymbol::ETH) => 000_570_776_255,
			_ => Zero::zero(),
		}
	};
}

impl brml_vtoken_mint::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Assets;
	type VtokenMintDuration = VtokenMintDuration;
	type RateOfInterestEachBlock = RateOfInterestEachBlock;
	type WeightInfo = weights::pallet_vtoken_mint::WeightInfo<Runtime>;
}

// parameter_types! {
// 	pub const MaximumSwapInRatio: u8 = 2;
// 	pub const MinimumPassedInPoolTokenShares: PoolToken = 2;
// 	pub const MinimumSwapFee: SwapFee = 1; // 0.001%
// 	pub const MaximumSwapFee: SwapFee = 10_000; // 10%
// 	pub const FeePrecision: SwapFee = 100_000;
// 	pub const WeightPrecision: PoolWeight = 100_000;
// 	pub const BNCAssetId: AssetId = 0;
// 	pub const InitialPoolSupply: PoolToken = 1_000;
// 	pub const NumberOfSupportedTokens: u8 = 8;
// 	pub const BonusClaimAgeDenominator: BlockNumber = 14_400;
// 	pub const MaximumPassedInPoolTokenShares: PoolToken = 1_000_000;
// }

// impl brml_swap::Config for Runtime {
// 	type Event = Event;
// 	type SwapFee = SwapFee;
// 	type AssetId = AssetId;
// 	type PoolId = PoolId;
// 	type Balance = Balance;
// 	type AssetTrait = Assets;
// 	type PoolWeight = PoolWeight;
// 	type PoolToken = PoolToken;
// 	type MaximumSwapInRatio = MaximumSwapInRatio;
// 	type MinimumPassedInPoolTokenShares = MinimumPassedInPoolTokenShares;
// 	type MinimumSwapFee = MinimumSwapFee;
// 	type MaximumSwapFee = MaximumSwapFee;
// 	type FeePrecision = FeePrecision;
// 	type WeightPrecision = WeightPrecision;
// 	type BNCAssetId = BNCAssetId;
// 	type InitialPoolSupply = InitialPoolSupply;
// 	type NumberOfSupportedTokens = NumberOfSupportedTokens;
// 	type BonusClaimAgeDenominator = BonusClaimAgeDenominator;
// 	type MaximumPassedInPoolTokenShares = MaximumPassedInPoolTokenShares;
// }

// Bid module
// parameter_types! {
// 	pub const TokenOrderROIListLength: u8 = 200u8;
// 	pub const MinimumVotes: u64 = 100;
// 	pub const MaximumVotes: u64 = 50_000;
// 	pub const BlocksPerYear: BlockNumber = 60 * 60 * 24 * 365 / 6;
// 	pub const MaxProposalNumberForBidder: u32 = 5;
// 	pub const ROIPermillPrecision: u32 = 100;
// }

// impl brml_bid::Config for Runtime {
// 	type Event = Event;
// 	type AssetId = AssetId;
// 	type AssetTrait = Assets;
// 	type BiddingOrderId = BiddingOrderId;
// 	type EraId = EraId;
// 	type Balance = Balance;
// 	type TokenOrderROIListLength = TokenOrderROIListLength ;
// 	type MinimumVotes = MinimumVotes;
// 	type MaximumVotes = MaximumVotes;
// 	type BlocksPerYear = BlocksPerYear;
// 	type MaxProposalNumberForBidder = MaxProposalNumberForBidder;
// 	type ROIPermillPrecision = ROIPermillPrecision;
// }

// impl brml_staking_reward::Config for Runtime {
// 	type AssetTrait = Assets;
// 	type Balance = Balance;
// 	type AssetId = AssetId;
// }

parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			&CurrencyId::Token(TokenSymbol::BNC) => 1 * CENTS,
			_ => Zero::zero(),
		}
	};
}

impl brml_assets::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Assets;
	type WeightInfo = ();
}

impl orml_tokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
}

// bifrost runtime end

// culumus runtime start
impl cumulus_pallet_parachain_system::Config for Runtime {
	type Event = Event;
	type OnValidationData = ();
	type SelfParaId = ParachainInfo;
	type DownwardMessageHandlers = ZenlinkProtocol;
	type HrmpMessageHandlers = ZenlinkProtocol;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_xcm_handler::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type UpwardMessageSender = ParachainSystem;
	type HrmpMessageSender = ParachainSystem;
	type SendXcmOrigin = EnsureRoot<AccountId>;
	type AccountIdConverter = LocationConverter;
}

parameter_types! {
	pub const PolkadotNetworkId: NetworkId = NetworkId::Polkadot;
}

pub struct AccountId32Convert;
impl Convert<AccountId, [u8; 32]> for AccountId32Convert {
	fn convert(account_id: AccountId) -> [u8; 32] {
		account_id.into()
	}
}

parameter_types! {
	pub const GetBifrostTokenId: CurrencyId = CurrencyId::Token(TokenSymbol::BNC);
}

pub type BifrostToken = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Runtime {
	type Event = Event;
	type MultiCurrency = Assets;
	type NativeCurrency = BifrostToken;
	type GetNativeCurrencyId = GetBifrostTokenId;
	type WeightInfo = ();
}

parameter_types! {
	pub const RelayChainCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
	pub BifrostNetwork: NetworkId = NetworkId::Named("bifrost".into());
	pub const RococoLocation: MultiLocation = MultiLocation::X1(Junction::Parent);
	pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
	pub const DEXModuleId: ModuleId = ModuleId(*b"zenlink1");
	pub RelayChainOrigin: Origin = ZenlinkOrigin::Relay.into();
	pub Ancestry: MultiLocation = Junction::Parachain {
		id: ParachainInfo::parachain_id().into()
	}.into();

	pub SiblingParachains: Vec<MultiLocation> = vec![
		MultiLocation::X2(Junction::Parent, Junction::Parachain { id: 107 }),
		MultiLocation::X2(Junction::Parent, Junction::Parachain { id: 200 }),
		MultiLocation::X2(Junction::Parent, Junction::Parachain { id: 300 })
	];
}

pub type LocationConverter = (
	ParentIsDefault<AccountId>,
	SiblingParachainConvertsVia<Sibling, AccountId>,
	ChildParachainConvertsVia<ParaId, AccountId>,
	AccountId32Aliases<BifrostNetwork, AccountId>,
);

// pub type LocalAssetTransactor = MultiCurrencyAdapter<
// 	Currencies,
// 	IsConcreteWithGeneralKey<CurrencyId, RelayToNative>,
// 	LocationConverter,
// 	AccountId,
// 	CurrencyIdConverter<CurrencyId, RelayChainCurrencyId>,
// 	CurrencyId,
// >;

pub type LocalAssetTransactor =
    Transactor<Balances, ZenlinkProtocol, LocationConverter, AccountId, ParachainInfo>;

pub type LocalOriginConverter = (
	SovereignSignedViaLocation<LocationConverter, Origin>,
	RelayChainAsNative<RelayChainOrigin, Origin>,
	SiblingParachainAsNative<ZenlinkOrigin, Origin>,
	SignedAccountId32AsNative<BifrostNetwork, Origin>,
);

parameter_types! {
	pub NativeOrmlTokens: BTreeSet<(Vec<u8>, MultiLocation)> = {
		let mut t = BTreeSet::new();
		//TODO: might need to add other assets based on orml-tokens
		t.insert(("BNC".into(), (Junction::Parent, Junction::Parachain { id: 107 }).into()));
		t
	};
}

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = ZenlinkProtocol;
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = LocalOriginConverter;
	//TODO: might need to add other assets based on orml-tokens
	type IsReserve = NativePalletAssetOr<NativeOrmlTokens>;
	type IsTeleporter = ();
	type LocationInverter = LocationInverter<Ancestry>;
}

pub struct RelayToNative;
impl Convert<RelayChainBalance, Balance> for RelayToNative {
	fn convert(val: u128) -> Balance {
		// native is 12
		// relay is 12
		val
	}
}

pub struct NativeToRelay;
impl Convert<Balance, RelayChainBalance> for NativeToRelay {
	fn convert(val: u128) -> Balance {
		// native is 12
		// relay is 12
		val
	}
}

impl orml_xtokens::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type ToRelayChainBalance = NativeToRelay;
	type AccountId32Convert = AccountId32Convert;
	//TODO: change network id if kusama
	type RelayChainNetworkId = PolkadotNetworkId;
	type ParaId = ParachainInfo;
	type AccountIdConverter = LocationConverter;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

pub struct AccountId32Converter;
impl Convert<AccountId, [u8; 32]> for AccountId32Converter {
    fn convert(account_id: AccountId) -> [u8; 32] {
        account_id.into()
    }
}

impl zenlink_protocol::Config for Runtime {
    type Event = Event;
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type UpwardMessageSender = ParachainSystem;
    type HrmpMessageSender = ParachainSystem;
    type NativeCurrency = Balances;
    type AccountIdConverter = LocationConverter;
    type AccountId32Converter = AccountId32Converter;
    type ParaId = ParachainInfo;
    type ModuleId = DEXModuleId;
    type TargetChains = SiblingParachains;
}

// culumus runtime end

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = node_primitives::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		// Basic stuff
		System: frame_system::{Module, Call, Config, Storage, Event<T>} = 0,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage} = 1,
		Utility: pallet_utility::{Module, Call, Event} = 31,
		Scheduler: pallet_scheduler::{Module, Call, Storage, Event<T>} = 32,

		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent} = 2,
		Indices: pallet_indices::{Module, Call, Storage, Config<T>, Event<T>} = 3,
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>} = 4,
		Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>} = 5,
		// Authorship: pallet_authorship::{Module, Call, Storage, Inherent} = 30,

		// parachain modules
		ParachainSystem: cumulus_pallet_parachain_system::{Module, Call, Storage, Inherent, Event} = 6,
		TransactionPayment: pallet_transaction_payment::{Module, Storage} = 7,
		ParachainInfo: parachain_info::{Module, Storage, Config} = 8,
		XcmHandler: cumulus_pallet_xcm_handler::{Module, Call, Event<T>, Origin} = 9,

		// bifrost modules
		BrmlAssets: brml_assets::{Module, Call, Event<T>} = 10,
		VtokenMint: brml_vtoken_mint::{Module, Call, Storage, Event<T>, Config<T>} = 11,
		// Swap: brml_swap::{Module, Call, Storage, Event<T>} = 12,
		// StakingReward: brml_staking_reward::{Module, Storage} = 13,
		Voucher: brml_voucher::{Module, Call, Storage, Event<T>, Config<T>} = 14,
		// Bid: brml_bid::{Module, Call, Storage, Event<T>} = 15,

		// ORML
		XTokens: orml_xtokens::{Module, Storage, Call, Event<T>} = 16,
		Assets: orml_tokens::{Module, Storage, Event<T>, Config<T>} = 17,
		Currencies: orml_currencies::{Module, Call, Event<T>} = 18,

		// zenlink
		ZenlinkProtocol: zenlink_protocol::{Module, Origin, Call, Storage, Event<T>} = 19,
	}
);

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
///
/// When you change this, you **MUST** modify [`sign`] in `bin/node/testing/src/keyring.rs`!
///
/// [`sign`]: <../../testing/src/keyring.rs.html>
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
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<Runtime, Block, frame_system::ChainContext<Runtime>, Runtime, AllModules>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
			data.check_extrinsics(&block)
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			RandomnessCollectiveFlip::random_seed()
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<
		Block,
		Balance,
	> for Runtime {
		fn query_info(uxt: <Block as BlockT>::Extrinsic, len: u32) -> RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(uxt: <Block as BlockT>::Extrinsic, len: u32) -> FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};
			// Trying to add benchmarks directly to the Session Pallet caused cyclic dependency issues.
			// To get around that, we separated the Session benchmarks into its own crate, which is why
			// we need these two lines below.
			use pallet_session_benchmarking::Module as SessionBench;
			use pallet_offences_benchmarking::Module as OffencesBench;
			use frame_system_benchmarking::Module as SystemBench;

			impl pallet_session_benchmarking::Config for Runtime {}
			impl pallet_offences_benchmarking::Config for Runtime {}
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
				// Treasury Account
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da95ecffd7b6c0f78751baa9d281e0bfa3a6d6f646c70792f74727372790000000000000000000000000000000000000000").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmark!(params, batches, pallet_assets, Assets);
			add_benchmark!(params, batches, pallet_babe, Babe);
			add_benchmark!(params, batches, pallet_balances, Balances);
			add_benchmark!(params, batches, pallet_bounties, Bounties);
			add_benchmark!(params, batches, pallet_collective, Council);
			add_benchmark!(params, batches, pallet_contracts, Contracts);
			add_benchmark!(params, batches, pallet_democracy, Democracy);
			add_benchmark!(params, batches, pallet_elections_phragmen, Elections);
			add_benchmark!(params, batches, pallet_grandpa, Grandpa);
			add_benchmark!(params, batches, pallet_identity, Identity);
			add_benchmark!(params, batches, pallet_im_online, ImOnline);
			add_benchmark!(params, batches, pallet_indices, Indices);
			add_benchmark!(params, batches, pallet_lottery, Lottery);
			add_benchmark!(params, batches, pallet_mmr, Mmr);
			add_benchmark!(params, batches, pallet_multisig, Multisig);
			add_benchmark!(params, batches, pallet_offences, OffencesBench::<Runtime>);
			add_benchmark!(params, batches, pallet_proxy, Proxy);
			add_benchmark!(params, batches, pallet_scheduler, Scheduler);
			add_benchmark!(params, batches, pallet_session, SessionBench::<Runtime>);
			add_benchmark!(params, batches, pallet_staking, Staking);
			add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
			add_benchmark!(params, batches, pallet_timestamp, Timestamp);
			add_benchmark!(params, batches, pallet_tips, Tips);
			add_benchmark!(params, batches, pallet_treasury, Treasury);
			add_benchmark!(params, batches, pallet_utility, Utility);
			add_benchmark!(params, batches, pallet_vesting, Vesting);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}

	// zenlink runtime outer apis
	impl zenlink_protocol_runtime_api::ZenlinkProtocolApi<Block, AccountId> for Runtime {
        fn get_assets() -> Vec<ZenlinkAssetId> {
            ZenlinkProtocol::assets_list()
        }

        fn get_balance(
            asset_id: ZenlinkAssetId,
            owner: AccountId
        ) -> Balance {
            ZenlinkProtocol::asset_balance_of(&asset_id, &owner)
        }

        fn get_sovereigns_info(
            asset_id: ZenlinkAssetId
        ) -> Vec<(u32, AccountId, Balance)> {
            ZenlinkProtocol::get_sovereigns_info(&asset_id)
        }

        fn get_all_pairs() -> Vec<PairInfo<AccountId, Balance>> {
            ZenlinkProtocol::get_all_pairs()
        }

        fn get_owner_pairs(
            owner: AccountId
        ) -> Vec<PairInfo<AccountId, Balance>> {
            ZenlinkProtocol::get_owner_pairs(&owner)
        }

        //buy amount token price
        fn get_amount_in_price(
            supply: Balance,
            path: Vec<ZenlinkAssetId>
        ) -> Balance {
            ZenlinkProtocol::get_in_price(supply, path)
        }

        //sell amount token price
        fn get_amount_out_price(
            supply: Balance,
            path: Vec<ZenlinkAssetId>
        ) -> Balance {
            ZenlinkProtocol::get_out_price(supply, path)
        }

        fn get_estimate_lptoken(
            token_0: ZenlinkAssetId,
            token_1: ZenlinkAssetId,
            amount_0_desired: Balance,
            amount_1_desired: Balance,
            amount_0_min: Balance,
            amount_1_min: Balance,
        ) -> Balance{
            ZenlinkProtocol::get_estimate_lptoken(
                token_0,
                token_1,
                amount_0_desired,
                amount_1_desired,
                amount_0_min,
                amount_1_min)
        }
    }

	// impl asset rpc methods for runtime
	// impl brml_assets_rpc_runtime_api::AssetsApi<node_primitives::Block, AssetId, AccountId, Balance> for Runtime {
	// 	fn asset_balances(asset_id: AssetId, who: AccountId) -> u64 {
	// 		Assets::asset_balances(asset_id, who)
	// 	}

	// 	fn asset_tokens(who: AccountId) -> Vec<AssetId> {
	// 		Assets::asset_tokens(who)
	// 	}
	// }

	// impl brml_vtoken_mint_rpc_runtime_api::VtokenMintPriceApi<node_primitives::Block, AssetId, node_primitives::VtokenMintPrice> for Runtime {
	// 	fn get_vtoken_mint_rate(asset_id: AssetId) -> node_primitives::VtokenMintPrice {
	// 		VtokenMint::get_vtoken_mint_price(asset_id)
	// 	}
	// }
}

cumulus_pallet_parachain_system::register_validate_block!(Runtime, Executive);

#[cfg(test)]
mod tests {
	use super::*;
	use frame_system::offchain::CreateSignedTransaction;

	#[test]
	fn validate_transaction_submitter_bounds() {
		fn is_submit_signed_transaction<T>() where
			T: CreateSignedTransaction<Call>,
		{}

		is_submit_signed_transaction::<Runtime>();
	}
}

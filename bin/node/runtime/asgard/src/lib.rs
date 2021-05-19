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

use codec::{Decode, Encode};
use frame_support::{
	construct_runtime, parameter_types, match_type,
	traits::All,
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		DispatchClass, IdentityFee, Weight,
	},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
pub use node_primitives::{AccountId, Signature};
use node_primitives::{
	AccountIndex, Amount, Balance, BlockNumber, CurrencyId, Hash, Index, Moment, TokenSymbol,
};
pub use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::traits::{
	self, BlakeTwo256, Block as BlockT, SaturatedConversion, StaticLookup,
};
use sp_runtime::transaction_validity::{TransactionSource, TransactionValidity};
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys, ApplyExtrinsicResult,
	FixedPointNumber, Perbill, Perquintill,
};
use sp_std::{collections::btree_set::BTreeSet, prelude::*};
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;
use static_assertions::const_assert;

#[cfg(any(feature = "std", test))]
pub use frame_system::Call as SystemCall;
#[cfg(any(feature = "std", test))]
pub use pallet_balances::Call as BalancesCall;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;

/// Constant values used within the runtime.
pub mod constants;
use constants::{currency::*, time::*};
use sp_runtime::generic::Era;

// XCM imports
use cumulus_primitives_core::{relay_chain::Balance as RelayChainBalance, ParaId};
// use orml_currencies::BasicCurrencyAdapter;
// use orml_traits::parameter_type_with_key;
// use orml_xcm_support::XcmHandler as XcmHandlerT;
use polkadot_parachain::primitives::Sibling;
use xcm::v0::{MultiAsset, MultiLocation, MultiLocation::*, Junction::*, BodyId, NetworkId};
use xcm_builder::{
	AccountId32Aliases, CurrencyAdapter, LocationInverter, ParentIsDefault, RelayChainAsNative,
	SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
	SovereignSignedViaLocation, EnsureXcmOrigin, AllowUnpaidExecutionFrom, ParentAsSuperuser,
	AllowTopLevelPaidExecutionFrom, TakeWeightCredit, FixedWeightBounds, IsConcrete, NativeAsset,
	UsingComponents, SignedToAccountId32,
};
use xcm_executor::XcmExecutor;
use pallet_xcm::{XcmPassthrough, EnsureXcm, IsMajorityOfBody};
use xcm::v0::Xcm;

// use zenlink_protocol::{
// 	make_x2_location, AssetId, MultiAssetHandler, NativeCurrencyAdaptor, OtherAssetAdaptor,
// 	PairInfo, ParaChainWhiteList, TokenBalance, TransactorAdaptor,
// };

/// Weights for pallets used in the runtime.
mod weights;

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. This means the client is \
						built with `SKIP_WASM_BUILD` flag and it is only usable for \
						production chains. Please rebuild with the flag disabled.",
	)
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
	spec_version: 2,
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
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
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
	type AccountStore = frame_system::Pallet<Runtime>;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 10 * MILLICENTS;
	pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
	pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
	pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
}

impl pallet_transaction_payment::Config for Runtime {
	// type OnChargeTransaction = ChargeTransactionFee;
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

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as traits::Verify>::Signer,
		account: AccountId,
		nonce: Index,
	) -> Option<(
		Call,
		<UncheckedExtrinsic as traits::Extrinsic>::SignaturePayload,
	)> {
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
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = Indices::unlookup(account);
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (address, signature.into(), extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as traits::Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
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

// impl brml_voucher::Config for Runtime {
// 	type Event = Event;
// 	type Balance = Balance;
// 	type WeightInfo = weights::pallet_voucher::WeightInfo<Runtime>;
// }
//
// parameter_types! {
// 	// 3 hours(1800 blocks) as an era
// 	pub const VtokenMintDuration: BlockNumber = 3 * 60 * MINUTES;
// 	pub const StakingPalletId: PalletId = PalletId(*b"staking ");
// }
// impl brml_vtoken_mint::Config for Runtime {
// 	type Event = Event;
// 	type MultiCurrency = Assets;
// 	type PalletId = StakingPalletId;
// 	type MinterReward = MinterReward;
// 	type DEXOperations = ZenlinkProtocol;
// 	type RandomnessSource = RandomnessCollectiveFlip;
// 	type WeightInfo = weights::pallet_vtoken_mint::WeightInfo<Runtime>;
// }
//
// parameter_type_with_key! {
// 	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
// 		match currency_id {
// 			&CurrencyId::Token(TokenSymbol::ASG) => 1 * CENTS,
// 			_ => Zero::zero(),
// 		}
// 	};
// }
//
// impl brml_assets::Config for Runtime {
// 	type Event = Event;
// 	type MultiCurrency = Assets;
// 	type WeightInfo = ();
// }
//
// impl orml_tokens::Config for Runtime {
// 	type Event = Event;
// 	type Balance = Balance;
// 	type Amount = Amount;
// 	type CurrencyId = CurrencyId;
// 	type WeightInfo = ();
// 	type ExistentialDeposits = ExistentialDeposits;
// 	type OnDust = ();
// }
// parameter_types! {
// 	pub const NativeCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::ASG);
// }
//
// impl brml_charge_transaction_fee::Config for Runtime {
// 	type Event = Event;
// 	type Balance = Balance;
// 	type WeightInfo = ();
// 	type CurrenciesHandler = Currencies;
// 	type Currency = Balances;
// 	type ZenlinkDEX = ZenlinkProtocol;
// 	type OnUnbalanced = ();
// 	type NativeCurrencyId = NativeCurrencyId;
// }
//
// parameter_types! {
// 	pub const TwoYear: BlockNumber = DAYS * 365 * 2;
// 	pub const RewardPeriod: BlockNumber = 50;
// 	pub const MaximumExtendedPeriod: BlockNumber = 100;
// 	pub const ShareWeightPalletId: PalletId = PalletId(*b"weight  ");
// }
//
// impl brml_minter_reward::Config for Runtime {
// 	type Event = Event;
// 	type MultiCurrency = Currencies;
// 	type TwoYear = TwoYear;
// 	type PalletId = ShareWeightPalletId;
// 	type RewardPeriod = RewardPeriod;
// 	type MaximumExtendedPeriod = MaximumExtendedPeriod;
// 	type DEXOperations = ZenlinkProtocol;
// 	type ShareWeight = Balance;
// }

// bifrost runtime end

// culumus runtime start
// impl cumulus_pallet_parachain_system::Config for Runtime {
// 	type Event = Event;
// 	type OnValidationData = ();
// 	type SelfParaId = parachain_info::Module<Runtime>;
// 	type DownwardMessageHandlers = XcmHandler;
// 	type XcmpMessageHandlers = XcmHandler;
// }
//
// impl parachain_info::Config for Runtime {}
//
// impl cumulus_pallet_xcm_handler::Config for Runtime {
// 	type Event = Event;
// 	type XcmExecutor = XcmExecutor<XcmConfig>;
// 	type UpwardMessageSender = ParachainSystem;
// 	type SendXcmOrigin = EnsureRoot<AccountId>;
// 	type AccountIdConverter = LocationConverter;
// 	type XcmpMessageSender = ParachainSystem;
// }
//
// parameter_types! {
// 	pub const PolkadotNetworkId: NetworkId = NetworkId::Polkadot;
// }
//
// pub struct AccountId32Convert;
// impl Convert<AccountId, [u8; 32]> for AccountId32Convert {
// 	fn convert(account_id: AccountId) -> [u8; 32] {
// 		account_id.into()
// 	}
// }
//
// parameter_types! {
// 	pub const GetBifrostTokenId: CurrencyId = CurrencyId::Token(TokenSymbol::ASG);
// }
//
// pub type BifrostToken = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
//
// impl orml_currencies::Config for Runtime {
// 	type Event = Event;
// 	type MultiCurrency = Assets;
// 	type NativeCurrency = BifrostToken;
// 	type GetNativeCurrencyId = GetBifrostTokenId;
// 	type WeightInfo = ();
// }
//
// parameter_types! {
// 	pub const RococoLocation: MultiLocation = MultiLocation::X1(Junction::Parent);
// 	pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
// 	pub RelayChainOrigin: Origin = cumulus_pallet_xcm_handler::Origin::Relay.into();
// 	pub Ancestry: MultiLocation = Junction::Parachain {
// 		id: ParachainInfo::parachain_id().into()
// 	}.into();
// }
//
// type LocationConverter = (
// 	ParentIsDefault<AccountId>,
// 	SiblingParachainConvertsVia<Sibling, AccountId>,
// 	AccountId32Aliases<RococoNetwork, AccountId>,
// );
//
// pub type ZenlinkXcmTransactor =
// 	TransactorAdaptor<ZenlinkProtocol, LocationConverter, AccountId, ParachainInfo>;
//
// type LocalOriginConverter = (
// 	SovereignSignedViaLocation<LocationConverter, Origin>,
// 	RelayChainAsNative<RelayChainOrigin, Origin>,
// 	SiblingParachainAsNative<cumulus_pallet_xcm_handler::Origin, Origin>,
// 	SignedAccountId32AsNative<RococoNetwork, Origin>,
// );
//
// parameter_types! {
// 	pub NativeOrmlTokens: BTreeSet<(Vec<u8>, MultiLocation)> = {
// 		let mut t = BTreeSet::new();
// 		//TODO: might need to add other assets based on orml-tokens
// 		t.insert(("ASG".into(), (Junction::Parent, Junction::Parachain { id: 2001 }).into()));
// 		t
// 	};
// }
//
// pub struct XcmConfig;
// impl xcm_executor::Config for XcmConfig {
// 	type Call = Call;
// 	type XcmSender = XcmHandler;
// 	// How to withdraw and deposit an asset.
// 	type AssetTransactor = ZenlinkXcmTransactor;
// 	type OriginConverter = LocalOriginConverter;
// 	type IsReserve = ParaChainWhiteList<ZenlinkRegistedParaChains>;
// 	type IsTeleporter = ();
// 	type LocationInverter = LocationInverter<Ancestry>;
// }
//
// pub struct RelayToNative;
// impl Convert<RelayChainBalance, Balance> for RelayToNative {
// 	fn convert(val: u128) -> Balance {
// 		// native is 12
// 		// relay is 12
// 		val
// 	}
// }
//
// pub struct NativeToRelay;
// impl Convert<Balance, RelayChainBalance> for NativeToRelay {
// 	fn convert(val: u128) -> Balance {
// 		// native is 12
// 		// relay is 12
// 		val
// 	}
// }
//
// pub struct HandleXcm;
// impl XcmHandlerT<AccountId> for HandleXcm {
// 	fn execute_xcm(origin: AccountId, xcm: Xcm) -> DispatchResult {
// 		XcmHandler::execute_xcm(origin, xcm)
// 	}
// }
//
// pub struct CurrencyIdConvert;
// impl Convert<CurrencyId, Option<MultiLocation>> for CurrencyIdConvert {
// 	fn convert(id: CurrencyId) -> Option<MultiLocation> {
// 		match id {
// 			CurrencyId::Token(TokenSymbol::DOT) => Some(X1(Parent)),
// 			_ => None,
// 		}
// 	}
// }
// impl Convert<MultiLocation, Option<CurrencyId>> for CurrencyIdConvert {
// 	fn convert(location: MultiLocation) -> Option<CurrencyId> {
// 		match location {
// 			X1(Parent) => Some(CurrencyId::Token(TokenSymbol::DOT)),
// 			X3(Parent, Parachain { id }, GeneralKey(key))
// 				if ParaId::from(id) == ParachainInfo::get() =>
// 			{
// 				// decode the general key
// 				if let Ok(currency_id) = CurrencyId::decode(&mut &key[..]) {
// 					// check if `currency_id` is cross-chain asset
// 					match currency_id {
// 						CurrencyId::Token(TokenSymbol::ASG) => Some(currency_id),
// 						_ => None,
// 					}
// 				} else {
// 					None
// 				}
// 			}
// 			_ => None,
// 		}
// 	}
// }
// impl Convert<MultiAsset, Option<CurrencyId>> for CurrencyIdConvert {
// 	fn convert(asset: MultiAsset) -> Option<CurrencyId> {
// 		if let MultiAsset::ConcreteFungible { id, amount: _ } = asset {
// 			Self::convert(id)
// 		} else {
// 			None
// 		}
// 	}
// }
//
// parameter_types! {
// 	pub SelfLocation: MultiLocation = X2(Parent, Parachain { id: ParachainInfo::get().into() });
// }
//
// impl orml_xtokens::Config for Runtime {
// 	type Event = Event;
// 	type Balance = Balance;
// 	type CurrencyId = CurrencyId;
// 	type CurrencyIdConvert = CurrencyIdConvert;
// 	type AccountId32Convert = AccountId32Convert;
// 	type SelfLocation = SelfLocation;
// 	type XcmHandler = HandleXcm;
// }
//
// parameter_types! {
// 	pub const ZenlinkPalletId: PalletId = PalletId(*b"zenlink1");
// 	pub ZenlinkRegistedParaChains: Vec<(MultiLocation, u128)> = vec![
// 		// Phala local and live, 1 PHA
// 		(make_x2_location(30),    1_000_000_000_000),
// 		// Sherpax live
// 		(make_x2_location(59),  500),
// 		// Bifrost local and live, 0.01 ASG
// 		(make_x2_location(2001),   10_000_000_000),
// 		// Zenlink live
// 		(make_x2_location(188), 500),
// 		// Zenlink local
// 		(make_x2_location(200), 500),
// 		// Sherpax local
// 		(make_x2_location(300), 500),
// 		// Plasm local and live, 0.001 PLM
// 		(make_x2_location(5000), 1_000_000_000_000)
// 	];
// }
//
// pub struct AccountId32Converter;
// impl Convert<AccountId, [u8; 32]> for AccountId32Converter {
// 	fn convert(account_id: AccountId) -> [u8; 32] {
// 		account_id.into()
// 	}
// }
//
// pub type AdaptedBasicCurrency =
// 	orml_currencies::BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
//
// impl zenlink_protocol::Config for Runtime {
// 	type Event = Event;
// 	type XcmExecutor = XcmExecutor<XcmConfig>;
// 	type AccountIdConverter = LocationConverter;
// 	type AccountId32Converter = AccountId32Converter;
// 	type ParaId = ParachainInfo;
// 	type PalletId = ZenlinkPalletId;
// 	type TargetChains = ZenlinkRegistedParaChains;
// 	type NativeCurrency = NativeCurrencyAdaptor<Runtime, Balances>;
// 	type OtherAssets = OtherAssetAdaptor<Runtime, Currencies>;
// }
// culumus runtime end

// culumus runtime start
parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 4;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type Event = Event;
	type OnValidationData = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpMessageHandler = DmpQueue;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const RocLocation: MultiLocation = X1(Parent);
	pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
	pub RelayChainOrigin: Origin = cumulus_pallet_xcm::Origin::Relay.into();
	pub Ancestry: MultiLocation = X1(Parachain(ParachainInfo::parachain_id().into()));
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
	AccountId32Aliases<RococoNetwork, AccountId>,
);

/// Means for transacting assets on this chain.
pub type LocalAssetTransactor = CurrencyAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RocLocation>,
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
	SignedAccountId32AsNative<RococoNetwork, Origin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<Origin>,
);

parameter_types! {
	// One XCM operation is 1_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1_000_000;
	// One ROC buys 1 second of weight.
	pub const WeightPrice: (MultiLocation, u128) = (X1(Parent), BNCS);
}

match_type! {
	pub type ParentOrParentsUnitPlurality: impl Contains<MultiLocation> = {
		X1(Parent) | X2(Parent, Plurality { id: BodyId::Unit, .. })
	};
}

pub type Barrier = (
	TakeWeightCredit,
	AllowTopLevelPaidExecutionFrom<All<MultiLocation>>,
	AllowUnpaidExecutionFrom<ParentOrParentsUnitPlurality>,
	// ^^^ Parent & its unit plurality gets free execution
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type Call = Call;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = LocalAssetTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = NativeAsset;
	type IsTeleporter = NativeAsset;	// <- should be enough to allow teleportation of ROC
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
	type Trader = UsingComponents<IdentityFee<Balance>, RocLocation, AccountId, Balances, ()>;
	type ResponseHandler = ();	// Don't handle responses for now.
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = (
	SignedToAccountId32<Origin, AccountId, RococoNetwork>,
);

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
);

impl pallet_xcm::Config for Runtime {
	type Event = Event;
	type SendXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<Origin, LocalOriginToLocation>;
	type XcmExecuteFilter = All<(MultiLocation, Xcm<Call>)>;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type XcmTeleportFilter = All<(MultiLocation, Vec<MultiAsset>)>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ChannelInfo = ParachainSystem;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type ExecuteOverweightOrigin = frame_system::EnsureRoot<AccountId>;
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
}
// culumus runtime end


construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = node_primitives::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		// Basic stuff
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Call, Storage} = 1,
		Utility: pallet_utility::{Pallet, Call, Event} = 31,
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 32,

		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 2,
		Indices: pallet_indices::{Pallet, Call, Storage, Config<T>, Event<T>} = 3,
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 4,
		Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 5,

		// parachain modules
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>, ValidateUnsigned} = 20,
		ParachainInfo: parachain_info::{Pallet, Storage, Config} = 21,

		Aura: pallet_aura::{Pallet, Config<T>},
		AuraExt: cumulus_pallet_aura_ext::{Pallet, Config},

		// XCM helpers
		XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 50,
		PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin} = 51,
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Event<T>, Origin} = 52,
		DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 53,

		// parachain modules
		// ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event} = 6,
		// TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 7,
		// ParachainInfo: parachain_info::{Pallet, Storage, Config} = 8,
		// XcmHandler: cumulus_pallet_xcm_handler::{Pallet, Call, Event<T>, Origin} = 9,

		// bifrost modules
		// BrmlAssets: brml_assets::{Pallet, Call, Event<T>} = 10,
		// VtokenMint: brml_vtoken_mint::{Pallet, Call, Storage, Event<T>, Config<T>} = 11,
		// MinterReward: brml_minter_reward::{Pallet, Storage, Event<T>, Config<T>} = 13,
		// Voucher: brml_voucher::{Pallet, Call, Storage, Event<T>, Config<T>} = 14,
		// ChargeTransactionFee: brml_charge_transaction_fee::{Pallet, Call, Storage, Event<T>} = 20,

		// ORML
		// XTokens: orml_xtokens::{Pallet, Storage, Call, Event<T>} = 16,
		// Assets: orml_tokens::{Pallet, Storage, Event<T>, Config<T>} = 17,
		// Currencies: orml_currencies::{Pallet, Call, Event<T>} = 18,

		// zenlink
		// ZenlinkProtocol: zenlink_protocol::{Pallet, Call, Storage, Event<T>} = 19,
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
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPallets,
>;

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

	// impl brml_charge_transaction_fee_rpc_runtime_api::ChargeTransactionFeeRuntimeApi<Block, AccountId> for Runtime {
	// 	fn get_fee_token_and_amount(who: AccountId, fee: Balance) -> (CurrencyId, Balance) {
	// 	let rs = ChargeTransactionFee::cal_fee_token_and_amount(&who, fee);
	// 		match rs {
	// 			Ok(val) => val,
	// 			_ => (CurrencyId::Token(TokenSymbol::ASG), Zero::zero()),
	// 		}
	// 	}
	// }

	// zenlink runtime outer apis
	// impl zenlink_protocol_runtime_api::ZenlinkProtocolApi<Block, AccountId> for Runtime {
	// 	fn get_assets() -> Vec<AssetId> {
	// 		ZenlinkProtocol::assets_list()
	// 	}
	//
	// 	fn get_balance(
	// 		asset_id: AssetId,
	// 		owner: AccountId
	// 	) -> TokenBalance {
	// 		ZenlinkProtocol::multi_asset_balance_of(&asset_id, &owner)
	// 	}
	//
	// 	fn get_sovereigns_info(
	// 		asset_id: AssetId
	// 	) -> Vec<(u32, AccountId, TokenBalance)> {
	// 		ZenlinkProtocol::get_sovereigns_info(&asset_id)
	// 	}
	//
	// 	fn get_all_pairs() -> Vec<PairInfo<AccountId, TokenBalance>> {
	// 		ZenlinkProtocol::get_all_pairs()
	// 	}
	//
	// 	fn get_owner_pairs(
	// 		owner: AccountId
	// 	) -> Vec<PairInfo<AccountId, TokenBalance>> {
	// 		ZenlinkProtocol::get_owner_pairs(&owner)
	// 	}
	//
	// 	fn get_amount_in_price(
	// 		supply: TokenBalance,
	// 		path: Vec<AssetId>
	// 	) -> TokenBalance {
	// 		ZenlinkProtocol::desired_in_amount(supply, path)
	// 	}
	//
	// 	fn get_amount_out_price(
	// 		supply: TokenBalance,
	// 		path: Vec<AssetId>
	// 	) -> TokenBalance {
	// 		ZenlinkProtocol::supply_out_amount(supply, path)
	// 	}
	//
	// 	fn get_estimate_lptoken(
	// 		token_0: AssetId,
	// 		token_1: AssetId,
	// 		amount_0_desired: TokenBalance,
	// 		amount_1_desired: TokenBalance,
	// 		amount_0_min: TokenBalance,
	// 		amount_1_min: TokenBalance,
	// 	) -> TokenBalance{
	// 		ZenlinkProtocol::get_estimate_lptoken(
	// 			token_0,
	// 			token_1,
	// 			amount_0_desired,
	// 			amount_1_desired,
	// 			amount_0_min,
	// 			amount_1_min)
	// 	}
	// }
}

cumulus_pallet_parachain_system::register_validate_block!(
	Runtime,
	cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
);

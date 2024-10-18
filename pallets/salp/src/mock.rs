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

// Ensure we're `no_std` when compiling for Wasm.

#![cfg(test)]

use bifrost_asset_registry::AssetIdMaps;
use bifrost_primitives::{
	Amount, Balance, BifrostCrowdloanId, BifrostEntranceAccount, BifrostExitAccount,
	BuybackPalletId,
	CurrencyId::{self, *},
	IncentivePoolAccount, MessageId, MockXcmExecutor, ParaId, SlpOperator, SlpxOperator,
	StableAssetPalletId,
	TokenSymbol::{self, *},
	ZenlinkPalletId, ASG, KSM, KUSD, VKSM,
};
use bifrost_xcm_interface::traits::XcmHelper;
use cumulus_primitives_core::ParaId as Pid;
use frame_support::{
	construct_runtime, derive_impl, ord_parameter_types, parameter_types,
	sp_runtime::{DispatchError, DispatchResult, SaturatedConversion},
	traits::{ConstU128, ConstU64, EnsureOrigin, Everything, Get, LockIdentifier, Nothing},
	weights::Weight,
};
use frame_system::{EnsureRoot, EnsureSignedBy, RawOrigin};
use orml_traits::{location::RelativeReserveProvider, parameter_type_with_key, MultiCurrency};
use sp_arithmetic::Percent;
use sp_core::ConstU32;
pub use sp_runtime::Perbill;
use sp_runtime::{
	traits::{Convert, IdentityLookup, UniqueSaturatedInto},
	BuildStorage,
};
use sp_std::marker::PhantomData;
use xcm::prelude::*;
use xcm_builder::{FixedWeightBounds, FrameTransactionalProcessor};
use xcm_executor::XcmExecutor;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler, PairLpGenerate, ZenlinkMultiAssets,
};

use crate as salp;
use bifrost_primitives::MoonbeamChainId;

pub(crate) type AccountId = <<Signature as sp_runtime::traits::Verify>::Signer as sp_runtime::traits::IdentifyAccount>::AccountId;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u32;
pub(crate) type Signature = sp_runtime::MultiSignature;

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Sudo: pallet_sudo,
		Balances: pallet_balances,
		Currencies: bifrost_currencies,
		Tokens: orml_tokens,
		XTokens: orml_xtokens,
		Multisig: pallet_multisig,
		Salp: salp,
		ZenlinkProtocol: zenlink_protocol,
		AssetRegistry: bifrost_asset_registry,
		PolkadotXcm: pallet_xcm,
		StableAsset: bifrost_stable_asset,
		StablePool: bifrost_stable_pool,
		VtokenMinting: bifrost_vtoken_minting,
		XcmInterface: bifrost_xcm_interface,
	}
);

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = ASG;
	pub const RelayCurrencyId: CurrencyId = KSM;
	pub const StableCurrencyId: CurrencyId = KUSD;
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type Block = Block;
	type Lookup = IdentityLookup<Self::AccountId>;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
	pub const TransferFee: u128 = 0;
	pub const CreationFee: u128 = 0;
	pub const TransactionByteFee: u128 = 0;
	pub const MaxLocks: u32 = 999_999;
	pub const MaxReserves: u32 = 999_999;
}

impl pallet_balances::Config for Test {
	type AccountStore = System;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
}

parameter_types! {
	pub const DepositBase: Balance = 0;
	pub const DepositFactor: Balance = 0;
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Test {
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type RuntimeEvent = RuntimeEvent;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Test>;
}

impl pallet_sudo::Config for Test {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
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
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type CurrencyHooks = ();
}

pub type BifrostToken =
	bifrost_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Test {
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BifrostToken;
	type WeightInfo = ();
}

parameter_types! {
	pub const GetExchangeFee: (u32, u32) = (3, 1000);   // 0.3%
	pub const SelfParaId: u32 = 2001;
}

impl zenlink_protocol::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiAssetsHandler = MultiAssets;
	type PalletId = ZenlinkPalletId;
	type SelfParaId = SelfParaId;
	type TargetChains = ();
	type WeightInfo = ();
	type AssetId = ZenlinkAssetId;
	type LpGenerate = PairLpGenerate<Self>;
}

ord_parameter_types! {
	pub const CouncilAccount: AccountId = AccountId::from([1u8; 32]);
}
impl bifrost_asset_registry::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureSignedBy<CouncilAccount, AccountId>;
	type WeightInfo = ();
}

type MultiAssets = ZenlinkMultiAssets<ZenlinkProtocol, Balances, LocalAssetAdaptor<Currencies>>;

// Below is the implementation of tokens manipulation functions other than native token.
pub struct LocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for LocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId = CurrencyId>,
{
	fn local_balance_of(asset_id: ZenlinkAssetId, who: &AccountId) -> AssetBalance {
		let currency_id: CurrencyId = asset_id.try_into().unwrap();
		Local::free_balance(currency_id, &who).saturated_into()
	}

	fn local_total_supply(asset_id: ZenlinkAssetId) -> AssetBalance {
		let currency_id: CurrencyId = asset_id.try_into().unwrap();
		Local::total_issuance(currency_id).saturated_into()
	}

	fn local_is_exists(asset_id: ZenlinkAssetId) -> bool {
		let rs: Result<CurrencyId, _> = asset_id.try_into();
		match rs {
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
		let currency_id: CurrencyId = asset_id.try_into().unwrap();
		Local::transfer(currency_id, &origin, &target, amount.unique_saturated_into())?;

		Ok(())
	}

	fn local_deposit(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		let currency_id: CurrencyId = asset_id.try_into().unwrap();
		Local::deposit(currency_id, &origin, amount.unique_saturated_into())?;
		return Ok(amount);
	}

	fn local_withdraw(
		asset_id: ZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		let currency_id: CurrencyId = asset_id.try_into().unwrap();
		Local::withdraw(currency_id, &origin, amount.unique_saturated_into())?;

		Ok(amount)
	}
}

pub const TREASURY_ACCOUNT: AccountId = AccountId::new([9u8; 32]);

parameter_types! {
	pub const MinContribution: Balance = 10;
	pub const RemoveKeysLimit: u32 = 50;
	pub const SlotLength: BlockNumber = 8u32 as BlockNumber;
	pub const LeasePeriod: BlockNumber = 6 * WEEKS;
	pub const VSBondValidPeriod: BlockNumber = 30 * DAYS;
	pub const ReleaseCycle: BlockNumber = 1 * DAYS;
	pub const ReleaseRatio: Percent = Percent::from_percent(50);
	pub ConfirmMuitiSigAccount: AccountId = Multisig::multi_account_id(&vec![
		ALICE,
		BRUCE,
		CATHI
	],2);
	pub const TreasuryAccount: AccountId = TREASURY_ACCOUNT;
	pub const BatchLimit: u32 = 50;
}

pub struct EnsureConfirmAsGovernance;
impl EnsureOrigin<RuntimeOrigin> for EnsureConfirmAsGovernance {
	type Success = AccountId;

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		Into::<Result<RawOrigin<AccountId>, RuntimeOrigin>>::into(o).and_then(|o| match o {
			RawOrigin::Signed(who) => Ok(who),
			RawOrigin::Root => Ok(ConfirmMuitiSigAccount::get()),
			r => Err(RuntimeOrigin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::from(RawOrigin::Signed(ConfirmMuitiSigAccount::get())))
	}
}

// To control the result returned by `MockXcmExecutor`
pub(crate) static mut MOCK_XCM_RESULT: (bool, bool) = (true, true);

// Mock XcmExecutor
pub struct MockSalpXcmExecutor;

impl XcmHelper<crate::AccountIdOf<Test>, crate::BalanceOf<Test>> for MockSalpXcmExecutor {
	fn contribute(
		_contributer: AccountId,
		_index: ParaId,
		_value: Balance,
	) -> Result<MessageId, DispatchError> {
		let result = unsafe { MOCK_XCM_RESULT.0 };

		match result {
			true => Ok([0; 32]),
			false => Err(DispatchError::BadOrigin),
		}
	}
}

pub struct EnsurePoolAssetId;
impl bifrost_stable_asset::traits::ValidateAssetId<CurrencyId> for EnsurePoolAssetId {
	fn validate(_: CurrencyId) -> bool {
		true
	}
}

impl bifrost_stable_asset::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type AssetId = CurrencyId;
	type Balance = Balance;
	type Assets = Tokens;
	type PalletId = StableAssetPalletId;
	type AtLeast64BitUnsigned = u128;
	type FeePrecision = ConstU128<10_000_000_000>;
	type APrecision = ConstU128<100>;
	type PoolAssetLimit = ConstU32<5>;
	type SwapExactOverAmount = ConstU128<100>;
	type WeightInfo = ();
	type ListingOrigin = EnsureSignedBy<CouncilAccount, AccountId>;
	type EnsurePoolAssetId = EnsurePoolAssetId;
}

impl bifrost_stable_pool::Config for Test {
	type WeightInfo = ();
	type ControlOrigin = EnsureConfirmAsGovernance;
	type CurrencyId = CurrencyId;
	type MultiCurrency = Tokens;
	type StableAsset = StableAsset;
	type VtokenMinting = VtokenMinting;
	type CurrencyIdConversion = AssetIdMaps<Test>;
	type CurrencyIdRegister = AssetIdMaps<Test>;
}

parameter_types! {
	pub const MaximumUnlockIdOfUser: u32 = 1_000;
	pub const MaximumUnlockIdOfTimeUnit: u32 = 1_000;
}

pub struct SlpxInterface;
impl SlpxOperator<Balance> for SlpxInterface {
	fn get_moonbeam_transfer_to_fee() -> Balance {
		Default::default()
	}
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: Location| -> Option<u128> {
		Some(u128::MAX)
	};
}

parameter_types! {
	pub SelfRelativeLocation: Location = Location::here();
	pub const MaxAssetsForTransfer: usize = 2;
}

impl orml_xtokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = ();
	type AccountIdToLocation = ();
	type UniversalLocation = UniversalLocation;
	type SelfLocation = SelfRelativeLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = ();
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type LocationsFilter = Everything;
	type ReserveProvider = RelativeReserveProvider;
	type RateLimiter = ();
	type RateLimiterId = ();
}

pub struct Slp;
// Functions to be called by other pallets.
impl SlpOperator<CurrencyId> for Slp {
	fn all_delegation_requests_occupied(_currency_id: CurrencyId) -> bool {
		true
	}
}

impl bifrost_vtoken_minting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Tokens;
	type ControlOrigin = EnsureConfirmAsGovernance;
	type MaximumUnlockIdOfUser = MaximumUnlockIdOfUser;
	type MaximumUnlockIdOfTimeUnit = MaximumUnlockIdOfTimeUnit;
	type EntranceAccount = BifrostEntranceAccount;
	type ExitAccount = BifrostExitAccount;
	type FeeAccount = CouncilAccount;
	type RedeemFeeAccount = CouncilAccount;
	type RelayChainToken = RelayCurrencyId;
	type WeightInfo = ();
	type OnRedeemSuccess = ();
	type XcmTransfer = XTokens;
	type MoonbeamChainId = MoonbeamChainId;
	type BifrostSlpx = SlpxInterface;
	type ChannelCommission = ();
	type MaxLockRecords = ConstU32<100>;
	type IncentivePoolAccount = IncentivePoolAccount;
	type BbBNC = ();
}

parameter_types! {
	pub const SalpLockId: LockIdentifier = *b"salplock";
}

impl salp::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type RuntimeOrigin = RuntimeOrigin;
	type LeasePeriod = LeasePeriod;
	type MinContribution = MinContribution;
	type MultiCurrency = Tokens;
	type PalletId = BifrostCrowdloanId;
	type RelayChainToken = RelayCurrencyId;
	type ReleaseCycle = ReleaseCycle;
	type ReleaseRatio = ReleaseRatio;
	type RemoveKeysLimit = RemoveKeysLimit;
	type SlotLength = SlotLength;
	type VSBondValidPeriod = VSBondValidPeriod;
	type EnsureConfirmAsGovernance = EnsureConfirmAsGovernance;
	type WeightInfo = ();
	type XcmInterface = MockSalpXcmExecutor;
	type TreasuryAccount = TreasuryAccount;
	type BuybackPalletId = BuybackPalletId;
	type CurrencyIdConversion = AssetIdMaps<Test>;
	type CurrencyIdRegister = AssetIdMaps<Test>;
	type StablePool = StablePool;
	type VtokenMinting = VtokenMinting;
}

parameter_types! {
	// One XCM operation is 200_000_000 XcmWeight, cross-chain transfer ~= 2x of transfer = 3_000_000_000
	pub UnitWeightCost: Weight = Weight::from_parts(200_000_000, 0);
	pub const MaxInstructions: u32 = 100;
	pub UniversalLocation: InteriorLocation = Parachain(2001).into();
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
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

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<Location> = Some(Parent.into());
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
	type XcmRouter = ();
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

pub struct BifrostAccountIdToMultiLocation;
impl Convert<AccountId, Location> for BifrostAccountIdToMultiLocation {
	fn convert(account: AccountId) -> Location {
		(AccountId32 { network: None, id: account.into() }).into()
	}
}

impl bifrost_xcm_interface::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type UpdateOrigin = EnsureRoot<AccountId>;
	type MultiCurrency = Currencies;
	type RelayNetwork = RelayNetwork;
	type RelaychainCurrencyId = RelayCurrencyId;
	type ParachainSovereignAccount = TreasuryAccount;
	type XcmExecutor = MockXcmExecutor;
	type AccountIdToLocation = BifrostAccountIdToMultiLocation;
	type SalpHelper = Salp;
	type ParachainId = ParaInfo;
	type CallBackTimeOut = ConstU64<10>;
	type CurrencyIdConvert = AssetIdMaps<Test>;
}

pub struct ParaInfo;
impl Get<Pid> for ParaInfo {
	fn get() -> Pid {
		Pid::from(2001)
	}
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pub const DOLLARS: Balance = 1_000_000_000_000;

	let currency = vec![
		(Native(BNC), DOLLARS / 100, None),
		(Stable(TokenSymbol::KUSD), DOLLARS / 10_000, None),
		(Token(TokenSymbol::KSM), DOLLARS / 10_000, None),
		(Token(ZLK), DOLLARS / 1000_000, None),
		(Token(KAR), DOLLARS / 10_000, None),
		(Token(RMRK), DOLLARS / 1000_000, None),
		(Token(PHA), 4 * DOLLARS / 100, None),
		(Token(MOVR), DOLLARS / 1000_000, None),
		(Token(DOT), DOLLARS / 1000_000, None),
	];
	let vcurrency = vec![Native(BNC), Token(TokenSymbol::KSM), Token(MOVR)];
	let vsbond = vec![];
	bifrost_asset_registry::GenesisConfig::<Test> {
		currency,
		vcurrency,
		vsbond,
		phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	pallet_sudo::GenesisConfig::<Test> { key: Some(ALICE) }
		.assimilate_storage(&mut t)
		.unwrap();

	orml_tokens::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, NativeCurrencyId::get(), INIT_BALANCE),
			(ALICE, RelayCurrencyId::get(), INIT_BALANCE),
			(ALICE, CurrencyId::VSToken(TokenSymbol::KSM), INIT_BALANCE),
			(ALICE, VKSM, INIT_BALANCE),
			(BRUCE, NativeCurrencyId::get(), INIT_BALANCE),
			(BRUCE, RelayCurrencyId::get(), INIT_BALANCE),
			(CATHI, NativeCurrencyId::get(), INIT_BALANCE),
			(CATHI, RelayCurrencyId::get(), INIT_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	crate::GenesisConfig::<Test> { initial_multisig_account: Some(ALICE) }
		.assimilate_storage(&mut t)
		.unwrap();

	t.into()
}

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60 / (12 as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;

pub(crate) const ALICE: AccountId = AccountId::new([0u8; 32]);
pub(crate) const BRUCE: AccountId = AccountId::new([1u8; 32]);
pub(crate) const CATHI: AccountId = AccountId::new([2u8; 32]);

pub(crate) const INIT_BALANCE: Balance = 100_000;

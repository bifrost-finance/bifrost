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

#![cfg(test)]

use super::*;
use crate::{self as flexible_fee, tests::CHARLIE};
use bifrost_asset_registry::AssetIdMaps;
use bifrost_primitives::{
	Balance, CurrencyId, DerivativeAccountHandler, DerivativeIndex, ExtraFeeInfo, MessageId,
	ParaId, TokenSymbol, VTokenSupplyProvider, VKSM,
};
use bifrost_vtoken_voting::AccountVote;
use bifrost_xcm_interface::traits::XcmHelper;
use cumulus_primitives_core::ParaId as Pid;
use frame_support::{
	ord_parameter_types, parameter_types,
	sp_runtime::{DispatchError, DispatchResult},
	traits::{Everything, Get, LockIdentifier, Nothing},
	weights::{ConstantMultiplier, IdentityFee},
	PalletId,
};
use frame_system as system;
use frame_system::{EnsureRoot, EnsureSignedBy};
use orml_traits::MultiCurrency;
use pallet_balances::Call as BalancesCall;
use pallet_xcm::EnsureResponse;
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{
		AccountIdConversion, BlakeTwo256, BlockNumberProvider, IdentityLookup, UniqueSaturatedInto,
	},
	AccountId32, BuildStorage, SaturatedConversion,
};
use sp_std::marker::PhantomData;
use std::convert::TryInto;
use xcm::prelude::*;
use xcm_builder::FixedWeightBounds;
use xcm_executor::XcmExecutor;
use zenlink_protocol::{
	AssetId as ZenlinkAssetId, LocalAssetHandler, PairLpGenerate, ZenlinkMultiAssets,
};

pub type AccountId = AccountId32;
pub type BlockNumber = u32;
pub type Amount = i128;

pub const TREASURY_ACCOUNT: AccountId32 = AccountId32::new([9u8; 32]);

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, u128, RuntimeCall, ()>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test {
		System: system,
		Tokens: orml_tokens,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		FlexibleFee: flexible_fee,
		ZenlinkProtocol: zenlink_protocol,
		Currencies: bifrost_currencies,
		Salp: bifrost_salp,
		AssetRegistry: bifrost_asset_registry,
		PolkadotXcm: pallet_xcm,
		VtokenVoting: bifrost_vtoken_voting,
	}
);

pub(crate) const BALANCE_TRANSFER_CALL: <Test as frame_system::Config>::RuntimeCall =
	RuntimeCall::Balances(BalancesCall::transfer_allow_death { dest: ALICE, value: 69 });

pub(crate) const SALP_CONTRIBUTE_CALL: <Test as frame_system::Config>::RuntimeCall =
	RuntimeCall::Salp(bifrost_salp::Call::contribute { index: 2001, value: 1_000_000_000_000 });

pub(crate) const VTOKENVOTING_VOTE_CALL: <Test as frame_system::Config>::RuntimeCall =
	RuntimeCall::VtokenVoting(bifrost_vtoken_voting::Call::vote {
		vtoken: VKSM,
		poll_index: 1u32,
		vote: AccountVote::Split { aye: 1, nay: 1 },
	});

impl bifrost_asset_registry::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureSignedBy<One, AccountId>;
	type WeightInfo = ();
}

parameter_types! {
	pub const BlockHashCount: u32 = 250;
}

impl system::Config for Test {
	type AccountData = pallet_balances::AccountData<u128>;
	type AccountId = AccountId;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockWeights = ();
	type RuntimeCall = RuntimeCall;
	type DbWeight = ();
	type RuntimeEvent = RuntimeEvent;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Nonce = u128;
	type Block = Block;
	// needs to be u128 against u64, otherwise the account address will be half cut.
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type RuntimeOrigin = RuntimeOrigin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Test {
	type FeeMultiplierUpdate = ();
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type OnChargeTransaction = FlexibleFee;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = IdentityFee<Balance>;
	type RuntimeEvent = RuntimeEvent;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = u128;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type RuntimeHoldReason = ();
	type FreezeIdentifier = ();
	type MaxHolds = ConstU32<0>;
	type MaxFreezes = ConstU32<0>;
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		1
	};
}

parameter_types! {
	pub DustAccount: AccountId = PalletId(*b"orml/dst").into_account_truncating();
	pub MaxLocks: u32 = 2;
}

impl orml_tokens::Config for Test {
	type Amount = i128;
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

parameter_types! {
	pub const TreasuryAccount: AccountId32 = TREASURY_ACCOUNT;
	pub const MaxFeeCurrencyOrderListLen: u32 = 50;
}

ord_parameter_types! {
	pub const One: AccountId = CHARLIE;
}

impl crate::Config for Test {
	type Currency = Balances;
	type DexOperator = ZenlinkProtocol;
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type TreasuryAccount = TreasuryAccount;
	type MaxFeeCurrencyOrderListLen = MaxFeeCurrencyOrderListLen;
	type OnUnbalanced = ();
	type WeightInfo = ();
	type ExtraFeeMatcher = ExtraFeeMatcher;
	type ParachainId = ParaInfo;
	type ControlOrigin = EnsureRoot<AccountId>;
	type XcmWeightAndFeeHandler = XcmDestWeightAndFee;
}

pub struct XcmDestWeightAndFee;
impl XcmDestWeightAndFeeHandler<CurrencyId, Balance> for XcmDestWeightAndFee {
	fn get_operation_weight_and_fee(
		_token: CurrencyId,
		_operation: XcmOperationType,
	) -> Option<(Weight, Balance)> {
		Some((Weight::from_parts(100, 100), 100u32.into()))
	}

	fn set_xcm_dest_weight_and_fee(
		_currency_id: CurrencyId,
		_operation: XcmOperationType,
		_weight_and_fee: Option<(Weight, Balance)>,
	) -> DispatchResult {
		Ok(())
	}
}

pub struct ExtraFeeMatcher;
impl FeeGetter<RuntimeCall> for ExtraFeeMatcher {
	fn get_fee_info(c: &RuntimeCall) -> ExtraFeeInfo {
		match *c {
			RuntimeCall::Salp(bifrost_salp::Call::contribute { .. }) => ExtraFeeInfo {
				extra_fee_name: ExtraFeeName::SalpContribute,
				extra_fee_currency: RelayCurrencyId::get(),
			},
			RuntimeCall::VtokenVoting(bifrost_vtoken_voting::Call::vote { vtoken, .. }) =>
				ExtraFeeInfo {
					extra_fee_name: ExtraFeeName::VoteVtoken,
					extra_fee_currency: vtoken.to_token().unwrap_or(vtoken),
				},
			_ => ExtraFeeInfo::default(),
		}
	}
}

pub struct ParaInfo;
impl Get<Pid> for ParaInfo {
	fn get() -> Pid {
		Pid::from(2001)
	}
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
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
	pub const ZenlinkPalletId: PalletId = PalletId(*b"/zenlink");
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

// Build genesis storage according to the mock runtime.
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}

//************** Salp mock start *****************

// To control the result returned by `MockXcmExecutor`
pub(crate) static mut MOCK_XCM_RESULT: (bool, bool) = (true, true);

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

// Mock XcmExecutor
pub struct MockXcmExecutor;

impl XcmHelper<AccountIdOf<Test>, crate::pallet::PalletBalanceOf<Test>> for MockXcmExecutor {
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

pub const ALICE: AccountId = AccountId::new([0u8; 32]);

parameter_types! {
	pub const MinContribution: Balance = 10;
	pub const BifrostCrowdloanId: PalletId = PalletId(*b"bf/salp#");
	pub const RemoveKeysLimit: u32 = 50;
	pub const SlotLength: BlockNumber = 8u32 as BlockNumber;
	pub const LeasePeriod: BlockNumber = 8u32 as BlockNumber;
	pub const VSBondValidPeriod: BlockNumber = 8u32 as BlockNumber;
	pub const ReleaseCycle: BlockNumber = 8u32 as BlockNumber;
	pub const ReleaseRatio: Percent = Percent::from_percent(50);
	pub ConfirmMuitiSigAccount: AccountId = ALICE;
	pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const BuybackPalletId: PalletId = PalletId(*b"bf/salpc");
	pub const SalpLockId: LockIdentifier = *b"salplock";
	pub const BatchLimit: u32 = 50;
}

impl bifrost_salp::Config for Test {
	type BancorPool = ();
	type RuntimeEvent = RuntimeEvent;
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
	type WeightInfo = ();
	type EnsureConfirmAsGovernance = EnsureRoot<AccountId>;
	type XcmInterface = MockXcmExecutor;
	type TreasuryAccount = TreasuryAccount;
	type BuybackPalletId = BuybackPalletId;
	type DexOperator = ZenlinkProtocol;
	type CurrencyIdConversion = AssetIdMaps<Test>;
	type CurrencyIdRegister = AssetIdMaps<Test>;
	type ParachainId = ParaInfo;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type StablePool = ();
	type VtokenMinting = ();
	type LockId = SalpLockId;
	type BatchLimit = BatchLimit;
}

parameter_types! {
	// One XCM operation is 200_000_000 XcmWeight, cross-chain transfer ~= 2x of transfer = 3_000_000_000
	pub UnitWeightCost: Weight = Weight::from_parts(200_000_000, 0);
	pub const MaxInstructions: u32 = 100;
	pub UniversalLocation: InteriorMultiLocation = X1(Parachain(2001));
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
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
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
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
}

//************** Salp mock end *****************

// ************** VtokenVoting mock start *****************
pub struct SimpleVTokenSupplyProvider;

impl VTokenSupplyProvider<CurrencyId, Balance> for SimpleVTokenSupplyProvider {
	fn get_vtoken_supply(_: CurrencyId) -> Option<Balance> {
		Some(u64::MAX.into())
	}

	fn get_token_supply(_: CurrencyId) -> Option<Balance> {
		Some(u64::MAX.into())
	}
}

parameter_types! {
	pub const ReferendumCheckInterval: BlockNumber = 300;
}

impl bifrost_vtoken_voting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
	type ResponseOrigin = EnsureResponse<Everything>;
	type XcmDestWeightAndFee = XcmDestWeightAndFee;
	type DerivativeAccount = DerivativeAccount;
	type RelaychainBlockNumberProvider = RelaychainDataProvider;
	type VTokenSupplyProvider = SimpleVTokenSupplyProvider;
	type MaxVotes = ConstU32<256>;
	type ParachainId = ParaInfo;
	type QueryTimeout = QueryTimeout;
	type ReferendumCheckInterval = ReferendumCheckInterval;
	type WeightInfo = ();
}

pub struct DerivativeAccount;
impl DerivativeAccountHandler<CurrencyId, Balance> for DerivativeAccount {
	fn check_derivative_index_exists(
		_token: CurrencyId,
		_derivative_index: DerivativeIndex,
	) -> bool {
		true
	}

	fn get_multilocation(
		_token: CurrencyId,
		_derivative_index: DerivativeIndex,
	) -> Option<MultiLocation> {
		Some(Parent.into())
	}

	fn get_stake_info(
		token: CurrencyId,
		derivative_index: DerivativeIndex,
	) -> Option<(Balance, Balance)> {
		Self::get_multilocation(token, derivative_index)
			.and_then(|_location| Some((u32::MAX.into(), u32::MAX.into())))
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn init_minimums_and_maximums(_token: CurrencyId) {}

	#[cfg(feature = "runtime-benchmarks")]
	fn new_delegator_ledger(_token: CurrencyId, _who: MultiLocation) {}

	#[cfg(feature = "runtime-benchmarks")]
	fn add_delegator(_token: CurrencyId, _index: DerivativeIndex, _who: MultiLocation) {}
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
	type BlockNumber = BlockNumberFor<Test>;

	fn current_block_number() -> Self::BlockNumber {
		RelaychainBlockNumber::get().into()
	}
}

ord_parameter_types! {
	pub const QueryTimeout: BlockNumber = 100;
}

// ************** VtokenVoting mock end *****************

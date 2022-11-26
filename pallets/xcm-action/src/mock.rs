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
#![cfg(test)]
#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use crate as xcm_action;
use bifrost_asset_registry::AssetIdMaps;
use bifrost_slp::{QueryId, QueryResponseManager};
use cumulus_primitives_core::ParaId;
use frame_support::{
	construct_runtime, ord_parameter_types,
	pallet_prelude::*,
	parameter_types,
	traits::{Everything, Nothing},
	PalletId,
};
use frame_system::EnsureSignedBy;
use hex_literal::hex;
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::{location::AbsoluteReserveProvider, parameter_type_with_key, MultiCurrency};
use sp_core::{blake2_256, H256};
use sp_runtime::{
	testing::Header,
	traits::{
		AccountIdConversion, BlakeTwo256, Convert, IdentityLookup, TrailingZeroInput,
		UniqueSaturatedInto,
	},
	AccountId32, SaturatedConversion,
};
use sp_std::vec;
pub use xcm::latest::prelude::*;
use xcm::{
	latest::{Junction, MultiLocation},
	opaque::latest::{
		Junction::Parachain,
		Junctions::{X1, X2},
		NetworkId,
	},
};
pub use xcm_builder::{
	AccountId32Aliases, AllowTopLevelPaidExecutionFrom, AllowUnpaidExecutionFrom,
	ChildParachainAsNative, ChildParachainConvertsVia, ChildSystemParachainAsSuperuser,
	CurrencyAdapter as XcmCurrencyAdapter, EnsureXcmOrigin, FixedRateOfFungible, FixedWeightBounds,
	IsConcrete, LocationInverter, NativeAsset, ParentAsSuperuser, ParentIsPreset,
	RelayChainAsNative, SiblingParachainAsNative, SiblingParachainConvertsVia,
	SignedAccountId32AsNative, SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit,
};
use xcm_executor::{Config, XcmExecutor};
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler, ZenlinkMultiAssets,
};

pub type Balance = u128;
pub type Amount = i128;
pub type BlockNumber = u64;
pub type AccountId = AccountId32;

pub const BNC: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
pub const DOT: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
pub const vDOT: CurrencyId = CurrencyId::VToken(TokenSymbol::DOT);
pub const KSM: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
pub const vKSM: CurrencyId = CurrencyId::VToken(TokenSymbol::KSM);
pub const MOVR: CurrencyId = CurrencyId::Token(TokenSymbol::MOVR);
pub const vMOVR: CurrencyId = CurrencyId::VToken(TokenSymbol::MOVR);
pub const ALICE: AccountId = AccountId32::new([0u8; 32]);
pub const BOB: AccountId = AccountId32::new([1u8; 32]);
pub const CHARLIE: AccountId = AccountId32::new([3u8; 32]);

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
  pub enum Test where
	Block = Block,
	NodeBlock = Block,
	UncheckedExtrinsic = UncheckedExtrinsic,
  {
	System: frame_system,
	Balances: pallet_balances,
	Tokens: orml_tokens,
	Currencies: orml_currencies,
	AssetRegistry: bifrost_asset_registry,
	Slp: bifrost_slp,
	VtokenMinting: bifrost_vtoken_minting,
	ZenlinkProtocol: zenlink_protocol,
	XTokens: orml_xtokens,
	XcmAction: xcm_action,
  }
);

// Pallet system configuration
parameter_types! {
  pub const BlockHashCount: u32 = 250;
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type BlockWeights = ();
	type BlockLength = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

// Pallet balances configuration
parameter_types! {
  pub const ExistentialDeposit: u128 = 1;
}

impl pallet_balances::Config for Test {
	type MaxReserves = ConstU32<2>;
	type ReserveIdentifier = [u8; 8];
	type MaxLocks = ();
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
}

pub type AdaptedBasicCurrency =
	orml_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Test {
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

// Pallet orml-tokens configuration
parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> u128 {
		0
	};
}
pub type ReserveIdentifier = [u8; 8];
impl orml_tokens::Config for Test {
	type Event = Event;
	type Balance = u128;
	type Amount = i128;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = ();
	type MaxLocks = ();
	type DustRemovalWhitelist = Nothing;
	type ReserveIdentifier = ReserveIdentifier;
	type MaxReserves = ConstU32<100_000>;
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}

// Pallet vtoken-minting configuration
parameter_types! {
	pub const MaximumUnlockIdOfUser: u32 = 10;
	pub const MaximumUnlockIdOfTimeUnit: u32 = 50;
	pub BifrostEntranceAccount: PalletId = PalletId(*b"bf/vtkin");
	pub BifrostExitAccount: PalletId = PalletId(*b"bf/vtout");
	pub BifrostFeeAccount: AccountId = hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();
	pub const RelayCurrencyId: CurrencyId = KSM;
}

ord_parameter_types! {
	pub const One: AccountId = ALICE;
}

impl bifrost_vtoken_minting::Config for Test {
	type Event = Event;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
	type MaximumUnlockIdOfUser = MaximumUnlockIdOfUser;
	type MaximumUnlockIdOfTimeUnit = MaximumUnlockIdOfTimeUnit;
	type EntranceAccount = BifrostEntranceAccount;
	type ExitAccount = BifrostExitAccount;
	type FeeAccount = BifrostFeeAccount;
	type BifrostSlp = Slp;
	type RelayChainToken = RelayCurrencyId;
	type CurrencyIdConversion = AssetIdMaps<Test>;
	type CurrencyIdRegister = AssetIdMaps<Test>;
	type WeightInfo = ();
	type OnRedeemSuccess = ();
}
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

type MultiAssets = ZenlinkMultiAssets<ZenlinkProtocol, Balances, LocalAssetAdaptor<Currencies>>;

parameter_types! {
	pub const ZenlinkPalletId: PalletId = PalletId(*b"/zenlink");
	pub const GetExchangeFee: (u32, u32) = (3, 1000);   // 0.3%
	pub const SelfParaId: u32 = 2001;
}

impl zenlink_protocol::Config for Test {
	type Event = Event;
	type MultiAssetsHandler = MultiAssets;
	type PalletId = ZenlinkPalletId;
	type SelfParaId = SelfParaId;

	type TargetChains = ();
	type XcmExecutor = ();
	type Conversion = ();
	type WeightInfo = ();
}

pub struct AccountIdToMultiLocation;
impl Convert<AccountId, MultiLocation> for AccountIdToMultiLocation {
	fn convert(account_id: AccountId) -> MultiLocation {
		MultiLocation::from(Junction::AccountId32 {
			network: NetworkId::Any,
			id: account_id.into(),
		})
	}
}

parameter_types! {
	pub const BaseXcmWeight: Weight = 100_000_000;
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsForTransfer: usize = 2;
}

pub type Barrier = AllowUnpaidExecutionFrom<Everything>;

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = ();
	// How to withdraw and deposit an asset.
	type AssetTransactor = ();
	type OriginConverter = ();
	type IsReserve = NativeAsset;
	type IsTeleporter = NativeAsset;
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = Barrier;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type Trader = ();
	type ResponseHandler = ();
	type AssetTrap = ();
	type AssetClaims = ();
	type SubscriptionService = ();
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		None
	};
}

parameter_types! {
	pub const UnitWeightCost: Weight = 1;
	pub Ancestry: MultiLocation = Here.into();
}

impl orml_xtokens::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = ();
	type AccountIdToMultiLocation = AccountIdToMultiLocation;
	type SelfLocation = ();
	type MinXcmFee = ParachainMinFee;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type MultiLocationsFilter = Everything;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type LocationInverter = LocationInverter<Ancestry>;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type ReserveProvider = AbsoluteReserveProvider;
}

ord_parameter_types! {
	pub const CouncilAccount: AccountId = AccountId::from([1u8; 32]);
}
impl bifrost_asset_registry::Config for Test {
	type Event = Event;
	type Currency = Balances;
	type RegisterOrigin = EnsureSignedBy<CouncilAccount, AccountId>;
	type WeightInfo = ();
}

pub struct SubAccountIndexMultiLocationConvertor;
impl Convert<(u16, CurrencyId), MultiLocation> for SubAccountIndexMultiLocationConvertor {
	fn convert((sub_account_index, currency_id): (u16, CurrencyId)) -> MultiLocation {
		match currency_id {
			CurrencyId::Token(TokenSymbol::MOVR) => MultiLocation::new(
				1,
				X2(
					Parachain(2023),
					Junction::AccountKey20 {
						network: NetworkId::Any,
						key: Slp::derivative_account_id_20(
							hex!["7369626cd1070000000000000000000000000000"].into(),
							sub_account_index,
						)
						.into(),
					},
				),
			),
			_ => MultiLocation::new(
				1,
				X1(Junction::AccountId32 {
					network: NetworkId::Any,
					id: Self::derivative_account_id(
						ParaId::from(2001u32).into_account_truncating(),
						sub_account_index,
					)
					.into(),
				}),
			),
		}
	}
}

// Mock Utility::derivative_account_id function.
impl SubAccountIndexMultiLocationConvertor {
	pub fn derivative_account_id(who: AccountId, index: u16) -> AccountId {
		let entropy = (b"modlpy/utilisuba", who, index).using_encoded(blake2_256);
		Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
			.expect("infinite length input; no invalid inputs for type; qed")
	}
}

pub struct ParachainId;
impl Get<ParaId> for ParachainId {
	fn get() -> ParaId {
		2001.into()
	}
}

parameter_types! {
	pub const MaxTypeEntryPerBlock: u32 = 10;
	pub const MaxRefundPerBlock: u32 = 10;
}

pub struct SubstrateResponseManager;
impl QueryResponseManager<QueryId, MultiLocation, u64> for SubstrateResponseManager {
	fn get_query_response_record(_query_id: QueryId) -> bool {
		Default::default()
	}
	fn create_query_record(_responder: &MultiLocation, _timeout: u64) -> u64 {
		Default::default()
	}
	fn remove_query_record(_query_id: QueryId) -> bool {
		Default::default()
	}
}

impl bifrost_slp::Config for Test {
	type Event = Event;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
	type WeightInfo = ();
	type VtokenMinting = VtokenMinting;
	type AccountConverter = SubAccountIndexMultiLocationConvertor;
	type ParachainId = ParachainId;
	type XcmRouter = ();
	type XcmExecutor = ();
	type SubstrateResponseManager = SubstrateResponseManager;
	type MaxTypeEntryPerBlock = MaxTypeEntryPerBlock;
	type MaxRefundPerBlock = MaxRefundPerBlock;
	type OnRefund = ();
	type ParachainStaking = ();
}

pub struct XcmFake;
impl Into<Result<cumulus_pallet_xcm::Origin, XcmFake>> for XcmFake {
	fn into(self) -> Result<cumulus_pallet_xcm::Origin, XcmFake> {
		todo!("please test via local-integration-tests")
	}
}
impl From<Origin> for XcmFake {
	fn from(_: Origin) -> Self {
		todo!("please test via local-integration-tests")
	}
}
impl SendXcm for XcmFake {
	fn send_xcm(
		_destination: impl Into<xcm::latest::MultiLocation>,
		_message: xcm::latest::Xcm<()>,
	) -> xcm::latest::SendResult {
		todo!("please test via local-integration-tests")
	}
}

// Pallet xcm-action configuration
parameter_types! {
	pub const XcmActionPalletId: PalletId = PalletId(*b"/xcmactn");
}

impl xcm_action::Config for Test {
	type Event = Event;
	type Origin = XcmFake;
	type MultiCurrency = Currencies;
	type DexOperator = ZenlinkProtocol;
	type VtokenMintingInterface = VtokenMinting;
	type XcmTransfer = XTokens;
	type CurrencyIdConvert = AssetIdMaps<Test>;
	type PalletId = XcmActionPalletId;
	type ParachainId = ParachainId;
	type WeightInfo = ();
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { endowed_accounts: vec![] }
	}
}

impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn one_hundred_for_alice_n_bob(self) -> Self {
		self.balances(vec![
			(ALICE, BNC, 10000),
			(BOB, BNC, 10000),
			(CHARLIE, BNC, 10000),
			(ALICE, DOT, 10000),
			(ALICE, vDOT, 40000),
			(ALICE, KSM, 30000),
			(ALICE, vKSM, 30000),
			(BOB, vKSM, 10000),
			(BOB, KSM, 10000000000),
			(BOB, MOVR, 1000000000000000000000),
		])
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub fn one_hundred_precision_for_each_currency_type_for_whitelist_account(self) -> Self {
		use frame_benchmarking::whitelisted_caller;
		let whitelist_caller: AccountId = whitelisted_caller();
		self.balances(vec![
			(whitelist_caller.clone(), BNC, 10000),
			(whitelist_caller.clone(), KSM, 10000),
			(whitelist_caller.clone(), vKSM, 10000),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		pallet_balances::GenesisConfig::<Test> {
			balances: self
				.endowed_accounts
				.clone()
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == BNC)
				.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Test> {
			balances: self
				.endowed_accounts
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id != BNC)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}

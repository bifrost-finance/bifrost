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
#![allow(non_upper_case_globals)]

use bifrost_asset_registry::AssetIdMaps;
use bifrost_primitives::{
	currency::{BNC, DOT, FIL, KSM, MOVR, VBNC, VFIL, VKSM, VMOVR},
	CurrencyId, CurrencyIdMapping, SlpxOperator, TokenSymbol,
};
use bifrost_runtime_common::{micro, milli};
use bifrost_slp::{QueryId, QueryResponseManager};
use bifrost_ve_minting::{Point, VeMintingInterface};
pub use cumulus_primitives_core::ParaId;
use frame_support::{
	derive_impl, ord_parameter_types,
	pallet_prelude::Get,
	parameter_types,
	traits::{Everything, Nothing},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use hex_literal::hex;
use orml_traits::{location::RelativeReserveProvider, parameter_type_with_key};
use sp_runtime::{
	traits::{ConstU32, IdentityLookup},
	AccountId32, BuildStorage, DispatchError, DispatchResult,
};
use xcm::{prelude::*, v3::Weight};
use xcm_builder::{FixedWeightBounds, FrameTransactionalProcessor};
use xcm_executor::XcmExecutor;

use crate as vtoken_minting;

pub type BlockNumber = u64;
pub type Amount = i128;
pub type Balance = u128;

pub type AccountId = AccountId32;

pub const ALICE: AccountId = AccountId32::new([0u8; 32]);
pub const BOB: AccountId = AccountId32::new([1u8; 32]);
pub const CHARLIE: AccountId = AccountId32::new([3u8; 32]);

frame_support::construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		Tokens: orml_tokens,
		XTokens: orml_xtokens,
		Balances: pallet_balances,
		Currencies: bifrost_currencies,
		VtokenMinting: vtoken_minting,
		Slp: bifrost_slp,
		AssetRegistry: bifrost_asset_registry,
		PolkadotXcm: pallet_xcm,
	}
);

type Block = frame_system::mocking::MockBlock<Runtime>;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type Block = Block;
	type Lookup = IdentityLookup<Self::AccountId>;
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
}

pub type AdaptedBasicCurrency =
	bifrost_currencies::BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Runtime {
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	// pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
	// pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const StableCurrencyId: CurrencyId = CurrencyId::Stable(TokenSymbol::KUSD);
	// pub SelfParaId: u32 = ParachainInfo::parachain_id().into();
	pub const PolkadotCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
}

impl pallet_balances::Config for Runtime {
	type AccountStore = frame_system::Pallet<Runtime>;
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
	type MaxFreezes = ConstU32<0>;
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		env_logger::try_init().unwrap_or(());

		log::debug!(
			"{:?}",currency_id
		);
		match currency_id {
			&BNC => 10 * milli::<Runtime>(NativeCurrencyId::get()),   // 0.01 BNC
			&KSM => 0,
			&VKSM => 0,
			&FIL => 0,
			&VFIL => 0,
			&MOVR => 1 * micro::<Runtime>(MOVR),	// MOVR has a decimals of 10e18
			&VMOVR => 1 * micro::<Runtime>(MOVR),	// MOVR has a decimals of 10e18
			&VBNC => 10 * milli::<Runtime>(NativeCurrencyId::get()),  // 0.01 BNC
			_ => AssetIdMaps::<Runtime>::get_currency_metadata(*currency_id)
				.map_or(Balance::max_value(), |metatata| metatata.minimal_balance)
		}
	};
}
impl orml_tokens::Config for Runtime {
	type Amount = i128;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = Nothing;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type CurrencyHooks = ();
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: Location| -> Option<u128> {
		Some(u128::MAX)
	};
}

parameter_types! {
	pub SelfRelativeLocation: Location = Location::here();
	pub const BaseXcmWeight: Weight = Weight::from_parts(1000_000_000u64, 0);
	pub const MaxAssetsForTransfer: usize = 2;
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = ();
	type AccountIdToLocation = ();
	type UniversalLocation = UniversalLocation;
	type SelfLocation = SelfRelativeLocation;
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

parameter_types! {
	pub const MaximumUnlockIdOfUser: u32 = 1_000;
	pub const MaximumUnlockIdOfTimeUnit: u32 = 1_000;
	pub const MaxLockRecords: u32 = 64;
	pub BifrostEntranceAccount: PalletId = PalletId(*b"bf/vtkin");
	pub BifrostExitAccount: PalletId = PalletId(*b"bf/vtout");
	pub IncentivePoolAccount: PalletId = PalletId(*b"bf/inpoo");
	pub BifrostFeeAccount: AccountId = hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();
}

ord_parameter_types! {
	pub const One: AccountId = ALICE;
	pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
}

impl vtoken_minting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
	type MaximumUnlockIdOfUser = MaximumUnlockIdOfUser;
	type MaximumUnlockIdOfTimeUnit = MaximumUnlockIdOfTimeUnit;
	type MaxLockRecords = MaxLockRecords;
	type EntranceAccount = BifrostEntranceAccount;
	type ExitAccount = BifrostExitAccount;
	type FeeAccount = BifrostFeeAccount;
	type RedeemFeeAccount = BifrostFeeAccount;
	type IncentivePoolAccount = IncentivePoolAccount;
	type BifrostSlp = Slp;
	type BifrostSlpx = SlpxInterface;
	type VeMinting = VeMinting;
	type RelayChainToken = RelayCurrencyId;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type CurrencyIdRegister = AssetIdMaps<Runtime>;
	type WeightInfo = ();
	type OnRedeemSuccess = ();
	type XcmTransfer = XTokens;
	type AstarParachainId = ConstU32<2007>;
	type MoonbeamParachainId = ConstU32<2023>;
	type HydradxParachainId = ConstU32<2034>;
	type MantaParachainId = ConstU32<2104>;
	type InterlayParachainId = ConstU32<2032>;
	type ChannelCommission = ();
	type AssetIdMaps = AssetIdMaps<Runtime>;
}

ord_parameter_types! {
	pub const CouncilAccount: AccountId = AccountId::from([1u8; 32]);
}
impl bifrost_asset_registry::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureSignedBy<CouncilAccount, AccountId>;
	type WeightInfo = ();
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
	pub const MaxLengthLimit: u32 = 100;
}

pub struct SubstrateResponseManager;
impl QueryResponseManager<QueryId, Location, u64, RuntimeCall> for SubstrateResponseManager {
	fn get_query_response_record(_query_id: QueryId) -> bool {
		Default::default()
	}
	fn create_query_record(
		_responder: Location,
		_call_back: Option<RuntimeCall>,
		_timeout: u64,
	) -> u64 {
		Default::default()
	}
	fn remove_query_record(_query_id: QueryId) -> bool {
		Default::default()
	}
}

pub struct SlpxInterface;
impl SlpxOperator<Balance> for SlpxInterface {
	fn get_moonbeam_transfer_to_fee() -> Balance {
		Default::default()
	}
}

pub const TREASURY_ACCOUNT: AccountId = AccountId32::new([9u8; 32]);
parameter_types! {
	pub const TreasuryAccount: AccountId32 = TREASURY_ACCOUNT;
}

impl bifrost_slp::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
	type WeightInfo = ();
	type VtokenMinting = VtokenMinting;
	type AccountConverter = ();
	type ParachainId = ParachainId;
	type SubstrateResponseManager = SubstrateResponseManager;
	type MaxTypeEntryPerBlock = MaxTypeEntryPerBlock;
	type MaxRefundPerBlock = MaxRefundPerBlock;
	type ParachainStaking = ();
	type XcmTransfer = XTokens;
	type MaxLengthLimit = MaxLengthLimit;
	type XcmWeightAndFeeHandler = ();
	type ChannelCommission = ();
	type StablePoolHandler = ();
	type AssetIdMaps = AssetIdMaps<Runtime>;
	type TreasuryAccount = TreasuryAccount;
}

parameter_types! {
	// One XCM operation is 200_000_000 XcmWeight, cross-chain transfer ~= 2x of transfer = 3_000_000_000
	pub UnitWeightCost: Weight = Weight::from_parts(200_000_000, 0);
	pub const MaxInstructions: u32 = 100;
	pub UniversalLocation: InteriorLocation = Parachain(2001).into();
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
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<Location> = Some(Parent.into());
}

impl pallet_xcm::Config for Runtime {
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
			(ALICE, BNC, 1000000000000000000000),
			(BOB, BNC, 1000000000000),
			(BOB, VKSM, 1000),
			(BOB, KSM, 1000000000000),
			(BOB, MOVR, 1000000000000000000000),
			(BOB, VFIL, 1000),
			(BOB, FIL, 100000000000000000000000),
			(CHARLIE, MOVR, 100000000000000000000000),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
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

		orml_tokens::GenesisConfig::<Runtime> {
			balances: self
				.endowed_accounts
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id != BNC)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		bifrost_asset_registry::GenesisConfig::<Runtime> {
			currency: vec![
				(DOT, 100_000_000, None),
				(KSM, 10_000_000, None),
				(BNC, 10_000_000, None),
				(FIL, 10_000_000, None),
			],
			vcurrency: vec![],
			vsbond: vec![],
			phantom: Default::default(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}

/// Run until a particular block.
pub fn run_to_block(n: BlockNumber) {
	use frame_support::traits::Hooks;
	while System::block_number() <= n {
		VtokenMinting::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		VtokenMinting::on_initialize(System::block_number());
	}
}

use bifrost_primitives::PoolId;
use bifrost_ve_minting::IncentiveConfig;
// Mock VeMinting Struct
pub struct VeMinting;
impl VeMintingInterface<AccountId, CurrencyId, Balance, BlockNumber> for VeMinting {
	fn balance_of(_addr: &AccountId, _time: Option<BlockNumber>) -> Result<Balance, DispatchError> {
		Ok(100)
	}

	fn total_supply(_t: BlockNumber) -> Result<Balance, DispatchError> {
		Ok(10000)
	}

	fn increase_amount_inner(_who: &AccountId, _position: u128, _value: Balance) -> DispatchResult {
		Ok(())
	}

	fn deposit_for(_who: &AccountId, _position: u128, _value: Balance) -> DispatchResult {
		Ok(())
	}

	fn withdraw_inner(_who: &AccountId, _position: u128) -> DispatchResult {
		Ok(())
	}

	fn supply_at(_: Point<u128, u64>, _: u64) -> Result<u128, sp_runtime::DispatchError> {
		todo!()
	}

	fn find_block_epoch(_: u64, _: sp_core::U256) -> sp_core::U256 {
		todo!()
	}

	fn create_lock_inner(
		_: &sp_runtime::AccountId32,
		_: u128,
		_: u64,
	) -> Result<(), sp_runtime::DispatchError> {
		todo!()
	}

	fn increase_unlock_time_inner(
		_: &sp_runtime::AccountId32,
		_: u128,
		_: u64,
	) -> Result<(), sp_runtime::DispatchError> {
		todo!()
	}

	fn auto_notify_reward(
		_: u32,
		_: u64,
		_: Vec<(CurrencyId, Balance)>,
	) -> Result<(), sp_runtime::DispatchError> {
		todo!()
	}

	fn update_reward(
		_pool_id: PoolId,
		_addr: Option<&AccountId>,
		_share_info: Option<(Balance, Balance)>,
	) -> DispatchResult {
		Ok(())
	}

	fn get_rewards(
		_pool_id: PoolId,
		_addr: &AccountId,
		_share_info: Option<(Balance, Balance)>,
	) -> DispatchResult {
		Ok(())
	}

	fn set_incentive(
		_pool_id: PoolId,
		_rewards_duration: Option<BlockNumber>,
		_incentive_controller: Option<AccountId>,
	) {
	}
	fn add_reward(
		_addr: &AccountId,
		_conf: &mut IncentiveConfig<CurrencyId, Balance, BlockNumber, AccountId>,
		_rewards: &Vec<(CurrencyId, Balance)>,
		_remaining: Balance,
	) -> DispatchResult {
		Ok(())
	}
	fn notify_reward(
		_pool_id: u32,
		_addr: &Option<AccountId>,
		_rewards: Vec<(CurrencyId, Balance)>,
	) -> DispatchResult {
		Ok(())
	}
}

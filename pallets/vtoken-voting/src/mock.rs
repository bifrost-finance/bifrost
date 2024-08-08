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

use crate as vtoken_voting;
use crate::{BalanceOf, DerivativeAccountHandler, DerivativeIndex, DispatchResult};
use bifrost_primitives::{
	currency::{KSM, VBNC, VKSM},
	traits::XcmDestWeightAndFeeHandler,
	CurrencyId, DoNothingRouter, TokenSymbol, VTokenSupplyProvider, XcmOperationType,
};
use cumulus_primitives_core::ParaId;
use frame_support::{
	derive_impl, ord_parameter_types,
	pallet_prelude::Weight,
	parameter_types,
	traits::{Everything, Get, Nothing},
	weights::RuntimeDbWeight,
};
use frame_system::EnsureRoot;
use pallet_xcm::EnsureResponse;
use sp_runtime::{
	traits::{BlockNumberProvider, ConstU32, IdentityLookup},
	BuildStorage, Perbill,
};
use xcm::{prelude::*, v3::MultiLocation};
use xcm_builder::{FixedWeightBounds, FrameTransactionalProcessor};
use xcm_executor::XcmExecutor;

pub type BlockNumber = u64;
pub type Amount = i128;
pub type Balance = u128;
pub type AccountId = u64;

type Block = frame_system::mocking::MockBlock<Runtime>;

pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const CONTROLLER: u64 = 1000;

frame_support::construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		Tokens: orml_tokens,
		Balances: pallet_balances,
		Currencies: bifrost_currencies,
		PolkadotXcm: pallet_xcm,
		VtokenVoting: vtoken_voting,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const DbWeight: RuntimeDbWeight = RuntimeDbWeight { read: 1, write: 2 };
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
		match currency_id {
			&KSM => 0,
			&VBNC => 0,
			&VKSM => 0,
			_ => 0,
		}
	};
}
impl orml_tokens::Config for Runtime {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = Nothing;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ConstU32<100>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type CurrencyHooks = ();
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
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = ();
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
	type XcmRouter = DoNothingRouter;
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

ord_parameter_types! {
	pub const Controller: u64 = CONTROLLER;
	pub const QueryTimeout: BlockNumber = 100;
}

pub struct ParachainId;
impl Get<ParaId> for ParachainId {
	fn get() -> ParaId {
		2001u32.into()
	}
}

pub struct XcmDestWeightAndFee;
impl XcmDestWeightAndFeeHandler<CurrencyId, BalanceOf<Runtime>> for XcmDestWeightAndFee {
	fn get_operation_weight_and_fee(
		_token: CurrencyId,
		_operation: XcmOperationType,
	) -> Option<(Weight, Balance)> {
		Some((Weight::from_parts(4000000000, 100000), 4000000000u32.into()))
	}

	fn set_xcm_dest_weight_and_fee(
		_currency_id: CurrencyId,
		_operation: XcmOperationType,
		_weight_and_fee: Option<(Weight, Balance)>,
	) -> DispatchResult {
		Ok(())
	}
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
		Some(xcm::v3::Parent.into())
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
	pub static ReferendumCheckInterval: BlockNumber = 1;
}

pub struct RelaychainDataProvider;

impl RelaychainDataProvider {
	pub fn set_block_number(block: BlockNumber) {
		RelaychainBlockNumber::set(block);
	}
}

impl BlockNumberProvider for RelaychainDataProvider {
	type BlockNumber = BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		RelaychainBlockNumber::get()
	}
}

parameter_types! {
	// modify TokenSupply to be twice that of VTokenSupply, making the exchange rate for vtokenming 1:2
	pub static VTokenSupply: Balance = u64::MAX.checked_div(2u64).unwrap().into();
	pub static TokenSupply: Balance = u64::MAX.into();
}

pub struct SimpleVTokenSupplyProvider;

impl SimpleVTokenSupplyProvider {
	pub fn set_vtoken_supply(supply: Balance) {
		VTokenSupply::set(supply);
	}

	pub fn set_token_supply(supply: Balance) {
		TokenSupply::set(supply);
	}
}

impl VTokenSupplyProvider<CurrencyId, Balance> for SimpleVTokenSupplyProvider {
	fn get_vtoken_supply(_: CurrencyId) -> Option<Balance> {
		Some(VTokenSupply::get())
	}

	fn get_token_supply(_: CurrencyId) -> Option<Balance> {
		Some(TokenSupply::get())
	}
}

impl vtoken_voting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureRoot<AccountId>;
	type ResponseOrigin = EnsureResponse<Everything>;
	type XcmDestWeightAndFee = XcmDestWeightAndFee;
	type DerivativeAccount = DerivativeAccount;
	type RelaychainBlockNumberProvider = RelaychainDataProvider;
	type VTokenSupplyProvider = SimpleVTokenSupplyProvider;
	type MaxVotes = ConstU32<256>;
	type ParachainId = ParachainId;
	type QueryTimeout = QueryTimeout;
	type ReferendumCheckInterval = ReferendumCheckInterval;
	type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
	pallet_balances::GenesisConfig::<Runtime> {
		balances: vec![(ALICE, 10), (BOB, 20), (CHARLIE, 30)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	orml_tokens::GenesisConfig::<Runtime> {
		balances: vec![
			(1, VKSM, 10),
			(2, VKSM, 20),
			(3, VKSM, 30),
			(4, VKSM, 40),
			(5, VKSM, 50),
			(1, VBNC, 10),
			(2, VBNC, 20),
			(3, VBNC, 30),
			(4, VBNC, 40),
			(5, VBNC, 50),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	vtoken_voting::GenesisConfig::<Runtime> {
		delegators: vec![
			(VKSM, vec![0, 1, 2, 3, 4, 5, 10, 11, 15, 20, 21]),
			(VBNC, vec![0, 1, 2, 3, 4, 5, 10, 11, 15, 20, 21]),
		],
		undeciding_timeouts: vec![(VKSM, 100), (VBNC, 100)],
		vote_cap_ratio: vec![(VKSM, Perbill::from_percent(10)), (VBNC, Perbill::from_percent(10))],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

#[cfg(feature = "runtime-benchmarks")]
pub fn new_test_ext_benchmark() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Runtime>::default()
		.build_storage()
		.unwrap()
		.into()
}

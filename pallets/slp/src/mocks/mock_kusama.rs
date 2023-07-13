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

// Ensure we're `no_std` when compiling for Wasm.

#![cfg(test)]

use bifrost_asset_registry::AssetIdMaps;
// use parachain_staking::ParachainStakingInterface;
use codec::{Decode, Encode};
pub use cumulus_primitives_core::ParaId;
use frame_support::{
	construct_runtime, ord_parameter_types,
	pallet_prelude::Get,
	parameter_types,
	traits::{Everything, GenesisBuild, Nothing},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use hex_literal::hex;
use node_primitives::{
	currency::{BNC, KSM, VKSM},
	Amount, Balance, CurrencyId, SlpxOperator, TokenSymbol,
};
use orml_traits::{location::RelativeReserveProvider, parameter_type_with_key};
use sp_core::{bounded::BoundedVec, hashing::blake2_256, ConstU32, H256};
pub use sp_runtime::{testing::Header, Perbill};
use sp_runtime::{
	traits::{AccountIdConversion, Convert, TrailingZeroInput},
	AccountId32, Percent,
};
use sp_std::{boxed::Box, vec::Vec};
use xcm::v3::{prelude::*, Weight};
use xcm_builder::FixedWeightBounds;
use xcm_executor::XcmExecutor;

use crate as bifrost_slp;
use crate::{Config, QueryResponseManager};

pub type AccountId = AccountId32;
pub type Block = frame_system::mocking::MockBlock<Runtime>;
pub type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;

pub const ALICE: AccountId = AccountId32::new([1u8; 32]);
pub const BOB: AccountId = AccountId32::new([2u8; 32]);
pub const CHARLIE: AccountId = AccountId32::new([3u8; 32]);
pub const DAVE: AccountId = AccountId32::new([4u8; 32]);
pub const EDDIE: AccountId = AccountId32::new([5u8; 32]);

construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
		Indices: pallet_indices::{Pallet, Call, Storage, Event<T>} = 2,
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 10,
		Currencies: orml_currencies::{Pallet, Call} = 72,
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>} = 71,
		XTokens: orml_xtokens::{Pallet, Call, Event<T>} = 70,
		Slp: bifrost_slp::{Pallet, Call, Storage, Event<T>} = 116,
		VtokenMinting: bifrost_vtoken_minting::{Pallet, Call, Storage, Event<T>} = 115,
		AssetRegistry: bifrost_asset_registry::{Pallet, Call, Event<T>, Storage} = 114,
		ParachainStaking: parachain_staking::{Pallet, Call, Storage, Event<T>} = 25,
		Utility: pallet_utility::{Pallet, Call, Event} = 50,
		PolkadotXcm: pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin, Config},
	}
);

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = BNC;
	pub const RelayCurrencyId: CurrencyId = KSM;
}

impl pallet_utility::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = u64;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = Indices;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = frame_support::traits::Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const Deposit: u128 = 1_000_000_000_000;
}

impl pallet_indices::Config for Runtime {
	type AccountIndex = u32;
	type Currency = Balances;
	type Deposit = Deposit;
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1;
	pub const MaxLocks: u32 = 999_999;
	pub const MaxReserves: u32 = 999_999;
}

impl pallet_balances::Config for Runtime {
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
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type HoldIdentifier = ();
	type FreezeIdentifier = ();
	type MaxHolds = ConstU32<0>;
	type MaxFreezes = ConstU32<0>;
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

impl orml_tokens::Config for Runtime {
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

pub type BifrostToken = orml_currencies::BasicCurrencyAdapter<Runtime, Balances, Amount, u64>;

impl orml_currencies::Config for Runtime {
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BifrostToken;
	type WeightInfo = ();
}

parameter_type_with_key! {
	pub ParachainMinFee: |_location: MultiLocation| -> Option<u128> {
		Some(u128::MAX)
	};
}

parameter_types! {
	pub SelfRelativeLocation: MultiLocation = MultiLocation::here();
	pub const BaseXcmWeight: Weight = Weight::from_parts(1000_000_000u64, 0);
	pub const MaxAssetsForTransfer: usize = 2;
}

impl orml_xtokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type CurrencyIdConvert = ();
	type AccountIdToMultiLocation = ();
	type UniversalLocation = UniversalLocation;
	type SelfLocation = SelfRelativeLocation;
	type XcmExecutor = XcmExecutor<XcmConfig>;
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type BaseXcmWeight = BaseXcmWeight;
	type MaxAssetsForTransfer = MaxAssetsForTransfer;
	type MinXcmFee = ParachainMinFee;
	type MultiLocationsFilter = Everything;
	type ReserveProvider = RelativeReserveProvider;
}

parameter_types! {
	pub const MaximumUnlockIdOfUser: u32 = 10;
	pub const MaximumUnlockIdOfTimeUnit: u32 = 50;
	pub BifrostEntranceAccount: PalletId = PalletId(*b"bf/vtkin");
	pub BifrostExitAccount: PalletId = PalletId(*b"bf/vtout");
	pub BifrostFeeAccount: AccountId = hex!["e4da05f08e89bf6c43260d96f26fffcfc7deae5b465da08669a9d008e64c2c63"].into();
}

pub struct SlpxInterface;
impl SlpxOperator<Balance> for SlpxInterface {
	fn get_moonbeam_transfer_to_fee() -> Balance {
		Default::default()
	}
}

impl bifrost_vtoken_minting::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
	type MaximumUnlockIdOfUser = MaximumUnlockIdOfUser;
	type MaximumUnlockIdOfTimeUnit = MaximumUnlockIdOfTimeUnit;
	type EntranceAccount = BifrostEntranceAccount;
	type ExitAccount = BifrostExitAccount;
	type FeeAccount = BifrostFeeAccount;
	type RelayChainToken = RelayCurrencyId;
	type CurrencyIdConversion = AssetIdMaps<Runtime>;
	type CurrencyIdRegister = AssetIdMaps<Runtime>;
	type BifrostSlp = Slp;
	type BifrostSlpx = SlpxInterface;
	type WeightInfo = ();
	type OnRedeemSuccess = ();
	type XcmTransfer = XTokens;
	type AstarParachainId = ConstU32<2007>;
	type MoonbeamParachainId = ConstU32<2023>;
	type HydradxParachainId = ConstU32<2034>;
}

parameter_types! {
	pub const MinBlocksPerRound: u32 = 3;
	pub const DefaultBlocksPerRound: u32 = 5;
	pub const LeaveCandidatesDelay: u32 = 2;
	pub const CandidateBondLessDelay: u32 = 2;
	pub const LeaveDelegatorsDelay: u32 = 2;
	pub const RevokeDelegationDelay: u32 = 2;
	pub const DelegationBondLessDelay: u32 = 2;
	pub const RewardPaymentDelay: u32 = 2;
	pub const MinSelectedCandidates: u32 = 5;
	pub const MaxTopDelegationsPerCandidate: u32 = 4;
	pub const MaxBottomDelegationsPerCandidate: u32 = 4;
	pub const MaxDelegationsPerDelegator: u32 = 4;
	pub const DefaultCollatorCommission: Perbill = Perbill::from_percent(20);
	pub const DefaultParachainBondReservePercent: Percent = Percent::from_percent(30);
	pub const MinCollatorStk: u128 = 10;
	pub const MinDelegatorStk: u128 = 5;
	pub const MinDelegation: u128 = 3;
	pub AllowInflation: bool = true;
	pub PaymentInRound: u128 = 10;
	pub const ParachainStakingPalletId: PalletId = PalletId(*b"bf/stake");
	pub ToMigrateInvulnables: Vec<AccountId> = vec![AccountId32::new([1u8; 32])];
	pub InitSeedStk: u128 = 10;
}
impl parachain_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MonetaryGovernanceOrigin = frame_system::EnsureRoot<AccountId>;
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
	type MinCandidateStk = MinCollatorStk;
	type MinDelegatorStk = MinDelegatorStk;
	type MinDelegation = MinDelegation;
	type OnCollatorPayout = ();
	type OnNewRound = ();
	type WeightInfo = ();
	type AllowInflation = AllowInflation;
	type PaymentInRound = PaymentInRound;
	type PalletId = ParachainStakingPalletId;
	type ToMigrateInvulnables = ToMigrateInvulnables;
	type InitSeedStk = InitSeedStk;
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

ord_parameter_types! {
	pub const One: AccountId = AccountId32::new([1u8; 32]);
}

parameter_types! {
	pub BifrostParachainAccountId20: [u8; 20] = hex_literal::hex!["7369626cd1070000000000000000000000000000"].into();
}

pub struct SubAccountIndexMultiLocationConvertor;
impl Convert<(u16, CurrencyId), MultiLocation> for SubAccountIndexMultiLocationConvertor {
	fn convert((sub_account_index, currency_id): (u16, CurrencyId)) -> MultiLocation {
		match currency_id {
			// AccountKey20 format of Bifrost sibling para account
			CurrencyId::Token(TokenSymbol::MOVR) => MultiLocation::new(
				1,
				X2(
					Parachain(2023),
					AccountKey20 {
						network: None,
						key: Slp::derivative_account_id_20(
							hex!["7369626cd1070000000000000000000000000000"].into(),
							sub_account_index,
						)
						.into(),
					},
				),
			),
			// Only relay chain use the Bifrost para account with "para"
			CurrencyId::Token(TokenSymbol::KSM) => MultiLocation::new(
				1,
				X1(Junction::AccountId32 {
					network: None,
					id: Self::derivative_account_id(
						ParaId::from(2001u32).into_account_truncating(),
						sub_account_index,
					)
					.into(),
				}),
			),
			// Bifrost Kusama Native token
			CurrencyId::Native(TokenSymbol::BNC) => MultiLocation::new(
				0,
				X1(Junction::AccountId32 {
					network: None,
					id: Self::derivative_account_id(
						polkadot_parachain::primitives::Sibling::from(2001u32)
							.into_account_truncating(),
						sub_account_index,
					)
					.into(),
				}),
			),
			// Other sibling chains use the Bifrost para account with "sibl"
			_ => {
				// get parachain id
				if let Some(location) = BifrostCurrencyIdConvert::convert(currency_id) {
					if let Some(Parachain(para_id)) = location.interior().first() {
						MultiLocation::new(
							1,
							X2(
								Parachain(*para_id),
								Junction::AccountId32 {
									network: None,
									id: Self::derivative_account_id(
										polkadot_parachain::primitives::Sibling::from(2001u32)
											.into_account_truncating(),
										sub_account_index,
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
	pub const MaxLengthLimit: u32 = 100;
}

pub struct BifrostCurrencyIdConvert;
impl Convert<CurrencyId, Option<MultiLocation>> for BifrostCurrencyIdConvert {
	fn convert(id: CurrencyId) -> Option<MultiLocation> {
		use CurrencyId::*;
		use TokenSymbol::*;

		match id {
			Token(MOVR) => Some(MultiLocation::new(1, X2(Parachain(2023), PalletInstance(10)))),
			Token(KSM) => Some(MultiLocation::parent()),
			Native(BNC) => Some(MultiLocation::new(
				0,
				X1(Junction::from(BoundedVec::try_from("0x0001".encode()).unwrap())),
			)),
			Token(PHA) => Some(MultiLocation::new(1, X1(Parachain(2004)))),
			_ => None,
		}
	}
}

pub struct SubstrateResponseManager;
impl QueryResponseManager<QueryId, MultiLocation, u64, RuntimeCall> for SubstrateResponseManager {
	fn get_query_response_record(_query_id: QueryId) -> bool {
		Default::default()
	}
	fn create_query_record(
		_responder: &MultiLocation,
		_call_back: Option<RuntimeCall>,
		_timeout: u64,
	) -> u64 {
		Default::default()
	}
	fn remove_query_record(_query_id: QueryId) -> bool {
		Default::default()
	}
}

impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type MultiCurrency = Currencies;
	type ControlOrigin = EnsureSignedBy<One, AccountId>;
	type WeightInfo = ();
	type VtokenMinting = VtokenMinting;
	type BifrostSlpx = SlpxInterface;
	type AccountConverter = SubAccountIndexMultiLocationConvertor;
	type ParachainId = ParachainId;
	type XcmRouter = ();
	type XcmExecutor = ();
	type SubstrateResponseManager = SubstrateResponseManager;
	type MaxTypeEntryPerBlock = MaxTypeEntryPerBlock;
	type MaxRefundPerBlock = MaxRefundPerBlock;
	type OnRefund = ();
	type ParachainStaking = ParachainStaking;
	type XcmTransfer = XTokens;
	type MaxLengthLimit = MaxLengthLimit;
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
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub ReachableDest: Option<MultiLocation> = Some(Parent.into());
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
	type WeightInfo = pallet_xcm::TestWeightInfo; // TODO: config after polkadot impl WeightInfo for ()
	#[cfg(feature = "runtime-benchmarks")]
	type ReachableDest = ReachableDest;
	type AdminOrigin = EnsureRoot<AccountId>;
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

	pub fn one_hundred_for_alice(self) -> Self {
		self.balances(vec![(ALICE, BNC, 100), (ALICE, KSM, 100), (ALICE, VKSM, 100)])
	}

	#[cfg(feature = "runtime-benchmarks")]
	pub fn one_hundred_precision_for_each_currency_type_for_whitelist_account(self) -> Self {
		use frame_benchmarking::whitelisted_caller;
		use sp_runtime::traits::AccountIdConversion;
		let whitelist_caller: AccountId = whitelisted_caller();
		let pool_account: AccountId = PalletId(*b"lighten#").into_account_truncating();

		self.balances(vec![
			(whitelist_caller.clone(), KSM, 100_000_000_000_000),
			(whitelist_caller.clone(), VKSM, 100_000_000_000_000),
			(pool_account.clone(), KSM, 100_000_000_000_000),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Runtime>().unwrap();

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

		t.into()
	}
}

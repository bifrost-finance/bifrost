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

use cumulus_primitives_core::ParaId as Pid;
use std::convert::TryInto;

use bifrost_asset_registry::AssetIdMaps;
#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::whitelisted_caller;
use frame_support::{
	ord_parameter_types, parameter_types,
	sp_runtime::{DispatchError, DispatchResult},
	sp_std::marker::PhantomData,
	traits::{Contains, Get, Nothing},
	weights::{ConstantMultiplier, IdentityFee},
	PalletId,
};
use frame_system as system;
use frame_system::{EnsureRoot, EnsureSignedBy};
use node_primitives::{CurrencyId, MessageId, ParaId, TokenSymbol};
use orml_traits::MultiCurrency;
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_runtime::{
	generic,
	testing::Header,
	traits::{AccountIdConversion, BlakeTwo256, IdentityLookup, UniqueSaturatedInto},
	AccountId32, SaturatedConversion,
};
use xcm_interface::traits::XcmHelper;
use zenlink_protocol::{
	AssetId as ZenlinkAssetId, AssetIdConverter, LocalAssetHandler, PairLpGenerate,
	ZenlinkMultiAssets,
};

use super::*;
use crate as flexible_fee;
#[allow(unused_imports)]
use crate::{
	fee_dealer::FixedCurrencyFeeRate,
	misc_fees::{ExtraFeeMatcher, MiscFeeHandler, NameGetter},
};

pub type AccountId = AccountId32;
pub type BlockNumber = u32;
pub type Amount = i128;

pub const TREASURY_ACCOUNT: AccountId32 = AccountId32::new([9u8; 32]);

pub type Balance = u64;
pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, u64, RuntimeCall, ()>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: system::{Pallet, Call, Storage, Event<T>},
		Tokens: orml_tokens::{Pallet, Storage, Event<T>},
		Balances: balances::{Pallet, Call, Storage, Event<T>},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Event<T>},
		FlexibleFee: flexible_fee::{Pallet, Call, Storage,Event<T>},
		ZenlinkProtocol: zenlink_protocol::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Storage},
		Salp: bifrost_salp::{Pallet, Call, Storage, Event<T>},
		AssetRegistry: bifrost_asset_registry::{Pallet, Call, Storage, Event<T>},
	}
);

ord_parameter_types! {
	pub const One: AccountId = ALICE;
}

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
	type AccountData = balances::AccountData<u64>;
	type AccountId = AccountId;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockNumber = u32;
	type BlockWeights = ();
	type RuntimeCall = RuntimeCall;
	type DbWeight = ();
	type RuntimeEvent = RuntimeEvent;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type Index = u64;
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

impl balances::Config for Test {
	type AccountStore = System;
	type Balance = u64;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
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

// Aggregate name getter to get fee names if the call needs to pay extra fees.
// If any call need to pay extra fees, it should be added as an item here.
// Used together with AggregateExtraFeeFilter below.
pub struct FeeNameGetter;
impl NameGetter<RuntimeCall> for FeeNameGetter {
	fn get_name(c: &RuntimeCall) -> ExtraFeeName {
		match *c {
			RuntimeCall::Salp(bifrost_salp::Call::contribute { .. }) =>
				ExtraFeeName::SalpContribute,
			_ => ExtraFeeName::NoExtraFee,
		}
	}
}

// Aggregate filter to filter if the call needs to pay extra fees
// If any call need to pay extra fees, it should be added as an item here.
pub struct AggregateExtraFeeFilter;
impl Contains<RuntimeCall> for AggregateExtraFeeFilter {
	fn contains(c: &RuntimeCall) -> bool {
		match *c {
			RuntimeCall::Salp(bifrost_salp::Call::contribute { .. }) => true,
			_ => false,
		}
	}
}

pub struct ContributeFeeFilter;
impl Contains<RuntimeCall> for ContributeFeeFilter {
	fn contains(c: &RuntimeCall) -> bool {
		match *c {
			RuntimeCall::Salp(bifrost_salp::Call::contribute { .. }) => true,
			_ => false,
		}
	}
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
	pub const AlternativeFeeCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const AltFeeCurrencyExchangeRate: (u32, u32) = (1, 100);
	pub const TreasuryAccount: AccountId32 = TREASURY_ACCOUNT;
	pub const SalpContributeFee: Balance = 100_000_000;
}

impl crate::Config for Test {
	type Currency = Balances;
	type DexOperator = ZenlinkProtocol;
	type FeeDealer = FlexibleFee;
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type TreasuryAccount = TreasuryAccount;
	type NativeCurrencyId = NativeCurrencyId;
	type AlternativeFeeCurrencyId = AlternativeFeeCurrencyId;
	type AltFeeCurrencyExchangeRate = AltFeeCurrencyExchangeRate;
	type OnUnbalanced = ();
	type WeightInfo = ();
	type ExtraFeeMatcher = ExtraFeeMatcher<Test, FeeNameGetter, AggregateExtraFeeFilter>;
	type MiscFeeHandler =
		MiscFeeHandler<Test, AlternativeFeeCurrencyId, SalpContributeFee, ContributeFeeFilter>;
	type ParachainId = ParaInfo;
}

pub struct ParaInfo;
impl Get<Pid> for ParaInfo {
	fn get() -> Pid {
		Pid::from(2001)
	}
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
}

pub type AdaptedBasicCurrency =
	orml_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Test {
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
	type XcmExecutor = ();
	type WeightInfo = ();
	type AssetId = ZenlinkAssetId;
	type LpGenerate = PairLpGenerate<Self>;
	type AccountIdConverter = ();
	type AssetIdConverter = AssetIdConverter;
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

#[cfg(feature = "runtime-benchmarks")]
pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, CurrencyId, Balance)>,
}

#[cfg(feature = "runtime-benchmarks")]
impl Default for ExtBuilder {
	fn default() -> Self {
		Self { endowed_accounts: vec![] }
	}
}

#[cfg(feature = "runtime-benchmarks")]
impl ExtBuilder {
	pub fn balances(mut self, endowed_accounts: Vec<(AccountId32, CurrencyId, Balance)>) -> Self {
		self.endowed_accounts = endowed_accounts;
		self
	}

	pub fn one_hundred_precision_for_each_currency_type_for_whitelist_account(self) -> Self {
		let whitelist_caller: AccountId32 = whitelisted_caller();
		let c0 = CurrencyId::Token(TokenSymbol::try_from(0u8).unwrap_or_default());
		let c1 = CurrencyId::Token(TokenSymbol::try_from(1u8).unwrap_or_default());
		let c2 = CurrencyId::Token(TokenSymbol::try_from(2u8).unwrap_or_default());
		let c3 = CurrencyId::Token(TokenSymbol::try_from(3u8).unwrap_or_default());
		let c4 = CurrencyId::Token(TokenSymbol::try_from(4u8).unwrap_or_default());
		let c5 = CurrencyId::Token(TokenSymbol::try_from(5u8).unwrap_or_default());

		self.balances(vec![
			(whitelist_caller.clone(), c0, 100_000_000_000_000),
			(whitelist_caller.clone(), c1, 100_000_000_000_000),
			(whitelist_caller.clone(), c2, 100_000_000_000_000),
			(whitelist_caller.clone(), c3, 100_000_000_000_000),
			(whitelist_caller.clone(), c4, 100_000_000_000_000),
			(whitelist_caller.clone(), c5, 100_000_000_000_000),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

		orml_tokens::GenesisConfig::<Test> { balances: self.endowed_accounts }
			.assimilate_storage(&mut t)
			.unwrap();

		t.into()
	}
}

// Build genesis storage according to the mock runtime.
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}

//************** Salp mock start *****************

// To control the result returned by `MockXcmExecutor`
pub(crate) static mut MOCK_XCM_RESULT: (bool, bool) = (true, true);

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

// Mock XcmExecutor
pub struct MockXcmExecutor;

impl XcmHelper<AccountIdOf<Test>, crate::pallet::PalletBalanceOf<Test>> for MockXcmExecutor {
	fn contribute(_index: ParaId, _value: Balance) -> Result<MessageId, DispatchError> {
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
}

impl bifrost_salp::Config for Test {
	type BancorPool = ();
	type RuntimeEvent = RuntimeEvent;
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
	type WeightInfo = ();
	type EnsureConfirmAsGovernance = EnsureRoot<AccountId>;
	type XcmInterface = MockXcmExecutor;
	type TreasuryAccount = TreasuryAccount;
	type BuybackPalletId = BuybackPalletId;
	type DexOperator = ZenlinkProtocol;
	type CurrencyIdConversion = AssetIdMaps<Test>;
	type CurrencyIdRegister = AssetIdMaps<Test>;
	type ParachainId = ParaInfo;
}

//************** Salp mock end *****************

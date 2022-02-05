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

use std::convert::TryInto;

// pub use polkadot_parachain::primitives::Id;
pub use cumulus_primitives_core::ParaId;
#[cfg(feature = "runtime-benchmarks")]
use frame_benchmarking::whitelisted_caller;
use frame_support::{
	parameter_types,
	sp_runtime::{DispatchError, DispatchResult},
	sp_std::marker::PhantomData,
	traits::{Contains, EnsureOrigin, Nothing},
	weights::{
		constants::ExtrinsicBaseWeight, IdentityFee, Weight, WeightToFeeCoefficients,
		WeightToFeePolynomial,
	},
	PalletId,
};
use frame_system as system;
use frame_system::RawOrigin;
use node_primitives::{
	CurrencyId, MessageId, ParachainTransactProxyType, ParachainTransactType, TokenSymbol,
	TransferOriginType, XcmBaseWeight,
};
use orml_traits::{MultiCurrency, XcmTransfer};
use smallvec::smallvec;
use sp_arithmetic::Percent;
use sp_core::H256;
use sp_runtime::{
	generic,
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup, UniqueSaturatedInto},
	AccountId32, Perbill, SaturatedConversion,
};
use sp_std::cell::RefCell;
use xcm::{latest::prelude::*, DoubleEncoded};
use xcm_support::BifrostXcmExecutor;
use zenlink_protocol::{AssetId as ZenlinkAssetId, LocalAssetHandler, ZenlinkMultiAssets};

use super::*;
use crate as flexible_fee;
// use node_primitives::Balance;
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
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, u64, Call, ()>;

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
		// TransactionPayment: pallet_transaction_payment::{Module, Storage},
		FlexibleFee: flexible_fee::{Pallet, Call, Storage,Event<T>},
		ZenlinkProtocol: zenlink_protocol::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Storage, Event<T>},
		Salp: bifrost_salp::{Pallet, Call, Storage, Event<T>},
	}
);

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
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type Index = u64;
	// needs to be u128 against u64, otherwise the account address will be half cut.
	type Lookup = IdentityLookup<Self::AccountId>;
	type OnKilledAccount = ();
	type OnNewAccount = ();
	type OnSetCode = ();
	type Origin = Origin;
	type PalletInfo = PalletInfo;
	type SS58Prefix = ();
	type SystemWeightInfo = ();
	type Version = ();
}

thread_local! {
	static WEIGHT_TO_FEE: RefCell<u64> = RefCell::new(1);
}

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		let p = 1_000_000_000_000 / 30_000; // RELAY_CENTS
		let q = 10 * Balance::from(ExtrinsicBaseWeight::get()); // ExtrinsicBaseWeight = 125 * 1_000_000_000_000 / 1000 / 1000 = 125_000_000
		smallvec![frame_support::weights::WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(p % q, q),
			coeff_integer: p / q,
		}]
	}
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
	pub const OperationalFeeMultiplier: u8 = 5;
}

impl pallet_transaction_payment::Config for Test {
	type FeeMultiplierUpdate = ();
	type OnChargeTransaction = FlexibleFee;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl balances::Config for Test {
	type AccountStore = System;
	type Balance = u64;
	type DustRemoval = ();
	type Event = Event;
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
	pub MaxLocks: u32 = 2;
}

impl orml_tokens::Config for Test {
	type Amount = i128;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = Nothing;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type OnDust = orml_tokens::TransferDust<Test, ()>;
	type WeightInfo = ();
}

// Aggregate name getter to get fee names if the call needs to pay extra fees.
// If any call need to pay extra fees, it should be added as an item here.
// Used together with AggregateExtraFeeFilter below.
pub struct FeeNameGetter;
impl NameGetter<Call> for FeeNameGetter {
	fn get_name(c: &Call) -> ExtraFeeName {
		match *c {
			Call::Salp(bifrost_salp::Call::contribute { .. }) => ExtraFeeName::SalpContribute,
			_ => ExtraFeeName::NoExtraFee,
		}
	}
}

// Aggregate filter to filter if the call needs to pay extra fees
// If any call need to pay extra fees, it should be added as an item here.
pub struct AggregateExtraFeeFilter;
impl Contains<Call> for AggregateExtraFeeFilter {
	fn contains(c: &Call) -> bool {
		match *c {
			Call::Salp(bifrost_salp::Call::contribute { .. }) => true,
			_ => false,
		}
	}
}

pub struct ContributeFeeFilter;
impl Contains<Call> for ContributeFeeFilter {
	fn contains(c: &Call) -> bool {
		match *c {
			Call::Salp(bifrost_salp::Call::contribute { .. }) => true,
			_ => false,
		}
	}
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
	pub const AlternativeFeeCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const AltFeeCurrencyExchangeRate: (u32, u32) = (1, 100);
	pub const TreasuryAccount: AccountId32 = TREASURY_ACCOUNT;
	pub SalpWeightHolder: XcmBaseWeight = XcmBaseWeight::from(4 * XCM_WEIGHT + ContributionWeight::get()) + u64::pow(2, 24).into();
}

impl crate::Config for Test {
	type Currency = Balances;
	type DexOperator = ZenlinkProtocol;
	type FeeDealer = FixedCurrencyFeeRate<Test>;
	// type FeeDealer = FlexibleFee;
	type Event = Event;
	type MultiCurrency = Currencies;
	type TreasuryAccount = TreasuryAccount;
	type NativeCurrencyId = NativeCurrencyId;
	type AlternativeFeeCurrencyId = AlternativeFeeCurrencyId;
	type AltFeeCurrencyExchangeRate = AltFeeCurrencyExchangeRate;
	type OnUnbalanced = ();
	type WeightInfo = ();
	type ExtraFeeMatcher = ExtraFeeMatcher<Test, FeeNameGetter, AggregateExtraFeeFilter>;
	type MiscFeeHandler = MiscFeeHandler<
		Test,
		AlternativeFeeCurrencyId,
		WeightToFee,
		SalpWeightHolder,
		ContributeFeeFilter,
	>;
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
}

pub type AdaptedBasicCurrency =
	orml_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Test {
	type Event = Event;
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
	type Event = Event;
	type MultiAssetsHandler = MultiAssets;
	type PalletId = ZenlinkPalletId;
	type SelfParaId = SelfParaId;

	type TargetChains = ();
	type XcmExecutor = ();
	type Conversion = ();
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

// Mock XcmExecutor
pub struct MockXcmExecutor;

impl BifrostXcmExecutor for MockXcmExecutor {
	fn transact_weight(_: u64, _: u32) -> u64 {
		return 0;
	}

	fn transact_id(_data: &[u8]) -> MessageId {
		return [0; 32];
	}

	fn ump_transact(
		_origin: MultiLocation,
		_call: DoubleEncoded<()>,
		_weight: u64,
		_relayer: bool,
		_nonce: u32,
	) -> Result<[u8; 32], XcmError> {
		let result = unsafe { MOCK_XCM_RESULT.0 };

		match result {
			true => Ok([0; 32]),
			false => Err(XcmError::Unimplemented),
		}
	}

	fn ump_transfer_asset(
		_origin: MultiLocation,
		_dest: MultiLocation,
		_amount: u128,
		_relay: bool,
		_nonce: u32,
	) -> Result<MessageId, XcmError> {
		let result = unsafe { MOCK_XCM_RESULT.1 };

		match result {
			true => Ok([0; 32]),
			false => Err(XcmError::Unimplemented),
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
	pub const XcmTransferOrigin: TransferOriginType = TransferOriginType::FromRelayChain;
	pub BaseXcmWeight:u64 = 1_000_000_000 as u64;
	pub ContributionWeight:u64 = 1_000_000_000 as u64;
	pub AddProxyWeight:u64 = 1_000_000_000 as u64;
	pub PrimaryAccount: AccountId = ALICE;
	pub ConfirmMuitiSigAccount: AccountId = ALICE;
	pub RelaychainSovereignSubAccount: MultiLocation = MultiLocation::parent();
	pub SalpTransactProxyType: ParachainTransactProxyType = ParachainTransactProxyType::Derived;
	pub SalpTransactType: ParachainTransactType = ParachainTransactType::Xcm;
	pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
	pub const RelayNetwork: NetworkId = NetworkId::Kusama;
}

pub const XCM_WEIGHT: u64 = 1_000_000_000;

pub struct EnsureConfirmAsMultiSig;
impl EnsureOrigin<Origin> for EnsureConfirmAsMultiSig {
	type Success = AccountId;

	fn try_origin(o: Origin) -> Result<Self::Success, Origin> {
		Into::<Result<RawOrigin<AccountId>, Origin>>::into(o).and_then(|o| match o {
			RawOrigin::Signed(who) => Ok(who),
			RawOrigin::Root => Ok(Default::default()),
			r => Err(Origin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> Origin {
		Origin::from(RawOrigin::Signed(ALICE))
	}
}

pub struct MockXTokens;

impl XcmTransfer<AccountId, Balance, CurrencyId> for MockXTokens {
	fn transfer(
		_who: AccountId,
		_currency_id: CurrencyId,
		_amount: Balance,
		_dest: MultiLocation,
		_dest_weight: Weight,
	) -> DispatchResult {
		Ok(())
	}

	fn transfer_multi_asset(
		_who: AccountId,
		_asset: MultiAsset,
		_dest: MultiLocation,
		_dest_weight: Weight,
	) -> DispatchResult {
		Ok(())
	}
}

use bifrost_runtime_common::r#impl::BifrostAccountIdToMultiLocation;

impl bifrost_salp::Config for Test {
	type BancorPool = ();
	type BifrostXcmExecutor = MockXcmExecutor;
	type Event = Event;
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
	type XcmTransferOrigin = XcmTransferOrigin;
	type WeightInfo = ();
	type SelfParaId = SelfParaId;
	type BaseXcmWeight = BaseXcmWeight;
	type ContributionWeight = ContributionWeight;
	type EnsureConfirmAsMultiSig = EnsureConfirmAsMultiSig;
	type EnsureConfirmAsGovernance = EnsureConfirmAsMultiSig;
	type AddProxyWeight = AddProxyWeight;
	type XcmTransfer = MockXTokens;
	type SovereignSubAccountLocation = RelaychainSovereignSubAccount;
	type TransactProxyType = SalpTransactProxyType;
	type TransactType = SalpTransactType;
	type RelayNetwork = RelayNetwork;
	type XcmExecutor = ();
	type AccountIdToMultiLocation = BifrostAccountIdToMultiLocation;
}

//************** Salp mock end *****************

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
use crate::{self as flexible_fee, mock_price::MockOraclePriceProvider};
use bifrost_currencies::BasicCurrencyAdapter;
use bifrost_primitives::{Balance, CurrencyId, FlexibleFeePalletId, TokenSymbol, ZenlinkPalletId};
use cumulus_primitives_core::ParaId as Pid;
use frame_support::{
	derive_impl, parameter_types,
	sp_runtime::{DispatchError, DispatchResult},
	traits::{ConstU128, Get, Nothing},
	weights::{ConstantMultiplier, IdentityFee},
	PalletId,
};
use frame_system::{self, EnsureRoot};
use orml_traits::MultiCurrency;
use pallet_balances::Call as BalancesCall;
use sp_runtime::{
	traits::{AccountIdConversion, IdentityLookup, UniqueSaturatedInto},
	AccountId32, BuildStorage, SaturatedConversion,
};
use sp_std::marker::PhantomData;
use std::convert::TryInto;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, LocalAssetHandler, PairLpGenerate, ZenlinkMultiAssets,
};

pub type AccountId = AccountId32;
pub type BlockNumber = u32;
pub type Amount = i128;

pub const TREASURY_ACCOUNT: AccountId32 = AccountId32::new([9u8; 32]);

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances = 10,
		Tokens: orml_tokens,
		TransactionPayment: pallet_transaction_payment,
		FlexibleFee: flexible_fee,
		ZenlinkProtocol: zenlink_protocol,
		Currencies: bifrost_currencies,
		AssetRegistry: bifrost_asset_registry,
	}
);

pub(crate) const BALANCE_TRANSFER_CALL: <Test as frame_system::Config>::RuntimeCall =
	RuntimeCall::Balances(BalancesCall::transfer_allow_death { dest: ALICE, value: 69 });

impl bifrost_asset_registry::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
}

parameter_types! {
	pub const BlockHashCount: u32 = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<Balance>;
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
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = ();
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

impl crate::Config for Test {
	type DexOperator = ZenlinkProtocol;
	type RuntimeEvent = RuntimeEvent;
	type MultiCurrency = Currencies;
	type TreasuryAccount = TreasuryAccount;
	type MaxFeeCurrencyOrderListLen = MaxFeeCurrencyOrderListLen;
	type WeightInfo = ();
	type ParachainId = ParaInfo;
	type ControlOrigin = EnsureRoot<AccountId>;
	type XcmWeightAndFeeHandler = XcmDestWeightAndFee;
	type MinAssetHubExecutionFee = ConstU128<3>;
	type MinRelaychainExecutionFee = ConstU128<3>;
	type RelaychainCurrencyId = RelayCurrencyId;
	type XcmRouter = ();
	type PalletId = FlexibleFeePalletId;
	type OraclePriceProvider = MockOraclePriceProvider;
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

pub struct ParaInfo;
impl Get<Pid> for ParaInfo {
	fn get() -> Pid {
		Pid::from(2001)
	}
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = BNC;
}

impl bifrost_currencies::Config for Test {
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;
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
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	// We use default for brevity, but you can configure as desired if needed.
	pallet_balances::GenesisConfig::<Test>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	t.into()
}

pub const ALICE: AccountId = AccountId::new([0u8; 32]);

parameter_types! {
	pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
}

impl BalanceCmp<AccountId> for Test {
	type Error = Error<Test>;

	fn cmp_with_precision(
		account: &AccountId,
		currency: &CurrencyId,
		amount: u128,
		amount_precision: u32,
	) -> Result<Ordering, Self::Error> {
		Pallet::<Test>::cmp_with_precision(account, currency, amount, amount_precision)
	}
}

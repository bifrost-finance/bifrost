// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

#![cfg(test)]

use super::*;
use crate as charge_transaction_fee;
use frame_support::{
	parameter_types,
	weights::{IdentityFee, WeightToFeeCoefficients, WeightToFeePolynomial},
};
use frame_system as system;
use smallvec::smallvec;
use sp_std::cell::RefCell;
// use node_primitives::Balance;
use frame_support::PalletId;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};

use node_primitives::{CurrencyId, TokenSymbol};
use zenlink_protocol::{make_x2_location, MultiLocation, NativeCurrencyAdaptor, OtherAssetAdaptor};

pub type BlockNumber = u64;
pub type Amount = i128;

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
		Assets: orml_tokens::{Pallet, Storage, Event<T>},
		Balances: balances::{Pallet, Call, Storage, Event<T>},
		// TransactionPayment: pallet_transaction_payment::{Module, Storage},
		ChargeTransactionFee: charge_transaction_fee::{Pallet, Call, Storage,Event<T>},
		Zenlink: zenlink_protocol::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

impl system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type Call = Call;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u128;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

thread_local! {
	static WEIGHT_TO_FEE: RefCell<u128> = RefCell::new(1);
}

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = u128;

	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		smallvec![frame_support::weights::WeightToFeeCoefficient {
			degree: 1,
			coeff_frac: Perbill::zero(),
			coeff_integer: WEIGHT_TO_FEE.with(|v| *v.borrow()),
			negative: false,
		}]
	}
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Config for Test {
	type OnChargeTransaction = ChargeTransactionFee;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl balances::Config for Test {
	type Balance = u64;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type MaxLocks = ();
	type WeightInfo = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		0
	};
}

impl orml_tokens::Config for Test {
	type Event = Event;
	type Balance = Balance;
	type Amount = i128;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type OnDust = orml_tokens::TransferDust<Test, ()>;
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::ASG);
}

impl crate::Config for Test {
	type Event = Event;
	type Balance = u64;
	type WeightInfo = ();
	type CurrenciesHandler = Currencies;
	type Currency = Balances;
	type OnUnbalanced = ();
	type ZenlinkDEX = Zenlink;
	type NativeCurrencyId = NativeCurrencyId;
}

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::ASG);
}

pub type AdaptedBasicCurrency =
	orml_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Test {
	type Event = Event;
	type MultiCurrency = Assets;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}

parameter_types! {
	pub const ZenlinkPalletId: PalletId = PalletId(*b"zenlink1");
	pub ZenlinkRegistedParaChains: Vec<(MultiLocation, u128)> = vec![
		// Phala local and live, 1 PHA
		(make_x2_location(30),	1_000_000_000_000),
		// Sherpax live
		(make_x2_location(59),  500),
		// Bifrost local and live, 0.01 BNC
		(make_x2_location(1024),   10_000_000_000),
		// Zenlink live
		(make_x2_location(188), 500),
		// Zenlink local
		(make_x2_location(200), 500),
		// Sherpax local
		(make_x2_location(300), 500),
		// Plasm local and live, 0.001 PLM
		(make_x2_location(5000), 1_000_000_000_000)
	];
}

impl zenlink_protocol::Config for Test {
	type Event = Event;
	type XcmExecutor = ();
	type AccountIdConverter = ();
	type AccountId32Converter = ();
	type ParaId = ();
	type PalletId = ZenlinkPalletId;
	type TargetChains = ();
	type NativeCurrency = NativeCurrencyAdaptor<Test, Balances>;
	type OtherAssets = OtherAssetAdaptor<Test, Currencies>;
}

// Build genesis storage according to the mock runtime.
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap()
		.into()
}

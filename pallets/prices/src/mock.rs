// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Mocks for the prices module.

use super::*;
use frame_support::{
	construct_runtime, derive_impl, ord_parameter_types, parameter_types,
	traits::{AsEnsureOriginWithArg, Nothing},
};
use frame_system::{EnsureRoot, EnsureSigned, EnsureSignedBy};
use sp_runtime::{traits::IdentityLookup, FixedPointNumber};

use bifrost_asset_registry::AssetIdMaps;
pub use bifrost_primitives::{
	currency::{FIL, VFIL},
	DOT, KSM, MANTA, VDOT,
};
use bifrost_primitives::{Moment, ASTR, BNC, DOT_U, GLMR};
use sp_runtime::BuildStorage;

pub type AccountId = u128;
pub type BlockNumber = u64;
pub const ALICE: AccountId = 1;
pub const CHARLIE: AccountId = 2;

pub const PRICE_ONE: u128 = 1_000_000_000_000_000_000;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<Balance>;
}

pub type TimeStampedPrice = orml_oracle::TimestampedValue<Price, Moment>;
pub struct MockDataProvider;
impl DataProvider<CurrencyId, TimeStampedPrice> for MockDataProvider {
	fn get(asset_id: &CurrencyId) -> Option<TimeStampedPrice> {
		match *asset_id {
			BNC => Some(TimeStampedPrice {
				value: Price::from_inner(200_000_000_000_000_000),
				timestamp: 0,
			}),
			DOT =>
				Some(TimeStampedPrice { value: Price::saturating_from_integer(100), timestamp: 0 }),
			KSM =>
				Some(TimeStampedPrice { value: Price::saturating_from_integer(500), timestamp: 0 }),
			MANTA => Some(TimeStampedPrice {
				value: Price::from_inner(600_000_000_000_000_000),
				timestamp: 0,
			}),
			VDOT => Some(TimeStampedPrice {
				value: Price::from_inner(15000000000_0000000000),
				timestamp: 0,
			}),
			FIL => Some(TimeStampedPrice {
				value: Price::from_inner(6666666666_6666666600),
				timestamp: 0,
			}),
			VFIL => Some(TimeStampedPrice {
				value: Price::from_inner(6666666666_6666666600),
				timestamp: 0,
			}),
			_ => None,
		}
	}
}

impl DataProviderExtended<CurrencyId, TimeStampedPrice> for MockDataProvider {
	fn get_no_op(asset_id: &CurrencyId) -> Option<TimeStampedPrice> {
		match *asset_id {
			DOT =>
				Some(TimeStampedPrice { value: Price::saturating_from_integer(100), timestamp: 0 }),
			KSM =>
				Some(TimeStampedPrice { value: Price::saturating_from_integer(500), timestamp: 0 }),
			_ => None,
		}
	}

	fn get_all_values() -> Vec<(CurrencyId, Option<TimeStampedPrice>)> {
		vec![]
	}
}

impl DataFeeder<CurrencyId, TimeStampedPrice, AccountId> for MockDataProvider {
	fn feed_value(
		_: Option<AccountId>,
		_: CurrencyId,
		_: TimeStampedPrice,
	) -> sp_runtime::DispatchResult {
		Ok(())
	}
}

ord_parameter_types! {
	pub const One: AccountId = 1;
}

// pallet-balances configuration
parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Test {
	type AccountStore = frame_system::Pallet<Test>;
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

// pallet-assets configuration
parameter_types! {
	pub const AssetDeposit: u64 = 1;
	pub const ApprovalDeposit: u64 = 1;
	pub const AssetAccountDeposit: u64 = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: u64 = 1;
	pub const MetadataDepositPerByte: u64 = 1;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = CurrencyId;
	type AssetIdParameter = CurrencyId;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type AssetAccountDeposit = AssetAccountDeposit;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	type CallbackHandle = ();
}

parameter_types! {
	pub const RelayCurrency: CurrencyId = DOT;
	pub const NativeCurrencyId: CurrencyId = BNC;
}

impl crate::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Source = MockDataProvider;
	type FeederOrigin = EnsureSignedBy<One, AccountId>;
	type UpdateOrigin = EnsureSignedBy<One, AccountId>;
	type RelayCurrency = RelayCurrency;
	type CurrencyIdConvert = AssetIdMaps<Test>;
	type Assets = Currencies;
	type WeightInfo = ();
}

pub type Amount = i128;
pub type AdaptedBasicCurrency =
	bifrost_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Test {
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type WeightInfo = ();
}

impl bifrost_asset_registry::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RegisterOrigin = EnsureSignedBy<One, AccountId>;
	type WeightInfo = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

impl orml_tokens::Config for Test {
	type Amount = i128;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type DustRemovalWhitelist = Nothing;
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = ();
	type CurrencyHooks = ();
}

type Block = frame_system::mocking::MockBlock<Test>;

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		Tokens: orml_tokens,
		Currencies: bifrost_currencies,
		Prices: crate,
		AssetRegistry: bifrost_asset_registry,
	}
);

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	bifrost_asset_registry::GenesisConfig::<Test> {
		currency: vec![
			(KSM, 1, None),
			(BNC, 1, None),
			(DOT, 1, Some(("_".to_string(), "_".to_string(), 10))),
			(ASTR, 1, None),
			(GLMR, 1, None),
			(DOT_U, 1, None),
			(FIL, 1, Some(("_".to_string(), "_".to_string(), 10))),
			(VFIL, 1, Some(("_".to_string(), "_".to_string(), 10))),
		],
		vcurrency: vec![VDOT],
		vsbond: vec![],
		phantom: Default::default(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let endowed_accounts: Vec<(AccountId, CurrencyId, Balance)> = vec![
		(ALICE, DOT, 1000 * PRICE_ONE),
		(ALICE, FIL, 1000 * PRICE_ONE),
		(ALICE, VDOT, 1000 * PRICE_ONE),
	];

	orml_tokens::GenesisConfig::<Test> {
		balances: endowed_accounts
			.clone()
			.into_iter()
			.filter(|(_, currency_id, _)| *currency_id != BNC)
			.collect::<Vec<_>>(),
	}
	.assimilate_storage(&mut t)
	.unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(ALICE, 100_000_000), (CHARLIE, 100_000_000)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		Assets::force_create(RuntimeOrigin::root(), DOT.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), VDOT.into(), ALICE, true, 1).unwrap();
		Assets::force_create(RuntimeOrigin::root(), FIL.into(), ALICE, true, 1).unwrap();

		Assets::mint(RuntimeOrigin::signed(ALICE), DOT.into(), ALICE, 1000 * PRICE_ONE).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), VDOT.into(), ALICE, 1000 * PRICE_ONE).unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), FIL.into(), ALICE, 1000 * PRICE_ONE).unwrap();

		Prices::set_foreign_asset(RuntimeOrigin::signed(ALICE), VFIL, FIL).unwrap();
	});

	ext
}

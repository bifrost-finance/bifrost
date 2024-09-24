#![cfg(test)]

use cumulus_primitives_core::ParaId;
use crate as xcm_interface;
use frame_support::{derive_impl, parameter_types, traits::Everything};
use frame_support::__private::Get;
use frame_support::traits::Nothing;
use frame_system as system;
use frame_system::EnsureRoot;
use sp_core::crypto::AccountId32;
use sp_core::{ConstU32, H256};
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use bifrost_asset_registry::AssetIdMaps;
use bifrost_primitives::{Amount, Balance, BlockNumber, CurrencyId, MockXcmRouter, BNC};

pub type AccountId = AccountId32;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		XcmInterface: xcm_interface,
		Balances: pallet_balances,
        Currencies: bifrost_currencies,
        Tokens: orml_tokens,
        AssetRegistry: bifrost_asset_registry,
	}
);

parameter_types! {
	pub const SS58Prefix: u8 = 42;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
    type BaseCallFilter = Everything;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type AccountData = pallet_balances::AccountData<Balance>;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type PalletInfo = PalletInfo;
    type SS58Prefix = SS58Prefix;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
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

parameter_types! {
	pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
    type AccountStore = System;
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

impl bifrost_asset_registry::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type RegisterOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = BNC;
}

pub type AdaptedBasicCurrency =
bifrost_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Test {
    type GetNativeCurrencyId = NativeCurrencyId;
    type MultiCurrency = Tokens;
    type NativeCurrency = AdaptedBasicCurrency;
    type WeightInfo = ();
}

pub struct ParachainId;
impl Get<ParaId> for ParachainId {
    fn get() -> ParaId {
        2030.into()
    }
}

impl xcm_interface::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type MultiCurrency = Currencies;
    type WeightInfo = ();
    type UpdateOrigin = EnsureRoot<AccountId>;
    type XcmRouter = MockXcmRouter;
    type AccountIdToLocation = ();
    type CurrencyIdConvert = AssetIdMaps<Test>;
    type ParachainId = ParachainId;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}
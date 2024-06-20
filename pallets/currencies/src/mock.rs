//! Mocks for the currencies module.

#![cfg(test)]

use super::*;
use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU32, ConstU64, Nothing},
	PalletId,
};
use orml_traits::{currency::MutationHooks, parameter_type_with_key};
use sp_runtime::{
	traits::{AccountIdConversion, IdentityLookup},
	AccountId32, BuildStorage,
};

use crate as currencies;

pub type ReserveIdentifier = [u8; 8];

pub type AccountId = AccountId32;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<u64>;
}

type CurrencyId = u32;
type Balance = u64;

impl pallet_balances::Config for Runtime {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU64<2>;
	type AccountStore = frame_system::Pallet<Runtime>;
	type MaxLocks = ();
	type MaxReserves = ConstU32<2>;
	type ReserveIdentifier = ReserveIdentifier;
	type WeightInfo = ();
	type FreezeIdentifier = [u8; 8];
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

parameter_types! {
	pub DustAccount: AccountId = PalletId(*b"orml/dst").into_account_truncating();
}

pub struct CurrencyHooks<T>(marker::PhantomData<T>);
impl<T: orml_tokens::Config> MutationHooks<T::AccountId, T::CurrencyId, T::Balance>
	for CurrencyHooks<T>
where
	T::AccountId: From<AccountId32>,
{
	type OnDust = orml_tokens::TransferDust<T, DustAccount>;
	type OnSlash = ();
	type PreDeposit = ();
	type PostDeposit = ();
	type PreTransfer = ();
	type PostTransfer = ();
	type OnNewTokenAccount = ();
	type OnKilledTokenAccount = ();
}

impl orml_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = i64;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = CurrencyHooks<Runtime>;
	type MaxLocks = ConstU32<100_000>;
	type MaxReserves = ConstU32<100_000>;
	type ReserveIdentifier = ReserveIdentifier;
	type DustRemovalWhitelist = Nothing;
}

pub const NATIVE_CURRENCY_ID: CurrencyId = 1;
pub const X_TOKEN_ID: CurrencyId = 2;

parameter_types! {
	pub const GetNativeCurrencyId: CurrencyId = NATIVE_CURRENCY_ID;
}

impl Config for Runtime {
	type MultiCurrency = Tokens;
	type NativeCurrency = AdaptedBasicCurrency;
	type GetNativeCurrencyId = GetNativeCurrencyId;
	type WeightInfo = ();
}
pub type NativeCurrency = NativeCurrencyOf<Runtime>;
pub type AdaptedBasicCurrency = BasicCurrencyAdapter<Runtime, PalletBalances, i64, u64>;

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		Currencies: currencies,
		Tokens: orml_tokens,
		PalletBalances: pallet_balances,
	}
);

pub const ALICE: AccountId = AccountId32::new([1u8; 32]);
pub const BOB: AccountId = AccountId32::new([2u8; 32]);
pub const EVA: AccountId = AccountId32::new([5u8; 32]);
pub const ID_1: LockIdentifier = *b"1       ";
pub const RID_1: ReserveIdentifier = [1u8; 8];
pub const RID_2: ReserveIdentifier = [2u8; 8];

#[derive(Default)]
pub struct ExtBuilder {
	balances: Vec<(AccountId, CurrencyId, Balance)>,
}

impl ExtBuilder {
	pub fn balances(mut self, balances: Vec<(AccountId, CurrencyId, Balance)>) -> Self {
		self.balances = balances;
		self
	}

	pub fn one_hundred_for_alice_n_bob(self) -> Self {
		self.balances(vec![
			(ALICE, NATIVE_CURRENCY_ID, 100),
			(BOB, NATIVE_CURRENCY_ID, 100),
			(ALICE, X_TOKEN_ID, 100),
			(BOB, X_TOKEN_ID, 100),
		])
	}

	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: self
				.balances
				.clone()
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id == NATIVE_CURRENCY_ID)
				.map(|(account_id, _, initial_balance)| (account_id, initial_balance))
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		orml_tokens::GenesisConfig::<Runtime> {
			balances: self
				.balances
				.into_iter()
				.filter(|(_, currency_id, _)| *currency_id != NATIVE_CURRENCY_ID)
				.collect::<Vec<_>>(),
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}

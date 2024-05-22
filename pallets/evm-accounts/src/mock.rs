#![cfg(test)]
use super::*;

use crate as pallet_evm_accounts;
use crate::{Balance, Config};
use frame_support::parameter_types;
use frame_support::sp_runtime::{
	traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature,
};
use frame_support::traits::Everything;
use frame_system::EnsureRoot;
use orml_traits::parameter_type_with_key;
pub use sp_core::{H160, H256};
use std::cell::RefCell;
use std::collections::HashMap;

pub type AssetId = u32;
pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
type Block = frame_system::mocking::MockBlock<Test>;

pub const ONE: Balance = 1_000_000_000_000;
pub const INITIAL_BALANCE: Balance = 1_000_000_000_000 * ONE;

pub const ALICE: AccountId = AccountId::new([1; 32]);

pub const HDX: AssetId = 0;

thread_local! {
	pub static NONCE: RefCell<HashMap<H160, U256>> = RefCell::new(HashMap::default());
}

frame_support::construct_runtime!(
	pub enum Test
	 {
		 System: frame_system,
		 EVMAccounts: pallet_evm_accounts,
		 Tokens: orml_tokens,
	 }

);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 63;
	pub const NativeAssetId: AssetId = HDX;
}

pub struct EvmNonceProviderMock;
impl EvmNonceProvider for EvmNonceProviderMock {
	fn get_nonce(evm_address: H160) -> U256 {
		NONCE
			.with(|v| v.borrow().get(&evm_address).copied())
			.unwrap_or_default()
	}
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type FeeMultiplier = sp_core::ConstU32<10>;
	type EvmNonceProvider = EvmNonceProviderMock;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type WeightInfo = ();
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Block = Block;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_asset_id: AssetId| -> Balance {
		1
	};
}

impl orml_tokens::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = i128;
	type CurrencyId = AssetId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = ();
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = ();
	type DustRemovalWhitelist = Everything;
}

pub struct ExtBuilder {
	endowed_accounts: Vec<(AccountId, AssetId, Balance)>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		NONCE.with(|v| {
			v.borrow_mut().clear();
		});

		Self {
			endowed_accounts: vec![(ALICE, HDX, INITIAL_BALANCE)],
		}
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

		orml_tokens::GenesisConfig::<Test> {
			balances: self.endowed_accounts,
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let mut r: sp_io::TestExternalities = t.into();
		r.execute_with(|| System::set_block_number(1));
		r
	}

	pub fn with_non_zero_nonce(self, account_id: AccountId) -> Self {
		let evm_address = EVMAccounts::evm_address(&account_id);
		NONCE.with(|v| {
			let mut m = v.borrow_mut();
			m.insert(evm_address, U256::one());
		});
		self
	}
}

pub fn expect_events(e: Vec<RuntimeEvent>) {
	test_utils::expect_events::<RuntimeEvent, Test>(e);
}

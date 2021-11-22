//! Mocks for the merkle-distributor
use super::*;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

use crate::mock::TokenSymbol::*;
use frame_support::{construct_runtime, parameter_types, traits::Contains};
use orml_traits::parameter_type_with_key;
use sp_runtime::{
    testing::Header,
    traits::{IdentifyAccount, IdentityLookup, Verify},
    AccountId32, MultiSignature,
};

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
    Token(TokenSymbol),
    Other(TokenSymbol),
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TokenSymbol {
    Test1 = 0,
    Test2 = 1,
}

mod merkle_distributor {
    pub use super::super::*;
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MdPalletId: PalletId = PalletId(*b"zlk/md**");
    pub const StringLimit: u32 = 50;
    pub const MaxReserves: u32 = 50;
    pub const ExistentialDeposit: u64 = 1;
}

parameter_type_with_key! {
    pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
        Default::default()
    };
}

pub(crate) const CURRENCY_TEST1: CurrencyId = CurrencyId::Token(TokenSymbol::Test1);

type Balance = u128;

pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

pub const ALICE: AccountId = AccountId32::new([0u8; 32]);
pub const BOB: AccountId = AccountId32::new([1u8; 32]);

impl frame_system::Config for Runtime {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = Call;
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
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
}

pub struct DustRemovalWhitelist;

impl Contains<AccountId> for DustRemovalWhitelist {
    fn contains(_: &AccountId) -> bool {
        true
    }
}

impl orml_tokens::Config for Runtime {
    type Event = Event;
    type Balance = Balance;
    type Amount = i128;
    type CurrencyId = CurrencyId;
    type WeightInfo = ();
    type ExistentialDeposits = ExistentialDeposits;
    type OnDust = ();
    type MaxLocks = ();
    type DustRemovalWhitelist = DustRemovalWhitelist;
}

impl pallet_balances::Config for Runtime {
    type Balance = u128;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
}

impl Config for Runtime {
    type Event = Event;
    type Balance = Balance;
    type CurrencyId = CurrencyId;
    type MerkleDistributorId = u32;
    type PalletId = MdPalletId;
    type StringLimit = StringLimit;
    type MultiCurrency = Tokens;
    type WeightInfo = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
        MerkleDistributor: merkle_distributor::{Pallet, Storage, Call, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Tokens: orml_tokens::{Pallet, Storage, Event<T>, Config<T>},
    }
);

pub type MdPallet = Pallet<Runtime>;

pub(crate) const UNIT: Balance = 1_000_000_000_000;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Runtime>()
        .unwrap()
        .into();
    pallet_balances::GenesisConfig::<Runtime> {
        balances: vec![(ALICE, 1_000_000 * UNIT), (BOB, 1_000_000 * UNIT)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    orml_tokens::GenesisConfig::<Runtime> {
        balances: vec![
            (ALICE, CURRENCY_TEST1, 1_000_000_000_000 * UNIT),
            (BOB, CurrencyId::Token(Test2), 1_000_000 * UNIT),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}

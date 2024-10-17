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

// Ensure we're `no_std` when compiling for Wasm.
use frame_support::{
	construct_runtime, parameter_types,
	traits::{EnsureOrigin, GenesisBuild, Nothing},
	weights::{Weight, WeightToFeeCoefficient, WeightToFeeCoefficients, WeightToFeePolynomial},
	PalletId,
};
use frame_system::RawOrigin;
use bifrost_primitives::{Amount, Balance, CurrencyId, MessageId, TokenSymbol};
use smallvec::smallvec;
use sp_arithmetic::Percent;
use sp_core::H256;
pub use sp_runtime::Perbill;
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentityLookup},
};

use crate as salp;
use crate::WeightInfo;

pub(crate) type AccountId = <<Signature as sp_runtime::traits::Verify>::Signer as sp_runtime::traits::IdentifyAccount>::AccountId;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u32;
pub(crate) type Index = u32;
pub(crate) type Signature = sp_runtime::MultiSignature;
pub(crate) type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;

construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Currencies: bifrost_currencies,
		Tokens: orml_tokens,
		Multisig: pallet_multisig,
		Salp: salp,
	}
);

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
	pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::DOT);
	pub const StableCurrencyId: CurrencyId = CurrencyId::Stable(TokenSymbol::KUSD);
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
}

impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockWeights = ();
	type RuntimeCall = RuntimeCall;
	type DbWeight = ();
	type RuntimeEvent = RuntimeEvent;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Nonce = u32;
	type Block = Block;
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
	pub const ExistentialDeposit: u128 = 1;
	pub const TransferFee: u128 = 0;
	pub const CreationFee: u128 = 0;
	pub const TransactionByteFee: u128 = 0;
	pub const MaxLocks: u32 = 999_999;
	pub const MaxReserves: u32 = 999_999;
}

impl pallet_balances::Config for Test {
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
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
}

parameter_types! {
	pub const DepositBase: Balance = 0;
	pub const DepositFactor: Balance = 0;
	pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Test {
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type DepositBase = DepositBase;
	type DepositFactor = DepositFactor;
	type RuntimeEvent = RuntimeEvent;
	type MaxSignatories = MaxSignatories;
	type WeightInfo = pallet_multisig::weights::SubstrateWeight<Test>;
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

impl orml_tokens::Config for Test {
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

pub type BifrostToken = bifrost_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl bifrost_currencies::Config for Test {
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BifrostToken;
	type WeightInfo = ();
}

parameter_types! {
	pub const MinContribution: Balance = 10;
	pub const RemoveKeysLimit: u32 = 50;
	pub const SlotLength: BlockNumber = 8u32 as BlockNumber;
	pub const LeasePeriod: BlockNumber = 6 * WEEKS;
	pub const VSBondValidPeriod: BlockNumber = 30 * DAYS;
	pub const ReleaseCycle: BlockNumber = 1 * DAYS;
	pub const ReleaseRatio: Percent = Percent::from_percent(50);
	pub PrimaryAccount: AccountId = ALICE;
	pub ConfirmMuitiSigAccount: AccountId = Multisig::multi_account_id(&vec![
		ALICE,
		BRUCE,
		CATHI
	],2);
}

pub struct EnsureConfirmAsGovernance;
impl EnsureOrigin<RuntimeOrigin> for EnsureConfirmAsGovernance {
	type Success = AccountId;

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		Into::<Result<RawOrigin<AccountId>, RuntimeOrigin>>::into(o).and_then(|o| match o {
			RawOrigin::Signed(who) => Ok(who),
			RawOrigin::Root => Ok(ConfirmMuitiSigAccount::get()),
			r => Err(RuntimeOrigin::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::from(RawOrigin::Signed(ConfirmMuitiSigAccount::get())))
	}
}

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
	type Balance = Balance;
	fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
		smallvec![WeightToFeeCoefficient {
			degree: 1,
			negative: false,
			coeff_frac: Perbill::from_rational(90u32, 100u32),
			coeff_integer: 1,
		}]
	}
}

impl salp::Config for Test {
	type BancorPool = ();
	type RuntimeEvent = RuntimeEvent;
	type LeasePeriod = LeasePeriod;
	type MinContribution = MinContribution;
	type MultiCurrency = Tokens;
	type PalletId = BifrostCrowdloanId;
	type RelayChainToken = RelayCurrencyId;
	type ReleaseCycle = ReleaseCycle;
	type ReleaseRatio = ReleaseRatio;
	type BatchKeysLimit = RemoveKeysLimit;
	type SlotLength = SlotLength;
	type WeightInfo = SalpWeightInfo;
	type EnsureConfirmAsGovernance = EnsureConfirmAsGovernance;
}

pub struct SalpWeightInfo;
impl WeightInfo for SalpWeightInfo {
	fn redeem() -> Weight {
		Weight::zero()
	}

	fn refund() -> Weight {
		Weight::zero()
	}

	fn set_multisig_confirm_account() -> Weight {
		Weight::zero()
	}

	fn issue() -> Weight {
		Weight::zero()
	}

	fn fund_success() -> Weight {
		Weight::zero()
	}

	fn fund_fail() -> Weight {
		Weight::zero()
	}

	fn continue_fund() -> Weight {
		Weight::zero()
	}

	fn fund_retire() -> Weight {
		Weight::zero()
	}

	fn create() -> Weight {
		Weight::zero()
	}

	fn edit() -> Weight {
		Weight::zero()
	}

	fn withdraw() -> Weight {
		Weight::zero()
	}

	fn dissolve_refunded() -> Weight {
		Weight::zero()
	}

	fn dissolve() -> Weight {
		Weight::zero()
	}
}

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	orml_tokens::GenesisConfig::<Test> {
		balances: vec![
			(ALICE, NativeCurrencyId::get(), INIT_BALANCE),
			(ALICE, RelayCurrencyId::get(), INIT_BALANCE),
			(BRUCE, NativeCurrencyId::get(), INIT_BALANCE),
			(BRUCE, RelayCurrencyId::get(), INIT_BALANCE),
			(CATHI, NativeCurrencyId::get(), INIT_BALANCE),
			(CATHI, RelayCurrencyId::get(), INIT_BALANCE),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	crate::GenesisConfig::<Test> { initial_multisig_account: Some(ALICE) }
		.assimilate_storage(&mut t)
		.unwrap();

	t.into()
}

// These time units are defined in number of blocks.
pub const MINUTES: BlockNumber = 60 / (12 as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;
pub const WEEKS: BlockNumber = DAYS * 7;

pub(crate) const ALICE: AccountId = AccountId::new([0u8; 32]);
pub(crate) const BRUCE: AccountId = AccountId::new([1u8; 32]);
pub(crate) const CATHI: AccountId = AccountId::new([2u8; 32]);

pub(crate) const INIT_BALANCE: Balance = 100_000;

pub(crate) const CONTRIBUTON_INDEX: MessageId = [0; 32];

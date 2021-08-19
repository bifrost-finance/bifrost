// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use frame_support::{
	construct_runtime, parameter_types,
	sp_io::TestExternalities,
	sp_runtime::{
		generic,
		traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
		BuildStorage, MultiSignature,
	},
};
use node_primitives::{Amount, Balance, CurrencyId, TokenSymbol};
use sp_core::H256;

use crate as lm;

pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type Block = frame_system::mocking::MockBlock<T>;
pub(crate) type BlockNumber = u32;
pub(crate) type Index = u32;
pub(crate) type Signature = MultiSignature;
pub(crate) type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<T>;

construct_runtime!(
	pub enum T where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>, Config<T>},
		TechnicalCommittee: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
		LM: lm::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const NativeCurrencyId: CurrencyId = CurrencyId::Native(TokenSymbol::ASG);
	pub const RelayCurrencyId: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
}

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for T {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = ();
	type BlockHashCount = BlockHashCount;
	type BlockLength = ();
	type BlockNumber = BlockNumber;
	type BlockWeights = ();
	type Call = Call;
	type DbWeight = ();
	type Event = Event;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type Index = Index;
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

parameter_types! {
	pub const ExistentialDeposit: u128 = 0;
	pub const TransferFee: u128 = 0;
	pub const CreationFee: u128 = 0;
	pub const TransactionByteFee: u128 = 0;
	pub const MaxLocks: u32 = 999_999;
	pub const MaxReserves: u32 = 999_999;
}

impl pallet_balances::Config for T {
	type AccountStore = System;
	/// The type for recording an account's balance.
	type Balance = Balance;
	type DustRemoval = ();
	/// The ubiquitous event type.
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = pallet_balances::weights::SubstrateWeight<T>;
}

pub type BifrostToken = orml_currencies::BasicCurrencyAdapter<T, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for T {
	type Event = Event;
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BifrostToken;
	type WeightInfo = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		0
	};
}

impl orml_tokens::Config for T {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type OnDust = ();
	type WeightInfo = ();
	type DustRemovalWhitelist = ();
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance1;
impl pallet_collective::Config<TechnicalCollective> for T {
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type Event = Event;
	type MaxMembers = TechnicalMaxMembers;
	type MaxProposals = TechnicalMaxProposals;
	type MotionDuration = TechnicalMotionDuration;
	type Origin = Origin;
	type Proposal = Call;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<T>;
}

parameter_types! {
	pub const RelayChainTokenSymbol: TokenSymbol = TokenSymbol::KSM;
	pub const MaximumDepositInPool: Balance = 1_000_000 * UNIT;
	pub const MinimumDeposit: Balance = 1_000_000;
	pub const MinimumRewardPerBlock: Balance = 1_000;
	pub const MinimumDuration: BlockNumber = MINUTES;
	pub const MaximumApproved: u32 = 4;
}

impl lm::Config for T {
	type Event = Event;
	type ControlOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type MultiCurrency = Tokens;
	type RelayChainTokenSymbol = RelayChainTokenSymbol;
	type MaximumDepositInPool = MaximumDepositInPool;
	type MinimumDepositOfUser = MinimumDeposit;
	type MinimumRewardPerBlock = MinimumRewardPerBlock;
	type MinimumDuration = MinimumDuration;
	type MaximumApproved = MaximumApproved;
}

pub(crate) fn new_test_ext() -> TestExternalities {
	GenesisConfig {
		tokens: orml_tokens::GenesisConfig::<T> {
			balances: vec![
				(CREATOR, REWARD_1, REWARD_AMOUNT),
				(CREATOR, REWARD_2, REWARD_AMOUNT),
				(USER_1, FARMING_DEPOSIT_1, DEPOSIT_AMOUNT),
				(USER_1, FARMING_DEPOSIT_2, DEPOSIT_AMOUNT),
				(USER_2, FARMING_DEPOSIT_1, DEPOSIT_AMOUNT),
				(USER_2, FARMING_DEPOSIT_2, DEPOSIT_AMOUNT),
				(USER_1, MINING_DEPOSIT, DEPOSIT_AMOUNT),
				(USER_2, MINING_DEPOSIT, DEPOSIT_AMOUNT),
				(RICHER, FARMING_DEPOSIT_1, 1_000_000_000_000 * UNIT),
				(RICHER, FARMING_DEPOSIT_2, 1_000_000_000_000 * UNIT),
				(RICHER, MINING_DEPOSIT, 1_000_000_000_000 * UNIT),
			],
		},
		technical_committee: pallet_collective::GenesisConfig {
			members: vec![TC_MEMBER_1, TC_MEMBER_2, TC_MEMBER_3],
			phantom: Default::default(),
		},
	}
	.build_storage()
	.unwrap()
	.into()
}

pub(crate) const MINUTES: BlockNumber = 60 / (12 as BlockNumber);
pub(crate) const HOURS: BlockNumber = MINUTES * 60;
pub(crate) const DAYS: BlockNumber = HOURS * 24;

pub(crate) const UNIT: Balance = 1_000_000_000_000;

pub(crate) const MINING_TRADING_PAIR: (CurrencyId, CurrencyId) =
	(CurrencyId::Token(TokenSymbol::DOT), CurrencyId::Token(TokenSymbol::KSM));
pub(crate) const MINING_DEPOSIT: CurrencyId =
	CurrencyId::LPToken(TokenSymbol::DOT, 2u8, TokenSymbol::KSM, 2u8);
pub(crate) const FARMING_DEPOSIT_1: CurrencyId = CurrencyId::VSToken(TokenSymbol::KSM);
pub(crate) const FARMING_DEPOSIT_2: CurrencyId = CurrencyId::VSBond(TokenSymbol::KSM, 2001, 13, 20);
pub(crate) const DEPOSIT_AMOUNT: Balance = UNIT;
pub(crate) const REWARD_1: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
pub(crate) const REWARD_2: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
pub(crate) const REWARD_AMOUNT: Balance = UNIT;

pub(crate) const CREATOR: AccountId = AccountId::new([0u8; 32]);
pub(crate) const USER_1: AccountId = AccountId::new([1u8; 32]);
pub(crate) const USER_2: AccountId = AccountId::new([2u8; 32]);
pub(crate) const TC_MEMBER_1: AccountId = AccountId::new([3u8; 32]);
pub(crate) const TC_MEMBER_2: AccountId = AccountId::new([4u8; 32]);
pub(crate) const TC_MEMBER_3: AccountId = AccountId::new([5u8; 32]);
pub(crate) const RICHER: AccountId = AccountId::new([6u8; 32]);

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

use frame_support::{
	construct_runtime, parameter_types,
	sp_io::TestExternalities,
	sp_runtime::{
		generic,
		traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
		BuildStorage, MultiSignature,
	},
	traits::Contains,
	PalletId,
};
use node_primitives::{traits::CheckSubAccount, Amount, Balance, CurrencyId, TokenSymbol};
use sp_core::H256;

use crate as lm;
use crate::{PoolId, StorageVersion};

pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type BlockNumber = u32;
pub(crate) type Index = u32;
pub(crate) type Signature = MultiSignature;
pub(crate) type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Event<T>},
		Currencies: orml_currencies::{Pallet, Call, Event<T>},
		Tokens: orml_tokens::{Pallet, Call, Storage, Event<T>, Config<T>},
		Collective: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
		LM: lm::{Pallet, Call, Storage, Event<T>, Config<T>},
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

impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<Balance>;
	type AccountId = AccountId;
	type BaseCallFilter = frame_support::traits::Everything;
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
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 1_000_000;
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
	type Event = Event;
	type ExistentialDeposit = ExistentialDeposit;
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
}

pub type BifrostToken = orml_currencies::BasicCurrencyAdapter<Test, Balances, Amount, BlockNumber>;

impl orml_currencies::Config for Test {
	type Event = Event;
	type GetNativeCurrencyId = NativeCurrencyId;
	type MultiCurrency = Tokens;
	type NativeCurrency = BifrostToken;
	type WeightInfo = ();
}

orml_traits::parameter_type_with_key! {
	pub ExistentialDeposits: |currency_id: CurrencyId| -> Balance {
		match currency_id {
			CurrencyId::LPToken(..) => 10_000_000,
			_ => 1_000_000,
		}
	};
}

parameter_types! {
	pub BifrostTreasuryFakeAccount: AccountId = AccountId::new([155u8;32]);
}

pub struct DustRemovalWhitelist;
impl Contains<AccountId> for DustRemovalWhitelist {
	fn contains(a: &AccountId) -> bool {
		*a == BifrostTreasuryFakeAccount::get() ||
			*a == INVESTOR ||
			LiquidityMiningPalletId::get().check_sub_account::<PoolId>(a)
	}
}

impl orml_tokens::Config for Test {
	type Amount = Amount;
	type Balance = Balance;
	type CurrencyId = CurrencyId;
	type Event = Event;
	type ExistentialDeposits = ExistentialDeposits;
	type MaxLocks = MaxLocks;
	type OnDust = orml_tokens::TransferDust<Test, BifrostTreasuryFakeAccount>;
	type WeightInfo = ();
	type DustRemovalWhitelist = DustRemovalWhitelist;
}

parameter_types! {
	pub const TechnicalMotionDuration: BlockNumber = 3 * DAYS;
	pub const TechnicalMaxProposals: u32 = 100;
	pub const TechnicalMaxMembers: u32 = 100;
}

type TechnicalCollective = pallet_collective::Instance1;
impl pallet_collective::Config<TechnicalCollective> for Test {
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type Event = Event;
	type MaxMembers = TechnicalMaxMembers;
	type MaxProposals = TechnicalMaxProposals;
	type MotionDuration = TechnicalMotionDuration;
	type Origin = Origin;
	type Proposal = Call;
	type WeightInfo = pallet_collective::weights::SubstrateWeight<Test>;
}

parameter_types! {
	pub const RelayChainTokenSymbol: TokenSymbol = TokenSymbol::KSM;
	pub const MaximumDepositInPool: Balance = 1_000_000 * UNIT;
	pub const MinimumDeposit: Balance = 1_000_000;
	pub const MinimumRewardPerBlock: Balance = 1_000;
	pub const MinimumDuration: BlockNumber = MINUTES;
	pub const MaximumApproved: u32 = 4;
	pub const MaximumOptionRewards: u32 = 7;
	pub const LiquidityMiningPalletId: PalletId = PalletId(*b"mining##");
}

impl lm::Config for Test {
	type Event = Event;
	type ControlOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
	type MultiCurrency = Tokens;
	type RelayChainTokenSymbol = RelayChainTokenSymbol;
	type MaximumDepositInPool = MaximumDepositInPool;
	type MinimumDepositOfUser = MinimumDeposit;
	type MinimumRewardPerBlock = MinimumRewardPerBlock;
	type MinimumDuration = MinimumDuration;
	type MaximumCharged = MaximumApproved;
	type MaximumOptionRewards = MaximumOptionRewards;
	type PalletId = LiquidityMiningPalletId;
	type WeightInfo = ();
}

pub(crate) fn new_test_ext() -> TestExternalities {
	GenesisConfig {
		tokens: orml_tokens::GenesisConfig::<Test> {
			balances: vec![
				(INVESTOR, REWARD_1, REWARD_AMOUNT),
				(INVESTOR, REWARD_2, REWARD_AMOUNT),
				(USER_1, FARMING_DEPOSIT_1, DEPOSIT_AMOUNT),
				(USER_1, FARMING_DEPOSIT_2, DEPOSIT_AMOUNT),
				(USER_2, FARMING_DEPOSIT_1, DEPOSIT_AMOUNT),
				(USER_2, FARMING_DEPOSIT_2, DEPOSIT_AMOUNT),
				(USER_1, MINING_DEPOSIT, DEPOSIT_AMOUNT),
				(USER_2, MINING_DEPOSIT, DEPOSIT_AMOUNT),
				(USER_1, SINGLE_TOKEN_DEPOSIT, DEPOSIT_AMOUNT),
				(USER_2, SINGLE_TOKEN_DEPOSIT, DEPOSIT_AMOUNT),
				(RICHER, FARMING_DEPOSIT_1, 1_000_000_000_000 * UNIT),
				(RICHER, FARMING_DEPOSIT_2, 1_000_000_000_000 * UNIT),
				(RICHER, MINING_DEPOSIT, 1_000_000_000_000 * UNIT),
				(RICHER, SINGLE_TOKEN_DEPOSIT, 1_000_000_000_000 * UNIT),
			],
		},
		collective: pallet_collective::GenesisConfig {
			members: vec![TC_MEMBER_1, TC_MEMBER_2, TC_MEMBER_3],
			phantom: Default::default(),
		},
		lm: crate::GenesisConfig {
			pallet_version: StorageVersion::V2_0_0,
			_phantom: Default::default(),
		},
	}
	.build_storage()
	.unwrap()
	.into()
}

pub(crate) const MINUTES: BlockNumber = 60 / (12 as BlockNumber);
pub(crate) const HOURS: BlockNumber = MINUTES * 60;
pub(crate) const DAYS: BlockNumber = HOURS * 24;

pub(crate) const REDEEM_LIMIT_TIME: BlockNumber = 100;
pub(crate) const UNLOCK_LIMIT_NUMS: u32 = 3;

pub(crate) const UNIT: Balance = 1_000_000_000_000;

pub(crate) const MINING_TRADING_PAIR: (CurrencyId, CurrencyId) =
	(CurrencyId::Token(TokenSymbol::DOT), CurrencyId::Token(TokenSymbol::KSM));
pub(crate) const MINING_DEPOSIT: CurrencyId =
	CurrencyId::LPToken(TokenSymbol::DOT, 2u8, TokenSymbol::KSM, 2u8);
pub(crate) const FARMING_DEPOSIT_1: CurrencyId = CurrencyId::VSToken(TokenSymbol::KSM);
pub(crate) const FARMING_DEPOSIT_2: CurrencyId = CurrencyId::VSBond(TokenSymbol::BNC, 2001, 13, 20);
pub(crate) const SINGLE_TOKEN_DEPOSIT: CurrencyId = CurrencyId::Token(TokenSymbol::ZLK);
pub(crate) const DEPOSIT_AMOUNT: Balance = UNIT;
pub(crate) const REWARD_1: CurrencyId = CurrencyId::Native(TokenSymbol::BNC);
pub(crate) const REWARD_2: CurrencyId = CurrencyId::Token(TokenSymbol::KSM);
pub(crate) const REWARD_AMOUNT: Balance = UNIT;

pub(crate) const INVESTOR: AccountId = AccountId::new([0u8; 32]);
pub(crate) const USER_1: AccountId = AccountId::new([1u8; 32]);
pub(crate) const USER_2: AccountId = AccountId::new([2u8; 32]);
pub(crate) const TC_MEMBER_1: AccountId = AccountId::new([3u8; 32]);
pub(crate) const TC_MEMBER_2: AccountId = AccountId::new([4u8; 32]);
pub(crate) const TC_MEMBER_3: AccountId = AccountId::new([5u8; 32]);
pub(crate) const RICHER: AccountId = AccountId::new([6u8; 32]);
pub(crate) const BEGGAR: AccountId = AccountId::new([7u8; 32]);

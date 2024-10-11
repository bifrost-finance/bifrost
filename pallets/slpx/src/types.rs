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

use crate::pallet;
use ethereum::TransactionAction;
use orml_traits::MultiCurrency;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::{H160, H256, U256};
use sp_runtime::{traits::ConstU32, BoundedVec, RuntimeDebug};
use sp_std::vec::Vec;
use xcm::prelude::Weight;

/// Max. allowed size of 65_536 bytes.
pub const MAX_ETHEREUM_XCM_INPUT_SIZE: u32 = 2u32.pow(16);

/// Max. allowed gas limit of 720_000 gas units. Note that this might change in the future.
pub const MAX_GAS_LIMIT: u32 = 720_000;

/// EVM function selector: setTokenAmount(bytes2,uint256,uint256)
pub const EVM_FUNCTION_SELECTOR: [u8; 4] = [154, 65, 185, 36];

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type CurrencyIdOf<T> = <<T as pallet::Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;
pub type BalanceOf<T> =
	<<T as pallet::Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum SupportChain {
	Astar,
	Moonbeam,
	Hydradx,
	Interlay,
	Manta,
}

#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum TargetChain<AccountId> {
	Astar(H160),
	Moonbeam(H160),
	Hydradx(AccountId),
	Interlay(AccountId),
	Manta(AccountId),
}

impl<AccountId> TargetChain<AccountId> {
	pub fn support_chain(self: &TargetChain<AccountId>) -> SupportChain {
		match self {
			TargetChain::Astar(_) => SupportChain::Astar,
			TargetChain::Moonbeam(_) => SupportChain::Moonbeam,
			TargetChain::Hydradx(_) => SupportChain::Hydradx,
			TargetChain::Interlay(_) => SupportChain::Interlay,
			TargetChain::Manta(_) => SupportChain::Manta,
		}
	}
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct EthereumCallConfiguration<BlockNumber> {
	/// XCM message execution costs to be consumed
	pub xcm_fee: u128,
	/// XCM message execution weight to be consumed
	pub xcm_weight: Weight,
	/// Wait for the period to call XCM once
	pub period: BlockNumber,
	/// Block number of the last call
	pub last_block: BlockNumber,
	/// Specify the address of the calling contract
	pub contract: H160,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub struct EthereumXcmTransactionV2 {
	/// Gas limit to be consumed by EVM execution.
	pub gas_limit: U256,
	/// Either a Call (the callee, account or contract address) or Create (currently unsupported).
	pub action: TransactionAction,
	/// Value to be transfered.
	pub value: U256,
	/// Input data for a contract call. Max. size 65_536 bytes.
	pub input: BoundedVec<u8, ConstU32<MAX_ETHEREUM_XCM_INPUT_SIZE>>,
	/// Map of addresses to be pre-paid to warm storage.
	pub access_list: Option<Vec<(H160, Vec<H256>)>>,
}

/// Xcm transact's Ethereum transaction.
#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo)]
pub enum EthereumXcmTransaction {
	V1,
	V2(EthereumXcmTransactionV2),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum EthereumXcmCall {
	#[codec(index = 0)]
	Transact(EthereumXcmTransaction),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum MoonbeamCall {
	#[codec(index = 109)]
	EthereumXcm(EthereumXcmCall),
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum OrderCaller<AccountId> {
	Substrate(AccountId),
	Evm(H160),
}

#[derive(Clone, Debug, Eq, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum OrderType {
	Mint,
	Redeem,
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Order<AccountId, CurrencyId, Balance, BlockNumber> {
	pub source_chain_caller: OrderCaller<AccountId>,
	pub source_chain_id: u64,
	pub source_chain_block_number: Option<u128>,
	pub bifrost_chain_caller: AccountId,
	pub derivative_account: AccountId,
	pub create_block_number: BlockNumber,
	pub currency_id: CurrencyId,
	pub currency_amount: Balance,
	pub order_type: OrderType,
	pub remark: BoundedVec<u8, ConstU32<32>>,
	pub target_chain: TargetChain<AccountId>,
	pub channel_id: u32,
}

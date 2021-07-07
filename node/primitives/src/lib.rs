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

//! Low-level types used throughout the Bifrost code.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature, OpaqueExtrinsic,
};
use sp_std::{convert::Into, prelude::*};

mod bridge;
mod currency;
mod tests;
pub mod traits;

pub use crate::{
	bridge::*,
	currency::{CurrencyId, TokenSymbol},
	traits::*,
};

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;

/// An index to an asset
pub type AssetId = u32;

/// Vtoken Mint type
pub type VtokenMintPrice = u128;

/// Balance of an account.
pub type Balance = u128;

/// Price of an asset.
pub type Price = u64;

/// Precision of symbol.
pub type Precision = u32;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
pub type Timestamp = u64;

/// Digest item type.
pub type DigestItem = generic::DigestItem<Hash>;

/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

/// Block type.
pub type Block = generic::Block<Header, OpaqueExtrinsic>;

/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// Balancer pool swap fee.
pub type SwapFee = u128;

/// Balancer pool ID.
pub type PoolId = u32;

/// Balancer pool weight.
pub type PoolWeight = u128;

/// Balancer pool token.
pub type PoolToken = u128;

/// Index of a transaction in the chain. 32-bit should be plenty.
pub type Nonce = u32;

///
pub type BiddingOrderId = u64;

///
pub type EraId = u32;

/// Signed version of Balance
pub type Amount = i128;

/// Parachain Id
pub type ParaId = u32;

/// The measurement type for counting lease periods (generally the same as `BlockNumber`).
pub type LeasePeriod = BlockNumber;

#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(Deserialize, Serialize))]
pub enum TokenType {
	/// Native token, only used by BNC
	Native,
	/// Stable token
	Stable,
	/// Origin token from bridge
	Token,
	// v-token of origin token
	VToken,
}

impl Default for TokenType {
	fn default() -> Self {
		Self::Native
	}
}

impl TokenType {
	pub fn is_base_token(&self) -> bool {
		*self == TokenType::Native
	}

	pub fn is_stable(&self) -> bool {
		*self == TokenType::Stable
	}

	pub fn is_token(&self) -> bool {
		*self == TokenType::Token
	}

	pub fn is_v_token(&self) -> bool {
		*self == TokenType::VToken
	}
}

/// Token struct
#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct Token<AssetId, Balance> {
	pub symbol: Vec<u8>,
	pub precision: u16,
	pub total_supply: Balance,
	pub token_type: TokenType,
	pub pair: Option<AssetId>,
}

impl<AssetId, Balance: Copy> Token<AssetId, Balance> {
	pub fn new(
		symbol: Vec<u8>,
		precision: u16,
		total_supply: Balance,
		token_type: TokenType,
	) -> Self {
		Self { symbol, precision, total_supply, token_type, pair: None }
	}

	pub fn add_pair(&mut self, asset_id: AssetId) {
		self.pair = Some(asset_id);
	}
}

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct AccountAsset<Balance> {
	pub balance: Balance,
	pub locked: Balance,
	pub available: Balance,
	pub cost: Balance,
	pub income: Balance,
}

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
pub struct VtokenPool<Balance> {
	/// A pool that hold the total amount of token minted to vtoken
	pub token_pool: Balance,
	/// A pool that hold the total amount of vtoken minted from token
	pub vtoken_pool: Balance,
	/// Total reward for current mint duration
	pub current_reward: Balance,
	/// Total reward for next mint duration
	pub pending_reward: Balance,
}

impl<Balance: Default + Copy> VtokenPool<Balance> {
	pub fn new(token_amount: Balance, vtoken_amount: Balance) -> Self {
		Self { token_pool: token_amount, vtoken_pool: vtoken_amount, ..Default::default() }
	}

	pub fn new_round(&mut self) {
		self.current_reward = self.pending_reward;
		self.pending_reward = Default::default();
	}
}

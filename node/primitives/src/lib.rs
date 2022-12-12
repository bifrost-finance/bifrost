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

//! Low-level types used throughout the Bifrost code.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::MaxEncodedLen;
use scale_info::TypeInfo;
use sp_core::{Decode, Encode, RuntimeDebug};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature, OpaqueExtrinsic,
};

pub mod currency;
mod salp;
pub mod traits;
pub use salp::*;

#[cfg(test)]
mod tests;

pub use crate::{
	currency::{
		AssetIds, CurrencyId, ForeignAssetId, TokenId, TokenSymbol, DOT, DOT_TOKEN_ID, FIL, GLMR,
		GLMR_TOKEN_ID,
	},
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
pub type DigestItem = generic::DigestItem;

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

/// Index used for the child trie
pub type TrieIndex = u32;

/// Distribution Id
pub type DistributionId = u32;

#[derive(
	Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord, scale_info::TypeInfo,
)]
pub enum ExtraFeeName {
	SalpContribute,
	StatemineTransfer,
	NoExtraFee,
}

// For vtoken-minting and slp modules
#[derive(Encode, Decode, Clone, RuntimeDebug, Eq, TypeInfo, MaxEncodedLen)]
pub enum TimeUnit {
	// Kusama staking time unit
	Era(#[codec(compact)] u32),
	SlashingSpan(#[codec(compact)] u32),
	// Moonriver staking time unit
	Round(#[codec(compact)] u32),
	// 1000 blocks. Can be used by Filecoin.
	// 30 seconds per block. Kblock means 8.33 hours.
	Kblock(#[codec(compact)] u32),
	// 1 hour. Should be Unix Timstamp in seconds / 3600
	Hour(#[codec(compact)] u32)
}

impl Default for TimeUnit {
	fn default() -> Self {
		TimeUnit::Era(0u32)
	}
}

impl PartialEq for TimeUnit {
	fn eq(&self, other: &Self) -> bool {
		match (&self, other) {
			(Self::Era(a), Self::Era(b)) => a.eq(b),
			(Self::SlashingSpan(a), Self::SlashingSpan(b)) => a.eq(b),
			(Self::Round(a), Self::Round(b)) => a.eq(b),
			(Self::Kblock(a), Self::Kblock(b)) => a.eq(b),
			_ => false,
		}
	}
}

impl Ord for TimeUnit {
	fn cmp(&self, other: &Self) -> sp_std::cmp::Ordering {
		match (&self, other) {
			(Self::Era(a), Self::Era(b)) => a.cmp(b),
			(Self::SlashingSpan(a), Self::SlashingSpan(b)) => a.cmp(b),
			(Self::Round(a), Self::Round(b)) => a.cmp(b),
			(Self::Kblock(a), Self::Kblock(b)) => a.cmp(b),
			_ => sp_std::cmp::Ordering::Less,
		}
	}
}

impl PartialOrd for TimeUnit {
	fn partial_cmp(&self, other: &Self) -> Option<sp_std::cmp::Ordering> {
		match (&self, other) {
			(Self::Era(a), Self::Era(b)) => Some(a.cmp(b)),
			(Self::SlashingSpan(a), Self::SlashingSpan(b)) => Some(a.cmp(b)),
			(Self::Round(a), Self::Round(b)) => Some(a.cmp(b)),
			(Self::Kblock(a), Self::Kblock(b)) => Some(a.cmp(b)),
			_ => None,
		}
	}
}

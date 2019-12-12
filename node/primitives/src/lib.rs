// Copyright 2019 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

//! Low-level types used throughout the Bifrost code.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode};
use rstd::prelude::*;
use sr_primitives::{
	generic,
	traits::{Verify, BlakeTwo256, IdentifyAccount},
	OpaqueExtrinsic,
	MultiSignature
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

/// Balance of an account.
pub type Balance = u128;

/// Precision of symbol.
pub type Precision = u32;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = primitives::H256;

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

/// Token type
#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
pub struct Token<Balance> {
	pub symbol: Vec<u8>,
	pub precision: u16,
	pub total_supply: Balance,
}

impl<Balance> Token<Balance> {
	pub fn new(symbol: Vec<u8>, precision: u16, total_supply: Balance) -> Self {
		Self {
			symbol,
			precision,
			total_supply,
		}
	}
}

/// Clearing handler for assets change
pub trait ClearingHandler<AssetId, AccountId, BlockNumber, Balance> {
	/// Clearing for assets change
	fn asset_clearing(
		asset_id: AssetId,
		target: AccountId,
		last_block: BlockNumber,
		prev_amount: Balance,
		curr_amount: Balance,
	);

	/// Clearing for token change
	fn token_clearing(
		asset_id: AssetId,
		last_block: BlockNumber,
		prev_amount: Balance,
		curr_amount: Balance,
	);
}

impl<A, AC, BN, B> ClearingHandler<A, AC, BN, B> for () {
	fn asset_clearing(_: A, _: AC, _: BN, _: B, _: B) {}
	fn token_clearing(_: A, _: BN, _: B, _: B) {}
}

/// Asset create handler
pub trait AssetCreate<AssetId, Balance> {
	/// Asset create
	fn asset_create(symbol: Vec<u8>, precision: u16) -> (AssetId, Token<Balance>);
}

impl<A: Default, B: Default> AssetCreate<A, B> for () {
	fn asset_create(_: Vec<u8>, _: u16) -> (A, Token<B>) { Default::default() }
}

/// Asset issue handler
pub trait AssetIssue<AssetId, AccountId, Balance> {
	/// Asset issue
	fn asset_issue(asset_id: AssetId, target: AccountId, amount: Balance);
}

impl<A, AC, B> AssetIssue<A, AC, B> for () {
	fn asset_issue(_: A, _: AC, _: B) {}
}

/// Asset redeem handler
pub trait AssetRedeem<AssetId, AccountId, Balance> {
	/// Asset redeem
	fn asset_redeem(asset_id: AssetId, target: AccountId, amount: Balance, to_name: Option<Vec<u8>>);
}

impl<A, AC, B> AssetRedeem<A, AC, B> for () {
	fn asset_redeem(_: A, _: AC, _: B, _: Option<Vec<u8>>) {}
}

/// Blockchain types
#[derive(PartialEq, Debug, Clone, Encode, Decode)]
pub enum BlockchainType {
	BIFROST,
	EOS,
}

impl Default for BlockchainType {
	fn default() -> Self {
		BlockchainType::BIFROST
	}
}

/// Symbol type of bridge asset
#[derive(Clone, Default, Encode, Decode)]
pub struct BridgeAssetSymbol<Precision> {
	pub blockchain: BlockchainType,
	pub symbol: Vec<u8>,
	pub precision: Precision,
}

/// Bridge asset type
#[derive(Clone, Default, Encode, Decode)]
pub struct BridgeAssetBalance<Precision, Balance> {
	pub symbol: BridgeAssetSymbol<Precision>,
	pub amount: Balance,
}

/// Bridge asset from other blockchain to Bifrost
pub trait BridgeAssetFrom<AccountId, Precision, Balance> {
	fn bridge_asset_from(target: AccountId, bridge_asset: BridgeAssetBalance<Precision, Balance>);
}

impl<A, P, B> BridgeAssetFrom<A, P, B> for () {
	fn bridge_asset_from(_: A, _: BridgeAssetBalance<P, B>) {}
}

/// Bridge asset from Bifrost to other blockchain
pub trait BridgeAssetTo<Precision, Balance> {
	fn bridge_asset_to(target: Vec<u8>, bridge_asset: BridgeAssetBalance<Precision, Balance>);
}

impl<P, B> BridgeAssetTo<P, B> for () {
	fn bridge_asset_to(_: Vec<u8>, _: BridgeAssetBalance<P, B>) {}
}
// Copyright 2019-2021 Liebi Technologies.
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

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature, OpaqueExtrinsic, RuntimeDebug, SaturatedConversion,
};
use sp_std::{convert::Into, prelude::*};

mod currency;
pub mod traits;

pub use crate::currency::{CurrencyId, TokenSymbol};
pub use crate::traits::{
	AssetReward, AssetTrait, CurrencyIdExt, DEXOperations, GetDecimals, RewardHandler,
	VtokenMintExt, MinterRewardExt
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

/// The balance of zenlink asset
pub type TokenBalance = u128;

/// The pair id of the zenlink dex.
pub type PairId = u32;

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

	pub fn is_stable_token(&self) -> bool {
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
		Self {
			symbol,
			precision,
			total_supply,
			token_type,
			pair: None,
		}
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
		Self {
			token_pool: token_amount,
			vtoken_pool: vtoken_amount,
			..Default::default()
		}
	}

	pub fn new_round(&mut self) {
		self.current_reward = self.pending_reward;
		self.pending_reward = Default::default();
	}
}

/// Blockchain types
#[derive(PartialEq, Debug, Clone, Encode, Decode)]
pub enum BlockchainType {
	BIFROST,
	EOS,
	IOST,
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

impl<Precision> BridgeAssetSymbol<Precision> {
	pub fn new(blockchain: BlockchainType, symbol: Vec<u8>, precision: Precision) -> Self {
		BridgeAssetSymbol {
			blockchain,
			symbol,
			precision,
		}
	}
}

/// Bridge asset type
#[derive(Clone, Default, Encode, Decode)]
pub struct BridgeAssetBalance<AccountId, AssetId, Precision, Balance> {
	pub symbol: BridgeAssetSymbol<Precision>,
	pub amount: Balance,
	pub memo: Vec<u8>,
	// store the account who send transaction to EOS
	pub from: AccountId,
	// which token type is sent to EOS
	pub asset_id: AssetId,
}

/// Zenlink type
#[derive(Encode, Decode, Eq, PartialEq, Clone, RuntimeDebug)]
pub struct Pair<AccountId, TokenBalance> {
	pub token_0: ZenlinkAssetId,
	pub token_1: ZenlinkAssetId,

	pub account: AccountId,
	pub total_liquidity: TokenBalance,
}

/// The id of Zenlink asset
/// NativeCurrency is this parachain native currency.
/// Other parachain's currency is represented by `ParaCurrency(u32)`, `u32` cast to the ParaId.
#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ZenlinkAssetId {
	NativeCurrency,
	ParaCurrency(u32),
}

impl ZenlinkAssetId {
	pub fn is_para_currency(&self) -> bool {
		matches!(self, ZenlinkAssetId::ParaCurrency(_))
	}
}

impl From<u32> for ZenlinkAssetId {
	fn from(id: u32) -> Self {
		ZenlinkAssetId::ParaCurrency(id)
	}
}

impl From<u128> for ZenlinkAssetId {
	fn from(id: u128) -> Self {
		ZenlinkAssetId::ParaCurrency(id as u32)
	}
}

impl From<CurrencyId> for ZenlinkAssetId {
	fn from(id: CurrencyId) -> Self {
		if id.is_native() {
			ZenlinkAssetId::NativeCurrency
		} else {
			match id {
				CurrencyId::Token(some_id) => {
					let u32_id = some_id as u32;
					ZenlinkAssetId::ParaCurrency(u32_id)
				}
				_ => todo!("Not support now."),
			}
		}
	}
}

impl Into<CurrencyId> for ZenlinkAssetId {
	fn into(self) -> CurrencyId {
		match self {
			ZenlinkAssetId::NativeCurrency => CurrencyId::Token(TokenSymbol::BNC),
			ZenlinkAssetId::ParaCurrency(some_id) => {
				let id: u8 = some_id.saturated_into();
				CurrencyId::Token(TokenSymbol::from(id))
			}
		}
	}
}

#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub enum StorageVersion {
	V0,
	V1,
	V2,
	V3,
}

impl Default for StorageVersion {
	fn default() -> Self {
		Self::V0
	}
}

/// App-specific crypto used for reporting equivocation/misbehavior in BABE and
/// GRANDPA. Any rewards for misbehavior reporting will be paid out to this
/// account.
pub mod report {
	use super::{Signature, Verify};
	use frame_system::offchain::AppCrypto;
	use sp_core::crypto::{key_types, KeyTypeId};

	/// Key type for the reporting module. Used for reporting BABE and GRANDPA
	/// equivocations.
	pub const KEY_TYPE: KeyTypeId = key_types::REPORTING;

	mod app {
		use sp_application_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, super::KEY_TYPE);
	}

	/// Identity of the equivocation/misbehavior reporter.
	pub type ReporterId = app::Public;

	/// An `AppCrypto` type to allow submitting signed transactions using the reporting
	/// application key as signer.
	pub struct ReporterAppCrypto;

	impl AppCrypto<<Signature as Verify>::Signer, Signature> for ReporterAppCrypto {
		type RuntimeAppPublic = ReporterId;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn currency_id_from_string_should_work() {
		let currency_id_str = "BNC";
		let bnc_currency_id = CurrencyId::try_from(currency_id_str.as_bytes().to_vec());
		assert!(bnc_currency_id.is_ok());
		assert_eq!(
			bnc_currency_id.unwrap(),
			CurrencyId::Token(TokenSymbol::BNC)
		);
	}

	#[test]
	fn currency_id_ext_test_should_work() {
		let currency_id_str = "BNC";
		let bnc_currency_id = CurrencyId::try_from(currency_id_str.as_bytes().to_vec());
		assert_eq!(bnc_currency_id.unwrap().is_native(), true);
		// assert_eq!(CurrencyId::from(TokenSymbol::vDOT),CurrencyId::Token(TokenSymbol::vDOT).);
		assert_eq!(TokenSymbol::DOT.decimals(), 10u32);
		assert_eq!(TokenSymbol::DOT as u8, 2u8);
		assert_eq!(CurrencyId::Token(TokenSymbol::vDOT).is_vtoken(), true);
		assert_eq!(CurrencyId::Token(TokenSymbol::aUSD).is_stable_token(), true);
		assert_eq!(
			CurrencyId::Token(TokenSymbol::BNC).get_native_token(),
			Some(TokenSymbol::BNC)
		);
		assert_eq!(
			CurrencyId::Token(TokenSymbol::vDOT).get_token_pair(),
			Some((TokenSymbol::DOT, TokenSymbol::vDOT))
		);
	}
}

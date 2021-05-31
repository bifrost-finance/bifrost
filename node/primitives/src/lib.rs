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
use core::convert::TryFrom;
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
	AssetReward, BridgeAssetFrom, BridgeAssetTo, CurrencyIdExt, MinterRewardExt, RewardHandler,
	RewardTrait, TokenInfo, VtokenMintExt,
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

/// Parachain Id
pub type ParaId = u32;

/// The measurement type for counting lease periods (generally the same as `BlockNumber`).
type LeasePeriod = BlockNumber;

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
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
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
			ZenlinkAssetId::NativeCurrency => CurrencyId::Native(TokenSymbol::ASG),
			ZenlinkAssetId::ParaCurrency(some_id) => {
				let id: u8 = some_id.saturated_into();
				CurrencyId::Token(TokenSymbol::try_from(id).unwrap())
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
		let currency_id = CurrencyId::try_from("DOT".as_bytes().to_vec());
		assert!(currency_id.is_ok());
		assert_eq!(currency_id.unwrap(), CurrencyId::Token(TokenSymbol::DOT));
	}

	#[test]
	fn currency_id_to_u64_should_work() {
		let e00 = CurrencyId::Token(TokenSymbol::ASG);
		let e01 = CurrencyId::Token(TokenSymbol::BNC);
		let e02 = CurrencyId::Token(TokenSymbol::AUSD);
		let e03 = CurrencyId::Token(TokenSymbol::DOT);
		let e04 = CurrencyId::Token(TokenSymbol::KSM);
		let e05 = CurrencyId::Token(TokenSymbol::ETH);

		assert_eq!(0x0000_0000_0000_0000, e00.currency_id());
		assert_eq!(0x0001_0000_0000_0000, e01.currency_id());
		assert_eq!(0x0002_0000_0000_0000, e02.currency_id());
		assert_eq!(0x0003_0000_0000_0000, e03.currency_id());
		assert_eq!(0x0004_0000_0000_0000, e04.currency_id());
		assert_eq!(0x0005_0000_0000_0000, e05.currency_id());

		let e10 = CurrencyId::VToken(TokenSymbol::ASG);
		let e11 = CurrencyId::VToken(TokenSymbol::BNC);
		let e12 = CurrencyId::VToken(TokenSymbol::AUSD);
		let e13 = CurrencyId::VToken(TokenSymbol::DOT);
		let e14 = CurrencyId::VToken(TokenSymbol::KSM);
		let e15 = CurrencyId::VToken(TokenSymbol::ETH);

		assert_eq!(0x0100_0000_0000_0000, e10.currency_id());
		assert_eq!(0x0101_0000_0000_0000, e11.currency_id());
		assert_eq!(0x0102_0000_0000_0000, e12.currency_id());
		assert_eq!(0x0103_0000_0000_0000, e13.currency_id());
		assert_eq!(0x0104_0000_0000_0000, e14.currency_id());
		assert_eq!(0x0105_0000_0000_0000, e15.currency_id());

		let e20 = CurrencyId::Native(TokenSymbol::ASG);
		let e21 = CurrencyId::Native(TokenSymbol::BNC);
		let e22 = CurrencyId::Native(TokenSymbol::AUSD);
		let e23 = CurrencyId::Native(TokenSymbol::DOT);
		let e24 = CurrencyId::Native(TokenSymbol::KSM);
		let e25 = CurrencyId::Native(TokenSymbol::ETH);

		assert_eq!(0x0200_0000_0000_0000, e20.currency_id());
		assert_eq!(0x0201_0000_0000_0000, e21.currency_id());
		assert_eq!(0x0202_0000_0000_0000, e22.currency_id());
		assert_eq!(0x0203_0000_0000_0000, e23.currency_id());
		assert_eq!(0x0204_0000_0000_0000, e24.currency_id());
		assert_eq!(0x0205_0000_0000_0000, e25.currency_id());

		let e30 = CurrencyId::Stable(TokenSymbol::ASG);
		let e31 = CurrencyId::Stable(TokenSymbol::BNC);
		let e32 = CurrencyId::Stable(TokenSymbol::AUSD);
		let e33 = CurrencyId::Stable(TokenSymbol::DOT);
		let e34 = CurrencyId::Stable(TokenSymbol::KSM);
		let e35 = CurrencyId::Stable(TokenSymbol::ETH);

		assert_eq!(0x0300_0000_0000_0000, e30.currency_id());
		assert_eq!(0x0301_0000_0000_0000, e31.currency_id());
		assert_eq!(0x0302_0000_0000_0000, e32.currency_id());
		assert_eq!(0x0303_0000_0000_0000, e33.currency_id());
		assert_eq!(0x0304_0000_0000_0000, e34.currency_id());
		assert_eq!(0x0305_0000_0000_0000, e35.currency_id());

		let e40 = CurrencyId::VSToken(TokenSymbol::ASG);
		let e41 = CurrencyId::VSToken(TokenSymbol::BNC);
		let e42 = CurrencyId::VSToken(TokenSymbol::AUSD);
		let e43 = CurrencyId::VSToken(TokenSymbol::DOT);
		let e44 = CurrencyId::VSToken(TokenSymbol::KSM);
		let e45 = CurrencyId::VSToken(TokenSymbol::ETH);

		assert_eq!(0x0400_0000_0000_0000, e40.currency_id());
		assert_eq!(0x0401_0000_0000_0000, e41.currency_id());
		assert_eq!(0x0402_0000_0000_0000, e42.currency_id());
		assert_eq!(0x0403_0000_0000_0000, e43.currency_id());
		assert_eq!(0x0404_0000_0000_0000, e44.currency_id());
		assert_eq!(0x0405_0000_0000_0000, e45.currency_id());

		let e50 = CurrencyId::VSBond(TokenSymbol::ASG, 0x07d0, 0x0000, 0x000f);
		let e51 = CurrencyId::VSBond(TokenSymbol::BNC, 0x07d1, 0x000f, 0x001f);
		let e52 = CurrencyId::VSBond(TokenSymbol::AUSD, 0x07d2, 0x001f, 0x002f);
		let e53 = CurrencyId::VSBond(TokenSymbol::DOT, 0x07d3, 0x002f, 0x003f);
		let e54 = CurrencyId::VSBond(TokenSymbol::KSM, 0x07d4, 0x003f, 0x004f);
		let e55 = CurrencyId::VSBond(TokenSymbol::ETH, 0x07d5, 0x004f, 0x005f);

		assert_eq!(0x0500_07d0_0000_000f, e50.currency_id());
		assert_eq!(0x0501_07d1_000f_001f, e51.currency_id());
		assert_eq!(0x0502_07d2_001f_002f, e52.currency_id());
		assert_eq!(0x0503_07d3_002f_003f, e53.currency_id());
		assert_eq!(0x0504_07d4_003f_004f, e54.currency_id());
		assert_eq!(0x0505_07d5_004f_005f, e55.currency_id());
	}

	#[test]
	fn u64_to_currency_id_should_work() {
		let e00 = CurrencyId::Token(TokenSymbol::ASG);
		let e01 = CurrencyId::Token(TokenSymbol::BNC);
		let e02 = CurrencyId::Token(TokenSymbol::AUSD);
		let e03 = CurrencyId::Token(TokenSymbol::DOT);
		let e04 = CurrencyId::Token(TokenSymbol::KSM);
		let e05 = CurrencyId::Token(TokenSymbol::ETH);

		assert_eq!(e00, CurrencyId::try_from(0x0000_0000_0000_0000).unwrap());
		assert_eq!(e01, CurrencyId::try_from(0x0001_0000_0000_0000).unwrap());
		assert_eq!(e02, CurrencyId::try_from(0x0002_0000_0000_0000).unwrap());
		assert_eq!(e03, CurrencyId::try_from(0x0003_0000_0000_0000).unwrap());
		assert_eq!(e04, CurrencyId::try_from(0x0004_0000_0000_0000).unwrap());
		assert_eq!(e05, CurrencyId::try_from(0x0005_0000_0000_0000).unwrap());

		let e10 = CurrencyId::VToken(TokenSymbol::ASG);
		let e11 = CurrencyId::VToken(TokenSymbol::BNC);
		let e12 = CurrencyId::VToken(TokenSymbol::AUSD);
		let e13 = CurrencyId::VToken(TokenSymbol::DOT);
		let e14 = CurrencyId::VToken(TokenSymbol::KSM);
		let e15 = CurrencyId::VToken(TokenSymbol::ETH);

		assert_eq!(e10, CurrencyId::try_from(0x0100_0000_0000_0000).unwrap());
		assert_eq!(e11, CurrencyId::try_from(0x0101_0000_0000_0000).unwrap());
		assert_eq!(e12, CurrencyId::try_from(0x0102_0000_0000_0000).unwrap());
		assert_eq!(e13, CurrencyId::try_from(0x0103_0000_0000_0000).unwrap());
		assert_eq!(e14, CurrencyId::try_from(0x0104_0000_0000_0000).unwrap());
		assert_eq!(e15, CurrencyId::try_from(0x0105_0000_0000_0000).unwrap());

		let e20 = CurrencyId::Native(TokenSymbol::ASG);
		let e21 = CurrencyId::Native(TokenSymbol::BNC);
		let e22 = CurrencyId::Native(TokenSymbol::AUSD);
		let e23 = CurrencyId::Native(TokenSymbol::DOT);
		let e24 = CurrencyId::Native(TokenSymbol::KSM);
		let e25 = CurrencyId::Native(TokenSymbol::ETH);

		assert_eq!(e20, CurrencyId::try_from(0x0200_0000_0000_0000).unwrap());
		assert_eq!(e21, CurrencyId::try_from(0x0201_0000_0000_0000).unwrap());
		assert_eq!(e22, CurrencyId::try_from(0x0202_0000_0000_0000).unwrap());
		assert_eq!(e23, CurrencyId::try_from(0x0203_0000_0000_0000).unwrap());
		assert_eq!(e24, CurrencyId::try_from(0x0204_0000_0000_0000).unwrap());
		assert_eq!(e25, CurrencyId::try_from(0x0205_0000_0000_0000).unwrap());

		let e30 = CurrencyId::Stable(TokenSymbol::ASG);
		let e31 = CurrencyId::Stable(TokenSymbol::BNC);
		let e32 = CurrencyId::Stable(TokenSymbol::AUSD);
		let e33 = CurrencyId::Stable(TokenSymbol::DOT);
		let e34 = CurrencyId::Stable(TokenSymbol::KSM);
		let e35 = CurrencyId::Stable(TokenSymbol::ETH);

		assert_eq!(e30, CurrencyId::try_from(0x0300_0000_0000_0000).unwrap());
		assert_eq!(e31, CurrencyId::try_from(0x0301_0000_0000_0000).unwrap());
		assert_eq!(e32, CurrencyId::try_from(0x0302_0000_0000_0000).unwrap());
		assert_eq!(e33, CurrencyId::try_from(0x0303_0000_0000_0000).unwrap());
		assert_eq!(e34, CurrencyId::try_from(0x0304_0000_0000_0000).unwrap());
		assert_eq!(e35, CurrencyId::try_from(0x0305_0000_0000_0000).unwrap());

		let e40 = CurrencyId::VSToken(TokenSymbol::ASG);
		let e41 = CurrencyId::VSToken(TokenSymbol::BNC);
		let e42 = CurrencyId::VSToken(TokenSymbol::AUSD);
		let e43 = CurrencyId::VSToken(TokenSymbol::DOT);
		let e44 = CurrencyId::VSToken(TokenSymbol::KSM);
		let e45 = CurrencyId::VSToken(TokenSymbol::ETH);

		assert_eq!(e40, CurrencyId::try_from(0x0400_0000_0000_0000).unwrap());
		assert_eq!(e41, CurrencyId::try_from(0x0401_0000_0000_0000).unwrap());
		assert_eq!(e42, CurrencyId::try_from(0x0402_0000_0000_0000).unwrap());
		assert_eq!(e43, CurrencyId::try_from(0x0403_0000_0000_0000).unwrap());
		assert_eq!(e44, CurrencyId::try_from(0x0404_0000_0000_0000).unwrap());
		assert_eq!(e45, CurrencyId::try_from(0x0405_0000_0000_0000).unwrap());

		let e50 = CurrencyId::VSBond(TokenSymbol::ASG, 0x07d0, 0x0000, 0x000f);
		let e51 = CurrencyId::VSBond(TokenSymbol::BNC, 0x07d1, 0x000f, 0x001f);
		let e52 = CurrencyId::VSBond(TokenSymbol::AUSD, 0x07d2, 0x001f, 0x002f);
		let e53 = CurrencyId::VSBond(TokenSymbol::DOT, 0x07d3, 0x002f, 0x003f);
		let e54 = CurrencyId::VSBond(TokenSymbol::KSM, 0x07d4, 0x003f, 0x004f);
		let e55 = CurrencyId::VSBond(TokenSymbol::ETH, 0x07d5, 0x004f, 0x005f);

		assert_eq!(e50, CurrencyId::try_from(0x0500_07d0_0000_000f).unwrap());
		assert_eq!(e51, CurrencyId::try_from(0x0501_07d1_000f_001f).unwrap());
		assert_eq!(e52, CurrencyId::try_from(0x0502_07d2_001f_002f).unwrap());
		assert_eq!(e53, CurrencyId::try_from(0x0503_07d3_002f_003f).unwrap());
		assert_eq!(e54, CurrencyId::try_from(0x0504_07d4_003f_004f).unwrap());
		assert_eq!(e55, CurrencyId::try_from(0x0505_07d5_004f_005f).unwrap());
	}
}

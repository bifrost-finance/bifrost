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
	CurrencyIdExt, AssetReward, RewardHandler, VtokenMintExt, MinterRewardExt, RewardTrait,
	BridgeAssetFrom, BridgeAssetTo, TokenInfo,
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
	fn currency_id_to_u32_should_work() {
		let e00 = CurrencyId::Token(TokenSymbol::ASG);
		let e01 = CurrencyId::Token(TokenSymbol::BNC);
		let e02 = CurrencyId::Token(TokenSymbol::AUSD);
		let e03 = CurrencyId::Token(TokenSymbol::DOT);
		let e04 = CurrencyId::Token(TokenSymbol::KSM);
		let e05 = CurrencyId::Token(TokenSymbol::ETH);

		assert_eq!(0x00_00_00_00, e00.currency_id());
		assert_eq!(0x01_00_00_00, e01.currency_id());
		assert_eq!(0x02_00_00_00, e02.currency_id());
		assert_eq!(0x03_00_00_00, e03.currency_id());
		assert_eq!(0x04_00_00_00, e04.currency_id());
		assert_eq!(0x05_00_00_00, e05.currency_id());

		let e10 = CurrencyId::VToken(TokenSymbol::ASG);
		let e11 = CurrencyId::VToken(TokenSymbol::BNC);
		let e12 = CurrencyId::VToken(TokenSymbol::AUSD);
		let e13 = CurrencyId::VToken(TokenSymbol::DOT);
		let e14 = CurrencyId::VToken(TokenSymbol::KSM);
		let e15 = CurrencyId::VToken(TokenSymbol::ETH);

		assert_eq!(0x10_00_00_00, e10.currency_id());
		assert_eq!(0x11_00_00_00, e11.currency_id());
		assert_eq!(0x12_00_00_00, e12.currency_id());
		assert_eq!(0x13_00_00_00, e13.currency_id());
		assert_eq!(0x14_00_00_00, e14.currency_id());
		assert_eq!(0x15_00_00_00, e15.currency_id());

		let e20 = CurrencyId::Native(TokenSymbol::ASG);
		let e21 = CurrencyId::Native(TokenSymbol::BNC);
		let e22 = CurrencyId::Native(TokenSymbol::AUSD);
		let e23 = CurrencyId::Native(TokenSymbol::DOT);
		let e24 = CurrencyId::Native(TokenSymbol::KSM);
		let e25 = CurrencyId::Native(TokenSymbol::ETH);

		assert_eq!(0x20_00_00_00, e20.currency_id());
		assert_eq!(0x21_00_00_00, e21.currency_id());
		assert_eq!(0x22_00_00_00, e22.currency_id());
		assert_eq!(0x23_00_00_00, e23.currency_id());
		assert_eq!(0x24_00_00_00, e24.currency_id());
		assert_eq!(0x25_00_00_00, e25.currency_id());

		let e30 = CurrencyId::Stable(TokenSymbol::ASG);
		let e31 = CurrencyId::Stable(TokenSymbol::BNC);
		let e32 = CurrencyId::Stable(TokenSymbol::AUSD);
		let e33 = CurrencyId::Stable(TokenSymbol::DOT);
		let e34 = CurrencyId::Stable(TokenSymbol::KSM);
		let e35 = CurrencyId::Stable(TokenSymbol::ETH);

		assert_eq!(0x30_00_00_00, e30.currency_id());
		assert_eq!(0x31_00_00_00, e31.currency_id());
		assert_eq!(0x32_00_00_00, e32.currency_id());
		assert_eq!(0x33_00_00_00, e33.currency_id());
		assert_eq!(0x34_00_00_00, e34.currency_id());
		assert_eq!(0x35_00_00_00, e35.currency_id());

		let e40 = CurrencyId::VSToken(TokenSymbol::ASG);
		let e41 = CurrencyId::VSToken(TokenSymbol::BNC);
		let e42 = CurrencyId::VSToken(TokenSymbol::AUSD);
		let e43 = CurrencyId::VSToken(TokenSymbol::DOT);
		let e44 = CurrencyId::VSToken(TokenSymbol::KSM);
		let e45 = CurrencyId::VSToken(TokenSymbol::ETH);

		assert_eq!(0x40_00_00_00, e40.currency_id());
		assert_eq!(0x41_00_00_00, e41.currency_id());
		assert_eq!(0x42_00_00_00, e42.currency_id());
		assert_eq!(0x43_00_00_00, e43.currency_id());
		assert_eq!(0x44_00_00_00, e44.currency_id());
		assert_eq!(0x45_00_00_00, e45.currency_id());

		let e50 = CurrencyId::VSBond(TokenSymbol::ASG, 0x01, 0x00, 0xf0);
		let e51 = CurrencyId::VSBond(TokenSymbol::BNC, 0x02, 0x01, 0xf1);
		let e52 = CurrencyId::VSBond(TokenSymbol::AUSD, 0x03, 0x02, 0xf2);
		let e53 = CurrencyId::VSBond(TokenSymbol::DOT, 0x04, 0x03, 0xf3);
		let e54 = CurrencyId::VSBond(TokenSymbol::KSM, 0x05, 0x04, 0xf4);
		let e55 = CurrencyId::VSBond(TokenSymbol::ETH, 0x06, 0x05, 0xf5);

		assert_eq!(0x50_01_00_f0, e50.currency_id());
		assert_eq!(0x51_02_01_f1, e51.currency_id());
		assert_eq!(0x52_03_02_f2, e52.currency_id());
		assert_eq!(0x53_04_03_f3, e53.currency_id());
		assert_eq!(0x54_05_04_f4, e54.currency_id());
		assert_eq!(0x55_06_05_f5, e55.currency_id());
	}

	#[test]
	fn u32_to_currency_id_should_work() {
		let e00 = CurrencyId::Token(TokenSymbol::ASG);
		let e01 = CurrencyId::Token(TokenSymbol::BNC);
		let e02 = CurrencyId::Token(TokenSymbol::AUSD);
		let e03 = CurrencyId::Token(TokenSymbol::DOT);
		let e04 = CurrencyId::Token(TokenSymbol::KSM);
		let e05 = CurrencyId::Token(TokenSymbol::ETH);

		assert_eq!(e00, CurrencyId::try_from(0x00_00_00_00).unwrap());
		assert_eq!(e01, CurrencyId::try_from(0x01_00_00_00).unwrap());
		assert_eq!(e02, CurrencyId::try_from(0x02_00_00_00).unwrap());
		assert_eq!(e03, CurrencyId::try_from(0x03_00_00_00).unwrap());
		assert_eq!(e04, CurrencyId::try_from(0x04_00_00_00).unwrap());
		assert_eq!(e05, CurrencyId::try_from(0x05_00_00_00).unwrap());

		let e10 = CurrencyId::VToken(TokenSymbol::ASG);
		let e11 = CurrencyId::VToken(TokenSymbol::BNC);
		let e12 = CurrencyId::VToken(TokenSymbol::AUSD);
		let e13 = CurrencyId::VToken(TokenSymbol::DOT);
		let e14 = CurrencyId::VToken(TokenSymbol::KSM);
		let e15 = CurrencyId::VToken(TokenSymbol::ETH);

		assert_eq!(e10, CurrencyId::try_from(0x10_00_00_00).unwrap());
		assert_eq!(e11, CurrencyId::try_from(0x11_00_00_00).unwrap());
		assert_eq!(e12, CurrencyId::try_from(0x12_00_00_00).unwrap());
		assert_eq!(e13, CurrencyId::try_from(0x13_00_00_00).unwrap());
		assert_eq!(e14, CurrencyId::try_from(0x14_00_00_00).unwrap());
		assert_eq!(e15, CurrencyId::try_from(0x15_00_00_00).unwrap());

		let e20 = CurrencyId::Native(TokenSymbol::ASG);
		let e21 = CurrencyId::Native(TokenSymbol::BNC);
		let e22 = CurrencyId::Native(TokenSymbol::AUSD);
		let e23 = CurrencyId::Native(TokenSymbol::DOT);
		let e24 = CurrencyId::Native(TokenSymbol::KSM);
		let e25 = CurrencyId::Native(TokenSymbol::ETH);

		assert_eq!(e20, CurrencyId::try_from(0x20_00_00_00).unwrap());
		assert_eq!(e21, CurrencyId::try_from(0x21_00_00_00).unwrap());
		assert_eq!(e22, CurrencyId::try_from(0x22_00_00_00).unwrap());
		assert_eq!(e23, CurrencyId::try_from(0x23_00_00_00).unwrap());
		assert_eq!(e24, CurrencyId::try_from(0x24_00_00_00).unwrap());
		assert_eq!(e25, CurrencyId::try_from(0x25_00_00_00).unwrap());

		let e30 = CurrencyId::Stable(TokenSymbol::ASG);
		let e31 = CurrencyId::Stable(TokenSymbol::BNC);
		let e32 = CurrencyId::Stable(TokenSymbol::AUSD);
		let e33 = CurrencyId::Stable(TokenSymbol::DOT);
		let e34 = CurrencyId::Stable(TokenSymbol::KSM);
		let e35 = CurrencyId::Stable(TokenSymbol::ETH);

		assert_eq!(e30, CurrencyId::try_from(0x30_00_00_00).unwrap());
		assert_eq!(e31, CurrencyId::try_from(0x31_00_00_00).unwrap());
		assert_eq!(e32, CurrencyId::try_from(0x32_00_00_00).unwrap());
		assert_eq!(e33, CurrencyId::try_from(0x33_00_00_00).unwrap());
		assert_eq!(e34, CurrencyId::try_from(0x34_00_00_00).unwrap());
		assert_eq!(e35, CurrencyId::try_from(0x35_00_00_00).unwrap());

		let e40 = CurrencyId::VSToken(TokenSymbol::ASG);
		let e41 = CurrencyId::VSToken(TokenSymbol::BNC);
		let e42 = CurrencyId::VSToken(TokenSymbol::AUSD);
		let e43 = CurrencyId::VSToken(TokenSymbol::DOT);
		let e44 = CurrencyId::VSToken(TokenSymbol::KSM);
		let e45 = CurrencyId::VSToken(TokenSymbol::ETH);

		assert_eq!(e40, CurrencyId::try_from(0x40_00_00_00).unwrap());
		assert_eq!(e41, CurrencyId::try_from(0x41_00_00_00).unwrap());
		assert_eq!(e42, CurrencyId::try_from(0x42_00_00_00).unwrap());
		assert_eq!(e43, CurrencyId::try_from(0x43_00_00_00).unwrap());
		assert_eq!(e44, CurrencyId::try_from(0x44_00_00_00).unwrap());
		assert_eq!(e45, CurrencyId::try_from(0x45_00_00_00).unwrap());

		let e50 = CurrencyId::VSBond(TokenSymbol::ASG, 0x01, 0x00, 0xf0);
		let e51 = CurrencyId::VSBond(TokenSymbol::BNC, 0x02, 0x01, 0xf1);
		let e52 = CurrencyId::VSBond(TokenSymbol::AUSD, 0x03, 0x02, 0xf2);
		let e53 = CurrencyId::VSBond(TokenSymbol::DOT, 0x04, 0x03, 0xf3);
		let e54 = CurrencyId::VSBond(TokenSymbol::KSM, 0x05, 0x04, 0xf4);
		let e55 = CurrencyId::VSBond(TokenSymbol::ETH, 0x06, 0x05, 0xf5);

		assert_eq!(e50, CurrencyId::try_from(0x50_01_00_f0).unwrap());
		assert_eq!(e51, CurrencyId::try_from(0x51_02_01_f1).unwrap());
		assert_eq!(e52, CurrencyId::try_from(0x52_03_02_f2).unwrap());
		assert_eq!(e53, CurrencyId::try_from(0x53_04_03_f3).unwrap());
		assert_eq!(e54, CurrencyId::try_from(0x54_05_04_f4).unwrap());
		assert_eq!(e55, CurrencyId::try_from(0x55_06_05_f5).unwrap());
	}
}

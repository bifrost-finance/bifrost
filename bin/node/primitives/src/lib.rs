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

use codec::{Encode, Decode};
use sp_runtime::{
	generic, traits::{Verify, BlakeTwo256, IdentifyAccount}, OpaqueExtrinsic, MultiSignature
};
use sp_std::{
	convert::{Into, TryFrom, TryInto},
	prelude::*,
};
use sp_runtime::RuntimeDebug;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

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


#[derive(Encode, Decode, Clone, Copy, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(serde::Deserialize, serde::Serialize))]
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
	pub fn new(symbol: Vec<u8>, precision: u16, total_supply: Balance, token_type: TokenType) -> Self {
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

pub trait AssetTrait<AssetId, AccountId, Balance> {
	type Error;
	fn asset_create(symbol: Vec<u8>, precision: u16, token_type: TokenType) -> Result<(AssetId, Token<AssetId, Balance>), Self::Error>;

	fn asset_create_pair(symbol: Vec<u8>, precision: u16) -> Result<(AssetId, AssetId), Self::Error>;

	fn asset_issue(asset_id: AssetId, target: &AccountId, amount: Balance);

	fn asset_redeem(asset_id: AssetId, target: &AccountId, amount: Balance);

	fn asset_destroy(asset_id: AssetId, target: &AccountId, amount: Balance);

	fn asset_id_exists(who: &AccountId, symbol: &[u8], precision: u16) -> Option<AssetId>;

	fn token_exists(asset_id: AssetId) -> bool;

	fn get_account_asset(asset_id: AssetId, target: &AccountId) -> AccountAsset<Balance>;

	fn get_token(asset_id: AssetId) -> Token<AssetId, Balance>;

	fn lock_asset(who: &AccountId, asset_id: AssetId, locked: Balance);

	fn unlock_asset(who: &AccountId, asset_id: AssetId, unlocked: Balance);

	fn is_token(asset_id: AssetId) -> bool;

	fn is_v_token(asset_id: AssetId) -> bool;

	fn get_pair(asset_id: AssetId) -> Option<AssetId>;
}

impl<AssetId, AccountId, Balance> AssetTrait<AssetId, AccountId, Balance> for ()
	where AssetId: Default, AccountId: Default, Balance: Default
{
	type Error = core::convert::Infallible;
	fn asset_create(_: Vec<u8>, _: u16, _: TokenType) -> Result<(AssetId, Token<AssetId, Balance>), Self::Error> { Ok(Default::default()) }

	fn asset_create_pair(_: Vec<u8>, _: u16) -> Result<(AssetId, AssetId), Self::Error> { Ok(Default::default()) }

	fn asset_issue(_: AssetId, _: &AccountId, _: Balance) {}

	fn asset_redeem(_: AssetId, _: &AccountId, _: Balance) {}

	fn asset_destroy(_: AssetId, _: &AccountId, _: Balance) {}

	fn asset_id_exists(_: &AccountId, _: &[u8], _: u16) -> Option<AssetId> { Default::default() }

	fn token_exists(_: AssetId) -> bool { Default::default() }

	fn get_account_asset(_: AssetId, _: &AccountId) -> AccountAsset<Balance> { Default::default() }

	fn get_token(_: AssetId) -> Token<AssetId, Balance> { Default::default() }

	fn lock_asset( _: &AccountId, _: AssetId, _: Balance) {}

	fn unlock_asset( _: &AccountId, _: AssetId, _: Balance) {}

	fn is_token(_: AssetId) -> bool { Default::default() }

	fn is_v_token(_: AssetId) -> bool { Default::default() }

	fn get_pair(_: AssetId) -> Option<AssetId> { Default::default() }
}

pub trait TokenPriceHandler<AssetId, Price> {
	fn set_token_price(asset_id: AssetId, price: Price);
}

impl<Price> TokenPriceHandler<AssetId, Price> for () {
	fn set_token_price(_: AssetId, _: Price) {}
}

/// Asset redeem handler
pub trait AssetRedeem<AssetId, AccountId, Balance> {
	/// Asset redeem
	fn asset_redeem(asset_id: AssetId, target: AccountId, amount: Balance, to_name: Option<Vec<u8>>);
}

impl<A, AC, B> AssetRedeem<A, AC, B> for () {
	fn asset_redeem(_: A, _: AC, _: B, _: Option<Vec<u8>>) {}
}

/// Fetch vtoken mint rate handler
pub trait FetchVtokenMintPrice<AssetId, VtokenMintPrice> {
	/// fetch vtoken mint rate
	fn fetch_vtoken_price(asset_id: AssetId) -> VtokenMintPrice;
}

/// Fetch vtoken mint rate handler
pub trait FetchVtokenMintPool<AssetId, Balance> {
	/// fetch vtoken mint pool for calculate vtoken mint price
	fn fetch_vtoken_pool(asset_id: AssetId) -> VtokenPool<Balance>;
}

impl<AssetId, ER: Default> FetchVtokenMintPrice<AssetId, ER> for () {
	fn fetch_vtoken_price(_: AssetId) -> ER { Default::default() }
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

/// Bridge asset from other blockchain to Bifrost
pub trait BridgeAssetFrom<AccountId, AssetId, Precision, Balance> {
	fn bridge_asset_from(target: AccountId, bridge_asset: BridgeAssetBalance<AccountId, AssetId, Precision, Balance>);
}

impl<A, AI, P, B> BridgeAssetFrom<A, AI, P, B> for () {
	fn bridge_asset_from(_: A, _: BridgeAssetBalance<A, AI, P, B>) {}
}

/// Bridge asset from Bifrost to other blockchain
pub trait BridgeAssetTo<AccountId, AssetId, Precision, Balance> {
	type Error;
	fn bridge_asset_to(target: Vec<u8>, bridge_asset: BridgeAssetBalance<AccountId, AssetId, Precision, Balance>, ) -> Result<(), Self::Error>;
	fn redeem(asset_id: AssetId, amount: Balance, validator_address: Vec<u8>) -> Result<(), Self::Error>;
	fn stake(asset_id: AssetId, amount: Balance, validator_address: Vec<u8>) -> Result<(), Self::Error>;
	fn unstake(asset_id: AssetId, amount: Balance, validator_address: Vec<u8>) -> Result<(), Self::Error>;
}

impl<A, AI, P, B> BridgeAssetTo<A, AI, P, B> for () {
	type Error = core::convert::Infallible;
	fn bridge_asset_to(_: Vec<u8>, _: BridgeAssetBalance<A, AI, P, B>) -> Result<(), Self::Error> { Ok(()) }
	fn redeem(_: AI, _: B, _: Vec<u8>) -> Result<(), Self::Error> { Ok(()) }
	fn stake(_: AI, _: B, _: Vec<u8>) -> Result<(), Self::Error> { Ok(()) }
	fn unstake(_: AI, _: B, _: Vec<u8>) -> Result<(), Self::Error> { Ok(()) }
}

pub trait AssetReward<AssetId, Balance> {
	type Output;
	type Error;
	fn set_asset_reward(asset_id: AssetId, reward: Balance) -> Result<Self::Output, Self::Error>;
}

impl<A, B> AssetReward<A, B> for () {
	type Output = ();
	type Error = core::convert::Infallible;
	fn set_asset_reward(_: A, _: B) -> Result<Self::Output, Self::Error> { Ok(()) }
}

pub trait RewardHandler<AssetId, Balance> {
	fn send_reward(asset_id: AssetId, reward: Balance);
}

impl<A, B> RewardHandler<A, B> for () {
	fn send_reward(_: A, _: B) {}
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

pub trait RewardTrait<Balance, AccountId, AssetId> {
	type Error;
	fn record_reward(v_token_id: AssetId, vtoken_mint_amount: Balance, referer: AccountId) -> Result<(), Self::Error>;
	fn dispatch_reward(v_token_id: AssetId, staking_profit: Balance) -> Result<(), Self::Error>;
}

impl<A, B, AI> RewardTrait<A, B, AI> for () {
	type Error = core::convert::Infallible;
	fn record_reward(_: AI, _: A, _: B) -> Result<(), Self::Error> { Ok(()) }
	fn dispatch_reward(_: AI, _: A) -> Result<(), Self::Error> { Ok(()) }
}


#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TokenSymbol {
	BNC = 0,
	AUSD = 1,
	DOT = 2,
	KSM = 3,
}

impl TryFrom<u8> for TokenSymbol {
	type Error = ();

	fn try_from(v: u8) -> Result<Self, Self::Error> {
		match v {
			0 => Ok(TokenSymbol::BNC),
			1 => Ok(TokenSymbol::AUSD),
			2 => Ok(TokenSymbol::DOT),
			3 => Ok(TokenSymbol::KSM),
			_ => Err(()),
		}
	}
}

#[derive(Encode, Decode, Eq, PartialEq, Copy, Clone, RuntimeDebug, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	Token(TokenSymbol),
}

impl CurrencyId {
	pub fn is_token_currency_id(&self) -> bool {
		matches!(self, CurrencyId::Token(_))
	}
}

impl TryFrom<Vec<u8>> for CurrencyId {
	type Error = ();
	fn try_from(v: Vec<u8>) -> Result<CurrencyId, ()> {
		match v.as_slice() {
			b"BNC" => Ok(CurrencyId::Token(TokenSymbol::BNC)),
			b"AUSD" => Ok(CurrencyId::Token(TokenSymbol::AUSD)),
			b"DOT" => Ok(CurrencyId::Token(TokenSymbol::DOT)),
			b"KSM" => Ok(CurrencyId::Token(TokenSymbol::KSM)),
			_ => Err(()),
		}
	}
}

/// Note the pre-deployed ERC20 contracts depend on `CurrencyId` implementation,
/// and need to be updated if any change.
impl TryFrom<[u8; 32]> for CurrencyId {
	type Error = ();

	fn try_from(v: [u8; 32]) -> Result<Self, Self::Error> {
		if !v.starts_with(&[0u8; 29][..]) {
			return Err(());
		}

		// token
		if v[29] == 0 && v[31] == 0 {
			return v[30].try_into().map(CurrencyId::Token);
		}

		Err(())
	}
}

/// Note the pre-deployed ERC20 contracts depend on `CurrencyId` implementation,
/// and need to be updated if any change.
impl From<CurrencyId> for [u8; 32] {
	fn from(val: CurrencyId) -> Self {
		let mut bytes = [0u8; 32];
		match val {
			CurrencyId::Token(token) => {
				bytes[30] = token as u8;
			}
		}
		bytes
	}
}

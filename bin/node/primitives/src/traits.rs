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

#![allow(clippy::unnecessary_cast)]

use crate::{AccountAsset, BridgeAssetBalance, Pair, Token, TokenBalance, ZenlinkAssetId};
use codec::FullCodec;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize},
	DispatchError, DispatchResult,
};
use sp_std::{fmt::Debug, vec::Vec};

/// Get tokens precision
pub trait GetDecimals {
	fn decimals(&self) -> u32;
}

/// Extension trait for CurrencyId
pub trait CurrencyIdExt {
	type PairTokens;
	type TokenSymbol;
	fn is_vtoken(&self) -> bool;
	fn is_token(&self) -> bool;
	fn is_native(&self) -> bool;
	fn is_stable_token(&self) -> bool;
	fn get_native_token(&self) -> Option<Self::TokenSymbol>;
	fn get_stable_token(&self) -> Option<Self::TokenSymbol>;
	fn get_token_pair(&self) -> Option<Self::PairTokens>;
	fn into(symbol: Self::TokenSymbol) -> Self;
}

/// A handler to manipulate assets module
pub trait AssetTrait<CurrencyId, AccountId, Balance>
where
	CurrencyId: CurrencyIdExt,
{
	type Error;
	fn asset_issue(asset_id: CurrencyId, target: &AccountId, amount: Balance);

	fn asset_destroy(asset_id: CurrencyId, target: &AccountId, amount: Balance);

	fn asset_id_exists(who: &AccountId, symbol: &[u8], precision: u16) -> Option<CurrencyId>;

	fn token_exists(asset_id: CurrencyId) -> bool;

	fn get_account_asset(asset_id: CurrencyId, target: &AccountId) -> AccountAsset<Balance>;

	fn get_token(asset_id: CurrencyId) -> Token<CurrencyId, Balance>;
}

/// Default impls
impl<CurrencyId, AccountId, Balance> AssetTrait<CurrencyId, AccountId, Balance> for ()
where
	CurrencyId: Default + CurrencyIdExt,
	AccountId: Default,
	Balance: Default,
{
	type Error = core::convert::Infallible;
	fn asset_issue(_: CurrencyId, _: &AccountId, _: Balance) {}

	fn asset_destroy(_: CurrencyId, _: &AccountId, _: Balance) {}

	fn asset_id_exists(_: &AccountId, _: &[u8], _: u16) -> Option<CurrencyId> {
		Default::default()
	}

	fn token_exists(_: CurrencyId) -> bool {
		Default::default()
	}

	fn get_account_asset(_: CurrencyId, _: &AccountId) -> AccountAsset<Balance> {
		Default::default()
	}

	fn get_token(_: CurrencyId) -> Token<CurrencyId, Balance> {
		Default::default()
	}
}

pub trait TokenPriceHandler<CurrencyId, Price> {
	fn set_token_price(asset_id: CurrencyId, price: Price);
}

/// Asset redeem handler
pub trait AssetRedeem<CurrencyId, AccountId, Balance> {
	/// Asset redeem
	fn asset_redeem(
		asset_id: CurrencyId,
		target: AccountId,
		amount: Balance,
		to_name: Option<Vec<u8>>,
	);
}

/// Bridge asset from other blockchain to Bifrost
pub trait BridgeAssetFrom<AccountId, CurrencyId, Precision, Balance> {
	fn bridge_asset_from(
		target: AccountId,
		bridge_asset: BridgeAssetBalance<AccountId, CurrencyId, Precision, Balance>,
	);
}

/// Bridge asset from Bifrost to other blockchain
pub trait BridgeAssetTo<AccountId, CurrencyId, Precision, Balance> {
	type Error;
	fn bridge_asset_to(
		target: Vec<u8>,
		bridge_asset: BridgeAssetBalance<AccountId, CurrencyId, Precision, Balance>,
	) -> Result<(), Self::Error>;
	fn redeem(
		asset_id: CurrencyId,
		amount: Balance,
		validator_address: Vec<u8>,
	) -> Result<(), Self::Error>;
	fn stake(
		asset_id: CurrencyId,
		amount: Balance,
		validator_address: Vec<u8>,
	) -> Result<(), Self::Error>;
	fn unstake(
		asset_id: CurrencyId,
		amount: Balance,
		validator_address: Vec<u8>,
	) -> Result<(), Self::Error>;
}

pub trait AssetReward<CurrencyId, Balance> {
	type Output;
	type Error;
	fn set_asset_reward(asset_id: CurrencyId, reward: Balance)
		-> Result<Self::Output, Self::Error>;
}

pub trait RewardHandler<CurrencyId, Balance> {
	fn send_reward(asset_id: CurrencyId, reward: Balance);
}

pub trait RewardTrait<Balance, AccountId, CurrencyId> {
	type Error;
	fn record_reward(
		v_token_id: CurrencyId,
		vtoken_mint_amount: Balance,
		referer: AccountId,
	) -> Result<(), Self::Error>;
	fn dispatch_reward(v_token_id: CurrencyId, staking_profit: Balance) -> Result<(), Self::Error>;
}

/// Extension traits for assets module
pub trait MultiCurrencyExt<AccountId> {
	/// The currency identifier.
	type CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug;

	/// The balance of an account.
	type Balance: AtLeast32BitUnsigned
		+ FullCodec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default;

	/// Expand the total issuance by currency id
	fn expand_total_issuance(
		currency_id: Self::CurrencyId,
		amount: Self::Balance,
	) -> DispatchResult;

	/// Burn the total issuance by currency id
	fn reduce_total_issuance(
		currency_id: Self::CurrencyId,
		amount: Self::Balance,
	) -> DispatchResult;
}

/// Trait for others module to access vtoken-mint module
pub trait VtokenMintExt {
	/// The currency identifier.
	type CurrencyId: FullCodec
		+ Eq
		+ PartialEq
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ CurrencyIdExt;

	/// The balance of an account.
	type Balance: AtLeast32BitUnsigned
		+ FullCodec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default;

	/// Get mint pool by currency id
	fn get_mint_pool(currency_id: Self::CurrencyId) -> Self::Balance;

	/// Expand mint pool
	fn expand_mint_pool(currency_id: Self::CurrencyId, amount: Self::Balance) -> DispatchResult;

	/// Reduce mint pool
	fn reduce_mint_pool(currency_id: Self::CurrencyId, amount: Self::Balance) -> DispatchResult;
}

/// Handle mint reward
pub trait MinterRewardExt<AccountId, Balance, CurrencyId, BlockNumber> {
	type Error;

	fn reward_minted_vtoken(
		minter: &AccountId,
		currency_id: CurrencyId,
		minted_vtoken: Balance,
		block_num: BlockNumber
	) -> Result<(), Self::Error>;
}

/// Zenlink traits
pub trait DEXOperations<AccountId> {
	fn get_amount_out_by_path(
		amount_in: TokenBalance,
		path: &[ZenlinkAssetId],
	) -> Result<Vec<TokenBalance>, DispatchError>;
	
	fn get_amount_in_by_path(
		amount_out: TokenBalance,
		path: &[ZenlinkAssetId],
	) -> Result<Vec<TokenBalance>, DispatchError>;

	fn inner_swap_tokens_for_exact_tokens(
		who: &AccountId,
		amount_out: TokenBalance,
		amount_in_max: TokenBalance,
		path: &[ZenlinkAssetId],
		to: &AccountId,
	) -> DispatchResult;

	fn inner_swap_exact_tokens_for_tokens(
		who: &AccountId,
		amount_in: TokenBalance,
		amount_out_min: TokenBalance,
		path: &[ZenlinkAssetId],
		to: &AccountId,
	) -> DispatchResult;

	fn inner_create_pair(token_0: &ZenlinkAssetId, token_1: &ZenlinkAssetId) -> DispatchResult;

	fn get_pair_from_asset_id(
		token_0: &ZenlinkAssetId,
		token_1: &ZenlinkAssetId,
	) -> Option<Pair<AccountId, TokenBalance>>;
}

impl<AccountId> DEXOperations<AccountId> for () {
	fn get_amount_out_by_path(
		amount_in: TokenBalance,
		path: &[ZenlinkAssetId],
	) -> Result<Vec<TokenBalance>, DispatchError> {
		Ok(sp_std::vec![])
	}
	
	fn get_amount_in_by_path(
		amount_out: TokenBalance,
		path: &[ZenlinkAssetId],
	) -> Result<Vec<TokenBalance>, DispatchError> {
		Ok(sp_std::vec![])
	}

	fn inner_swap_tokens_for_exact_tokens(
		who: &AccountId,
		amount_out: TokenBalance,
		amount_in_max: TokenBalance,
		path: &[ZenlinkAssetId],
		to: &AccountId,
	) -> DispatchResult {
		Ok(())
	}

	fn inner_swap_exact_tokens_for_tokens(
		who: &AccountId,
		amount_in: TokenBalance,
		amount_out_min: TokenBalance,
		path: &[ZenlinkAssetId],
		to: &AccountId,
	) -> DispatchResult {
		Ok(())
	}

	fn inner_create_pair(token_0: &ZenlinkAssetId, token_1: &ZenlinkAssetId) -> DispatchResult {
		Ok(())
	}

	fn get_pair_from_asset_id(
		token_0: &ZenlinkAssetId,
		token_1: &ZenlinkAssetId,
	) -> Option<Pair<AccountId, TokenBalance>> {
		None
	}
}

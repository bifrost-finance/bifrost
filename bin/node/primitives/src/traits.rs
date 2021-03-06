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

use crate::{AccountAsset, Token, VtokenPool, BridgeAssetBalance};
use sp_std::vec::Vec;

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
pub trait AssetTrait<CurrencyId, AccountId, Balance> where CurrencyId: CurrencyIdExt {
	type Error;
	// fn asset_create(symbol: Vec<u8>, precision: u16, token_type: TokenType) -> Result<(CurrencyId, Token<CurrencyId, Balance>), Self::Error>;

	// fn asset_create_pair(symbol: Vec<u8>, precision: u16) -> Result<(CurrencyId, CurrencyId), Self::Error>;

	fn asset_issue(asset_id: CurrencyId, target: &AccountId, amount: Balance);

	// fn asset_redeem(asset_id: CurrencyId, target: &AccountId, amount: Balance);

	fn asset_destroy(asset_id: CurrencyId, target: &AccountId, amount: Balance);

	fn asset_id_exists(who: &AccountId, symbol: &[u8], precision: u16) -> Option<CurrencyId>;

	fn token_exists(asset_id: CurrencyId) -> bool;

	fn get_account_asset(asset_id: CurrencyId, target: &AccountId) -> AccountAsset<Balance>;

	fn get_token(asset_id: CurrencyId) -> Token<CurrencyId, Balance>;

	// fn lock_asset(who: &AccountId, asset_id: CurrencyId, locked: Balance);

	// fn unlock_asset(who: &AccountId, asset_id: CurrencyId, unlocked: Balance);

	fn is_token(asset_id: CurrencyId) -> bool;

	fn is_vtoken(asset_id: CurrencyId) -> bool;

	// fn get_pair(asset_id: CurrencyId) -> Option<CurrencyId>;
}

/// Default impls
impl<CurrencyId, AccountId, Balance> AssetTrait<CurrencyId, AccountId, Balance> for ()
	where CurrencyId: Default + CurrencyIdExt, AccountId: Default, Balance: Default
{
	type Error = core::convert::Infallible;
	// fn asset_create(_: Vec<u8>, _: u16, _: TokenType) -> Result<(CurrencyId, Token<CurrencyId, Balance>), Self::Error> { Ok(Default::default()) }

	// fn asset_create_pair(_: Vec<u8>, _: u16) -> Result<(CurrencyId, CurrencyId), Self::Error> { Ok(Default::default()) }

	fn asset_issue(_: CurrencyId, _: &AccountId, _: Balance) {}

	// fn asset_redeem(_: CurrencyId, _: &AccountId, _: Balance) {}

	fn asset_destroy(_: CurrencyId, _: &AccountId, _: Balance) {}

	fn asset_id_exists(_: &AccountId, _: &[u8], _: u16) -> Option<CurrencyId> { Default::default() }

	fn token_exists(_: CurrencyId) -> bool { Default::default() }

	fn get_account_asset(_: CurrencyId, _: &AccountId) -> AccountAsset<Balance> { Default::default() }

	fn get_token(_: CurrencyId) -> Token<CurrencyId, Balance> { Default::default() }

	// fn lock_asset( _: &AccountId, _: CurrencyId, _: Balance) {}

	// fn unlock_asset( _: &AccountId, _: CurrencyId, _: Balance) {}

	fn is_token(_: CurrencyId) -> bool { Default::default() }

	fn is_vtoken(_: CurrencyId) -> bool { Default::default() }

	// fn get_pair(_: CurrencyId) -> Option<CurrencyId> { Default::default() }
}

pub trait TokenPriceHandler<CurrencyId, Price> {
	fn set_token_price(asset_id: CurrencyId, price: Price);
}

/// Asset redeem handler
pub trait AssetRedeem<CurrencyId, AccountId, Balance> {
	/// Asset redeem
	fn asset_redeem(asset_id: CurrencyId, target: AccountId, amount: Balance, to_name: Option<Vec<u8>>);
}

/// Fetch vtoken mint rate handler
pub trait FetchVtokenMintPrice<CurrencyId, VtokenMintPrice> {
	/// fetch vtoken mint rate
	fn fetch_vtoken_price(asset_id: CurrencyId) -> VtokenMintPrice;
}

/// Bridge asset from other blockchain to Bifrost
pub trait BridgeAssetFrom<AccountId, CurrencyId, Precision, Balance> {
	fn bridge_asset_from(target: AccountId, bridge_asset: BridgeAssetBalance<AccountId, CurrencyId, Precision, Balance>);
}

/// Bridge asset from Bifrost to other blockchain
pub trait BridgeAssetTo<AccountId, CurrencyId, Precision, Balance> {
	type Error;
	fn bridge_asset_to(target: Vec<u8>, bridge_asset: BridgeAssetBalance<AccountId, CurrencyId, Precision, Balance>, ) -> Result<(), Self::Error>;
	fn redeem(asset_id: CurrencyId, amount: Balance, validator_address: Vec<u8>) -> Result<(), Self::Error>;
	fn stake(asset_id: CurrencyId, amount: Balance, validator_address: Vec<u8>) -> Result<(), Self::Error>;
	fn unstake(asset_id: CurrencyId, amount: Balance, validator_address: Vec<u8>) -> Result<(), Self::Error>;
}

pub trait AssetReward<CurrencyId, Balance> {
	type Output;
	type Error;
	fn set_asset_reward(asset_id: CurrencyId, reward: Balance) -> Result<Self::Output, Self::Error>;
}

pub trait RewardHandler<CurrencyId, Balance> {
	fn send_reward(asset_id: CurrencyId, reward: Balance);
}

pub trait RewardTrait<Balance, AccountId, CurrencyId> {
	type Error;
	fn record_reward(v_token_id: CurrencyId, vtoken_mint_amount: Balance, referer: AccountId) -> Result<(), Self::Error>;
	fn dispatch_reward(v_token_id: CurrencyId, staking_profit: Balance) -> Result<(), Self::Error>;
}

/// Fetch vtoken mint rate handler
pub trait FetchVtokenMintPool<CurrencyId, Balance> {
	/// fetch vtoken mint pool for calculate vtoken mint price
	fn fetch_vtoken_pool(asset_id: CurrencyId) -> VtokenPool<Balance>;
}
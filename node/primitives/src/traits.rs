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

#![allow(clippy::unnecessary_cast)]

use codec::{Decode, Encode, FullCodec};
use frame_support::{
	dispatch::DispatchError,
	pallet_prelude::DispatchResultWithPostInfo,
	sp_runtime::{traits::AccountIdConversion, TokenError, TypeId},
};
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize},
	DispatchResult,
};
use sp_std::{fmt::Debug, vec::Vec};

use crate::{AssetIds, CurrencyId, PoolId, TokenId};

pub trait TokenInfo {
	fn currency_id(&self) -> u64;
	fn name(&self) -> Option<&str>;
	fn symbol(&self) -> Option<&str>;
	fn decimals(&self) -> Option<u8>;
}

/// Extension trait for CurrencyId
pub trait CurrencyIdExt {
	type TokenSymbol;
	fn is_vtoken(&self) -> bool;
	fn is_token(&self) -> bool;
	fn is_vstoken(&self) -> bool;
	fn is_vsbond(&self) -> bool;
	fn is_native(&self) -> bool;
	fn is_stable(&self) -> bool;
	fn is_lptoken(&self) -> bool;
	fn is_foreign_asset(&self) -> bool;
	fn into(symbol: Self::TokenSymbol) -> Self;
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

pub trait BancorHandler<Balance> {
	fn add_token(currency_id: super::CurrencyId, amount: Balance) -> DispatchResult;
}

impl<Balance> BancorHandler<Balance> for () {
	fn add_token(_currency_id: super::CurrencyId, _amount: Balance) -> DispatchResult {
		DispatchResult::from(DispatchError::Token(TokenError::NoFunds))
	}
}

pub trait CheckSubAccount<T: Encode + Decode> {
	fn check_sub_account<S: Decode>(&self, account: &T) -> bool;
}

impl<T, Id> CheckSubAccount<T> for Id
where
	T: Encode + Decode,
	Id: Encode + Decode + TypeId + AccountIdConversion<T> + Eq,
{
	fn check_sub_account<S: Decode>(&self, account: &T) -> bool {
		match Id::try_from_sub_account::<S>(account) {
			Some((id, _)) => id.eq(self),
			None => false,
		}
	}
}

/// The interface to call VtokenMinting module functions.
pub trait VtokenMintingOperator<CurrencyId, Balance, AccountId, TimeUnit> {
	/// Get the currency tokenpool amount.
	fn get_token_pool(currency_id: CurrencyId) -> Balance;

	/// Increase the token amount for the storage "token_pool" in the VtokenMining module.
	fn increase_token_pool(currency_id: CurrencyId, token_amount: Balance) -> DispatchResult;

	/// Decrease the token amount for the storage "token_pool" in the VtokenMining module.
	fn decrease_token_pool(currency_id: CurrencyId, token_amount: Balance) -> DispatchResult;

	/// Update the ongoing era for a CurrencyId.
	fn update_ongoing_time_unit(currency_id: CurrencyId, time_unit: TimeUnit) -> DispatchResult;

	/// Get the current era of a CurrencyId.
	fn get_ongoing_time_unit(currency_id: CurrencyId) -> Option<TimeUnit>;

	/// Get the the unlocking records of a certain time unit.
	fn get_unlock_records(
		currency_id: CurrencyId,
		time_unit: TimeUnit,
	) -> Option<(Balance, Vec<u32>)>;

	/// Revise the currency indexed unlocking record by some amount.
	fn deduct_unlock_amount(
		currency_id: CurrencyId,
		index: u32,
		deduct_amount: Balance,
	) -> DispatchResult;

	/// Get currency Entrance and Exit accounts.【entrance_account, exit_account】
	fn get_entrance_and_exit_accounts() -> (AccountId, AccountId);

	/// Get the token_unlock_ledger storage info to refund to the due era unlocking users.
	fn get_token_unlock_ledger(
		currency_id: CurrencyId,
		index: u32,
	) -> Option<(AccountId, Balance, TimeUnit)>;
}

/// Trait for Vtoken-Minting module to check whether accept redeeming or not.
pub trait SlpOperator<CurrencyId> {
	fn all_delegation_requests_occupied(currency_id: CurrencyId) -> bool;
}

/// A mapping between AssetId and AssetMetadata.
pub trait AssetIdMapping<CurrencyId, MultiLocation, AssetMetadata> {
	/// Returns the AssetMetadata associated with a given `AssetIds`.
	fn get_asset_metadata(asset_ids: AssetIds) -> Option<AssetMetadata>;
	/// Returns the AssetMetadata associated with a given `CurrencyId`.
	fn get_currency_metadata(currency_id: CurrencyId) -> Option<AssetMetadata>;
	/// Returns the MultiLocation associated with a given CurrencyId.
	fn get_multi_location(currency_id: CurrencyId) -> Option<MultiLocation>;
	/// Returns the CurrencyId associated with a given MultiLocation.
	fn get_currency_id(multi_location: MultiLocation) -> Option<CurrencyId>;
}

/// The interface to call farming pallet functions.
pub trait FarmingInfo<Balance, CurrencyId> {
	/// Get the currency token shares.
	fn get_token_shares(pool_id: PoolId, currency_id: CurrencyId) -> Balance;
}

pub trait VtokenMintingInterface<AccountId, CurrencyId, Balance> {
	fn mint(
		exchanger: AccountId,
		token_id: CurrencyId,
		token_amount: Balance,
	) -> DispatchResultWithPostInfo;
	fn redeem(
		exchanger: AccountId,
		vtoken_id: CurrencyId,
		vtoken_amount: Balance,
	) -> DispatchResultWithPostInfo;
	fn token_to_vtoken(
		token_id: CurrencyId,
		vtoken_id: CurrencyId,
		token_amount: Balance,
	) -> Balance;
	fn vtoken_to_token(
		token_id: CurrencyId,
		vtoken_id: CurrencyId,
		vtoken_amount: Balance,
	) -> Balance;
	fn vtoken_id(token_id: CurrencyId) -> Option<CurrencyId>;
	fn token_id(vtoken_id: CurrencyId) -> Option<CurrencyId>;
}

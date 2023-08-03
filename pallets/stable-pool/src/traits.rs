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

//! traits for stable-pool
use crate::*;
use sp_runtime::AccountId32;

pub trait StablePoolHandler {
	type Balance;
	type AccountId;

	fn add_liquidity(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		amounts: Vec<Self::Balance>,
		min_mint_amount: Self::Balance,
	) -> DispatchResult;

	fn swap(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		currency_id_in: PoolTokenIndex,
		currency_id_out: PoolTokenIndex,
		amount: Self::Balance,
		min_dy: Self::Balance,
	) -> DispatchResult;

	fn redeem_single(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		amount: Self::Balance,
		i: PoolTokenIndex,
		min_redeem_amount: Self::Balance,
		asset_length: u32,
	) -> Result<(Self::Balance, Self::Balance), DispatchError>;

	fn redeem_multi(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		amounts: Vec<Self::Balance>,
		max_redeem_amount: Self::Balance,
	) -> DispatchResult;

	fn redeem_proportion(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		amount: Self::Balance,
		min_redeem_amounts: Vec<Self::Balance>,
	) -> DispatchResult;
}

impl<T: Config> StablePoolHandler for Pallet<T> {
	type Balance = T::Balance;
	type AccountId = T::AccountId;

	fn add_liquidity(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		amounts: Vec<Self::Balance>,
		min_mint_amount: Self::Balance,
	) -> DispatchResult {
		Self::mint_inner(&who, pool_id, amounts, min_mint_amount)
	}

	fn swap(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		currency_id_in: PoolTokenIndex,
		currency_id_out: PoolTokenIndex,
		amount: Self::Balance,
		min_dy: Self::Balance,
	) -> DispatchResult {
		Self::on_swap(&who, pool_id, currency_id_in, currency_id_out, amount, min_dy)
	}

	fn redeem_single(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		amount: Self::Balance,
		i: PoolTokenIndex,
		min_redeem_amount: Self::Balance,
		asset_length: u32,
	) -> Result<(Self::Balance, Self::Balance), DispatchError> {
		Self::redeem_single_inner(&who, pool_id, amount, i, min_redeem_amount, asset_length)
	}

	fn redeem_multi(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		amounts: Vec<Self::Balance>,
		max_redeem_amount: Self::Balance,
	) -> DispatchResult {
		Self::redeem_multi_inner(&who, pool_id, amounts, max_redeem_amount)
	}

	fn redeem_proportion(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		amount: Self::Balance,
		min_redeem_amounts: Vec<Self::Balance>,
	) -> DispatchResult {
		Self::redeem_proportion_inner(&who, pool_id, amount, min_redeem_amounts)
	}
}

impl StablePoolHandler for () {
	type Balance = u128;
	type AccountId = AccountId32;

	fn add_liquidity(
		_who: Self::AccountId,
		_pool_id: StableAssetPoolId,
		_amounts: Vec<Self::Balance>,
		_min_mint_amount: Self::Balance,
	) -> DispatchResult {
		Ok(())
	}

	fn swap(
		_who: Self::AccountId,
		_pool_id: StableAssetPoolId,
		_currency_id_in: PoolTokenIndex,
		_currency_id_out: PoolTokenIndex,
		_amount: Self::Balance,
		_min_dy: Self::Balance,
	) -> DispatchResult {
		Ok(())
	}

	fn redeem_single(
		_who: Self::AccountId,
		_pool_id: StableAssetPoolId,
		_amount: Self::Balance,
		_i: PoolTokenIndex,
		_min_redeem_amount: Self::Balance,
		_asset_length: u32,
	) -> Result<(Self::Balance, Self::Balance), DispatchError> {
		Ok((0, 0))
	}

	fn redeem_multi(
		_who: Self::AccountId,
		_pool_id: StableAssetPoolId,
		_amounts: Vec<Self::Balance>,
		_max_redeem_amount: Self::Balance,
	) -> DispatchResult {
		Ok(())
	}

	fn redeem_proportion(
		_who: Self::AccountId,
		_pool_id: StableAssetPoolId,
		_amount: Self::Balance,
		_min_redeem_amounts: Vec<Self::Balance>,
	) -> DispatchResult {
		Ok(())
	}
}

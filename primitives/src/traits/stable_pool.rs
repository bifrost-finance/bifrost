// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

use crate::*;

pub trait StablePoolHandler {
	type Balance;
	type AccountId;
	type CurrencyId;

	fn add_liquidity(
		who: Self::AccountId,
		pool_id: StableAssetPoolId,
		amounts: Vec<Self::Balance>,
		min_mint_amount: Self::Balance,
	) -> DispatchResult;

	fn swap(
		who: &Self::AccountId,
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

	fn get_pool_token_index(
		pool_id: StableAssetPoolId,
		currency_id: CurrencyId,
	) -> Option<PoolTokenIndex>;

	fn get_swap_output(
		pool_id: StableAssetPoolId,
		currency_id_in: PoolTokenIndex,
		currency_id_out: PoolTokenIndex,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError>;

	fn get_swap_input(
		pool_id: StableAssetPoolId,
		currency_id_in: PoolTokenIndex,
		currency_id_out: PoolTokenIndex,
		amount: Self::Balance,
	) -> Result<Self::Balance, DispatchError>;

	fn get_pool_id(
		currency_id_in: &Self::CurrencyId,
		currency_id_out: &Self::CurrencyId,
	) -> Option<(StableAssetPoolId, PoolTokenIndex, PoolTokenIndex)>;
}


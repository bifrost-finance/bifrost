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
use crate::*;
pub trait StablePool {
	type AssetId;
	type AtLeast64BitUnsigned;
	type Balance;
	type AccountId;
	type BlockNumber;

	fn set_token_rate(
		pool_id: StableAssetPoolId,
		token_rate_info: Vec<(
			Self::AssetId,
			(Self::AtLeast64BitUnsigned, Self::AtLeast64BitUnsigned),
		)>,
	) -> DispatchResult;

	fn get_token_rate(
		pool_id: StableAssetPoolId,
		asset_id: Self::AssetId,
	) -> Option<(Self::AtLeast64BitUnsigned, Self::AtLeast64BitUnsigned)>;
}

impl<T: Config> StablePool for Pallet<T> {
	type AssetId = AssetIdOf<T>;
	type AtLeast64BitUnsigned = AtLeast64BitUnsignedOf<T>;
	type Balance = T::Balance;
	type AccountId = T::AccountId;
	type BlockNumber = T::BlockNumber;

	fn set_token_rate(
		pool_id: StableAssetPoolId,
		token_rate_info: Vec<(
			Self::AssetId,
			(Self::AtLeast64BitUnsigned, Self::AtLeast64BitUnsigned),
		)>,
	) -> DispatchResult {
		if token_rate_info.last().is_none() {
			let res = TokenRateCaches::<T>::clear_prefix(pool_id, u32::max_value(), None);
			ensure!(res.maybe_cursor.is_none(), Error::<T>::TokenRateNotCleared);
		} else {
			let mut token_rate_info = token_rate_info.into_iter();
			let mut token_rate = token_rate_info.next();
			let mut cursor = TokenRateCaches::<T>::iter_prefix(pool_id);
			while let Some((asset_id, is_token_rate)) = cursor.next() {
				if let Some((new_asset_id, new_is_token_rate)) = token_rate {
					if asset_id == new_asset_id {
						if is_token_rate != new_is_token_rate {
							TokenRateCaches::<T>::insert(pool_id, asset_id, new_is_token_rate);
						}
						token_rate = token_rate_info.next();
					} else {
						TokenRateCaches::<T>::remove(pool_id, asset_id);
					}
				} else {
					TokenRateCaches::<T>::remove(pool_id, asset_id);
				}
			}
			while let Some((asset_id, is_token_rate)) = token_rate {
				TokenRateCaches::<T>::insert(pool_id, asset_id, is_token_rate);
				token_rate = token_rate_info.next();
			}
		}
		Ok(())
	}

	fn get_token_rate(
		pool_id: StableAssetPoolId,
		asset_id: Self::AssetId,
	) -> Option<(Self::AtLeast64BitUnsigned, Self::AtLeast64BitUnsigned)> {
		TokenRateCaches::<T>::get(pool_id, asset_id)
	}
}

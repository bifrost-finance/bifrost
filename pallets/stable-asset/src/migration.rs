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

#![cfg_attr(not(feature = "std"), no_std)]

use super::{Config, Weight, *};
use frame_support::traits::Get;

pub fn update_pallet_id<T: Config>() -> Weight {
	let pool_count: u32 = PoolCount::<T>::get();

	for pool_id in 0..pool_count {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let to: T::AccountId = T::PalletId::get().into_sub_account_truncating(pool_id);
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;

			for (_, &asset_id) in pool_info.assets.iter().enumerate() {
				let balance = T::Assets::free_balance(asset_id, &pool_info.account_id);
				if balance == Zero::zero() {
					continue;
				}
				T::Assets::transfer(asset_id, &pool_info.account_id, &to, balance)?;
			}
			let pool_asset_balance =
				T::Assets::free_balance(pool_info.pool_asset, &pool_info.account_id);
			T::Assets::transfer(
				pool_info.pool_asset,
				&pool_info.account_id,
				&to,
				pool_asset_balance,
			)?;

			pool_info.account_id = to;
			Ok(())
		})
		.ok();
	}

	let count: u64 = (pool_count * 3).into();
	Weight::from(T::DbWeight::get().reads_writes(count, count))
}

use frame_support::{pallet_prelude::PhantomData, traits::OnRuntimeUpgrade};
pub struct StableAssetOnRuntimeUpgrade<T>(PhantomData<T>);
impl<T: super::Config> OnRuntimeUpgrade for StableAssetOnRuntimeUpgrade<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<sp_std::prelude::Vec<u8>, sp_runtime::DispatchError> {
		#[allow(unused_imports)]
		use frame_support::PalletId;
		log::info!("Bifrost `pre_upgrade`...");

		Ok(vec![])
	}

	fn on_runtime_upgrade() -> Weight {
		log::info!("Bifrost `on_runtime_upgrade`...");

		let weight = super::migration::update_pallet_id::<T>();

		log::info!("Bifrost `on_runtime_upgrade finished`");

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_: sp_std::prelude::Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
		#[allow(unused_imports)]
		use frame_support::PalletId;
		log::info!("Bifrost `post_upgrade`...");
		let old_pallet_id: PalletId = PalletId(*b"nuts/sta");

		let pool_count: u32 = PoolCount::<T>::get();
		for pool_id in 0..pool_count {
			if let Some(pool_info) = Pools::<T>::get(pool_id) {
				let old_account_id: T::AccountId =
					old_pallet_id.into_sub_account_truncating(pool_id);
				for (_, &asset_id) in pool_info.assets.iter().enumerate() {
					let old_balance = T::Assets::free_balance(asset_id, &old_account_id);
					assert_eq!(old_balance, Zero::zero());

					let balance = T::Assets::free_balance(asset_id, &pool_info.account_id);
					if balance != Zero::zero() {
						continue;
					} else {
						log::info!(
							"New pool {:?} asset_id {:?} free_balance is zero.",
							pool_id,
							asset_id
						);
					}
				}
			} else {
				log::info!("Pool {:?} not found", pool_id);
			}
		}

		Ok(())
	}
}

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
use frame_support::{
	ensure,
	pallet_prelude::StorageVersion,
	traits::{GetStorageVersion, OnRuntimeUpgrade},
};
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

const LOG_TARGET: &str = "system-staking::migration";

pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// Check the storage version
		let onchain_version = Pallet::<T>::on_chain_storage_version();
		if onchain_version < 1 {
			// Transform storage values
			// We transform the storage values from the old into the new format.
			log::info!(target: LOG_TARGET, "Start to migrate TokenStatus storage...");
			TokenStatus::<T>::translate::<TokenInfo<BalanceOf<T>, BlockNumberFor<T>>, _>(
				|k: CurrencyId, old_token_info: TokenInfo<BalanceOf<T>, BlockNumberFor<T>>| {
					log::info!(target: LOG_TARGET, "Migrated to boundedvec for {:?}...", k);

					let mut new_token_info =
						<TokenInfo<BalanceOf<T>, BlockNumberFor<T>>>::default();

					new_token_info.farming_staking_amount = old_token_info.farming_staking_amount;
					new_token_info.system_stakable_amount = old_token_info.system_stakable_amount;
					new_token_info.system_shadow_amount = old_token_info.system_shadow_amount;
					new_token_info.pending_redeem_amount = old_token_info.pending_redeem_amount;

					new_token_info.current_config.exec_delay =
						BlockNumberFor::<T>::from(old_token_info.current_config.exec_delay);
					new_token_info.current_config.system_stakable_farming_rate =
						old_token_info.current_config.system_stakable_farming_rate;
					new_token_info.current_config.lptoken_rates =
						BoundedVec::try_from(old_token_info.current_config.lptoken_rates)
							.map_err(|e| { log::error!("Failed to convert old current_config.lptoken_rates into BoundedVec during migration for {:?}: {:?}", k, e) })
							.unwrap();
					new_token_info.current_config.add_or_sub =
						old_token_info.current_config.add_or_sub;
					new_token_info.current_config.system_stakable_base =
						old_token_info.current_config.system_stakable_base;
					new_token_info.current_config.farming_poolids =
						BoundedVec::try_from(old_token_info.current_config.farming_poolids)
							.map_err(|e| { log::error!("Failed to convert old current_config.farming_poolids into BoundedVec during migration for {:?}: {:?}", k, e) })
							.unwrap();

					new_token_info.new_config.exec_delay =
						BlockNumberFor::<T>::from(old_token_info.new_config.exec_delay);
					new_token_info.new_config.system_stakable_farming_rate =
						old_token_info.new_config.system_stakable_farming_rate;
					new_token_info.new_config.lptoken_rates =
						BoundedVec::try_from(old_token_info.new_config.lptoken_rates)
							.map_err(|e| { log::error!("Failed to convert old new_config.lptoken_rates into BoundedVec during migration for {:?}: {:?}", k, e) })
							.unwrap();
					new_token_info.new_config.add_or_sub = old_token_info.new_config.add_or_sub;
					new_token_info.new_config.system_stakable_base =
						old_token_info.new_config.system_stakable_base;
					new_token_info.new_config.farming_poolids =
						BoundedVec::try_from(old_token_info.new_config.farming_poolids)
							.map_err(|e| { log::error!("Failed to convert old new_config.farming_poolids into BoundedVec during migration for {:?}: {:?}", k, e) })
							.unwrap();

					Some(new_token_info)
				},
			);

			// Update the storage version
			StorageVersion::new(1).put::<Pallet<T>>();

			// Return the consumed weight
			let count = TokenStatus::<T>::iter().count();
			Weight::from(T::DbWeight::get().reads_writes(count as u64 + 1, count as u64 + 1))
		} else {
			// We don't do anything here.
			Weight::zero()
		}
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let cnt = TokenStatus::<T>::iter().count();
		// print out the pre-migrate storage count
		log::info!(target: LOG_TARGET, "TokenStatus pre-migrate storage count: {:?}", cnt);
		Ok((cnt as u64).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(cnt: Vec<u8>) -> Result<(), TryRuntimeError> {
		let new_count = TokenStatus::<T>::iter().count();

		let old_count: u64 = Decode::decode(&mut cnt.as_slice())
			.expect("the state parameter should be something that was generated by pre_upgrade");

		// print out the post-migrate storage count
		log::info!(
			target: LOG_TARGET,
			"TokenStatus post-migrate storage count: {:?}",
			new_count
		);

		ensure!(
			new_count as u64 == old_count,
			"Post-migration storage count does not match pre-migration count"
		);

		Ok(())
	}
}

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
use frame_support::{storage_alias, traits::OnRuntimeUpgrade};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

const LOG_TARGET: &str = "flexible-fee::migration";

#[storage_alias]
pub type UserFeeChargeOrderList<T: Config> = StorageMap<
	Pallet<T>,
	Twox64Concat,
	<T as frame_system::Config>::AccountId,
	Vec<CurrencyId>,
	OptionQuery,
>;

pub struct FlexibleFeeMigration<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for FlexibleFeeMigration<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// Check the storage version
		let onchain_version = Pallet::<T>::on_chain_storage_version();
		if onchain_version < 2 {
			log::info!(target: LOG_TARGET, "Start to migrate flexible-fee storage...");
			// Remove the UserFeeChargeOrderList storage content
			let count = UserFeeChargeOrderList::<T>::clear(u32::MAX, None).unique as u64;

			// Update the storage version
			StorageVersion::new(2).put::<Pallet<T>>();

			// Return the consumed weight
			Weight::from(T::DbWeight::get().reads_writes(count as u64 + 1, count as u64 + 1))
		} else {
			// We don't do anything here.
			Weight::zero()
		}
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let cnt = UserFeeChargeOrderList::<T>::iter().count();

		// print out the pre-migrate storage count
		log::info!(
			target: LOG_TARGET,
			"UserFeeChargeOrderList pre-migrate storage count: {:?}",
			cnt
		);
		Ok((cnt as u64).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_cnt: Vec<u8>) -> Result<(), TryRuntimeError> {
		let new_count = UserFeeChargeOrderList::<T>::iter().count();

		// print out the post-migrate storage count
		log::info!(
			target: LOG_TARGET,
			"UserFeeChargeOrderList post-migrate storage count: {:?}",
			new_count
		);

		ensure!(
			new_count as u64 == 0,
			"Post-migration storage count does not match pre-migration count"
		);
		Ok(())
	}
}

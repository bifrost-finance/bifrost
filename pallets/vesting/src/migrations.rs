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

//! Storage migrations for the vesting pallet.

use super::*;

// Migration from single schedule to multiple schedules.
pub(crate) mod v1 {
	use super::*;

	#[allow(dead_code)]
	#[cfg(feature = "try-runtime")]
	pub(crate) fn pre_migrate<T: Config>() -> Result<(), sp_runtime::DispatchError> {
		assert!(
			super::pallet::StorageVersion::<T>::get() == Releases::V0,
			"Storage version too high."
		);

		log::debug!(
			target: "runtime::vesting",
			"migration: Vesting storage version v1 PRE migration checks succesful!"
		);

		Ok(())
	}

	/// Migrate from single schedule to multi schedule storage.
	/// WARNING: This migration will delete schedules if `MaxVestingSchedules < 1`.
	#[allow(dead_code)]
	pub(crate) fn migrate<T: Config>() -> Weight {
		let mut reads_writes = 0;

		Vesting::<T>::translate::<VestingInfo<BalanceOf<T>, BlockNumberFor<T>>, _>(
			|_key, vesting_info| {
				reads_writes += 1;
				let v: Option<
					BoundedVec<
						VestingInfo<BalanceOf<T>, BlockNumberFor<T>>,
						MaxVestingSchedulesGet<T>,
					>,
				> = vec![vesting_info].try_into().ok();

				if v.is_none() {
					log::warn!(
						target: "runtime::vesting",
						"migration: Failed to move a vesting schedule into a BoundedVec"
					);
				}

				v
			},
		);

		T::DbWeight::get().reads_writes(reads_writes, reads_writes)
	}

	#[allow(dead_code)]
	#[cfg(feature = "try-runtime")]
	pub(crate) fn post_migrate<T: Config>() -> Result<(), sp_runtime::DispatchError> {
		assert_eq!(super::pallet::StorageVersion::<T>::get(), Releases::V1);

		for (_key, schedules) in Vesting::<T>::iter() {
			assert!(
				schedules.len() == 1,
				"A bounded vec with incorrect count of items was created."
			);

			for s in schedules {
				// It is ok if this does not pass, but ideally pre-existing schedules would pass
				// this validation logic so we can be more confident about edge cases.
				if !s.is_valid() {
					log::warn!(
						target: "runtime::vesting",
						"migration: A schedule does not pass new validation logic.",
					)
				}
			}
		}

		log::debug!(
			target: "runtime::vesting",
			"migration: Vesting storage version v1 POST migration checks successful!"
		);
		Ok(())
	}
}

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

use super::*;
use frame_support::{storage_alias, traits::OnRuntimeUpgrade, weights::Weight};

mod v0 {
	use super::*;

	#[storage_alias]
	pub(super) type DelegatorVote<T: Config> = StorageDoubleMap<
		Pallet<T>,
		Twox64Concat,
		CurrencyIdOf<T>,
		Twox64Concat,
		DerivativeIndex,
		AccountVote<BalanceOf<T>>,
	>;
}

pub mod v1 {
	use super::*;
	use frame_support::traits::StorageVersion;

	pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			if StorageVersion::get::<Pallet<T>>() == 0 {
				let weight_consumed = migrate_to_v1::<T>();
				log::info!("Migrating vtoken-voting storage to v1");
				StorageVersion::new(1).put::<Pallet<T>>();
				weight_consumed
			} else {
				log::warn!("vtoken-voting migration should be removed.");
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
			log::info!(
				"vtoken-voting before migration: version: {:?}",
				StorageVersion::get::<Pallet<T>>(),
			);
			log::info!(
				"vtoken-voting before migration: v0 count: {}",
				v0::DelegatorVote::<T>::iter().count(),
			);
			ensure!(
				v0::DelegatorVote::<T>::iter().count() > 0,
				"v0::DelegatorVote should not be empty before the migration"
			);

			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_: Vec<u8>) -> Result<(), &'static str> {
			log::info!(
				"vtoken-voting after migration: version: {:?}",
				StorageVersion::get::<Pallet<T>>(),
			);
			log::info!(
				"vtoken-voting after migration: v1 count: {}",
				DelegatorVote::<T>::iter().count()
			);
			ensure!(
				DelegatorVote::<T>::iter().count() > 0,
				"DelegatorVote should not be empty after the migration"
			);

			Ok(())
		}
	}
}

pub fn migrate_to_v1<T: Config>() -> Weight {
	let mut weight: Weight = Weight::zero();

	let old_keys = v0::DelegatorVote::<T>::iter_keys();
	let vtoken = VKSM;

	for (_, voting) in VotingFor::<T>::iter() {
		if let Voting::Casting(Casting { votes, .. }) = voting {
			for (poll_index, vote, derivative_index, _) in votes.iter() {
				if DelegatorVote::<T>::contains_key((vtoken, poll_index, derivative_index)) {
					let _ = Pallet::<T>::try_add_delegator_vote(
						vtoken,
						*poll_index,
						*derivative_index,
						*vote,
					);
					weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 2));
				} else {
					DelegatorVote::<T>::insert((vtoken, poll_index, derivative_index), vote);
					weight = weight.saturating_add(T::DbWeight::get().writes(1));
				}
			}
		}
	}

	for (vtoken, derivative_index) in old_keys {
		v0::DelegatorVote::<T>::remove(vtoken, derivative_index);
	}

	weight
}

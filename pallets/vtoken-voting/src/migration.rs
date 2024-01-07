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

use super::*;
use frame_support::{storage_alias, weights::Weight};

pub mod v1 {
	use super::*;

	#[storage_alias]
	pub(super) type DelegatorVoteRole<T: Config> = StorageDoubleMap<
		Pallet<T>,
		Twox64Concat,
		CurrencyIdOf<T>,
		Twox64Concat,
		DerivativeIndex,
		VoteRole,
	>;

	#[storage_alias]
	pub(super) type DelegatorVote<T: Config> = StorageNMap<
		Pallet<T>,
		(
			NMapKey<Twox64Concat, CurrencyIdOf<T>>,
			NMapKey<Twox64Concat, PollIndex>,
			NMapKey<Twox64Concat, DerivativeIndex>,
		),
		AccountVote<BalanceOf<T>>,
	>;
}

pub mod v2 {
	use super::*;
	use crate::{Config, CurrencyIdOf, Pallet};
	use cumulus_primitives_core::Weight;
	use frame_support::{pallet_prelude::StorageVersion, traits::OnRuntimeUpgrade};
	use sp_runtime::traits::Get;

	pub struct MigrateToV2<T, C>(
		sp_std::marker::PhantomData<T>,
		sp_std::marker::PhantomData<C>,
	);
	impl<T: Config, C: Get<CurrencyIdOf<T>>> OnRuntimeUpgrade for MigrateToV2<T, C> {
		fn on_runtime_upgrade() -> Weight {
			if StorageVersion::get::<Pallet<T>>() < 2 {
				let weight_consumed = migrate_to_v2::<T, C>();
				log::info!("Migrating vtoken-voting storage to v2");
				StorageVersion::new(2).put::<Pallet<T>>();
				weight_consumed
			} else {
				log::warn!("vtoken-voting migration should be removed.");
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
			log::info!(
				"vtoken-voting before migration: version: {:?}",
				StorageVersion::get::<Pallet<T>>(),
			);
			log::info!(
				"vtoken-voting before migration: count: {}",
				v1::DelegatorVote::<T>::iter().count(),
			);
			ensure!(
				v1::DelegatorVote::<T>::iter().count() > 0,
				"DelegatorVote should not be empty before the migration"
			);

			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
			log::info!(
				"vtoken-voting after migration: version: {:?}",
				StorageVersion::get::<Pallet<T>>(),
			);
			log::info!(
				"vtoken-voting after migration: count: {}",
				v1::DelegatorVote::<T>::iter().count()
			);
			ensure!(
				v1::DelegatorVote::<T>::iter().count() == 0,
				"DelegatorVote should be empty after the migration"
			);

			Ok(())
		}
	}
}

pub fn migrate_to_v2<T: Config, C: Get<CurrencyIdOf<T>>>() -> Weight {
	let mut weight: Weight = Weight::zero();

	let token = C::get();
	let vtoken = token.to_vtoken().unwrap();

	for who in ClassLocksFor::<T>::iter_keys() {
		let _ = T::MultiCurrency::remove_lock(CONVICTION_VOTING_ID, vtoken, &who);
		weight += T::DbWeight::get().writes(1);
	}

	let r1 = VotingFor::<T>::clear(u32::MAX, None);
	weight += T::DbWeight::get().writes(r1.loops as u64);

	let r2 = ClassLocksFor::<T>::clear(u32::MAX, None);
	weight += T::DbWeight::get().writes(r2.loops as u64);

	let r3 = ReferendumInfoFor::<T>::clear(u32::MAX, None);
	weight += T::DbWeight::get().writes(r3.loops as u64);

	let r4 = v1::DelegatorVote::<T>::clear(u32::MAX, None);
	weight += T::DbWeight::get().writes(r4.loops as u64);

	let r5 = v1::DelegatorVoteRole::<T>::clear(u32::MAX, None);
	weight += T::DbWeight::get().writes(r5.loops as u64);

	weight
}

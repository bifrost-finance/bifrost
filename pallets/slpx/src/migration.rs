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

use frame_support::{storage_alias, traits::OnRuntimeUpgrade};
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

use bifrost_primitives::currency::{ASTR, BNC, DOT, GLMR, KSM, MANTA, MOVR};

use crate::*;

pub struct BifrostKusamaAddCurrencyToSupportXcmFee<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for BifrostKusamaAddCurrencyToSupportXcmFee<T> {
	fn on_runtime_upgrade() -> Weight {
		//migrate the value type of SupportXcmFeeList
		let currency_list = BoundedVec::try_from(vec![BNC, MOVR, KSM]).unwrap();
		SupportXcmFeeList::<T>::put(currency_list);
		Weight::from(T::DbWeight::get().reads_writes(1 as u64 + 1, 2 as u64 + 1))
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let currency_count = SupportXcmFeeList::<T>::get().len();
		ensure!(currency_count == 0, "SupportXcmFeeList post-migrate storage count not match");

		Ok(sp_std::vec![])
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_cnt: Vec<u8>) -> Result<(), TryRuntimeError> {
		let currency_count_new = SupportXcmFeeList::<T>::get().len();
		ensure!(currency_count_new == 3, "Validators post-migrate storage count not match");

		Ok(())
	}
}

pub struct BifrostPolkadotAddCurrencyToSupportXcmFee<T>(sp_std::marker::PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for BifrostPolkadotAddCurrencyToSupportXcmFee<T> {
	fn on_runtime_upgrade() -> Weight {
		//migrate the value type of SupportXcmFeeList
		let currency_list = BoundedVec::try_from(vec![BNC, GLMR, DOT, ASTR, MANTA]).unwrap();
		SupportXcmFeeList::<T>::put(currency_list);
		Weight::from(T::DbWeight::get().reads_writes(1 as u64 + 1, 2 as u64 + 1))
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		let currency_count = SupportXcmFeeList::<T>::get().len();
		ensure!(currency_count == 0, "SupportXcmFeeList post-migrate storage count not match");

		Ok(sp_std::vec![])
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_cnt: Vec<u8>) -> Result<(), TryRuntimeError> {
		let currency_count_new = SupportXcmFeeList::<T>::get().len();
		ensure!(currency_count_new == 5, "Validators post-migrate storage count not match");

		Ok(())
	}
}

mod v0 {
	use frame_support::pallet_prelude::ValueQuery;

	use crate::types::OldOrder;

	use super::*;

	#[storage_alias]
	pub(super) type OrderQueue<T: Config> = StorageValue<
		Pallet<T>,
		BoundedVec<
			OldOrder<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>, BlockNumberFor<T>>,
			ConstU32<1000>,
		>,
		ValueQuery,
	>;
}

pub mod v1 {
	use frame_support::traits::StorageVersion;

	use super::*;

	pub struct MigrateToV1<T>(sp_std::marker::PhantomData<T>);
	impl<T: Config> OnRuntimeUpgrade for MigrateToV1<T> {
		fn on_runtime_upgrade() -> Weight {
			if StorageVersion::get::<Pallet<T>>() == 0 {
				let weight_consumed = migrate_to_v1::<T>();
				log::info!("Migrating slpx storage to v1");
				StorageVersion::new(1).put::<Pallet<T>>();
				weight_consumed.saturating_add(T::DbWeight::get().writes(1))
			} else {
				log::warn!("slpx migration should be removed.");
				T::DbWeight::get().reads(1)
			}
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::DispatchError> {
			log::info!("slpx before migration: version: {:?}", StorageVersion::get::<Pallet<T>>());
			log::info!("slpx before migration: v0 count: {}", v0::OrderQueue::<T>::get().len());

			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_: Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
			log::info!("slpx after migration: version: {:?}", StorageVersion::get::<Pallet<T>>());
			log::info!("slpx after migration: v1 count: {}", OrderQueue::<T>::get().len());

			Ok(())
		}
	}
}

pub fn migrate_to_v1<T: Config>() -> Weight {
	let mut weight: Weight = Weight::zero();

	let old_orders = v0::OrderQueue::<T>::get();

	let mut new_orders: BoundedVec<_, ConstU32<1000>> = BoundedVec::default();
	for old_order in old_orders.into_iter() {
		new_orders
			.try_push(Order {
				source_chain_caller: old_order.source_chain_caller,
				bifrost_chain_caller: old_order.bifrost_chain_caller,
				derivative_account: old_order.derivative_account,
				create_block_number: old_order.create_block_number,
				currency_id: old_order.currency_id,
				currency_amount: old_order.currency_amount,
				order_type: old_order.order_type,
				remark: old_order.remark,
				target_chain: old_order.target_chain,
				// default to 0
				channel_id: 0u32,
			})
			.expect("BoundedVec should not overflow");
		weight = weight.saturating_add(T::DbWeight::get().reads_writes(0, 1));
	}

	OrderQueue::<T>::put(new_orders);
	weight = weight.saturating_add(T::DbWeight::get().writes(1));

	v0::OrderQueue::<T>::kill();
	weight = weight.saturating_add(T::DbWeight::get().writes(1));

	weight
}

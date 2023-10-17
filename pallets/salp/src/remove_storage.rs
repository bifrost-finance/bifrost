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

use crate::{Config, QueryIdContributionInfo};
use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade};
use log;
use sp_std::marker::PhantomData;

pub struct RemoveUnusedQueryIdContributionInfo<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for RemoveUnusedQueryIdContributionInfo<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!("RemoveUnusedQueryIdContributionInfo::on_runtime_upgrade execute");

		for query_id in QueryIdContributionInfo::<T>::iter_keys() {
			let remove_list =
				[969u64, 949, 937, 938, 966, 954, 948, 968, 973, 956, 974, 950, 932, 962];
			if remove_list.contains(&query_id) {
				log::info!(
					"RemoveUnusedQueryIdContributionInfo::on_runtime_upgrade execute {:?}",
					query_id
				);
				QueryIdContributionInfo::<T>::remove(query_id)
			}
		}

		T::DbWeight::get().reads_writes(14u64, 14u64)
	}
}

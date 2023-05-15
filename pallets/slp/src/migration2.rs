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

use crate::{Config, Hash, Validators, ValidatorsByDelegator, Weight};
use frame_support::{log, pallet_prelude::*, traits::OnRuntimeUpgrade};
use sp_std::{marker::PhantomData, vec::Vec};
use xcm::v3::prelude::*;

pub struct MigrateValidatorsAndValidatorsByDelegatorStorages<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for MigrateValidatorsAndValidatorsByDelegatorStorages<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!("MigrateValidatorsAndValidatorsByDelegatorStorages starts.............",);

		let mut weight: Weight = Weight::zero();

		//migrate the value type of Validators
		Validators::<T>::translate(|_key, old_value: Vec<(MultiLocation, Hash<T>)>| {
			weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
			let mut new_value: Vec<MultiLocation> = Vec::new();
			for i in old_value {
				new_value.push(i.0);
			}
			Some(new_value)
		});

		//migrate the value type of Validators
		ValidatorsByDelegator::<T>::translate(
			|_key1, _key2, old_value: Vec<(MultiLocation, Hash<T>)>| {
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
				let mut new_value: Vec<MultiLocation> = Vec::new();
				for i in old_value {
					new_value.push(i.0);
				}
				Some(new_value)
			},
		);

		weight
	}
}

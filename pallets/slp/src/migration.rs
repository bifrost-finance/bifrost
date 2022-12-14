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

// #![cfg_attr(not(feature = "std"), no_std)]

use super::{Config, Weight};
use crate::{BalanceOf, XcmDestWeightAndFee, KSM};
use frame_support::traits::Get;
use sp_runtime::traits::UniqueSaturatedFrom;

pub fn update_vksm_xcm_fee<T: Config>() -> Weight {
	let mut write_count = 0;

	XcmDestWeightAndFee::<T>::translate::<(Weight, BalanceOf<T>), _>(
		|currency_id, _xcm_peration, (weight, fee)| {
			let mut new_fee = fee;
			if currency_id == KSM {
				new_fee = fee * BalanceOf::<T>::unique_saturated_from(10u128);
				write_count = write_count + 1;
			}

			Some((weight, new_fee))
		},
	);

	let entry_count = XcmDestWeightAndFee::<T>::iter().count() as u64;

	T::DbWeight::get().reads(entry_count) + T::DbWeight::get().writes(write_count)
}

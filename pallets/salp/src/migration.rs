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

#![cfg_attr(not(feature = "std"), no_std)]

use super::{Config, RedeemPool, Weight};
use frame_support::traits::Get;
use sp_runtime::traits::UniqueSaturatedInto;

pub fn update_redeem_pool<T: Config>() -> Weight {
	RedeemPool::<T>::set(147_780_374_204_392u128.unique_saturated_into());

	T::DbWeight::get().reads(1) + T::DbWeight::get().writes(1)
}

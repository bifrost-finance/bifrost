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

use crate::{BalanceOf, Config, TokenUnlockLedger};
use bifrost_primitives::{RedeemType, TimeUnit};
use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade};
use sp_std::marker::PhantomData;

/// Migrate TokenUnlockLedger
/// (T::AccountId, BalanceOf<T>, TimeUnit) to (T::AccountId, BalanceOf<T>,TimeUnit, RedeemType)
pub struct MigrateTokenUnlockLedger<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for MigrateTokenUnlockLedger<T> {
	fn on_runtime_upgrade() -> Weight {
		log::info!("MigrateTokenUnlockLedger::on_runtime_upgrade execute",);

		let mut weight: Weight = Weight::zero();

		// migrate the value type of TokenUnlockLedger
		TokenUnlockLedger::<T>::translate(
			|_key1, _key2, old_value: (T::AccountId, BalanceOf<T>, TimeUnit)| {
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 1));
				let new_value = (old_value.0, old_value.1, old_value.2, RedeemType::Native);
				Some(new_value)
			},
		);

		weight
	}
}

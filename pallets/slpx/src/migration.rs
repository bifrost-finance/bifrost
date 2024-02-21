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
use bifrost_primitives::currency::{ASTR, BNC, DOT, GLMR, KSM, MANTA, MOVR};
use frame_support::traits::OnRuntimeUpgrade;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;

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

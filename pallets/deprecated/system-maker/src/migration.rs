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

// Ensure we're `no_std` when compiling for Wasm.
use super::*;
pub use bifrost_primitives::currency::{KSM, VKSM};
use frame_support::{pallet_prelude::PhantomData, traits::OnRuntimeUpgrade};
use sp_core::Get;
use sp_runtime::traits::Zero;
pub struct SystemMakerClearPalletId<T>(PhantomData<T>);
impl<T: super::Config> OnRuntimeUpgrade for SystemMakerClearPalletId<T> {
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<sp_std::prelude::Vec<u8>, sp_runtime::DispatchError> {
		#[allow(unused_imports)]
		use frame_support::PalletId;
		log::info!("Bifrost `pre_upgrade`...");

		Ok(vec![])
	}

	fn on_runtime_upgrade() -> Weight {
		log::info!("Bifrost `on_runtime_upgrade`...");

		let account_id = T::SystemMakerPalletId::get().into_account_truncating();
		let ksm_balance = T::MultiCurrency::free_balance(KSM, &account_id);
		T::MultiCurrency::transfer(
			KSM,
			&T::SystemMakerPalletId::get().into_account_truncating(),
			&T::TreasuryAccount::get(),
			ksm_balance,
		)
		.ok();
		let vksm_balance = T::MultiCurrency::free_balance(VKSM, &account_id);
		T::MultiCurrency::transfer(
			VKSM,
			&T::SystemMakerPalletId::get().into_account_truncating(),
			&T::TreasuryAccount::get(),
			vksm_balance,
		)
		.ok();
		log::info!("KSM balance: {:?}", ksm_balance);
		log::info!("VKSM balance: {:?}", vksm_balance);

		log::info!("Bifrost `on_runtime_upgrade finished`");

		Weight::from(T::DbWeight::get().reads_writes(1, 1))
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_: sp_std::prelude::Vec<u8>) -> Result<(), sp_runtime::DispatchError> {
		#[allow(unused_imports)]
		use frame_support::PalletId;
		log::info!("Bifrost `post_upgrade`...");
		let account_id = T::SystemMakerPalletId::get().into_account_truncating();
		let ksm_balance = T::MultiCurrency::free_balance(KSM, &account_id);
		assert_eq!(ksm_balance, Zero::zero());
		let vksm_balance = T::MultiCurrency::free_balance(VKSM, &account_id);
		assert_eq!(vksm_balance, Zero::zero());

		Ok(())
	}
}

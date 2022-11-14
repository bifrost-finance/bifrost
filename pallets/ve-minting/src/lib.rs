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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, CheckedAdd, Saturating},
		ArithmeticError, Perbill,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{AccountId, Balance, BlockNumber, CurrencyId, Timestamp};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_core::U256;
// use sp_std::vec::Vec;
pub use weights::WeightInfo;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct LockedBalance {
	amount: Balance,
	end: BlockNumber,
}

// pub type Epoch = U256;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct Point {
	bias: Balance, // i128
	slope: i128,   // dweight / dt
	ts: Timestamp,
	blk: BlockNumber, // block
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type ControlOrigin: EnsureOrigin<Self::Origin>;

		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type VeMintingPalletId: Get<PalletId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created {},
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		NotSupportProportion,
		CalculationOverflow,
		ExistentialDeposit,
		DistributionNotExist,
	}

	#[pallet::storage]
	#[pallet::getter(fn supply)]
	pub type Supply<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ve_config)]
	pub type VeConfig<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn epoch)]
	pub type Epoch<T: Config> = StorageValue<_, U256, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn locked)]
	pub type Locked<T: Config> = StorageMap<_, Blake2_128Concat, AccountIdOf<T>, LockedBalance>;

	#[pallet::storage]
	#[pallet::getter(fn point_history)]
	pub type PointHistory<T: Config> = StorageMap<_, Twox64Concat, U256, Point>;

	#[pallet::storage]
	#[pallet::getter(fn user_point_history)]
	pub type UserPointHistory<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, AccountIdOf<T>, Blake2_128Concat, U256, Point>;

	#[pallet::storage]
	#[pallet::getter(fn slope_changes)]
	pub type SlopeChanges<T: Config> = StorageMap<_, Twox64Concat, Timestamp, i128>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(_bn: BlockNumberFor<T>, _remaining_weight: Weight) -> Weight {
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::set_minimum_mint())]
		pub fn create_distribution(origin: OriginFor<T>) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Self::deposit_event(Event::Created {});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn _checkpoint(addr: AccountId, old_locked: LockedBalance, new_locked: LockedBalance) {
			let _u_old = Point::default();
		}
		// fn execute_distribute_inner() -> DispatchResult {
		// 	Ok(())
		// }
	}
}

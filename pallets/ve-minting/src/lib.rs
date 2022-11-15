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
		traits::{AccountIdConversion, CheckedAdd, Saturating, Zero},
		ArithmeticError, Perbill, SaturatedConversion,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{AccountId, CurrencyId, Timestamp}; // BlockNumber, Balance
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
pub struct VeConfig<Balance> {
	amount: Balance,
	end: Timestamp,
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct LockedBalance<Balance> {
	amount: Balance,
	end: Timestamp,
}

// pub type Epoch = U256;

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub struct Point<Balance, BlockNumber> {
	bias: Balance,  // i128
	slope: Balance, // dweight / dt
	ts: Timestamp,
	blk: BlockNumber, // block
	fxs_amt: Balance,
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
	pub type Locked<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountIdOf<T>, LockedBalance<BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn point_history)]
	pub type PointHistory<T: Config> =
		StorageMap<_, Twox64Concat, U256, Point<BalanceOf<T>, BlockNumberFor<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_point_history)]
	pub type UserPointHistory<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountId, // AccountIdOf<T>
		Blake2_128Concat,
		U256,
		Point<BalanceOf<T>, BlockNumberFor<T>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn user_point_epoch)]
	pub type UserPointEpoch<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId, U256, ValueQuery>; // AccountIdOf<T>

	#[pallet::storage]
	#[pallet::getter(fn slope_changes)]
	pub type SlopeChanges<T: Config> =
		StorageMap<_, Twox64Concat, Timestamp, BalanceOf<T>, ValueQuery>;

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
		fn _checkpoint(
			addr: AccountId,
			old_locked: LockedBalance<BalanceOf<T>>,
			new_locked: LockedBalance<BalanceOf<T>>,
		) {
			let mut u_old = Point::default();
			let mut u_new = Point::default();
			let old_dslope = BalanceOf::<T>::zero();
			let new_dslope = BalanceOf::<T>::zero();
			let g_epoch: U256 = Self::epoch();

			let current_block_number: T::BlockNumber =
				frame_system::Pallet::<T>::block_number().into(); // BlockNumberFor<T>
			let current_timestamp: Timestamp =
				sp_timestamp::InherentDataProvider::from_system_time().timestamp().as_millis();
			if old_locked.end > current_timestamp && old_locked.amount > BalanceOf::<T>::zero() {
				u_old.slope = old_locked.amount / MAXTIME;
				u_old.bias = u_old
					.slope
					.saturating_mul((old_locked.end - current_timestamp).saturated_into());
			}
			if new_locked.end > current_timestamp && new_locked.amount > BalanceOf::<T>::zero() {
				u_new.slope = new_locked.amount / MAXTIME;
				u_new.bias = u_new
					.slope
					.saturating_mul((new_locked.end - current_timestamp).saturated_into());
			}

			old_dslope = Self::slope_changes(old_locked.end);
			if new_locked.end != 0 {
				if new_locked.end == old_locked.end {
					new_dslope = old_dslope
				} else {
					new_dslope = Self::slope_changes(new_locked.end)
				}
			}

			let last_point: Point<BalanceOf<T>, BlockNumberFor<T>> = Point {
				bias: Zero::zero(),
				slope: Zero::zero(),
				ts: current_timestamp,
				blk: current_block_number,
				fxs_amt: Zero::zero(),
			};
			if g_epoch > U256::zero() {
				last_point = Self::point_history(g_epoch);
				// } else {
				// 	last_point.fxs_amt = ERC20(Self::token).balanceOf(self)
			}
			let last_checkpoint = last_point.ts;
		}

		fn balanceOf(addr: AccountId, _t: Timestamp) -> BalanceOf<T> {
			let u_epoch = Self::user_point_epoch(addr);
			if u_epoch == U256::zero() {
				return Zero::zero();
			} else {
				let last_point: Point<BalanceOf<T>, BlockNumberFor<T>> =
					Self::user_point_history(addr, u_epoch);
				last_point.bias -=
					last_point.slope.saturating_mul((_t - last_point.ts).saturated_into());
				// .ok_or(ArithmeticError::Overflow)?;
				if last_point.bias < Zero::zero() {
					last_point.bias = Zero::zero();
				}
				last_point.bias
			}
		}
	}
}

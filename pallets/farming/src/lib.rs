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

pub mod primitives;
pub mod weights;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::{AccountIdConversion, CheckedSub},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
pub use pallet::*;
pub use primitives::{VstokenConversionExchangeFee, VstokenConversionExchangeRate};
use sp_arithmetic::per_things::Percent;
use sp_std::vec::Vec;
pub use weights::WeightInfo;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type ControlOrigin: EnsureOrigin<Self::Origin>;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type VsbondAccount: Get<PalletId>;

		/// Set default weight.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FarmingPoolCreated {},
		ExchangeFeeSet { exchange_fee: VstokenConversionExchangeFee<BalanceOf<T>> },
		ExchangeRateSet { lease: i32, exchange_rate: VstokenConversionExchangeRate },
		PolkadotLeaseSet { lease: u32 },
		KusamaLeaseSet { lease: u32 },
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		NotSupportTokenType,
		CalculationOverflow,
	}

	#[pallet::storage]
	#[pallet::getter(fn kusama_lease)]
	pub type KusamaLease<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn polkadot_lease)]
	pub type PolkadotLease<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn exchange_rate)]
	pub type ExchangeRate<T: Config> =
		StorageMap<_, Twox64Concat, i32, VstokenConversionExchangeRate, ValueQuery>;

	/// exchange fee
	#[pallet::storage]
	#[pallet::getter(fn exchange_fee)]
	pub type ExchangeFee<T: Config> =
		StorageValue<_, VstokenConversionExchangeFee<BalanceOf<T>>, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[transactional]
		#[pallet::weight(10000)]
		pub fn deposit(
			origin: OriginFor<T>,
			pid: CurrencyIdOf<T>,
			vsdot_amount: BalanceOf<T>,
			minimum_vsbond: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn create_farming_pool(
			origin: OriginFor<T>,
			// tokens: BoundedVec<(CurrencyIdOf<T>, u32)>,
			// basic_reward: BoundedVec<(CurrencyIdOf<T>, Balance)>,
			gauge_token: Option<CurrencyIdOf<T>>,
			charge_account: AccountIdOf<T>,
			#[pallet::compact] min_deposit_to_start: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
			#[pallet::compact] after_block_to_start: BlockNumberFor<T>,
			#[pallet::compact] withdraw_limit_time: BlockNumberFor<T>,
			#[pallet::compact] claim_limit_time: BlockNumberFor<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			// Self::deposit_event(Event::FarmingPoolCreated {  });
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn force_retire_pool(origin: OriginFor<T>, pid: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn close_pool(origin: OriginFor<T>, pid: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn reset_pool(origin: OriginFor<T>, pid: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn kill_pool(origin: OriginFor<T>, pid: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn edit_pool(origin: OriginFor<T>, pid: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}
	}
}

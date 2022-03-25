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
		traits::{AccountIdConversion, CheckedAdd, CheckedMul, CheckedSub, Saturating, Zero},
		SaturatedConversion,
	},
	transactional, BoundedVec, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_arithmetic::per_things::{PerThing, Perbill, Percent};
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

pub type MintId = u32;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::traits::tokens::currency;

	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
		// + MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::Origin>;

		#[pallet::constant]
		type VsbondAccount: Get<PalletId>;

		/// Set default weight.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		VsbondConvertToVsksm {
			currency_id: CurrencyIdOf<T>,
			vsbond_amount: BalanceOf<T>,
			vsksm_amount: BalanceOf<T>,
		},
		/// Several fees has been set.
		FeeSet {
			mint_fee: BalanceOf<T>,
			redeem_fee: BalanceOf<T>,
			// hosting_fee: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		BelowMinimumMint,
		Unexpected,
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
		StorageMap<_, Twox64Concat, u32, (Percent, Percent), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn exchange_fee)]
	pub type ExchangeFee<T: Config> = StorageValue<_, (BalanceOf<T>, BalanceOf<T>), ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// #[pallet::weight(T::WeightInfo::mint())]
		#[transactional]
		#[pallet::weight(10000)]
		pub fn vsbond_convert_to_vsksm(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			vsbond_amount: BalanceOf<T>,
			minimum_vsksm: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			let mut user_vsbond_balance = T::MultiCurrency::free_balance(currency_id, &exchanger);
			ensure!(user_vsbond_balance >= vsbond_amount, Error::<T>::BelowMinimumMint);
			ensure!(
				minimum_vsksm >=
					T::MultiCurrency::minimum_balance(CurrencyId::Token(TokenSymbol::KSM)),
				Error::<T>::BelowMinimumMint
			);
			let ksm_lease = KusamaLease::<T>::get();
			let remaining_due_lease: u32 = match currency_id {
				CurrencyId::VSBond(_, _, _, expire_lease) => {
					let mut remaining_due_lease =
						expire_lease.checked_sub(ksm_lease).ok_or(Error::<T>::Unexpected)?;
					remaining_due_lease =
						remaining_due_lease.checked_add(1u32).ok_or(Error::<T>::Unexpected)?;
					remaining_due_lease
				},
				_ => return Err(Error::<T>::BelowMinimumMint.into()),
			};
			let (convert_to_vsksm, convert_to_vsbond) = ExchangeRate::<T>::get(remaining_due_lease);
			let (kusama_exchange_fee, _) = ExchangeFee::<T>::get();
			let vsbond_balance =
				vsbond_amount.checked_sub(&kusama_exchange_fee).ok_or(Error::<T>::Unexpected)?;
			let vsksm_balance = convert_to_vsksm * vsbond_balance;
			ensure!(vsksm_balance >= minimum_vsksm, Error::<T>::BelowMinimumMint);
			T::MultiCurrency::transfer(
				currency_id,
				&exchanger,
				&T::VsbondAccount::get().into_account(),
				vsbond_amount,
			)?;
			T::MultiCurrency::deposit(
				CurrencyId::VSToken(TokenSymbol::KSM),
				&exchanger,
				vsksm_balance,
			)?;

			Self::deposit_event(Event::VsbondConvertToVsksm {
				currency_id,
				vsbond_amount,
				vsksm_amount: vsksm_balance,
			});
			Ok(())
		}
	}
}

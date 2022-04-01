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
		VsbondConvertToVsksm {
			currency_id: CurrencyIdOf<T>,
			vsbond_amount: BalanceOf<T>,
			vsksm_amount: BalanceOf<T>,
		},
		VsksmConvertToVsbond {
			currency_id: CurrencyIdOf<T>,
			vsbond_amount: BalanceOf<T>,
			vsksm_amount: BalanceOf<T>,
		},
		VsbondConvertToVsdot {
			currency_id: CurrencyIdOf<T>,
			vsbond_amount: BalanceOf<T>,
			vsdot_amount: BalanceOf<T>,
		},
		VsdotConvertToVsbond {
			currency_id: CurrencyIdOf<T>,
			vsbond_amount: BalanceOf<T>,
			vsdot_amount: BalanceOf<T>,
		},
		ExchangeFeeSet {
			exchange_fee: VstokenConversionExchangeFee<BalanceOf<T>>,
		},
		ExchangeRateSet {
			lease: i32,
			exchange_rate: VstokenConversionExchangeRate,
		},
		PolkadotLeaseSet {
			lease: u32,
		},
		KusamaLeaseSet {
			lease: u32,
		},
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
		pub fn vsbond_convert_to_vsksm(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			vsbond_amount: BalanceOf<T>,
			minimum_vsksm: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			let user_vsbond_balance = T::MultiCurrency::free_balance(currency_id, &exchanger);
			ensure!(user_vsbond_balance >= vsbond_amount, Error::<T>::NotEnoughBalance);
			ensure!(
				minimum_vsksm >=
					T::MultiCurrency::minimum_balance(CurrencyId::Token(TokenSymbol::KSM)),
				Error::<T>::NotEnoughBalance
			);

			// Calculate lease
			let ksm_lease = KusamaLease::<T>::get();
			let mut remaining_due_lease: i32 = match currency_id {
				CurrencyId::VSBond(symbol, _, _, expire_lease) => {
					ensure!(
						symbol == TokenSymbol::KSM || symbol == TokenSymbol::BNC,
						Error::<T>::NotSupportTokenType
					);
					let mut remaining_due_lease: i32 = expire_lease
						.checked_sub(ksm_lease)
						.ok_or(Error::<T>::CalculationOverflow)?
						.try_into()
						.map_err(|_| Error::<T>::CalculationOverflow)?;
					remaining_due_lease = remaining_due_lease
						.checked_add(1i32)
						.ok_or(Error::<T>::CalculationOverflow)?;
					remaining_due_lease
				},
				_ => return Err(Error::<T>::NotSupportTokenType.into()),
			};
			ensure!(remaining_due_lease <= 8i32, Error::<T>::NotSupportTokenType);

			// Get exchange rate, exchange fee
			if remaining_due_lease < 0i32 {
				remaining_due_lease = 0i32
			}
			let exchange_rate = ExchangeRate::<T>::get(remaining_due_lease);
			let exchange_fee = ExchangeFee::<T>::get();
			let vsbond_balance = vsbond_amount
				.checked_sub(&exchange_fee.vsbond_exchange_fee_of_vsksm)
				.ok_or(Error::<T>::CalculationOverflow)?;
			let vsksm_balance = exchange_rate.vsbond_convert_to_vsksm * vsbond_balance;
			ensure!(vsksm_balance >= minimum_vsksm, Error::<T>::NotEnoughBalance);

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
			T::MultiCurrency::deposit(
				currency_id,
				&T::TreasuryAccount::get(),
				exchange_fee.vsbond_exchange_fee_of_vsksm,
			)?;

			Self::deposit_event(Event::VsbondConvertToVsksm {
				currency_id,
				vsbond_amount,
				vsksm_amount: vsksm_balance,
			});
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn vsksm_convert_to_vsbond(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			vsksm_amount: BalanceOf<T>,
			minimum_vsbond: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			let user_vsksm_balance =
				T::MultiCurrency::free_balance(CurrencyId::VSToken(TokenSymbol::KSM), &exchanger);
			ensure!(user_vsksm_balance >= vsksm_amount, Error::<T>::NotEnoughBalance);
			ensure!(
				minimum_vsbond >= T::MultiCurrency::minimum_balance(currency_id),
				Error::<T>::NotEnoughBalance
			);

			// Calculate lease
			let ksm_lease = KusamaLease::<T>::get();
			let mut remaining_due_lease: i32 = match currency_id {
				CurrencyId::VSBond(symbol, _, _, expire_lease) => {
					ensure!(
						symbol == TokenSymbol::KSM || symbol == TokenSymbol::BNC,
						Error::<T>::NotSupportTokenType
					);
					let mut remaining_due_lease: i32 = (expire_lease as i64 - ksm_lease as i64)
						.try_into()
						.map_err(|_| Error::<T>::CalculationOverflow)?;
					remaining_due_lease = remaining_due_lease
						.checked_add(1i32)
						.ok_or(Error::<T>::CalculationOverflow)?;
					remaining_due_lease
				},
				_ => return Err(Error::<T>::NotSupportTokenType.into()),
			};
			ensure!(remaining_due_lease <= 8i32, Error::<T>::NotSupportTokenType);

			// Get exchange rate, exchange fee
			if remaining_due_lease < 0i32 {
				remaining_due_lease = 0i32
			}
			let exchange_rate = ExchangeRate::<T>::get(remaining_due_lease);
			let exchange_fee = ExchangeFee::<T>::get();
			let vsksm_balance = vsksm_amount
				.checked_sub(&exchange_fee.vsksm_exchange_fee)
				.ok_or(Error::<T>::CalculationOverflow)?;
			let vsbond_balance = exchange_rate.vsksm_convert_to_vsbond * vsksm_balance;
			ensure!(vsbond_balance >= minimum_vsbond, Error::<T>::NotEnoughBalance);

			T::MultiCurrency::transfer(
				currency_id,
				&T::VsbondAccount::get().into_account(),
				&exchanger,
				vsbond_balance,
			)?;
			T::MultiCurrency::withdraw(
				CurrencyId::VSToken(TokenSymbol::KSM),
				&exchanger,
				vsksm_amount,
			)?;
			T::MultiCurrency::deposit(
				CurrencyId::VSToken(TokenSymbol::KSM),
				&T::TreasuryAccount::get(),
				exchange_fee.vsksm_exchange_fee,
			)?;

			Self::deposit_event(Event::VsksmConvertToVsbond {
				currency_id,
				vsbond_amount: vsbond_balance,
				vsksm_amount: vsksm_balance,
			});
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn vsbond_convert_to_vsdot(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			vsbond_amount: BalanceOf<T>,
			minimum_vsdot: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			let user_vsbond_balance = T::MultiCurrency::free_balance(currency_id, &exchanger);
			ensure!(user_vsbond_balance >= vsbond_amount, Error::<T>::NotEnoughBalance);
			ensure!(
				minimum_vsdot >=
					T::MultiCurrency::minimum_balance(CurrencyId::Token(TokenSymbol::DOT)),
				Error::<T>::NotEnoughBalance
			);

			// Calculate lease
			let dot_lease = PolkadotLease::<T>::get();
			let mut remaining_due_lease: i32 = match currency_id {
				CurrencyId::VSBond(symbol, _, _, expire_lease) => {
					ensure!(symbol == TokenSymbol::DOT, Error::<T>::NotSupportTokenType);
					let mut remaining_due_lease: i32 = (expire_lease as i64 - dot_lease as i64)
						.try_into()
						.map_err(|_| Error::<T>::CalculationOverflow)?;
					remaining_due_lease = remaining_due_lease
						.checked_add(1i32)
						.ok_or(Error::<T>::CalculationOverflow)?;
					remaining_due_lease
				},
				_ => return Err(Error::<T>::NotSupportTokenType.into()),
			};
			ensure!(remaining_due_lease <= 8i32, Error::<T>::NotSupportTokenType);

			// Get exchange rate, exchange fee
			if remaining_due_lease < 0i32 {
				remaining_due_lease = 0i32
			}
			let exchange_rate = ExchangeRate::<T>::get(remaining_due_lease);
			let exchange_fee = ExchangeFee::<T>::get();
			let vsbond_balance = vsbond_amount
				.checked_sub(&exchange_fee.vsbond_exchange_fee_of_vsdot)
				.ok_or(Error::<T>::CalculationOverflow)?;
			let vsdot_balance = exchange_rate.vsbond_convert_to_vsdot * vsbond_balance;
			ensure!(vsdot_balance >= minimum_vsdot, Error::<T>::NotEnoughBalance);

			T::MultiCurrency::transfer(
				currency_id,
				&exchanger,
				&T::VsbondAccount::get().into_account(),
				vsbond_amount,
			)?;
			T::MultiCurrency::deposit(
				CurrencyId::VSToken(TokenSymbol::DOT),
				&exchanger,
				vsdot_balance,
			)?;
			T::MultiCurrency::deposit(
				currency_id,
				&T::TreasuryAccount::get(),
				exchange_fee.vsbond_exchange_fee_of_vsdot,
			)?;

			Self::deposit_event(Event::VsbondConvertToVsdot {
				currency_id,
				vsbond_amount,
				vsdot_amount: vsdot_balance,
			});
			Ok(())
		}

		#[transactional]
		#[pallet::weight(10000)]
		pub fn vsdot_convert_to_vsbond(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			vsdot_amount: BalanceOf<T>,
			minimum_vsbond: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			let user_vsdot_balance =
				T::MultiCurrency::free_balance(CurrencyId::VSToken(TokenSymbol::DOT), &exchanger);
			ensure!(user_vsdot_balance >= vsdot_amount, Error::<T>::NotEnoughBalance);
			ensure!(
				minimum_vsbond >= T::MultiCurrency::minimum_balance(currency_id),
				Error::<T>::NotEnoughBalance
			);

			// Calculate lease
			let dot_lease = PolkadotLease::<T>::get();
			let mut remaining_due_lease: i32 = match currency_id {
				CurrencyId::VSBond(symbol, _, _, expire_lease) => {
					ensure!(symbol == TokenSymbol::DOT, Error::<T>::NotSupportTokenType);
					let mut remaining_due_lease: i32 = (expire_lease as i64 - dot_lease as i64)
						.try_into()
						.map_err(|_| Error::<T>::CalculationOverflow)?;
					remaining_due_lease = remaining_due_lease
						.checked_add(1i32)
						.ok_or(Error::<T>::CalculationOverflow)?;
					remaining_due_lease
				},
				_ => return Err(Error::<T>::NotSupportTokenType.into()),
			};
			ensure!(remaining_due_lease <= 8i32, Error::<T>::NotSupportTokenType);

			// Get exchange rate, exchange fee
			if remaining_due_lease <= 0i32 {
				remaining_due_lease = 0i32
			}
			let exchange_rate = ExchangeRate::<T>::get(remaining_due_lease);
			let exchange_fee = ExchangeFee::<T>::get();
			let vsdot_balance = vsdot_amount
				.checked_sub(&exchange_fee.vsdot_exchange_fee)
				.ok_or(Error::<T>::CalculationOverflow)?;
			let vsbond_balance = exchange_rate.vsdot_convert_to_vsbond * vsdot_balance;
			ensure!(vsbond_balance >= minimum_vsbond, Error::<T>::NotEnoughBalance);

			T::MultiCurrency::transfer(
				currency_id,
				&T::VsbondAccount::get().into_account(),
				&exchanger,
				vsbond_balance,
			)?;
			T::MultiCurrency::withdraw(
				CurrencyId::VSToken(TokenSymbol::DOT),
				&exchanger,
				vsdot_amount,
			)?;
			T::MultiCurrency::deposit(
				CurrencyId::VSToken(TokenSymbol::DOT),
				&T::TreasuryAccount::get(),
				exchange_fee.vsdot_exchange_fee,
			)?;

			Self::deposit_event(Event::VsdotConvertToVsbond {
				currency_id,
				vsbond_amount: vsbond_balance,
				vsdot_amount: vsdot_balance,
			});
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_exchange_fee(
			origin: OriginFor<T>,
			exchange_fee: VstokenConversionExchangeFee<BalanceOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			ExchangeFee::<T>::mutate(|old_exchange_fee| {
				*old_exchange_fee = exchange_fee.clone();
			});

			Self::deposit_event(Event::ExchangeFeeSet { exchange_fee });
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_exchange_rate(
			origin: OriginFor<T>,
			lease: i32,
			exchange_rate: VstokenConversionExchangeRate,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			ExchangeRate::<T>::mutate(lease, |old_exchange_rate| {
				*old_exchange_rate = exchange_rate.clone();
			});

			Self::deposit_event(Event::ExchangeRateSet { lease, exchange_rate });
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_kusama_lease(origin: OriginFor<T>, lease: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			KusamaLease::<T>::mutate(|old_lease| {
				*old_lease = lease;
			});

			Self::deposit_event(Event::KusamaLeaseSet { lease });
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_polkadot_lease(origin: OriginFor<T>, lease: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			PolkadotLease::<T>::mutate(|old_lease| {
				*old_lease = lease;
			});

			Self::deposit_event(Event::PolkadotLeaseSet { lease });
			Ok(())
		}
	}
}

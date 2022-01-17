// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use frame_support::{pallet_prelude::*, transactional, PalletId};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
use sp_arithmetic::per_things::Percent;
use sp_runtime::traits::{AccountIdConversion, Saturating, UniqueSaturatedFrom, Zero};
pub use weights::WeightInfo;

mod mock;
mod tests;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pallet::*;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

const TRILLION: u128 = 1_000_000_000_000;
// These time units are defined in number of blocks.
const BLOCKS_PER_DAY: u32 = 60 / 12 * 60 * 24;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Currency operations handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
		/// The only origin that can modify bootstrap params
		type ControlOrigin: EnsureOrigin<Self::Origin>;
		/// Set default weight.
		type WeightInfo: WeightInfo;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotKSM,
		DenominatorZero,
		NotGreaterThanZero,
		ExceedPoolAmount,
		NotEnoughBalance,
		InvalidReleaseInterval,
		Overflow,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// [exchanger, ksm_amount]
		KSMExchanged(AccountIdOf<T>, BalanceOf<T>),
		/// [adder, ksm_amount]
		KSMAdded(AccountIdOf<T>, BalanceOf<T>),
		/// [original_prce, new_price]
		PriceEdited(BalanceOf<T>, BalanceOf<T>),
		/// [start, end]
		BlockIntervalEdited(BlockNumberFor<T>, BlockNumberFor<T>),
		/// [originla_amount_per_day, amount_per_day]
		ReleasedPerDayEdited(BalanceOf<T>, BalanceOf<T>),
	}

	/// The remaining amount which can be exchanged for
	#[pallet::storage]
	#[pallet::getter(fn get_pool_amount)]
	pub type PoolAmount<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// token amount that is released everyday.
	#[pallet::storage]
	#[pallet::getter(fn get_token_release_per_round)]
	pub type TokenReleasePerDay<T: Config> =
		StorageValue<_, BalanceOf<T>, ValueQuery, DefaultReleaseAmount<T>>;

	// Defult release amount is 30 KSM
	#[pallet::type_value]
	pub fn DefaultReleaseAmount<T: Config>() -> BalanceOf<T> {
		BalanceOf::<T>::unique_saturated_from(TRILLION.saturating_mul(30))
	}

	/// Token release start block
	#[pallet::storage]
	#[pallet::getter(fn get_start_and_end_release_block)]
	pub type StartEndReleaseBlock<T: Config> =
		StorageValue<_, (BlockNumberFor<T>, BlockNumberFor<T>), ValueQuery>;

	/// Exchange price discount: vsbond + vstoken => token
	#[pallet::storage]
	#[pallet::getter(fn get_exchange_price_discount)]
	pub type ExchangePriceDiscount<T: Config> =
		StorageValue<_, Percent, ValueQuery, DefaultPrice<T>>;

	// Defult price is 90%
	#[pallet::type_value]
	pub fn DefaultPrice<T: Config>() -> Percent {
		Percent::from_rational(90u32, 100u32)
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let (start, end) = Self::get_start_and_end_release_block();
			// relsease fixed amount every day if within release interval and has enough balance in
			// the pool account
			if (n <= end) & (n > start) {
				if (n - start) % BLOCKS_PER_DAY.into() == Zero::zero() {
					let ksm = CurrencyId::Token(TokenSymbol::KSM);
					let pool_account: AccountIdOf<T> = T::PalletId::get().into_account();
					let releae_per_day = Self::get_token_release_per_round();
					let total_amount = Self::get_pool_amount().saturating_add(releae_per_day);

					if T::MultiCurrency::ensure_can_withdraw(ksm, &pool_account, total_amount)
						.is_ok()
					{
						PoolAmount::<T>::mutate(|amt| *amt = total_amount);
					}
				}
			}
			T::WeightInfo::on_initialize()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Anyone can add KSM to the pool.
		#[pallet::weight(T::WeightInfo::add_ksm_to_pool())]
		#[transactional]
		pub fn add_ksm_to_pool(origin: OriginFor<T>, token_amount: BalanceOf<T>) -> DispatchResult {
			let adder = ensure_signed(origin)?;
			let ksm_id = CurrencyId::Token(TokenSymbol::KSM);

			let token_balance = T::MultiCurrency::free_balance(ksm_id, &adder);
			ensure!(token_balance >= token_amount, Error::<T>::NotEnoughBalance);

			let pool_account: AccountIdOf<T> = T::PalletId::get().into_account();
			T::MultiCurrency::transfer(ksm_id, &adder, &pool_account, token_amount)?;

			Self::deposit_event(Event::KSMAdded(adder, token_amount));

			Ok(())
		}

		// exchange vsksm and vsbond for ksm
		#[pallet::weight(T::WeightInfo::exchange_for_ksm())]
		#[transactional]
		pub fn exchange_for_ksm(
			origin: OriginFor<T>,
			token_amount: BalanceOf<T>, // The KSM amount the user exchanges for
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			ensure!(token_amount <= Self::get_pool_amount(), Error::<T>::ExceedPoolAmount);

			// Check exchanger's vsksm and vsbond balance
			let vsksm = CurrencyId::VSToken(TokenSymbol::KSM);
			let vsbond = CurrencyId::VSBond(TokenSymbol::BNC, 2001, 13, 20);
			let ksm = CurrencyId::Token(TokenSymbol::KSM);

			ensure!(
				Self::get_exchange_price_discount() != Percent::zero(),
				Error::<T>::DenominatorZero
			);
			let amount_needed =
				Self::get_exchange_price_discount().saturating_reciprocal_mul(token_amount);

			let vsksm_balance = T::MultiCurrency::free_balance(vsksm, &exchanger);
			let vsbond_balance = T::MultiCurrency::free_balance(vsbond, &exchanger);
			ensure!(vsksm_balance >= amount_needed, Error::<T>::NotEnoughBalance);
			ensure!(vsbond_balance >= amount_needed, Error::<T>::NotEnoughBalance);

			// Make changes to account token balances
			let pool_account: AccountIdOf<T> = T::PalletId::get().into_account();
			T::MultiCurrency::ensure_can_withdraw(ksm, &pool_account, token_amount)?;
			PoolAmount::<T>::mutate(|amt| *amt = amt.saturating_sub(token_amount));

			T::MultiCurrency::transfer(vsksm, &exchanger, &pool_account, amount_needed)?;
			T::MultiCurrency::transfer(vsbond, &exchanger, &pool_account, amount_needed)?;
			T::MultiCurrency::transfer(ksm, &pool_account, &exchanger, token_amount)?;

			Self::deposit_event(Event::KSMExchanged(exchanger, token_amount));

			Ok(())
		}

		// edit exchange discount price
		#[pallet::weight(T::WeightInfo::edit_exchange_price())]
		#[transactional]
		pub fn edit_exchange_price(
			origin: OriginFor<T>,
			price: BalanceOf<T>, /* the mumber of ksm we can get by giving out 100 vsksm and 100
			                      * vsbond */
		) -> DispatchResult {
			// Check origin
			T::ControlOrigin::ensure_origin(origin)?;
			ensure!(price <= BalanceOf::<T>::unique_saturated_from(100u128), Error::<T>::Overflow);

			let price_percent: Percent =
				Percent::from_rational(price, BalanceOf::<T>::unique_saturated_from(100u128));
			let original_price: BalanceOf<T> = Self::get_exchange_price_discount()
				.mul_floor(BalanceOf::<T>::unique_saturated_from(100u128));

			ExchangePriceDiscount::<T>::mutate(|p| *p = price_percent);

			Self::deposit_event(Event::PriceEdited(original_price, price));

			Ok(())
		}

		// edit token release amount per day
		#[pallet::weight(T::WeightInfo::edit_release_per_day())]
		#[transactional]
		pub fn edit_release_per_day(
			origin: OriginFor<T>,
			amount_per_day: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			T::ControlOrigin::ensure_origin(origin)?;
			ensure!(amount_per_day > Zero::zero(), Error::<T>::NotGreaterThanZero);

			let originla_amount_per_day = Self::get_token_release_per_round();
			TokenReleasePerDay::<T>::mutate(|amt| *amt = amount_per_day);

			Self::deposit_event(Event::ReleasedPerDayEdited(
				originla_amount_per_day,
				amount_per_day,
			));

			Ok(())
		}

		// edit token release start and end block
		#[pallet::weight(T::WeightInfo::edit_release_start_and_end_block())]
		#[transactional]
		pub fn edit_release_start_and_end_block(
			origin: OriginFor<T>,
			start: BlockNumberFor<T>,
			end: BlockNumberFor<T>,
		) -> DispatchResult {
			// Check origin
			T::ControlOrigin::ensure_origin(origin)?;

			let current_block_number = frame_system::Pallet::<T>::block_number(); // get current block number
			ensure!(start > current_block_number, Error::<T>::InvalidReleaseInterval);
			ensure!(end >= start, Error::<T>::InvalidReleaseInterval);

			StartEndReleaseBlock::<T>::mutate(|interval| *interval = (start, end));

			Self::deposit_event(Event::BlockIntervalEdited(start, end));

			Ok(())
		}
	}
}

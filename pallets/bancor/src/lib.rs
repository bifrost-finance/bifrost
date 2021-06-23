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

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
use sp_runtime::{SaturatedConversion, traits::{Zero, Saturating}};
use num_integer::Roots;
use node_primitives::{TokenSymbol, CurrencyId};
use sp_arithmetic::per_things::Permill;

mod mock;
mod tests;

pub use pallet::*;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::MultiCurrenciesHandler as MultiCurrency<AccountIdOf<T>>>::Balance;

const TWELVE_TEN: u128 = 1_000_000_000_000;
const MILLION: u128 = 1_000_000;

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug)]
pub struct BancorPool<Balance>{
	pub(crate) currency_id: CurrencyId,  // ksm, dot, etc.
	pub(crate) token_pool: Balance,  // token balance of the pool
	pub(crate) vstoken_pool: Balance, // vstoken balance of the pool
	pub(crate) token_base_supply: Balance, // initial supply of token for the pool
	pub(crate) vstoken_base_supply: Balance,  // initial supply of vstoken for the pool
}

pub trait BancorHandler<Balance> {
	fn add_token(currency_id: CurrencyId, amount: Balance) -> Result<(), DispatchError>;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type MultiCurrenciesHandler: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		CurrencyIdNotExist,
		AmountNotGreaterThanZero,
		BancorPoolNotExist,
		ConversionError,
		TokenSupplyNotEnought,
		VSTokenSupplyNotEnought
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Token has been sold.
		///
		/// [buyer, currencyId, token_sold, vsToken_paid]
		TokenSold(AccountIdOf<T>, CurrencyId, BalanceOf<T>, BalanceOf<T>),
		/// [buyer, currencyId, vsToken_sold, Token_paid]
		VSTokenSold(AccountIdOf<T>, CurrencyId, BalanceOf<T>, BalanceOf<T>),
	}

	// key is token, value is BancorPool struct.
	#[pallet::storage]
	#[pallet::getter(fn get_bancor_pool)]
	pub type BancorPools<T> = StorageMap<Hasher = Blake2_128Concat, Key = CurrencyId, Value = BancorPool<BalanceOf<T>>>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub bancor_pools: Vec<(CurrencyId, BalanceOf<T>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				bancor_pools: vec![
					(CurrencyId::Token(TokenSymbol::DOT), BalanceOf::<T>::saturated_from(10_000 * TWELVE_TEN as u128)),
					(CurrencyId::Token(TokenSymbol::KSM), BalanceOf::<T>::saturated_from(1_000_000 * TWELVE_TEN as u128)),
				],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (currency_id, base_balance) in self.bancor_pools.iter() {

				let pool = BancorPool {
					currency_id: *currency_id,
					token_pool: Zero::zero(),
					vstoken_pool: Zero::zero(),
					token_base_supply: base_balance.saturating_mul(BalanceOf::<T>::from(2u32)),
					vstoken_base_supply: *base_balance,
				};

				BancorPools::<T>::insert(currency_id, pool);
			}
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// exchange vstoken for token
		#[pallet::weight(1_000)]
		pub fn exchange_for_token(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			vstoken_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			let vstoken_id =currency_id.to_vstoken().map_err(|_| Error::<T>::ConversionError)?;

			// Get exchanger's vstoken balance
			let vstoken_balance = T::MultiCurrenciesHandler::free_balance(vstoken_id, &exchanger);
			ensure!(vstoken_balance >= vstoken_amount, Error::<T>::NotEnoughBalance);

			// make changes in the bancor pool
			let token_amount = Self::calculate_price_for_token(currency_id, vstoken_amount)?;
			BancorPools::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>>{
				match pool {
					Some(pool_info) => {
						ensure!(pool_info.token_pool >= token_amount, Error::<T>::TokenSupplyNotEnought);
						pool_info.token_pool = pool_info.token_pool.saturating_sub(token_amount);
						pool_info.vstoken_pool = pool_info.vstoken_pool.saturating_add(vstoken_amount);
						Ok(())
					},
					_ => Err(Error::<T>::BancorPoolNotExist)
				}
			})?;

			// make changes in account balance
			T::MultiCurrenciesHandler::withdraw(vstoken_id, &exchanger,  vstoken_amount)?;
			T::MultiCurrenciesHandler::deposit(currency_id, &exchanger, token_amount)?;
			
			Self::deposit_event(Event::TokenSold(exchanger, currency_id, token_amount, vstoken_amount));

			Ok(().into())
		}

		// exchange token for vstoken
		#[pallet::weight(1_000)]
		pub fn exchange_for_vstoken(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			token_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			let vstoken_id =currency_id.to_vstoken().map_err(|_| Error::<T>::ConversionError)?;

			// Get exchanger's token balance
			let token_balance = T::MultiCurrenciesHandler::free_balance(currency_id, &exchanger);
			ensure!(token_balance >= token_amount, Error::<T>::NotEnoughBalance);

			// make changes in the bancor pool
			let vstoken_amount = Self::calculate_price_for_vstoken(currency_id, token_amount)?;

			BancorPools::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>>{
				match pool {
					Some(pool_info) => {
						ensure!(pool_info.vstoken_pool >= vstoken_amount, Error::<T>::VSTokenSupplyNotEnought);
						pool_info.token_pool = pool_info.token_pool.saturating_add(token_amount);
						pool_info.vstoken_pool = pool_info.vstoken_pool.saturating_sub(vstoken_amount);
						Ok(())
					},
					_ => Err(Error::<T>::BancorPoolNotExist)
				}
			})?;

			// make changes in account balance
			T::MultiCurrenciesHandler::withdraw(currency_id, &exchanger, token_amount)?;
			T::MultiCurrenciesHandler::deposit(vstoken_id, &exchanger,  vstoken_amount)?;
			
			Self::deposit_event(Event::VSTokenSold(exchanger, currency_id, vstoken_amount, token_amount));

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Formula: Supply * ((1 + vsDOT/Balance) ^CW -1)
	/// Supply: The total amount of DOT currently Sent in plus initiated virtual amount of DOT
	/// Balance: The total amount of vsDOT currently Sent in plus initiated virtual amount of vsDOT
	/// CW: Constant, here is 1/2
	pub fn calculate_price_for_token(token_id: CurrencyId, vstoken_amount: BalanceOf<T>) -> Result<BalanceOf<T>, Error<T>> {
		// ensure!(token_id.exist(), Error::<T>::CurrencyIdNotExist);
		ensure!(vstoken_amount > Zero::zero(), Error::<T>::AmountNotGreaterThanZero);

		let pool_info = Self::get_bancor_pool(token_id).ok_or(Error::<T>::BancorPoolNotExist)?;
		ensure!(pool_info.token_pool > Zero::zero(), Error::<T>::TokenSupplyNotEnought);

		let (token_supply, vstoken_supply) = (pool_info.token_base_supply + pool_info.token_pool, pool_info.vstoken_base_supply + pool_info.vstoken_pool);
		// According to the formula, we can exchange for no more than the number of supply units out(in the case of token_base_supply is zero), which means ((1 + vsDOT/Balance) ^CW -1) should be less than or equal to 1.
		ensure!(vstoken_amount <= BalanceOf::<T>::saturated_from(3u128).saturating_mul(vstoken_supply), Error::<T>::VSTokenSupplyNotEnought);

		let token_supply_squre = token_supply.saturating_mul(token_supply);

		let lhs: u128 = (vstoken_amount.saturating_mul(token_supply_squre)/ vstoken_supply).saturating_add(token_supply_squre).saturated_into();
		let result = lhs.nth_root(2).saturating_sub(token_supply.saturated_into());
		let price = BalanceOf::<T>::saturated_from(result);

		// We can not exchage for more than that the the pool has
		ensure!(price <= pool_info.token_pool, Error::<T>::TokenSupplyNotEnought);

		Ok(price)
	}

	/// Formula: Balance * (1 - (1 - DOT/Supply)^ (1/CW))
	/// Supply: The total amount of DOT currently Sent in plus initiated virtual amount of DOT
	/// Balance: The total amount of vsDOT currently Sent in plus initiated virtual amount of vsDOT
	/// CW: Constant, here is 1/2
	pub fn calculate_price_for_vstoken(token_id: CurrencyId, token_amount: BalanceOf<T>) -> Result<BalanceOf<T>, Error<T>> {
		// ensure!(token_id.exist(), Error::<T>::CurrencyIdNotExist);
		ensure!(token_amount > Zero::zero(), Error::<T>::AmountNotGreaterThanZero);

		let pool_info = Self::get_bancor_pool(token_id).ok_or(Error::<T>::BancorPoolNotExist)?;
		ensure!(pool_info.vstoken_pool > Zero::zero(), Error::<T>::VSTokenSupplyNotEnought);

		let (token_supply, vstoken_supply) = (pool_info.token_base_supply + pool_info.token_pool, pool_info.vstoken_base_supply + pool_info.vstoken_pool);
		
		// According to the formula, we can exchange for no more than the number of balance units out(in the case of vstoken_base_supply is zero), which means (1 - (1 - DOT/Supply)^ (1/CW)) should be less than or equal to 1.
		ensure!(token_amount <= BalanceOf::<T>::saturated_from(2u128).saturating_mul(token_supply), Error::<T>::TokenSupplyNotEnought);

		let item = {
			if token_supply > token_amount {
				token_supply - token_amount
			} else {
				token_amount -token_supply
			}
		};

		let square_item = Permill::from_rational_approximation(item, token_supply).square();
		// Destruct the nominator from permill and divide the result by the denominator of a million.
		let rhs = Permill::one().saturating_sub(square_item);
		let rhs_nominator = BalanceOf::<T>::saturated_from(rhs.deconstruct());
		let price = rhs_nominator.saturating_mul(vstoken_supply) / BalanceOf::<T>::saturated_from(MILLION);

		// We can not exchage for more than that the the pool has
		ensure!(price <= pool_info.vstoken_pool, Error::<T>::VSTokenSupplyNotEnought);

		Ok(price)
	}
}

impl<T: Config> BancorHandler<BalanceOf<T>> for Pallet<T>{
	fn add_token(currency_id: CurrencyId, amount: BalanceOf<T>) -> Result<(), DispatchError> {
		BancorPools::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>>{
			match pool {
				Some(pool_info) => {
					pool_info.token_pool = pool_info.token_pool.saturating_add(amount);
					Ok(())
				},
				_ => Err(Error::<T>::BancorPoolNotExist)
			}
		})?;

		Ok(())
	}
}



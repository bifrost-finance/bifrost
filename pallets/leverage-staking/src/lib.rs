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
#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

use bifrost_primitives::{CurrencyIdConversion, Rate, VtokenMintingInterface};
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungibles::Mutate,
		tokens::{Fortitude, Precision, Preservation},
	},
	transactional, BoundedVec,
};
use frame_system::{ensure_signed, pallet_prelude::*};
pub use pallet_traits::{
	ConvertToBigUint, LendMarket as LendMarketTrait, LendMarketMarketDataProvider,
	LendMarketPositionDataProvider, MarketInfo, MarketStatus,
};
pub use parity_scale_codec::{Decode, Encode};
use sp_runtime::{
	traits::{CheckedSub, Zero},
	ArithmeticError, FixedPointNumber, FixedU128, SaturatedConversion,
};
use sp_std::{cmp::Ordering, marker::PhantomData};
pub use weights::WeightInfo;

use bifrost_stable_pool::traits::StablePoolHandler;
use lend_market::{AccountDeposits, AccountIdOf, AssetIdOf, BalanceOf};
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config + lend_market::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type WeightInfo: WeightInfo;

		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type VtokenMinting: VtokenMintingInterface<
			AccountIdOf<Self>,
			AssetIdOf<Self>,
			BalanceOf<Self>,
		>;

		type LendMarket: LendMarketTrait<AssetIdOf<Self>, AccountIdOf<Self>, BalanceOf<Self>>;

		type StablePoolHandler: StablePoolHandler<
			Balance = BalanceOf<Self>,
			AccountId = AccountIdOf<Self>,
			CurrencyId = AssetIdOf<Self>,
		>;

		type CurrencyIdConversion: CurrencyIdConversion<AssetIdOf<Self>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Arguments error, old rate is equal to new rate
		ArgumentsError,
		/// Not support token type
		NotSupportTokenType,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// User's leverage rate has been changed.
		FlashLoanDeposited {
			/// Account who change the leverage rate.
			who: AccountIdOf<T>,
			/// The asset id of the token.
			asset_id: AssetIdOf<T>,
			/// The old leverage rate.
			old_rate: Rate,
			/// The new leverage rate.
			new_rate: Rate,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Deposit flash loan
		///
		/// Using borrowed funds to increase the amount of liquid staking (yield-bearing) assets.
		///
		/// - `asset_id`: The asset id of the token
		/// - `rate`: Leverage rate
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::flash_loan_deposit())]
		pub fn flash_loan_deposit(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			rate: Rate,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Pallet::<T>::flash_loan_deposit_inner(who, asset_id, rate)
		}
	}
}

impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn flash_loan_deposit_inner(
		who: AccountIdOf<T>,
		asset_id: AssetIdOf<T>,
		rate: Rate,
	) -> DispatchResult {
		let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(asset_id)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;

		let deposits = AccountDeposits::<T>::get(vtoken_id, &who);
		if !deposits.is_collateral {
			T::LendMarket::do_collateral_asset(&who, vtoken_id, true)?;
		}
		let account_deposits = Self::current_collateral_amount(&who, vtoken_id)?;
		let account_borrows = lend_market::Pallet::<T>::get_current_borrow_balance(&who, asset_id)?;

		// Formula
		// current_rate = account_borrows / (
		// get_currency_amount_by_v_currency_amount(account_deposits) - account_borrows )
		let deposits_token_value = T::VtokenMinting::get_currency_amount_by_v_currency_amount(
			asset_id,
			vtoken_id,
			account_deposits,
		)?;
		let base_token_value = deposits_token_value
			.checked_sub(account_borrows)
			.ok_or(ArithmeticError::Overflow)?;
		let current_rate = FixedU128::saturating_from_rational(account_borrows, base_token_value);

		match rate.cmp(&current_rate) {
			Ordering::Less => {
				let reduce_amount = if rate.is_zero() {
					account_borrows
				} else {
					current_rate
						.checked_sub(&rate)
						.and_then(|r| r.checked_mul_int(base_token_value))
						.ok_or(ArithmeticError::Overflow)?
				};
				Self::reduce_leverage(&who, asset_id, vtoken_id, reduce_amount)?;
			},
			Ordering::Equal => return Err(Error::<T>::ArgumentsError.into()),
			Ordering::Greater => {
				let increase_amount = rate
					.checked_sub(&current_rate)
					.and_then(|r| r.checked_mul_int(base_token_value))
					.ok_or(ArithmeticError::Overflow)?;
				Self::increase_leverage(&who, asset_id, vtoken_id, increase_amount)?;
			},
		}
		Self::deposit_event(Event::<T>::FlashLoanDeposited {
			who,
			asset_id,
			old_rate: current_rate,
			new_rate: rate,
		});
		Ok(())
	}

	fn reduce_leverage(
		who: &T::AccountId,
		asset_id: AssetIdOf<T>,
		vtoken_id: AssetIdOf<T>,
		reduce_amount: BalanceOf<T>,
	) -> DispatchResult {
		let (pool_id, currency_id_in, currency_id_out) =
			T::StablePoolHandler::get_pool_id(&vtoken_id, &asset_id)
				.ok_or(Error::<T>::NotSupportTokenType)?;

		<T as lend_market::Config>::Assets::mint_into(asset_id, &who, reduce_amount)?;

		T::LendMarket::do_repay_borrow(&who, asset_id, reduce_amount)?;
		let redeem_amount = T::StablePoolHandler::get_swap_input(
			pool_id,
			currency_id_in,
			currency_id_out,
			reduce_amount,
		)?;
		// Do redeem
		T::LendMarket::do_redeem(&who, vtoken_id, redeem_amount)?;

		T::StablePoolHandler::swap(
			&who,
			pool_id,
			currency_id_in,
			currency_id_out,
			redeem_amount,
			reduce_amount,
		)?;
		<T as lend_market::Config>::Assets::burn_from(
			asset_id,
			&who,
			reduce_amount,
			Preservation::Protect,
			Precision::Exact,
			Fortitude::Force,
		)?;
		Ok(())
	}

	fn increase_leverage(
		who: &T::AccountId,
		asset_id: AssetIdOf<T>,
		vtoken_id: AssetIdOf<T>,
		increase_amount: BalanceOf<T>,
	) -> DispatchResult {
		<T as lend_market::Config>::Assets::mint_into(asset_id, &who, increase_amount)?;
		let vtoken_value = T::VtokenMinting::mint(
			who.clone(),
			asset_id,
			increase_amount,
			BoundedVec::default(),
			None,
		)?;
		T::LendMarket::do_mint(&who, vtoken_id, vtoken_value)?;
		T::LendMarket::do_borrow(&who, asset_id, increase_amount)?;
		<T as lend_market::Config>::Assets::burn_from(
			asset_id,
			&who,
			increase_amount,
			Preservation::Protect,
			Precision::Exact,
			Fortitude::Force,
		)?;
		Ok(())
	}

	fn current_collateral_amount(
		supplier: &T::AccountId,
		asset_id: AssetIdOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		if !AccountDeposits::<T>::contains_key(asset_id, supplier) {
			return Ok(BalanceOf::<T>::zero());
		}
		let deposits = AccountDeposits::<T>::get(asset_id, supplier);
		if deposits.voucher_balance.is_zero() {
			return Ok(BalanceOf::<T>::zero());
		}
		let exchange_rate = lend_market::Pallet::<T>::exchange_rate_stored(asset_id)?;
		let underlying_amount = lend_market::Pallet::<T>::calc_underlying_amount(
			deposits.voucher_balance,
			exchange_rate,
		)?;

		Ok(BalanceOf::<T>::saturated_from(underlying_amount))
	}
}

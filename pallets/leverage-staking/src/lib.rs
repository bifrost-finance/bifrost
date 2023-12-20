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
		fungibles::{Inspect, Mutate},
		tokens::{Fortitude, Precision},
	},
	BoundedVec,
};
use frame_system::{ensure_signed, pallet_prelude::*};
pub use pallet_traits::{
	ConvertToBigUint, LendMarket as LendMarketTrait, LendMarketMarketDataProvider,
	LendMarketPositionDataProvider, MarketInfo, MarketStatus, PriceFeeder,
};
pub use parity_scale_codec::{Decode, Encode};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Zero},
	ArithmeticError, FixedU128, Permill, RuntimeDebug,
};
use sp_std::marker::PhantomData;
pub use weights::WeightInfo;

use bifrost_stable_pool::traits::StablePoolHandler;
use lend_market::{AccountDeposits, AccountIdOf, AssetIdOf, BalanceOf, Markets};
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
		ArgumentsError,
		NotSupportTokenType,
		InsufficientBalance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		FlashLoanDeposited { who: AccountIdOf<T>, asset_id: AssetIdOf<T>, rate: Rate },
		FlashLoanRepaid { who: AccountIdOf<T>, asset_id: AssetIdOf<T> },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::flash_loan_deposit())]
		pub fn flash_loan_deposit(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			rate: Rate,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(asset_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			Self::do_repay(&who, asset_id)?;
			// Redeem all vouchers.
			let deposits = AccountDeposits::<T>::get(asset_id, &who);
			let exchange_rate = lend_market::Pallet::<T>::exchange_rate_stored(asset_id)?;
			let underlying_amount = lend_market::Pallet::<T>::calc_underlying_amount(
				deposits.voucher_balance,
				exchange_rate,
			)?;
			let _underlying_amount = lend_market::Pallet::<T>::do_redeem_all(&who, vtoken_id)?;

			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(asset_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;

			// let account_borrows = lend_market::Pallet::<T>::current_borrow_balance(&who,
			// asset_id)?; let underlying_amount = if !account_borrows.is_zero() {
			// 	Self::do_repay(&who, asset_id)?
			// } else {
			// 	// do_redeem_all
			// 	// let deposits = AccountDeposits::<T>::get(asset_id, &who);
			// 	lend_market::Pallet::<T>::do_redeem_all(&who, vtoken_id)?
			// };

			log::debug!("underlying_amount: {:?}", underlying_amount);
			if rate.is_zero() {
				return Ok(().into());
			}

			let free_balance = <T as lend_market::Config>::Assets::balance(asset_id, &who);
			ensure!(free_balance >= underlying_amount, Error::<T>::InsufficientBalance);

			let mut token_total_value = FixedU128::from_inner(underlying_amount)
				.checked_mul(&rate)
				.map(|r| r.into_inner())
				.ok_or(ArithmeticError::Underflow)?;

			let mut vtoken_total_amount: BalanceOf<T> = Zero::zero();
			if let Some(market) = Markets::<T>::get(asset_id) {
				let mut token_value = underlying_amount;
				while token_total_value > Zero::zero() {
					let vtoken_value = T::VtokenMinting::mint(
						who.clone(),
						asset_id,
						token_value,
						BoundedVec::default(),
					)?;
					T::LendMarket::do_mint(&who, vtoken_id, vtoken_value)?;
					let deposits = lend_market::Pallet::<T>::account_deposits(vtoken_id, &who);
					if !deposits.is_collateral {
						T::LendMarket::do_collateral_asset(&who, vtoken_id, true)?;
					}
					token_value = market.collateral_factor * token_value;
					token_value = match token_total_value < token_value {
						true => {
							vtoken_total_amount = vtoken_total_amount
								.checked_add(vtoken_value)
								.ok_or(ArithmeticError::Overflow)?;
							T::LendMarket::do_borrow(&who, asset_id, token_total_value)?;
							let vtoken_value = T::VtokenMinting::mint(
								who.clone(),
								asset_id,
								token_total_value,
								BoundedVec::default(),
							)?;
							T::LendMarket::do_mint(&who, vtoken_id, vtoken_value)?;
							let deposits =
								lend_market::Pallet::<T>::account_deposits(vtoken_id, &who);
							if !deposits.is_collateral {
								T::LendMarket::do_collateral_asset(&who, vtoken_id, true)?;
							}
							vtoken_total_amount = vtoken_total_amount
								.checked_add(vtoken_value)
								.ok_or(ArithmeticError::Overflow)?;
							token_total_value = Zero::zero();
							token_total_value
						},
						false => {
							vtoken_total_amount = vtoken_total_amount
								.checked_add(vtoken_value)
								.ok_or(ArithmeticError::Overflow)?;
							T::LendMarket::do_borrow(&who, asset_id, token_value)?;
							token_total_value = token_total_value.saturating_sub(token_value);
							token_value
						},
					};
				}
			}

			Self::deposit_event(Event::<T>::FlashLoanDeposited { who, asset_id, rate });
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn do_repay(
		who: &T::AccountId,
		asset_id: AssetIdOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(asset_id)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;

		let account_borrows = lend_market::Pallet::<T>::current_borrow_balance(who, asset_id)?;
		if account_borrows.is_zero() {
			return Ok(0);
		}

		let (pool_id, currency_id_in, currency_id_out) =
			T::StablePoolHandler::get_pool_id(&vtoken_id, &asset_id)
				.ok_or(Error::<T>::ArgumentsError)?;

		<T as lend_market::Config>::Assets::mint_into(asset_id, &who, account_borrows)?;

		T::LendMarket::do_repay_borrow(&who, asset_id, account_borrows)?;
		// Do redeem
		let deposits = AccountDeposits::<T>::get(asset_id, &who);
		let redeem_amount =
			lend_market::Pallet::<T>::do_redeem_voucher(&who, vtoken_id, deposits.voucher_balance)?;
		let exchange_rate = lend_market::Pallet::<T>::exchange_rate_stored(asset_id)?;
		let underlying_amount = lend_market::Pallet::<T>::calc_underlying_amount(
			deposits.voucher_balance,
			exchange_rate,
		)?;

		T::StablePoolHandler::swap(
			&who,
			pool_id,
			currency_id_in,
			currency_id_out,
			underlying_amount,
			underlying_amount,
		)?;
		<T as lend_market::Config>::Assets::burn_from(
			asset_id,
			&who,
			account_borrows,
			Precision::Exact,
			Fortitude::Force,
		)?;

		Self::deposit_event(Event::<T>::FlashLoanRepaid { who: who.clone(), asset_id });
		Ok(redeem_amount)
	}
}

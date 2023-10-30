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
#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

pub use codec::{Decode, Encode};
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungibles::{Inspect, Mutate},
		tokens::{Fortitude, Precision},
		Get,
	},
	BoundedVec, PalletId,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use node_primitives::{CurrencyIdConversion, CurrencyIdRegister, Rate, VtokenMintingInterface};
pub use pallet_traits::{
	ConvertToBigUint, LendMarket as LendMarketTrait, LendMarketMarketDataProvider,
	LendMarketPositionDataProvider, MarketInfo, MarketStatus, PriceFeeder,
};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, Zero},
	ArithmeticError, FixedU128, Permill, RuntimeDebug,
};
use sp_std::marker::PhantomData;
pub use weights::WeightInfo;

use bifrost_stable_pool::traits::StablePoolHandler;
use lend_market::{AccountIdOf, AssetIdOf, BalanceOf, Markets};
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

		type CurrencyIdRegister: CurrencyIdRegister<AssetIdOf<Self>>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;
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
		FlashLoanDeposited {
			who: AccountIdOf<T>,
			asset_id: AssetIdOf<T>,
			amount: BalanceOf<T>,
			rate: Rate,
		},
		FlashLoanRepaid {
			who: AccountIdOf<T>,
			asset_id: AssetIdOf<T>,
			rate: Rate,
		},
	}

	#[pallet::storage]
	pub type AccountFlashLoans<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetIdOf<T>,
		Blake2_128Concat,
		T::AccountId,
		AccountFlashLoanInfo<BalanceOf<T>>,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_price())]
		pub fn flash_loan_deposit(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			rate: Rate,
			maybe_input_value: Option<BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let free_balance = <T as lend_market::Config>::Assets::balance(asset_id, &who);
			let input_value =
				if let Some(flash_loan_info) = AccountFlashLoans::<T>::get(asset_id, &who) {
					Self::do_repay(&who, asset_id, None)?;
					let value = match maybe_input_value {
						Some(v) => {
							ensure!(free_balance >= v, Error::<T>::InsufficientBalance);
							v
						},
						None => {
							ensure!(
								free_balance >= flash_loan_info.amount,
								Error::<T>::InsufficientBalance
							);
							flash_loan_info.amount
						},
					};
					value
				} else {
					let value = maybe_input_value.ok_or(Error::<T>::ArgumentsError)?;
					ensure!(free_balance >= value, Error::<T>::InsufficientBalance);
					value
				};

			if rate.is_zero() {
				return Ok(().into());
			}

			let mut token_total_value = FixedU128::from_inner(input_value)
				.checked_mul(&rate)
				.map(|r| r.into_inner())
				.ok_or(ArithmeticError::Underflow)?;

			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(asset_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;

			let mut vtoken_total_amount: BalanceOf<T> = Zero::zero();
			if let Some(market) = Markets::<T>::get(asset_id) {
				let mut token_value = input_value;
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
				AccountFlashLoans::<T>::insert(
					asset_id,
					&who,
					AccountFlashLoanInfo {
						amount: input_value,
						leverage_rate: rate,
						vtoken_amount: vtoken_total_amount,
						collateral_factor: market.collateral_factor,
					},
				);
			}

			Self::deposit_event(Event::<T>::FlashLoanDeposited {
				who,
				asset_id,
				rate,
				amount: input_value,
			});
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn do_repay(
		who: &T::AccountId,
		asset_id: AssetIdOf<T>,
		maybe_rate: Option<Rate>,
	) -> DispatchResult {
		let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(asset_id)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;

		AccountFlashLoans::<T>::try_mutate_exists(
			asset_id,
			&who,
			|maybe_flash_loan_info| -> DispatchResult {
				let flash_loan_info =
					maybe_flash_loan_info.as_mut().ok_or(Error::<T>::ArgumentsError)?;
				let rate = match maybe_rate {
					Some(r) => {
						ensure!(flash_loan_info.leverage_rate >= r, Error::<T>::ArgumentsError);
						r
					},
					None => flash_loan_info.leverage_rate,
				};

				let token_value = FixedU128::from_inner(flash_loan_info.amount)
					.checked_mul(&rate)
					.map(|r| r.into_inner())
					.ok_or(ArithmeticError::Underflow)?;
				let (pool_id, currency_id_in, currency_id_out) =
					T::StablePoolHandler::get_pool_id(&vtoken_id, &asset_id)
						.ok_or(Error::<T>::ArgumentsError)?;

				<T as lend_market::Config>::Assets::mint_into(asset_id, &who, token_value)?;
				let total_rate = flash_loan_info
					.leverage_rate
					.checked_add(&FixedU128::from_inner(10_u128.pow(18)))
					.ok_or(ArithmeticError::Underflow)?;
				let vtoken_value = FixedU128::from_inner(flash_loan_info.vtoken_amount)
					.checked_mul(&rate)
					.and_then(|r: FixedU128| r.checked_div(&total_rate))
					.map(|r: FixedU128| r.into_inner())
					.ok_or(ArithmeticError::Underflow)?;

				T::LendMarket::do_repay_borrow(&who, asset_id, token_value)?;
				T::LendMarket::do_redeem(&who, vtoken_id, vtoken_value)?;

				T::StablePoolHandler::swap(
					&who,
					pool_id,
					currency_id_in,
					currency_id_out,
					vtoken_value,
					vtoken_value,
				)?;
				<T as lend_market::Config>::Assets::burn_from(
					asset_id,
					&who,
					token_value,
					Precision::Exact,
					Fortitude::Force,
				)?;
				flash_loan_info.leverage_rate = flash_loan_info
					.leverage_rate
					.checked_sub(&rate)
					.ok_or(ArithmeticError::Underflow)?;

				flash_loan_info.vtoken_amount = flash_loan_info
					.vtoken_amount
					.checked_sub(vtoken_value)
					.ok_or(ArithmeticError::Underflow)?;

				if flash_loan_info.leverage_rate.is_zero() {
					*maybe_flash_loan_info = None;
				}

				Self::deposit_event(Event::<T>::FlashLoanRepaid {
					who: who.clone(),
					asset_id,
					rate,
				});
				Ok(())
			},
		)
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct AccountFlashLoanInfo<Balance> {
	amount: Balance,
	vtoken_amount: Balance,
	leverage_rate: Rate,
	collateral_factor: Permill,
}

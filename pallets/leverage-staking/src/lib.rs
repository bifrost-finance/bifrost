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
	BoundedVec, PalletId, Parameter,
};
use frame_system::{ensure_signed, pallet_prelude::*};
use node_primitives::{
	CurrencyId, CurrencyIdConversion, CurrencyIdExt, CurrencyIdRegister, Rate, TimeUnit,
	VtokenMintingInterface,
};
use orml_traits::MultiCurrency;
pub use pallet_traits::{
	ConvertToBigUint, LendMarket as LendMarketTrait, LendMarketMarketDataProvider,
	LendMarketPositionDataProvider, MarketInfo, MarketStatus, PriceFeeder,
};
use sp_runtime::{
	traits::{CheckedMul, CheckedSub, StaticLookup},
	ArithmeticError, FixedU128, RuntimeDebug,
};
use sp_std::marker::PhantomData;
pub use weights::WeightInfo;

use bifrost_stable_pool::traits::StablePoolHandler;
use lend_market::{AccountIdOf, AssetIdOf, BalanceOf, InterestRateModel};
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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		FlashLoanDeposited { asset_id: AssetIdOf<T>, rate: Rate, input_value: BalanceOf<T> },
	}

	#[pallet::storage]
	pub type AccountFlashLoans<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetIdOf<T>,
		Blake2_128Concat,
		T::AccountId,
		AccountFlashLoanInfo<BalanceOf<T>>,
		// ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_price())]
		pub fn flash_loan_deposit(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			rate: Rate,
			input_value: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let token_value = FixedU128::from_inner(input_value)
				.checked_mul(&rate)
				.map(|r| r.into_inner())
				.ok_or(ArithmeticError::Underflow)?;

			let additional_issuance_token_value =
				token_value.checked_sub(input_value).ok_or(ArithmeticError::Underflow)?;

			<T as lend_market::Config>::Assets::mint_into(
				asset_id,
				&who,
				additional_issuance_token_value,
			)?;
			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(asset_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let vtoken_value =
				T::VtokenMinting::mint(who.clone(), asset_id, token_value, BoundedVec::default())?;
			T::LendMarket::do_mint(&who, vtoken_id, vtoken_value)?;
			let deposits = lend_market::Pallet::<T>::account_deposits(vtoken_id, &who);
			if deposits.is_collateral == false {
				T::LendMarket::do_collateral_asset(&who, vtoken_id, true)?;
			}
			T::LendMarket::do_borrow(&who, asset_id, additional_issuance_token_value)?;
			<T as lend_market::Config>::Assets::burn_from(
				asset_id,
				&who,
				additional_issuance_token_value,
				Precision::Exact,
				Fortitude::Force,
			)?;
			log::debug!("flash_loan_deposit: additional_issuance_token_value: {:?}, rate: {:?}, input_value: {:?}, token_value: {:?}, vtoken_id: {:?}, vtoken_value: {:?}", 
			additional_issuance_token_value, rate, input_value, token_value, vtoken_id, vtoken_value);

			AccountFlashLoans::<T>::insert(
				asset_id,
				&who,
				AccountFlashLoanInfo { amount: input_value, leverage_rate: rate },
			);
			Self::deposit_event(Event::<T>::FlashLoanDeposited { asset_id, rate, input_value });
			Ok(().into())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_price())]
		pub fn flash_loan_repay(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			rate: Rate,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(asset_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;

			log::debug!(
				"flash_loan_repay: vtoken_id: {:?}, asset_id: {:?}, rate: {:?}",
				vtoken_id,
				asset_id,
				rate
			);
			// let flash_loan_info = AccountFlashLoans::<T>::get(asset_id, &who);
			AccountFlashLoans::<T>::mutate(
				asset_id,
				&who,
				|maybe_flash_loan_info| -> DispatchResult {
					let flash_loan_info =
						maybe_flash_loan_info.as_mut().ok_or(Error::<T>::ArgumentsError)?;
					ensure!(flash_loan_info.leverage_rate >= rate, Error::<T>::ArgumentsError);

					let token_value = FixedU128::from_inner(flash_loan_info.amount)
						.checked_mul(&rate)
						.map(|r| r.into_inner())
						.ok_or(ArithmeticError::Underflow)?;
					// TODO: get VDOT amount from stable-pool through token_value
					let (pool_id, currency_id_in, currency_id_out) =
						T::StablePoolHandler::get_pool_id(&vtoken_id, &asset_id)
							.ok_or(Error::<T>::ArgumentsError)?;
					let vtoken_value = T::StablePoolHandler::get_swap_input(
						pool_id,
						currency_id_in,
						currency_id_out,
						token_value,
					)?;
					<T as lend_market::Config>::Assets::mint_into(vtoken_id, &who, vtoken_value)?;
					T::StablePoolHandler::swap(
						&who,
						pool_id,
						currency_id_in,
						currency_id_out,
						vtoken_value,
						token_value,
					)?;
					log::debug!("flash_loan_repay: token_value: {:?}, vtoken_value: {:?}, vtoken_id: {:?}, asset_id: {:?}, rate: {:?}, flash_loan_info: {:?}",
					token_value, vtoken_value, vtoken_id, asset_id, rate, flash_loan_info);
					T::LendMarket::do_repay_borrow(&who, asset_id, token_value)?;
					T::LendMarket::do_redeem(&who, vtoken_id, vtoken_value)?;
					<T as lend_market::Config>::Assets::burn_from(
						vtoken_id,
						&who,
						vtoken_value,
						Precision::Exact,
						Fortitude::Force,
					)?;

					flash_loan_info.leverage_rate = flash_loan_info
						.leverage_rate
						.checked_sub(&rate)
						.ok_or(ArithmeticError::Underflow)?;
					Ok(())
				},
			)?;
			Ok(().into())
		}
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct AccountFlashLoanInfo<Balance> {
	amount: Balance,
	leverage_rate: Rate,
}

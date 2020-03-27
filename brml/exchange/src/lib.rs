// Copyright 2019-2020 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.
#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use frame_support::{Parameter, decl_event, decl_error, decl_module, decl_storage, ensure, IterableStorageMap};
use frame_system::{self as system, ensure_root, ensure_signed};
use node_primitives::{AssetTrait, FetchExchangeRate, TokenType};
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, Zero};

pub trait Trait: frame_system::Trait {
	/// exchange rate
	type ExchangeRate: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance>;
	type RatePerBlock: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance> + Into<Self::ExchangeRate>;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy;

	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy;

	/// The units in which we record costs.
	type Cost: Member + Parameter + AtLeast32Bit + Default + Copy;

	/// The units in which we record incomes.
	type Income: Member + Parameter + AtLeast32Bit + Default + Copy;

	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance, Self::Cost, Self::Income>;

	/// event
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event! {
	pub enum Event {
		UpdateExchangeSuccess,
		UpdatezRatePerBlockSuccess,
		ExchangeTokenToVTokenSuccess,
		ExchangerVTokenToTokenSuccess,
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Asset id doesn't exist
		TokenNotExist,
		/// Amount of input should be less than or equal to origin balance
		InvalidBalanceForTransaction,
		/// Exchange rate doesn't be set
		ExchangeRateDoesNotSet,
		/// This is an invalid exchange rate
		InvalidExchangeRate,
		/// Vtoken id is not equal to token id
		InvalidTokenPair,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Exchange {
		/// exchange rate between two tokens, vtoken => (token, exchange_rate)
		ExchangeRate get(fn exchange_rate): map hasher(blake2_128_concat) T::AssetId => T::ExchangeRate;
		/// change rate per block, vtoken => (token, rate_per_block)
		RatePerBlock get(fn rate_per_block): map hasher(blake2_128_concat) T::AssetId => T::RatePerBlock;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		fn set_exchange_rate(
			origin,
			vtoken_id: T::AssetId,
			exchange_rate: T::ExchangeRate
		) {
			ensure_root(origin)?;

			ensure!(T::AssetTrait::token_exists(vtoken_id), Error::<T>::TokenNotExist);
			<ExchangeRate<T>>::insert(vtoken_id, exchange_rate);

			Self::deposit_event(Event::UpdateExchangeSuccess);
		}

		fn set_rate_per_block(
			origin,
			vtoken_id: T::AssetId,
			rate_per_block: T::RatePerBlock
		) {
			ensure_root(origin)?;

			ensure!(T::AssetTrait::token_exists(vtoken_id), Error::<T>::TokenNotExist);
			<RatePerBlock<T>>::insert(vtoken_id, rate_per_block);

			Self::deposit_event(Event::UpdatezRatePerBlockSuccess);
		}

		fn exchange_token_to_vtoken(
			origin,
			#[compact] token_amount: T::Balance,
			vtoken_id: T::AssetId
		) {
			let exchanger = ensure_signed(origin)?;

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(vtoken_id), Error::<T>::TokenNotExist);

			let token_id = vtoken_id; // token id is equal to vtoken id
			let token_balances = T::AssetTrait::get_account_asset(&token_id, TokenType::Token, &exchanger).balance;
			ensure!(token_balances >= token_amount, Error::<T>::InvalidBalanceForTransaction);

			// check exchange rate has been set
			ensure!(<ExchangeRate<T>>::contains_key(vtoken_id), Error::<T>::ExchangeRateDoesNotSet);

			let rate = <ExchangeRate<T>>::get(vtoken_id);

			ensure!(!rate.is_zero(), Error::<T>::InvalidExchangeRate);
			let vtokens_buy = token_amount.saturating_mul(rate.into());

			// transfer
			T::AssetTrait::asset_destroy(token_id, TokenType::Token, exchanger.clone(), token_amount);
			T::AssetTrait::asset_issue(vtoken_id, TokenType::VToken, exchanger, vtokens_buy);

			Self::deposit_event(Event::ExchangeTokenToVTokenSuccess);
		}

		fn exchange_vtoken_to_token(
			origin,
			#[compact] vtoken_amount: T::Balance,
			vtoken_id: T::AssetId,
		) {
			let exchanger = ensure_signed(origin)?;

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(vtoken_id), Error::<T>::TokenNotExist);

			let vtoken_balances = T::AssetTrait::get_account_asset(&vtoken_id, TokenType::VToken, &exchanger).balance;
			ensure!(vtoken_balances >= vtoken_amount, Error::<T>::InvalidBalanceForTransaction);

			// check exchange rate has been set
			ensure!(<ExchangeRate<T>>::contains_key(vtoken_id), Error::<T>::ExchangeRateDoesNotSet);

			let token_id = vtoken_id; // token id is equal to vtoken id
			let rate = <ExchangeRate<T>>::get(vtoken_id);

			ensure!(!rate.is_zero(), Error::<T>::InvalidExchangeRate);
			let tokens_buy = vtoken_amount / rate.into();

			T::AssetTrait::asset_destroy(vtoken_id, TokenType::VToken, exchanger.clone(), vtoken_amount);
			T::AssetTrait::asset_issue(token_id, TokenType::Token, exchanger, tokens_buy);

			Self::deposit_event(Event::ExchangerVTokenToTokenSuccess);
		}

		fn on_finalize() {
			for (vtoken_id, rate_per_block) in <RatePerBlock<T>>::iter() {
				if !<ExchangeRate<T>>::contains_key(vtoken_id) {
					continue;
				}
				<ExchangeRate<T>>::mutate(vtoken_id, |exchange_rate| {
					*exchange_rate = exchange_rate.saturating_sub(rate_per_block.into());
				});
			}

//			let vtoken_balances = T::AssetTrait::get_account_asset(&vtoken_id, TokenType::VToken, &exchanger).balance;
//			let benefit = 5;
//			let epoch = 2000;
//
//			let curr_rate = <ExchangeRate<T>>::get(vtoken_id);
//			let epoch_rate = (1 + benefit / vtoken_balances) * curr_rate;
//
//			let curr_blk_num = system::Module::<T>::block_number();
//			let specified_exchange_rate = {
//				curr_rate + ((epoch_rate - curr_rate) / epoch) * (curr_blk_num % epoch)
//			};
		}
	}
}

impl<T: Trait> Module<T> {
	pub fn get_exchange(vtoken_id: T::AssetId) -> T::ExchangeRate {
		let rate = <ExchangeRate<T>>::get(vtoken_id);

		rate
	}
}

impl<T: Trait> FetchExchangeRate<T::AssetId, T::ExchangeRate> for Module<T> {
	fn fetch_exchange_rate(asset_id: T::AssetId) -> T::ExchangeRate {
		let rate = <ExchangeRate<T>>::get(asset_id);

		rate
	}
}

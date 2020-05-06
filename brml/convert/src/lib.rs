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

extern crate alloc;
use alloc::vec::Vec;

mod mock;
mod tests;

use frame_support::traits::Get;
use frame_support::{Parameter, decl_event, decl_error, decl_module, decl_storage, ensure, IterableStorageMap};
use frame_system::{self as system, ensure_root, ensure_signed};
use node_primitives::{AssetTrait, AssetSymbol, ConvertPool, FetchConvertPrice, AssetReward, TokenType};
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, Zero};

pub trait Trait: frame_system::Trait {
	/// convert rate
	type ConvertPrice: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance>;
	type RatePerBlock: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance> + Into<Self::ConvertPrice>;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy;

	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + From<Self::BlockNumber> + Into<Self::ConvertPrice>;

	/// The units in which we record costs.
	type Cost: Member + Parameter + AtLeast32Bit + Default + Copy;

	/// The units in which we record incomes.
	type Income: Member + Parameter + AtLeast32Bit + Default + Copy;

	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance, Self::Cost, Self::Income>;

	/// event
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

	type ConvertDuration: Get<Self::BlockNumber>;
}

decl_event! {
	pub enum Event {
		UpdateConvertSuccess,
		UpdatezRatePerBlockSuccess,
		ConvertTokenToVTokenSuccess,
		ConvertVTokenToTokenSuccess,
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Asset id doesn't exist
		TokenNotExist,
		/// Amount of input should be less than or equal to origin balance
		InvalidBalanceForTransaction,
		/// Convert rate doesn't be set
		ConvertPriceDoesNotSet,
		/// This is an invalid convert rate
		InvalidConvertPrice,
		/// Vtoken id is not equal to token id
		InvalidTokenPair,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Convert {
		/// convert rate between two tokens, vtoken => (token, convert_rate)
		ConvertPrice get(fn convert_rate): map hasher(blake2_128_concat) T::AssetId => T::ConvertPrice;
		/// change rate per block, vtoken => (token, rate_per_block)
		RatePerBlock get(fn rate_per_block): map hasher(blake2_128_concat) T::AssetId => T::RatePerBlock;
		/// collect referrer, converter => ([(referrer1, 1000), (referrer2, 2000), ...], total_point)
		/// total_point = 1000 + 2000 + ...
		/// referrer must be unique, so check it unique while a new referrer incoming
		ReferrerChannels get(fn referrer_channels): map hasher(blake2_128_concat) T::AccountId =>
			(Vec<(T::AccountId, T::Balance)>, T::Balance);
		/// Convert pool
		Pool get(fn pool): map hasher(blake2_128_concat) T::AssetId => ConvertPool<T::Balance>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const ConvertDuration: T::BlockNumber = T::ConvertDuration::get();

		fn deposit_event() = default;

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn set_convert_rate(
			origin,
			token_symbol: AssetSymbol,
			convert_rate: T::ConvertPrice
		) {
			ensure_root(origin)?;

			let token_id = T::AssetId::from(token_symbol as u32);

			ensure!(T::AssetTrait::token_exists(token_id), Error::<T>::TokenNotExist);
			<ConvertPrice<T>>::insert(token_id, convert_rate);

			Self::deposit_event(Event::UpdateConvertSuccess);
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn set_rate_per_block(
			origin,
			token_symbol: AssetSymbol,
			rate_per_block: T::RatePerBlock
		) {
			ensure_root(origin)?;

			let token_id = T::AssetId::from(token_symbol as u32);

			ensure!(T::AssetTrait::token_exists(token_id), Error::<T>::TokenNotExist);
			<RatePerBlock<T>>::insert(token_id, rate_per_block);

			Self::deposit_event(Event::UpdatezRatePerBlockSuccess);
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn convert_token_to_vtoken(
			origin,
			#[compact] token_amount: T::Balance,
			token_symbol: AssetSymbol,
			referrer: Option<T::AccountId>
		) {
			let converter = ensure_signed(origin)?;

			let token_id = T::AssetId::from(token_symbol as u32);

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(token_id), Error::<T>::TokenNotExist);

			let token_balances = T::AssetTrait::get_account_asset(&token_id, TokenType::Token, &converter).balance;
			ensure!(token_balances >= token_amount, Error::<T>::InvalidBalanceForTransaction);

			// check convert rate has been set
			ensure!(<ConvertPrice<T>>::contains_key(token_id), Error::<T>::ConvertPriceDoesNotSet);

			let rate = <ConvertPrice<T>>::get(token_id);

			ensure!(!rate.is_zero(), Error::<T>::InvalidConvertPrice);
			let vtokens_buy = token_amount.saturating_mul(rate.into());

			// transfer
			T::AssetTrait::asset_destroy(token_id, TokenType::Token, converter.clone(), token_amount);
			T::AssetTrait::asset_issue(token_id, TokenType::VToken, converter, vtokens_buy);

			Self::increase_pool(token_id, token_amount, vtokens_buy);

			// save
//			if let referrer = Some(referrer) {
//				// first time to referrer
//				if !<ReferrerChannels<T>>::contains_key(&converter) {
//					let value = (vec![(referrer, vtokens_buy)], vtokens_buy);
//					<ReferrerChannels<T>>::insert(&converter, value);
//				} else {
//					// existed, but new referrer incoming
//					<ReferrerChannels<T>>::mutate(&converter, |points| {
//						if points.0.iter().any(|point| point.0 == referrer) {
//							point.0[1] += vtokens_buy;
//						} else {
//							let value = (vec![(referrer, vtokens_buy)], vtokens_buy);
//							<ReferrerChannels<T>>::insert(&converter, value);
//						}
////						points.0.iter().find(|point| )
//						points.1 += vtokens_buy;
//					});
//				}
//			} else {
//				();
//			}

			Self::deposit_event(Event::ConvertTokenToVTokenSuccess);
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn convert_vtoken_to_token(
			origin,
			#[compact] vtoken_amount: T::Balance,
			token_symbol: AssetSymbol,
		) {
			let converter = ensure_signed(origin)?;

			let token_id = T::AssetId::from(token_symbol as u32);

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(token_id), Error::<T>::TokenNotExist);

			let vtoken_balances = T::AssetTrait::get_account_asset(&token_id, TokenType::VToken, &converter).balance;
			ensure!(vtoken_balances >= vtoken_amount, Error::<T>::InvalidBalanceForTransaction);

			// check convert rate has been set
			ensure!(<ConvertPrice<T>>::contains_key(token_id), Error::<T>::ConvertPriceDoesNotSet);

			let rate = <ConvertPrice<T>>::get(token_id);

			ensure!(!rate.is_zero(), Error::<T>::InvalidConvertPrice);
			let tokens_buy = vtoken_amount / rate.into();

			T::AssetTrait::asset_destroy(token_id, TokenType::VToken, converter.clone(), vtoken_amount);
			T::AssetTrait::asset_issue(token_id, TokenType::Token, converter, tokens_buy);

			Self::decrease_pool(token_id, tokens_buy, vtoken_amount);

			Self::deposit_event(Event::ConvertVTokenToTokenSuccess);
		}

		fn on_finalize(block_number: T::BlockNumber) {
			// calculate & update convert rate
			for (token_id, convert_pool) in <Pool<T>>::iter() {
				<Pool<T>>::mutate(token_id, |convert_pool| {
					let current_reward = convert_pool.current_reward;
					let reward_per_block = current_reward / T::ConvertDuration::get().into();
					convert_pool.token_pool = convert_pool.token_pool.saturating_add(reward_per_block);

					if convert_pool.token_pool != Zero::zero()
						&& convert_pool.vtoken_pool != Zero::zero()
					{
						if <ConvertPrice<T>>::contains_key(token_id) {
							<ConvertPrice<T>>::mutate(token_id, |convert_rate| {
								*convert_rate = (convert_pool.token_pool / convert_pool.vtoken_pool).into();
							});
						}
					}
				});
			}

			if block_number % T::ConvertDuration::get() == Zero::zero() {
				// new convert round
				for (token_id, _convert_pool) in <Pool<T>>::iter() {
					<Pool<T>>::mutate(token_id, |convert_pool| {
						convert_pool.new_round();
					});
				}
			}

//			let vtoken_balances = T::AssetTrait::get_account_asset(&vtoken_id, TokenType::VToken, &converter).balance;
//			let benefit = 5;
//			let epoch = 2000;
//
//			let curr_rate = <ConvertPrice<T>>::get(vtoken_id);
//			let epoch_rate = (1 + benefit / vtoken_balances) * curr_rate;
//
//			let curr_blk_num = system::Module::<T>::block_number();
//			let specified_convert_rate = {
//				curr_rate + ((epoch_rate - curr_rate) / epoch) * (curr_blk_num % epoch)
//			};
		}
	}
}

impl<T: Trait> Module<T> {
	pub fn get_convert(token_id: T::AssetId) -> T::ConvertPrice {
		<ConvertPrice<T>>::get(token_id)
	}

	fn increase_pool(token_id: T::AssetId, token_amount: T::Balance, vtoken_amount: T::Balance) {
		<Pool<T>>::mutate(token_id, |pool| {
			pool.token_pool = pool.token_pool.saturating_add(token_amount);
			pool.vtoken_pool = pool.vtoken_pool.saturating_add(vtoken_amount);
		});
	}

	fn decrease_pool(token_id: T::AssetId, token_amount: T::Balance, vtoken_amount: T::Balance) {
		<Pool<T>>::mutate(token_id, |pool| {
			pool.token_pool = pool.token_pool.saturating_sub(token_amount);
			pool.vtoken_pool = pool.vtoken_pool.saturating_sub(vtoken_amount);
		});
	}
}

impl<T: Trait> FetchConvertPrice<T::AssetId, T::ConvertPrice> for Module<T> {
	fn fetch_convert_rate(asset_id: T::AssetId) -> T::ConvertPrice {
		let rate = <ConvertPrice<T>>::get(asset_id);

		rate
	}
}

impl<T: Trait> AssetReward<T::AssetId, T::Balance> for Module<T> {
	fn set_asset_reward(token_id: T::AssetId, reward: T::Balance) -> Result<(), ()> {
		if <Pool<T>>::contains_key(&token_id) {
			<Pool<T>>::mutate(token_id, |pool| {
				pool.pending_reward = pool.pending_reward.saturating_add(reward);
			});
			Ok(())
		} else {
			Err(())
		}
	}
}

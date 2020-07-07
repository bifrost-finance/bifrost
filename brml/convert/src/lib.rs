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

#[macro_use]
extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::btree_map::BTreeMap;

mod mock;
mod tests;

use frame_support::traits::Get;
use frame_support::weights::{FunctionOf, DispatchClass, Weight, Pays};
use frame_support::{Parameter, decl_event, decl_error, decl_module, decl_storage, debug, ensure, StorageValue, IterableStorageMap};
use frame_system::{self as system, ensure_root, ensure_signed};
use node_primitives::{AssetTrait, ConvertPool, FetchConvertPrice, AssetReward, TokenType};
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, Zero, MaybeSerializeDeserialize};

pub trait Trait: frame_system::Trait {
	/// convert rate
	type ConvertPrice: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance> + MaybeSerializeDeserialize;
	type RatePerBlock: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance> + Into<Self::ConvertPrice> + MaybeSerializeDeserialize;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize + From<Self::BlockNumber> + Into<Self::ConvertPrice>;

	/// The units in which we record costs.
	type Cost: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record incomes.
	type Income: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

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
		RedeemedPointsSuccess,
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Asset id doesn't exist
		TokenNotExist,
		/// Amount of input should be less than or equal to origin balance
		InvalidBalanceForTransaction,
		/// Convert price doesn't be set
		ConvertPriceIsNotSet,
		/// This is an invalid convert rate
		InvalidConvertPrice,
		/// Vtoken id is not equal to token id
		InvalidTokenPair,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Convert {
		/// convert price between two tokens, vtoken => (token, convert_price)
		ConvertPrice get(fn convert_price): map hasher(blake2_128_concat) TokenType => T::ConvertPrice;
		/// change rate per block, vtoken => (token, rate_per_block)
		RatePerBlock get(fn rate_per_block): map hasher(blake2_128_concat) TokenType => T::RatePerBlock;
		/// collect referrer, converter => ([(referrer1, 1000), (referrer2, 2000), ...], total_point)
		/// total_point = 1000 + 2000 + ...
		/// referrer must be unique, so check it unique while a new referrer incoming.
		/// and insert the new channel to the
		ReferrerChannels get(fn referrer_channels): map hasher(blake2_128_concat) T::AccountId =>
			(Vec<(T::AccountId, T::Balance)>, T::Balance);
		/// referer channels for all users
		AllReferrerChannels get(fn all_referer_channels): (BTreeMap<T::AccountId, T::Balance>, T::Balance);
		/// Convert pool
		Pool get(fn pool): map hasher(blake2_128_concat) TokenType => ConvertPool<T::Balance>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const ConvertDuration: T::BlockNumber = T::ConvertDuration::get();

		fn deposit_event() = default;

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn set_convert_price(
			origin,
			token_type: TokenType,
			convert_price: T::ConvertPrice
		) {
			ensure_root(origin)?;

			ensure!(T::AssetTrait::token_exists(token_type), Error::<T>::TokenNotExist);
			<ConvertPrice<T>>::insert(token_type, convert_price);

			Self::deposit_event(Event::UpdateConvertSuccess);
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn set_price_per_block(
			origin,
			token_type: TokenType,
			rate_per_block: T::RatePerBlock
		) {
			ensure_root(origin)?;

			ensure!(T::AssetTrait::token_exists(token_type), Error::<T>::TokenNotExist);
			<RatePerBlock<T>>::insert(token_type, rate_per_block);

			Self::deposit_event(Event::UpdatezRatePerBlockSuccess);
		}

		#[weight = FunctionOf(
			|args: (&TokenType, &T::Balance, &Option<T::AccountId>)| Module::<T>::calculate_referer_gas(args.2),
			DispatchClass::Normal,
			Pays::Yes
		)]
		fn convert_token_to_vtoken(
			origin,
			vtoken_type: TokenType,
			#[compact] token_amount: T::Balance,
			referrer: Option<T::AccountId>
		) {
			let converter = ensure_signed(origin)?;

			// get paired tokens
			let (token_type, vtoken_type) = vtoken_type.paired_token();

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(token_type), Error::<T>::TokenNotExist);

			let token_balances = T::AssetTrait::get_account_asset(token_type, &converter).balance;
			ensure!(token_balances >= token_amount, Error::<T>::InvalidBalanceForTransaction);

			// check convert price has been set
			ensure!(<ConvertPrice<T>>::contains_key(token_type), Error::<T>::ConvertPriceIsNotSet);

			let price = <ConvertPrice<T>>::get(token_type);

			ensure!(!price.is_zero(), Error::<T>::InvalidConvertPrice);
			let vtokens_buy = token_amount.saturating_mul(price.into());

			// transfer
			T::AssetTrait::asset_destroy(token_type, converter.clone(), token_amount);
			T::AssetTrait::asset_issue(vtoken_type, converter.clone(), vtokens_buy);

			Self::increase_pool(vtoken_type, token_amount, vtokens_buy);

			// save refer channel
			Self::handle_new_refer(converter, referrer, vtokens_buy);

			Self::deposit_event(Event::ConvertTokenToVTokenSuccess);
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn convert_vtoken_to_token(
			origin,
			token_type: TokenType,
			#[compact] vtoken_amount: T::Balance,
		) {
			let converter = ensure_signed(origin)?;

			// get paired tokens
			let (token_type, vtoken_type) = token_type.paired_token();

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(vtoken_type), Error::<T>::TokenNotExist);

			let vtoken_balances = T::AssetTrait::get_account_asset(vtoken_type, &converter).balance;
			ensure!(vtoken_balances >= vtoken_amount, Error::<T>::InvalidBalanceForTransaction);

			// check convert price has been set
			ensure!(<ConvertPrice<T>>::contains_key(token_type), Error::<T>::ConvertPriceIsNotSet);

			let price = <ConvertPrice<T>>::get(token_type);

			ensure!(!price.is_zero(), Error::<T>::InvalidConvertPrice);
			let tokens_buy = vtoken_amount / price.into();

			T::AssetTrait::asset_destroy(vtoken_type, converter.clone(), vtoken_amount);
			T::AssetTrait::asset_issue(token_type, converter.clone(), tokens_buy);

			Self::decrease_pool(vtoken_type, tokens_buy, vtoken_amount);

			// redeem income
			Self::redeem_income(converter, vtoken_amount);

			Self::deposit_event(Event::ConvertVTokenToTokenSuccess);
		}

		fn on_finalize(block_number: T::BlockNumber) {
			// calculate & update convert price
			for (token_id, _convert_pool) in <Pool<T>>::iter() {
				<Pool<T>>::mutate(token_id, |convert_pool| {
					let current_reward = convert_pool.current_reward;
					let reward_per_block = current_reward / T::ConvertDuration::get().into();
					convert_pool.token_pool = convert_pool.token_pool.saturating_add(reward_per_block);

					if convert_pool.token_pool != Zero::zero()
						&& convert_pool.vtoken_pool != Zero::zero()
					{
						if <ConvertPrice<T>>::contains_key(token_id) {
							<ConvertPrice<T>>::mutate(token_id, |convert_price| {
								*convert_price = (convert_pool.token_pool / convert_pool.vtoken_pool).into();
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
		}
	}
}

impl<T: Trait> Module<T> {
	// if the input has something, charge more fee on caller
	fn calculate_referer_gas(referer: &Option<T::AccountId>) -> Weight {
		if referer.is_some() { 100 } else { 10 }
	}

	pub fn get_convert(token_type: TokenType) -> T::ConvertPrice {
		<ConvertPrice<T>>::get(token_type)
	}

	fn increase_pool(token_type: TokenType, token_amount: T::Balance, vtoken_amount: T::Balance) {
		<Pool<T>>::mutate(token_type, |pool| {
			pool.token_pool = pool.token_pool.saturating_add(token_amount);
			pool.vtoken_pool = pool.vtoken_pool.saturating_add(vtoken_amount);
		});
	}

	fn decrease_pool(token_type: TokenType, token_amount: T::Balance, vtoken_amount: T::Balance) {
		<Pool<T>>::mutate(token_type, |pool| {
			pool.token_pool = pool.token_pool.saturating_sub(token_amount);
			pool.vtoken_pool = pool.vtoken_pool.saturating_sub(vtoken_amount);
		});
	}

	fn handle_new_refer(converter: T::AccountId, referrer: Option<T::AccountId>, vtokens_buy: T::Balance) {
		if let Some(ref refer) = referrer {
			if !<ReferrerChannels<T>>::contains_key(&converter) {
				// first time to referrer
				let value = (vec![(refer, vtokens_buy)], vtokens_buy);
				<ReferrerChannels<T>>::insert(&converter, value);
			} else {
				// existed, but new referrer incoming
				<ReferrerChannels<T>>::mutate(&converter, |incomes| {
					if incomes.0.iter().any(|income| income.0.eq(refer)) {
						for income in &mut incomes.0 {
							if income.0.eq(refer) {
								income.1 += vtokens_buy;
							}
						}
						incomes.1 += vtokens_buy;
					} else {
						incomes.1 += vtokens_buy;
						incomes.0.push((refer.clone(), vtokens_buy));
					}
				});
			}

			// update all channels
			if <AllReferrerChannels::<T>>::get().0.contains_key(refer) {
				<AllReferrerChannels::<T>>::mutate(|(channels, total)| {
					*total += vtokens_buy;
					if let Some(income) = channels.get_mut(&refer) {
						*income += vtokens_buy;
					}
				});
			} else {
				<AllReferrerChannels::<T>>::mutate(|(channels, total)| {
					// this referer is not in all referer channels
					let _ = channels.insert(refer.clone(), vtokens_buy);
					*total += vtokens_buy;
				});
			}
		} else {
			();
		}
	}

	fn redeem_income(converter: T::AccountId, incomes_to_redeem: T::Balance) {
		if <ReferrerChannels<T>>::contains_key(&converter) {
			// redeem the points by order
			// for instance: user C has two channels that like: (A, 1000), (B, 2000),
			// if C want to redeem 1500 points, first redeem 1000 from A, then 500 from B
			<ReferrerChannels<T>>::mutate(&converter, |incomes| {
				if incomes.1 < incomes_to_redeem {
					debug::warn!("you're redeem the points that is bigger than all you have.");
					return;
				}

				let mut rest: T::Balance = incomes_to_redeem;
				let mut all_rest: T::Balance = incomes_to_redeem;
				for income in &mut incomes.0 {
					// update user's channels
					if income.1 > rest {
						income.1 -= rest;
						rest = 0.into();
					} else {
						rest -= income.1;
						income.1 = 0.into();
					}

					// update all channels
					<AllReferrerChannels::<T>>::mutate(|(channels, _)| {
						if let Some(b) = channels.get_mut(&income.0) {
							if *b > all_rest {
								*b -= all_rest;
								all_rest = 0.into();
							} else {
								all_rest -= *b;
								*b = 0.into();
							}
						}
					});

					if rest > 0.into() {
						continue;
					}
				}
				// update user's total points
				incomes.1 -= incomes_to_redeem;
				// update all channels total points
				<AllReferrerChannels::<T>>::mutate(|(_, total)| {
					*total -= incomes_to_redeem;
				});
			});
		}
	}
}

impl<T: Trait> FetchConvertPrice<TokenType, T::ConvertPrice> for Module<T> {
	fn fetch_convert_price(token_type: TokenType) -> T::ConvertPrice {
		let price = <ConvertPrice<T>>::get(token_type);

		price
	}
}

impl<T: Trait> AssetReward<TokenType, T::Balance> for Module<T> {
	type Output = ();
	type Error = ();
	fn set_asset_reward(token_type: TokenType, reward: T::Balance) -> Result<(), ()> {
		if <Pool<T>>::contains_key(&token_type) {
			<Pool<T>>::mutate(token_type, |pool| {
				pool.pending_reward = pool.pending_reward.saturating_add(reward);
			});
			Ok(())
		} else {
			Err(())
		}
	}
}

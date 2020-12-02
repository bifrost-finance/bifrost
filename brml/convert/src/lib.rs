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
use frame_support::weights::DispatchClass;
use frame_support::{weights::Weight,Parameter, decl_event, decl_error, decl_module, decl_storage, debug, ensure, StorageValue, IterableStorageMap};
use frame_system::{ensure_root, ensure_signed};
use node_primitives::{AssetTrait, ConvertPool, FetchConvertPrice, FetchConvertPool, AssetReward, RewardHandler};
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, Zero, MaybeSerializeDeserialize};

pub trait WeightInfo {
	fn set_convert_price() -> Weight;
	fn set_price_per_block() -> Weight;
	fn to_vtoken<T: Trait>(referer: Option<&T::AccountId>) -> Weight;
	fn to_token() -> Weight;
}

impl WeightInfo for () {
	fn set_convert_price() -> Weight { Default::default() }
	fn set_price_per_block() -> Weight { Default::default() }
	fn to_vtoken<T: Trait>(_: Option<&T::AccountId>) -> Weight { Default::default() }
	fn to_token() -> Weight { Default::default() }
}

pub trait Trait: frame_system::Trait {
	/// convert rate
	type ConvertPrice: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance> + MaybeSerializeDeserialize;
	type RatePerBlock: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance> + Into<Self::ConvertPrice> + MaybeSerializeDeserialize;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize + From<Self::BlockNumber> + Into<Self::ConvertPrice>;

	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance>;

	/// event
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;

	type ConvertDuration: Get<Self::BlockNumber>;

	/// Set default weight
	type WeightInfo: WeightInfo;

}

decl_event! {
	pub enum Event {
		UpdateConvertSuccess,
		UpdateRatePerBlockSuccess,
		ConvertTokenToVTokenSuccess,
		ConvertVTokenToTokenSuccess,
		RedeemedPointsSuccess,
		UpdateConvertPoolSuccess,
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Asset id doesn't exist
		TokenNotExist,
		/// Amount of input should be less than or equal to origin balance
		InsufficientBalanceForTransaction,
		/// Convert price doesn't be set
		ConvertPriceIsNotSet,
		/// This is an invalid convert rate
		InvalidConvertPrice,
		/// Token type not support
		NotSupportTokenType,
		/// Cannot convert token with itself
		ConvertWithTheSameToken,
		/// Empty convert pool, cause there's no price at all
		EmptyConvertPool,
		/// The amount of token you want to convert is bigger than the convert poll
		NotEnoughConvertPool,
		/// No need to set new convert pool
		NotEmptyPool,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Convert {
		/// convert price between two tokens, vtoken => (token, convert_price)
		ConvertPrice get(fn convert_price) config(): map hasher(blake2_128_concat) T::AssetId => T::ConvertPrice;
		/// change rate per block, vtoken => (token, rate_per_block)
		RatePerBlock get(fn rate_per_block): map hasher(blake2_128_concat) T::AssetId => T::RatePerBlock;
		/// collect referrer, converter => ([(referrer1, 1000), (referrer2, 2000), ...], total_point)
		/// total_point = 1000 + 2000 + ...
		/// referrer must be unique, so check it unique while a new referrer incoming.
		/// and insert the new channel to the
		ReferrerChannels get(fn referrer_channels): map hasher(blake2_128_concat) T::AccountId =>
			(Vec<(T::AccountId, T::Balance)>, T::Balance);
		/// referer channels for all users
		AllReferrerChannels get(fn all_referer_channels): (BTreeMap<T::AccountId, T::Balance>, T::Balance);
		/// Convert pool
		Pool get(fn pool) config(): map hasher(blake2_128_concat) T::AssetId => ConvertPool<T::Balance>;
	}
	add_extra_genesis {
		build(|config: &GenesisConfig<T>| {
			for (asset_id, price) in config.convert_price.iter() {
				ConvertPrice::<T>::insert(asset_id, price);
			}

			for (asset_id, token_pool) in config.pool.iter() {
				let price: T::ConvertPrice = token_pool.vtoken_pool.into() / token_pool.token_pool.into();
				ConvertPrice::<T>::insert(asset_id, price);
				Pool::<T>::insert(asset_id, token_pool);
			}
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const ConvertDuration: T::BlockNumber = T::ConvertDuration::get();

		fn deposit_event() = default;

		#[weight = T::WeightInfo::set_convert_price()]
		fn set_convert_price(
			origin,
			asset_id: T::AssetId,
			convert_price: T::ConvertPrice
		) {
			ensure_root(origin)?;

			ensure!(T::AssetTrait::is_token(asset_id) || T::AssetTrait::is_v_token(asset_id), Error::<T>::NotSupportTokenType);

			ensure!(T::AssetTrait::token_exists(asset_id), Error::<T>::TokenNotExist);
			<ConvertPrice<T>>::insert(asset_id, convert_price);

			Self::deposit_event(Event::UpdateConvertSuccess);
		}

		#[weight = T::WeightInfo::set_price_per_block()]
		fn set_price_per_block(
			origin,
			asset_id: T::AssetId,
			rate_per_block: T::RatePerBlock
		) {
			ensure_root(origin)?;

			ensure!(T::AssetTrait::is_token(asset_id) || T::AssetTrait::is_v_token(asset_id), Error::<T>::NotSupportTokenType);

			ensure!(T::AssetTrait::token_exists(asset_id), Error::<T>::TokenNotExist);
			<RatePerBlock<T>>::insert(asset_id, rate_per_block);

			Self::deposit_event(Event::UpdateRatePerBlockSuccess);
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn set_convert_pool(
			origin,
			asset_id: T::AssetId,
			#[compact] new_token_pool: T::Balance,
			#[compact] new_vtoken_pool: T::Balance
		) {
			ensure_root(origin)?;

			let ConvertPool { token_pool, vtoken_pool, .. } = Pool::<T>::get(asset_id);
			ensure!(token_pool.is_zero() && vtoken_pool.is_zero(), Error::<T>::NotEmptyPool);
			ensure!(new_vtoken_pool / new_token_pool == T::Balance::from(100u32), Error::<T>::NotEmptyPool);

			<Pool<T>>::mutate(asset_id, |pool| {
				pool.token_pool = new_token_pool;
				pool.vtoken_pool = new_vtoken_pool;
			});

			Self::deposit_event(Event::UpdateConvertPoolSuccess);
		}

		#[weight = (T::WeightInfo::to_vtoken::<T>(referer.as_ref()), DispatchClass::Normal)]
		fn to_vtoken(
			origin,
			vtoken_asset_id: T::AssetId,
			#[compact] token_amount: T::Balance,
			referer: Option<T::AccountId>
		) {
			let converter = ensure_signed(origin)?;

			ensure!(T::AssetTrait::is_v_token(vtoken_asset_id), Error::<T>::NotSupportTokenType);

			// get paired tokens
			let token_asset_id = T::AssetTrait::get_pair(vtoken_asset_id).unwrap();

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(token_asset_id), Error::<T>::TokenNotExist);

			let token_balances = T::AssetTrait::get_account_asset(token_asset_id, &converter).balance;
			ensure!(token_balances >= token_amount, Error::<T>::InsufficientBalanceForTransaction);

			// use current covert pool to get latest price
			let ConvertPool { token_pool, vtoken_pool, .. } = Pool::<T>::get(token_asset_id);
			ensure!(!token_pool.is_zero() && !vtoken_pool.is_zero(), Error::<T>::EmptyConvertPool);

			// latest price should be vtoken_pool / token_pool
			let vtokens_buy = token_amount.saturating_mul(vtoken_pool) / token_pool;

			// transfer
			T::AssetTrait::asset_destroy(token_asset_id, &converter, token_amount);
			T::AssetTrait::asset_issue(vtoken_asset_id, &converter, vtokens_buy);

			// both are the same pool, but need to be updated together
			Self::increase_pool(token_asset_id, token_amount, vtokens_buy);

			// save refer channel
			Self::handle_new_refer(converter, referer, vtokens_buy);

			Self::deposit_event(Event::ConvertTokenToVTokenSuccess);
		}

		#[weight = T::WeightInfo::to_token()]
		fn to_token(
			origin,
			token_asset_id: T::AssetId,
			#[compact] vtoken_amount: T::Balance,
		) {
			let converter = ensure_signed(origin)?;

			ensure!(T::AssetTrait::is_token(token_asset_id), Error::<T>::NotSupportTokenType);

			// get paired tokens
			let vtoken_asset_id = T::AssetTrait::get_pair(token_asset_id).unwrap();

			// check  exist or not
			ensure!(T::AssetTrait::token_exists(vtoken_asset_id), Error::<T>::TokenNotExist);

			let vtoken_balances = T::AssetTrait::get_account_asset(vtoken_asset_id, &converter).balance;
			ensure!(vtoken_balances >= vtoken_amount, Error::<T>::InsufficientBalanceForTransaction);

			// use current covert pool to get latest price
			let ConvertPool { token_pool, vtoken_pool, .. } = Pool::<T>::get(token_asset_id);
			ensure!(!token_pool.is_zero() && !vtoken_pool.is_zero(), Error::<T>::EmptyConvertPool);

			let tokens_buy = vtoken_amount.saturating_mul(token_pool) / vtoken_pool;
			ensure!(vtoken_pool >= tokens_buy && vtoken_pool >= vtoken_amount, Error::<T>::NotEnoughConvertPool);

			T::AssetTrait::asset_destroy(vtoken_asset_id, &converter, vtoken_amount);
			T::AssetTrait::asset_issue(token_asset_id, &converter, tokens_buy);

			// both are the same pool, but need to be updated together
			Self::decrease_pool(token_asset_id, tokens_buy, vtoken_amount);

			// redeem income
			Self::redeem_income(converter, vtoken_amount);

			Self::deposit_event(Event::ConvertVTokenToTokenSuccess);
		}

		fn on_finalize(block_number: T::BlockNumber) {
			// calculate & update convert price
			for (token_id, _convert_pool) in <Pool<T>>::iter() {
				<Pool<T>>::mutate(token_id, |convert_pool| {
					// issue staking rewards
					let current_reward = convert_pool.current_reward;
					let reward_per_block = current_reward / T::ConvertDuration::get().into();
					convert_pool.token_pool = convert_pool.token_pool.saturating_add(reward_per_block);

					// update convert price after issued rewwards
					if convert_pool.token_pool != Zero::zero() && convert_pool.vtoken_pool != Zero::zero()
					{
						if <ConvertPrice<T>>::contains_key(token_id) {
							<ConvertPrice<T>>::mutate(token_id, |convert_price| {
								*convert_price = {
									let token_pool: T::ConvertPrice = convert_pool.token_pool.into();
									let vtoken_pool: T::ConvertPrice = convert_pool.vtoken_pool.into();
									vtoken_pool / token_pool
								};
							});
						}
					}
				});
			}

			// finishes current era of rewards, start next round
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
	pub fn get_convert(asset_id: T::AssetId) -> T::ConvertPrice {
		<ConvertPrice<T>>::get(asset_id)
	}

	fn increase_pool(asset_id: T::AssetId, token_amount: T::Balance, vtoken_amount: T::Balance) {
		<Pool<T>>::mutate(asset_id, |pool| {
			pool.token_pool = pool.token_pool.saturating_add(token_amount);
			pool.vtoken_pool = pool.vtoken_pool.saturating_add(vtoken_amount);
		});
	}

	fn decrease_pool(asset_id: T::AssetId, token_amount: T::Balance, vtoken_amount: T::Balance) {
		<Pool<T>>::mutate(asset_id, |pool| {
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
						rest = Zero::zero();
					} else {
						rest -= income.1;
						income.1 = Zero::zero();
					}

					// update all channels
					<AllReferrerChannels::<T>>::mutate(|(channels, _)| {
						if let Some(b) = channels.get_mut(&income.0) {
							if *b > all_rest {
								*b -= all_rest;
								all_rest = Zero::zero();
							} else {
								all_rest -= *b;
								*b = Zero::zero();
							}
						}
					});

					if rest > Zero::zero() {
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

impl<T: Trait> FetchConvertPrice<T::AssetId, T::ConvertPrice> for Module<T> {
	fn fetch_convert_price(asset_id: T::AssetId) -> T::ConvertPrice {
		let price = <ConvertPrice<T>>::get(asset_id);

		price
	}
}

impl<T: Trait> FetchConvertPool<T::AssetId, T::Balance> for Module<T> {
	fn fetch_convert_pool(asset_id: T::AssetId) -> ConvertPool<T::Balance> { Pool::<T>::get(asset_id) }
}

impl<T: Trait> AssetReward<T::AssetId, T::Balance> for Module<T> {
	type Output = ();
	type Error = ();
	fn set_asset_reward(asset_id: T::AssetId, reward: T::Balance) -> Result<(), ()> {
		if <Pool<T>>::contains_key(&asset_id) {
			<Pool<T>>::mutate(asset_id, |pool| {
				pool.pending_reward = pool.pending_reward.saturating_add(reward);
			});
			Ok(())
		} else {
			Err(())
		}
	}
}

impl<T: Trait> RewardHandler<T::AssetId, T::Balance> for Module<T> {
	fn send_reward(asset_id: T::AssetId, reward: T::Balance) {
		if <Pool<T>>::contains_key(asset_id) {
			<Pool<T>>::mutate(asset_id, |pool| {
				pool.pending_reward = pool.pending_reward.saturating_add(reward);
			});
		}
	}
}

// #[allow(dead_code)]
// mod weight_for {
// 	use frame_support::{traits::Get, weights::Weight};
// 	use super::Trait;
//
// 	/// asset_redeem weight
// 	pub(crate) fn convert_token_to_vtoken<T: Trait>(referer: Option<&T::AccountId>) -> Weight {
// 		let referer_weight = referer.map_or(1000, |_| 100);
// 		let db = T::DbWeight::get();
// 		db.reads_writes(1, 1)
// 			.saturating_add(referer_weight) // memo length
// 	}
// }

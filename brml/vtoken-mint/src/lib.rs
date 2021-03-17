// Copyright 2019-2021 Liebi Technologies.
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
use frame_support::{weights::Weight,Parameter, decl_event, decl_error, decl_module, decl_storage, ensure, StorageValue, IterableStorageMap};
use frame_system::{ensure_root, ensure_signed};
use node_primitives::{AssetTrait, VtokenPool, FetchVtokenMintPrice, FetchVtokenMintPool, AssetReward, RewardHandler};
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, Zero, MaybeSerializeDeserialize};

pub trait WeightInfo {
	fn to_vtoken<T: Config>(referer: Option<&T::AccountId>) -> Weight;
	fn to_token() -> Weight;
}

impl WeightInfo for () {
	fn to_vtoken<T: Config>(_: Option<&T::AccountId>) -> Weight { Default::default() }
	fn to_token() -> Weight { Default::default() }
}

pub trait Config: frame_system::Config {
	/// vtoken mint rate
	type MintPrice: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance> + MaybeSerializeDeserialize;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize + From<Self::BlockNumber> + Into<Self::MintPrice>;

	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance>;

	/// event
	type Event: From<Event> + Into<<Self as frame_system::Config>::Event>;

	type VtokenMintDuration: Get<Self::BlockNumber>;

	/// Set default weight
	type WeightInfo: WeightInfo;

}

decl_event! {
	pub enum Event {
		UpdateRatePerBlockSuccess,
		MintVTokenSuccess,
		MintTokenSuccess,
		RedeemedPointsSuccess,
		UpdateVtokenPoolSuccess,
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		/// Asset id doesn't exist
		TokenNotExist,
		/// Amount of input should be less than or equal to origin balance
		InsufficientBalanceForTransaction,
		/// Mint price doesn't be set
		MintPriceIsNotSet,
		/// This is an invalid mint rate
		InvalidMintPrice,
		/// Token type not support
		NotSupportTokenType,
		/// Cannot mint token with itself
		MintWithTheSameToken,
		/// Empty vtoken pool, cause there's no price at all
		EmptyVtokenPool,
		/// The amount of token you want to mint is bigger than the vtoken pool
		NotEnoughVtokenPool,
		/// No need to set new vtoken pool
		NotEmptyPool,
	}
}

decl_storage! {
	trait Store for Module<T: Config> as VtokenMint {
		/// mint price between two tokens, vtoken => (token, mint_price)
		MintPrice get(fn mint_price) config(): map hasher(blake2_128_concat) T::AssetId => T::MintPrice;
		/// collect referrer, minter => ([(referrer1, 1000), (referrer2, 2000), ...], total_point)
		/// total_point = 1000 + 2000 + ...
		/// referrer must be unique, so check it unique while a new referrer incoming.
		/// and insert the new channel to the
		ReferrerChannels get(fn referrer_channels): map hasher(blake2_128_concat) T::AccountId =>
			(Vec<(T::AccountId, T::Balance)>, T::Balance);
		/// referer channels for all users
		AllReferrerChannels get(fn all_referer_channels): (BTreeMap<T::AccountId, T::Balance>, T::Balance);
		/// Vtoken mint pool
		Pool get(fn pool) config(): map hasher(blake2_128_concat) T::AssetId => VtokenPool<T::Balance>;
	}
	add_extra_genesis {
		build(|config: &GenesisConfig<T>| {
			for (asset_id, price) in config.mint_price.iter() {
				MintPrice::<T>::insert(asset_id, price);
			}

			for (asset_id, token_pool) in config.pool.iter() {
				let price: T::MintPrice = token_pool.vtoken_pool.into() / token_pool.token_pool.into();
				MintPrice::<T>::insert(asset_id, price);
				Pool::<T>::insert(asset_id, token_pool);
			}
		});
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const VtokenMintDuration: T::BlockNumber = T::VtokenMintDuration::get();

		fn deposit_event() = default;

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn set_vtoken_pool(
			origin,
			asset_id: T::AssetId,
			#[compact] new_token_pool: T::Balance,
			#[compact] new_vtoken_pool: T::Balance
		) {
			ensure_root(origin)?;

			let VtokenPool { token_pool, vtoken_pool, .. } = Pool::<T>::get(asset_id);
			ensure!(token_pool.is_zero() && vtoken_pool.is_zero(), Error::<T>::NotEmptyPool);
			ensure!(new_vtoken_pool / new_token_pool == T::Balance::from(100u32), Error::<T>::NotEmptyPool);

			<Pool<T>>::mutate(asset_id, |pool| {
				pool.token_pool = new_token_pool;
				pool.vtoken_pool = new_vtoken_pool;
			});

			Self::deposit_event(Event::UpdateVtokenPoolSuccess);
		}

		#[weight = (T::WeightInfo::to_vtoken::<T>(referer.as_ref()), DispatchClass::Normal)]
		fn to_vtoken(
			origin,
			vtoken_asset_id: T::AssetId,
			#[compact] token_amount: T::Balance,
			referer: Option<T::AccountId>
		) {
			let minter = ensure_signed(origin)?;

			ensure!(T::AssetTrait::is_v_token(vtoken_asset_id), Error::<T>::NotSupportTokenType);

			// get paired tokens
			let token_asset_id = T::AssetTrait::get_pair(vtoken_asset_id).unwrap();

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(token_asset_id), Error::<T>::TokenNotExist);

			let token_balances = T::AssetTrait::get_account_asset(token_asset_id, &minter).balance;
			ensure!(token_balances >= token_amount, Error::<T>::InsufficientBalanceForTransaction);

			// use current covert pool to get latest price
			let VtokenPool { token_pool, vtoken_pool, .. } = Pool::<T>::get(token_asset_id);
			ensure!(!token_pool.is_zero() && !vtoken_pool.is_zero(), Error::<T>::EmptyVtokenPool);

			// latest price should be vtoken_pool / token_pool
			let vtokens_buy = token_amount.saturating_mul(vtoken_pool) / token_pool;

			// transfer
			T::AssetTrait::asset_destroy(token_asset_id, &minter, token_amount);
			T::AssetTrait::asset_issue(vtoken_asset_id, &minter, vtokens_buy);

			// both are the same pool, but need to be updated together
			Self::increase_pool(token_asset_id, token_amount, vtokens_buy);

			// save refer channel
			Self::handle_new_refer(minter, referer, vtokens_buy);

			Self::deposit_event(Event::MintVTokenSuccess);
		}

		#[weight = T::WeightInfo::to_token()]
		fn to_token(
			origin,
			token_asset_id: T::AssetId,
			#[compact] vtoken_amount: T::Balance,
		) {
			let minter = ensure_signed(origin)?;

			ensure!(T::AssetTrait::is_token(token_asset_id), Error::<T>::NotSupportTokenType);

			// get paired tokens
			let vtoken_asset_id = T::AssetTrait::get_pair(token_asset_id).unwrap();

			// check  exist or not
			ensure!(T::AssetTrait::token_exists(vtoken_asset_id), Error::<T>::TokenNotExist);

			let vtoken_balances = T::AssetTrait::get_account_asset(vtoken_asset_id, &minter).balance;
			ensure!(vtoken_balances >= vtoken_amount, Error::<T>::InsufficientBalanceForTransaction);

			// use current covert pool to get latest price
			let VtokenPool { token_pool, vtoken_pool, .. } = Pool::<T>::get(token_asset_id);
			ensure!(!token_pool.is_zero() && !vtoken_pool.is_zero(), Error::<T>::EmptyVtokenPool);

			let tokens_buy = vtoken_amount.saturating_mul(token_pool) / vtoken_pool;
			ensure!(vtoken_pool >= tokens_buy && vtoken_pool >= vtoken_amount, Error::<T>::NotEnoughVtokenPool);

			T::AssetTrait::asset_destroy(vtoken_asset_id, &minter, vtoken_amount);
			T::AssetTrait::asset_issue(token_asset_id, &minter, tokens_buy);

			// both are the same pool, but need to be updated together
			Self::decrease_pool(token_asset_id, tokens_buy, vtoken_amount);

			// redeem income
			Self::redeem_income(minter, vtoken_amount);

			Self::deposit_event(Event::MintTokenSuccess);
		}

		fn on_finalize(block_number: T::BlockNumber) {
			// calculate & update mint price
			for (token_id, _mint_pool) in <Pool<T>>::iter() {
				<Pool<T>>::mutate(token_id, |mint_pool| {
					// issue staking rewards
					let current_reward = mint_pool.current_reward;
					let reward_per_block = current_reward / T::VtokenMintDuration::get().into();
					mint_pool.token_pool = mint_pool.token_pool.saturating_add(reward_per_block);

					// update mint price after issued rewwards
					if mint_pool.token_pool != Zero::zero() && mint_pool.vtoken_pool != Zero::zero()
					{
						if <MintPrice<T>>::contains_key(token_id) {
							<MintPrice<T>>::mutate(token_id, |mint_price| {
								*mint_price = {
									let token_pool: T::MintPrice = mint_pool.token_pool.into();
									let vtoken_pool: T::MintPrice = mint_pool.vtoken_pool.into();
									vtoken_pool / token_pool
								};
							});
						}
					}
				});
			}

			// finishes current era of rewards, start next round
			if block_number % T::VtokenMintDuration::get() == Zero::zero() {
				// new vtoken mint round
				for (token_id, _mint_pool) in <Pool<T>>::iter() {
					<Pool<T>>::mutate(token_id, |mint_pool| {
						mint_pool.new_round();
					});
				}
			}
		}
	}
}

impl<T: Config> Module<T> {
	pub fn get_vtoken_mint_price(asset_id: T::AssetId) -> T::MintPrice {
		<MintPrice<T>>::get(asset_id)
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

	fn handle_new_refer(minter: T::AccountId, referrer: Option<T::AccountId>, vtokens_buy: T::Balance) {
		if let Some(ref refer) = referrer {
			if !<ReferrerChannels<T>>::contains_key(&minter) {
				// first time to referrer
				let value = (vec![(refer, vtokens_buy)], vtokens_buy);
				<ReferrerChannels<T>>::insert(&minter, value);
			} else {
				// existed, but new referrer incoming
				<ReferrerChannels<T>>::mutate(&minter, |incomes| {
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

	fn redeem_income(minter: T::AccountId, incomes_to_redeem: T::Balance) {
		if <ReferrerChannels<T>>::contains_key(&minter) {
			// redeem the points by order
			// for instance: user C has two channels that like: (A, 1000), (B, 2000),
			// if C want to redeem 1500 points, first redeem 1000 from A, then 500 from B
			<ReferrerChannels<T>>::mutate(&minter, |incomes| {
				if incomes.1 < incomes_to_redeem {
					log::warn!("you're redeem the points that is bigger than all you have.");
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

impl<T: Config> FetchVtokenMintPrice<T::AssetId, T::MintPrice> for Module<T> {
	fn fetch_vtoken_price(asset_id: T::AssetId) -> T::MintPrice {
		let price = <MintPrice<T>>::get(asset_id);

		price
	}
}

impl<T: Config> FetchVtokenMintPool<T::AssetId, T::Balance> for Module<T> {
	fn fetch_vtoken_pool(asset_id: T::AssetId) -> VtokenPool<T::Balance> { Pool::<T>::get(asset_id) }
}

impl<T: Config> AssetReward<T::AssetId, T::Balance> for Module<T> {
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

impl<T: Config> RewardHandler<T::AssetId, T::Balance> for Module<T> {
	fn send_reward(asset_id: T::AssetId, reward: T::Balance) {
		if <Pool<T>>::contains_key(asset_id) {
			<Pool<T>>::mutate(asset_id, |pool| {
				pool.pending_reward = pool.pending_reward.saturating_add(reward);
			});
		}
	}
}

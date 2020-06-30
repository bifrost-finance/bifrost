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
use core::convert::{From, Into, TryInto};
use fixed_point::{FixedI128, types::{*, extra}, transcendental, traits::FromFixed};
use frame_support::traits::{Get};
use frame_support::{decl_event, decl_error, decl_module, decl_storage, ensure, Parameter, dispatch::DispatchResult, StorageValue, IterableStorageMap};
use frame_system::{self as system, ensure_signed};
use node_primitives::{AssetTrait, AssetSymbol, TokenType};
use sp_runtime::traits::{MaybeSerializeDeserialize, Member, Saturating, AtLeast32Bit, Zero};

mod mock;
mod tests;

pub trait Trait: frame_system::Trait {
	/// event
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// fee
	type Fee: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize + Into<Self::Balance> + From<Self::Balance>;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record costs.
	type Cost: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record incomes.
	type Income: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance, Self::Cost, Self::Income>;

	/// InvariantValue
	type InvariantValue: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize + From<Self::Balance> + From<Self::Balance>;

	/// Weight
	type Weight: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize+ Into<Self::Balance> + From<Self::Balance> + Into<Self::InvariantValue>;

	/// Some limitations on Balancer protocol
	type MinimumBalance: Get<Self::Balance>;
	type FeePrecision: Get<Self::Balance>;
}

decl_event! {
	pub enum Event<T> where <T as Trait>::Balance, {
		AddLiquiditySuccess,
		RemoveLiquiditySuccess,
		WeightedAssetsDepositSuccess,
		WeightedAssetsWithdrawSuccess,
		SingleAssetDepositSuccess,
		SingleAssetWithdrawSuccess,
		SwapTokenSuccess(Balance, Balance),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Asset id doesn't exist
		TokenNotExist,
		/// Amount of input should be less than or equal to origin balance
		NotEnoughBalance,
		/// Too big slipage price
		ExceedSlipagePrice,
		/// Redeem too much
		RedeemTooMuch,
		/// Convert type with error
		ConvertFailure,
		/// Balance imitation on adding new pool
		LessThanMinimumBalance,
		/// Too many tokens added to pool
		TooManyTokensToPool,
		/// User have no current single pool
		NotExistedCurrentSinglePool,
		/// User have no current single pool
		NotExistedCurrentPool,
		/// User cannot swap between two the same token
		ForbidSameTokenSwap,
		/// Error on fix point crate
		FixedPointError,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Swap {
		/// Balancer pool token
		BalancerPoolToken get(fn all_pool_token) config(): T::Balance; // set pool token as 1000 by default

		/// Global pool, pool's details, like asset type, balance, weight, and value of function
		GlobalPool get(fn global_pool) config(): (Vec<(T::AssetId, TokenType, T::Balance, T::Weight)>, T::InvariantValue);

		/// Each user details for pool
		UserPool get(fn user_pool) config(): map hasher(blake2_128_concat) T::AccountId => (Vec<(T::AssetId, TokenType, T::Balance)>, T::Balance);

		/// User may add a single asst to liquidity
		UserSinglePool: map hasher(blake2_128_concat) (T::AccountId, T::AssetId, TokenType) => (T::Balance, T::Balance);
		// (T::Balance, BalancerPoolToken)

		/// Now only support 7 tokens
		NumberOfSupportedTokens get(fn count_of_supported_tokens) config(): u8;

		/// Total weights
		TotalWeight get(fn get_total_weight) build(|config: &GenesisConfig<T>| {
			config.total_weight.iter().fold(Zero::zero(), |acc: T::Weight, w| acc + *w)
		}): T::Weight;

		/// Each token's weight
		TokenWeight get(fn token_weight): map hasher(blake2_128_concat) (T::AssetId, TokenType) => T::Weight;

		/// Fee stuff
		SwapFee get(fn swap_fee) config(): T::Fee;
		ExitFee get(fn exit_fee) config(): T::Fee;
	}
	add_extra_genesis {
		config(total_weight): Vec<T::Weight>;
		build(|config: &GenesisConfig<T>| {
			// initialize pool token
			<BalancerPoolToken<T>>::put(config.all_pool_token);
			// initialize count of supported tokens
			NumberOfSupportedTokens::put(config.count_of_supported_tokens);
			// set fee
			<SwapFee<T>>::put(config.swap_fee);
			<ExitFee<T>>::put(config.exit_fee);
			// set token weight
			for p in config.global_pool.0.iter() {
				<TokenWeight<T>>::insert((p.0, p.1), p.3);
			}
			// initialize a pool for user
			for (who, pool) in config.user_pool.iter() {
				<UserPool<T>>::insert(who, pool);
			}
			// initialize global pool
			<GlobalPool<T>>::put(&config.global_pool);
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const MinimumBalance: T::Balance = T::MinimumBalance::get();
		const FeePrecision: T::Balance = T::FeePrecision::get();

		fn deposit_event() = default;

		#[weight = 0]
		fn add_liquidity(
			origin,
			#[compact] new_pool_token: T::Balance
		) {
			let provider = ensure_signed(origin)?;

			// ensure new pool's balances bigger than MinimumBalance
			ensure!(new_pool_token >= T::MinimumBalance::get(), Error::<T>::LessThanMinimumBalance);

			// two times db reading
			let all_pool_tokens = BalancerPoolToken::<T>::get();
			let whole_pool = GlobalPool::<T>::get();

			// ensure this user have all kind of tokens and enough balance to deposit
			let mut new_user_pool = Vec::with_capacity(whole_pool.0.len());
			for p in whole_pool.0.iter() {
				// ensure user have the token
				ensure!(T::AssetTrait::token_exists(p.0), Error::<T>::TokenNotExist);

				let balances = T::AssetTrait::get_account_asset(&p.0, p.1, &provider).balance;
				// about the algorithm: https://balancer.finance/whitepaper/#all-asset-depositwithdrawal
				let need_deposited = new_pool_token.saturating_mul(balances) / all_pool_tokens; // todo, div may lose precision
				// ensure user have enough token to deposit to this pool
				ensure!(balances >= need_deposited, Error::<T>::NotEnoughBalance);
				new_user_pool.push((p.0, p.1, need_deposited));
			}

			let provider_pool = UserPool::<T>::get(&provider);
			// first time to add liquidity
			if provider_pool.0.is_empty() || provider_pool.1 == 0.into() {
				UserPool::<T>::mutate(&provider, |pool| {
					// update user's pool
					pool.0 = new_user_pool.clone();
					pool.1 = new_pool_token;
				});
			} else {
				// add more liquidity
				UserPool::<T>::mutate(&provider, |pool| {
					// update pool token
					pool.1 = pool.1.saturating_add(new_pool_token);

					// update user's pool
					for p in pool.0.iter_mut().zip(new_user_pool.iter()) {
						(p.0).2 = (p.0).2.saturating_add((p.1).2);
					}
				});
			}

			// destroy token from user's assets
			for p in new_user_pool.iter() {
				T::AssetTrait::asset_redeem(p.0, p.1, provider.clone(), p.2);
			}

			// update whole pool token
			BalancerPoolToken::<T>::mutate(|pool_token| {
				*pool_token = pool_token.saturating_add(new_pool_token);
			});

			// update global pool
			GlobalPool::<T>::mutate(|pool| {
				for (p, n) in pool.0.iter_mut().zip(new_user_pool.iter()) {
					p.2 = p.2.saturating_add(n.2);
				}
			});

			Self::deposit_event(RawEvent::AddLiquiditySuccess);
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn add_single_liquidity(
			origin,
			token_symbol: AssetSymbol,
			token_type: TokenType,
			#[compact] token_amount_in: T::Balance,
		) -> DispatchResult {
			let provider = ensure_signed(origin)?;

			let token_id = T::AssetId::from(token_symbol as u32);

			// ensure user have token
			ensure!(T::AssetTrait::token_exists(token_id), Error::<T>::TokenNotExist);

			let balances = T::AssetTrait::get_account_asset(&token_id, token_type, &provider).balance;
			// ensure this use have enough balanes to deposit
			ensure!(balances >= token_amount_in, Error::<T>::NotEnoughBalance);

			// get current token balance and weight in the pool
			let (token_balance_in, token_weight_in) = {
				let whole_pool = GlobalPool::<T>::get();
				let mut token_balance_in = 0.into();
				let mut token_weight_in = 0.into();
				for p in whole_pool.0.iter() {
					if token_id == p.0 && token_type == p.1 {
						token_balance_in = p.2;
						token_weight_in = p.3;
						break;
					}
				}
				(token_balance_in, token_weight_in)
			};

			let pool_supply = BalancerPoolToken::<T>::get();

			let total_weight = TotalWeight::<T>::get();
			let swap_fee = SwapFee::<T>::get();

			// caculate how many pool token will be issued to user
			let new_pool_token = {
				let issued_pool_token = Self::calculate_pool_out_given_single_in(token_balance_in, token_weight_in, token_amount_in, total_weight, pool_supply, swap_fee)?;
				let pool_token_issued = u128::from_fixed(issued_pool_token);
				TryInto::<T::Balance>::try_into(pool_token_issued).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// first time to add liquidity
			if !UserSinglePool::<T>::contains_key((&provider, token_id, token_type)) {
				// add it to user's single pool
				UserSinglePool::<T>::insert(
					(&provider, token_id, token_type),
					(token_amount_in, new_pool_token)
				);
			} else {
				// add more liquidity to current single pool
				let single_pool = UserSinglePool::<T>::contains_key((&provider, token_id, token_type));
				UserSinglePool::<T>::mutate((&provider, token_id, token_type), |pool| {
					pool.0 = pool.0.saturating_add(token_amount_in);
					pool.1 = pool.1.saturating_add(new_pool_token);
				});
			}

			// update whole pool
			BalancerPoolToken::<T>::mutate(|pool_token| {
				*pool_token = pool_token.saturating_add(new_pool_token);
			});

			// destroy token from user
			T::AssetTrait::asset_redeem(token_id, token_type, provider, token_amount_in);

			Self::deposit_event(RawEvent::SingleAssetDepositSuccess);
			Ok(())
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn remove_single_liquidity(
			origin,
			token_symbol: AssetSymbol,
			token_type: TokenType,
			#[compact] pool_token_in: T::Balance
		) -> DispatchResult {
			let remover = ensure_signed(origin)?;
			let token_id = T::AssetId::from(token_symbol as u32);

			// ensure this user has the pool
			ensure!(
				UserSinglePool::<T>::contains_key((&remover, token_id, token_type)),
				Error::<T>::NotExistedCurrentSinglePool
			);

			// ensure user doesn't redeem exceed all he has
			let user_single_pool = UserSinglePool::<T>::get((&remover, token_id, token_type));
			ensure!(user_single_pool.1 >= pool_token_in, Error::<T>::NotEnoughBalance);

			let total_weight = TotalWeight::<T>::get();
			let swap_fee = SwapFee::<T>::get();
			let exit_fee = ExitFee::<T>::get();

			// get token's weight
			let token_weight = {
				let mut weight = T::Weight::from(0);
				for pool in GlobalPool::<T>::get().0.iter() {
					if token_id == pool.0 {
						weight = pool.3;
						break;
					}
				}
				weight
			};

			let pool_supply = BalancerPoolToken::<T>::get();
			ensure!(pool_token_in <= pool_supply, Error::<T>::NotEnoughBalance);

			// calculate how many balance user will get
			let token_amount = {
				let token_amount_out = Self::calculate_single_out_given_pool_in(token_weight, pool_token_in, total_weight, user_single_pool.0, pool_supply, swap_fee, exit_fee)?;
				let token_amount_out = u128::from_fixed(token_amount_out);

				TryInto::<T::Balance>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// update user asset
			T::AssetTrait::asset_issue(token_id, token_type, remover.clone(), token_amount);
			// update user's pool
			UserSinglePool::<T>::mutate((&remover, token_id, token_type), |pool| {
				pool.0 = pool.0.saturating_sub(token_amount);
				pool.1 = pool.1.saturating_sub(pool_token_in);
			});
			// update whole pool token
			BalancerPoolToken::<T>::mutate(|pool| {
				*pool = pool.saturating_sub(pool_token_in);
			});

			Self::deposit_event(RawEvent::SingleAssetWithdrawSuccess);

			Ok(())
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn remove_all_assets_liquidity(
			origin,
			#[compact] pool_amount_in: T::Balance
		) {
			let remover = ensure_signed(origin)?;

			// ensure this user have the pool
			ensure!(UserPool::<T>::contains_key(&remover), Error::<T>::NotExistedCurrentPool);

			let whole_pool = BalancerPoolToken::<T>::get();
			let user_pool = UserPool::<T>::get(&remover);
			// ensure user doesn't redeem too many
			ensure!(user_pool.1 >= pool_amount_in, Error::<T>::NotEnoughBalance);

			let mut redeemed_pool = Vec::with_capacity(user_pool.0.len());
			for p in user_pool.0.iter() {
				let to_redeem =  p.2.saturating_mul(pool_amount_in) / whole_pool;
				ensure!(to_redeem <= p.2, Error::<T>::NotEnoughBalance);
				redeemed_pool.push((p.0, p.1, to_redeem));
			}

			// update user pool
			UserPool::<T>::mutate(&remover, |pool| {
				pool.1 = pool.1.saturating_sub(pool_amount_in);
				for (p, r) in pool.0.iter_mut().zip(redeemed_pool.iter()) {
					p.2 = p.2.saturating_sub(r.2);
				}
			});

			// update whole pool
			BalancerPoolToken::<T>::mutate(|pool| {
				*pool = pool.saturating_sub(pool_amount_in);
			});

			// redeem assets
			for p in redeemed_pool.iter() {
				T::AssetTrait::asset_issue(p.0, p.1, remover.clone(), p.2);
			}

			Self::deposit_event(RawEvent::WeightedAssetsWithdrawSuccess);
		}

		// consider maxPrice and minAmountOut
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn swap_out_given_in(
			origin,
			token_in_symbol: AssetSymbol,
			token_in_type: TokenType,
			#[compact]token_amount_in: T::Balance,
			min_token_amount_out: Option<T::Balance>,
			token_out_symbol: AssetSymbol,
			token_out_ype: TokenType,
			max_price: Option<T::Balance>
		) -> DispatchResult {
			let swaper = ensure_signed(origin)?;

			// ensure token symbol is different
			ensure!(token_in_symbol != token_out_symbol, Error::<T>::ForbidSameTokenSwap);

			// ensure this user have the pool
			ensure!(UserPool::<T>::contains_key(&swaper), Error::<T>::NotExistedCurrentPool);

			let token_in_id = T::AssetId::from(token_in_symbol as u32);
			let token_out_id = T::AssetId::from(token_out_symbol as u32);

			let swaper_pool = UserPool::<T>::get(&swaper);
			// ensure this user have enough balance to make a transaction
			ensure!(
				swaper_pool.0.iter().filter(|p| p.0 == token_in_id).all(|p| p.2 >= token_amount_in),
				Error::<T>::NotEnoughBalance
			);

			// spot price before do a swap
			// let spot_price_before = todo!("Self::calculate_spot_price");
			// ensure!(spot_price <= max_price, "detailed error");

			let total_weight = TotalWeight::<T>::get();
			let swap_fee = SwapFee::<T>::get();

			let ((token_balance_in, token_weight_in), (token_balance_out, token_weight_out)) = {
				let mut weight_in = T::Weight::from(0);
				let mut weight_out = T::Weight::from(0);
				let mut token_balance_in = T::Balance::from(0);
				let mut token_balance_out = T::Balance::from(0);
				for pool in GlobalPool::<T>::get().0.iter() {
					if token_in_id == pool.0 {
						weight_in = pool.3;
						token_balance_in = pool.2;
					}
					if token_out_id == pool.0 {
						weight_out = pool.3;
						token_balance_out = pool.2;
					}
				}
				((token_balance_in, weight_in), (token_balance_out, weight_out))
			};

			// do a swap
			let token_amount_out = {
				let fixed_token_amount_out = Self::calculate_out_given_in(token_balance_in, token_weight_in, token_amount_in, token_balance_out, token_weight_out, swap_fee)?;
				let token_amount_out = u128::from_fixed(fixed_token_amount_out);

				TryInto::<T::Balance>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// ensure token_amount_in is bigger than you exepect
			// ensure!(token_amount_in >= min_token_amount_out, "");

			// spot price before do a swap
			// let spot_price_before = todo!("Self::calculate_spot_price");
			// ensure!(spot_price >= max_price, "detailed error");

			// update user pool
			UserPool::<T>::mutate(&swaper, |pool| {
				for p in pool.0.iter_mut() {
					if token_in_id == p.0 {
						p.2 = p.2.saturating_sub(token_amount_in);
					}
					if token_out_id == p.0 {
						p.2 = p.2.saturating_add(token_amount_out);
					}
				}
			});

			// update global pool
			GlobalPool::<T>::mutate(|pool| {
				for p in pool.0.iter_mut() {
					if token_in_id == p.0 {
						p.2 = p.2.saturating_sub(token_amount_in);
					}
					if token_out_id == p.0 {
						p.2 = p.2.saturating_add(token_amount_out);
					}
				}
			});

			Self::deposit_event(RawEvent::SwapTokenSuccess(token_amount_in, token_amount_out));

			Ok(())
		}
	}
}

impl<T: Trait> Module<T> {
	pub(crate) fn total_weight(pool: &[(T::AssetId, T::Balance, T::Weight)]) -> T::Weight {
		pool.iter().fold(0.into(), |acc, v| acc + v.2)
	}

	pub(crate) fn weight_ratio(upper: T::Weight, down: T::Weight) -> Result<FixedI128<extra::U64>, Error<T>> {
		let u = TryInto::<u128>::try_into(upper).map_err(|_| Error::<T>::ConvertFailure)?;
		let d = TryInto::<u128>::try_into(down).map_err(|_| Error::<T>::ConvertFailure)?;

		let fixed = {
			let u = FixedI128::<extra::U64>::from_num(u);
			let d = FixedI128::<extra::U64>::from_num(d);
			u.saturating_div(d)
		};

		Ok(fixed)
	}

	/**********************************************************************************************
	// https://balancer.finance/whitepaper/#value-function                                       //
	**********************************************************************************************/
	pub(crate) fn value_function(pool: &[(T::AssetId, T::Balance, T::Weight)]) -> Result<T::InvariantValue, Error<T>> {
		let total_weight = Self::total_weight(pool);

		let mut v = FixedI128::<extra::U64>::from_num(1);
		for p in pool.iter() {
			let base = {
				let v = TryInto::<u128>::try_into(p.1).map_err(|_| Error::<T>::ConvertFailure)?;
				FixedI128::<extra::U64>::from_num(v)
			};
			let exp = Self::weight_ratio(p.2, total_weight)?;
			let power = transcendental::pow(base, exp).map_err(|_| Error::<T>::FixedPointError)?;

			v = v.saturating_mul(power);
		}
		let fixed_v = u128::from_fixed(v);

		TryInto::<T::InvariantValue>::try_into(fixed_v).map_err(|_| Error::<T>::ConvertFailure)
	}

	/**********************************************************************************************
	// https://balancer.finance/whitepaper/#spot-price                                           //
	// spot price                                                                                //
	// sP = spotPrice                                                                            //
	// bI = tokenBalanceIn                ( bI / wI )         1                                  //
	// bO = tokenBalanceOut         sP =  -----------  *  ----------                             //
	// wI = tokenWeightIn                 ( bO / wO )     ( 1 - sF )                             //
	// wO = tokenWeightOut                                                                       //
	// sF = swapFee                                                                              //
	**********************************************************************************************/
	pub(crate) fn calculate_spot_price(
		token_balance_in: T::Balance,
		token_weight_in: T::Weight,
		token_balance_out: T::Balance,
		token_weight_out: T::Weight,
		swap_fee: T::Fee
	) -> Result<I64F64, Error<T>> {
		// convert to u128
		let token_balance_in = TryInto::<u128>::try_into(token_balance_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_weight_in = TryInto::<u128>::try_into(token_weight_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_balance_out = TryInto::<u128>::try_into(token_balance_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_weight_out = TryInto::<u128>::try_into(token_weight_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee = TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;

		// convert to fixed num
		let token_balance_in = FixedI128::<extra::U64>::from_num(token_balance_in);
		let token_weight_in = FixedI128::<extra::U64>::from_num(token_weight_in);
		let token_balance_out = FixedI128::<extra::U64>::from_num(token_balance_out);
		let token_weight_out = FixedI128::<extra::U64>::from_num(token_weight_out);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get()).map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			let fee = FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision);
			FixedI128::<extra::U64>::from_num(1).saturating_sub(fee)
		};

		let fixed_price = token_balance_in.saturating_mul(token_weight_out) / (token_weight_in.saturating_mul(token_balance_out).saturating_mul(swap_fee));
		Ok(fixed_price)
	}

	/**********************************************************************************************
	// calcInGivenOut                                                                            //
	// aI = tokenAmountIn                                                                        //
	// bO = tokenBalanceOut               /  /     bO      \    (wO / wI)      \                 //
	// bI = tokenBalanceIn          bI * |  | ------------  | ^            - 1  |                //
	// aO = tokenAmountOut    aI =        \  \ ( bO - aO ) /                   /                 //
	// wI = tokenWeightIn           --------------------------------------------                 //
	// wO = tokenWeightOut                          ( 1 - sF )                                   //
	// sF = swapFee                                                                              //
	**********************************************************************************************/
	pub(crate) fn calculate_in_given_out(
		token_balance_in: T::Balance,
		token_weight_in: T::Weight,
		token_balance_out: T::Balance,
		token_weight_out: T::Weight,
		token_amount_out: T::Balance,
		swap_fee: T::Fee
	) -> Result<I64F64, Error<T>> {
		// type convert to u128
		let token_balance_in = TryInto::<u128>::try_into(token_balance_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_balance_out = TryInto::<u128>::try_into(token_balance_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_amount_out = TryInto::<u128>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee = TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;
		// to fixed num
		let token_balance_in = I64F64::from_num(token_balance_in);
		let token_balance_out = FixedI128::<extra::U64>::from_num(token_balance_out);
		let token_amount_out = FixedI128::<extra::U64>::from_num(token_amount_out);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get()).map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			let fee = FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision);
			FixedI128::<extra::U64>::from_num(1).saturating_sub(fee)
		};

		// pow exp
		let weight_ratio = Self::weight_ratio(token_weight_in, token_weight_out)?;
		// pow base
		let base = token_balance_out.saturating_div(token_balance_out.saturating_sub(token_amount_out));
		let fixed_token_amount_in = {
			let fixed_power: FixedI128::<extra::U64> = transcendental::pow(base, weight_ratio).map_err(|_| Error::<T>::FixedPointError)?;
			let upper = token_balance_in.saturating_mul(fixed_power.saturating_sub(FixedI128::<extra::U64>::from_num(1)));
			upper.saturating_div(swap_fee)
		};

		Ok(fixed_token_amount_in)
	}

	/**********************************************************************************************
	// calcOutGivenIn                                                                            //
	// aO = tokenAmountOut                                                                       //
	// bO = tokenBalanceOut                                                                      //
	// bI = tokenBalanceIn              /      /            bI             \    (wI / wO) \      //
	// aI = tokenAmountIn    aO = bO * |  1 - | --------------------------  | ^            |     //
	// wI = tokenWeightIn               \      \ ( bI + ( aI * ( 1 - sF )) /              /      //
	// wO = tokenWeightOut                                                                       //
	// sF = swapFee                                                                              //
	**********************************************************************************************/
	pub(crate) fn calculate_out_given_in(
		token_balance_in: T::Balance,
		token_weight_in: T::Weight,
		token_amount_in: T::Balance,
		token_balance_out: T::Balance,
		token_weight_out: T::Weight,
		swap_fee: T::Fee
	) -> Result<I64F64, Error<T>> {
		// type convert to u128
		let token_balance_in = TryInto::<u128>::try_into(token_balance_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_balance_out = TryInto::<u128>::try_into(token_balance_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_amount_in = TryInto::<u128>::try_into(token_amount_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee = TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;
		// to fixed num
		let token_balance_in = I64F64::from_num(token_balance_in);
		let token_balance_out = FixedI128::<extra::U64>::from_num(token_balance_out);
		let token_amount_in = FixedI128::<extra::U64>::from_num(token_amount_in);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get()).map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			let fee = FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision);
			FixedI128::<extra::U64>::from_num(1).saturating_sub(fee)
		};

		// pow exp
		let weight_ratio = Self::weight_ratio(token_weight_in, token_weight_out)?;
		// pow base
		let base = {
			let down = token_balance_in.saturating_add(token_amount_in.saturating_mul(swap_fee));
			token_balance_in.saturating_div(down)
		};

		let fixed_token_amount_out = {
			let rhs = FixedI128::<extra::U64>::from_num(1).saturating_sub(transcendental::pow(base, weight_ratio).map_err(|_| Error::<T>::FixedPointError)?);
			token_balance_out.saturating_mul(rhs)
		};

		Ok(fixed_token_amount_out)
	}

	/**********************************************************************************************
	// calcPoolOutGivenSingleIn                                                                  //
	// pAo = poolAmountOut         /                                              \              //
	// tAi = tokenAmountIn        ///      /     //    wI \      \\       \     wI \             //
	// wI = tokenWeightIn        //| tAi *| 1 - || 1 - --  | * sF || + tBi \    --  \            //
	// tW = totalWeight     pAo=||  \      \     \\    tW /      //         | ^ tW   | * pS - pS //
	// tBi = tokenBalanceIn      \\  ------------------------------------- /        /            //
	// pS = poolSupply            \\                    tBi               /        /             //
	// sF = swapFee                \                                              /              //
	**********************************************************************************************/
	pub(crate) fn calculate_pool_out_given_single_in(
		token_balance_in: T::Balance,
		token_weight_in: T::Weight,
		token_amount_in: T::Balance,
		token_total_weight: T::Weight,
		pool_supply: T::Balance,
		swap_fee: T::Fee
	) -> Result<I64F64, Error<T>> {
		// type convert to u128
		let token_balance_in = TryInto::<u128>::try_into(token_balance_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_amount_in = TryInto::<u128>::try_into(token_amount_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_supply = TryInto::<u128>::try_into(pool_supply).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee = TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;
		// to fixed num
		let token_balance_in = I64F64::from_num(token_balance_in);
		let token_amount_in = FixedI128::<extra::U64>::from_num(token_amount_in);
		let pool_supply = FixedI128::<extra::U64>::from_num(pool_supply);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get()).map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision)
		};

		let weight_ratio = Self::weight_ratio(token_weight_in, token_total_weight)?;

		let pool_token_issued = {
			let fee = FixedI128::<extra::U64>::from_num(1) - FixedI128::<extra::U64>::from_num(1).saturating_sub(weight_ratio).saturating_mul(swap_fee);
			let base = token_amount_in.saturating_mul(fee).saturating_div(token_balance_in).saturating_add(FixedI128::<extra::U64>::from_num(1));
			let lhs: FixedI128::<extra::U64> = transcendental::pow(base, weight_ratio).map_err(|_| Error::<T>::FixedPointError)?;
			pool_supply.saturating_mul(lhs.saturating_sub(FixedI128::<extra::U64>::from_num(1)))
		};

		Ok(pool_token_issued)
	}

	/**********************************************************************************************
	// calcSingleInGivenPoolOut                                                                  //
	// tAi = tokenAmountIn              //(pS + pAo)\     /    1    \\                           //
	// pS = poolSupply                 || ---------  | ^ | --------- || * bI - bI                //
	// pAo = poolAmountOut              \\    pS    /     \(wI / tW)//                           //
	// bI = balanceIn          tAi =  --------------------------------------------               //
	// wI = weightIn                              /      wI  \                                   //
	// tW = totalWeight                          |  1 - ----  |  * sF                            //
	// sF = swapFee                               \      tW  /                                   //
	**********************************************************************************************/
	pub(crate) fn calculate_single_in_given_pool_out(
		token_balance_in: T::Balance,
		token_weight_in: T::Weight,
		token_total_weight: T::Weight,
		pool_amount_out: T::Balance,
		pool_supply: T::Balance,
		swap_fee: T::Fee
	) -> Result<T::Balance, Error<T>> {
		// type convert to u128
		let token_balance_in = TryInto::<u128>::try_into(token_balance_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_amount_out = TryInto::<u128>::try_into(pool_amount_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_supply = TryInto::<u128>::try_into(pool_supply).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee = TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;
		// to fixed num
		let token_balance_in = I64F64::from_num(token_balance_in);
		let pool_amount_out = FixedI128::<extra::U64>::from_num(pool_amount_out);
		let pool_supply = FixedI128::<extra::U64>::from_num(pool_supply);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get()).map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision)
		};

		let weight_ratio = Self::weight_ratio(token_weight_in, token_total_weight)?;

		let token_amount_in = {
			let base = pool_supply.saturating_add(pool_amount_out).saturating_div(pool_supply);
			let reversed_weight_ratio = Self::weight_ratio(token_total_weight, token_weight_in)?;
			let power: FixedI128::<extra::U64> = transcendental::pow(base, reversed_weight_ratio).map_err(|_| Error::<T>::FixedPointError)?;
			let upper = power.saturating_sub(FixedI128::<extra::U64>::from_num(1)).saturating_mul(token_balance_in);
			let down = FixedI128::<extra::U64>::from_num(1).saturating_sub(weight_ratio).saturating_mul(swap_fee);
			upper.saturating_div(down)
		};

		// convert to T::Balance
		let token_amount_in = u128::from_fixed(token_amount_in);

		TryInto::<T::Balance>::try_into(token_amount_in).map_err(|_| Error::<T>::ConvertFailure)
	}

	/**********************************************************************************************
	// calcSingleOutGivenPoolIn                                                                  //
	// tAo = tokenAmountOut            /      /                                             \\   //
	// bO = tokenBalanceOut           /      // pS - (pAi * (1 - eF)) \     /    1    \      \\  //
	// pAi = poolAmountIn            | bO - || ----------------------- | ^ | --------- | * b0 || //
	// ps = poolSupply                \      \\          pS           /     \(wO / tW)/      //  //
	// wI = tokenWeightIn      tAo =   \      \                                             //   //
	// tW = totalWeight                    /     /      wO \       \                             //
	// sF = swapFee                    *  | 1 - |  1 - ---- | * sF  |                            //
	// eF = exitFee                        \     \      tW /       /                             //
	**********************************************************************************************/
	pub(crate) fn calculate_single_out_given_pool_in(
		token_weight_in: T::Weight,
		pool_amount_in: T::Balance,
		token_total_weight: T::Weight,
		token_balance_out: T::Balance,
		pool_supply: T::Balance,
		swap_fee: T::Fee,
		exit_fee: T::Fee
	) -> Result<I64F64, Error<T>> {
		// type convert to u128
		let token_balance_out = TryInto::<u128>::try_into(token_balance_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_amount_in = TryInto::<u128>::try_into(pool_amount_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_supply = TryInto::<u128>::try_into(pool_supply).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee = TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;
		let exit_fee = TryInto::<u128>::try_into(exit_fee).map_err(|_| Error::<T>::ConvertFailure)?;
		// to fixed num
		let token_balance_out = I64F64::from_num(token_balance_out);
		let pool_amount_in = FixedI128::<extra::U64>::from_num(pool_amount_in);
		let pool_supply = FixedI128::<extra::U64>::from_num(pool_supply);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get()).map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision)
		};
		let exit_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get()).map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			FixedI128::<extra::U64>::from_num(exit_fee).saturating_div(precision)
		};

		let weight_ratio = Self::weight_ratio(token_weight_in, token_total_weight)?;
		let base = {
			let upper = pool_supply.saturating_sub(pool_amount_in.saturating_mul(FixedI128::<extra::U64>::from_num(1).saturating_sub(exit_fee)));
			upper.saturating_div(pool_supply)
		};
		let reversed_weight_ratio = Self::weight_ratio(token_total_weight, token_weight_in)?;
		let power: FixedI128::<extra::U64> = transcendental::pow(base, reversed_weight_ratio).map_err(|_| Error::<T>::FixedPointError)?;
		let lhs = token_balance_out.saturating_mul(FixedI128::<extra::U64>::from_num(1).saturating_sub(power));
		let rhs = {
			let fee = FixedI128::<extra::U64>::from_num(1).saturating_sub(weight_ratio).saturating_mul(swap_fee);
			FixedI128::<extra::U64>::from_num(1).saturating_sub(fee)
		};

		let token_amount_out = lhs.saturating_mul(rhs);
		Ok(token_amount_out)

		// convert to T::Balance
//		let token_amount_out = u128::from_fixed(token_amount_out);
//
//		TryInto::<T::Balance>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)
	}
}

mod weight_for {
	use frame_support::{traits::Get, weights::Weight};
	use super::Trait;

	/// add liquidity weight
	pub(crate) fn add_liquidity<T: Trait>() -> Weight {
		todo!();
	}
}

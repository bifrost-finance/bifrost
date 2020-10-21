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

// The swap pool algorithm implements Balancer protocol
// For more details, refer to https://balancer.finance/whitepaper/

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use core::convert::{From, Into, TryInto};
use core::ops::Div;
use fixed_point::{FixedI128, types::{*, extra}, transcendental, traits::FromFixed};
use frame_support::traits::{Get};
use frame_support::{weights::Weight,decl_event, decl_error, decl_module, decl_storage, ensure, Parameter, dispatch::DispatchResult, StorageValue};
use frame_system::{ensure_signed};
use node_primitives::{AssetTrait, TokenSymbol};
use sp_runtime::traits::{MaybeSerializeDeserialize, Member, Saturating, AtLeast32Bit, Zero};

mod mock;
mod tests;

pub trait WeightInfo{
	fn add_liquidity() -> Weight;
	fn add_single_liquidity() -> Weight;
	fn remove_single_asset_liquidity() -> Weight;
	fn remove_assets_liquidity() -> Weight;
	fn swap() -> Weight;
}

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
	type PoolWeight: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize+ Into<Self::Balance> + From<Self::Balance>;

	/// Some limitations on Balancer protocol
	type InitPoolSupply: Get<Self::Balance>;
	type MaximumSwapInRatio: Get<Self::Balance>;
	type MinimumBalance: Get<Self::Balance>;
	type MaximumSwapFee: Get<Self::Fee>;
	type MinimumSwapFee: Get<Self::Fee>;
	type FeePrecision: Get<Self::Balance>;

	/// Set default weight
	type WeightInfo : WeightInfo;
}

decl_event! {
	pub enum Event<T> where <T as Trait>::Balance, {
		AddLiquiditySuccess,
		RemoveLiquiditySuccess,
		AddSingleLiquiditySuccess,
		RemoveSingleLiquiditySuccess,
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
		/// Exceed too many amount of token, it should be trade_amount / all_amount <= 1 / 2
		ExceedMaximumSwapInRatio,
		/// Less than expected price while trading
		LessThanExpectedPrice,
		/// Bigger than expected price while trading
		BiggerThanExpectedPrice,
		/// Less than expected amount while trading
		LessThanExpectedAmount,
		/// Bigger than expected price while trading
		BiggerThanExpectedAmount,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Swap {
		/// Balancer pool token
		BalancerPoolToken get(fn all_pool_token) config(): T::Balance; // set pool token as 1000 by default

		/// Global pool, pool's details, like asset type, balance, weight, and value of function
		GlobalPool get(fn global_pool) config(): (Vec<(TokenSymbol, T::Balance, T::PoolWeight)>, T::InvariantValue);

		/// Each user details for pool
		UserPool get(fn user_pool) config(): map hasher(blake2_128_concat) T::AccountId => (Vec<(TokenSymbol, T::Balance)>, T::Balance);

		/// User may add a single asst to liquidity
		UserSinglePool: map hasher(blake2_128_concat) (T::AccountId, TokenSymbol) => (T::Balance, T::Balance);
		// (T::Balance, BalancerPoolToken)

		/// Now only support 7 tokens
		NumberOfSupportedTokens get(fn count_of_supported_tokens) config(): u8;

		/// Total weights
		TotalWeight get(fn get_total_weight) build(|config: &GenesisConfig<T>| {
			config.total_weight.iter().fold(Zero::zero(), |acc: T::PoolWeight, w| acc + *w)
		}): T::PoolWeight;

		/// Each token's weight
		TokenWeight get(fn token_weight): map hasher(blake2_128_concat) TokenSymbol => T::PoolWeight;

		/// Fee stuff
		LiquidityFee get(fn liquidity_fee): T::Fee = T::Fee::from(0); // now we don't charge fee on adding or removing liquidity
		SwapFee get(fn swap_fee) config(): T::Fee;
		ExitFee get(fn exit_fee) config(): T::Fee;

		/// shared fee pool
		SharedRewardPool get(fn shared_reward): Vec<(TokenSymbol, T::Balance)>;
	}
	add_extra_genesis {
		config(total_weight): Vec<T::PoolWeight>;
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
				<TokenWeight<T>>::insert(p.0, p.2);
			}
			// initialize a pool for user
			for (who, pool) in config.user_pool.iter() {
				<UserPool<T>>::insert(who, pool);
			}
			// initialize global pool
			<GlobalPool<T>>::put(&config.global_pool);

			// initialize reward pool
			let reward = vec![
				(TokenSymbol::aUSD, T::Balance::from(0)),
				(TokenSymbol::DOT, T::Balance::from(0)),
				(TokenSymbol::vDOT, T::Balance::from(0)),
				(TokenSymbol::KSM, T::Balance::from(0)),
				(TokenSymbol::vKSM, T::Balance::from(0)),
				(TokenSymbol::EOS, T::Balance::from(0)),
				(TokenSymbol::vEOS, T::Balance::from(0)),
			];
			<SharedRewardPool::<T>>::put(reward);
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const InitPoolSupply: T::Balance = T::InitPoolSupply::get();
		// when in a trade, trade_amount / all_amount <= 1 / 2
		const MaximumSwapInRatio: T::Balance = T::MaximumSwapInRatio::get();
		// when add liquidity, deposit this amount at least
		const MinimumBalance: T::Balance = T::MinimumBalance::get();
		const MaximumSwapFee: T::Fee = T::MaximumSwapFee::get();
		const MinimumSwapFee: T::Fee = T::MinimumSwapFee::get();
		const FeePrecision: T::Balance = T::FeePrecision::get();

		fn deposit_event() = default;

		// #[weight = weight_for::add_liquidity::<T>()]
		#[weight = T::WeightInfo::add_liquidity()]
		fn add_liquidity(
			origin,
			#[compact] new_pool_token: T::Balance
		) {
			let provider = ensure_signed(origin)?;

			// ensure new pool's balances bigger than MinimumBalance
			ensure!(new_pool_token >= T::MinimumBalance::get(), Error::<T>::LessThanMinimumBalance);

			// two times db reading
			let all_pool_tokens = BalancerPoolToken::<T>::get();
			let gpool = GlobalPool::<T>::get();

			// ensure this user have all kind of tokens and enough balance to deposit
			let mut new_user_pool = Vec::with_capacity(gpool.0.len());
			for p in gpool.0.iter() {
				// ensure user have the token
				ensure!(T::AssetTrait::token_exists(p.0), Error::<T>::TokenNotExist);

				let balances = T::AssetTrait::get_account_asset(p.0, &provider).balance;
				ensure!(balances.gt(&T::Balance::from(0)), Error::<T>::NotEnoughBalance);
				// about the algorithm: https://balancer.finance/whitepaper/#all-asset-depositwithdrawal
				let need_deposited = new_pool_token.saturating_mul(balances) / all_pool_tokens; // todo, div may lose precision
				// ensure user have enough token to deposit to this pool
				ensure!(balances >= need_deposited, Error::<T>::NotEnoughBalance);
				new_user_pool.push((p.0, need_deposited));
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
					for (p, n) in pool.0.iter_mut().zip(new_user_pool.iter()) {
						p.1 = p.1.saturating_add(n.1);
					}
				});
			}

			// destroy token from user's assets
			for p in new_user_pool.iter() {
				T::AssetTrait::asset_redeem(p.0, &provider, p.1);
			}

			// update whole pool token
			BalancerPoolToken::<T>::mutate(|pool_token| {
				*pool_token = pool_token.saturating_add(new_pool_token);
			});

			// update global pool
			GlobalPool::<T>::mutate(|pool| {
				for (p, n) in pool.0.iter_mut().zip(new_user_pool.iter()) {
					p.1 = p.1.saturating_add(n.1);
				}
			});

			Self::deposit_event(RawEvent::AddLiquiditySuccess);
		}

		// #[weight = T::DbWeight::get().reads_writes(1, 1)]
		#[weight = T::WeightInfo::add_single_liquidity()]
		fn add_single_liquidity(
			origin,
			token_symbol: TokenSymbol,
			#[compact] token_amount_in: T::Balance,
		) -> DispatchResult {
			let provider = ensure_signed(origin)?;

			// ensure user have token
			ensure!(T::AssetTrait::token_exists(token_symbol), Error::<T>::TokenNotExist);

			let balances = T::AssetTrait::get_account_asset(token_symbol, &provider).balance;
			// ensure this use have enough balanes to deposit
			ensure!(balances.gt(&T::Balance::from(0)), Error::<T>::NotEnoughBalance);
			ensure!(balances >= token_amount_in, Error::<T>::NotEnoughBalance);

			// get current token balance and weight in the pool
			let (token_balance_in, token_weight_in) = {
				let whole_pool = GlobalPool::<T>::get();
				let mut token_balance_in = 0.into();
				let mut token_weight_in = 0.into();
				for p in whole_pool.0.iter() {
					if token_symbol == p.0 {
						token_balance_in = p.1;
						token_weight_in = p.2;
						break;
					}
				}
				(token_balance_in, token_weight_in)
			};

			let pool_supply = BalancerPoolToken::<T>::get();
			let total_weight = TotalWeight::<T>::get();
			let swap_fee = LiquidityFee::<T>::get();

			// caculate how many pool token will be issued to user
			let new_pool_token = {
				let issued_pool_token = Self::calculate_pool_out_given_single_in(token_balance_in, token_weight_in, token_amount_in, total_weight, pool_supply, swap_fee)?;
				let pool_token_issued = u128::from_fixed(issued_pool_token);
				TryInto::<T::Balance>::try_into(pool_token_issued).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// first time to add liquidity
			if !UserSinglePool::<T>::contains_key((&provider, token_symbol)) {
				// add it to user's single pool
				UserSinglePool::<T>::insert(
					(&provider, token_symbol),
					(token_amount_in, new_pool_token)
				);
			} else {
				// add more liquidity to current single pool
				UserSinglePool::<T>::mutate((&provider, token_symbol), |pool| {
					pool.0 = pool.0.saturating_add(token_amount_in);
					pool.1 = pool.1.saturating_add(new_pool_token);
				});
			}

			// update whole pool
			BalancerPoolToken::<T>::mutate(|pool_token| {
				*pool_token = pool_token.saturating_add(new_pool_token);
			});

			// update global pool
			GlobalPool::<T>::mutate(|pool| {
				for p in pool.0.iter_mut() {
					if token_symbol == p.0 {
						p.1 = p.1.saturating_add(token_amount_in);
					}
				}
			});

			// destroy token from user
			T::AssetTrait::asset_redeem(token_symbol, &provider, token_amount_in);

			Self::deposit_event(RawEvent::AddSingleLiquiditySuccess);
			Ok(())
		}

		// #[weight = T::DbWeight::get().reads_writes(1, 1)]
		#[weight = T::WeightInfo::remove_single_asset_liquidity()]
		fn remove_single_asset_liquidity(
			origin,
			token_symbol: TokenSymbol,
			#[compact] pool_token_in: T::Balance
		) -> DispatchResult {
			let remover = ensure_signed(origin)?;

			// ensure this user has the pool
			ensure!(
				UserSinglePool::<T>::contains_key((&remover, token_symbol)),
				Error::<T>::NotExistedCurrentSinglePool
			);

			// ensure user doesn't redeem exceed all he has
			let user_single_pool = UserSinglePool::<T>::get((&remover, token_symbol));
			ensure!(user_single_pool.1 >= pool_token_in, Error::<T>::NotEnoughBalance);

			let whole_pool = BalancerPoolToken::<T>::get();
			ensure!(whole_pool >= pool_token_in, Error::<T>::NotEnoughBalance);

			let total_weight = TotalWeight::<T>::get();
			let swap_fee = LiquidityFee::<T>::get();
			let exit_fee = ExitFee::<T>::get();

			// get token's weight
			let (token_weight, pool_token) = {
				let mut weight = T::PoolWeight::from(0);
				let mut pool_token = T::Balance::from(0);
				for pool in GlobalPool::<T>::get().0.iter() {
					if token_symbol == pool.0 {
						weight = pool.2;
						pool_token = pool.1;
						break;
					}
				}
				(weight, pool_token)
			};

			// calculate how many balance user will get
			let token_amount = {
				let pool_supply = BalancerPoolToken::<T>::get();
				let token_amount_out = Self::calculate_single_out_given_pool_in(token_weight, pool_token_in, total_weight, pool_token, pool_supply, swap_fee, exit_fee)?;
				let token_amount_out = u128::from_fixed(token_amount_out);

				TryInto::<T::Balance>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?
			};

			let mut redeemed_reward: T::Balance = 0.into();
			SharedRewardPool::<T>::mutate(|reward| {
				for r in reward.iter_mut() {
					if token_symbol == r.0 {
						redeemed_reward = r.1.saturating_mul(pool_token_in) / whole_pool;
						r.1 = r.1.saturating_sub(redeemed_reward);
					}
				}
			});

			// update user asset
			T::AssetTrait::asset_issue(token_symbol, &remover, token_amount.saturating_add(redeemed_reward));
			// update user's pool
			UserSinglePool::<T>::mutate((&remover, token_symbol), |pool| {
				pool.0 = pool.0.saturating_sub(token_amount);
				pool.1 = pool.1.saturating_sub(pool_token_in);
			});
			// update whole pool token
			BalancerPoolToken::<T>::mutate(|pool| {
				*pool = pool.saturating_sub(pool_token_in);
			});

			// update global pool
			GlobalPool::<T>::mutate(|pool| {
				for p in pool.0.iter_mut() {
					if token_symbol == p.0 {
						p.1 = p.1.saturating_sub(token_amount);
					}
				}
			});

			Self::deposit_event(RawEvent::RemoveSingleLiquiditySuccess);

			Ok(())
		}

		// #[weight = T::DbWeight::get().reads_writes(1, 1)]
		#[weight = T::WeightInfo::remove_assets_liquidity()]
		fn remove_assets_liquidity(
			origin,
			#[compact] pool_amount_in: T::Balance
		) {
			let remover = ensure_signed(origin)?;

			// ensure this user have the pool
			ensure!(UserPool::<T>::contains_key(&remover), Error::<T>::NotExistedCurrentPool);

			let whole_pool = BalancerPoolToken::<T>::get();
			ensure!(whole_pool >= pool_amount_in, Error::<T>::NotEnoughBalance);

			let user_pool = UserPool::<T>::get(&remover);
			// ensure user doesn't redeem too many
			ensure!(user_pool.1 >= pool_amount_in, Error::<T>::NotEnoughBalance);

			let mut redeemed_pool = Vec::with_capacity(user_pool.0.len());
			for p in user_pool.0.iter() {
				let to_redeem =  p.1.saturating_mul(pool_amount_in) / user_pool.1;
				ensure!(to_redeem <= p.1, Error::<T>::NotEnoughBalance);
				redeemed_pool.push((p.0, to_redeem));
			}

			// update user pool
			UserPool::<T>::mutate(&remover, |pool| {
				pool.1 = pool.1.saturating_sub(pool_amount_in);
				for (p, r) in pool.0.iter_mut().zip(redeemed_pool.iter()) {
					p.1 = p.1.saturating_sub(r.1);
				}
			});

			// update whole pool
			BalancerPoolToken::<T>::mutate(|pool| {
				*pool = pool.saturating_sub(pool_amount_in);
			});

			// redeem assets
			let mut redeemed_rewards = Vec::with_capacity(redeemed_pool.len());
			SharedRewardPool::<T>::mutate(|reward| {
				for r in reward.iter_mut() {
					let redeemed_reward = r.1.saturating_mul(pool_amount_in) / whole_pool;
					redeemed_rewards.push((r.0, redeemed_reward));
					r.1 = r.1.saturating_sub(redeemed_reward);
				}
			});

			for (p, r) in redeemed_pool.iter().zip(redeemed_rewards.iter()) {
				T::AssetTrait::asset_issue(p.0, &remover, p.1.saturating_add(r.1));
			}

			// update global pool
			GlobalPool::<T>::mutate(|pool| {
				for (p, r) in pool.0.iter_mut().zip(redeemed_pool.iter()) {
					p.1 = p.1.saturating_sub(r.1);
				}
			});

			Self::deposit_event(RawEvent::RemoveLiquiditySuccess);
		}

		// consider maxPrice and minAmountOut
		// #[weight = T::DbWeight::get().reads_writes(1, 1)]
		#[weight = T::WeightInfo::swap()]
		fn swap(
			origin,
			token_in_type: TokenSymbol,
			#[compact]token_amount_in: T::Balance,
			min_token_amount_out: Option<T::Balance>,
			token_out_type: TokenSymbol,
			max_price: Option<T::Balance>
		) -> DispatchResult {
			let swaper = ensure_signed(origin)?;

			// ensure token symbol is different
			ensure!(token_in_type != token_out_type, Error::<T>::ForbidSameTokenSwap);

			let balances = T::AssetTrait::get_account_asset(token_in_type, &swaper).balance;
			// ensure this use have enough balanes to deposit
			ensure!(balances.ge(&token_amount_in), Error::<T>::NotEnoughBalance);
			// trade less half of balances
			ensure!(balances.div(token_amount_in) >= T::MaximumSwapInRatio::get(), Error::<T>::ExceedMaximumSwapInRatio);

//			let swaper_pool = UserPool::<T>::get(&swaper);
//			let total_weight = TotalWeight::<T>::get();
			let swap_fee = SwapFee::<T>::get();

			let charged_fee = Self::calculate_charged_swap_fee(token_amount_in, swap_fee);

			let ((token_balance_in, token_weight_in), (token_balance_out, token_weight_out)) = {
				let mut weight_in = T::PoolWeight::from(0);
				let mut weight_out = T::PoolWeight::from(0);
				let mut token_balance_in = T::Balance::from(0);
				let mut token_balance_out = T::Balance::from(0);
				for pool in GlobalPool::<T>::get().0.iter() {
					if token_in_type == pool.0 {
						weight_in = pool.2;
						token_balance_in = pool.1;
					}
					if token_out_type == pool.0 {
						weight_out = pool.2;
						token_balance_out = pool.1;
					}
				}
				((token_balance_in, weight_in), (token_balance_out, weight_out))
			};

			let spot_price_before = {
				let price = Self::calculate_spot_price(token_balance_in, token_weight_in, token_balance_out, token_weight_out, swap_fee)?;
				Self::convert_float(price)?
			};

			// compare spot price before do a swap
			if max_price.is_some() {
				ensure!(Some(spot_price_before) <= max_price, Error::<T>::BiggerThanExpectedPrice);
			}

			// do a swap
			let token_amount_out = {
				let fixed_token_amount_out = Self::calculate_out_given_in(
					token_balance_in,
					token_weight_in,
					token_amount_in,
					token_balance_out,
					token_weight_out,
					swap_fee
				)?;
				let token_amount_out = u128::from_fixed(fixed_token_amount_out);

				TryInto::<T::Balance>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// ensure token_amount_in is bigger than you exepect
			if min_token_amount_out.is_some() {
				ensure!(Some(token_amount_out) >= min_token_amount_out, Error::<T>::LessThanExpectedAmount);
			}

			// spot price before do a swap
			let spot_price_after = {
				let price = Self::calculate_spot_price(
					token_balance_in.saturating_add(token_amount_in),
					token_weight_in,
					token_balance_out.saturating_sub(token_amount_out),
					token_weight_out,
					swap_fee
				)?;
				Self::convert_float(price)?
			};
			if max_price.is_some() {
				ensure!(Some(spot_price_after) >= max_price, Error::<T>::LessThanExpectedPrice);
				ensure!(spot_price_before <= spot_price_after, "The Price should rise after trade");
				ensure!(spot_price_before <= token_amount_in.div(token_amount_out), "todo, what does it means");
			}

			// update global pool
			GlobalPool::<T>::mutate(|pool| {
				for p in pool.0.iter_mut() {
					if token_in_type == p.0 {
						p.1 = p.1.saturating_add(token_amount_in);
					}
					if token_out_type == p.0 {
						p.1 = p.1.saturating_sub(token_amount_out);
					}
				}
			});

			// update reward pool
			SharedRewardPool::<T>::mutate(|reward| {
				for r in reward.iter_mut() {
					if token_in_type == r.0 {
						r.1 = r.1.saturating_add(charged_fee);
					}
				}
			});

			// update user pool
			UserPool::<T>::mutate(&swaper, |pool| {
				for p in pool.0.iter_mut() {
					if token_in_type == p.0 {
						p.1 = p.1.saturating_add(token_amount_in);
					}
					if token_out_type == p.0 {
						p.1 = p.1.saturating_sub(token_amount_out);
					}
				}
			});

			// destroy token from user
			T::AssetTrait::asset_redeem(token_in_type, &swaper, token_amount_in);
			// what you get
			T::AssetTrait::asset_issue(token_out_type, &swaper, token_amount_out);

			Self::deposit_event(RawEvent::SwapTokenSuccess(token_amount_in, token_amount_out));

			Ok(())
		}
	}
}

#[allow(dead_code)]
impl<T: Trait> Module<T> {
	pub(crate) fn convert_float(input: I64F64) -> Result<T::Balance, Error<T>> {
		let converted = u128::from_fixed(input);
		TryInto::<T::Balance>::try_into(converted).map_err(|_| Error::<T>::ConvertFailure)
	}

	#[allow(dead_code)]
	pub(crate) fn calculate_charged_swap_fee(balance: T::Balance, swap_fee: T::Fee) -> T::Balance {
		let swap_fee: T::Balance = swap_fee.into();
		balance.saturating_mul(swap_fee) / T::FeePrecision::get()
	}

	#[allow(dead_code)]
	pub(crate) fn single_liquidity_charged_fee(balance: T::Balance, token_weight: T::PoolWeight, total_weight: T::PoolWeight, swap_fee: T::Fee) ->(T::Balance, T::Balance) {
		let charged_fee = {
			let token_weight: T::Balance = token_weight.into();
			let total_weight: T::Balance = total_weight.into();
			let swap_fee: T::Balance = swap_fee.into();
			let proportion = balance.saturating_mul(total_weight.saturating_sub(token_weight)) / total_weight;
			proportion.saturating_mul(swap_fee) / T::FeePrecision::get()
		};
		(balance, balance.saturating_mul(charged_fee))
	}

	pub(crate) fn total_weight(pool: &[(T::AssetId, T::Balance, T::PoolWeight)]) -> T::PoolWeight {
		pool.iter().fold(0.into(), |acc, v| acc + v.2)
	}

	pub(crate) fn weight_ratio(upper: T::PoolWeight, down: T::PoolWeight) -> Result<FixedI128<extra::U64>, Error<T>> {
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
	#[allow(dead_code)]
	pub(crate) fn value_function(pool: &[(T::AssetId, T::Balance, T::PoolWeight)]) -> Result<T::InvariantValue, Error<T>> {
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
		token_weight_in: T::PoolWeight,
		token_balance_out: T::Balance,
		token_weight_out: T::PoolWeight,
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
		token_weight_in: T::PoolWeight,
		token_balance_out: T::Balance,
		token_weight_out: T::PoolWeight,
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
		token_weight_in: T::PoolWeight,
		token_amount_in: T::Balance,
		token_balance_out: T::Balance,
		token_weight_out: T::PoolWeight,
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
		token_weight_in: T::PoolWeight,
		token_amount_in: T::Balance,
		token_total_weight: T::PoolWeight,
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
		token_weight_in: T::PoolWeight,
		token_total_weight: T::PoolWeight,
		pool_amount_out: T::Balance,
		pool_supply: T::Balance,
		swap_fee: T::Fee
	) -> Result<I64F64, Error<T>> {
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

		Ok(token_amount_in)
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
		token_weight_in: T::PoolWeight,
		pool_amount_in: T::Balance,
		token_total_weight: T::PoolWeight,
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
	}
}

// #[allow(dead_code)]
// mod weight_for {
// 	use frame_support::{traits::Get, weights::Weight};
// 	use super::Trait;
//
// 	/// add liquidity weight
// 	pub(crate) fn add_liquidity<T: Trait>() -> Weight {
// 		let reads_writes = T::DbWeight::get().reads_writes(1, 1);
// 		reads_writes * 1000 as Weight
// 	}
//
// 	/// add single liquidity
// 	pub(crate) fn add_single_liquidity<T: Trait>() -> Weight {
// 		todo!();
// 	}
// }

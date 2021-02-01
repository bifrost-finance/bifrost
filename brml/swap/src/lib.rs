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

// The swap pool algorithm implements Balancer protocol
// For more details, refer to https://balancer.finance/whitepaper/

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::collections::btree_map::BTreeMap;
use alloc::fmt::Debug;
use alloc::vec::Vec;
use codec::{Decode, Encode};
use core::convert::{From, Into, TryInto};
use core::ops::Div;
use fixed_point::{
	traits::FromFixed,
	transcendental,
	types::{extra, *},
	FixedI128,
};
use frame_support::traits::Get;
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure, Parameter,
};
use frame_system::ensure_signed;
use node_primitives::AssetTrait;
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Saturating, Zero};

mod mock;
mod tests;

pub trait Config: frame_system::Config {
	/// event
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// fee
	type SwapFee: Member
		+ Parameter
		+ AtLeast32Bit
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ Into<Self::Balance>
		+ From<Self::Balance>;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// Pool Id
	type PoolId: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// AssetTrait to handle assets
	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance>;

	/// Weight
	type PoolWeight: Member
		+ Parameter
		+ AtLeast32Bit
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ Into<Self::Balance>
		+ From<Self::Balance>;

	/// Weight
	type PoolToken: Member
		+ Parameter
		+ AtLeast32Bit
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ Into<Self::Balance>
		+ From<Self::Balance>;

	/// Some limitations on Balancer protocol

	/// when in a trade, trade_amount / all_amount <= 1 / 2. MaximumSwapInRatio keeps a number of 2.
	/// Reverse it to be 1/2 when using it.
	type MaximumSwapInRatio: Get<u8>;

	/// when adding liquidity, deposit at least this amount of pool token shares
	type MinimumPassedInPoolTokenShares: Get<Self::PoolToken>;

	/// Minimum percentage of fee that the pool owner can set.
	type MinimumSwapFee: Get<Self::SwapFee>;

	/// Maximum percentage of fee that the pool owner can set.
	type MaximumSwapFee: Get<Self::SwapFee>;

	/// Used to calculate fee rate to prevent precision lost in float type.
	type FeePrecision: Get<Self::SwapFee>;

	/// Used to calculate weight in percentage to prevent precision lost in float type.
	type WeightPrecision: Get<Self::PoolWeight>;

	/// The up-limit of tokens supported.
	type BNCAssetId: Get<Self::AssetId>;

	/// the asset id of BNC
	type InitialPoolSupply: Get<Self::PoolToken>;

	/// The max age denominator used in calculating unclaimed BNC bonus for liquidity providers.
	type NumberOfSupportedTokens: Get<u8>;

	/// the initial share for the pool creator.
	type BonusClaimAgeDenominator: Get<Self::BlockNumber>;

	/// passed in pool share should be no more than this number.
	type MaximumPassedInPoolTokenShares: Get<Self::PoolToken>;
}

decl_event! {
	pub enum Event<T> where <T as Config>::Balance, {
		AddLiquiditySuccess,
		RemoveLiquiditySuccess,
		AddSingleLiquiditySuccess,
		RemoveSingleLiquiditySuccess,
		SwapTokenSuccess(Balance, Balance),
		CreatePoolSuccess,
		SetPoolStatusSuccess,
		SetSwapFeeSuccess,
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		PoolNotExist,
		PoolNotActive,
		TokenNotExist,
		NotEnoughBalance,
		ConvertFailure,
		LessThanMinimumPassedInPoolTokenShares,
		TooManyTokensToPool,
		UserNotInThePool,
		ForbidSameTokenSwap,
		FixedPointError,
		ExceedMaximumSwapInRatio,
		LessThanExpectedAmount,
		BiggerThanExpectedAmount,
		AmountBelowZero,
		FeeRateExceedMinimumLimit,
		FeeRateExceedMaximumLimit,
		NotPoolOwner,
		MoreThanMaximumPassedInPoolTokenShares,
	}
}

/// struct for pool details
#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug, Copy)]
pub struct PoolDetails<AccountId, SwapFee> {
	///The owner of the pool, who has the privileges to set or change the parameters of the pool.
	owner: AccountId,
	/// The current swap rate of the pool.
	swap_fee_rate: SwapFee,
	/// Pool status. If is true, users can add liquidity into or swap in the pool.
	/// Otherwise, user operations will be prevented.
	active: bool,
}

/// struct for pool creating token info.
#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug, Copy)]
pub struct PoolCreateTokenDetails<AssetId, Balance, PoolWeight> {
	/// token asset id
	token_id: AssetId,
	/// token balance that the pool creator wants to deposit into the pool for the first time.
	token_balance: Balance,
	/// token weight that the pool creator wants to give to the token
	token_weight: PoolWeight,
}

decl_storage! {
	trait Store for Module<T: Config> as Swap {
		/// Pool info
		Pools get(fn pools): map hasher(blake2_128_concat) T::PoolId => PoolDetails<T::AccountId, T::SwapFee>;

		/// Token weights info for pools. Weights must be normalized at the beginning.
		/// Sum of all the token weights for a pool must be 1 * WeightPrecision. Should be ensured when set up the pool.
		TokenWeightsInPool get(fn token_weights_in_pool): double_map
			hasher(blake2_128_concat) T::PoolId,
			hasher(blake2_128_concat) T::AssetId
			=> T::PoolWeight;

		/// Token balance info for pools
		TokenBalancesInPool get(fn token_balances_in_pool): double_map
			hasher(blake2_128_concat) T::PoolId,
			hasher(blake2_128_concat) T::AssetId
			=> T::Balance;

		/// total pool tokens in pool.
		PoolTokensInPool get(fn pool_tokens_in_pool): map hasher(blake2_128_concat) T::PoolId => T::PoolToken;

		/// Users' pool tokens in different pools
		UserPoolTokensInPool get(fn user_pool_tokens_in_pool): double_map
			hasher(blake2_128_concat) T::AccountId,
			hasher(blake2_128_concat) T::PoolId
			=> T::PoolToken;

		/// Record user unclaimed liquidity bonus. There are two occasions that will trigger the calculation of unclaimed bonus:
		/// 1. The user adds or removes his liquidity to the pool.
		/// 2. The user claims his bonus.
		/// The value part of the map is a tuple contains (un_claimed_Bonus, last_calculation_block).
		/// "un_claimed_Bonus" shows the remaining unclaimed but calculated bonus balance.
		/// "last_calculation_block" records the block number of last time when liquidity bonus calculation is triggered.
		UserUnclaimedBonusInPool get(fn user_unclaimed_bonus_in_pool): double_map
			hasher(blake2_128_concat) T::AccountId,
			hasher(blake2_128_concat)  T::PoolId
			=> (T::Balance, T::BlockNumber);	// (un_claimed_Bonus, last_calculation_block)

		/// Record the calculated deducted BNC bonus amount for each pool,
		/// including deducted but unclaimed amount as well as claimed amount
		DeductedBonusAmountInPool get(fn deducted_bonus_amount_in_pool): map hasher(blake2_128_concat) T::PoolId => T::Balance;
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		const MaximumSwapInRatio: u8 = T::MaximumSwapInRatio::get();
		const MinimumPassedInPoolTokenShares: T::PoolToken = T::MinimumPassedInPoolTokenShares::get();
		const MinimumSwapFee: T::SwapFee = T::MinimumSwapFee::get();
		const MaximumSwapFee: T::SwapFee = T::MaximumSwapFee::get();
		const FeePrecision: T::SwapFee = T::FeePrecision::get();
		const WeightPrecision: T::PoolWeight = T::WeightPrecision::get();
		const NumberOfSupportedTokens: u8 = T::NumberOfSupportedTokens::get();
		const BNCAssetId: T::AssetId = T::BNCAssetId::get();
		const BonusClaimAgeDenominator: T::BlockNumber = T::BonusClaimAgeDenominator::get();
		const InitialPoolSupply: T::PoolToken = T::InitialPoolSupply::get();
		const MaximumPassedInPoolTokenShares: T::PoolToken = T::MaximumPassedInPoolTokenShares::get();

		fn deposit_event() = default;

		/// Add liquidity by providing all of the tokens in proportion.
		/// The user inputs a pool token share in the front end, and the front end will automatically calculate the
		/// amount of each asset that should be provided liquidity with.
		/// (add liquidity)(many assets) given share in => amount out
		#[weight = 1_000]
		fn add_liquidity_given_shares_in(
			origin,
			pool_id: T::PoolId,
			#[compact] new_pool_token: T::PoolToken,
		) {
			let provider = ensure_signed(origin)?;

			ensure!(Pools::<T>::contains_key(pool_id), Error::<T>::PoolNotExist);
			ensure!(Pools::<T>::get(pool_id).active, Error::<T>::PoolNotActive);
			ensure!(new_pool_token >= T::MinimumPassedInPoolTokenShares::get(), Error::<T>::LessThanMinimumPassedInPoolTokenShares);
			ensure!(new_pool_token <= T::MaximumPassedInPoolTokenShares::get(), Error::<T>::MoreThanMaximumPassedInPoolTokenShares);

			// get the iterator of the items(assetId => balance) with the same first key(pool_id)
			let token_balances_in_pool_iter = TokenBalancesInPool::<T>::iter_prefix(pool_id);
			// record how many tokens the user should deposit if he wants to acquire certain pool token share
			let mut user_should_deposit_tokens = BTreeMap::new();

			// calculate how many tokens for the user to deposit for each of the assets
			for tk in token_balances_in_pool_iter {  //0 position is assetId, 1 position is balance

				// get the total pool token shares for the specific pool
				let all_pool_tokens = PoolTokensInPool::<T>::get(pool_id);
				// Asset id
				let token_id = tk.0;
				// get the user's balance for a specific token
				let user_token_pool_balance = T::AssetTrait::get_account_asset(token_id, &provider).available;
				// the balance of a specific token in a pool
				let token_pool_balance = TokenBalancesInPool::<T>::get(pool_id, token_id);
				// the amount of the token that the user should deposit
				let should_deposit_amount = token_pool_balance.saturating_mul(new_pool_token.into()) / all_pool_tokens.into();

				// ensure the user has enough balances for all kinds of tokens in the pool
				ensure!(user_token_pool_balance >= should_deposit_amount, Error::<T>::NotEnoughBalance);
				// record the should-be-deposited amount each of the token
				user_should_deposit_tokens.insert(token_id, should_deposit_amount);
			}

			Self::revise_storages_except_token_balances_when_adding_liquidity(pool_id, new_pool_token, &provider)?;

			// issue new pool token to the user
			// updates all the token balances of each token in the pool, and destroy corresponding user balances
			for (tk, blc) in user_should_deposit_tokens.iter() {
				TokenBalancesInPool::<T>::mutate(pool_id, tk, |token_balance| {
					*token_balance = token_balance.saturating_add(*blc);
				});

				// destroy token from user's asset_redeem(assetId, &target, amount)
				T::AssetTrait::asset_redeem(*tk, &provider, *blc);
			}

			Self::deposit_event(RawEvent::AddLiquiditySuccess);
		}

		/// A user adds liquidity by depositing only one kind of token.
		/// So we need to calculate the corresponding pool token share the user should get.
		/// (add liquidity)(single asset) given amount in => share out
		#[weight = 1_000]
		fn add_single_liquidity_given_amount_in(
			origin,
			pool_id: T::PoolId,
			asset_id: T::AssetId,
			#[compact] token_amount_in: T::Balance,
		) -> DispatchResult {
			let provider = ensure_signed(origin)?;

			ensure!(T::AssetTrait::token_exists(asset_id), Error::<T>::TokenNotExist);
			ensure!(Pools::<T>::contains_key(pool_id), Error::<T>::PoolNotExist);
			ensure!(Pools::<T>::get(pool_id).active, Error::<T>::PoolNotActive);
			ensure!(token_amount_in > Zero::zero(), Error::<T>::AmountBelowZero);
			// get the user's balance for a specific token
			let user_token_balance = T::AssetTrait::get_account_asset(asset_id, &provider).available;
			ensure!(user_token_balance >= token_amount_in, Error::<T>::NotEnoughBalance);

			// calculate how many pool token will be issued to user
			let new_pool_token = {
				// get current token balance and weight in the pool
				let token_balance_in = TokenBalancesInPool::<T>::get(pool_id, asset_id);
				let token_weight_in = TokenWeightsInPool::<T>::get(pool_id, asset_id);
				// get the total pool token shares for the specific pool
				let pool_supply = PoolTokensInPool::<T>::get(pool_id);
				// get the swap fee rate of the pool
				let swap_fee_rate = Pools::<T>::get(pool_id).swap_fee_rate;
				let issued_pool_token = Self::calculate_pool_out_given_single_in(token_balance_in, token_weight_in,
					token_amount_in, pool_supply, swap_fee_rate)?;
				let pool_token_issued = u128::from_fixed(issued_pool_token);
				TryInto::<T::PoolToken>::try_into(pool_token_issued).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// Before revising storages, we should make sure the added pool token shares meet the minimum requirement.
			ensure!(new_pool_token >= T::MinimumPassedInPoolTokenShares::get(), Error::<T>::LessThanMinimumPassedInPoolTokenShares);

			Self::revise_storages_except_token_balances_when_adding_liquidity(pool_id, new_pool_token, &provider)?;

			// Updates the token balance that the user adds liquidity with in the pool
			TokenBalancesInPool::<T>::mutate(pool_id, asset_id, |token_balance| {
				*token_balance = token_balance.saturating_add(token_amount_in);
			});

			// destroy token from user's asset_redeem(asset_id, &target, amount)
			T::AssetTrait::asset_redeem(asset_id, &provider, token_amount_in);

			Self::deposit_event(RawEvent::AddSingleLiquiditySuccess);
			Ok(())
		}

		/// A user adds liquidity by depositing only one kind of token.
		/// So we need to calculate the corresponding pool token share the user should get.
		/// (add liquidity)(single asset) given share in => amount out
		#[weight = 1_000]
		fn add_single_liquidity_given_shares_in(
			origin,
			pool_id: T::PoolId,
			asset_id: T::AssetId,
			#[compact] new_pool_token: T::PoolToken,
		) -> DispatchResult {
			let provider = ensure_signed(origin)?;

			ensure!(T::AssetTrait::token_exists(asset_id), Error::<T>::TokenNotExist);
			ensure!(Pools::<T>::contains_key(pool_id), Error::<T>::PoolNotExist);
			ensure!(Pools::<T>::get(pool_id).active, Error::<T>::PoolNotActive);
			ensure!(new_pool_token >= T::MinimumPassedInPoolTokenShares::get(), Error::<T>::LessThanMinimumPassedInPoolTokenShares);
			ensure!(new_pool_token <= T::MaximumPassedInPoolTokenShares::get(), Error::<T>::MoreThanMaximumPassedInPoolTokenShares);

			// calculate how many token-in amount should the user provide to the pool to acquire the corresponding pool token shares.
			let token_amount_in = {
				// get current token balance and weight in the pool
				let token_balance_in = TokenBalancesInPool::<T>::get(pool_id, asset_id);
				let token_weight_in = TokenWeightsInPool::<T>::get(pool_id, asset_id);
				// get the total pool token shares for the specific pool
				let pool_supply = PoolTokensInPool::<T>::get(pool_id);
				// get the swap fee rate of the pool
				let swap_fee_rate = Pools::<T>::get(pool_id).swap_fee_rate;
				let should_token_amount_in = Self::calculate_single_in_given_pool_out(token_balance_in, token_weight_in,
					new_pool_token, pool_supply, swap_fee_rate)?;

				let should_token_amount_in = u128::from_fixed(should_token_amount_in);
				TryInto::<T::Balance>::try_into(should_token_amount_in).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// get the user's balance for a specific token
			let user_token_balance = T::AssetTrait::get_account_asset(asset_id, &provider).available;
			ensure!(user_token_balance >= token_amount_in, Error::<T>::NotEnoughBalance);

			Self::revise_storages_except_token_balances_when_adding_liquidity(pool_id, new_pool_token, &provider)?;

			// Updates the token balance that the user adds liquidity with in the pool
			TokenBalancesInPool::<T>::mutate(pool_id, asset_id, |token_balance| {
				*token_balance = token_balance.saturating_add(token_amount_in);
			});

			// destroy token from user's asset_redeem(asset_id, &target, amount)
			T::AssetTrait::asset_redeem(asset_id, &provider, token_amount_in);

			Self::deposit_event(RawEvent::AddSingleLiquiditySuccess);
			Ok(())
		}

		/// User remove liquidity with only one kind of token
		/// (remove liquidity)(single asset) given share in => amount out
		#[weight = 1_000]
		fn remove_single_asset_liquidity_given_shares_in(
			origin,
			pool_id: T::PoolId,
			asset_id: T::AssetId,
			#[compact] pool_token_out: T::PoolToken  // The pool token that the user want to remove liquidity with from the pool.
		) -> DispatchResult {
			let remover = ensure_signed(origin)?;

			ensure!(T::AssetTrait::token_exists(asset_id), Error::<T>::TokenNotExist);
			ensure!(Pools::<T>::contains_key(pool_id), Error::<T>::PoolNotExist);
			ensure!(Pools::<T>::get(pool_id).active, Error::<T>::PoolNotActive);
			ensure!(pool_token_out >= T::MinimumPassedInPoolTokenShares::get(), Error::<T>::LessThanMinimumPassedInPoolTokenShares);
			ensure!(UserPoolTokensInPool::<T>::contains_key(&remover, pool_id), Error::<T>::UserNotInThePool);
			ensure!(pool_token_out <= T::MaximumPassedInPoolTokenShares::get(), Error::<T>::MoreThanMaximumPassedInPoolTokenShares);
			ensure!(UserPoolTokensInPool::<T>::get(&remover, pool_id) >= pool_token_out, Error::<T>::NotEnoughBalance);

			// calculate how many balance user will get
			let token_amount = {
				// Pool swap fee rate, which is an integer, should be divided by rate precision when being used.
				let swap_fee_rate = Pools::<T>::get(pool_id).swap_fee_rate;
				// out-token's weight in the pool, which is an normalized integer,
				//should be divided by weight precision when being used.
				let out_token_weight = TokenWeightsInPool::<T>::get(pool_id, asset_id);
				// out-token's balance in the pool, which is the number of the specific token.
				let out_token_balance_in_pool = TokenBalancesInPool::<T>::get(pool_id, asset_id);
				// total pool token that the specific pool has issued.
				let pool_supply = PoolTokensInPool::<T>::get(pool_id);
				let token_amount_out = Self::calculate_single_out_given_pool_in(out_token_weight, pool_token_out,
					out_token_balance_in_pool, pool_supply, swap_fee_rate)?;
				let token_amount_out = u128::from_fixed(token_amount_out);

				TryInto::<T::Balance>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// update user asset
			T::AssetTrait::asset_issue(asset_id, &remover, token_amount);

			// update TokenBalancesInPool map.
			TokenBalancesInPool::<T>::mutate(pool_id, asset_id, |token_balance| {
				*token_balance = token_balance.saturating_sub(token_amount);
			});

			Self::revise_storages_except_token_balances_when_removing_liquidity(pool_id, pool_token_out, &remover)?;
			Self::deposit_event(RawEvent::RemoveSingleLiquiditySuccess);

			Ok(())
		}

		/// User remove liquidity with only one kind of token
		/// (remove liquidity)(single asset) given amount in => shares out
		#[weight = 1_000]
		fn remove_single_asset_liquidity_given_amount_in(
			origin,
			pool_id: T::PoolId,
			asset_id: T::AssetId,
			#[compact] token_amount: T::Balance  // The number of out-token that the user want to remove liquidity with from the pool.
		) -> DispatchResult {
			let remover = ensure_signed(origin)?;
			// out-token's balance in the pool, which is the number of the specific token.
			let out_token_balance_in_pool = TokenBalancesInPool::<T>::get(pool_id, asset_id);

			ensure!(T::AssetTrait::token_exists(asset_id), Error::<T>::TokenNotExist);
			ensure!(Pools::<T>::contains_key(pool_id), Error::<T>::PoolNotExist);
			ensure!(Pools::<T>::get(pool_id).active, Error::<T>::PoolNotActive);
			ensure!(token_amount > Zero::zero(), Error::<T>::AmountBelowZero);
			ensure!(token_amount < out_token_balance_in_pool, Error::<T>::NotEnoughBalance);
			ensure!(UserPoolTokensInPool::<T>::contains_key(&remover, pool_id), Error::<T>::UserNotInThePool);

			// calculate how many pool tokens that the user wants to remove liquidity with
			let pool_token_out = {
				// Pool swap fee rate, which is an integer, should be divided by rate precision when being used.
				let swap_fee_rate = Pools::<T>::get(pool_id).swap_fee_rate;
				// out-token's weight in the pool, which is an normalized integer,
				// should be divided by weight precision when being used.
				let out_token_weight = TokenWeightsInPool::<T>::get(pool_id, asset_id);
				// total pool token that the specific pool has issued.
				let pool_supply = PoolTokensInPool::<T>::get(pool_id);

				let pool_token_out = Self::calculate_pool_in_given_single_out(out_token_weight, token_amount,
					out_token_balance_in_pool, pool_supply, swap_fee_rate)?;
				let pool_token_out = u128::from_fixed(pool_token_out);

				TryInto::<T::PoolToken>::try_into(pool_token_out).map_err(|_| Error::<T>::ConvertFailure)?
			};

			ensure!(UserPoolTokensInPool::<T>::get(&remover, pool_id) >= pool_token_out, Error::<T>::NotEnoughBalance);
			ensure!(pool_token_out >= T::MinimumPassedInPoolTokenShares::get(), Error::<T>::LessThanMinimumPassedInPoolTokenShares);

			// update user asset
			T::AssetTrait::asset_issue(asset_id, &remover, token_amount);

			// update TokenBalancesInPool map.
			TokenBalancesInPool::<T>::mutate(pool_id, asset_id, |token_balance| {
				*token_balance = token_balance.saturating_sub(token_amount);
			});

			Self::revise_storages_except_token_balances_when_removing_liquidity(pool_id, pool_token_out, &remover)?;
			Self::deposit_event(RawEvent::RemoveSingleLiquiditySuccess);

			Ok(())
		}

		/// User removes all the tokens in the pool in proportion of his pool token shares.
		/// (remove liquidity)(many assets) given share in => amount out
		#[weight = 1_000]
		fn remove_assets_liquidity_given_shares_in(
			origin,
			pool_id: T::PoolId,
			#[compact] pool_amount_out: T::PoolToken
		) {
			let remover = ensure_signed(origin)?;

			ensure!(Pools::<T>::contains_key(pool_id), Error::<T>::PoolNotExist);
			ensure!(Pools::<T>::get(pool_id).active, Error::<T>::PoolNotActive);
			ensure!(UserPoolTokensInPool::<T>::contains_key(&remover, pool_id), Error::<T>::UserNotInThePool);
			ensure!(pool_amount_out >= T::MinimumPassedInPoolTokenShares::get(), Error::<T>::LessThanMinimumPassedInPoolTokenShares);

			let user_pool_token_in_pool = UserPoolTokensInPool::<T>::get(remover.clone(), pool_id);
			ensure!(pool_amount_out <= user_pool_token_in_pool, Error::<T>::NotEnoughBalance);
			// get the iterator of the items(asset_id => balance) with the same first key(pool_id)
			let token_balances_in_pool_iter = TokenBalancesInPool::<T>::iter_prefix(pool_id);
			// calculate how many tokens for each of the assets that user can withdraw.
			// Meanwhile, issue money to user's account and deducted from the pool.
			for tk in token_balances_in_pool_iter {  //0 position is asset_id, 1 position is balance
				// get the total pool token shares for the specific pool
				let all_pool_tokens = PoolTokensInPool::<T>::get(pool_id);
				// Asset id
				let token_id = tk.0;
				// the balance of a specific token in a pool
				let token_pool_balance = TokenBalancesInPool::<T>::get(pool_id, token_id);
				// the amount of the token that the user should deposit
				let can_withdraw_amount = token_pool_balance.saturating_mul(pool_amount_out.into()) / all_pool_tokens.into();
				// issue money to user's account
				T::AssetTrait::asset_issue(token_id, &remover, can_withdraw_amount);

				// deduct the corresponding token balance in the pool
				TokenBalancesInPool::<T>::mutate(pool_id, token_id, |token_balance| {
					*token_balance = token_balance.saturating_sub(can_withdraw_amount);
				});
			}

			Self::revise_storages_except_token_balances_when_removing_liquidity(pool_id, pool_amount_out, &remover)?;
			Self::deposit_event(RawEvent::RemoveLiquiditySuccess);
		}

		// ****************************************************************************
		/// User swap one token for another kind of token, given an exact amount for token-in.
		#[weight = 1_000]
		fn swap_exact_in(
			origin,
			pool_id: T::PoolId,
			token_in_asset_id: T::AssetId,
			#[compact]token_amount_in: T::Balance, // the input token amount that the user is willing to pay.
			min_token_amount_out: Option<T::Balance>,  // The least output token amount that the user can accept
			token_out_asset_id: T::AssetId,
		) -> DispatchResult {
			let swapper = ensure_signed(origin)?;

			ensure!(token_in_asset_id != token_out_asset_id, Error::<T>::ForbidSameTokenSwap);
			ensure!(T::AssetTrait::token_exists(token_in_asset_id), Error::<T>::TokenNotExist);
			ensure!(T::AssetTrait::token_exists(token_out_asset_id), Error::<T>::TokenNotExist);
			ensure!(Pools::<T>::contains_key(pool_id), Error::<T>::PoolNotExist);
			ensure!(Pools::<T>::get(pool_id).active, Error::<T>::PoolNotActive);
			// get the user's balance for a specific token
			let user_token_balance = T::AssetTrait::get_account_asset(token_in_asset_id, &swapper).available;
			ensure!(user_token_balance >= token_amount_in, Error::<T>::NotEnoughBalance);

			// get the total token-in token amount for the specific pool
			let token_in_pool_amount = TokenBalancesInPool::<T>::get(pool_id, token_in_asset_id);
			// MaximumSwapInRatio is a reverse number.(2 => 1/2), trade less half of pool balances.
			ensure!(token_in_pool_amount.div(token_amount_in) >= T::MaximumSwapInRatio::get().into(), Error::<T>::ExceedMaximumSwapInRatio);

			// do a swap
			let token_amount_out = {
				// get the total token-out token amount for the specific pool
				let token_out_pool_amount = TokenBalancesInPool::<T>::get(pool_id, token_out_asset_id);
				// The normalized weight of the token-in in the pool.
				let weight_in = TokenWeightsInPool::<T>::get(pool_id, token_in_asset_id);
				// The normalized weight of the token-out in the pool.
				let weight_out = TokenWeightsInPool::<T>::get(pool_id, token_out_asset_id);
				// Pool swap fee rate, which is an integer, should be divided by rate precision when being used.
				let swap_fee_rate = Pools::<T>::get(pool_id).swap_fee_rate;

				let fixed_token_amount_out = Self::calculate_out_given_in(token_in_pool_amount, weight_in,
					token_amount_in, token_out_pool_amount, weight_out, swap_fee_rate)?;
				let token_amount_out = u128::from_fixed(fixed_token_amount_out);

				TryInto::<T::Balance>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// ensure token_amount_in is bigger than you expect
			if min_token_amount_out.is_some() {
				ensure!(Some(token_amount_out) >= min_token_amount_out, Error::<T>::LessThanExpectedAmount);
			}

			// deducted token-in amount from the user account
			T::AssetTrait::asset_redeem(token_in_asset_id, &swapper, token_amount_in);
			// add up token-out amount to the user account
			T::AssetTrait::asset_issue(token_out_asset_id, &swapper, token_amount_out);

			// update the token-in amount in the pool
			TokenBalancesInPool::<T>::mutate(pool_id, token_in_asset_id, |token_balance| {
				*token_balance = token_balance.saturating_add(token_amount_in);
			});

			// update the token-out amount in the pool
			TokenBalancesInPool::<T>::mutate(pool_id, token_out_asset_id, |token_balance| {
				*token_balance = token_balance.saturating_sub(token_amount_out);
			});

			Self::deposit_event(RawEvent::SwapTokenSuccess(token_amount_in, token_amount_out));

			Ok(())
		}

		/// User swap one token for another kind of token, given an exact amount for token-out.
		#[weight = 1_000]
		fn swap_exact_out(
			origin,
			pool_id: T::PoolId,
			token_out_asset_id: T::AssetId,
			#[compact]token_amount_out: T::Balance, // the out token amount that the user wants to get.
			max_token_amount_in: Option<T::Balance>,  // most input token amount user can accept to get token amount out.
			token_in_asset_id: T::AssetId,
		) -> DispatchResult {
			let swapper = ensure_signed(origin)?;
			ensure!(token_in_asset_id != token_out_asset_id, Error::<T>::ForbidSameTokenSwap);
			ensure!(T::AssetTrait::token_exists(token_in_asset_id), Error::<T>::TokenNotExist);
			ensure!(T::AssetTrait::token_exists(token_out_asset_id), Error::<T>::TokenNotExist);
			ensure!(Pools::<T>::contains_key(pool_id), Error::<T>::PoolNotExist);
			ensure!(Pools::<T>::get(pool_id).active, Error::<T>::PoolNotActive);

			// get the total token-out token amount for the specific pool
			let token_out_pool_amount = TokenBalancesInPool::<T>::get(pool_id, token_out_asset_id);
			// MaximumSwapInRatio is a reverse number.(2 => 1/2), trade less half of pool balances.
			ensure!(token_out_pool_amount >= token_amount_out.saturating_mul(T::MaximumSwapInRatio::get().into()), Error::<T>::ExceedMaximumSwapInRatio);

			// do a swap
			let token_amount_in = {
				// get the total token-in token amount for the specific pool
				let token_in_pool_amount = TokenBalancesInPool::<T>::get(pool_id, token_in_asset_id);
				// The normalized weight of the token-in in the pool.
				let weight_in = TokenWeightsInPool::<T>::get(pool_id, token_in_asset_id);
				// The normalized weight of the token-out in the pool.
				let weight_out = TokenWeightsInPool::<T>::get(pool_id, token_out_asset_id);
				// Pool swap fee rate, which is an integer, should be divided by rate precision when being used.
				let swap_fee_rate = Pools::<T>::get(pool_id).swap_fee_rate;

				let fixed_token_amount_in = Self::calculate_in_given_out(token_in_pool_amount, weight_in,
					token_out_pool_amount, weight_out, token_amount_out, swap_fee_rate)?;

				let token_amount_in = u128::from_fixed(fixed_token_amount_in);
				TryInto::<T::Balance>::try_into(token_amount_in).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// ensure calculated token_amount_in is smaller than you expect
			if max_token_amount_in.is_some() {
				ensure!(Some(token_amount_in) <= max_token_amount_in, Error::<T>::BiggerThanExpectedAmount);
			}

			// get the user's balance for a specific token
			let user_token_balance = T::AssetTrait::get_account_asset(token_in_asset_id, &swapper).available;
			ensure!(user_token_balance >= token_amount_in, Error::<T>::NotEnoughBalance);

			// deducted token-in amount from the user account
			T::AssetTrait::asset_redeem(token_in_asset_id, &swapper, token_amount_in);
			// add up token-out amount to the user account
			T::AssetTrait::asset_issue(token_out_asset_id, &swapper, token_amount_out);

			// update the token-in amount in the pool
			TokenBalancesInPool::<T>::mutate(pool_id, token_in_asset_id, |token_balance| {
				*token_balance = token_balance.saturating_add(token_amount_in);
			});

			// update the token-out amount in the pool
			TokenBalancesInPool::<T>::mutate(pool_id, token_out_asset_id, |token_balance| {
				*token_balance = token_balance.saturating_sub(token_amount_out);
			});

			Self::deposit_event(RawEvent::SwapTokenSuccess(token_amount_in, token_amount_out));

			Ok(())
		}

		/// User claims bonus from only one pool
		#[weight = 1_000]
		pub fn claim_bonus(
			origin,
			pool_id: T::PoolId
		) -> DispatchResult {
			let claimer = ensure_signed(origin)?;
			ensure!(Pools::<T>::contains_key(pool_id), Error::<T>::PoolNotExist);
			ensure!(Pools::<T>::get(pool_id).active, Error::<T>::PoolNotActive);

			// ensure the user has pool tokens for the pool
			ensure!(UserPoolTokensInPool::<T>::contains_key(&claimer, pool_id), Error::<T>::UserNotInThePool);

			Self::update_unclaimed_bonus_related_states(&claimer, pool_id)?;

			UserUnclaimedBonusInPool::<T>::mutate(&claimer, pool_id, |(unclaimed_bonus_balance, _block_num)| {
				// issue corresponding BNC bonus to the user's account
				T::AssetTrait::asset_issue(T::BNCAssetId::get(), &claimer, *unclaimed_bonus_balance);
				// mutate the user's unclaimed BNC bonus to zero
				*unclaimed_bonus_balance = Zero::zero();
			});

			Ok(())
		}

		/// Above are the exchange functions.
		/// Below are the exchange management functions.
		#[weight = 1_000]
		pub fn create_pool(
			origin,
			swap_fee_rate: T::SwapFee,  // this number is an integer to avoid precision loss, should be divided by fee precision constant when used.
			token_for_pool_vec: Vec<PoolCreateTokenDetails<T::AssetId, T::Balance, T::PoolWeight>>,
		) -> DispatchResult {
			let creator = ensure_signed(origin)?;
			// swap fee rate should be greater or equals to MinimumSwapFee.
			ensure!(swap_fee_rate >= T::MinimumSwapFee::get(), Error::<T>::FeeRateExceedMinimumLimit);
			// swap fee rate should be greater or equals to MaximumSwapFee.
			ensure!(swap_fee_rate <= T::MaximumSwapFee::get(), Error::<T>::FeeRateExceedMaximumLimit);

			// create three iterators for the map to be able to use multiple times.
			let map_iter = token_for_pool_vec.iter();
			ensure!(map_iter.len() <= T::NumberOfSupportedTokens::get() as usize, Error::<T>::TooManyTokensToPool);

			let mut total_weight: T::PoolWeight = Zero::zero();

			let map_iter_1 = token_for_pool_vec.iter();
			// ensure all the elements of the tokenForPoolMap are ok.
			for token_info in map_iter_1 {
				ensure!(T::AssetTrait::token_exists(token_info.token_id), Error::<T>::TokenNotExist);
				ensure!(token_info.token_balance > Zero::zero(), Error::<T>::AmountBelowZero);

				// get the user's balance for a specific token
				let user_token_balance = T::AssetTrait::get_account_asset(token_info.token_id, &creator).available;
				ensure!(user_token_balance >= token_info.token_balance, Error::<T>::NotEnoughBalance);
				// Add up the total weight
				total_weight = total_weight.saturating_add(token_info.token_weight);
			}

			// set up the new pool.
			// get the current length of the pool map
			let new_pool_id: T::PoolId = T::PoolId::from(Pools::<T>::iter().count() as u32);

			let new_pool = PoolDetails::<T::AccountId, T::SwapFee> {
				owner: creator.clone(),
				swap_fee_rate,
				active: false,
			};

			Pools::<T>::insert(new_pool_id, new_pool);

			let map_iter_2 = token_for_pool_vec.iter();
			// initialize the pool
			for token_info in map_iter_2 {
				// destroy user's token
				T::AssetTrait::asset_redeem(token_info.token_id, &creator, token_info.token_balance);

				// insert TokenWeightsInPool
				let token_normalized_weight = token_info.token_weight.saturating_mul(T::WeightPrecision::get()) / total_weight;
				TokenWeightsInPool::<T>::insert(new_pool_id, token_info.token_id, token_normalized_weight);

				// insert TokenBalancesInPool
				TokenBalancesInPool::<T>::insert(new_pool_id, token_info.token_id, token_info.token_balance);
			}

			// calculate and update PoolTokensInPool
			// first depositor can get a constant number of share in default
			PoolTokensInPool::<T>::insert(new_pool_id, T::InitialPoolSupply::get());

			// update UserPoolTokensInPool
			UserPoolTokensInPool::<T>::insert(&creator, new_pool_id, T::InitialPoolSupply::get());

			// get current block number
			let current_block_num = <frame_system::Module<T>>::block_number();
			// update UserUnclaimedBonusInPool
			UserUnclaimedBonusInPool::<T>::insert(&creator, new_pool_id, (T::Balance::from(0u32), current_block_num));

			// create a new entry for DeductedBonusAmountInPool
			DeductedBonusAmountInPool::<T>::insert(new_pool_id, T::Balance::from(0u32));

			// deposit pool created successfully event
			Self::deposit_event(RawEvent::CreatePoolSuccess);

			Ok(())
		}

		/// set the pool status to be true or false.
		#[weight = 1_000]
		pub fn set_pool_status(
			origin,
			pool_id: T::PoolId,
			new_status: bool
		) -> DispatchResult {
			let setter = ensure_signed(origin)?;

			let pool_details = Pools::<T>::get(pool_id);
			let pool_owner = pool_details.owner;
			ensure!(setter == pool_owner, Error::<T>::NotPoolOwner);

			Pools::<T>::mutate(pool_id, |pool_details| {
				if pool_details.active != new_status {
					pool_details.active = new_status;
				}
			});

			Self::deposit_event(RawEvent::SetPoolStatusSuccess);
			Ok(())
		}

		/// reset the swap fee
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn set_swap_fee(
			origin,
			pool_id: T::PoolId,
			new_swap_fee: T::SwapFee,
		) -> DispatchResult {
			let setter = ensure_signed(origin)?;

			let pool_details = Pools::<T>::get(pool_id);
			let pool_owner = pool_details.owner;

			ensure!(setter == pool_owner, Error::<T>::NotPoolOwner);
			ensure!(new_swap_fee >= T::MinimumSwapFee::get(), Error::<T>::FeeRateExceedMinimumLimit);
			ensure!(new_swap_fee <= T::MaximumSwapFee::get(), Error::<T>::FeeRateExceedMaximumLimit);

			// set the new swap fee
			Pools::<T>::mutate(pool_id, |pool_details| {
				if pool_details.swap_fee_rate != new_swap_fee {
					pool_details.swap_fee_rate = new_swap_fee;
				}
			});

			Self::deposit_event(RawEvent::SetSwapFeeSuccess);

			Ok(())
		}
	}
}

#[allow(dead_code)]
impl<T: Config> Module<T> {
	pub(crate) fn convert_float(input: I64F64) -> Result<T::Balance, Error<T>> {
		let converted = u128::from_fixed(input);
		TryInto::<T::Balance>::try_into(converted).map_err(|_| Error::<T>::ConvertFailure)
	}

	pub(crate) fn revise_storages_except_token_balances_when_adding_liquidity(
		pool_id: T::PoolId,		   // pool id
		new_pool_token: T::PoolToken, // to-be-issued pool token share to the user
		provider: &T::AccountId,	  // the user account_id
	) -> DispatchResult {
		// update the pool token amount of the specific pool
		PoolTokensInPool::<T>::mutate(pool_id, |pool_token_num| {
			*pool_token_num = pool_token_num.saturating_add(new_pool_token);
		});

		// update the pool token amount that the user possesses for a specific pool
		UserPoolTokensInPool::<T>::mutate(&provider, pool_id, |user_pool_token| {
			*user_pool_token = user_pool_token.saturating_add(new_pool_token);
		});

		Self::update_unclaimed_bonus_related_states(&provider, pool_id)?;
		Ok(())
	}
	pub(crate) fn revise_storages_except_token_balances_when_removing_liquidity(
		pool_id: T::PoolId,		   // pool id
		pool_token_out: T::PoolToken, // to-be-issued pool token share to the user
		remover: &T::AccountId,	   // the user account_id
	) -> DispatchResult {
		// Calculate and update user's unclaimed bonus in the pool.
		Self::update_unclaimed_bonus_related_states(&remover, pool_id)?;

		// Following are the updates.
		// update user's pool token shares in the pool.
		UserPoolTokensInPool::<T>::mutate(&remover, pool_id, |pool_token_shares| {
			*pool_token_shares = pool_token_shares.saturating_sub(pool_token_out);

			// if the remain balance equals zero, delete the record.
			if *pool_token_shares == Zero::zero() {
				UserPoolTokensInPool::<T>::remove(&remover, pool_id);
			}
		});

		// update the total pool balance in the pool.
		PoolTokensInPool::<T>::mutate(pool_id, |total_pool_balance| {
			*total_pool_balance = total_pool_balance.saturating_sub(pool_token_out);
		});
		Ok(())
	}

	pub(crate) fn update_unclaimed_bonus_related_states(
		account_id: &T::AccountId, // the user account_id
		pool_id: T::PoolId,		// pool id
	) -> DispatchResult {
		// Calculate the unclaimed bonus amount and update the UserUnclaimedBonusInPool map.
		let unclaimed_amount = {
			// Get the total amount of BNC bonus for the pool without consideration of the amount users have claimed.
			let bonus_pool_total_balance = Self::get_bonus_pool_balance(pool_id);
			let already_claimed_bonus_amount = DeductedBonusAmountInPool::<T>::get(pool_id);
			let remained_bonus_pool = bonus_pool_total_balance - already_claimed_bonus_amount;
			let amount =
				Self::calculate_unclaimed_bonus(&account_id, pool_id, remained_bonus_pool)?;
			Self::convert_float(amount)?
		};

		//get current block number update unclaimed bonus in pool.
		let current_block_num = <frame_system::Module<T>>::block_number();
		if UserUnclaimedBonusInPool::<T>::contains_key(&account_id, pool_id) {
			UserUnclaimedBonusInPool::<T>::mutate(
				&account_id,
				pool_id,
				|(unclaimed_bonus, last_calculation_block)| {
					*unclaimed_bonus = unclaimed_bonus.saturating_add(unclaimed_amount);
					*last_calculation_block = current_block_num;
				},
			);
		} else {
			UserUnclaimedBonusInPool::<T>::insert(
				&account_id,
				pool_id,
				(unclaimed_amount, current_block_num),
			);
		}

		// update the DeductedBonusAmountInPool map.
		DeductedBonusAmountInPool::<T>::mutate(pool_id, |already_deducted_bonus| {
			*already_deducted_bonus = already_deducted_bonus.saturating_add(unclaimed_amount);
		});

		Ok(())
	}

	/// ***********************************************************************************************************//
	///            user_pool_token    min(not calculated bonus block number, constant denominator for block number)//
	///  ratio =  -----------------  *   ----------------------------------------------                            //
	///              total_supply            constant denominator for block number                                 //
	/// ***********************************************************************************************************//
	/// calculate the un-calculated bonus and update it to the unclaimed bonus storage for the user
	/// whenever the liquidity share of the user changes.
	/// This requires a user to claim bonus every (constant block number). Otherwise, the user will lose the chance.
	pub(crate) fn calculate_unclaimed_bonus(
		account_id: &T::AccountId,
		pool_id: T::PoolId,
		remained_bonus_pool: T::Balance,
	) -> Result<FixedI128<extra::U64>, Error<T>> {
		let user_pool_token = UserPoolTokensInPool::<T>::get(&account_id, pool_id);
		let all_pool_token = PoolTokensInPool::<T>::get(pool_id);
		let current_block_num = <frame_system::Module<T>>::block_number(); //get current block number

		// get last unclaimed bonus information for the user
		let (_last_unclaimed_amount, last_calculate_block_num) =
			UserUnclaimedBonusInPool::<T>::get(&account_id, pool_id);
		// the block number between last calculation time and now.
		let unclaimed_bonus = {
			let mut pool_token_age = current_block_num - last_calculate_block_num;
			if pool_token_age > T::BonusClaimAgeDenominator::get() {
				pool_token_age = T::BonusClaimAgeDenominator::get();
			}

			// below are the data format transforming stuff.
			// u128 format.
			let user_pool_token = TryInto::<u128>::try_into(user_pool_token)
				.map_err(|_| Error::<T>::ConvertFailure)?;
			let all_pool_token = TryInto::<u128>::try_into(all_pool_token)
				.map_err(|_| Error::<T>::ConvertFailure)?;
			let pool_token_age = TryInto::<u128>::try_into(pool_token_age)
				.map_err(|_| Error::<T>::ConvertFailure)?;
			let age_denominator = TryInto::<u128>::try_into(T::BonusClaimAgeDenominator::get())
				.map_err(|_| Error::<T>::ConvertFailure)?;
			let remained_bonus_pool = TryInto::<u128>::try_into(remained_bonus_pool)
				.map_err(|_| Error::<T>::ConvertFailure)?;

			// fixed format.
			let user_pool_token = FixedI128::<extra::U64>::from_num(user_pool_token);
			let all_pool_token = FixedI128::<extra::U64>::from_num(all_pool_token);
			let pool_token_age = FixedI128::<extra::U64>::from_num(pool_token_age);
			let age_denominator = FixedI128::<extra::U64>::from_num(age_denominator);
			let remained_bonus_pool = FixedI128::<extra::U64>::from_num(remained_bonus_pool);

			// real calculation happens here.
			let bonus_ratio = user_pool_token
				.saturating_div(all_pool_token)
				.saturating_mul(pool_token_age)
				.saturating_div(age_denominator);
			bonus_ratio.saturating_mul(remained_bonus_pool)
		};

		Ok(unclaimed_bonus)
	}

	/// calculate weight ratio
	pub(crate) fn weight_ratio(
		upper: T::PoolWeight,
		down: T::PoolWeight,
	) -> Result<FixedI128<extra::U64>, Error<T>> {
		let u = TryInto::<u128>::try_into(upper).map_err(|_| Error::<T>::ConvertFailure)?;
		let d = TryInto::<u128>::try_into(down).map_err(|_| Error::<T>::ConvertFailure)?;

		let fixed = {
			let u = FixedI128::<extra::U64>::from_num(u);
			let d = FixedI128::<extra::U64>::from_num(d);
			u.saturating_div(d)
		};

		Ok(fixed)
	}

	///**********************************************************************************************
	/// calcInGivenOut                                                                               //
	/// aI = tokenAmountIn                                                                           //
	/// bO = tokenBalanceOut            /  /     bO      \    (wO / wI)     \                        //
	/// bI = tokenBalanceIn       bI * |  | ------------  | ^           - 1  |                       //
	/// aO = tokenAmountOut aI =        \  \ ( bO - aO ) /                   /                       //
	/// wI = tokenWeightIn        --------------------------------------------                       //
	/// wO = tokenWeightOut                   ( 1 - sF )                                             //
	/// sF = swapFee                                                                                 //
	/// **********************************************************************************************/
	pub(crate) fn calculate_in_given_out(
		token_balance_in: T::Balance,
		token_weight_in: T::PoolWeight,
		token_balance_out: T::Balance,
		token_weight_out: T::PoolWeight,
		token_amount_out: T::Balance,
		swap_fee: T::SwapFee,
	) -> Result<I64F64, Error<T>> {
		// type convert to u128
		let token_balance_in =
			TryInto::<u128>::try_into(token_balance_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_balance_out =
			TryInto::<u128>::try_into(token_balance_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_amount_out =
			TryInto::<u128>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee =
			TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;
		// to fixed num
		let token_balance_in = I64F64::from_num(token_balance_in);
		let token_balance_out = FixedI128::<extra::U64>::from_num(token_balance_out);
		let token_amount_out = FixedI128::<extra::U64>::from_num(token_amount_out);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get())
				.map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			let fee = FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision);
			FixedI128::<extra::U64>::from_num(1).saturating_sub(fee)
		};

		// pow exp
		let weight_ratio = Self::weight_ratio(token_weight_in, token_weight_out)?;
		// pow base
		let base =
			token_balance_out.saturating_div(token_balance_out.saturating_sub(token_amount_out));
		let fixed_token_amount_in = {
			let fixed_power: FixedI128<extra::U64> =
				transcendental::pow(base, weight_ratio).map_err(|_| Error::<T>::FixedPointError)?;
			let upper = token_balance_in
				.saturating_mul(fixed_power.saturating_sub(FixedI128::<extra::U64>::from_num(1)));
			upper.saturating_div(swap_fee)
		};

		Ok(fixed_token_amount_in)
	}

	///**********************************************************************************************
	/// calcOutGivenIn                                                                              //
	/// aO = tokenAmountOut                                                                         //
	/// bO = tokenBalanceOut                                                                        //
	/// bI = tokenBalanceIn            /      /              bI            \     (wI / wO) \        //
	/// aI = tokenAmountIn  aO = bO * |  1 - | --------------------------  | ^             |        //
	/// wI = tokenWeightIn             \      \ ( bI + ( aI * ( 1 - sF )) /               /         //
	/// wO = tokenWeightOut                                                                         //
	/// sF = swapFee                                                                                //
	///**********************************************************************************************/
	pub(crate) fn calculate_out_given_in(
		token_balance_in: T::Balance,
		token_weight_in: T::PoolWeight,
		token_amount_in: T::Balance,
		token_balance_out: T::Balance,
		token_weight_out: T::PoolWeight,
		swap_fee: T::SwapFee,
	) -> Result<I64F64, Error<T>> {
		// type convert to u128
		let token_balance_in =
			TryInto::<u128>::try_into(token_balance_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_balance_out =
			TryInto::<u128>::try_into(token_balance_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_amount_in =
			TryInto::<u128>::try_into(token_amount_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee =
			TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;
		// to fixed num
		let token_balance_in = I64F64::from_num(token_balance_in);
		let token_balance_out = FixedI128::<extra::U64>::from_num(token_balance_out);
		let token_amount_in = FixedI128::<extra::U64>::from_num(token_amount_in);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get())
				.map_err(|_| Error::<T>::ConvertFailure)?;
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
			let rhs = FixedI128::<extra::U64>::from_num(1).saturating_sub(
				transcendental::pow(base, weight_ratio).map_err(|_| Error::<T>::FixedPointError)?,
			);
			token_balance_out.saturating_mul(rhs)
		};

		Ok(fixed_token_amount_out)
	}

	///**********************************************************************************************
	/// calcPoolOutGivenSingleIn                                                                   //
	/// pAo = poolAmountOut      /                                              \                  //
	/// tAi = tokenAmountIn     ///      /     //    wI \      \\       \     wI \                 //
	/// wI = tokenWeightIn     //| tAi *| 1 - || 1 - --  | * sF || + tBi \    --  \                //
	/// tW = totalWeight   pAo=||  \     \     \\    tW /      //         | ^ tW   | * pS - pS     //
	/// tBi = tokenBalanceIn   \\  ------------------------------------- /        /                //
	/// pS = poolSupply         \\                    tBi               /        /                 //
	/// sF = swapFee             \                                              /                  //
	///**********************************************************************************************/
	pub(crate) fn calculate_pool_out_given_single_in(
		token_balance_in: T::Balance,
		token_weight_in: T::PoolWeight,
		token_amount_in: T::Balance,
		pool_supply: T::PoolToken,
		swap_fee: T::SwapFee,
	) -> Result<I64F64, Error<T>> {
		// type convert to u128
		let token_balance_in =
			TryInto::<u128>::try_into(token_balance_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_amount_in =
			TryInto::<u128>::try_into(token_amount_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_supply =
			TryInto::<u128>::try_into(pool_supply).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee =
			TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;
		// to fixed num
		let token_balance_in = I64F64::from_num(token_balance_in);
		let token_amount_in = FixedI128::<extra::U64>::from_num(token_amount_in);
		let pool_supply = FixedI128::<extra::U64>::from_num(pool_supply);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get())
				.map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision)
		};

		let weight_ratio = Self::weight_ratio(token_weight_in, T::WeightPrecision::get())?;

		let pool_token_issued = {
			let fee = FixedI128::<extra::U64>::from_num(1)
				- FixedI128::<extra::U64>::from_num(1)
					.saturating_sub(weight_ratio)
					.saturating_mul(swap_fee);
			let base = token_amount_in
				.saturating_mul(fee)
				.saturating_div(token_balance_in)
				.saturating_add(FixedI128::<extra::U64>::from_num(1));
			let lhs: FixedI128<extra::U64> =
				transcendental::pow(base, weight_ratio).map_err(|_| Error::<T>::FixedPointError)?;
			pool_supply.saturating_mul(lhs.saturating_sub(FixedI128::<extra::U64>::from_num(1)))
		};

		Ok(pool_token_issued)
	}

	///**********************************************************************************************
	/// calcSingleInGivenPoolOut                                                                  //
	/// tAi = tokenAmountIn           //(pS + pAo)\     /    1    \\                              //
	/// pS = poolSupply              || ---------  | ^ | --------- || * bI - bI                   //
	/// pAo = poolAmountOut           \\    pS    /     \(wI / tW)//                              //
	/// bI = balanceIn          tAi =  --------------------------------------------               //
	/// wI = weightIn                             /       wI  \                                   //
	/// tW = totalWeight                          |  1 - ----  |  * sF                            //
	/// sF = swapFee                               \      tW  /                                   //
	///**********************************************************************************************/
	pub(crate) fn calculate_single_in_given_pool_out(
		token_balance_in: T::Balance,
		token_weight_in: T::PoolWeight,
		pool_amount_out: T::PoolToken,
		pool_supply: T::PoolToken,
		swap_fee: T::SwapFee,
	) -> Result<I64F64, Error<T>> {
		// type convert to u128
		let token_balance_in =
			TryInto::<u128>::try_into(token_balance_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_amount_out =
			TryInto::<u128>::try_into(pool_amount_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_supply =
			TryInto::<u128>::try_into(pool_supply).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee =
			TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;
		// to fixed num
		let token_balance_in = I64F64::from_num(token_balance_in);
		let pool_amount_out = FixedI128::<extra::U64>::from_num(pool_amount_out);
		let pool_supply = FixedI128::<extra::U64>::from_num(pool_supply);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get())
				.map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision)
		};

		let weight_ratio = Self::weight_ratio(token_weight_in, T::WeightPrecision::get())?;

		let token_amount_in = {
			let base = pool_supply
				.saturating_add(pool_amount_out)
				.saturating_div(pool_supply);
			let reversed_weight_ratio =
				Self::weight_ratio(T::WeightPrecision::get(), token_weight_in)?;
			let power: FixedI128<extra::U64> = transcendental::pow(base, reversed_weight_ratio)
				.map_err(|_| Error::<T>::FixedPointError)?;
			let upper = power
				.saturating_sub(FixedI128::<extra::U64>::from_num(1))
				.saturating_mul(token_balance_in);
			let down = FixedI128::<extra::U64>::from_num(1)
				.saturating_sub(weight_ratio)
				.saturating_mul(swap_fee);
			upper.saturating_div(down)
		};

		Ok(token_amount_in)
	}

	///**********************************************************************************************
	/// calcSingleOutGivenPoolIn                                                                    //
	/// tAo = tokenAmountOut          /      /                                             \\       //
	/// bO = tokenBalanceOut         /      // pS - (pAi * (1 - eF)) \     /    1    \      \\      //
	/// pAi = poolAmountIn          | bO - || ----------------------- | ^ | --------- | * b0 ||     //
	/// ps = poolSupply              \      \\          pS           /     \(wO / tW)/      //      //
	/// wI = tokenWeightIn    tAo =   \      \                                            //        //
	/// tW = totalWeight                    /    /      wO \       \                                //
	/// sF = swapFee                    *  | 1 -|  1 - ---- | * sF  |                               //
	///                                     \    \      tW /       /                                //
	///**********************************************************************************************/
	pub(crate) fn calculate_single_out_given_pool_in(
		token_weight_in: T::PoolWeight,
		pool_amount_in: T::PoolToken,
		token_balance_out: T::Balance,
		pool_supply: T::PoolToken,
		swap_fee: T::SwapFee,
	) -> Result<I64F64, Error<T>> {
		// type convert to u128
		let token_balance_out =
			TryInto::<u128>::try_into(token_balance_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_amount_in =
			TryInto::<u128>::try_into(pool_amount_in).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_supply =
			TryInto::<u128>::try_into(pool_supply).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee =
			TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;

		// to fixed num
		let token_balance_out = I64F64::from_num(token_balance_out);
		let pool_amount_in = FixedI128::<extra::U64>::from_num(pool_amount_in);
		let pool_supply = FixedI128::<extra::U64>::from_num(pool_supply);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get())
				.map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision)
		};

		let weight_ratio = Self::weight_ratio(token_weight_in, T::WeightPrecision::get())?;
		let base = {
			let upper = pool_supply.saturating_sub(pool_amount_in);
			upper.saturating_div(pool_supply)
		};
		// calculate the percentage of the token weight in proportion of the pool token weights.
		let reversed_weight_ratio = Self::weight_ratio(T::WeightPrecision::get(), token_weight_in)?;
		let power: FixedI128<extra::U64> = transcendental::pow(base, reversed_weight_ratio)
			.map_err(|_| Error::<T>::FixedPointError)?;
		let lhs = token_balance_out
			.saturating_mul(FixedI128::<extra::U64>::from_num(1).saturating_sub(power));
		let rhs = {
			let fee = FixedI128::<extra::U64>::from_num(1)
				.saturating_sub(weight_ratio)
				.saturating_mul(swap_fee);
			FixedI128::<extra::U64>::from_num(1).saturating_sub(fee)
		};

		let token_amount_out = lhs.saturating_mul(rhs);
		Ok(token_amount_out)
	}

	///*************************************************************************************************/
	/// calcPoolInGivenSingleOut
	/// tAo = tokenAmountOut         /    /                            \               \
	/// bO = tokenBalanceOut        /    /  /           tAo            \\              \
	/// pAo = poolAmountOut pAo =  | 1 -|1-| -------------------------- || ^  (wO / tW) | * ps
	/// ps = poolSupply             \    \  \ bO * (1-(1- wO/tW) * sF) //              /
	/// wO = tokenWeightOut          \    \                            /              /
	/// tW = totalWeight
	/// sF = swapFee
	///
	///**************************************************************************************************/
	pub(crate) fn calculate_pool_in_given_single_out(
		token_weight_out: T::PoolWeight,
		token_amount_out: T::Balance,
		token_balance_out: T::Balance,
		pool_supply: T::PoolToken,
		swap_fee: T::SwapFee,
	) -> Result<I64F64, Error<T>> {
		// type convert to u128
		let token_amount_out =
			TryInto::<u128>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let token_balance_out =
			TryInto::<u128>::try_into(token_balance_out).map_err(|_| Error::<T>::ConvertFailure)?;
		let pool_supply =
			TryInto::<u128>::try_into(pool_supply).map_err(|_| Error::<T>::ConvertFailure)?;
		let swap_fee =
			TryInto::<u128>::try_into(swap_fee).map_err(|_| Error::<T>::ConvertFailure)?;

		// to fixed num
		let token_balance_out = I64F64::from_num(token_balance_out);
		let token_amount_out = FixedI128::<extra::U64>::from_num(token_amount_out);
		let pool_supply = FixedI128::<extra::U64>::from_num(pool_supply);
		let swap_fee = {
			let precision = TryInto::<u128>::try_into(T::FeePrecision::get())
				.map_err(|_| Error::<T>::ConvertFailure)?;
			let precision = FixedI128::<extra::U64>::from_num(precision);
			FixedI128::<extra::U64>::from_num(swap_fee).saturating_div(precision)
		};

		let weight_ratio = Self::weight_ratio(token_weight_out, T::WeightPrecision::get())?;

		let inside_part_rhs_denominator_multiplier = FixedI128::<extra::U64>::from_num(1)
			.saturating_sub(FixedI128::<extra::U64>::from_num(1).saturating_mul(swap_fee));
		let inside_part_rhs = token_amount_out
			.saturating_div(token_balance_out)
			.saturating_div(inside_part_rhs_denominator_multiplier);
		let inside_part = FixedI128::<extra::U64>::from_num(1).saturating_sub(inside_part_rhs);
		let power: FixedI128<extra::U64> = transcendental::pow(inside_part, weight_ratio)
			.map_err(|_| Error::<T>::FixedPointError)?;

		let pool_amount_out =
			pool_supply.saturating_mul(FixedI128::<extra::U64>::from_num(1).saturating_sub(power));
		Ok(pool_amount_out)
	}

	// ********************************************************************************
	// below are the interfaces needed from other pallets.
	// Query for the current bonus balance for the pool
	pub(crate) fn get_bonus_pool_balance(_pool_id: T::PoolId) -> T::Balance {
		T::Balance::from(1_000_000_u32) // to get from other pallets. Not yet implemented
	}
}

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
	StorageValue,
};
use frame_system::{self as system, ensure_signed};
use node_primitives::{AssetTrait, TokenSymbol};
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Saturating, Zero};

mod mock;
mod tests;

pub trait Trait: frame_system::Trait {
	/// event
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// fee
	type Fee: Member
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

	/// The units in which we record costs.
	type Cost: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	/// The units in which we record incomes.
	type Income: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;

	type AssetTrait: AssetTrait<
		Self::AssetId,
		Self::AccountId,
		Self::Balance,
		Self::Cost,
		Self::Income,
	>;

	/// Weight
	type PoolWeight: Member
		+ Parameter
		+ AtLeast32Bit
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ Into<Self::Balance>
		+ From<Self::Balance>;

	/// Some limitations on Balancer protocol
	type MaximumSwapInRatio: Get<Self::Balance>; // must be less 1/2. Since we can only keep interger in the invariant, so this number should be 2. When it is use, we reverse it to be 1/2.
	type MinimumAddedPoolTokenShares: Get<Self::Balance>;
	type MinimumSwapFee: Get<Self::Fee>;
	type FeePrecision: Get<Self::Fee>;
	type WeightPrecision: Get<Self::PoolWeight>;
	type NumberOfSupportedTokens: Get<u8>;
	type MaxIntervalForCalculatingLiquidityBonus: Get<Self::BlockNumber>; // used in the ration for calculating liquidity bonus.
	type BNCAssetId: Get<Self::AssetId>;
	type BonusClaimAgeDenominator: Get<Self::BlockNumber>;
	type FirstPoolTokenShare: Get<Self::Balance>;
}

decl_event! {
	pub enum Event<T> where <T as Trait>::Balance, {
		AddLiquiditySuccess,
		RemoveLiquiditySuccess,
		AddSingleLiquiditySuccess,
		RemoveSingleLiquiditySuccess,
		SwapTokenSuccess(Balance, Balance),
		CreatePoolSuccess,
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Pool id doesn't exist
		PoolNotExist,
		/// Pool is not in the state of active
		PoolNotActive,
		/// Asset id doesn't exist
		TokenNotExist,
		/// Amount of input should be less than or equal to origin balance
		NotEnoughBalance,
		/// Convert type with error
		ConvertFailure,
		/// Balance limitation on adding new liquidity
		LessThanMinimumAddedPoolTokenShares,
		/// Too many tokens added to pool
		TooManyTokensToPool,
		/// User have no pool token in the pool
		UserNotInThePool,
		/// User cannot swap between two the same token
		ForbidSameTokenSwap,
		/// Error on fix point crate
		FixedPointError,
		/// Exceed too many amount of token, it should be trade_amount / pool total amount <= 1 / 2
		ExceedMaximumSwapInRatio,
		/// Less than expected amount while trading
		LessThanExpectedAmount,
		/// Bigger than expected price while trading
		BiggerThanExpectedAmount,
		// Amount should be bigger than zero
		AmountShouldBiggerThanZero,
		// Fee rate should be no less than zero
		FeeRateShouldNoLessThanZero,
		// Fee rate should be less than one
		FeeRateShouldLessThanOne,
		// not the owner of the pool
		NotPoolOwner,
	}
}

#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
pub struct PoolDetails<AccountId, Fee> {
	owner: AccountId, // The owner of the pool, who has the privilages to set or change the parameters of the pool.
	swapFeeRate: Fee, // The current swap rate of the pool.
	active: bool, // Pool status. If is true, users can add liquidity into or swap in the pool. Otherwise, user operations will be prevented.
}


#[derive(Encode, Decode, Default, Clone, Eq, PartialEq, Debug)]
pub struct PoolCreateTokenDetails<AssetId, Balance, PoolWeight> {
	token_id: AssetId, // token asset id
	token_balance: Balance, // token balance that the pool creator wants to deposit into the pool for the first time.
	token_weight: PoolWeight, // token weight that the pool creator wants to give to the token
}


decl_storage! {
	trait Store for Module<T: Trait> as Swap {

		/// Pool info
		Pools get(fn pools): map hasher(blake2_128_concat) T::PoolId => Option<PoolDetails<
			T:: AccountId,
			T:: Fee,
		>>;

		/// Token weights info for pools. Weights must be normalized at the beginning. The sum of all the token weights for a pool must be 1 * WeightPrecision. Should be ensured when set up the pool.
		TokenWeightsInPool get(fn token_weights_in_pool): double_map
			hasher(blake2_128_concat) T::PoolId,
			hasher(blake2_128_concat) T::AssetId
			=> T::PoolWeight;

		/// Token blance info for pools
		TokenBalancesInPool get(fn token_balances_in_pool): double_map
			hasher(blake2_128_concat) T::PoolId,
			hasher(blake2_128_concat) T::AssetId
			=> T::Balance;

		/// total pool tokens in pool.
		PoolTokensInPool get(fn pool_tokens_in_pool): map hasher(blake2_128_concat) T::PoolId => T::Balance;


		/// Users' pool tokens in different pools
		UserPoolTokensInPool get(fn user_pool_tokens_in_pool): double_map
			hasher(blake2_128_concat) T::AccountId,
			hasher(blake2_128_concat) T::PoolId
			=> T::Balance;

		/// Record user unclaimed liquidity bouns. There are two occassions that will trigger the calculation of unclaimed bonus:
		/// 1. The user adds or removes his liqidity to the pool.
		/// 2. The user claims his bonus.
		/// The value part of the map is a tuple contains (un_claimed_Bonus, last_calculation_block).
		/// "un_claimed_Bonus" shows the remaining unclaimed but calculated bonus balance.
		/// "last_calculation_block" records the block number of last time when liquidity bonus calculation is triggered.
		UserUnclaimedBonusInPool get(fn user_unclaimed_bonus_in_pool): double_map
			hasher(blake2_128_concat) T::AccountId,
			hasher(blake2_128_concat)  T::PoolId
			=> (T::Balance, T::BlockNumber);  // (un_claimed_Bonus, last_calculation_block)

		/// Record the calculated deducted BNC bonus amount for each pool, including deducted but unclaimed amount as well as claimed amount
		DeductedBounusAmountInPool get(fn deducted_bonus_amount_in_pool): map hasher(blake2_128_concat) T::PoolId => T::Balance;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		// when in a trade, trade_amount / all_amount <= 1 / 2
		const MaximumSwapInRatio: T::Balance = T::MaximumSwapInRatio::get();  // MaximumSwapInRatio keeps a number of 2. Reverse it to be 1/2 when using it.
		// when adding liquidity, deposit at least this amount of pool token shares
		const MinimumAddedPoolTokenShares: T::Balance = T::MinimumAddedPoolTokenShares::get();
		// Minimu swap fee amount, in order to prevent malicious attack by doing small amount swaps
		const MinimumSwapFee: T::Fee = T::MinimumSwapFee::get();
		// Used to calculate fee rate to prevent precision lost in float type.
		const FeePrecision: T::Fee = T::FeePrecision::get();
		// Used to calculate weight in percentage to prevent precision lost in float type.
		const WeightPrecision: T::PoolWeight = T::WeightPrecision::get();
		// The uplimit of tokens supported.
		const NumberOfSupportedTokens: u8 = T::NumberOfSupportedTokens::get();
		// Used in calculating the token age ratio of liquidity bonus.
		const MaxIntervalForCalculatingLiquidityBonus: T::BlockNumber = T::MaxIntervalForCalculatingLiquidityBonus::get();
		// the asset id of BNC
		const BNCAssetId: T::AssetId = T::BNCAssetId::get();
		// The max age denominator used in calculating unclaimed BNC bonus for liquidity providers.
		const BonusClaimAgeDenominator: T::BlockNumber = T::BonusClaimAgeDenominator::get();
		// the initial share for the pool creator.
		const FirstPoolTokenShare: T::Balance = T::FirstPoolTokenShare::get();

		fn deposit_event() = default;

		// ****************************************************************************
		// Add liquidity by providing all of the tokens in proportion.
		// The user inputs a pool token share in the front end, and the front end will automatically calculate the amount of each aseet that should be provided liquidity with.
		// (add liquidity)(many assets) given share in => amount out
		#[weight = weight_for::add_liquidity::<T>()]
		fn add_liquidity_given_shares_in(
			origin,
			poolId: T::PoolId,
			#[compact] new_pool_token: T::Balance,
		) {
			let provider = ensure_signed(origin)?;

			ensure!(Pools::<T>::contains_key(poolId), Error::<T>::PoolNotExist);  // ensure the pool exists
			ensure!(Pools::<T>::get(poolId).unwrap().active, Error::<T>::PoolNotActive);  // ensure pool is in the active state, which means initial setup of the pool has been done and the pool is open for adding liquidity and swapping.
			ensure!(new_pool_token >= T::MinimumAddedPoolTokenShares::get(), Error::<T>::LessThanMinimumAddedPoolTokenShares);  // ensure newly added liquidity is bigger than MinimumAddedPoolTokenShares of pool tokens
			ensure!(new_pool_token > Zero::zero(), Error::<T>::AmountShouldBiggerThanZero); // ensure the new pool token amount is bigger than zero.

			let token_balances_in_pool_iter = TokenBalancesInPool::<T>::iter_prefix(poolId);  // get the iterator of the items(assetId => blance) with the same first key(poolId)
			let mut user_should_deposit_tokens = BTreeMap::new();  // record how many tokens the user should deposit if he wants to aquire certain pool token share

			// calculate how many tokens for the user to deposit for each of the assets
			for tk in token_balances_in_pool_iter {  //0 pisition is assetId, 1 position is balance

				let all_pool_tokens = PoolTokensInPool::<T>::get(poolId);  // get the total pool token shares for the specific pool
				let new_pool_token_percent = new_pool_token / all_pool_tokens;  // calculate that the newly added pool tokens share accounts for how much percentage of the original pool tokens.
				let tokenId = tk.0;  // Asset id
				let UserTokenPoolBalance = UserPoolTokensInPool::<T>::get(&provider, poolId);  // the balance of a asset for a user in a pool
				let tokenPoolBalance = TokenBalancesInPool::<T>::get(poolId, tokenId);  // the balance of a specific token in a pool
				let should_deposit_amount = tokenPoolBalance * new_pool_token_percent;   // the amount of the token that the user should deposit
				ensure!(UserTokenPoolBalance >= should_deposit_amount, Error::<T>::NotEnoughBalance);  // ensure the user has enough balances for all kinds of tokens in the pool
				user_should_deposit_tokens.insert(tokenId, should_deposit_amount);  // record the should-be-deposited amount each of the token
			}

			Self::revise_storages_except_token_blances_when_adding_liquidity(poolId, new_pool_token, &provider);

			// issue new pool token to the user
			// updates all the token balances of each token in the pool, and destroy corresponding user balances
			for (tk, blc) in user_should_deposit_tokens.iter() {
				TokenBalancesInPool::<T>::mutate(poolId, tk, |token_blance| {
					*token_blance = token_blance.saturating_add(*blc);
				});

				// destroy token from user's asset_redeem(assetId, &target, amount)
				T::AssetTrait::asset_redeem(tk, &provider, *blc);
			}

			Self::deposit_event(RawEvent::AddLiquiditySuccess);
		}


		// ****************************************************************************
		// A user adds liquidity by depositing only one kind of token. So we need to calculate the corresponding pool token share the user should get.
		// (add liquidity)(single asset) given amount in => share out
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn add_single_liquidity_given_amount_in(
			origin,
			poolId: T::PoolId,
			assetId: T::AssetId,
			#[compact] token_amount_in: T::Balance,
		) -> DispatchResult {
			let provider = ensure_signed(origin)?;

			ensure!(T::AssetTrait::token_exists(assetId), Error::<T>::TokenNotExist);  // ensure the token id exist
			ensure!(Pools::<T>::contains_key(poolId), Error::<T>::PoolNotExist);  // ensure the pool exists
			ensure!(Pools::<T>::get(poolId).unwrap().active, Error::<T>::PoolNotActive);  // ensure pool is in the active state, which means initial setup of the pool has been done and the pool is open for adding liquidity and swapping.
			ensure!(token_amount_in > Zero::zero(), Error::<T>::AmountShouldBiggerThanZero); // ensure the token amount in is bigger than zero.

			let user_token_balance = T::AssetTrait::get_account_asset(assetId, &provider).balance;  // get the user's balance for a specific token
			ensure!(user_token_balance >= token_amount_in, Error::<T>::NotEnoughBalance);

			// caculate how many pool token will be issued to user
			let new_pool_token = {
				// get current token balance and weight in the pool
				let token_balance_in = TokenBalancesInPool::<T>::get(poolId, assetId);
				let token_weight_in = TokenWeightsInPool::<T>::get(poolId, assetId);
				let pool_supply = PoolTokensInPool::<T>::get(poolId);  // get the total pool token shares for the specific pool
				let swap_fee_rate = Pools::<T>::get(poolId).unwrap().swapFeeRate; // get the swap fee rate of the pool
				let issued_pool_token = Self::calculate_pool_out_given_single_in(token_balance_in, token_weight_in, token_amount_in, pool_supply, swap_fee_rate)?;
				let pool_token_issued = u128::from_fixed(issued_pool_token);
				TryInto::<T::Balance>::try_into(pool_token_issued).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// Before revising storages, we should make sure the added pool token shares meet the minimum requirement.
			ensure!(new_pool_token / new_pool_token >= T::MinimumAddedPoolTokenShares::get(), Error::<T>::LessThanMinimumAddedPoolTokenShares);

			Self::revise_storages_except_token_blances_when_adding_liquidity(poolId, new_pool_token, &provider);

			// Updates the token balance that the user adds liquidity with in the pool
			TokenBalancesInPool::<T>::mutate(poolId, assetId, |token_blance| {
				*token_blance = token_blance.saturating_add(token_amount_in);
			});

			// destroy token from user's asset_redeem(assetId, &target, amount)
			T::AssetTrait::asset_redeem(assetId, &provider, token_amount_in);

			Self::deposit_event(RawEvent::AddSingleLiquiditySuccess);
			Ok(())
		}


		// ****************************************************************************
		// A user adds liquidity by depositing only one kind of token. So we need to calculate the corresponding pool token share the user should get.
		// (add liquidity)(single asset) given share in => amount out
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn add_single_liquidity_given_shares_in(
			origin,
			poolId: T::PoolId,
			assetId: T::AssetId,
			new_pool_token: T::Balance,
		) -> DispatchResult {
			let provider = ensure_signed(origin)?;

			ensure!(T::AssetTrait::token_exists(assetId), Error::<T>::TokenNotExist);  // ensure the token id exist
			ensure!(Pools::<T>::contains_key(poolId), Error::<T>::PoolNotExist);  // ensure the pool exists
			ensure!(Pools::<T>::get(poolId).unwrap().active, Error::<T>::PoolNotActive);  // ensure pool is in the active state, which means initial setup of the pool has been done and the pool is open for adding liquidity and swapping.
			ensure!(new_pool_token > Zero::zero(), Error::<T>::AmountShouldBiggerThanZero); // ensure the new_pool_token in is bigger than zero.
			ensure!((new_pool_token / new_pool_token) >= T::MinimumAddedPoolTokenShares::get(), Error::<T>::LessThanMinimumAddedPoolTokenShares);  // Make sure the added pool token shares meet the minimum requirement.

			// caculate how many token-in amount should the user provide to the pool to acquire the corresponding pool token shares.
			let token_amount_in = {
				// get current token balance and weight in the pool
				let token_balance_in = TokenBalancesInPool::<T>::get(poolId, assetId);
				let token_weight_in = TokenWeightsInPool::<T>::get(poolId, assetId);
				let pool_supply = PoolTokensInPool::<T>::get(poolId);  // get the total pool token shares for the specific pool
				let swap_fee_rate = Pools::<T>::get(poolId).unwrap().swapFeeRate; // get the swap fee rate of the pool
				let should_token_amount_in = Self::calculate_single_in_given_pool_out(token_balance_in, token_weight_in, new_pool_token, pool_supply, swap_fee_rate)?;

				let should_token_amount_in = u128::from_fixed(should_token_amount_in);
				TryInto::<T::Balance>::try_into(should_token_amount_in).map_err(|_| Error::<T>::ConvertFailure)?
			};

			let user_token_balance = T::AssetTrait::get_account_asset(assetId, &provider).balance;  // get the user's balance for a specific token
			ensure!(user_token_balance >= token_amount_in, Error::<T>::NotEnoughBalance);

			Self::revise_storages_except_token_blances_when_adding_liquidity(poolId, new_pool_token, &provider);

			// Updates the token balance that the user adds liquidity with in the pool
			TokenBalancesInPool::<T>::mutate(poolId, assetId, |token_blance| {
				*token_blance = token_blance.saturating_add(token_amount_in);
			});

			// destroy token from user's asset_redeem(assetId, &target, amount)
			T::AssetTrait::asset_redeem(assetId, &provider, token_amount_in);

			Self::deposit_event(RawEvent::AddSingleLiquiditySuccess);
			Ok(())
		}


		// ****************************************************************************
		// User remove liquidity with only one kind of token
		// (remove liquidity)(single asset) given share in => amount out
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn remove_single_asset_liquidity_given_shares_in(
			origin,
			poolId: T::PoolId,
			assetId: T::AssetId,
			#[compact] pool_token_out: T::Balance  // The pool token that the user want to remove liquidity with from the pool.
		) -> DispatchResult {
			let remover = ensure_signed(origin)?;

			ensure!(T::AssetTrait::token_exists(assetId), Error::<T>::TokenNotExist);  // ensure the token id exist
			ensure!(Pools::<T>::contains_key(poolId), Error::<T>::PoolNotExist);  // ensure the pool exists
			ensure!(Pools::<T>::get(poolId).unwrap().active, Error::<T>::PoolNotActive);  // ensure pool is in the active state, which means initial setup of the pool has been done and the pool is open for adding liquidity and swapping.
			ensure!(pool_token_out > Zero::zero(), Error::<T>::AmountShouldBiggerThanZero); // ensure the pool token out amount in is bigger than zero.
			ensure!(UserPoolTokensInPool::<T>::contains_key(&remover, poolId), Error::<T>::UserNotInThePool);  // ensure this user has the specific pool token share
			ensure!(UserPoolTokensInPool::<T>::get(&remover, poolId) >= pool_token_out, Error::<T>::NotEnoughBalance);  // ensure the user has more pool token share than what he is going to withdrawl.

			// calculate how many balance user will get
			let token_amount = {
				let swap_fee_rate = Pools::<T>::get(poolId).unwrap().swapFeeRate;  // Pool swap fee rate, which is an integer, should be divided by rate precision when being used.
				let out_token_weight = TokenWeightsInPool::<T>::get(poolId, assetId);  // out-token's weight in the pool, which is an normalized integer, should be divided by weight precision when being used.
				let out_token_balance_in_pool = TokenBalancesInPool::<T>::get(poolId, assetId);  // out-token's balance in the pool, which is the number of the specific token.
				let pool_supply = PoolTokensInPool::<T>::get(poolId);  // total pool token that the specific pool has issued.
				let token_amount_out = Self::calculate_single_out_given_pool_in(out_token_weight, pool_token_out, out_token_balance_in_pool, pool_supply, swap_fee_rate)?;
				let token_amount_out = u128::from_fixed(token_amount_out);

				TryInto::<T::Balance>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// update user asset
			T::AssetTrait::asset_issue(assetId, &remover, token_amount);

			// update TokenBalancesInPool map.
			TokenBalancesInPool::<T>::mutate(poolId, assetId, |token_balance| {
				*token_balance = token_balance.saturating_add(token_amount);
			});

			Self::revise_storages_except_token_blances_when_removing_liquidity(poolId, pool_token_out, &remover);
			Self::deposit_event(RawEvent::RemoveSingleLiquiditySuccess);

			Ok(())
		}


		// ****************************************************************************
		// User remove liquidity with only one kind of token
		// (remove liquidity)(single asset) given amount in => shares out
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn remove_single_asset_liquidity_given_amount_in(
			origin,
			poolId: T::PoolId,
			assetId: T::AssetId,
			token_amount: T::Balance  // The number of out-token that the user want to remove liquidity with from the pool.
		) -> DispatchResult {
			let remover = ensure_signed(origin)?;

			ensure!(T::AssetTrait::token_exists(assetId), Error::<T>::TokenNotExist);  // ensure the token id exist
			ensure!(Pools::<T>::contains_key(poolId), Error::<T>::PoolNotExist);  // ensure the pool exists
			ensure!(Pools::<T>::get(poolId).unwrap().active, Error::<T>::PoolNotActive);  // ensure pool is in the active state, which means initial setup of the pool has been done and the pool is open for adding liquidity and swapping.
			ensure!(token_amount > Zero::zero(), Error::<T>::AmountShouldBiggerThanZero); // ensure the token out amount in is bigger than zero.
			ensure!(UserPoolTokensInPool::<T>::contains_key(&remover, poolId), Error::<T>::UserNotInThePool);  // ensure this user has the specific pool token share

			// calculate how many pool tokens that the user wants to remove liquidity with
			let pool_token_out = {
				let swap_fee_rate = Pools::<T>::get(poolId).unwrap().swapFeeRate;  // Pool swap fee rate, which is an integer, should be divided by rate precision when being used.
				let out_token_weight = TokenWeightsInPool::<T>::get(poolId, assetId);  // out-token's weight in the pool, which is an normalized integer, should be divided by weight precision when being used.
				let out_token_balance_in_pool = TokenBalancesInPool::<T>::get(poolId, assetId);  // out-token's balance in the pool, which is the number of the specific token.
				let pool_supply = PoolTokensInPool::<T>::get(poolId);  // total pool token that the specific pool has issued.

				let pool_token_out = Self::calculate_pool_in_given_single_out(out_token_weight, token_amount, out_token_balance_in_pool, pool_supply, swap_fee_rate)?;
				let pool_token_out = u128::from_fixed(pool_token_out);

				TryInto::<T::Balance>::try_into(pool_token_out).map_err(|_| Error::<T>::ConvertFailure)?
			};

			ensure!(UserPoolTokensInPool::<T>::get(&remover, poolId) >= pool_token_out, Error::<T>::NotEnoughBalance);  // ensure the user has more pool token share than what he is going to withdrawl.

			// update user asset
			T::AssetTrait::asset_issue(assetId, &remover, token_amount);

			// update TokenBalancesInPool map.
			TokenBalancesInPool::<T>::mutate(poolId, assetId, |token_balance| {
				*token_balance = token_balance.saturating_sub(token_amount);
			});

			Self::revise_storages_except_token_blances_when_removing_liquidity(poolId, pool_token_out, &remover);
			Self::deposit_event(RawEvent::RemoveSingleLiquiditySuccess);

			Ok(())
		}


		// ****************************************************************************
		// User removes all the tokens in the pool in proportion of his pool token shares.
		// (remove liquidity)(many assets) given share in => amount out
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn remove_assets_liquidity_given_shares_in(
			origin,
			poolId: T::PoolId,
			#[compact] pool_amount_out: T::Balance
		) {
			let remover = ensure_signed(origin)?;

			ensure!(Pools::<T>::contains_key(poolId), Error::<T>::PoolNotExist);  // ensure the pool exists
			ensure!(Pools::<T>::get(poolId).unwrap().active, Error::<T>::PoolNotActive);  // ensure pool is in the active state, which means initial setup of the pool has been done and the pool is open for adding liquidity and swapping.
			ensure!(UserPoolTokensInPool::<T>::contains_key(&remover, poolId), Error::<T>::UserNotInThePool);  // ensure this user has the specific pool token share
			ensure!(pool_amount_out > Zero::zero(), Error::<T>::AmountShouldBiggerThanZero); // ensure the token out amount in is bigger than zero.

			let token_balances_in_pool_iter = TokenBalancesInPool::<T>::iter_prefix(poolId);  // get the iterator of the items(assetId => blance) with the same first key(poolId)
			// calculate how many tokens ffor each of the assets that user can withdrawl. Meanwhile, issue money to user's account and deducted from the pool.
			for tk in token_balances_in_pool_iter {  //0 pisition is assetId, 1 position is balance

				let all_pool_tokens = PoolTokensInPool::<T>::get(poolId);  // get the total pool token shares for the specific pool
				let pool_amount_out_percent = pool_amount_out / all_pool_tokens;  // calculate that the newly added pool tokens share accounts for how much percentage of the original pool tokens.
				let tokenId = tk.0;  // Asset id
				let tokenPoolBalance = TokenBalancesInPool::<T>::get(poolId, tokenId);  // the balance of a specific token in a pool
				let can_withdrawl_amount = tokenPoolBalance * pool_amount_out_percent;   // the amount of the token that the user should deposit
				// issue money to user's account
				T::AssetTrait::asset_issue(tokenId, &remover, can_withdrawl_amount);

				// deduct the corresponding token balance in the pool
				TokenBalancesInPool::<T>::mutate(poolId, tokenId, |token_balance| {
					*token_balance = token_balance.saturating_sub(can_withdrawl_amount);
				});
			}

			Self::revise_storages_except_token_blances_when_removing_liquidity(poolId, pool_amount_out, &remover);
			Self::deposit_event(RawEvent::RemoveLiquiditySuccess);
		}

		// ****************************************************************************
		// User swap one token for another kind of token, given an exact amount for token-in.
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn swap_exact_in(
			origin,
			poolId: T::PoolId,
			token_in_assetId: T::AssetId,
			#[compact]token_amount_in: T::Balance, // the input token amount that the user is willing to pay.
			min_token_amount_out: Option<T::Balance>,  // The least output token amount that the user can accept
			token_out_assetId: T::AssetId,
		) -> DispatchResult {
			let swapper = ensure_signed(origin)?;

			ensure!(token_in_assetId != token_out_assetId, Error::<T>::ForbidSameTokenSwap);  // ensure token_in_assetId is different from token_out_assetId.
			ensure!(T::AssetTrait::token_exists(token_in_assetId), Error::<T>::TokenNotExist);  // ensure the input token id exist
			ensure!(T::AssetTrait::token_exists(token_out_assetId), Error::<T>::TokenNotExist);  // ensure the output token id exist
			ensure!(Pools::<T>::contains_key(poolId), Error::<T>::PoolNotExist);  // ensure the pool exists
			ensure!(Pools::<T>::get(poolId).unwrap().active, Error::<T>::PoolNotActive);  // ensure pool is in the active state, which means initial setup of the pool has been done and the pool is open for adding liquidity and swapping.
			ensure!(token_amount_in > Zero::zero(), Error::<T>::AmountShouldBiggerThanZero); // ensure the amount in bigger than zero.

			let user_token_balance = T::AssetTrait::get_account_asset(token_in_assetId, &swapper).balance;  // get the user's balance for a specific token
			ensure!(user_token_balance >= token_amount_in, Error::<T>::NotEnoughBalance);  // ensure the user has enough token-in balance to swap.

			let token_in_pool_amount = TokenBalancesInPool::<T>::get(poolId, token_in_assetId);  // get the total token-in token amount for the specific pool
			ensure!(token_in_pool_amount.div(token_amount_in) >= T::MaximumSwapInRatio::get(), Error::<T>::ExceedMaximumSwapInRatio);  // MaximumSwapInRatio is a reverse number.(2 => 1/2), trade less half of pool balances.

			// do a swap
			let token_amount_out = {
				let token_out_pool_amount = TokenBalancesInPool::<T>::get(poolId, token_out_assetId);  // get the total token-out token amount for the specific pool
				let weight_in = TokenWeightsInPool::<T>::get(poolId, token_in_assetId); // The normalized weight of the token-in in the pool.
				let weight_out = TokenWeightsInPool::<T>::get(poolId, token_out_assetId);  // The normalized weight of the token-out in the pool.
				let swap_fee_rate = Pools::<T>::get(poolId).unwrap().swapFeeRate;  // Pool swap fee rate, which is an integer, should be divided by rate precision when being used.

				let fixed_token_amount_out = Self::calculate_out_given_in(token_in_pool_amount, weight_in, token_amount_in, token_out_pool_amount, weight_out, swap_fee_rate)?;
				let token_amount_out = u128::from_fixed(fixed_token_amount_out);

				TryInto::<T::Balance>::try_into(token_amount_out).map_err(|_| Error::<T>::ConvertFailure)?
			};

			// ensure token_amount_in is bigger than you exepect
			if min_token_amount_out.is_some() {
				ensure!(Some(token_amount_out) >= min_token_amount_out, Error::<T>::LessThanExpectedAmount);
			}

			T::AssetTrait::asset_redeem(token_in_assetId, &swapper, token_amount_in); // deducted token-in amount from the user account
			T::AssetTrait::asset_issue(token_out_assetId, &swapper, token_amount_out);  // add up token-out amount to the user account

			// update the token-in amount in the pool
			TokenBalancesInPool::<T>::mutate(poolId, token_in_assetId, |token_balance| {
				*token_balance = token_balance.saturating_add(token_amount_in);
			});

			// update the token-out amount in the pool
			TokenBalancesInPool::<T>::mutate(poolId, token_out_assetId, |token_balance| {
				*token_balance = token_balance.saturating_sub(token_amount_out);
			});

			Self::deposit_event(RawEvent::SwapTokenSuccess(token_amount_in, token_amount_out));

			Ok(())
		}


		// ****************************************************************************
		// User swap one token for another kind of token, given an exact amount for token-out.
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		fn swap_exact_out(
			origin,
			poolId: T::PoolId,
			token_out_assetId: T::AssetId,
			#[compact]token_amount_out: T::Balance, // the out token amount that the user wants to get.
			max_token_amount_in: Option<T::Balance>,  // The most input token amount that the user can accept to get the token amount out.
			token_in_assetId: T::AssetId,
		) -> DispatchResult {
			let swapper = ensure_signed(origin)?;

			ensure!(token_in_assetId != token_out_assetId, Error::<T>::ForbidSameTokenSwap);  // ensure token_in_assetId is different from token_out_assetId.

			ensure!(T::AssetTrait::token_exists(token_in_assetId), Error::<T>::TokenNotExist);  // ensure the input token id exist
			ensure!(T::AssetTrait::token_exists(token_out_assetId), Error::<T>::TokenNotExist);  // ensure the output token id exist



			ensure!(Pools::<T>::contains_key(poolId), Error::<T>::PoolNotExist);  // ensure the pool exists
			ensure!(Pools::<T>::get(poolId).unwrap().active, Error::<T>::PoolNotActive);  // ensure pool is in the active state, which means initial setup of the pool has been done and the pool is open for adding liquidity and swapping.
			ensure!(token_amount_out > Zero::zero(), Error::<T>::AmountShouldBiggerThanZero); // ensure the amount out is bigger than zero.

			let token_out_pool_amount = TokenBalancesInPool::<T>::get(poolId, token_out_assetId);  // get the total token-out token amount for the specific pool
			ensure!(token_out_pool_amount.div(token_amount_out) >= T::MaximumSwapInRatio::get(), Error::<T>::ExceedMaximumSwapInRatio);  // MaximumSwapInRatio is a reverse number.(2 => 1/2), trade less half of pool balances.

			// do a swap
			let token_amount_in = {
				let token_in_pool_amount = TokenBalancesInPool::<T>::get(poolId, token_in_assetId);  // get the total token-in token amount for the specific pool
				let weight_in = TokenWeightsInPool::<T>::get(poolId, token_in_assetId); // The normalized weight of the token-in in the pool.
				let weight_out = TokenWeightsInPool::<T>::get(poolId, token_out_assetId);  // The normalized weight of the token-out in the pool.
				let swap_fee_rate = Pools::<T>::get(poolId).unwrap().swapFeeRate;  // Pool swap fee rate, which is an integer, should be divided by rate precision when being used.
				let fixed_token_amount_in = Self::calculate_in_given_out(token_in_pool_amount, weight_in, token_amount_out, weight_out, token_out_pool_amount, swap_fee_rate)?;
				let token_amount_in = u128::from_fixed(fixed_token_amount_in);
				TryInto::<T::Balance>::try_into(token_amount_in).map_err(|_| Error::<T>::ConvertFailure)?
			};



			// ensure calculated token_amount_in is smaller than you exepect
			if max_token_amount_in.is_some() {
				ensure!(Some(token_amount_in) <= max_token_amount_in, Error::<T>::BiggerThanExpectedAmount);
			}

			let user_token_balance = T::AssetTrait::get_account_asset(token_in_assetId, &swapper).balance;  // get the user's balance for a specific token
			ensure!(user_token_balance >= token_amount_in, Error::<T>::NotEnoughBalance);  // ensure the user has enough token-in balance to swap.

			T::AssetTrait::asset_redeem(token_in_assetId, &swapper, token_amount_in); // deducted token-in amount from the user account
			T::AssetTrait::asset_issue(token_out_assetId, &swapper, token_amount_out);  // add up token-out amount to the user account

			// update the token-in amount in the pool
			TokenBalancesInPool::<T>::mutate(poolId, token_in_assetId, |token_balance| {
				*token_balance = token_balance.saturating_add(token_amount_in);
			});

			// update the token-out amount in the pool
			TokenBalancesInPool::<T>::mutate(poolId, token_out_assetId, |token_balance| {
				*token_balance = token_balance.saturating_sub(token_amount_out);
			});

			Self::deposit_event(RawEvent::SwapTokenSuccess(token_amount_in, token_amount_out));

			Ok(())
		}


		// User claims bonus from only one pool
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn claim_bonus(
			origin,
			poolId: T::PoolId
		) -> DispatchResult {
			let claimer = ensure_signed(origin)?;
			ensure!(Pools::<T>::contains_key(poolId), Error::<T>::PoolNotExist);  // ensure the pool exists
			ensure!(Pools::<T>::get(poolId).unwrap().active, Error::<T>::PoolNotActive);  // ensure pool is in the active state, which means initial setup of the pool has been done and the pool is open for adding liquidity and swapping.

			// ensure the user has pool tokens for the pool
			ensure!(UserPoolTokensInPool::<T>::contains_key(&claimer, poolId), Error::<T>::UserNotInThePool);

			Self::update_unclaimed_bonus_related_states(&claimer, poolId);

			UserUnclaimedBonusInPool::<T>::mutate(&claimer, poolId, |(unclaimed_bonus_balance, block_num)| {
				// issue corresponding BNC bonus to the user's account
				T::AssetTrait::asset_issue(T::BNCAssetId::get(), &claimer, *unclaimed_bonus_balance);
				// mutate the user's unclaimed BNC bonus to zero
				*unclaimed_bonus_balance = Zero::zero();
			});

			Ok(())
		}

		// ******************************************************
		// ***  Above are the exchange functions.			  ***
		// ***  Below are the exchange manangement functions. ***
		// ******************************************************
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn create_pool(
			origin,
			swapFeeRate: T::Fee,  // this number is an integer to avoid precision loss, should be divided by fee precision constant when used.
			tokenForPoolVec: Vec<PoolCreateTokenDetails<T::AssetId, T::Balance, T::PoolWeight>>,
		) -> DispatchResult {
			let creator = ensure_signed(origin)?;

			ensure!(swapFeeRate >= Zero::zero(), Error::<T>::FeeRateShouldNoLessThanZero);  // swap fee rate should be greater or equals to zero.
			ensure!(swapFeeRate < T::FeePrecision::get(), Error::<T>::FeeRateShouldLessThanOne);  // swap fee rate should be greater or equals to zero.

			// create three iterators for the map to be able to use multiple times.
			let map_iter = tokenForPoolVec.iter();
			let map_iter_1 = map_iter.cloned();
			let map_iter_2 = map_iter.cloned();

			ensure!(map_iter.len() <= T::NumberOfSupportedTokens::get().into(), Error::<T>::TooManyTokensToPool);  // ensure the vec's length is less than the maximum support number

			let mut total_weight = T::PoolWeight::from(0);

			// ensure all the elements of the tokenForPoolMap are ok.
			for token_info in map_iter_1 {
				ensure!(T::AssetTrait::token_exists(token_info.token_id), Error::<T>::TokenNotExist);  // ensure token asset id exists.
				ensure!(token_info.token_balance > Zero::zero(), Error::<T>::AmountShouldBiggerThanZero);  // ensure the initial token balances are greater than zero.

				let user_token_balance = T::AssetTrait::get_account_asset(token_info.token_id, &creator).balance;  // get the user's balance for a specific token
				ensure!(user_token_balance >= token_info.token_balance, Error::<T>::NotEnoughBalance);  // ensure user's balance is enough for deposit.

				total_weight = total_weight + token_info.token_weight;  // Add up the total weight
			}

			// set up the new pool.
			let new_pool_id = Pools::<T>::iter().count(); // get the current length of the pool map

			let new_pool = PoolDetails::<T::AccountId, T::Fee> {
				owner: creator.clone(),
				swapFeeRate: swapFeeRate,
				active: false,
			};

			Pools::<T>::insert(new_pool_id.into(), new_pool);


			// initialize the pool
			for token_info in map_iter_2 {
				// destroy user's token
				T::AssetTrait::asset_redeem(token_info.token_id, &creator, token_info.token_balance);

				// insert TokenWeightsInPool
				let token_normalized_weight = token_info.token_weight * T::WeightPrecision::get() / total_weight;
				TokenWeightsInPool::<T>::insert(new_pool_id.into(), token_info.token_id, token_normalized_weight);

				// insert TokenBalancesInPool
				TokenBalancesInPool::<T>::insert(new_pool_id.into(), token_info.token_id, token_info.token_balance);
			}

			// calculate and update PoolTokensInPool
			// first depositor can get a constant number of share in default
			PoolTokensInPool::<T>::insert(new_pool_id.into(), T::FirstPoolTokenShare::get());

			// update UserPoolTokensInPool
			UserPoolTokensInPool::<T>::insert(&creator, new_pool_id.into(), T::FirstPoolTokenShare::get());

			let current_block_num = <frame_system::Module<T>>::block_number();  //get current block number
			// update UserUnclaimedBonusInPool
			UserUnclaimedBonusInPool::<T>::insert(&creator, new_pool_id.into(), (Zero::zero(), current_block_num));

			// create a new entry for DeductedBounusAmountInPool
			DeductedBounusAmountInPool::<T>::insert(new_pool_id.into(), Zero::zero());

			// deposit pool created sucessfully event
			Self::deposit_event(RawEvent::CreatePoolSuccess);

			Ok(())
		}

		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn set_pool_status(
			origin,
			poolId: T::PoolId,
			new_status: bool) -> DispatchResult {
			let setter = ensure_signed(origin)?;

			let pool_details = Pools::<T>::get(poolId).unwrap();
			let pool_owner = pool_details.owner;
			ensure!(setter == pool_owner, Error::<T>::NotPoolOwner);  // ensure the origin is the pool owner

			if new_status == false || new_status == true {
				Pools::<T>::mutate(poolId, |pool_details| {
					pool_details.unwrap().active = new_status;
				});
			}
			Ok(())
		}

		// reset the swap fee
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn set_swap_fee(
			origin,
			poolId: T::PoolId,
			new_swap_fee: T::Fee,
		) -> DispatchResult {
			let setter = ensure_signed(origin)?;

			let pool_details = Pools::<T>::get(poolId).unwrap();
			let pool_owner = pool_details.owner;

			ensure!(setter == pool_owner, Error::<T>::NotPoolOwner);  // ensure the origin is the pool owner
			ensure!(new_swap_fee >= Zero::zero(), Error::<T>::FeeRateShouldNoLessThanZero); // swap fee rate should be bigger than or equal to zero.
			ensure!(new_swap_fee < T::FeePrecision::get(), Error::<T>::FeeRateShouldLessThanOne); // swap fee rate should be less than one.

			// set the new swap fee
			Pools::<T>::mutate(poolId, |pool_details| {
				pool_details.unwrap().swapFeeRate = new_swap_fee;
			});

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

	pub(crate) fn revise_storages_except_token_blances_when_adding_liquidity(
		poolId: T::PoolId,          // pool id
		new_pool_token: T::Balance, // to-be-issued pool token share to the user
		provider: &T::AccountId,    // the user accountId
	) -> DispatchResult {
		// update the pool token amount of the specific pool
		PoolTokensInPool::<T>::mutate(poolId, |pool_token_num| {
			*pool_token_num = pool_token_num.saturating_add(new_pool_token);
		});

		// update the pool token amount that the user possesses for a specific pool
		UserPoolTokensInPool::<T>::mutate(&provider, poolId, |user_pool_token| {
			*user_pool_token = user_pool_token.saturating_add(new_pool_token);
		});

		Self::update_unclaimed_bonus_related_states(&provider, poolId);
		Ok(())
	}
	pub(crate) fn revise_storages_except_token_blances_when_removing_liquidity(
		poolId: T::PoolId,          // pool id
		pool_token_out: T::Balance, // to-be-issued pool token share to the user
		remover: &T::AccountId,     // the user accountId
	) -> DispatchResult {
		// Calculate and update user's unclaimed bonus in the pool.
		Self::update_unclaimed_bonus_related_states(&remover, poolId);

		// Following are the updates.
		// update user's pool token shares in the pool.
		UserPoolTokensInPool::<T>::mutate(&remover, poolId, |pool_token_shares| {
			*pool_token_shares = pool_token_shares.saturating_sub(pool_token_out);

			// if the remain balance equals zero, delete the record.
			if *pool_token_shares == Zero::zero() {
				UserPoolTokensInPool::<T>::remove(&remover, poolId);
			}
		});

		// update the total pool balance in the pool.
		PoolTokensInPool::<T>::mutate(poolId, |total_pool_balance| {
			*total_pool_balance = total_pool_balance.saturating_sub(pool_token_out);
		});
		Ok(())
	}

	pub(crate) fn update_unclaimed_bonus_related_states(
		accountId: &T::AccountId, // the user accountId
		poolId: T::PoolId,        // pool id
	) -> DispatchResult {
		// Calculate the unclaimd bonus amount and update the UserUnclaimedBonusInPool map.
		let unclaimed_amount = {
			let bonus_pool_total_balance = Self::get_bonus_pool_balance(poolId); // Get the total amount of BNC bonus for the pool without consideration of the amount users have claimed.
			let already_claimed_bonus_amount = DeductedBounusAmountInPool::<T>::get(poolId);
			let remained_bonus_pool = bonus_pool_total_balance - already_claimed_bonus_amount;
			let amount = Self::calculate_unclaimed_bonus(&accountId, poolId, remained_bonus_pool)?;
			Self::convert_float(amount)?
		};

		let current_block_num = <frame_system::Module<T>>::block_number(); //get current block number
																   // update unclaimed bonus in pool.
		if UserUnclaimedBonusInPool::<T>::contains_key(&accountId, poolId) {
			UserUnclaimedBonusInPool::<T>::mutate(
				&accountId,
				poolId,
				|(unclaimed_Bonus, last_calculation_block)| {
					*unclaimed_Bonus = unclaimed_Bonus.saturating_add(unclaimed_amount);
					*last_calculation_block = current_block_num;
				},
			);
		} else {
			UserUnclaimedBonusInPool::<T>::insert(
				&accountId,
				poolId,
				(unclaimed_amount, current_block_num),
			);
		}

		// update the DeductedBounusAmountInPool map.
		DeductedBounusAmountInPool::<T>::mutate(poolId, |already_deducted_bonus| {
			*already_deducted_bonus = already_deducted_bonus.saturating_add(unclaimed_amount);
		});

		Ok(())
	}

	// ***********************************************************************************
	//             user_pool_token             uncalculated bonus block number
	//  ratio =  -----------------  *   ----------------------------------------------
	//               total_supply           constant denominator for block number
	// ***********************************************************************************
	// calculate the un-calculated bonus and update it to the unclaimed bonus storage for the user whenver the liquidity share of the user changes.
	// This requires a user to claim bonus every (constant block number). Otherwise, the user will lose the chance.
	pub(crate) fn calculate_unclaimed_bonus(
		accountId: &T::AccountId,
		poolId: T::PoolId,
		remained_bonus_pool: T::Balance,
	) -> Result<FixedI128<extra::U64>, Error<T>> {
		let user_pool_token = UserPoolTokensInPool::<T>::get(&accountId, poolId);
		let all_pool_token = PoolTokensInPool::<T>::get(poolId);
		let current_block_num = <frame_system::Module<T>>::block_number(); //get current block number

		let (last_unclaimed_amount, last_calculat_block_num) =
			UserUnclaimedBonusInPool::<T>::get(&accountId, poolId); // get last unclaimed bonus information for the user
		let pool_token_age = current_block_num - last_calculat_block_num; // the block number between last calculation time and now.
		let unclaimed_bonus = {
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

			// fiexed format.
			let user_pool_token = FixedI128::<extra::U64>::from_num(user_pool_token);
			let all_pool_token = FixedI128::<extra::U64>::from_num(all_pool_token);
			let pool_token_age = FixedI128::<extra::U64>::from_num(pool_token_age);
			let age_denominator = FixedI128::<extra::U64>::from_num(age_denominator);
			let remained_bonus_pool = FixedI128::<extra::U64>::from_num(remained_bonus_pool);

			// real calcuation happens here.
			let bonus_ratio = user_pool_token
				.saturating_div(all_pool_token)
				.saturating_mul(pool_token_age)
				.saturating_div(age_denominator);
			bonus_ratio.saturating_mul(remained_bonus_pool)
		};

		Ok(unclaimed_bonus)
	}

	// calculate weight ratio
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
		swap_fee: T::Fee,
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
		swap_fee: T::Fee,
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
		pool_supply: T::Balance,
		swap_fee: T::Fee,
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
		pool_amount_out: T::Balance,
		pool_supply: T::Balance,
		swap_fee: T::Fee,
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

	/**********************************************************************************************
	// calcSingleOutGivenPoolIn                                                                  //
	// tAo = tokenAmountOut            /      /                                             \\   //
	// bO = tokenBalanceOut           /      // pS - (pAi * (1 - eF)) \     /    1    \      \\  //
	// pAi = poolAmountIn            | bO - || ----------------------- | ^ | --------- | * b0 || //
	// ps = poolSupply                \      \\          pS           /     \(wO / tW)/      //  //
	// wI = tokenWeightIn      tAo =   \      \                                             //   //
	// tW = totalWeight                    /     /      wO \       \                             //
	// sF = swapFee                    *  | 1 - |  1 - ---- | * sF  |                            //
	//                                     \     \      tW /       /                             //
		**********************************************************************************************/
	pub(crate) fn calculate_single_out_given_pool_in(
		token_weight_in: T::PoolWeight,
		pool_amount_in: T::Balance,
		token_balance_out: T::Balance,
		pool_supply: T::Balance,
		swap_fee: T::Fee,
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
		let reversed_weight_ratio = Self::weight_ratio(T::WeightPrecision::get(), token_weight_in)?; // calculate the percentage of the token weight in proportion of the pool token weights.
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

	/*************************************************************************************************/
	// calcPoolInGivenSingleOut
	// tAo = tokenAmountOut            /     /                            \              \
	// bO = tokenBalanceOut           /     /  /          tAo             \\              \
	// pAo = poolAmountOut    pAo =  | 1 - |1-| -------------------------- || ^  (wO / tW) | * ps
	// ps = poolSupply                \     \  \ bO * (1-(1- wO/tW) * sF) //              /
	// wO = tokenWeightOut              \    \                            /              /
	// tW = totalWeight
	// sF = swapFee
	//
	//**************************************************************************************************/
	pub(crate) fn calculate_pool_in_given_single_out(
		token_weight_out: T::PoolWeight,
		token_amount_out: T::Balance,
		token_balance_out: T::Balance,
		pool_supply: T::Balance,
		swap_fee: T::Fee,
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
	pub(crate) fn get_bonus_pool_balance(poolId: T::PoolId) -> T::Balance {
		T::Balance::from(100_000_000) // to get from other pallets. Not yet implemented
	}
}

#[allow(dead_code)]
mod weight_for {
	use super::Trait;
	use frame_support::{traits::Get, weights::Weight};

	/// add liquidity weight
	pub(crate) fn add_liquidity<T: Trait>() -> Weight {
		let reads_writes = T::DbWeight::get().reads_writes(1, 1);
		reads_writes * 1000 as Weight
	}

	/// add single liquidity
	pub(crate) fn add_single_liquidity<T: Trait>() -> Weight {
		todo!();
	}
}

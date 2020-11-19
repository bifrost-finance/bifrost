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

//! Tests for the module.

#![cfg(test)]

use crate::*;
use crate::mock::*;
use float_cmp::approx_eq;
use frame_support::{assert_ok, dispatch::DispatchError};
use node_primitives::TokenSymbol;
use fixed_point::{
	traits::FromFixed,
	transcendental,
	types::{extra, *},
	FixedI128,
};


fn initialize_pool_for_dispatches() {

	// initialize token asset types.
	assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18));  // TokenSymbol id 0
	assert_ok!(Assets::create(Origin::root(), b"DOT".to_vec(), 18));  // TokenSymbol id 1
	assert_ok!(Assets::create(Origin::root(), b"vDOT".to_vec(), 18));  // TokenSymbol id 2
	assert_ok!(Assets::create(Origin::root(), b"KSM".to_vec(), 18));  // TokenSymbol id 3
	assert_ok!(Assets::create(Origin::root(), b"vKSM".to_vec(), 18));  // TokenSymbol id 4
	assert_ok!(Assets::create(Origin::root(), b"EOS".to_vec(), 18));  // TokenSymbol id 5
	assert_ok!(Assets::create(Origin::root(), b"vEOS".to_vec(), 18));  // TokenSymbol id 6
	assert_ok!(Assets::create(Origin::root(), b"IOST".to_vec(), 18));  // TokenSymbol id 7
	assert_ok!(Assets::create(Origin::root(), b"vIOST".to_vec(), 18));  // TokenSymbol id 8

	// initialize some parameters used to dispatch the create_pool call.
	let alice = 1;
	let bob = 2;
	let asud_id = 0;
	let dot_id = 1;
	let ksm_id = 3;
	let ausd_type = TokenSymbol::from(asud_id);
	let dot_type = TokenSymbol::from(dot_id);
	let ksm_type = TokenSymbol::from(ksm_id);

	// issue tokens to Alice's account.
	assert_ok!(Assets::issue(Origin::root(), ausd_type, alice, 10_000));
	assert_ok!(Assets::issue(Origin::root(), dot_type, alice, 30_000));
	assert_ok!(Assets::issue(Origin::root(), ksm_type, alice, 30_000));

	// issue tokens to Bob's account.
	assert_ok!(Assets::issue(Origin::root(), ausd_type, bob, 1_000_000));
	assert_ok!(Assets::issue(Origin::root(), dot_type, bob, 1_000_000));
	assert_ok!(Assets::issue(Origin::root(), ksm_type, bob, 1_000_000));

	// initialize the parameters for create_pool.
	let creator = Origin::signed(alice);
	let swap_fee_rate = 500;

	let vec_node_1 = PoolCreateTokenDetails::<Test> {
		token_id: ausd_type,
		token_balance: 500,
		token_weight: 20,
	};

	let vec_node_2 = PoolCreateTokenDetails::<Test> {
		token_id: dot_type,
		token_balance: 1_000,
		token_weight: 40,
	};

	let vec_node_3 = PoolCreateTokenDetails::<Test> {
		token_id: ksm_type,
		token_balance: 400,
		token_weight: 40,
	};

	let token_for_pool_vec: Vec<PoolCreateTokenDetails<Test>> = vec![vec_node_1, vec_node_2, vec_node_3] ;
	run_to_block(2);  // set the block number to 2.
	// Dispatch the create_pool call to create a new pool.
	assert_ok!(Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec));

	let pool_id = 0;
	let new_status = true;
	assert_ok!(Swap::set_pool_status(creator.clone(), pool_id, new_status));
}


#[test]
fn convert_float_should_work() {
 let fixed_in = FixedI128::<extra::U64>::from_num(8);

 let result = Swap::convert_float(fixed_in);
 assert!(result.is_ok());
 assert_eq!(8, result.unwrap());

}

#[test]
fn weight_ratio_should_work() {
	let upper = 100u64;
	let down = 1000u64;

	let weight_ratio = Swap::weight_ratio(upper, down);
	assert!(weight_ratio.is_ok());
	approx_eq!(f32, 0.1f32, f32::from_fixed(weight_ratio.unwrap()), epsilon = 0.000_000_000_001);

}


#[test]
fn calculate_out_given_in_should_work() {

	let token_balance_in = 1_000_000;
	let token_weight_in = 20_000;
	let token_amount_in = 1_000;
	let token_balance_out = 50_000_000;
	let token_weight_out = 40_000;
	let swap_fee = 500;

	let target = 6_450_675.3_655f32;
	let to_buy = Swap::calculate_out_given_in(token_balance_in, token_weight_in, token_amount_in, token_balance_out, token_weight_out, swap_fee);
	assert!(to_buy.is_ok());

	let to_buy = to_buy.map(f32::from_fixed).unwrap();
	approx_eq!(f32, target, to_buy, epsilon = 0.000_000_000_001);
}


#[test]
fn create_pool_should_work() {
	new_test_ext().execute_with(|| {

		// initialize some parameters used to dispatch the create_pool call.
		let alice = 1;
		let asud_id = 0;
		let dot_id = 1;
		let ksm_id = 3;
		let ausd_type = TokenSymbol::from(asud_id);
		let dot_type = TokenSymbol::from(dot_id);
		let ksm_type = TokenSymbol::from(ksm_id);

		// initialize token asset types.
		assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18));  // TokenSymbol id 0
		assert_ok!(Assets::create(Origin::root(), b"DOT".to_vec(), 18));  // TokenSymbol id 1
		assert_ok!(Assets::create(Origin::root(), b"vDOT".to_vec(), 18));  // TokenSymbol id 2
		assert_ok!(Assets::create(Origin::root(), b"KSM".to_vec(), 18));  // TokenSymbol id 3
		assert_ok!(Assets::create(Origin::root(), b"vKSM".to_vec(), 18));  // TokenSymbol id 4
		assert_ok!(Assets::create(Origin::root(), b"EOS".to_vec(), 18));  // TokenSymbol id 5
		assert_ok!(Assets::create(Origin::root(), b"vEOS".to_vec(), 18));  // TokenSymbol id 6
		assert_ok!(Assets::create(Origin::root(), b"IOST".to_vec(), 18));  // TokenSymbol id 7
		assert_ok!(Assets::create(Origin::root(), b"vIOST".to_vec(), 18));  // TokenSymbol id 8

		// issue tokens to Alice's account.
		assert_ok!(Assets::issue(Origin::root(), ausd_type, alice, 10_000));
		assert_ok!(Assets::issue(Origin::root(), dot_type, alice, 30_000));
		assert_ok!(Assets::issue(Origin::root(), ksm_type, alice, 30_000));

		// initialize the parameters for create_pool.
		let creator = Origin::signed(alice);
		let swap_fee_rate = 500;

		let vec_node_1 = PoolCreateTokenDetails::<Test> {
			token_id: ausd_type,
			token_balance: 500,
			token_weight: 20,
		};

		let vec_node_2 = PoolCreateTokenDetails::<Test> {
			token_id: dot_type,
			token_balance: 1_000,
			token_weight: 40,
		};

		let vec_node_3 = PoolCreateTokenDetails::<Test> {
			token_id: ksm_type,
			token_balance: 400,
			token_weight: 40,
		};

		let token_for_pool_vec: Vec<PoolCreateTokenDetails<Test>> = vec![vec_node_1.clone(), vec_node_2.clone(), vec_node_3.clone()] ;
		run_to_block(2);  // set the block number to 2.

		// Dispatch the create_pool call to create a new pool.
		assert_ok!(Swap::create_pool(creator, swap_fee_rate, token_for_pool_vec));

		// validate the value of storage Pools.
		let result = Swap::pools(0);
		assert_eq!(result.owner, alice);  // Validate that the pool owner is Alice.
		assert_eq!(result.swap_fee_rate, 500); // Validate the swap fee is 500.
		assert_eq!(result.active, false);  // Validate the inital value of pool state is inactive.

		// validate the value of storage TokenWeightsInPool.
		assert_eq!(Swap::token_weights_in_pool(0, ausd_type), 20_000);  // the weight of ausd token
		assert_eq!(Swap::token_weights_in_pool(0, dot_type), 40_000);  // the weight of dot token
		assert_eq!(Swap::token_weights_in_pool(0, ksm_type), 40_000);  // the weight of ksm token

		// validate the value of storage TokenBalancesInPool.
		assert_eq!(Swap::token_balances_in_pool(0, ausd_type), 500);  // the balance of ausd token
		assert_eq!(Swap::token_balances_in_pool(0, dot_type), 1_000);  // the balance of dot_type token
		assert_eq!(Swap::token_balances_in_pool(0, ksm_type), 400);  // the balance of ksm_type token

		// validate the value of storage PoolTokensInPool.
		assert_eq!(Swap::pool_tokens_in_pool(0), 1_000);  // the pool token balance of pool 0

		// validate the value of storage UserPoolTokensInPool.
		assert_eq!(Swap::user_pool_tokens_in_pool(alice, 0), 1_000);  // the pool token balance of alice in pool 0

		// validate the value of storage UserUnclaimedBonusInPool.
		assert_eq!(Swap::user_unclaimed_bonus_in_pool(alice, 0), (0, 2));  // user unclaimed balance and the creation time of this record.

		// validate the value of storage DeductedBounusAmountInPool.
		assert_eq!(Swap::deducted_bonus_amount_in_pool(0), 0);  // the deducted bonus amount of pool 0.
	
		// Below are the incorrect operations.
		let bob = 2;
		let creator = Origin::signed(bob);


		// issue tokens to Alice's account.
		assert_ok!(Assets::issue(Origin::root(), ausd_type, alice, 10_000));
		assert_ok!(Assets::issue(Origin::root(), dot_type, alice, 30_000));
		assert_ok!(Assets::issue(Origin::root(), ksm_type, alice, 30_000));

		// swap fee rate exceeds 100%.
		let swap_fee_rate = 500_000;
		let token_for_pool_vec: Vec<PoolCreateTokenDetails<Test>> = vec![vec_node_1.clone(), vec_node_2.clone(), vec_node_3.clone()] ;
		assert_eq!(Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec), Err(DispatchError::Module { index: 0, error: 15, message: Some("FeeRateExceedMaximumLimit") }));

		// swap fee rate is below 0%.
		let swap_fee_rate = 0;
		let token_for_pool_vec: Vec<PoolCreateTokenDetails<Test>> = vec![vec_node_1.clone(), vec_node_2.clone(), vec_node_3.clone()] ;
		assert_eq!(Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec), Err(DispatchError::Module { index: 0, error: 14, message: Some("FeeRateExceedMinimumLimit") }));

		// the length of the vector is 9, which exceeds the biggest supported token number in the pool.
		let swap_fee_rate = 1_000;
		let token_for_pool_vec: Vec<PoolCreateTokenDetails<Test>> = vec![vec_node_1.clone(), vec_node_1.clone(), vec_node_1.clone(), vec_node_1.clone(), vec_node_1.clone(), vec_node_1.clone(), vec_node_1.clone(), vec_node_1.clone(), vec_node_1.clone()] ;
		assert_eq!(Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec), Err(DispatchError::Module { index: 0, error: 6, message: Some("TooManyTokensToPool") }));

		// validate the tokens used in creating a pool exist. Right now it doesn't work for the type TokenSymbol. When id changes to asset id in the later version, this test should work.
		// let vec_node_4 = PoolCreateTokenDetails::<Test> {
		// 	token_id: TokenSymbol::from(78),
		// 	token_balance: 400,
		// 	token_weight: 40,
		// };
		// let swap_fee_rate = 1_000;
		// let token_for_pool_vec: Vec<PoolCreateTokenDetails<Test>> = vec![vec_node_1.clone(), vec_node_2.clone(), vec_node_3.clone(), vec_node_4.clone()] ;
		// assert_eq!(Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec), Err(DispatchError::Module { index: 0, error: 6, message: Some("TokenNotExist") }));

		// validate token amount used to create a pool must be bigger than zero.
		let vec_node_4 = PoolCreateTokenDetails::<Test> {
			token_id: dot_type,
			token_balance: 0,
			token_weight: 40,
		};
		let swap_fee_rate = 1_000;
		assert_ok!(Assets::issue(Origin::root(), ausd_type, bob, 1_000));
		assert_ok!(Assets::issue(Origin::root(), dot_type, bob, 100));
		let token_for_pool_vec: Vec<PoolCreateTokenDetails<Test>> = vec![vec_node_1.clone(),  vec_node_4.clone()] ;
		assert_eq!(Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec), Err(DispatchError::Module { index: 0, error: 13, message: Some("AmountBelowZero") }));


		// validate token amount used to create a pool must not exceed user's balance.
		let vec_node_4 = PoolCreateTokenDetails::<Test> {
			token_id: dot_type,
			token_balance: 1_000,
			token_weight: 40,
		};
		let swap_fee_rate = 1_000;
		assert_ok!(Assets::issue(Origin::root(), ausd_type, bob, 1_000));
		assert_ok!(Assets::issue(Origin::root(), dot_type, bob, 100));
		let token_for_pool_vec: Vec<PoolCreateTokenDetails<Test>> = vec![vec_node_1.clone(),  vec_node_4.clone()] ;
		assert_eq!(Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec), Err(DispatchError::Module { index: 0, error: 3, message: Some("NotEnoughBalance") }));
	});
}


#[test]
fn add_liquidity_given_shares_in_should_work() {
	new_test_ext().execute_with(|| {
		// initialize a pool.
		initialize_pool_for_dispatches();

		let alice = 1;
		let bob = 2;
		let creator = Origin::signed(bob);
		let pool_id = 0;
		let new_pool_token = 200;  // Alice initial pool token amount is 1000. Bob want's to get 20% of that of Alice's. 

		let asud_id = 0;
		let dot_id = 1;
		let ksm_id = 3;
		let ausd_type = TokenSymbol::from(asud_id);
		let dot_type = TokenSymbol::from(dot_id);
		let ksm_type = TokenSymbol::from(ksm_id);

		assert_ok!(Swap::add_liquidity_given_shares_in(creator, pool_id, new_pool_token));  // Bob to get 200 share in the pool.

		// check wehter the pool has added 200 shares.ksm_type
		assert_eq!(Swap::pool_tokens_in_pool(pool_id), 1_200);

		// check wether bob has got 200 shares of pool.ksm_type
		assert_eq!(Swap::user_pool_tokens_in_pool(bob, pool_id), 200);

		// check wether the token balances are right.
		assert_eq!(Swap::token_balances_in_pool(pool_id, ausd_type),600);
		assert_eq!(Swap::token_balances_in_pool(pool_id, dot_type), 1_200);
		assert_eq!(Swap::token_balances_in_pool(pool_id, ksm_type), 480);

		// check wether bob's account has been deducted corresponding amount for different tokens.
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(ausd_type, &bob).balance, 999_900);  // get the user's balance for ausd
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(dot_type, &bob).balance, 999_800);  // get the user's balance for dot
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(ksm_type, &bob).balance, 999_920);  // get the user's balance for ksm

		// Below are the incorrect operations.

		// no such pool_id
		let pool_id = 1;
		let creator = Origin::signed(bob);
		assert_eq!(Swap::add_liquidity_given_shares_in(creator.clone(), pool_id, new_pool_token), Err(DispatchError::Module{index: 0, error: 0, message: Some("PoolNotExist")})); 
		
		// pool status is false, thus it's not albe to add liquidity in.
		let pool_id = 0;
		assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
		assert_eq!(Swap::add_liquidity_given_shares_in(creator.clone(), pool_id, new_pool_token), Err(DispatchError::Module{index: 0, error: 1, message: Some("PoolNotActive")}));

		// Everytime a user at least adds more than or equal to the MinimumAddedPoolTokenShares
		let new_pool_token = 1;
		assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
		assert_eq!(Swap::add_liquidity_given_shares_in(creator.clone(), pool_id, new_pool_token), Err(DispatchError::Module{index: 0, error: 5, message: Some("LessThanMinimumPassedInPoolTokenShares")}));

		// Everytime a user at least adds more than or equal to the MinimumAddedPoolTokenShares
		let new_pool_token = 1_000_000;
		assert_eq!(Swap::add_liquidity_given_shares_in(creator.clone(), pool_id, new_pool_token), Err(DispatchError::Module{index: 0, error: 3, message: Some("NotEnoughBalance")}));
	
		// deposit pool token share is more than maximum added share limit.
		let new_pool_token = 100_000_000;
		assert_eq!(Swap::add_liquidity_given_shares_in(creator.clone(), pool_id, new_pool_token), Err(DispatchError::Module { index: 0, error: 17, message: Some("MoreThanMaximumPassedInPoolTokenShares") }));
	
	});
}


#[test]
fn add_single_liquidity_given_amount_in_should_work() {
	new_test_ext().execute_with(|| {
		// initialize a pool.
		initialize_pool_for_dispatches();

		let alice = 1;
		let bob = 2;
		let creator = Origin::signed(bob);
		let pool_id = 0;
		let asud_id = 0;
		let ausd_type = TokenSymbol::from(asud_id);
		let asset_id = ausd_type;
		let token_amount_in = 5_000;

		assert_ok!(Swap::add_single_liquidity_given_amount_in(creator, pool_id, asset_id, token_amount_in));

		// check wehter the pool has added 603 shares.
		assert_eq!(Swap::pool_tokens_in_pool(pool_id), 1_603);

		// check wether bob has got 603 shares of pool.
		assert_eq!(Swap::user_pool_tokens_in_pool(bob, pool_id), 603);

		// check wether the token balances are right.
		assert_eq!(Swap::token_balances_in_pool(pool_id, ausd_type),5_500);

		// check wether bob's account has been deducted corresponding amount for ausd.
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(ausd_type, &bob).balance, 995_000);  // get the user's balance for ausd

		// Below are the incorrect operations.

		// // no such token. Right now it doesn't work for the type TokenSymbol. When id changes to asset id in the later version, this test should work.
		// let creator = Origin::signed(bob);
		// let asset_id = TokenSymbol::from(100);
		// assert_eq!(Swap::add_single_liquidity_given_amount_in(creator.clone(), pool_id, asset_id, token_amount_in), Err(DispatchError::Module { index: 0, error: 6, message: Some("TokenNotExist") }));

		// no such pool.
		let creator = Origin::signed(bob);
		let pool_id = 1;
		assert_eq!(Swap::add_single_liquidity_given_amount_in(creator.clone(), pool_id, asset_id, token_amount_in), Err(DispatchError::Module { index: 0, error: 0, message: Some("PoolNotExist") }));

		// pool not active.
		let pool_id = 0;
		assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
		assert_eq!(Swap::add_single_liquidity_given_amount_in(creator.clone(), pool_id, asset_id, token_amount_in), Err(DispatchError::Module { index: 0, error: 1, message: Some("PoolNotActive") }));

		// token in amount below or equal to zero.
		let token_amount_in = 0;
		assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
		assert_eq!(Swap::add_single_liquidity_given_amount_in(creator.clone(), pool_id, asset_id, token_amount_in), Err(DispatchError::Module { index: 0, error: 13, message: Some("AmountBelowZero") }));

		// deposit amount is more than what user has.
		let token_amount_in = 100_000_000;
		assert_eq!(Swap::add_single_liquidity_given_amount_in(creator.clone(), pool_id, asset_id, token_amount_in), Err(DispatchError::Module { index: 0, error: 3, message: Some("NotEnoughBalance") }));

		// less than the minimum share adding limit.
		let token_amount_in = 10;
		assert_eq!(Swap::add_single_liquidity_given_amount_in(creator.clone(), pool_id, asset_id, token_amount_in), Err(DispatchError::Module { index: 0, error: 5, message: Some("LessThanMinimumPassedInPoolTokenShares") }));
	});
}

#[test]
fn add_single_liquidity_given_shares_in_should_work() {
	new_test_ext().execute_with(|| {
		// initialize a pool.
		initialize_pool_for_dispatches();

		let alice = 1;
		let bob = 2;
		let creator = Origin::signed(bob);
		let pool_id = 0;
		let new_pool_token = 200;  // Alice initial pool token amount is 1000. Bob want's to get 20% of that of Alice's. 

		let asud_id = 0;
		let dot_id = 1;
		let ksm_id = 3;
		let ausd_type = TokenSymbol::from(asud_id);
		let dot_type = TokenSymbol::from(dot_id);
		let ksm_type = TokenSymbol::from(ksm_id);

		let asset_id = ausd_type;
		assert_ok!(Swap::add_single_liquidity_given_shares_in(creator, pool_id, asset_id, new_pool_token));  // Bob to get 200 share in the pool.

		// check wehter the pool has added 200 shares.ksm_type
		assert_eq!(Swap::pool_tokens_in_pool(pool_id), 1_200);

		// check wether bob has got 200 shares of pool.ksm_type
		assert_eq!(Swap::user_pool_tokens_in_pool(bob, pool_id), 200);

		// check wether the token balances are right.
		assert_eq!(Swap::token_balances_in_pool(pool_id, ausd_type),19_104);
		assert_eq!(Swap::token_balances_in_pool(pool_id, dot_type), 1_000);
		assert_eq!(Swap::token_balances_in_pool(pool_id, ksm_type), 400);

		// check wether bob's account has been deducted corresponding amount for different tokens.
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(ausd_type, &bob).balance, 981_396);  // get the user's balance for ausd
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(dot_type, &bob).balance, 1_000_000);  // get the user's balance for dot
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(ksm_type, &bob).balance, 1_000_000);  // get the user's balance for ksm

		// Below are the incorrect operations.

		// // no such token. Right now it doesn't work for the type TokenSymbol. When id changes to asset id in the later version, this test should work.
		// let creator = Origin::signed(bob);
		// let asset_id = TokenSymbol::from(100);
		// assert_eq!(Swap::add_single_liquidity_given_shares_in(creator.clone(), pool_id, asset_id, new_pool_token), Err(DispatchError::Module { index: 0, error: 6, message: Some("TokenNotExist") }));

		// no such pool.
		let creator = Origin::signed(bob);
		let pool_id = 1;
		assert_eq!(Swap::add_single_liquidity_given_shares_in(creator.clone(), pool_id, asset_id, new_pool_token), Err(DispatchError::Module { index: 0, error: 0, message: Some("PoolNotExist") }));

		// pool not active.
		let pool_id = 0;
		assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
		assert_eq!(Swap::add_single_liquidity_given_shares_in(creator.clone(), pool_id, asset_id, new_pool_token), Err(DispatchError::Module { index: 0, error: 1, message: Some("PoolNotActive") }));

		// less than the minimum share adding limit.
		let new_pool_token = 0;
		assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
		assert_eq!(Swap::add_single_liquidity_given_shares_in(creator.clone(), pool_id, asset_id, new_pool_token), Err(DispatchError::Module { index: 0, error: 13, message: Some("AmountBelowZero") }));


		// deposit pool token share is more than maximum added share limit.
		let new_pool_token = 100_000_000;
		assert_eq!(Swap::add_single_liquidity_given_shares_in(creator.clone(), pool_id, asset_id, new_pool_token), Err(DispatchError::Module { index: 0, error: 17, message: Some("MoreThanMaximumPassedInPoolTokenShares") }));

		// deposit pool token share is more than what user can afford.
		let new_pool_token = 1_000_000;
		assert_eq!(Swap::add_single_liquidity_given_shares_in(creator.clone(), pool_id, asset_id, new_pool_token), Err(DispatchError::Module { index: 0, error: 3, message: Some("NotEnoughBalance") }));
	});
}


#[test]
fn remove_single_asset_liquidity_given_shares_in_should_work(){
	new_test_ext().execute_with(|| {
		// initialize a pool.
		initialize_pool_for_dispatches();

		let alice = 1;
		let remover = Origin::signed(alice);
		let pool_id = 0;
		let pool_token_out = 200;  // Alice initial pool token amount is 1000. Bob want's to get 20% of that of Alice's. 

		let asud_id = 0;
		let dot_id = 1;
		let ksm_id = 3;
		let ausd_type = TokenSymbol::from(asud_id);
		let dot_type = TokenSymbol::from(dot_id);
		let ksm_type = TokenSymbol::from(ksm_id);

		let asset_id = ausd_type;
		assert_ok!(Swap::remove_single_asset_liquidity_given_shares_in(remover, pool_id, asset_id, pool_token_out));  // Alice to get 200 share out from the pool.
	
		// check wehter the pool has been withdrawled by 200 shares.
		assert_eq!(Swap::pool_tokens_in_pool(pool_id), 800);

		// // check wether Alice has got 200 shares of pool.ksm_type
		assert_eq!(Swap::user_pool_tokens_in_pool(alice, pool_id), 800);

		// check wether the token balances are right.
		assert_eq!(Swap::token_balances_in_pool(pool_id, ausd_type),178);
		assert_eq!(Swap::token_balances_in_pool(pool_id, dot_type), 1_000);
		assert_eq!(Swap::token_balances_in_pool(pool_id, ksm_type), 400);

		// check wether Alice's account has been added by corresponding amount for ausd.
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(ausd_type, &alice).balance, 9_822);  // get the user's balance for ausd
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(dot_type, &alice).balance, 29_000);  // get the user's balance for dot
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(ksm_type, &alice).balance, 29_600);  // get the user's balance for ksm

	
		// Below are the incorrect operations.

		// // no such token. Right now it doesn't work for the type TokenSymbol. When id changes to asset id in the later version, this test should work.
		// let creator = Origin::signed(alice);
		// let asset_id = TokenSymbol::from(100);
		// assert_eq!(Swap::remove_single_asset_liquidity_given_shares_in(creator.clone(), pool_id, asset_id, pool_token_out), Err(DispatchError::Module { index: 0, error: 6, message: Some("TokenNotExist") }));

		// no such pool.
		let remover = Origin::signed(alice);
		let pool_id = 1;
		assert_eq!(Swap::remove_single_asset_liquidity_given_shares_in(remover.clone(), pool_id, asset_id, pool_token_out), Err(DispatchError::Module { index: 0, error: 0, message: Some("PoolNotExist") }));

		// pool not active.
		let pool_id = 0;
		assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
		assert_eq!(Swap::remove_single_asset_liquidity_given_shares_in(remover.clone(), pool_id, asset_id, pool_token_out), Err(DispatchError::Module { index: 0, error: 1, message: Some("PoolNotActive") }));

		// less than the minimum share adding limit.
		let pool_token_out = 0;
		assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
		assert_eq!(Swap::remove_single_asset_liquidity_given_shares_in(remover.clone(), pool_id, asset_id, pool_token_out), Err(DispatchError::Module { index: 0, error: 5, message: Some("LessThanMinimumPassedInPoolTokenShares") }));

		// Bob not in the pool.
		let bob =2;
		let pool_token_out = 100;
		assert_eq!(Swap::remove_single_asset_liquidity_given_shares_in(Origin::signed(bob), pool_id, asset_id, pool_token_out), Err(DispatchError::Module { index: 0, error: 7, message: Some("UserNotInThePool") }));

		// deposit pool token share is more than maximum added share limit.
		let pool_token_out = 3_000_000;
		assert_eq!(Swap::remove_single_asset_liquidity_given_shares_in(remover.clone(), pool_id, asset_id, pool_token_out), Err(DispatchError::Module { index: 0, error: 17, message: Some("MoreThanMaximumPassedInPoolTokenShares") }));

		// deposit pool token share is more than what user can afford.
		let pool_token_out = 1_000_000;
		assert_eq!(Swap::remove_single_asset_liquidity_given_shares_in(remover.clone(), pool_id, asset_id, pool_token_out), Err(DispatchError::Module { index: 0, error: 3, message: Some("NotEnoughBalance") }));
	});
}


#[test]
fn remove_single_asset_liquidity_given_amount_in_should_work(){
	new_test_ext().execute_with(|| {
		// initialize a pool.
		initialize_pool_for_dispatches();

		let alice = 1;
		let remover = Origin::signed(alice);
		let pool_id = 0;
		let token_amount = 400;  // Alice initial pool token amount is 1000. 
		let asud_id = 0;
		let dot_id = 1;
		let ksm_id = 3;
		let ausd_type = TokenSymbol::from(asud_id);
		let dot_type = TokenSymbol::from(dot_id);
		let ksm_type = TokenSymbol::from(ksm_id);

		let asset_id = ausd_type;
		assert_ok!(Swap::remove_single_asset_liquidity_given_amount_in(remover, pool_id, asset_id, token_amount));  // Alice to get 200 share out from the pool.
	
		// check wehter the pool has added 603 shares.
		assert_eq!(Swap::pool_tokens_in_pool(pool_id), 692);

		// check wether bob has got 603 shares of pool.
		assert_eq!(Swap::user_pool_tokens_in_pool(alice, pool_id), 692);

		// check wether the token balances are right.
		assert_eq!(Swap::token_balances_in_pool(pool_id, ausd_type),100);

		// check wether bob's account has been deducted corresponding amount for ausd.
		assert_eq!(<Test as Trait>::AssetTrait::get_account_asset(ausd_type, &alice).balance, 9_900);  // get the user's balance for ausd

		// Below are the incorrect operations.

		// // no such token. Right now it doesn't work for the type TokenSymbol. When id changes to asset id in the later version, this test should work.
		// let remover = Origin::signed(alice);
		// let asset_id = TokenSymbol::from(100);
		// assert_eq!(Swap::remove_single_asset_liquidity_given_amount_in(remover, pool_id, asset_id, token_amount), Err(DispatchError::Module { index: 0, error: 6, message: Some("TokenNotExist") }));

		// no such pool.
		let remover = Origin::signed(alice);
		let pool_id = 1;
		assert_eq!(Swap::remove_single_asset_liquidity_given_amount_in(remover.clone(), pool_id, asset_id, token_amount), Err(DispatchError::Module { index: 0, error: 0, message: Some("PoolNotExist") }));

		// pool not active.
		let pool_id = 0;
		assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
		assert_eq!(Swap::remove_single_asset_liquidity_given_amount_in(remover.clone(), pool_id, asset_id, token_amount), Err(DispatchError::Module { index: 0, error: 1, message: Some("PoolNotActive") }));

		// token in amount below or equal to zero.
		let token_amount = 0;
		assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
		assert_eq!(Swap::remove_single_asset_liquidity_given_amount_in(remover.clone(), pool_id, asset_id, token_amount), Err(DispatchError::Module { index: 0, error: 13, message: Some("AmountBelowZero") }));

		// bob not in the pool.
		let bob = 2;
		let token_amount = 20;
		let remover = Origin::signed(bob);
		assert_eq!(Swap::remove_single_asset_liquidity_given_amount_in(remover.clone(), pool_id, asset_id, token_amount), Err(DispatchError::Module { index: 0, error: 7, message: Some("UserNotInThePool") }));

		// deposit amount is more than what user has.
		let remover = Origin::signed(alice);
		let token_amount = 2_000;
		assert_eq!(Swap::remove_single_asset_liquidity_given_amount_in(remover.clone(), pool_id, asset_id, token_amount), Err(DispatchError::Module { index: 0, error: 3, message: Some("NotEnoughBalance") }));

		// // less than the minimum share adding limit.
		// let token_amount = 10;
		// assert_eq!(Swap::remove_single_asset_liquidity_given_amount_in(remover.clone(), pool_id, asset_id, token_amount), Err(DispatchError::Module { index: 0, error: 5, message: Some("LessThanMinimumPassedInPoolTokenShares") }));


	});
}


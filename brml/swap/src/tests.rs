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

use crate::mock::*;
use crate::*;
use fixed_point::{traits::FromFixed, types::extra, FixedI128};
use float_cmp::approx_eq;
use frame_support::{assert_ok, dispatch::DispatchError};
use node_primitives::TokenType;

fn initialize_pool_for_dispatches() {
    // initialize token asset types.
    assert_ok!(Assets::create(
        Origin::root(),
        b"BNC".to_vec(),
        18,
        TokenType::Stable
    )); // Asset Id 0
    assert_ok!(Assets::create(
        Origin::root(),
        b"aUSD".to_vec(),
        18,
        TokenType::Stable
    )); // Asset Id 1
    assert_ok!(Assets::create_pair(Origin::root(), b"DOT".to_vec(), 18)); // Asset Id id 2,3
    assert_ok!(Assets::create_pair(Origin::root(), b"KSM".to_vec(), 18)); // Asset Id id 4,5
    assert_ok!(Assets::create_pair(Origin::root(), b"EOS".to_vec(), 18)); // Asset Id id 6,7
    assert_ok!(Assets::create_pair(Origin::root(), b"IOST".to_vec(), 18)); // Asset Id id 8,9

    // initialize some parameters used to dispatch the create_pool call.
    let alice = 1;
    let bob = 2;
    let asud_id = 1;
    let dot_id = 2;
    let ksm_id = 4;

    // issue tokens to Alice's account.
    assert_ok!(Assets::issue(Origin::root(), asud_id, alice, 10_000));
    assert_ok!(Assets::issue(Origin::root(), dot_id, alice, 30_000));
    assert_ok!(Assets::issue(Origin::root(), ksm_id, alice, 30_000));

    // issue tokens to Bob's account.
    assert_ok!(Assets::issue(Origin::root(), asud_id, bob, 1_000_000));
    assert_ok!(Assets::issue(Origin::root(), dot_id, bob, 1_000_000));
    assert_ok!(Assets::issue(Origin::root(), ksm_id, bob, 1_000_000));

    // initialize the parameters for create_pool.
    let creator = Origin::signed(alice);
    let swap_fee_rate = 5_000;

    let vec_node_1 = PoolCreateTokenDetails {
        token_id: asud_id,
        token_balance: 500,
        token_weight: 20,
    };

    let vec_node_2 = PoolCreateTokenDetails {
        token_id: dot_id,
        token_balance: 1_000,
        token_weight: 40,
    };

    let vec_node_3 = PoolCreateTokenDetails {
        token_id: ksm_id,
        token_balance: 400,
        token_weight: 40,
    };

    let token_for_pool_vec: Vec<
        PoolCreateTokenDetails<
            <Test as Trait>::AssetId,
            <Test as Trait>::Balance,
            <Test as Trait>::PoolWeight,
        >,
    > = vec![vec_node_1, vec_node_2, vec_node_3];
    run_to_block(2); // set the block number to 2.
                     // Dispatch the create_pool call to create a new pool.
    assert_ok!(Swap::create_pool(
        creator.clone(),
        swap_fee_rate,
        token_for_pool_vec
    ));

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
    approx_eq!(
        f32,
        0.1f32,
        f32::from_fixed(weight_ratio.unwrap()),
        epsilon = 0.000_000_000_001
    );
}

#[test]
fn calculate_out_given_in_should_work() {
    let token_balance_in = 1_000_000;
    let token_weight_in = 20_000;
    let token_amount_in = 1_000;
    let token_balance_out = 50_000_000;
    let token_weight_out = 40_000;
    let swap_fee = 5_000;

    let target = 6_450_675.3_655f32;
    let to_buy = Swap::calculate_out_given_in(
        token_balance_in,
        token_weight_in,
        token_amount_in,
        token_balance_out,
        token_weight_out,
        swap_fee,
    );
    assert!(to_buy.is_ok());

    let to_buy = to_buy.map(f32::from_fixed).unwrap();
    approx_eq!(f32, target, to_buy, epsilon = 0.000_000_000_001);
}

#[test]
fn create_pool_should_work() {
    new_test_ext().execute_with(|| {
        // initialize some parameters used to dispatch the create_pool call.
        let alice = 1;
        let asud_id = 1;
        let dot_id = 2;
        let ksm_id = 4;

        // initialize token asset types.
        assert_ok!(Assets::create(
            Origin::root(),
            b"BNC".to_vec(),
            18,
            TokenType::Stable
        )); // Asset Id 0
        assert_ok!(Assets::create(
            Origin::root(),
            b"aUSD".to_vec(),
            18,
            TokenType::Stable
        )); // Asset Id 1
        assert_ok!(Assets::create_pair(Origin::root(), b"DOT".to_vec(), 18)); // Asset Id id 2,3
        assert_ok!(Assets::create_pair(Origin::root(), b"KSM".to_vec(), 18)); // Asset Id id 4,5
        assert_ok!(Assets::create_pair(Origin::root(), b"EOS".to_vec(), 18)); // Asset Id id 6,7
        assert_ok!(Assets::create_pair(Origin::root(), b"IOST".to_vec(), 18)); // Asset Id id 8,9

        // issue tokens to Alice's account.
        assert_ok!(Assets::issue(Origin::root(), asud_id, alice, 10_000));
        assert_ok!(Assets::issue(Origin::root(), dot_id, alice, 30_000));
        assert_ok!(Assets::issue(Origin::root(), ksm_id, alice, 30_000));

        // initialize the parameters for create_pool.
        let creator = Origin::signed(alice);
        let swap_fee_rate = 5_000;

        let vec_node_1 = PoolCreateTokenDetails {
            token_id: asud_id,
            token_balance: 500,
            token_weight: 20,
        };

        let vec_node_2 = PoolCreateTokenDetails {
            token_id: dot_id,
            token_balance: 1_000,
            token_weight: 40,
        };

        let vec_node_3 = PoolCreateTokenDetails {
            token_id: ksm_id,
            token_balance: 400,
            token_weight: 40,
        };

        let token_for_pool_vec: Vec<
            PoolCreateTokenDetails<
                <Test as Trait>::AssetId,
                <Test as Trait>::Balance,
                <Test as Trait>::PoolWeight,
            >,
        > = vec![vec_node_1.clone(), vec_node_2.clone(), vec_node_3.clone()];
        run_to_block(2); // set the block number to 2.

        // Dispatch the create_pool call to create a new pool.
        assert_ok!(Swap::create_pool(
            creator,
            swap_fee_rate,
            token_for_pool_vec
        ));

        // validate the value of storage Pools.
        let result = Swap::pools(0);
        assert_eq!(result.owner, alice); // Validate that the pool owner is Alice.
        assert_eq!(result.swap_fee_rate, 5_000); // Validate the swap fee is 5000.
        assert_eq!(result.active, false); // Validate the initial value of pool state is inactive.

        // validate the value of storage TokenWeightsInPool.
        assert_eq!(Swap::token_weights_in_pool(0, asud_id), 20_000); // the weight of ausd token
        assert_eq!(Swap::token_weights_in_pool(0, dot_id), 40_000); // the weight of dot token
        assert_eq!(Swap::token_weights_in_pool(0, ksm_id), 40_000); // the weight of ksm token

        // validate the value of storage TokenBalancesInPool.
        assert_eq!(Swap::token_balances_in_pool(0, asud_id), 500); // the balance of ausd token
        assert_eq!(Swap::token_balances_in_pool(0, dot_id), 1_000); // the balance of dot token
        assert_eq!(Swap::token_balances_in_pool(0, ksm_id), 400); // the balance of ksm token

        // validate the value of storage PoolTokensInPool.
        assert_eq!(Swap::pool_tokens_in_pool(0), 1_000); // the pool token balance of pool 0

        // validate the value of storage UserPoolTokensInPool.
        assert_eq!(Swap::user_pool_tokens_in_pool(alice, 0), 1_000); // the pool token balance of alice in pool 0

        // validate the value of storage UserUnclaimedBonusInPool.
        assert_eq!(Swap::user_unclaimed_bonus_in_pool(alice, 0), (0, 2));

        // validate the value of storage DeductedBonusAmountInPool.
        assert_eq!(Swap::deducted_bonus_amount_in_pool(0), 0); // the deducted bonus amount of pool 0.

        // Below are the incorrect operations.
        let bob = 2;
        let creator = Origin::signed(bob);

        // issue tokens to Alice's account.
        assert_ok!(Assets::issue(Origin::root(), asud_id, alice, 10_000));
        assert_ok!(Assets::issue(Origin::root(), dot_id, alice, 30_000));
        assert_ok!(Assets::issue(Origin::root(), ksm_id, alice, 30_000));

        // swap fee rate exceeds 100%.
        let swap_fee_rate = 500_000;
        let token_for_pool_vec: Vec<
            PoolCreateTokenDetails<
                <Test as Trait>::AssetId,
                <Test as Trait>::Balance,
                <Test as Trait>::PoolWeight,
            >,
        > = vec![vec_node_1.clone(), vec_node_2.clone(), vec_node_3.clone()];
        assert_eq!(
            Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec),
            Err(DispatchError::Module {
                index: 0,
                error: 15,
                message: Some("FeeRateExceedMaximumLimit")
            })
        );

        // swap fee rate is below 0%.
        let swap_fee_rate = 0;
        let token_for_pool_vec: Vec<
            PoolCreateTokenDetails<
                <Test as Trait>::AssetId,
                <Test as Trait>::Balance,
                <Test as Trait>::PoolWeight,
            >,
        > = vec![vec_node_1.clone(), vec_node_2.clone(), vec_node_3.clone()];
        assert_eq!(
            Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec),
            Err(DispatchError::Module {
                index: 0,
                error: 14,
                message: Some("FeeRateExceedMinimumLimit")
            })
        );

        // the length of the vector is 9, which exceeds the biggest supported token number in the pool.
        let swap_fee_rate = 10_000;
        let token_for_pool_vec: Vec<
            PoolCreateTokenDetails<
                <Test as Trait>::AssetId,
                <Test as Trait>::Balance,
                <Test as Trait>::PoolWeight,
            >,
        > = vec![
            vec_node_1.clone(),
            vec_node_1.clone(),
            vec_node_1.clone(),
            vec_node_1.clone(),
            vec_node_1.clone(),
            vec_node_1.clone(),
            vec_node_1.clone(),
            vec_node_1.clone(),
            vec_node_1.clone(),
        ];
        assert_eq!(
            Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec),
            Err(DispatchError::Module {
                index: 0,
                error: 6,
                message: Some("TooManyTokensToPool")
            })
        );

        // validate the tokens used in creating a pool exist. Right now it doesn't work for the type Asset Id.
        // When id changes to asset id in the later version, this test should work.
        let vec_node_4 = PoolCreateTokenDetails {
            token_id: 78,
            token_balance: 400,
            token_weight: 40,
        };
        let swap_fee_rate = 10_000;
        let token_for_pool_vec: Vec<
            PoolCreateTokenDetails<
                <Test as Trait>::AssetId,
                <Test as Trait>::Balance,
                <Test as Trait>::PoolWeight,
            >,
        > = vec![
            vec_node_1.clone(),
            vec_node_2.clone(),
            vec_node_3.clone(),
            vec_node_4.clone(),
        ];
        assert_eq!(
            Swap::create_pool(Origin::signed(alice), swap_fee_rate, token_for_pool_vec),
            Err(DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("TokenNotExist")
            })
        );

        // validate token amount used to create a pool must be bigger than zero.
        let vec_node_4 = PoolCreateTokenDetails {
            token_id: dot_id,
            token_balance: 0,
            token_weight: 40,
        };
        let swap_fee_rate = 10_000;
        assert_ok!(Assets::issue(Origin::root(), asud_id, bob, 1_000));
        assert_ok!(Assets::issue(Origin::root(), dot_id, bob, 100));
        let token_for_pool_vec: Vec<
            PoolCreateTokenDetails<
                <Test as Trait>::AssetId,
                <Test as Trait>::Balance,
                <Test as Trait>::PoolWeight,
            >,
        > = vec![vec_node_1.clone(), vec_node_4.clone()];
        assert_eq!(
            Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec),
            Err(DispatchError::Module {
                index: 0,
                error: 13,
                message: Some("AmountBelowZero")
            })
        );

        // validate token amount used to create a pool must not exceed user's balance.
        let vec_node_4 = PoolCreateTokenDetails {
            token_id: dot_id,
            token_balance: 1_000,
            token_weight: 40,
        };
        let swap_fee_rate = 10_000;
        assert_ok!(Assets::issue(Origin::root(), asud_id, bob, 1_000));
        assert_ok!(Assets::issue(Origin::root(), dot_id, bob, 100));
        let token_for_pool_vec: Vec<
            PoolCreateTokenDetails<
                <Test as Trait>::AssetId,
                <Test as Trait>::Balance,
                <Test as Trait>::PoolWeight,
            >,
        > = vec![vec_node_1.clone(), vec_node_4.clone()];
        assert_eq!(
            Swap::create_pool(creator.clone(), swap_fee_rate, token_for_pool_vec),
            Err(DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotEnoughBalance")
            })
        );
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
        let new_pool_token = 200; // Alice initial pool token amount is 1000. Bob want's to get 20% of that of Alice's.

        let asud_id = 1;
        let dot_id = 2;
        let ksm_id = 4;

        assert_ok!(Swap::add_liquidity_given_shares_in(
            creator,
            pool_id,
            new_pool_token
        )); // Bob to get 200 share in the pool.

        // check whether the pool has added 200 shares.]
        assert_eq!(Swap::pool_tokens_in_pool(pool_id), 1_200);

        // check wether bob has got 200 shares of pool.
        assert_eq!(Swap::user_pool_tokens_in_pool(bob, pool_id), 200);

        // check wether the token balances are right.
        assert_eq!(Swap::token_balances_in_pool(pool_id, asud_id), 600);
        assert_eq!(Swap::token_balances_in_pool(pool_id, dot_id), 1_200);
        assert_eq!(Swap::token_balances_in_pool(pool_id, ksm_id), 480);

        // check wether bob's account has been deducted corresponding amount for different tokens.
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(asud_id, &bob).available,
            999_900
        ); // get the user's balance for ausd
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(dot_id, &bob).available,
            999_800
        ); // get the user's balance for dot
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(ksm_id, &bob).available,
            999_920
        ); // get the user's balance for ksm

        // Below are the incorrect operations.

        // no such pool_id
        let pool_id = 1;
        let creator = Origin::signed(bob);
        assert_eq!(
            Swap::add_liquidity_given_shares_in(creator.clone(), pool_id, new_pool_token),
            Err(DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("PoolNotExist")
            })
        );

        // pool status is false, thus it's not able to add liquidity in.
        let pool_id = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
        assert_eq!(
            Swap::add_liquidity_given_shares_in(creator.clone(), pool_id, new_pool_token),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("PoolNotActive")
            })
        );

        // Every time a user at least adds more than or equal to the MinimumAddedPoolTokenShares
        let new_pool_token = 1;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        assert_eq!(
            Swap::add_liquidity_given_shares_in(creator.clone(), pool_id, new_pool_token),
            Err(DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("LessThanMinimumPassedInPoolTokenShares")
            })
        );

        // Every time a user at least adds more than or equal to the MinimumAddedPoolTokenShares
        let new_pool_token = 1_000_000;
        assert_eq!(
            Swap::add_liquidity_given_shares_in(creator.clone(), pool_id, new_pool_token),
            Err(DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotEnoughBalance")
            })
        );

        // deposit pool token share is more than maximum added share limit.
        let new_pool_token = 100_000_000;
        assert_eq!(
            Swap::add_liquidity_given_shares_in(creator.clone(), pool_id, new_pool_token),
            Err(DispatchError::Module {
                index: 0,
                error: 17,
                message: Some("MoreThanMaximumPassedInPoolTokenShares")
            })
        );
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
        let asud_id = 1;
        let asset_id = asud_id;
        let token_amount_in = 5_000;

        assert_ok!(Swap::add_single_liquidity_given_amount_in(
            creator,
            pool_id,
            asset_id,
            token_amount_in
        ));

        // check whether the pool has added 603 shares.
        assert_eq!(Swap::pool_tokens_in_pool(pool_id), 1_603);

        // check wether bob has got 603 shares of pool.
        assert_eq!(Swap::user_pool_tokens_in_pool(bob, pool_id), 603);

        // check wether the token balances are right.
        assert_eq!(Swap::token_balances_in_pool(pool_id, asud_id), 5_500);

        // check wether bob's account has been deducted corresponding amount for ausd.
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(asud_id, &bob).available,
            995_000
        ); // get the user's balance for ausd

        // Below are the incorrect operations.

        // When id changes to asset id in the later version, this test should work.
        let creator = Origin::signed(bob);
        let asset_id = 100;
        assert_eq!(
            Swap::add_single_liquidity_given_amount_in(
                creator.clone(),
                pool_id,
                asset_id,
                token_amount_in
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("TokenNotExist")
            })
        );

        // no such pool.
        let creator = Origin::signed(bob);
        let pool_id = 1;
        let asset_id = asud_id;
        assert_eq!(
            Swap::add_single_liquidity_given_amount_in(
                creator.clone(),
                pool_id,
                asset_id,
                token_amount_in
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("PoolNotExist")
            })
        );

        // pool not active.
        let pool_id = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
        assert_eq!(
            Swap::add_single_liquidity_given_amount_in(
                creator.clone(),
                pool_id,
                asset_id,
                token_amount_in
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("PoolNotActive")
            })
        );

        // token in amount below or equal to zero.
        let token_amount_in = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        assert_eq!(
            Swap::add_single_liquidity_given_amount_in(
                creator.clone(),
                pool_id,
                asset_id,
                token_amount_in
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 13,
                message: Some("AmountBelowZero")
            })
        );

        // deposit amount is more than what user has.
        let token_amount_in = 100_000_000;
        assert_eq!(
            Swap::add_single_liquidity_given_amount_in(
                creator.clone(),
                pool_id,
                asset_id,
                token_amount_in
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotEnoughBalance")
            })
        );

        // less than the minimum share adding limit.
        let token_amount_in = 10;
        assert_eq!(
            Swap::add_single_liquidity_given_amount_in(
                creator.clone(),
                pool_id,
                asset_id,
                token_amount_in
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("LessThanMinimumPassedInPoolTokenShares")
            })
        );
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
        let new_pool_token = 200; // Alice initial pool token amount is 1000. Bob want's to get 20% of that of Alice's.

        let asud_id = 1;
        let dot_id = 2;
        let ksm_id = 4;

        let asset_id = asud_id;
        assert_ok!(Swap::add_single_liquidity_given_shares_in(
            creator,
            pool_id,
            asset_id,
            new_pool_token
        )); // Bob to get 200 share in the pool.

        // check whether the pool has added 200 shares.ksm_id
        assert_eq!(Swap::pool_tokens_in_pool(pool_id), 1_200);

        // check wether bob has got 200 shares of pool.ksm_id
        assert_eq!(Swap::user_pool_tokens_in_pool(bob, pool_id), 200);

        // check wether the token balances are right.
        assert_eq!(Swap::token_balances_in_pool(pool_id, asud_id), 19_104);
        assert_eq!(Swap::token_balances_in_pool(pool_id, dot_id), 1_000);
        assert_eq!(Swap::token_balances_in_pool(pool_id, ksm_id), 400);

        // check wether bob's account has been deducted corresponding amount for different tokens.
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(asud_id, &bob).available,
            981_396
        ); // get the user's balance for ausd
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(dot_id, &bob).available,
            1_000_000
        ); // get the user's balance for dot
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(ksm_id, &bob).available,
            1_000_000
        ); // get the user's balance for ksm

        // Below are the incorrect operations.

        // When id changes to asset id in the later version, this test should work.
        let creator = Origin::signed(bob);
        let asset_id = 100;
        assert_eq!(
            Swap::add_single_liquidity_given_shares_in(
                creator.clone(),
                pool_id,
                asset_id,
                new_pool_token
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("TokenNotExist")
            })
        );

        // no such pool.
        let creator = Origin::signed(bob);
        let pool_id = 1;
        let asset_id = asud_id;
        assert_eq!(
            Swap::add_single_liquidity_given_shares_in(
                creator.clone(),
                pool_id,
                asset_id,
                new_pool_token
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("PoolNotExist")
            })
        );

        // pool not active.
        let pool_id = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
        assert_eq!(
            Swap::add_single_liquidity_given_shares_in(
                creator.clone(),
                pool_id,
                asset_id,
                new_pool_token
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("PoolNotActive")
            })
        );

        // less than the minimum share adding limit.
        let new_pool_token = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        assert_eq!(
            Swap::add_single_liquidity_given_shares_in(
                creator.clone(),
                pool_id,
                asset_id,
                new_pool_token
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("LessThanMinimumPassedInPoolTokenShares")
            })
        );

        // deposit pool token share is more than maximum added share limit.
        let new_pool_token = 100_000_000;
        assert_eq!(
            Swap::add_single_liquidity_given_shares_in(
                creator.clone(),
                pool_id,
                asset_id,
                new_pool_token
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 17,
                message: Some("MoreThanMaximumPassedInPoolTokenShares")
            })
        );

        // deposit pool token share is more than what user can afford.
        let new_pool_token = 1_000_000;
        assert_eq!(
            Swap::add_single_liquidity_given_shares_in(
                creator.clone(),
                pool_id,
                asset_id,
                new_pool_token
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotEnoughBalance")
            })
        );
    });
}

#[test]
fn remove_single_asset_liquidity_given_shares_in_should_work() {
    new_test_ext().execute_with(|| {
        // initialize a pool.
        initialize_pool_for_dispatches();

        let alice = 1;
        let remover = Origin::signed(alice);
        let pool_id = 0;
        let pool_token_out = 200; // Alice initial pool token amount is 1000. Bob want's to get 20% of that of Alice's.

        let asud_id = 1;
        let dot_id = 2;
        let ksm_id = 4;

        let asset_id = asud_id;
        assert_ok!(Swap::remove_single_asset_liquidity_given_shares_in(
            remover,
            pool_id,
            asset_id,
            pool_token_out
        )); // Alice to get 200 share out from the pool.

        // check whether the pool has been withdrew by 200 shares.
        assert_eq!(Swap::pool_tokens_in_pool(pool_id), 800);

        // // check wether Alice has got 200 shares of pool.ksm_id
        assert_eq!(Swap::user_pool_tokens_in_pool(alice, pool_id), 800);

        // check wether the token balances are right.
        assert_eq!(Swap::token_balances_in_pool(pool_id, asud_id), 178);
        assert_eq!(Swap::token_balances_in_pool(pool_id, dot_id), 1_000);
        assert_eq!(Swap::token_balances_in_pool(pool_id, ksm_id), 400);

        // check wether Alice's account has been added by corresponding amount for ausd.
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(asud_id, &alice).available,
            9_822
        ); // get the user's balance for ausd
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(dot_id, &alice).available,
            29_000
        ); // get the user's balance for dot
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(ksm_id, &alice).available,
            29_600
        ); // get the user's balance for ksm

        // Below are the incorrect operations.

        // When id changes to asset id in the later version, this test should work.
        let creator = Origin::signed(alice);
        let asset_id = 100;
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_shares_in(
                creator.clone(),
                pool_id,
                asset_id,
                pool_token_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("TokenNotExist")
            })
        );

        // no such pool.
        let remover = Origin::signed(alice);
        let pool_id = 1;
        let asset_id = asud_id;
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_shares_in(
                remover.clone(),
                pool_id,
                asset_id,
                pool_token_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("PoolNotExist")
            })
        );

        // pool not active.
        let pool_id = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_shares_in(
                remover.clone(),
                pool_id,
                asset_id,
                pool_token_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("PoolNotActive")
            })
        );

        // less than the minimum share adding limit.
        let pool_token_out = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_shares_in(
                remover.clone(),
                pool_id,
                asset_id,
                pool_token_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("LessThanMinimumPassedInPoolTokenShares")
            })
        );

        // Bob not in the pool.
        let bob = 2;
        let pool_token_out = 100;
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_shares_in(
                Origin::signed(bob),
                pool_id,
                asset_id,
                pool_token_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 7,
                message: Some("UserNotInThePool")
            })
        );

        // deposit pool token share is more than maximum added share limit.
        let pool_token_out = 3_000_000;
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_shares_in(
                remover.clone(),
                pool_id,
                asset_id,
                pool_token_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 17,
                message: Some("MoreThanMaximumPassedInPoolTokenShares")
            })
        );

        // deposit pool token share is more than what user can afford.
        let pool_token_out = 1_000_000;
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_shares_in(
                remover.clone(),
                pool_id,
                asset_id,
                pool_token_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotEnoughBalance")
            })
        );
    });
}

#[test]
fn remove_single_asset_liquidity_given_amount_in_should_work() {
    new_test_ext().execute_with(|| {
        // initialize a pool.
        initialize_pool_for_dispatches();

        let alice = 1;
        let remover = Origin::signed(alice);
        let pool_id = 0;
        let token_amount = 400; // Alice initial pool token amount is 1000.
        let asud_id = 1;
        let asset_id = asud_id;
        assert_ok!(Swap::remove_single_asset_liquidity_given_amount_in(
            remover,
            pool_id,
            asset_id,
            token_amount
        )); // Alice to get 200 share out from the pool.

        // check whether the pool has added 603 shares.
        assert_eq!(Swap::pool_tokens_in_pool(pool_id), 692);

        // check wether bob has got 603 shares of pool.
        assert_eq!(Swap::user_pool_tokens_in_pool(alice, pool_id), 692);

        // check wether the token balances are right.
        assert_eq!(Swap::token_balances_in_pool(pool_id, asud_id), 100);

        // check wether bob's account has been deducted corresponding amount for ausd.
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(asud_id, &alice).available,
            9_900
        ); // get the user's balance for ausd

        // Below are the incorrect operations.

        // When id changes to asset id in the later version, this test should work.
        let remover = Origin::signed(alice);
        let asset_id = 100;
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_amount_in(
                remover,
                pool_id,
                asset_id,
                token_amount
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("TokenNotExist")
            })
        );

        // no such pool.
        let remover = Origin::signed(alice);
        let pool_id = 1;
        let asset_id = asud_id;
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_amount_in(
                remover.clone(),
                pool_id,
                asset_id,
                token_amount
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("PoolNotExist")
            })
        );

        // pool not active.
        let pool_id = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_amount_in(
                remover.clone(),
                pool_id,
                asset_id,
                token_amount
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("PoolNotActive")
            })
        );

        // token in amount below or equal to zero.
        let token_amount = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_amount_in(
                remover.clone(),
                pool_id,
                asset_id,
                token_amount
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 13,
                message: Some("AmountBelowZero")
            })
        );

        // bob not in the pool.
        let bob = 2;
        let token_amount = 20;
        let remover = Origin::signed(bob);
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_amount_in(
                remover.clone(),
                pool_id,
                asset_id,
                token_amount
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 7,
                message: Some("UserNotInThePool")
            })
        );

        // deposit amount is more than what user has.
        let remover = Origin::signed(alice);
        let token_amount = 2_000;
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_amount_in(
                remover.clone(),
                pool_id,
                asset_id,
                token_amount
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotEnoughBalance")
            })
        );

        // pool token taken out is more than what the user has.
        let token_amount = 100;
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_amount_in(
                remover.clone(),
                pool_id,
                asset_id,
                token_amount
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotEnoughBalance")
            })
        );

        // less than the minimum pool share limit when adding of removing.
        let token_amount = 1;
        assert_eq!(
            Swap::remove_single_asset_liquidity_given_amount_in(
                remover.clone(),
                pool_id,
                asset_id,
                token_amount
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("LessThanMinimumPassedInPoolTokenShares")
            })
        );
    });
}

#[test]
fn remove_assets_liquidity_given_shares_in_should_work() {
    new_test_ext().execute_with(|| {
        // initialize a pool.
        initialize_pool_for_dispatches();

        let alice = 1;
        let remover = Origin::signed(alice);
        let pool_id = 0;
        let asud_id = 1;
        let dot_id = 2;
        let ksm_id = 4;

        let pool_amount_out = 500; // The pool token share that Alice wants to withdraw from the pool.
        assert_ok!(Swap::remove_assets_liquidity_given_shares_in(
            remover,
            pool_id,
            pool_amount_out
        )); // Alice to get 500 share out from the pool.

        // check whether the pool has added 400 shares.
        assert_eq!(Swap::pool_tokens_in_pool(pool_id), 500);

        // check wether alice has got 500 shares of pool.
        assert_eq!(Swap::user_pool_tokens_in_pool(alice, pool_id), 500);

        // check wether the token balances are right.
        assert_eq!(Swap::token_balances_in_pool(pool_id, asud_id), 250);
        assert_eq!(Swap::token_balances_in_pool(pool_id, dot_id), 500);
        assert_eq!(Swap::token_balances_in_pool(pool_id, ksm_id), 200);

        // check wether Alice's account has been added corresponding amount for different tokens.
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(asud_id, &alice).available,
            9_750
        ); // get the user's balance for ausd
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(dot_id, &alice).available,
            29_500
        ); // get the user's balance for dot
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(ksm_id, &alice).available,
            29_800
        ); // get the user's balance for ksm

        // Below are the incorrect operations.

        // no such pool_id
        let pool_id = 1;
        let remover = Origin::signed(alice);
        assert_eq!(
            Swap::remove_assets_liquidity_given_shares_in(
                remover.clone(),
                pool_id,
                pool_amount_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("PoolNotExist")
            })
        );

        // pool status is false, thus it's not able to add liquidity in.
        let pool_id = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
        assert_eq!(
            Swap::remove_assets_liquidity_given_shares_in(
                remover.clone(),
                pool_id,
                pool_amount_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("PoolNotActive")
            })
        );

        // bob not in the pool
        let bob = 2;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        let remover = Origin::signed(bob);
        assert_eq!(
            Swap::remove_assets_liquidity_given_shares_in(
                remover.clone(),
                pool_id,
                pool_amount_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 7,
                message: Some("UserNotInThePool")
            })
        );

        // Every time a user at least adds more than or equal to the MinimumAddedPoolTokenShares
        let pool_amount_out = 1;
        let remover = Origin::signed(alice);
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        assert_eq!(
            Swap::remove_assets_liquidity_given_shares_in(
                remover.clone(),
                pool_id,
                pool_amount_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("LessThanMinimumPassedInPoolTokenShares")
            })
        );

        // Every time a user at least adds more than or equal to the MinimumAddedPoolTokenShares
        let pool_amount_out = 1_000_000;
        assert_eq!(
            Swap::remove_assets_liquidity_given_shares_in(
                remover.clone(),
                pool_id,
                pool_amount_out
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotEnoughBalance")
            })
        );
    });
}

#[test]
fn swap_exact_in_should_work() {
    new_test_ext().execute_with(|| {
        // initialize a pool.
        initialize_pool_for_dispatches();

        let alice = 1;
        let bob = 2;
        let swapper = Origin::signed(bob);
        let pool_id = 0;
        let asud_id = 1;
        let dot_id = 2;
        let ksm_id = 4;

        let token_in_asset_id = asud_id;
        let token_out_asset_id = dot_id;
        let token_amount_in = 100;
        let min_token_amount_out = Some(1);
        assert_ok!(Swap::swap_exact_in(
            swapper,
            pool_id,
            token_in_asset_id,
            token_amount_in,
            min_token_amount_out,
            token_out_asset_id
        )); // Bob to swap 100 ausd for x amount of dot.

        // check whether the pool has added 100 ausd.
        assert_eq!(
            Swap::token_balances_in_pool(pool_id, token_in_asset_id),
            600
        );
        assert_eq!(
            Swap::token_balances_in_pool(pool_id, token_out_asset_id),
            917
        );
        assert_eq!(Swap::token_balances_in_pool(pool_id, ksm_id), 400);

        // check whether bob's account has been added and deducted with corresponding amounts.
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(asud_id, &bob).available,
            999_900
        ); // get the user's balance for ausd
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(dot_id, &bob).available,
            1_000_083
        ); // get the user's balance for dot
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(ksm_id, &bob).available,
            1_000_000
        ); // get the user's balance for dot

        // Below are the incorrect operations.

        // token-in asset is the same as the token-out asset.
        let swapper = Origin::signed(bob);
        assert_eq!(
            Swap::swap_exact_in(
                swapper.clone(),
                pool_id,
                token_in_asset_id,
                token_amount_in,
                min_token_amount_out,
                token_in_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 8,
                message: Some("ForbidSameTokenSwap")
            })
        );

        //  When id changes to asset id in the later version, this test should work.
        let asset_id = 100;
        assert_eq!(
            Swap::swap_exact_in(
                swapper.clone(),
                pool_id,
                asset_id,
                token_amount_in,
                min_token_amount_out,
                token_in_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("TokenNotExist")
            })
        );
        assert_eq!(
            Swap::swap_exact_in(
                swapper.clone(),
                pool_id,
                token_in_asset_id,
                token_amount_in,
                min_token_amount_out,
                asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("TokenNotExist")
            })
        );

        // no such pool_id
        let pool_id = 1;
        assert_eq!(
            Swap::swap_exact_in(
                swapper.clone(),
                pool_id,
                token_in_asset_id,
                token_amount_in,
                min_token_amount_out,
                token_out_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("PoolNotExist")
            })
        );

        // pool status is false, thus it's not able to add liquidity in.
        let pool_id = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
        assert_eq!(
            Swap::swap_exact_in(
                swapper.clone(),
                pool_id,
                token_in_asset_id,
                token_amount_in,
                min_token_amount_out,
                token_out_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("PoolNotActive")
            })
        );

        // swap more than the user has
        let token_amount_in = 9_000_000;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        assert_eq!(
            Swap::swap_exact_in(
                swapper.clone(),
                pool_id,
                token_in_asset_id,
                token_amount_in,
                min_token_amount_out,
                token_out_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotEnoughBalance")
            })
        );

        // swap more than the pool size
        let token_amount_in = 800;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        assert_eq!(
            Swap::swap_exact_in(
                swapper.clone(),
                pool_id,
                token_in_asset_id,
                token_amount_in,
                min_token_amount_out,
                token_out_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 10,
                message: Some("ExceedMaximumSwapInRatio")
            })
        );

        // less than the passed in minimum token out amount
        let min_token_amount_out = Some(500);
        let token_amount_in = 200;
        assert_eq!(
            Swap::swap_exact_in(
                swapper.clone(),
                pool_id,
                token_in_asset_id,
                token_amount_in,
                min_token_amount_out,
                token_out_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 11,
                message: Some("LessThanExpectedAmount")
            })
        );
    });
}

#[test]
fn swap_exact_out_should_work() {
    new_test_ext().execute_with(|| {
        // initialize a pool.
        initialize_pool_for_dispatches();

        let alice = 1;
        let bob = 2;
        let swapper = Origin::signed(bob);
        let pool_id = 0;
        let asud_id = 1;
        let dot_id = 2;
        let ksm_id = 4;

        let token_in_asset_id = asud_id;
        let token_out_asset_id = dot_id;
        let token_amount_out = 200;
        let max_token_amount_in = Some(1000);

        assert_ok!(Swap::swap_exact_out(
            swapper,
            pool_id,
            token_out_asset_id,
            token_amount_out,
            max_token_amount_in,
            token_in_asset_id
        )); // Bob to get 200 dot out from the pool.

        // check whether the pool has added 400 shares.
        assert_eq!(
            Swap::token_balances_in_pool(pool_id, token_out_asset_id),
            800
        );
        assert_eq!(
            Swap::token_balances_in_pool(pool_id, token_in_asset_id),
            562
        );
        assert_eq!(Swap::token_balances_in_pool(pool_id, ksm_id), 400);

        // check whether bob's account has been added and deducted with corresponding amounts.
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(asud_id, &bob).available,
            999_938
        ); // get the user's balance for ausd
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(dot_id, &bob).available,
            1_000_200
        ); // get the user's balance for dot
        assert_eq!(
            <Test as Trait>::AssetTrait::get_account_asset(ksm_id, &bob).available,
            1_000_000
        ); // get the user's balance for ksm

        // // Below are the incorrect operations.

        // token-in asset is the same as the token-out asset.
        let swapper = Origin::signed(bob);
        assert_eq!(
            Swap::swap_exact_out(
                swapper.clone(),
                pool_id,
                token_out_asset_id,
                token_amount_out,
                max_token_amount_in,
                token_out_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 8,
                message: Some("ForbidSameTokenSwap")
            })
        );

        //  When id changes to asset id in the later version, this test should work.
        let asset_id = 100;
        assert_eq!(
            Swap::swap_exact_out(
                swapper.clone(),
                pool_id,
                token_out_asset_id,
                token_amount_out,
                max_token_amount_in,
                asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("TokenNotExist")
            })
        );
        assert_eq!(
            Swap::swap_exact_out(
                swapper.clone(),
                pool_id,
                asset_id,
                token_amount_out,
                max_token_amount_in,
                token_in_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 2,
                message: Some("TokenNotExist")
            })
        );

        // no such pool_id
        let pool_id = 1;
        assert_eq!(
            Swap::swap_exact_out(
                swapper.clone(),
                pool_id,
                token_out_asset_id,
                token_amount_out,
                max_token_amount_in,
                token_in_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("PoolNotExist")
            })
        );

        // pool status is false, thus it's not able to add liquidity in.
        let pool_id = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
        assert_eq!(
            Swap::swap_exact_out(
                swapper.clone(),
                pool_id,
                token_out_asset_id,
                token_amount_out,
                max_token_amount_in,
                token_in_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("PoolNotActive")
            })
        );

        // swap more than the pool size
        let token_amount_out = 2000;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        assert_eq!(
            Swap::swap_exact_out(
                swapper.clone(),
                pool_id,
                token_out_asset_id,
                token_amount_out,
                max_token_amount_in,
                token_in_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 10,
                message: Some("ExceedMaximumSwapInRatio")
            })
        );

        // less than the passed in minimum token out amount
        let max_token_amount_in = Some(20);
        let token_amount_out = 400;
        assert_eq!(
            Swap::swap_exact_out(
                swapper.clone(),
                pool_id,
                token_out_asset_id,
                token_amount_out,
                max_token_amount_in,
                token_in_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 12,
                message: Some("BiggerThanExpectedAmount")
            })
        );

        // swap more than the user has
        let max_token_amount_in = Some(2000);
        let token_amount_out = 400;
        <Test as Trait>::AssetTrait::asset_redeem(token_in_asset_id, &bob, 999_800); // destroy most of bob's ausd
        assert_eq!(
            Swap::swap_exact_out(
                swapper.clone(),
                pool_id,
                token_out_asset_id,
                token_amount_out,
                max_token_amount_in,
                token_in_asset_id
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("NotEnoughBalance")
            })
        );
    });
}

#[test]
fn claim_bonus_should_work() {
    new_test_ext().execute_with(|| {
        // create a pool and set it active.
        initialize_pool_for_dispatches();

        // Bob adds 200 shares liquidity to the pool
        let alice = 1;
        let bob = 2u64;
        let charlie = 3;
        let claimer = Origin::signed(bob);
        let pool_id = 0;
        let bnc_id = 0;
        let new_pool_token = 200; // Alice initial pool token amount is 1000. Bob want's to get 20% of that of Alice's.

        assert_ok!(Swap::add_liquidity_given_shares_in(
            claimer.clone(),
            pool_id,
            new_pool_token
        ));

        // fake a record in UserUnclaimedBonusInPool
        UserUnclaimedBonusInPool::<Test>::insert(bob, 0, (50, 1000));
        assert_eq!(Swap::user_unclaimed_bonus_in_pool(bob, 0), (50, 1000));

        // fake a record in DeductedBonusAmountInPool
        DeductedBonusAmountInPool::<Test>::insert(0, 500_000);
        assert_eq!(Swap::deducted_bonus_amount_in_pool(0), 500_000);

        run_to_block(5000);
        // claim bonus
        assert_ok!(Swap::claim_bonus(claimer.clone(), 0));
        // approx_eq!(f32, 23_148.15f32, f32::from_fixed(result), epsilon = 0.000_000_000_001);

        // check user's BNC account to see whether the amount issued is right.
        let result = <Test as Trait>::AssetTrait::get_account_asset(bnc_id, &bob).available;
        assert_eq!(result, 23_198u64);

        // Below are the incorrect operations.

        // no such pool_id
        let pool_id = 1;
        assert_eq!(
            Swap::claim_bonus(claimer.clone(), pool_id),
            Err(DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("PoolNotExist")
            })
        );

        // pool status is false, thus it's not able to add liquidity in.
        let pool_id = 0;
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, false));
        assert_eq!(
            Swap::claim_bonus(claimer.clone(), pool_id),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("PoolNotActive")
            })
        );

        // charlie doesn't have pool token in pool 0.
        assert_ok!(Swap::set_pool_status(Origin::signed(alice), pool_id, true));
        assert_eq!(
            Swap::claim_bonus(Origin::signed(charlie), pool_id),
            Err(DispatchError::Module {
                index: 0,
                error: 7,
                message: Some("UserNotInThePool")
            })
        );
    });
}

#[test]
fn set_swap_fee_should_work() {
    new_test_ext().execute_with(|| {
        // create a pool and set it active.
        initialize_pool_for_dispatches();

        let alice = 1;
        let bob = 2;
        let setter = Origin::signed(alice);
        let pool_id = 0;
        let new_swap_fee = 10_000;

        assert_ok!(Swap::set_swap_fee(setter, pool_id, new_swap_fee));
        // check if the pool has new swap fee rate
        assert_eq!(Swap::pools(pool_id).swap_fee_rate, new_swap_fee);

        // Below are the incorrect operations.
        // bob is not the owner of the pool.
        assert_eq!(
            Swap::set_swap_fee(Origin::signed(bob), pool_id, new_swap_fee),
            Err(DispatchError::Module {
                index: 0,
                error: 16,
                message: Some("NotPoolOwner")
            })
        );

        // swap fee rate exceed maximum limit.
        let new_swap_fee = 1_000_000;
        let setter = Origin::signed(alice);
        assert_eq!(
            Swap::set_swap_fee(setter.clone(), pool_id, new_swap_fee),
            Err(DispatchError::Module {
                index: 0,
                error: 15,
                message: Some("FeeRateExceedMaximumLimit")
            })
        );

        // swap fee rate exceed minimum limit.
        let new_swap_fee = 0;
        let setter = Origin::signed(alice);
        assert_eq!(
            Swap::set_swap_fee(setter.clone(), pool_id, new_swap_fee),
            Err(DispatchError::Module {
                index: 0,
                error: 14,
                message: Some("FeeRateExceedMinimumLimit")
            })
        );
    });
}

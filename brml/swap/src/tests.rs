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
use float_cmp::approx_eq;
use frame_support::{assert_ok, dispatch::DispatchError};
use node_primitives::TokenSymbol;

#[test]
fn total_weight_should_work() {
    let pool = vec![
        (0, 100, 20),
        (1, 10, 40),
        (2, 100, 50),
        (3, 200, 10),
        (4, 40, 20),
        (5, 60, 20),
        (6, 30, 20),
    ];
    let total_weight = Swap::total_weight(&pool);
    assert_eq!(total_weight, 180);
}

#[test]
fn weight_ratio_should_work() {
    let (w1, w2) = (30, 50);
    let ratio = Swap::weight_ratio(w1, w2);
    assert!(ratio.is_ok());
    let ratio = ratio.map(f32::from_fixed).unwrap();
    approx_eq!(f32, 0.6f32, ratio, epsilon = 0.000_000_000_001);
}

#[test]
fn value_function_should_work() {
    let pool = vec![
        (0, 100, 20),
        (1, 10, 40),
        (2, 100, 50),
        (3, 200, 10),
        (4, 40, 20),
        (5, 60, 20),
        (6, 30, 20),
    ];
    let value_of_function = Swap::value_function(&pool);
    assert!(value_of_function.is_ok());
    assert_eq!(value_of_function.unwrap(), 46);
}
//073_610_415_623_4351516
#[test]
fn calculate_spot_price_should_work() {
    let swap_fee = 0;
    let token_balance_in = 1000;
    let token_weight_in = 1;
    let token_balance_out = 1000;
    let token_weight_out = 49;
    let price = Swap::calculate_spot_price(
        token_balance_in,
        token_weight_in,
        token_balance_out,
        token_weight_out,
        swap_fee,
    );
    assert!(price.is_ok());
    assert_eq!(price.unwrap(), 49);

    // with swap fee
    let swap_fee = 150;
    let price = Swap::calculate_spot_price(
        token_balance_in,
        token_weight_in,
        token_balance_out,
        token_weight_out,
        swap_fee,
    );
    assert!(price.is_ok());

    let price = price.map(f32::from_fixed);
    approx_eq!(
        f32,
        49.073_610_415_623f32,
        price.unwrap(),
        epsilon = 0.000_000_000_001
    );
}

#[test]
fn calculate_out_given_in_should_work() {
    let swap_fee = 100;
    let token_balance_in = 1000;
    let token_weight_in = 1;
    let token_balance_out = 1000;
    let token_weight_out = 49;
    let amount_in = 500;

    let target = 8.233_908_519_628f32;
    let to_buy = Swap::calculate_out_given_in(
        token_balance_in,
        token_weight_in,
        amount_in,
        token_balance_out,
        token_weight_out,
        swap_fee,
    );
    assert!(to_buy.is_ok());

    let to_buy = to_buy.map(f32::from_fixed).unwrap();
    approx_eq!(f32, target, to_buy, epsilon = 0.000_000_000_001);

    // trade back
    let token_balance_in = 1000 - target as u64;
    let token_balance_out = 1000 + amount_in;
    let amount_in = target as u64;
    let to_buy = Swap::calculate_out_given_in(
        token_balance_in,
        token_weight_out,
        amount_in,
        token_balance_out,
        token_weight_in,
        swap_fee,
    );
    assert!(to_buy.is_ok());

    let to_buy = to_buy.map(f32::from_fixed).unwrap();
    let target = 487.643_591_413_530f32;
    approx_eq!(f32, target, to_buy, epsilon = 0.000_000_000_001);
}

#[test]
fn calculate_pool_out_given_single_in_should_work() {
    let swap_fee = 100; // 0.001
    let token_balance_in = 1000;
    let token_weight_in = 1;
    let token_amount_in = 100;
    let total_token_weight = 50;
    let pool_supply = 100;

    let issued_pool = Swap::calculate_pool_out_given_single_in(
        token_balance_in,
        token_weight_in,
        token_amount_in,
        total_token_weight,
        pool_supply,
        swap_fee,
    );
    assert!(issued_pool.is_ok());

    let target = 0.190_623_628_671f32;
    let issued_pool = issued_pool.map(f32::from_fixed).unwrap();
    approx_eq!(f32, target, issued_pool, epsilon = 0.000_000_000_001);
}

#[test]
fn calculate_in_given_out_should_work() {
    let token_balance_in = 12;
    let token_weight_in = 10;
    let token_balance_out = 4;
    let token_weight_out = 10;
    let swap_fee = 100;

    let under_trade = 1;

    let desired = Swap::calculate_in_given_out(
        token_balance_in,
        token_weight_in,
        token_balance_out,
        token_weight_out,
        under_trade,
        swap_fee,
    );
    let target = 4.004_004_004_004f32;
    let desired = desired.map(f32::from_fixed).unwrap();
    approx_eq!(f32, target, desired, epsilon = 0.000_000_000_001);
}

#[test]
fn calculate_single_out_given_pool_in_should_work() {
    let token_weight_in = 1;
    let pool_amount_in = 10;
    let token_total_weight = 50;
    let token_balance_out = 20;
    let pool_supply = 1000;
    let swap_fee = 100;
    let exit_fee = 0;

    let token_amount = Swap::calculate_single_out_given_pool_in(
        token_weight_in,
        pool_amount_in,
        token_total_weight,
        token_balance_out,
        pool_supply,
        swap_fee,
        exit_fee,
    );
    let target = 7.892_136_857_259f32;
    let token_amount = token_amount.map(f32::from_fixed).unwrap();
    approx_eq!(f32, target, token_amount, epsilon = 0.000_000_000_001);
}

#[test]
fn calculate_single_in_given_pool_out_should_work() {
    let token_balance_in = 1000;
    let token_weight_in = 1;
    let token_total_weight = 50;
    let pool_amount_out = 10;
    let pool_supply = 1000;
    let swap_fee = 100;

    let single_in = Swap::calculate_single_in_given_pool_out(
        token_balance_in,
        token_weight_in,
        token_total_weight,
        pool_amount_out,
        pool_supply,
        swap_fee,
    );
    assert!(single_in.is_ok());
}

#[test]
fn add_liquidity_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(2);

        // this pool has two tokens, an each one has 1000 balance, weight 1 and 49
        let raw_pool = vec![(TokenSymbol::DOT, 1000, 1), (TokenSymbol::KSM, 1000, 49)];
        let original_pool = (raw_pool, 0);
        <GlobalPool<Test>>::put(original_pool);

        // issue a vtoken to alice
        let alice = 1u64;
        let dot_symbol = b"DOT".to_vec();
        let precise = 4;
        let dot_token_amount = 10000;

        assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18)); // let dot start from 1

        // issue dot token
        assert_ok!(assets::Module::<Test>::create(
            Origin::root(),
            dot_symbol,
            precise
        ));
        let dot_id = Assets::next_asset_id() - 1;
        let dot_type = TokenSymbol::from(dot_id);

        assert_ok!(assets::Module::<Test>::issue(
            Origin::root(),
            dot_type,
            alice,
            dot_token_amount
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount
        );

        assert_ok!(Assets::create(Origin::root(), b"vDOT".to_vec(), 12)); // skip vDOT

        // issue ksm token
        let ksm_symbol = b"KSM".to_vec();
        let precise = 4;
        let ksm_token_amount = 100000;

        assert_ok!(assets::Module::<Test>::create(
            Origin::root(),
            ksm_symbol,
            precise
        ));
        let ksm_id = Assets::next_asset_id() - 1;
        let ksm_type = TokenSymbol::from(ksm_id);

        assert_ok!(assets::Module::<Test>::issue(
            Origin::root(),
            ksm_type,
            alice,
            ksm_token_amount
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((ksm_type, alice)).balance,
            ksm_token_amount
        );

        // set swap fee
        let fee = 100;
        <SwapFee<Test>>::put(fee);
        assert_eq!(<SwapFee<Test>>::get(), fee);

        // issue intialized pool token
        let pool_token = 1000;
        <BalancerPoolToken<Test>>::put(pool_token);

        // add liquidity less than MinimumBalance
        assert_eq!(
            Swap::add_liquidity(Origin::signed(alice), 9),
            Err(DispatchError::Module {
                index: 0,
                error: 5,
                message: Some("LessThanMinimumBalance")
            })
        );

        // first time to deposit to pool
        let new_pool_token = 10;
        assert_ok!(Swap::add_liquidity(Origin::signed(alice), new_pool_token));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount - 100
        );
        assert_eq!(
            <assets::AccountAssets<Test>>::get((ksm_type, alice)).balance,
            ksm_token_amount - 1000
        );

        let gpool = <GlobalPool<Test>>::get();
        let target = vec![1100u64, 2000];
        for (p, t) in gpool.0.iter().zip(target.iter()) {
            assert_eq!(p.1, *t);
        }

        // continue to add liuquidity
        assert_ok!(Swap::add_liquidity(Origin::signed(alice), new_pool_token));
        let gpool = <GlobalPool<Test>>::get();
        let target = vec![1198u64, 2980];
        for (p, t) in gpool.0.iter().zip(target.iter()) {
            assert_eq!(p.1, *t);
        }
    });
}

#[test]
fn add_single_liquidity_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(2);

        // this pool has two tokens, an each one has 1000 balance, weight 1 and 49
        let raw_pool = vec![(TokenSymbol::DOT, 1000, 1), (TokenSymbol::KSM, 1000, 49)];
        let original_pool = (raw_pool, 0);
        <GlobalPool<Test>>::put(original_pool);

        // issue a vtoken to alice
        let alice = 1u64;
        let dot_symbol = b"DOT".to_vec();
        let precise = 4;
        let dot_token_amount = 1000;

        assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18)); // let dot start from 1

        // issue dot token
        assert_ok!(assets::Module::<Test>::create(
            Origin::root(),
            dot_symbol,
            precise
        ));
        let dot_id = Assets::next_asset_id() - 1;
        let dot_type = TokenSymbol::from(dot_id);

        assert_ok!(assets::Module::<Test>::issue(
            Origin::root(),
            dot_type,
            alice,
            dot_token_amount
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount
        );

        assert_ok!(Assets::create(Origin::root(), b"vDOT".to_vec(), 12)); // skip vDOT

        // create ksm token, but issue nothing
        let ksm_symbol = b"KSM".to_vec();
        assert_ok!(assets::Module::<Test>::create(
            Origin::root(),
            ksm_symbol,
            precise
        ));
        let ksm_id = Assets::next_asset_id() - 1;
        let ksm_type = TokenSymbol::from(ksm_id);

        // set swap fee
        let fee = 100;
        <SwapFee<Test>>::put(fee);
        assert_eq!(<SwapFee<Test>>::get(), fee);

        // issue intialized pool token
        let pool_token = 1000;
        <BalancerPoolToken<Test>>::put(pool_token);

        // set weight
        let total_weight = 50;
        <TotalWeight<Test>>::put(total_weight);

        let token_amount_in = 100;

        // add a token alice doesn't have
        assert_eq!(
            Swap::add_single_liquidity(Origin::signed(alice), TokenSymbol::IOST, token_amount_in),
            Err(DispatchError::Module {
                index: 0,
                error: 0,
                message: Some("TokenNotExist")
            })
        );

        // test with a created token but with 0 balance
        assert_eq!(
            Swap::add_single_liquidity(Origin::signed(alice), ksm_type, token_amount_in),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("NotEnoughBalance")
            })
        );

        assert_eq!(
            Swap::add_single_liquidity(Origin::signed(alice), dot_type, dot_token_amount + 1),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("NotEnoughBalance")
            })
        );

        // first time to add liquidity
        assert_ok!(Swap::add_single_liquidity(
            Origin::signed(alice),
            dot_type,
            token_amount_in
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount - 100
        );
        assert_eq!(<BalancerPoolToken<Test>>::get(), 1001);
        assert_eq!(<UserSinglePool<Test>>::get((alice, dot_type)), (100, 1));

        // continue to add liuquidity
        assert_ok!(Swap::add_single_liquidity(
            Origin::signed(alice),
            dot_type,
            token_amount_in
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount - 200
        );
        assert_eq!(<BalancerPoolToken<Test>>::get(), 1002);
        assert_eq!(<UserSinglePool<Test>>::get((alice, dot_type)), (200, 2));
    });
}

#[test]
fn swap_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(2);

        // this pool has two tokens, an each one has 1000 balance, weight 1 and 49
        let raw_pool = vec![(TokenSymbol::DOT, 0, 1), (TokenSymbol::KSM, 0, 49)];
        let original_pool = (raw_pool, 0);
        <GlobalPool<Test>>::put(original_pool);

        // issue a vtoken to alice
        let alice = 1u64;
        let dot_symbol = b"DOT".to_vec();
        let precise = 4;
        let dot_token_amount = 1_000_000;

        assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18)); // let dot start from 1

        // issue dot token
        assert_ok!(assets::Module::<Test>::create(
            Origin::root(),
            dot_symbol,
            precise
        ));
        let dot_id = Assets::next_asset_id() - 1;
        let dot_type = TokenSymbol::from(dot_id);

        // issue dot token
        assert_ok!(assets::Module::<Test>::issue(
            Origin::root(),
            dot_type,
            alice,
            dot_token_amount
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount
        );

        assert_ok!(Assets::create(Origin::root(), b"vDOT".to_vec(), 12)); // skip vDOT

        // create ksm token
        let ksm_symbol = b"KSM".to_vec();
        let ksm_token_amount = 1_000_000;
        assert_ok!(assets::Module::<Test>::create(
            Origin::root(),
            ksm_symbol,
            precise
        ));
        let ksm_id = Assets::next_asset_id() - 1;
        let ksm_type = TokenSymbol::from(ksm_id);

        assert_ok!(assets::Module::<Test>::issue(
            Origin::root(),
            ksm_type,
            alice,
            ksm_token_amount
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((ksm_type, alice)).balance,
            ksm_token_amount
        );

        // set swap fee
        let fee = 100; // 0.1%
        <SwapFee<Test>>::put(fee);
        assert_eq!(<SwapFee<Test>>::get(), fee);

        // issue intialized pool token
        let pool_token = 1000;
        <BalancerPoolToken<Test>>::put(pool_token);

        // init reward pool
        let reward = vec![(dot_type, 0), (ksm_type, 0)];
        <SharedRewardPool<Test>>::put(reward);

        // trade with the same token
        assert_eq!(
            Swap::swap(Origin::signed(alice), dot_type, 700, None, dot_type, None),
            Err(DispatchError::Module {
                index: 0,
                error: 9,
                message: Some("ForbidSameTokenSwap")
            })
        );

        // trade amount bigger alice has
        assert_eq!(
            Swap::swap(
                Origin::signed(alice),
                dot_type,
                dot_token_amount + 1,
                None,
                ksm_type,
                None
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("NotEnoughBalance")
            })
        );

        // trade more than half of all amount
        assert_eq!(
            Swap::swap(
                Origin::signed(alice),
                dot_type,
                700_000,
                None,
                ksm_type,
                None
            ),
            Err(DispatchError::Module {
                index: 0,
                error: 11,
                message: Some("ExceedMaximumSwapInRatio")
            })
        );

        // add liquidity
        let new_pool_token = 500;
        assert_ok!(Swap::add_liquidity(Origin::signed(alice), new_pool_token));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount / 2
        );
        assert_eq!(
            <assets::AccountAssets<Test>>::get((ksm_type, alice)).balance,
            ksm_token_amount / 2
        );

        let alice_pool = <UserPool<Test>>::get(alice);
        assert_eq!(alice_pool.0[0].1, dot_token_amount / 2);
        assert_eq!(alice_pool.0[1].1, ksm_token_amount / 2);
        assert_eq!(alice_pool.1, new_pool_token);

        let gpool = <GlobalPool<Test>>::get();
        let expected = vec![(dot_type, 500000, 1), (ksm_type, 500000, 49)];
        assert_eq!(gpool.0, expected);
        assert_eq!(
            <BalancerPoolToken<Test>>::get(),
            new_pool_token + pool_token
        );

        // do a trade
        assert_ok!(Swap::swap(
            Origin::signed(alice),
            dot_type,
            5000,
            None,
            ksm_type,
            None
        ));
        // assert charged fee
        assert_eq!(<SharedRewardPool::<Test>>::get()[0], (dot_type, 5));
        // global pool check
        let gpool = <GlobalPool<Test>>::get();
        let expected = vec![
            (dot_type, 505000, 1),
            (ksm_type, 499899, 49), // should be 500000 - 100.51453390162312, but lost precision
        ];
        assert_eq!(gpool.0, expected);
        // user pool check
        let user_pool = <UserPool<Test>>::get(alice);
        assert_eq!(user_pool.1, new_pool_token);
        let expected = vec![
            (dot_type, 505000),
            (ksm_type, 499899), // should be 500000 - 100.51453390162312, but lost precision
        ];
        assert_eq!(user_pool.0, expected);

        // check alice account after trade
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount / 2 - 5000
        );

        // swap back
        assert_ok!(Swap::swap(
            Origin::signed(alice),
            ksm_type,
            101,
            None,
            dot_type,
            None
        ));
        let gpool = <GlobalPool<Test>>::get();
        let expected = vec![
            (dot_type, 500031, 1),
            (ksm_type, 500000, 49), // losing precision causes this problem
        ];
        assert_eq!(gpool.0, expected);

        // quit from liquidity
        assert_ok!(Swap::remove_assets_liquidity(
            Origin::signed(alice),
            new_pool_token
        ));
        assert_eq!(<SharedRewardPool::<Test>>::get()[0], (dot_type, 4));

        let gpool = <GlobalPool<Test>>::get();
        let expected = vec![(dot_type, 0, 1), (ksm_type, 0, 49)];
        assert_eq!(gpool.0, expected);

        // int(5/3) = 1 is reward
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount + 1
        );
    });
}

#[test]
fn remove_single_liquidity_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(2);

        // this pool has two tokens, an each one has 1000 balance, weight 1 and 49
        let raw_pool = vec![(TokenSymbol::DOT, 0, 1), (TokenSymbol::KSM, 0, 49)];
        let original_pool = (raw_pool, 0);
        <GlobalPool<Test>>::put(original_pool);

        // issue a vtoken to alice
        let alice = 1u64;
        let bob = 2u64;
        let jim = 3u64;

        let dot_symbol = b"DOT".to_vec();
        let precise = 4;
        let dot_token_amount = 1_000_000;

        assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18)); // let dot start from 1

        // issue dot token
        assert_ok!(assets::Module::<Test>::create(
            Origin::root(),
            dot_symbol,
            precise
        ));
        let dot_id = Assets::next_asset_id() - 1;
        let dot_type = TokenSymbol::from(dot_id);

        // issue dot token
        assert_ok!(assets::Module::<Test>::issue(
            Origin::root(),
            dot_type,
            alice,
            dot_token_amount
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount
        );

        assert_ok!(Assets::create(Origin::root(), b"vDOT".to_vec(), 12)); // skip vDOT

        // create ksm token
        let ksm_symbol = b"KSM".to_vec();
        let ksm_token_amount = 1_000_000;
        assert_ok!(assets::Module::<Test>::create(
            Origin::root(),
            ksm_symbol,
            precise
        ));
        let ksm_id = Assets::next_asset_id() - 1;
        let ksm_type = TokenSymbol::from(ksm_id);

        // issue ksm token
        assert_ok!(assets::Module::<Test>::issue(
            Origin::root(),
            ksm_type,
            alice,
            ksm_token_amount
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((ksm_type, alice)).balance,
            ksm_token_amount
        );

        // issue bob dot token
        assert_ok!(assets::Module::<Test>::issue(
            Origin::root(),
            dot_type,
            bob,
            dot_token_amount
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, bob)).balance,
            dot_token_amount
        );

        // set swap fee
        let fee = 1000; // 1%
        <SwapFee<Test>>::put(fee);
        assert_eq!(<SwapFee<Test>>::get(), fee);

        // issue intialized pool token
        let pool_token = 1000;
        <BalancerPoolToken<Test>>::put(pool_token);

        // set weight
        let total_weight = 50;
        <TotalWeight<Test>>::put(total_weight);

        // init reward pool
        let reward = vec![(dot_type, 0), (ksm_type, 0)];
        <SharedRewardPool<Test>>::put(reward);

        // add liquidity
        let new_pool_token = 500;
        assert_ok!(Swap::add_liquidity(Origin::signed(alice), new_pool_token));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount / 2
        );
        assert_eq!(
            <assets::AccountAssets<Test>>::get((ksm_type, alice)).balance,
            ksm_token_amount / 2
        );
        assert_eq!(<BalancerPoolToken<Test>>::get(), 1500);

        let token_amount_in = 100000;
        // bob wants to add single liquidity
        assert_ok!(Swap::add_single_liquidity(
            Origin::signed(bob),
            dot_type,
            token_amount_in
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, bob)).balance,
            dot_token_amount - token_amount_in
        );
        assert_eq!(<BalancerPoolToken<Test>>::get(), 1505); // lose precision, 1500 + 5.4796312543396398422
        assert_eq!(
            <UserSinglePool<Test>>::get((bob, dot_type)),
            (token_amount_in, 5)
        ); // lose precision

        // bob doesn't have this token in pool
        assert_eq!(
            Swap::remove_single_asset_liquidity(Origin::signed(jim), dot_type, 13),
            Err(DispatchError::Module {
                index: 0,
                error: 7,
                message: Some("NotExistedCurrentSinglePool")
            })
        );
        // alice doesn't have vdot
        assert_eq!(
            Swap::remove_single_asset_liquidity(Origin::signed(alice), dot_type, 13),
            Err(DispatchError::Module {
                index: 0,
                error: 7,
                message: Some("NotExistedCurrentSinglePool")
            })
        );
        // alice redeems too much
        assert_eq!(
            Swap::remove_single_asset_liquidity(Origin::signed(bob), dot_type, 5 + 1),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("NotEnoughBalance")
            })
        );

        // do a swap
        assert_ok!(Swap::swap(
            Origin::signed(alice),
            dot_type,
            1000,
            None,
            ksm_type,
            None
        ));
        // check alice gets how many ksm
        assert_eq!(
            <assets::AccountAssets<Test>>::get((ksm_type, alice)).balance,
            500016
        ); // lose precision
        assert_eq!(<SharedRewardPool::<Test>>::get()[0].1, 10);

        // remove liqudity
        assert_ok!(Swap::remove_single_asset_liquidity(
            Origin::signed(bob),
            dot_type,
            5
        ));
        assert_eq!(<SharedRewardPool::<Test>>::get()[0].1, 10);
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, bob)).balance,
            992123
        ); // lose precision
        assert_eq!(<UserSinglePool<Test>>::get((bob, dot_type)), (7877, 0)); // 999951 + 49 == 1_000_000
        assert_eq!(<BalancerPoolToken<Test>>::get(), 1500);
    });
}

#[test]
fn remove_assets_liquidity_should_work() {
    new_test_ext().execute_with(|| {
        run_to_block(2);

        // this pool has two tokens, an each one has 1000 balance, weight 1 and 49
        let raw_pool = vec![(TokenSymbol::DOT, 0, 1), (TokenSymbol::KSM, 0, 49)];
        let original_pool = (raw_pool, 0);
        <GlobalPool<Test>>::put(original_pool);

        // issue a vtoken to alice
        let alice = 1u64;
        let bob = 2u64;

        let dot_symbol = b"DOT".to_vec();
        let precise = 4;
        let dot_token_amount = 10_000;

        assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18)); // let dot start from 1

        // issue dot token
        assert_ok!(assets::Module::<Test>::create(
            Origin::root(),
            dot_symbol,
            precise
        ));
        let dot_id = Assets::next_asset_id() - 1;
        let dot_type = TokenSymbol::from(dot_id);

        assert_ok!(assets::Module::<Test>::issue(
            Origin::root(),
            dot_type,
            alice,
            dot_token_amount
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount
        );

        assert_ok!(Assets::create(Origin::root(), b"vDOT".to_vec(), 12)); // skip vDOT

        // create ksm token
        let ksm_symbol = b"KSM".to_vec();
        let ksm_token_amount = 100_000;
        assert_ok!(assets::Module::<Test>::create(
            Origin::root(),
            ksm_symbol,
            precise
        ));
        let ksm_id = Assets::next_asset_id() - 1;
        let ksm_type = TokenSymbol::from(ksm_id);

        // issue ksm token
        assert_ok!(assets::Module::<Test>::issue(
            Origin::root(),
            ksm_type,
            alice,
            ksm_token_amount
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((ksm_type, alice)).balance,
            ksm_token_amount
        );

        // set swap fee
        let fee = 100;
        <SwapFee<Test>>::put(fee);
        assert_eq!(<SwapFee<Test>>::get(), fee);

        // issue intialized pool token
        let pool_token = 1000;
        <BalancerPoolToken<Test>>::put(pool_token);

        // init reward pool
        let reward = vec![(dot_type, 0), (ksm_type, 0)];
        <SharedRewardPool<Test>>::put(reward);

        // first time to deposit to pool
        let new_pool_token = 10;
        assert_ok!(Swap::add_liquidity(Origin::signed(alice), new_pool_token));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount - 100
        );
        assert_eq!(
            <assets::AccountAssets<Test>>::get((ksm_type, alice)).balance,
            ksm_token_amount - 1000
        );

        let gpool = <GlobalPool<Test>>::get();
        let target = vec![100u64, 1000];
        for (p, t) in gpool.0.iter().zip(target.iter()) {
            assert_eq!(p.1, *t);
        }

        let user_pool = <UserPool<Test>>::get(alice);
        let target = vec![100u64, 1000];
        for (p, t) in user_pool.0.iter().zip(target.iter()) {
            assert_eq!(p.1, *t);
        }
        assert_eq!(user_pool.1, new_pool_token);

        // suppose bob doesn't have any pool
        assert_eq!(
            Swap::remove_assets_liquidity(Origin::signed(bob), new_pool_token),
            Err(DispatchError::Module {
                index: 0,
                error: 8,
                message: Some("NotExistedCurrentPool")
            })
        );
        // alice redeems too much
        assert_eq!(
            Swap::remove_assets_liquidity(Origin::signed(alice), new_pool_token + 1),
            Err(DispatchError::Module {
                index: 0,
                error: 1,
                message: Some("NotEnoughBalance")
            })
        );

        // remove liquidity
        assert_ok!(Swap::remove_assets_liquidity(
            Origin::signed(alice),
            new_pool_token
        ));
        assert_eq!(
            <assets::AccountAssets<Test>>::get((dot_type, alice)).balance,
            dot_token_amount
        );
        assert_eq!(
            <assets::AccountAssets<Test>>::get((ksm_type, alice)).balance,
            ksm_token_amount
        );

        let gpool = <GlobalPool<Test>>::get();
        assert!(gpool.0.iter().all(|p| p.1 == 0));

        let user_pool = <UserPool<Test>>::get(alice);
        assert!(user_pool.0.iter().all(|p| p.1 == 0));
        assert_eq!(user_pool.1, 0);
    });
}

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
use frame_support::assert_ok;
use node_primitives::TokenType;

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
	let price = Swap::calculate_spot_price(token_balance_in, token_weight_in, token_balance_out, token_weight_out, swap_fee);
	assert!(price.is_ok());
	assert_eq!(price.unwrap(), 49);

	// with swap fee
	let swap_fee = 150;
	let price = Swap::calculate_spot_price(token_balance_in, token_weight_in, token_balance_out, token_weight_out, swap_fee);
	assert!(price.is_ok());

	let price = price.map(f32::from_fixed);
	approx_eq!(f32, 49.073_610_415_623f32, price.unwrap(), epsilon = 0.000_000_000_001);
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
	let to_buy = Swap::calculate_out_given_in(token_balance_in, token_weight_in, amount_in, token_balance_out, token_weight_out, swap_fee);
	assert!(to_buy.is_ok());

	let to_buy = to_buy.map(f32::from_fixed).unwrap();
	approx_eq!(f32, target, to_buy, epsilon = 0.000_000_000_001);

	// trade back
	let token_balance_in = 1000 - target as u64;
	let token_balance_out = 1000 + amount_in;
	let amount_in = target as u64;
	let to_buy = Swap::calculate_out_given_in(token_balance_in, token_weight_out, amount_in, token_balance_out, token_weight_in, swap_fee);
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
		swap_fee
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

	let desired = Swap::calculate_in_given_out(token_balance_in, token_weight_in, token_balance_out, token_weight_out, under_trade, swap_fee);
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

	let token_amount = Swap::calculate_single_out_given_pool_in(token_weight_in, pool_amount_in, token_total_weight, token_balance_out, pool_supply, swap_fee, exit_fee);
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

	let single_in = Swap::calculate_single_in_given_pool_out(token_balance_in, token_weight_in, token_total_weight, pool_amount_out, pool_supply, swap_fee);
	assert!(single_in.is_ok());
}

#[test]
fn add_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		// this pool has two tokens, an each one has 1000 balance, weight 1 and 49
		let raw_pool = vec![
			(0, TokenType::Token, 1000, 1),
			(1, TokenType::Token, 1000, 49)
		];
		let original_pool = (raw_pool, 0);
		<GlobalPool<Test>>::put(original_pool);

		// issue a vtoken to alice
		let alice = 1u64;
		let dot_token = vec![0x12, 0x34];
		let precise = 4;
		let dot_token_amount = 10000;
		// issue dot token
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, dot_token.into(), precise));
		let dot_token_id = <assets::NextAssetId<Test>>::get() - 1;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, dot_token_id.into(), TokenType::Token, alice, dot_token_amount));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount);

		// issue ksm token
		let ksm_token = vec![0x12, 0x56];
		let ksm_token_amount = 100000;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, ksm_token.into(), precise));
		let ksm_token_id = <assets::NextAssetId<Test>>::get() - 1;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, ksm_token_id.into(), TokenType::Token, alice, ksm_token_amount));
		assert_eq!(<assets::AccountAssets<Test>>::get((ksm_token_id, TokenType::Token, alice)).balance, ksm_token_amount);

		// set swap fee
		let fee = 100;
		<SwapFee<Test>>::put(fee);
		assert_eq!(<SwapFee<Test>>::get(), fee);

		// issue intialized pool token
		let pool_token = 1000;
		<BalancerPoolToken<Test>>::put(pool_token);

		// first time to deposit to pool
		let new_pool_token = 1;
		assert_ok!(Swap::add_liquidity(Origin::signed(alice), new_pool_token));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount - 10);
		assert_eq!(<assets::AccountAssets<Test>>::get((ksm_token_id, TokenType::Token, alice)).balance, ksm_token_amount - 100);
		let gpool = <GlobalPool<Test>>::get();
		let target = vec![1010u64, 1100];
		for (p, t) in gpool.0.iter().zip(target.iter()) {
			assert_eq!(p.2, *t);
		}

		// continue to add liuquidity
		assert_ok!(Swap::add_liquidity(Origin::signed(alice), new_pool_token));
		let gpool = <GlobalPool<Test>>::get();
		let target = vec![1019u64, 1199];
		for (p, t) in gpool.0.iter().zip(target.iter()) {
			assert_eq!(p.2, *t);
		}
	});
}

#[test]
fn add_single_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		// this pool has two tokens, an each one has 1000 balance, weight 1 and 49
		let raw_pool = vec![
			(0, TokenType::Token, 1000, 1),
			(1, TokenType::Token, 1000, 49)
		];
		let original_pool = (raw_pool, 0);
		<GlobalPool<Test>>::put(original_pool);

		// issue a vtoken to alice
		let alice = 1u64;
		let dot_token = vec![0x12, 0x34];
		let precise = 4;
		let dot_token_amount = 1000;
		// issue dot token, and just one token
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, dot_token.into(), precise));
		let dot_token_id = <assets::NextAssetId<Test>>::get() - 1;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, dot_token_id.into(), TokenType::Token, alice, dot_token_amount));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount);

		// set swap fee
		let fee = 100;
		<SwapFee<Test>>::put(fee);
		assert_eq!(<SwapFee<Test>>::get(), fee);

		// issue intialized pool token
		let pool_token = 1000;
		<BalancerPoolToken<Test>>::put(pool_token);

		// set weight
		let total_weight = 50;
		<TotalWeight::<Test>>::put(total_weight);

		let token_symbol = AssetSymbol::DOT;
		let token_type = TokenType::Token;
		let token_amount_in = 100;
		// first time to add liquidity
		assert_ok!(Swap::add_single_liquidity(Origin::signed(alice), token_symbol, token_type, token_amount_in));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount - 100);
		assert_eq!(<BalancerPoolToken<Test>>::get(), 1001);
		assert_eq!(<UserSinglePool<Test>>::get((alice, dot_token_id, token_type)), (100, 1));

		// continue to add liuquidity
		assert_ok!(Swap::add_single_liquidity(Origin::signed(alice), token_symbol, token_type, token_amount_in));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount - 200);
		assert_eq!(<BalancerPoolToken<Test>>::get(), 1002);
		assert_eq!(<UserSinglePool<Test>>::get((alice, dot_token_id, token_type)), (200, 2));
	});
}

#[test]
fn swap_out_given_in_should_work() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		// this pool has two tokens, an each one has 1000 balance, weight 1 and 49
		let raw_pool = vec![
			(0, TokenType::Token, 1000, 1),
			(1, TokenType::Token, 1000, 49)
		];
		let original_pool = (raw_pool, 0);
		<GlobalPool<Test>>::put(original_pool);

		// issue a vtoken to alice
		let alice = 1u64;
		let dot_token = vec![0x12, 0x34];
		let precise = 4;
		let dot_token_amount = 1000;
		// issue dot token
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, dot_token.into(), precise));
		let dot_token_id = <assets::NextAssetId<Test>>::get() - 1;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, dot_token_id.into(), TokenType::Token, alice, dot_token_amount));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount);

		// issue ksm token
		let ksm_token = vec![0x12, 0x56];
		let ksm_token_amount = 1000;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, ksm_token.into(), precise));
		let ksm_token_id = <assets::NextAssetId<Test>>::get() - 1;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, ksm_token_id.into(), TokenType::Token, alice, ksm_token_amount));
		assert_eq!(<assets::AccountAssets<Test>>::get((ksm_token_id, TokenType::Token, alice)).balance, ksm_token_amount);

		// alice pool
		let alice_pool = (
			vec![
				(0, TokenType::Token, 1000),
				(1, TokenType::Token, 1000)
			],
			1000
		);
		<UserPool::<Test>>::insert(alice, alice_pool);

		// set swap fee
		let fee = 100;
		<SwapFee<Test>>::put(fee);
		assert_eq!(<SwapFee<Test>>::get(), fee);

		// issue intialized pool token
		let pool_token = 1000;
		<BalancerPoolToken<Test>>::put(pool_token);

		assert_ok!(Swap::swap_out_given_in(Origin::signed(alice), AssetSymbol::DOT, TokenType::Token, 500, None, AssetSymbol::KSM, TokenType::Token, None));
		let gpool = <GlobalPool<Test>>::get();
		let expected = vec![
			(0, TokenType::Token, 1500, 1),
			(1, TokenType::Token, 992, 49) // should be 1000 - 8.233_908_519_628f32, but lost precision
		];
		assert_eq!(gpool.0, expected);

		assert_ok!(Swap::swap_out_given_in(Origin::signed(alice), AssetSymbol::KSM, TokenType::Token, 8, None, AssetSymbol::DOT, TokenType::Token, None));
		let gpool = <GlobalPool<Test>>::get();
		let expected = vec![
			(0, TokenType::Token, 1013, 1), // precision problem, should be 1000
			(1, TokenType::Token, 1000, 49)
		];
		assert_eq!(gpool.0, expected);
	});
}

#[test]
fn remove_single_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		// this pool has two tokens, an each one has 1000 balance, weight 1 and 49
		let raw_pool = vec![
			(0, TokenType::Token, 1000, 1), // dot
			(1, TokenType::Token, 1000, 49) // ksm
		];
		let original_pool = (raw_pool, 0);
		<GlobalPool<Test>>::put(original_pool);

		// issue a vtoken to alice
		let alice = 1u64;
		let dot_token = vec![0x12, 0x34];
		let precise = 4;
		let dot_token_amount = 1000_000;
		// issue dot token, and just one token
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, dot_token.into(), precise));
		let dot_token_id = <assets::NextAssetId<Test>>::get() - 1;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, dot_token_id.into(), TokenType::Token, alice, dot_token_amount));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount);

		// set swap fee
		let fee = 0;
		<SwapFee<Test>>::put(fee);
		assert_eq!(<SwapFee<Test>>::get(), fee);

		// issue intialized pool token
		let pool_token = 1000;
		<BalancerPoolToken<Test>>::put(pool_token);

		// set weight
		let total_weight = 50;
		<TotalWeight::<Test>>::put(total_weight);

		let token_symbol = AssetSymbol::DOT;
		let token_type = TokenType::Token;
		let token_amount_in = 1000;
		// first time to add liquidity
		assert_ok!(Swap::add_single_liquidity(Origin::signed(alice), token_symbol, token_type, token_amount_in));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount - 1000);
		assert_eq!(<BalancerPoolToken<Test>>::get(), 1013); // lose precision
		assert_eq!(<UserSinglePool<Test>>::get((alice, dot_token_id, token_type)), (1000, 13)); // lose precision

		// remove liqudity
		assert_ok!(Swap::remove_single_liquidity(Origin::signed(alice), token_symbol, token_type, 13));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, 999951); // lose precision
		assert_eq!(<UserSinglePool<Test>>::get((alice, dot_token_id, token_type)), (49, 0)); // 999951 + 49 == 1_000_000
		assert_eq!(<BalancerPoolToken<Test>>::get(), 1000);
	});
}

#[test]
fn remove_all_assets_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		run_to_block(2);

		// this pool has two tokens, an each one has 1000 balance, weight 1 and 49
		let raw_pool = vec![
			(0, TokenType::Token, 1000, 1),
			(1, TokenType::Token, 1000, 49)
		];
		let original_pool = (raw_pool, 0);
		<GlobalPool<Test>>::put(original_pool);

		// issue a vtoken to alice
		let alice = 1u64;
		let dot_token = vec![0x12, 0x34];
		let precise = 4;
		let dot_token_amount = 10000;
		// issue dot token
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, dot_token.into(), precise));
		let dot_token_id = <assets::NextAssetId<Test>>::get() - 1;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, dot_token_id.into(), TokenType::Token, alice, dot_token_amount));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount);

		// issue ksm token
		let ksm_token = vec![0x12, 0x56];
		let ksm_token_amount = 100000;
		assert_ok!(assets::Module::<Test>::create(Origin::ROOT, ksm_token.into(), precise));
		let ksm_token_id = <assets::NextAssetId<Test>>::get() - 1;
		assert_ok!(assets::Module::<Test>::issue(Origin::ROOT, ksm_token_id.into(), TokenType::Token, alice, ksm_token_amount));
		assert_eq!(<assets::AccountAssets<Test>>::get((ksm_token_id, TokenType::Token, alice)).balance, ksm_token_amount);

		// set swap fee
		let fee = 100;
		<SwapFee<Test>>::put(fee);
		assert_eq!(<SwapFee<Test>>::get(), fee);

		// issue intialized pool token
		let pool_token = 1000;
		<BalancerPoolToken<Test>>::put(pool_token);

		// first time to deposit to pool
		let new_pool_token = 1;
		assert_ok!(Swap::add_liquidity(Origin::signed(alice), new_pool_token));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount - 10);
		assert_eq!(<assets::AccountAssets<Test>>::get((ksm_token_id, TokenType::Token, alice)).balance, ksm_token_amount - 100);

		let gpool = <GlobalPool<Test>>::get();
		let target = vec![1010u64, 1100];
		for (p, t) in gpool.0.iter().zip(target.iter()) {
			assert_eq!(p.2, *t);
		}

		let user_pool = <UserPool::<Test>>::get(alice);
		let target = vec![10u64, 100];
		for (p, t) in user_pool.0.iter().zip(target.iter()) {
			assert_eq!(p.2, *t);
		}
		assert_eq!(user_pool.1, new_pool_token);

		// remove liquidity
		assert_ok!(Swap::remove_all_assets_liquidity(Origin::signed(alice), new_pool_token));
		assert_eq!(<assets::AccountAssets<Test>>::get((dot_token_id, TokenType::Token, alice)).balance, dot_token_amount);
		assert_eq!(<assets::AccountAssets<Test>>::get((ksm_token_id, TokenType::Token, alice)).balance, ksm_token_amount);

		let gpool = <GlobalPool<Test>>::get();
		let target = vec![1000u64, 1000];
		for (p, t) in gpool.0.iter().zip(target.iter()) {
			assert_eq!(p.2, *t);
		}

		let user_pool = <UserPool::<Test>>::get(alice);
		assert!(user_pool.0.iter().all(|p| p.2 == 0));
		assert_eq!(user_pool.1, 0);
	});
}

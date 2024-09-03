// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{
	mock::*, traits::StableAsset as StableAssetInterface, Error, MintResult, PoolCount, Pools,
	RedeemMultiResult, RedeemProportionResult, RedeemSingleResult, StableAssetPoolInfo, SwapResult,
};
use frame_support::{assert_noop, assert_ok};
use orml_traits::MultiCurrency;
use sp_runtime::DispatchError;

pub const BALANCE_OFF: u128 = 1;

fn last_event() -> RuntimeEvent {
	frame_system::pallet::Pallet::<Test>::events()
		.pop()
		.expect("RuntimeEvent expected")
		.event
}

fn create_pool() -> (i64, i64, i64, u64) {
	let coin0 = TestAssets::create_asset().expect("asset should be created");
	let coin1 = TestAssets::create_asset().expect("asset should be created");
	let pool_asset = TestAssets::create_asset().expect("asset should be created");
	let amount: Balance = 100_000_000;
	assert_ok!(TestAssets::deposit(coin1, &1, amount));
	assert_ok!(TestAssets::deposit(coin0, &1, amount));
	assert_ok!(StableAsset::create_pool(
		RuntimeOrigin::signed(1),
		pool_asset,
		vec![coin0, coin1],
		vec![10000000000u128, 10000000000u128],
		10000000u128,
		20000000u128,
		50000000u128,
		10000u128,
		2,
		1,
		1000000000000000000u128,
	));
	assert_ok!(StableAsset::set_token_rate(0, vec![(coin0, (1, 1)), (coin1, (1, 1))]));
	(coin0, coin1, pool_asset, 8319403528785522541u64)
}

#[test]
fn create_pool_successful() {
	new_test_ext().execute_with(|| {
		assert_eq!(PoolCount::<Test>::get(), 0);
		assert_ok!(StableAsset::create_pool(
			RuntimeOrigin::signed(1),
			1,
			vec![1, 2],
			vec![1u128, 1u128],
			1u128,
			1u128,
			1u128,
			1u128,
			1,
			1,
			1000000000000000000u128,
		));
		assert_eq!(
			Pools::<Test>::get(0),
			Some(StableAssetPoolInfo {
				pool_id: 0,
				pool_asset: 1,
				assets: vec![1, 2],
				precisions: vec![1u128, 1u128],
				mint_fee: 1u128,
				swap_fee: 1u128,
				redeem_fee: 1u128,
				total_supply: 0u128,
				a: 1u128,
				a_block: 0,
				future_a: 1u128,
				future_a_block: 0,
				balances: vec![0, 0],
				fee_recipient: 1,
				account_id: 8319403528785522541u64,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);
	});
}

#[test]
fn create_pool_precisions_mismatch() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			StableAsset::create_pool(
				RuntimeOrigin::signed(1),
				1,
				vec![1, 2],
				vec![1u128, 1u128, 1u128],
				1u128,
				1u128,
				1u128,
				1u128,
				1,
				1,
				1000000000000000000u128,
			),
			Error::<Test>::ArgumentsMismatch
		);
	});
}

#[test]
fn create_pool_asset_not_enough() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			StableAsset::create_pool(
				RuntimeOrigin::signed(1),
				1,
				vec![1],
				vec![1u128, 1u128, 1u128],
				1u128,
				1u128,
				1u128,
				1u128,
				1,
				1,
				1000000000000000000u128,
			),
			Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn modify_a_successful() {
	new_test_ext().execute_with(|| {
		let (coin0, coin1, pool_asset, swap_id) = create_pool();

		assert_ok!(StableAsset::modify_a(RuntimeOrigin::signed(1), 0, 100, 100));
		assert_eq!(
			Pools::<Test>::get(0),
			Some(StableAssetPoolInfo {
				pool_id: 0,
				pool_asset,
				assets: vec![coin0, coin1],
				precisions: vec![10000000000u128, 10000000000u128],
				mint_fee: 10000000u128,
				swap_fee: 20000000u128,
				redeem_fee: 50000000u128,
				total_supply: 0u128,
				a: 10000u128,
				a_block: 0,
				future_a: 100u128,
				future_a_block: 100,
				balances: vec![0, 0],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);
	});
}

#[test]
fn modify_a_argument_error_failed() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();

		assert_noop!(
			StableAsset::modify_a(RuntimeOrigin::signed(1), 0, 100, 0),
			Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn modify_a_pool_not_found() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();

		assert_noop!(
			StableAsset::modify_a(RuntimeOrigin::signed(1), 1, 100, 1000),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn mint_successful_equal_amounts() {
	new_test_ext().execute_with(|| {
		let (coin0, coin1, pool_asset, swap_id) = create_pool();

		let amounts = vec![10000000u128, 10000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_eq!(
			Pools::<Test>::get(0),
			Some(StableAssetPoolInfo {
				pool_id: 0,
				pool_asset,
				assets: vec![coin0, coin1],
				precisions: vec![10000000000u128, 10000000000u128],
				mint_fee: 10000000u128,
				swap_fee: 20000000u128,
				redeem_fee: 50000000u128,
				total_supply: 199999980000000000u128,
				a: 10000u128,
				a_block: 0,
				future_a: 10000u128,
				future_a_block: 0,
				balances: vec![99999990000000000u128, 99999990000000000u128],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);

		assert_eq!(TestAssets::free_balance(coin0, &1), 90000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin1, &1), 90000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin0, &swap_id), 10000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin1, &swap_id), 10000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(pool_asset, &1), 199800000000000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(pool_asset, &2), 200000000000000u128 - BALANCE_OFF);
	});
}

#[test]
fn mint_successful_different_amounts() {
	new_test_ext().execute_with(|| {
		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_eq!(
			Pools::<Test>::get(0),
			Some(StableAssetPoolInfo {
				pool_id: 0,
				pool_asset,
				assets: vec![coin0, coin1],
				precisions: vec![10000000000u128, 10000000000u128],
				mint_fee: 10000000u128,
				swap_fee: 20000000u128,
				redeem_fee: 50000000u128,
				total_supply: 299906783104508635u128,
				a: 10000u128,
				a_block: 0,
				future_a: 10000u128,
				future_a_block: 0,
				balances: vec![99999990000000000u128, 199999990000000000u128],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);

		assert_eq!(TestAssets::free_balance(coin0, &1), 90000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin1, &1), 80000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin0, &swap_id), 10000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin1, &swap_id), 20000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(pool_asset, &1), 299606896309149793u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(pool_asset, &2), 299906803112262u128 - BALANCE_OFF);
		if let RuntimeEvent::StableAsset(crate::pallet::Event::LiquidityAdded {
			minter: _,
			pool_id: _,
			output_amount: mint_amount,
			input_amounts: _,
			fee_amount,
			a: _,
			balances: _,
			total_supply: _,
			min_output_amount: _,
		}) = last_event()
		{
			assert_eq!(mint_amount, 299606896309149793u128);
			assert_eq!(fee_amount, 299906803112262u128);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn mint_failed_no_pool() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_noop!(
			StableAsset::mint(RuntimeOrigin::signed(1), 3, amounts, 0),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn mint_failed_too_many_amounts() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128, 20000000u128];
		assert_noop!(
			StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0),
			Error::<Test>::ArgumentsMismatch
		);
	});
}

#[test]
fn mint_failed_zero_amount() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![0u128, 20000000u128];
		assert_noop!(
			StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0),
			Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn mint_failed_under_min() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_noop!(
			StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 2000000000000000000000000u128),
			Error::<Test>::MintUnderMin
		);
	});
}

#[test]
fn mint_failed_overflow() {
	new_test_ext().execute_with(|| {
		let (coin0, coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		assert_ok!(<Test as crate::Config>::Assets::deposit(coin0, &1, 10000000000u128));
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin1, &1, 20000000000u128));
		let amounts = vec![10000000000u128, 20000000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0u128),);
	});
}

#[test]
fn swap_successful() {
	new_test_ext().execute_with(|| {
		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_ok!(StableAsset::swap(RuntimeOrigin::signed(1), 0, 0, 1, 5000000u128, 0, 2));
		assert_eq!(
			Pools::<Test>::get(0),
			Some(StableAssetPoolInfo {
				pool_id: 0,
				pool_asset,
				assets: vec![coin0, coin1],
				precisions: vec![10000000000u128, 10000000000u128],
				mint_fee: 10000000u128,
				swap_fee: 20000000u128,
				redeem_fee: 50000000u128,
				total_supply: 300006969999594867u128,
				a: 10000u128,
				a_block: 0,
				future_a: 10000u128,
				future_a_block: 0,
				balances: vec![149999990000000000u128, 150006980000000000u128],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);
		assert_eq!(TestAssets::free_balance(coin0, &1), 85000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin1, &1), 84999301u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin0, &swap_id), 15000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin1, &swap_id), 15000699u128 - BALANCE_OFF);
		if let RuntimeEvent::StableAsset(crate::pallet::Event::TokenSwapped {
			swapper: _,
			pool_id: _,
			input_asset: _,
			output_asset: _,
			input_amount: dx,
			output_amount: dy,
			a: _,
			balances: _,
			total_supply: _,
			min_output_amount: _,
		}) = last_event()
		{
			assert_eq!(dx, 5000000u128);
			assert_eq!(dy, 4999301u128);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn swap_failed_same_token() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::swap(RuntimeOrigin::signed(1), 0, 1, 1, 5000000u128, 0, 2),
			Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn swap_failed_no_pool() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::swap(RuntimeOrigin::signed(1), 3, 0, 1, 5000000u128, 0, 2),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn swap_failed_invalid_first_token() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::swap(RuntimeOrigin::signed(1), 0, 2, 1, 5000000u128, 0, 2),
			Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn swap_failed_invalid_second_token() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::swap(RuntimeOrigin::signed(1), 0, 0, 2, 5000000u128, 0, 2),
			Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn swap_failed_invalid_amount() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::swap(RuntimeOrigin::signed(1), 0, 0, 1, 0u128, 0, 2),
			Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn swap_failed_under_min() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::swap(
				RuntimeOrigin::signed(1),
				0,
				0,
				1,
				5000000u128,
				50000000000000000u128,
				2
			),
			Error::<Test>::SwapUnderMin
		);
	});
}

#[test]
fn swap_failed_under_overflow() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::swap(RuntimeOrigin::signed(1), 0, 0, 1, 500000000u128, 0u128, 2),
			DispatchError::Other("Overflow")
		);
	});
}

#[test]
fn redeem_proportion_successful() {
	new_test_ext().execute_with(|| {
		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_ok!(StableAsset::redeem_proportion(
			RuntimeOrigin::signed(1),
			0,
			100000000000000000u128,
			vec![0u128, 0u128]
		));
		assert_eq!(
			Pools::<Test>::get(0),
			Some(StableAssetPoolInfo {
				pool_id: 0,
				pool_asset,
				assets: vec![coin0, coin1],
				precisions: vec![10000000000u128, 10000000000u128],
				mint_fee: 10000000u128,
				swap_fee: 20000000u128,
				redeem_fee: 50000000u128,
				total_supply: 200406793007994387u128,
				a: 10000u128,
				a_block: 0,
				future_a: 10000u128,
				future_a_block: 0,
				balances: vec![66823020000000000u128, 133646050000000000u128],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);
		assert_eq!(TestAssets::free_balance(coin0, &1), 93317697u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin1, &1), 86635394u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin0, &swap_id), 6682303u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin1, &swap_id), 13364606u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(pool_asset, &1), 199606896309149793u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(pool_asset, &2), 799916706598014u128 - BALANCE_OFF);
		if let RuntimeEvent::StableAsset(crate::pallet::Event::RedeemedProportion {
			redeemer: _,
			pool_id: _,
			input_amount: amount,
			output_amounts: amounts,
			fee_amount,
			a: _,
			balances: _,
			total_supply: _,
			min_output_amounts: _,
		}) = last_event()
		{
			assert_eq!(amount, 100000000000000000u128);
			assert_eq!(amounts, vec![3317697u128, 6635394u128]);
			assert_eq!(fee_amount, 500000000000000u128);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn redeem_proportion_failed_zero_amount() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_proportion(RuntimeOrigin::signed(1), 0, 0u128, vec![0u128, 0u128]),
			Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn redeem_proportion_failed_limits_mismatch() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_proportion(
				RuntimeOrigin::signed(1),
				0,
				100000000000000000u128,
				vec![0u128, 0u128, 0u128]
			),
			Error::<Test>::ArgumentsMismatch
		);
	});
}

#[test]
fn redeem_proportion_failed_overflow() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_proportion(
				RuntimeOrigin::signed(1),
				0,
				10000000000000000000u128,
				vec![0u128, 0u128]
			),
			Error::<Test>::Math
		);
	});
}

#[test]
fn redeem_proportion_failed_limits_breached() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_proportion(
				RuntimeOrigin::signed(1),
				0,
				100000000000000000u128,
				vec![100000000000000000u128, 0u128]
			),
			Error::<Test>::RedeemUnderMin
		);
	});
}

#[test]
fn redeem_proportion_failed_no_pool() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_proportion(
				RuntimeOrigin::signed(1),
				3,
				100000000000000000u128,
				vec![0u128, 0u128]
			),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn redeem_single_successful() {
	new_test_ext().execute_with(|| {
		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_ok!(StableAsset::redeem_single(
			RuntimeOrigin::signed(1),
			0,
			100000000000000000u128,
			0,
			0u128,
			2,
		));
		assert_eq!(
			Pools::<Test>::get(0),
			Some(StableAssetPoolInfo {
				pool_id: 0,
				pool_asset,
				assets: vec![coin0, coin1],
				precisions: vec![10000000000u128, 10000000000u128],
				mint_fee: 10000000u128,
				swap_fee: 20000000u128,
				redeem_fee: 50000000u128,
				total_supply: 200406798920642765u128,
				a: 10000u128,
				a_block: 0,
				future_a: 10000u128,
				future_a_block: 0,
				balances: vec![4968380000000000u128, 199999990000000000u128],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);
		assert_eq!(TestAssets::free_balance(coin0, &1), 99503160u128);
		assert_eq!(TestAssets::free_balance(coin1, &1), 80000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin0, &swap_id), 496838u128);
		assert_eq!(TestAssets::free_balance(coin1, &swap_id), 20000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(pool_asset, &1), 199606896309149793u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(pool_asset, &2), 799922619246391u128);
		if let RuntimeEvent::StableAsset(crate::pallet::Event::RedeemedSingle {
			redeemer: _,
			pool_id: _,
			input_amount,
			output_amount,
			fee_amount,
			a: _,
			balances: _,
			total_supply: _,
			min_output_amount: _,
			output_asset: _,
		}) = last_event()
		{
			assert_eq!(input_amount, 100000000000000000u128);
			assert_eq!(output_amount, 9503161u128);
			assert_eq!(fee_amount, 500000000000000u128);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn redeem_single_failed_zero_amount() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_single(RuntimeOrigin::signed(1), 0, 0u128, 0, 0u128, 2),
			Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn redeem_single_failed_overflow() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_single(
				RuntimeOrigin::signed(1),
				0,
				1000000000000000000u128,
				0,
				0u128,
				2
			),
			Error::<Test>::Math
		);
	});
}

#[test]
fn redeem_single_failed_under_min() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_single(
				RuntimeOrigin::signed(1),
				0,
				100000000000000000u128,
				0,
				100000000000000000u128,
				2
			),
			Error::<Test>::RedeemUnderMin
		);
	});
}

#[test]
fn redeem_single_failed_invalid_token() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_single(
				RuntimeOrigin::signed(1),
				0,
				100000000000000000u128,
				3,
				0u128,
				2
			),
			Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn redeem_single_failed_no_pool() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_single(
				RuntimeOrigin::signed(1),
				3,
				100000000000000000u128,
				3,
				0u128,
				2
			),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn redeem_multi_successful() {
	new_test_ext().execute_with(|| {
		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_ok!(StableAsset::redeem_multi(
			RuntimeOrigin::signed(1),
			0,
			vec![5000000u128, 5000000u128],
			1100000000000000000u128,
		));
		assert_eq!(
			Pools::<Test>::get(0),
			Some(StableAssetPoolInfo {
				pool_id: 0,
				pool_asset,
				assets: vec![coin0, coin1],
				precisions: vec![10000000000u128, 10000000000u128],
				mint_fee: 10000000u128,
				swap_fee: 20000000u128,
				redeem_fee: 50000000u128,
				total_supply: 199834552642910514u128,
				a: 10000u128,
				a_block: 0,
				future_a: 10000u128,
				future_a_block: 0,
				balances: vec![49999990000000000u128, 149999990000000000u128],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);
		assert_eq!(TestAssets::free_balance(coin0, &1), 95000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin1, &1), 85000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin0, &swap_id), 5000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(coin1, &swap_id), 15000000u128 - BALANCE_OFF);
		assert_eq!(TestAssets::free_balance(pool_asset, &1), 199031790317593892u128);
		assert_eq!(TestAssets::free_balance(pool_asset, &2), 802782333070040u128);
		if let RuntimeEvent::StableAsset(crate::pallet::Event::RedeemedMulti {
			redeemer: _,
			pool_id: _,
			output_amounts: amounts,
			input_amount: amount,
			fee_amount,
			a: _,
			balances: _,
			total_supply: _,
			max_input_amount: _,
		}) = last_event()
		{
			assert_eq!(amount, 100575105991555900u128);
			assert_eq!(amounts, vec![5000000u128, 5000000u128]);
			assert_eq!(fee_amount, 502875529957779u128);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn redeem_multi_failed_not_enough_assets() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_multi(
				RuntimeOrigin::signed(1),
				0,
				vec![1000000000u128, 1000000000u128],
				1100000000000000000u128,
			),
			Error::<Test>::Math
		);
	});
}

#[test]
fn redeem_multi_failed_over_max() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_multi(
				RuntimeOrigin::signed(1),
				0,
				vec![5000000u128, 5000000u128],
				110000u128,
			),
			Error::<Test>::RedeemOverMax
		);
	});
}

#[test]
fn redeem_multi_failed_no_pool() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_noop!(
			StableAsset::redeem_multi(
				RuntimeOrigin::signed(1),
				1,
				vec![5000000u128, 5000000u128],
				110000u128,
			),
			Error::<Test>::PoolNotFound
		);
	});
}

#[test]
fn swap_exact_success() {
	new_test_ext().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 10000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));

		let amount = 1000345u128;
		let pool_info = Pools::<Test>::get(0).unwrap();

		let result = StableAsset::get_swap_amount_exact(&pool_info, 0, 1, amount).unwrap();
		let result_two = StableAsset::get_swap_amount(&pool_info, 0, 1, result.dx).unwrap();
		assert_eq!(result_two.dy >= amount, true);
	});
}

#[test]
fn swap_exact_success_different_precision() {
	new_test_ext().execute_with(|| {
		let coin0 = TestAssets::create_asset().expect("asset should be created");
		let coin1 = TestAssets::create_asset().expect("asset should be created");
		let pool_asset = TestAssets::create_asset().expect("asset should be created");
		let amount: Balance = 100_000_000;
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin0, &1, amount));
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin1, &1, amount));
		assert_ok!(StableAsset::create_pool(
			RuntimeOrigin::signed(1),
			pool_asset,
			vec![coin0, coin1],
			vec![1u128, 100000u128], // precision
			10000000u128,
			20000000u128,
			50000000u128,
			10000u128,
			2,
			1,
			1000000000000000000u128,
		));
		System::set_block_number(2);

		let amounts = vec![10000000u128, 10000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));

		let amount = 1000345u128;
		let pool_info = Pools::<Test>::get(0).unwrap();

		let result = StableAsset::get_swap_amount_exact(&pool_info, 0, 1, amount).unwrap();
		let result_two = StableAsset::get_swap_amount(&pool_info, 0, 1, result.dx).unwrap();
		assert_eq!(result_two.dy >= amount, true);
	});
}

#[test]
fn modify_fees_successful() {
	new_test_ext().execute_with(|| {
		let (coin0, coin1, pool_asset, swap_id) = create_pool();

		assert_ok!(StableAsset::modify_fees(
			RuntimeOrigin::signed(1),
			0,
			Some(100),
			Some(200),
			Some(300)
		));
		assert_eq!(
			Pools::<Test>::get(0),
			Some(StableAssetPoolInfo {
				pool_id: 0,
				pool_asset,
				assets: vec![coin0, coin1],
				precisions: vec![10000000000u128, 10000000000u128],
				mint_fee: 100u128,
				swap_fee: 200u128,
				redeem_fee: 300u128,
				total_supply: 0u128,
				a: 10000u128,
				a_block: 0,
				future_a: 10000u128,
				future_a_block: 0,
				balances: vec![0, 0],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);
	});
}

#[test]
fn get_mint_amount_same_as_mint() {
	new_test_ext().execute_with(|| {
		let (coin0, _coin1, _pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts.clone(), 0));
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin0, &swap_id, 100_000_000_000));

		let pool_info = Pools::<Test>::get(0).unwrap();
		assert_eq!(pool_info.balances, vec![99999990000000000u128, 199999990000000000u128]);
		assert_eq!(
			StableAsset::get_balance_update_amount(&pool_info).unwrap().balances,
			vec![1000099999990000000000u128, 199999990000000000u128]
		);

		assert_eq!(
			StableAsset::get_mint_amount(&pool_info, &amounts).unwrap(),
			MintResult {
				mint_amount: 91642963023874214130u128,
				fee_amount: 91734697721595809u128,
				balances: vec![1000199999990000000000u128, 399999990000000000u128],
				total_supply: 623501681289305978589u128,
			}
		);

		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		if let RuntimeEvent::StableAsset(crate::pallet::Event::LiquidityAdded {
			minter: _,
			pool_id: _,
			a: _,
			input_amounts: _,
			min_output_amount: _,
			balances: _,
			total_supply,
			fee_amount,
			output_amount,
		}) = last_event()
		{
			assert_eq!(total_supply, 623501681289305978589u128);
			assert_eq!(fee_amount, 91734697721595809u128);
			assert_eq!(output_amount, 91642963023874214130u128);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn get_swap_amount_same_as_swap() {
	new_test_ext().execute_with(|| {
		let (coin0, _coin1, _pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin0, &swap_id, 100_000_000_000));

		let pool_info = Pools::<Test>::get(0).unwrap();
		assert_eq!(pool_info.balances, vec![99999990000000000u128, 199999990000000000u128]);
		assert_eq!(
			StableAsset::get_balance_update_amount(&pool_info).unwrap().balances,
			vec![1000099999990000000000u128, 199999990000000000u128]
		);

		assert_eq!(
			StableAsset::get_swap_amount(&pool_info, 0, 1, 5000000u128).unwrap(),
			SwapResult {
				dx: 5000000u128,
				dy: 3120u128,
				y: 199968725309184067u128,
				balance_i: 1000149999990000000000u128
			}
		);

		assert_ok!(StableAsset::swap(RuntimeOrigin::signed(1), 0, 0, 1, 5000000u128, 0, 2));
		if let RuntimeEvent::StableAsset(crate::pallet::Event::TokenSwapped {
			swapper: _,
			pool_id: _,
			input_asset: _,
			output_asset: _,
			input_amount: dx,
			output_amount: dy,
			a: _,
			balances: _,
			total_supply: _,
			min_output_amount: _,
		}) = last_event()
		{
			assert_eq!(dx, 5000000u128);
			assert_eq!(dy, 3120u128);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn get_swap_amount_exact_same_as_swap() {
	new_test_ext().execute_with(|| {
		let (coin0, _coin1, _pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin0, &swap_id, 100_000_000_000));

		let pool_info = Pools::<Test>::get(0).unwrap();
		assert_eq!(pool_info.balances, vec![99999990000000000u128, 199999990000000000u128]);
		assert_eq!(
			StableAsset::get_balance_update_amount(&pool_info).unwrap().balances,
			vec![1000099999990000000000u128, 199999990000000000u128]
		);

		assert_eq!(
			StableAsset::get_swap_amount_exact(&pool_info, 0, 1, 3120u128).unwrap(),
			SwapResult {
				dx: 4999349u128,
				dy: 3120u128,
				y: 1000149992487303920870u128,
				balance_i: 1000099999990000000000u128
			}
		);

		assert_ok!(StableAsset::swap(RuntimeOrigin::signed(1), 0, 0, 1, 4999349u128, 0, 2));
		if let RuntimeEvent::StableAsset(crate::pallet::Event::TokenSwapped {
			swapper: _,
			pool_id: _,
			input_asset: _,
			output_asset: _,
			input_amount: dx,
			output_amount: dy,
			a: _,
			balances: _,
			total_supply: _,
			min_output_amount: _,
		}) = last_event()
		{
			assert_eq!(dx, 4999349u128);
			assert_eq!(dy, 3120u128);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn get_redeem_proportion_amount_same_as_redeem_proportion() {
	new_test_ext().execute_with(|| {
		let (coin0, _coin1, _pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin0, &swap_id, 100_000_000_000));

		let pool_info = Pools::<Test>::get(0).unwrap();
		assert_eq!(pool_info.balances, vec![99999990000000000u128, 199999990000000000u128]);
		assert_eq!(
			StableAsset::get_balance_update_amount(&pool_info).unwrap().balances,
			vec![1000099999990000000000u128, 199999990000000000u128]
		);

		assert_eq!(
			StableAsset::get_redeem_proportion_amount(&pool_info, 100000000000000000u128).unwrap(),
			RedeemProportionResult {
				amounts: vec![18713074u128, 3742u128],
				balances: vec![999912869248389498442u128, 199962567595789254u128],
				fee_amount: 500000000000000u128,
				total_supply: 531667483567710168650u128,
				redeem_amount: 99500000000000000u128,
			}
		);

		assert_ok!(StableAsset::redeem_proportion(
			RuntimeOrigin::signed(1),
			0,
			100000000000000000u128,
			vec![0u128, 0u128]
		));
		if let RuntimeEvent::StableAsset(crate::pallet::Event::RedeemedProportion {
			redeemer: _,
			pool_id: _,
			a: _,
			input_amount: _,
			min_output_amounts: _,
			balances: _,
			total_supply,
			fee_amount,
			output_amounts,
		}) = last_event()
		{
			assert_eq!(total_supply, 531667485117219048370u128);
			assert_eq!(fee_amount, 500000000000000u128);
			assert_eq!(output_amounts, vec![18713074u128, 3742u128]);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn get_redeem_single_amount_same_as_redeem_single() {
	new_test_ext().execute_with(|| {
		let (coin0, _coin1, _pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin0, &swap_id, 100_000_000_000));

		let pool_info = Pools::<Test>::get(0).unwrap();
		assert_eq!(pool_info.balances, vec![99999990000000000u128, 199999990000000000u128]);
		assert_eq!(
			StableAsset::get_balance_update_amount(&pool_info).unwrap().balances,
			vec![1000099999990000000000u128, 199999990000000000u128]
		);

		assert_eq!(
			StableAsset::get_redeem_single_amount(&pool_info, 100000000000000000u128, 0).unwrap(),
			RedeemSingleResult {
				dy: 24696237u128,
				fee_amount: 500000000000000u128,
				total_supply: 531667483567710168650u128,
				balances: vec![999853037611795513052u128, 199999990000000000u128],
				redeem_amount: 99500000000000000u128,
			}
		);

		assert_ok!(StableAsset::redeem_single(
			RuntimeOrigin::signed(1),
			0,
			100000000000000000u128,
			0,
			0u128,
			2,
		));
		if let RuntimeEvent::StableAsset(crate::pallet::Event::RedeemedSingle {
			redeemer: _,
			pool_id: _,
			a: _,
			input_amount: _,
			output_asset: _,
			min_output_amount: _,
			balances: _,
			total_supply: _,
			fee_amount,
			output_amount,
		}) = last_event()
		{
			assert_eq!(fee_amount, 500000000000000u128);
			assert_eq!(output_amount, 24696237u128);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn get_redeem_multi_amount_same_as_redeem_multi() {
	new_test_ext().execute_with(|| {
		let (coin0, _coin1, _pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts, 0));
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin0, &swap_id, 100_000_000_000));

		let pool_info = Pools::<Test>::get(0).unwrap();
		assert_eq!(pool_info.balances, vec![99999990000000000u128, 199999990000000000u128]);
		assert_eq!(
			StableAsset::get_balance_update_amount(&pool_info).unwrap().balances,
			vec![1000099999990000000000u128, 199999990000000000u128]
		);

		assert_eq!(
			StableAsset::get_redeem_multi_amount(&pool_info, &vec![5000000u128, 5000000u128])
				.unwrap(),
			RedeemMultiResult {
				redeem_amount: 36679896644936924929u128,
				fee_amount: 183399483224684624u128,
				balances: vec![1000049999990000000000u128, 149999990000000000u128],
				total_supply: 495270486405997928345u128,
				burn_amount: 36496497161712240305u128,
			}
		);

		assert_ok!(StableAsset::redeem_multi(
			RuntimeOrigin::signed(1),
			0,
			vec![5000000u128, 5000000u128],
			40000000000000000000u128,
		));
		if let RuntimeEvent::StableAsset(crate::pallet::Event::RedeemedMulti {
			redeemer: _,
			pool_id: _,
			a: _,
			output_amounts: _,
			max_input_amount: _,
			balances: _,
			total_supply: _,
			fee_amount,
			input_amount,
		}) = last_event()
		{
			assert_eq!(fee_amount, 183399483224684624u128);
			assert_eq!(input_amount, 36679896644936924929u128);
		} else {
			panic!("Unexpected event");
		}
	});
}

#[test]
fn swap_should_work_with_unbalance_coin0() {
	new_test_ext().execute_with(|| {
		let (coin0, _coin1, _pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 10000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts.clone(), 0));

		// increase the balance of coin0
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin0, &swap_id, 100_000_000_000));

		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts.clone(), 0));
		assert_ok!(StableAsset::swap(RuntimeOrigin::signed(1), 0, 0, 1, 5000000u128, 0, 2));
		assert_ok!(StableAsset::swap(RuntimeOrigin::signed(1), 0, 1, 0, 5000000u128, 0, 2));

		assert_ok!(StableAsset::redeem_proportion(
			RuntimeOrigin::signed(1),
			0,
			100000000000000000u128,
			vec![0u128, 0u128]
		));
		assert_ok!(StableAsset::redeem_single(
			RuntimeOrigin::signed(1),
			0,
			100000000000000000u128,
			0,
			0u128,
			2,
		));
		assert_ok!(StableAsset::redeem_single(
			RuntimeOrigin::signed(1),
			0,
			100000000000000000u128,
			1,
			0u128,
			2,
		));
		assert_ok!(StableAsset::redeem_multi(
			RuntimeOrigin::signed(1),
			0,
			vec![5000000u128, 5000000u128],
			40000000000000000000u128,
		));
	});
}

#[test]
fn swap_should_work_with_unbalance_coin1() {
	new_test_ext().execute_with(|| {
		let (_coin0, coin1, _pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 10000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts.clone(), 0));

		// increase the balance of coin1
		assert_ok!(<Test as crate::Config>::Assets::deposit(coin1, &swap_id, 100_000_000_000));

		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(1), 0, amounts.clone(), 0));
		assert_ok!(StableAsset::swap(RuntimeOrigin::signed(1), 0, 0, 1, 5000000u128, 0, 2));
		assert_ok!(StableAsset::swap(RuntimeOrigin::signed(1), 0, 1, 0, 5000000u128, 0, 2));

		assert_ok!(StableAsset::redeem_proportion(
			RuntimeOrigin::signed(1),
			0,
			100000000000000000u128,
			vec![0u128, 0u128]
		));
		assert_ok!(StableAsset::redeem_single(
			RuntimeOrigin::signed(1),
			0,
			100000000000000000u128,
			0,
			0u128,
			2,
		));
		assert_ok!(StableAsset::redeem_single(
			RuntimeOrigin::signed(1),
			0,
			100000000000000000u128,
			1,
			0u128,
			2,
		));
		assert_ok!(StableAsset::redeem_multi(
			RuntimeOrigin::signed(1),
			0,
			vec![5000000u128, 5000000u128],
			40000000000000000000u128,
		));
	});
}

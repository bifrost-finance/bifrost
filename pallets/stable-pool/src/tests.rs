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
use crate::{mock::*, AssetIdOf, AtLeast64BitUnsignedOf, Error};
use bifrost_primitives::{StableAssetPalletId, VtokenMintingOperator};
use bifrost_stable_asset::{PoolCount, Pools, StableAssetPoolInfo};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use orml_traits::MultiCurrency;
use sp_runtime::{traits::AccountIdConversion, Permill};

pub const BALANCE_OFF: u128 = 0;

fn create_pool() -> (CurrencyId, CurrencyId, CurrencyId, u128) {
	let coin0 = DOT;
	let coin1 = VDOT;
	let pool_asset: CurrencyId = CurrencyId::BLP(0);

	assert_ok!(StablePool::create_pool(
		RuntimeOrigin::root(),
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
	(coin0, coin1, pool_asset, 30160825295207673652903702381u128)
}

fn create_pool2() -> (CurrencyId, CurrencyId, CurrencyId, u128) {
	let coin0 = DOT;
	let coin1 = VDOT;
	let pool_asset = CurrencyId::BLP(0);

	assert_ok!(StablePool::create_pool(
		RuntimeOrigin::root(),
		vec![coin0, coin1],
		vec![1u128, 1u128],
		10000000u128,
		20000000u128,
		50000000u128,
		10000u128,
		2,
		1,
		1_000_000_000_000u128,
	));
	(coin0, coin1, pool_asset, 30160825295207673652903702381u128)
}

fn create_movr_pool() -> (CurrencyId, CurrencyId, CurrencyId, u128) {
	let coin0 = MOVR;
	let coin1 = VMOVR;
	let pool_asset: CurrencyId = CurrencyId::BLP(0);

	assert_ok!(StablePool::create_pool(
		RuntimeOrigin::root(),
		vec![coin0, coin1],
		vec![1u128, 1u128],
		10000000u128,
		20000000u128,
		50000000u128,
		10000u128,
		2,
		1,
		million_unit(1),
	));
	(coin0, coin1, pool_asset, 30160825295207673652903702381u128)
}

fn init() -> (CurrencyId, CurrencyId, CurrencyId, u128) {
	assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
	assert_ok!(VtokenMinting::mint(Some(3).into(), DOT, 100_000_000, BoundedVec::default(), None));
	let (coin0, coin1, pool_asset, swap_id) = create_pool();
	System::set_block_number(2);
	let amounts = vec![10000000u128, 20000000u128];
	assert_ok!(StableAsset::mint(RuntimeOrigin::signed(3), 0, amounts, 0));
	assert_ok!(StableAsset::swap(RuntimeOrigin::signed(3), 0, 0, 1, 5000000u128, 0, 2));
	assert_eq!(Tokens::free_balance(DOT, &3), 85000000u128 - BALANCE_OFF);
	(coin0, coin1, pool_asset, swap_id)
}

#[test]
fn modify_a_argument_error_failed() {
	env_logger::try_init().unwrap_or(());
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();

		assert_noop!(
			StableAsset::modify_a(RuntimeOrigin::signed(1), 0, 100, 0),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn calc() {
	env_logger::try_init().unwrap_or(());
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();

		assert_noop!(
			StableAsset::modify_a(RuntimeOrigin::signed(1), 0, 100, 0),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn create_pool_successful() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let coin0 = DOT;
		let coin1 = VDOT;
		assert_eq!(PoolCount::<Test>::get(), 0);
		assert_ok!(StablePool::create_pool(
			RuntimeOrigin::root(),
			vec![coin0, coin1],
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
				pool_asset: CurrencyId::BLP(0),
				assets: vec![coin0, coin1],
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
				account_id: 30160825295207673652903702381u128,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);
	});
}

#[test]
fn mint_successful_equal_amounts() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		assert_ok!(VtokenMinting::mint(
			Some(3).into(),
			DOT,
			100_000_000,
			BoundedVec::default(),
			None
		));
		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		System::set_block_number(2);
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		));
		let amounts = vec![10000000u128, 10000000u128];
		assert_noop!(
			StableAsset::mint(
				RuntimeOrigin::signed(1),
				0,
				amounts.clone(),
				2000000000000000000000000u128
			),
			bifrost_stable_asset::Error::<Test>::MintUnderMin
		);
		assert_ok!(StablePool::mint_inner(&3, 0, amounts, 0));
		// assert_ok!(StableAsset::mint(RuntimeOrigin::signed(3), 0, amounts.clone(), 0));
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
				total_supply: 200000000000000000u128,
				a: 10000u128,
				a_block: 0,
				future_a: 10000u128,
				future_a_block: 0,
				balances: vec![100000000000000000u128, 100000000000000000u128],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);

		assert_eq!(Tokens::free_balance(coin0, &3), 90000000u128 + BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &3), 90000000u128 + BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin0, &swap_id), 10000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &swap_id), 10000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(pool_asset, &3), 199800000000000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(pool_asset, &2), 200000000000000u128 - BALANCE_OFF);
		// fee_recipient
	});
}

#[test]
fn swap_successful() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		assert_ok!(VtokenMinting::mint(
			Some(3).into(),
			DOT,
			100_000_000,
			BoundedVec::default(),
			None
		));
		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(3), 0, amounts, 0));
		assert_ok!(StableAsset::swap(RuntimeOrigin::signed(3), 0, 0, 1, 5000000u128, 0, 2));
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
				total_supply: 300006989999594867u128,
				a: 10000u128,
				a_block: 0,
				future_a: 10000u128,
				future_a_block: 0,
				balances: vec![150000000000000000u128, 150006990000000000u128],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);
		assert_eq!(Tokens::free_balance(coin0, &3), 85000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &3), 84999301u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin0, &swap_id), 15000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &swap_id), 15000699u128 - BALANCE_OFF);
	});
}

#[test]
fn get_swap_output_amount() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		env_logger::try_init().unwrap_or(());
		System::set_block_number(2);
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		assert_ok!(VtokenMinting::mint(
			Some(3).into(),
			DOT,
			100_000_000,
			BoundedVec::default(),
			None
		));

		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		));
		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StablePool::mint_inner(&3, 0, amounts, 0));
		assert_eq!(StablePool::get_swap_output(0, 0, 1, 5000000u128).ok(), Some(4999301));
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 4999302),
			Error::<Test>::SwapUnderMin
		);
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 4999301));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(DOT, (1, 1)), (VDOT, (90_000_000, 100_000_000))]
		));
		assert_eq!(StablePool::get_swap_output(0, 0, 1, 5000000u128).ok(), Some(4485945));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		));
		assert_eq!(StablePool::get_swap_output(0, 0, 1, 5000000u128).ok(), Some(4980724));
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 1, 1, 5000000u128, 0),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 3, 0, 1, 5000000u128, 0),
			bifrost_stable_asset::Error::<Test>::PoolNotFound
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 2, 1, 5000000u128, 0),
			bifrost_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 0, 2, 5000000u128, 0),
			bifrost_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 0, 1, 0u128, 0),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 0, 1, 500000000u128, 0u128),
			orml_tokens::Error::<Test>::BalanceTooLow
		);
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
				total_supply: 300006989999594867u128,
				a: 10000u128,
				a_block: 2,
				future_a: 10000u128,
				future_a_block: 2,
				balances: vec![150000000000000000u128, 150006990000000000u128],
				fee_recipient: 2,
				account_id: swap_id,
				yield_recipient: 1,
				precision: 1000000000000000000u128,
			})
		);
		assert_eq!(Tokens::free_balance(coin0, &3), 85000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &3), 84999301u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin0, &swap_id), 15000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &swap_id), 15000699u128 - BALANCE_OFF);
	});
}

#[test]
fn mint_swap_redeem1() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		assert_ok!(VtokenMinting::mint(
			Some(3).into(),
			DOT,
			100_000_000,
			BoundedVec::default(),
			None
		));
		assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), 3, VDOT, 90_000_000, 0));

		let (coin0, coin1, _pool_asset, swap_id) = create_pool();
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(DOT, (1, 1)), (VDOT, (90_000_000, 100_000_000))]
		));
		System::set_block_number(2);

		let amounts = vec![10_000_000u128, 20_000_000u128];
		assert_ok!(StablePool::mint_inner(&3, 0, amounts, 0));
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(Tokens::free_balance(coin0, &3), 85000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &3), 74502743u128 - BALANCE_OFF); // 90_000_000 - 22_222_222 + 4_502_743
		assert_eq!(Tokens::free_balance(coin0, &swap_id), 15000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &swap_id), 15497257u128 - BALANCE_OFF);
		assert_ok!(StablePool::on_swap(&4u128, 0, 0, 1, 15_000_000u128, 0));
		assert_ok!(StablePool::on_swap(&1u128, 0, 0, 1, 500_000_000u128, 0));
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 0, 15_000_000u128, vec![0]),
			bifrost_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_ok!(StablePool::redeem_proportion_inner(&3, 0, 15_000_000_000_000u128, vec![0, 0]));
	});
}

#[test]
fn mint_swap_redeem2() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		assert_ok!(VtokenMinting::mint(
			Some(3).into(),
			DOT,
			100_000_000,
			BoundedVec::default(),
			None
		));
		assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), 3, VDOT, 90_000_000, 0));
		let (coin0, coin1, pool_asset, swap_id) = create_pool2();
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(DOT, (1, 1)), (VDOT, (90_000_000, 100_000_000))]
		));
		let amounts = vec![10_000_000u128, 20_000_000u128];
		assert_ok!(StablePool::mint_inner(&3, 0, amounts, 0));
		assert_eq!(Tokens::free_balance(coin1, &3), 70_000_000 - BALANCE_OFF); // 90_000_000 - 20_000_000
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(Tokens::free_balance(coin0, &3), 85000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &3), 74502743u128 - BALANCE_OFF); // 90_000_000 - 20_000_000 + 4502743
		assert_eq!(Tokens::free_balance(coin0, &swap_id), 15000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &swap_id), 15497257u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(pool_asset, &3), 32176560);
		assert_eq!(Tokens::free_balance(pool_asset, &2), 42231);
		assert_eq!(Tokens::free_balance(pool_asset, &1), 0);
		assert_eq!(
			Tokens::free_balance(pool_asset, &2) + Tokens::free_balance(pool_asset, &3),
			<Test as crate::Config>::MultiCurrency::total_issuance(pool_asset)
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 0, 15_000_000u128, vec![0]),
			bifrost_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 0, 0u128, vec![0u128, 0u128]),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 0, 15_000_000u128, vec![0, 0, 0]),
			bifrost_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(
				&3,
				0,
				15_000_000u128,
				vec![100000000000000000u128, 0]
			),
			bifrost_stable_asset::Error::<Test>::RedeemUnderMin
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 3, 15_000_000u128, vec![0, 0]),
			bifrost_stable_asset::Error::<Test>::PoolNotFound
		);
		let pool_account: u128 = StableAssetPalletId::get().into_account_truncating();
		assert_eq!(Tokens::free_balance(coin0, &pool_account), 15000000);
		assert_eq!(Tokens::free_balance(coin0, &3), 85000000u128 - BALANCE_OFF);
		assert_ok!(StablePool::redeem_proportion_inner(&3, 0, 32176560, vec![0, 0]));
		assert_eq!(Tokens::free_balance(pool_asset, &2), 203114);
		assert_eq!(Tokens::free_balance(coin0, &pool_account), 94563);
		let free = 85000000 + 15000000 - 94563; // 99905437
		assert_eq!(Tokens::free_balance(coin0, &3), free);
	});
}

#[test]
fn mint_swap_redeem_for_precisions() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		assert_ok!(VtokenMinting::mint(
			Some(3).into(),
			DOT,
			100_000_000,
			BoundedVec::default(),
			None
		));
		assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), 3, VDOT, 90_000_000, 0));
		let (coin0, coin1, _pool_asset, _swap_id) = create_pool2();
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(DOT, (1, 1)), (VDOT, (90_000_000, 100_000_000))]
		));
		let amounts = vec![10_000_000u128, 20_000_000u128];
		assert_ok!(StablePool::mint_inner(&3, 0, amounts, 0));
		assert_eq!(Tokens::free_balance(coin1, &3), 70_000_000 - BALANCE_OFF); // 90_000_000 - 20_000_000
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(Tokens::free_balance(coin0, &3), 85000000u128 - BALANCE_OFF);
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 0, 15_000_000u128, vec![0]),
			bifrost_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_ok!(StablePool::redeem_proportion_inner(&3, 0, 32176560, vec![0, 0]));
	});
}

#[test]
fn redeem_single() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let coin0 = DOT;
		let coin1 = VDOT;
		let pool_asset = CurrencyId::BLP(0);

		assert_ok!(<Test as crate::Config>::MultiCurrency::deposit(
			coin0.into(),
			&6,
			1_000_000_000_000u128
		));
		assert_ok!(<Test as crate::Config>::MultiCurrency::deposit(
			coin1.into(),
			&6,
			1_000_000_000_000u128
		));
		assert_eq!(Tokens::free_balance(coin0, &6), 1_000_000_000_000u128);

		let amounts = vec![100_000_000_000u128, 100_000_000_000u128];
		assert_ok!(StablePool::create_pool(
			RuntimeOrigin::root(),
			vec![coin0.into(), coin1.into()],
			vec![1u128.into(), 1u128.into()],
			0u128.into(),
			0u128.into(),
			0u128.into(),
			220u128.into(),
			5,
			5,
			1000000000000u128.into()
		));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(DOT, (1, 1)), (VDOT, (10, 11))]
		));
		assert_ok!(StablePool::add_liquidity(RuntimeOrigin::signed(6).into(), 0, amounts, 0));
		assert_eq!(Tokens::free_balance(coin0, &6), 900_000_000_000u128);
		assert_eq!(Tokens::free_balance(coin1, &6), 900_000_000_000u128);
		assert_eq!(Tokens::free_balance(pool_asset, &6), 209_955_833_377);
		assert_ok!(StablePool::redeem_single(
			RuntimeOrigin::signed(6).into(),
			0,
			5_000_000_000u128,
			0,
			0,
			2
		));
		assert_noop!(
			StablePool::redeem_single(RuntimeOrigin::signed(6), 0, 0u128, 0, 0u128, 2),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_noop!(
			StablePool::redeem_single(
				RuntimeOrigin::signed(6),
				0,
				1000000000000000000u128,
				0,
				0u128,
				2
			),
			bifrost_stable_asset::Error::<Test>::Math
		);
		assert_noop!(
			StablePool::redeem_single(
				RuntimeOrigin::signed(6),
				0,
				5_000_000_000u128,
				0,
				6_000_000_000u128,
				2
			),
			bifrost_stable_asset::Error::<Test>::RedeemUnderMin
		);
		assert_noop!(
			StablePool::redeem_single(
				RuntimeOrigin::signed(6),
				0,
				100000000000000000u128,
				3,
				0u128,
				2
			),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_noop!(
			StablePool::redeem_single(
				RuntimeOrigin::signed(6),
				3,
				100000000000000000u128,
				3,
				0u128,
				2
			),
			bifrost_stable_asset::Error::<Test>::PoolNotFound
		);
		assert_eq!(Tokens::free_balance(pool_asset, &6), 204_955_833_377);
		assert_eq!(Tokens::free_balance(coin0, &6), 904_942_938_280);
		assert_eq!(Tokens::free_balance(coin1, &6), 900_000_000_000u128);
		assert_ok!(StablePool::redeem_single(
			RuntimeOrigin::signed(6).into(),
			0,
			5_000_000_000u128,
			1,
			0,
			2
		));
		assert_eq!(Tokens::free_balance(coin1, &6), 904_596_263_064);
		assert_noop!(
			StablePool::modify_fees(
				RuntimeOrigin::root(),
				0,
				Some(10_000_000_000),
				Some(10_000_000_000),
				Some(10_000_000_000)
			),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_ok!(StablePool::modify_fees(
			RuntimeOrigin::root(),
			0,
			Some(9_999_999_999),
			Some(9_999_999_999),
			Some(9_999_999_999),
		));
		assert_ok!(StablePool::redeem_single(
			RuntimeOrigin::signed(6).into(),
			0,
			5_000_000_000u128,
			1,
			0,
			2
		));
		assert_eq!(Tokens::free_balance(coin1, &6), 904_596_263_064);
		assert_ok!(StablePool::modify_fees(
			RuntimeOrigin::root(),
			0,
			Some(9_999_999_999),
			Some(9_999_999_999),
			Some(999_999_999),
		));
		assert_ok!(StablePool::redeem_single(
			RuntimeOrigin::signed(6).into(),
			0,
			5_000_000_000u128,
			1,
			0,
			2
		));
		assert_eq!(Tokens::free_balance(coin1, &6), 908_716_032_298);
	});
}

#[test]
fn redeem_multi() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let coin0 = DOT;
		let coin1 = VDOT;
		let pool_asset = CurrencyId::BLP(0);

		assert_ok!(<Test as crate::Config>::MultiCurrency::deposit(
			coin0.into(),
			&6,
			1_000_000_000_000u128
		));
		assert_ok!(<Test as crate::Config>::MultiCurrency::deposit(
			coin1.into(),
			&6,
			1_000_000_000_000u128
		));
		assert_eq!(Tokens::free_balance(coin0, &6), 1_000_000_000_000u128);

		let amounts = vec![100_000_000_000u128, 100_000_000_000u128];
		assert_ok!(StablePool::create_pool(
			RuntimeOrigin::root(),
			vec![coin0.into(), coin1.into()],
			vec![1u128.into(), 1u128.into()],
			0u128.into(),
			0u128.into(),
			0u128.into(),
			220u128.into(),
			5,
			5,
			1000000000000u128.into()
		));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(DOT, (1, 1)), (VDOT, (10, 11))]
		));

		assert_noop!(
			StablePool::add_liquidity(
				RuntimeOrigin::signed(6),
				0,
				amounts.clone(),
				2000000000000000000000000u128
			),
			Error::<Test>::MintUnderMin
		);
		let amounts2 = vec![10000000u128, 20000000u128, 20000000u128];
		assert_noop!(
			StablePool::add_liquidity(RuntimeOrigin::signed(1), 0, amounts2.clone(), 0),
			bifrost_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_noop!(
			StablePool::add_liquidity(RuntimeOrigin::signed(1), 3, amounts2, 0),
			bifrost_stable_asset::Error::<Test>::PoolNotFound
		);
		let amounts_has_zero = vec![0u128, 20000000u128];
		assert_noop!(
			StablePool::add_liquidity(RuntimeOrigin::signed(1), 0, amounts_has_zero, 0),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_ok!(StablePool::add_liquidity(RuntimeOrigin::signed(6).into(), 0, amounts, 0));
		assert_eq!(Tokens::free_balance(coin0, &6), 900_000_000_000u128);
		assert_eq!(Tokens::free_balance(coin1, &6), 900_000_000_000u128);
		assert_eq!(Tokens::free_balance(pool_asset, &6), 209_955_833_377);

		assert_noop!(
			StablePool::redeem_multi(
				RuntimeOrigin::signed(1),
				0,
				vec![200_000_000_000u128, 200_000_000_000u128],
				1100000000000000000u128,
			),
			bifrost_stable_asset::Error::<Test>::Math
		);
		assert_noop!(
			StablePool::redeem_multi(
				RuntimeOrigin::signed(1),
				0,
				vec![20_000_000_000u128, 20_000_000_000u128],
				1_000_000_000u128,
			),
			Error::<Test>::RedeemOverMax
		);
		assert_ok!(StablePool::redeem_multi(
			RuntimeOrigin::signed(6).into(),
			0,
			vec![5_000_000_000, 5_000_000_000],
			12_000_000_000u128,
		));
		assert_eq!(Tokens::free_balance(coin1, &6), 905_000_000_000);
		assert_eq!(Tokens::free_balance(coin0, &6), 905_000_000_000);
		assert_eq!(Tokens::free_balance(pool_asset, &6), 199_458_041_709);
	});
}

#[test]
fn bnc_add_liquidity_should_work() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let coin0 = BNC;
		let coin1 = VBNC;
		let pool_asset = CurrencyId::BLP(0);

		assert_ok!(<Test as crate::Config>::MultiCurrency::deposit(
			coin1.into(),
			&6,
			1_000_000_000_000u128
		));
		assert_eq!(Tokens::free_balance(coin1, &6), 1_000_000_000_000u128);
		assert_ok!(<Test as crate::Config>::MultiCurrency::deposit(
			coin1.into(),
			&6,
			1_000_000_000_000u128
		));
		assert_eq!(Tokens::free_balance(coin1, &6), 2_000_000_000_000u128);
		assert_eq!(Balances::free_balance(&6), 100_000_000_000_000);

		let amounts = vec![100_000_000_000u128, 100_000_000_000u128];
		assert_ok!(StablePool::create_pool(
			RuntimeOrigin::root(),
			vec![coin0.into(), coin1.into()],
			vec![1u128.into(), 1u128.into()],
			0u128.into(),
			0u128.into(),
			0u128.into(),
			220u128.into(),
			5,
			5,
			1000000000000u128.into()
		));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(BNC, (1, 1)), (VBNC, (10, 11))]
		));
		assert_eq!(Tokens::free_balance(pool_asset, &6), 0);
		assert_ok!(StablePool::add_liquidity(RuntimeOrigin::signed(6).into(), 0, amounts, 0));
		assert_eq!(Tokens::free_balance(pool_asset, &6), 209_955_833_377);
	});
}

#[test]
fn edit_token_rate() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_noop!(
			StablePool::edit_token_rate(
				RuntimeOrigin::root(),
				0,
				vec![(BNC, (1, 1)), (VBNC, (10, 11))]
			),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool2();
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(BNC, (1, 1)), (VBNC, (10, 11))]
		));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![(VBNC, (10, 11)), (BNC, (1, 1))]
		);

		assert_ok!(StablePool::edit_token_rate(RuntimeOrigin::root(), 0, vec![(VBNC, (10, 12))]));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![(VBNC, (10, 12)), (BNC, (1, 1))]
		);

		assert_ok!(StablePool::edit_token_rate(RuntimeOrigin::root(), 0, vec![]));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![]
		);
		assert_ok!(StablePool::edit_token_rate(RuntimeOrigin::root(), 0, vec![(VBNC, (10, 12))]));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![(VBNC, (10, 12))]
		);
	});
}

#[test]
fn redeem_movr() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let (coin0, coin1, pool_asset, _swap_id) = create_movr_pool();
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(coin0, (1, 1)), (coin1, (90_000_000, 100_000_000))]
		));
		let amounts = vec![million_unit(100_000), million_unit(200_000)];
		assert_ok!(StablePool::mint_inner(&0, 0, amounts, 0));
		assert_eq!(Tokens::free_balance(pool_asset, &0), 321765598211330627258732);
		assert_ok!(StablePool::redeem_proportion_inner(&0, 0, million_unit(300_000), vec![0, 0]));
		assert_eq!(Tokens::free_balance(pool_asset, &0), 21765598211330627258732);
		assert_eq!(Tokens::free_balance(coin0, &0), 992676625984156921892393);
		assert_eq!(Tokens::free_balance(coin1, &0), 985353251968313843784786);
	});
}

#[test]
fn config_vtoken_auto_refresh_should_work() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let (coin0, coin1, _pool_asset, _swap_id) = init();

		assert_ok!(StablePool::config_vtoken_auto_refresh(
			RuntimeOrigin::root(),
			VDOT,
			Permill::from_percent(10)
		));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		));
		assert_ok!(Currencies::deposit(VDOT, &3, 100));
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		);
		assert_ok!(<Test as crate::Config>::VtokenMinting::increase_token_pool(DOT, 1000));
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![(coin0, (1, 1)), (coin1, (100000100, 100001000))]
		);

		assert_eq!(Tokens::free_balance(DOT, &3), 75000000u128 - BALANCE_OFF);
	});
}

#[test]
fn over_the_hardcap_should_not_work() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let (coin0, coin1, _pool_asset, _swap_id) = init();

		assert_ok!(StablePool::config_vtoken_auto_refresh(
			RuntimeOrigin::root(),
			VDOT,
			Permill::from_percent(10)
		));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		));

		assert_ok!(<Test as crate::Config>::VtokenMinting::increase_token_pool(DOT, 20_000_000));
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		);

		assert_ok!(StablePool::config_vtoken_auto_refresh(
			RuntimeOrigin::root(),
			VDOT,
			Permill::from_percent(20)
		));
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![(coin0, (1, 1)), (coin1, (100000000, 120000000))]
		);

		assert_eq!(Tokens::free_balance(DOT, &3), 75000000u128 - BALANCE_OFF);
	});
}

#[test]
fn not_config_vtoken_auto_refresh_should_not_work() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let (coin0, coin1, _pool_asset, _swap_id) = init();

		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		));
		assert_ok!(<Test as crate::Config>::VtokenMinting::increase_token_pool(DOT, 1000));
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		);
		assert_eq!(Tokens::free_balance(DOT, &3), 80000000u128 - BALANCE_OFF);
	});
}

#[test]
fn config_vtoken_auto_refresh_should_not_work() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_noop!(
			StablePool::config_vtoken_auto_refresh(
				RuntimeOrigin::root(),
				DOT,
				Permill::from_percent(20)
			),
			bifrost_stable_asset::Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn remove_vtoken_auto_refresh_should_work() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let (coin0, coin1, _pool_asset, _swap_id) = init();
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::root(),
			0,
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		));
		assert_ok!(StablePool::config_vtoken_auto_refresh(
			RuntimeOrigin::root(),
			VDOT,
			Permill::from_percent(20)
		));
		assert_ok!(<Test as crate::Config>::VtokenMinting::increase_token_pool(DOT, 20_000_000));

		assert_ok!(StablePool::remove_vtoken_auto_refresh(RuntimeOrigin::root(), VDOT));
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		);

		assert_ok!(StablePool::config_vtoken_auto_refresh(
			RuntimeOrigin::root(),
			VDOT,
			Permill::from_percent(20)
		));
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(
			bifrost_stable_asset::TokenRateCaches::<Test>::iter_prefix(0).collect::<Vec<(
				AssetIdOf<Test>,
				(AtLeast64BitUnsignedOf<Test>, AtLeast64BitUnsignedOf<Test>),
			)>>(),
			vec![(coin0, (1, 1)), (coin1, (100000000, 120000000))]
		);
	});
}

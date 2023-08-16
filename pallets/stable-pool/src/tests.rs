// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, BoundedVec};
use nutsfinance_stable_asset::{StableAsset as StableAssetInterface, StableAssetPoolInfo};
use orml_traits::MultiCurrency;
use sp_runtime::traits::AccountIdConversion;

pub const BALANCE_OFF: u128 = 0;

fn create_pool() -> (CurrencyId, CurrencyId, CurrencyId, u128) {
	let coin0 = DOT;
	let coin1 = VDOT;
	let pool_asset: CurrencyId = CurrencyId::BLP(0);

	assert_ok!(StablePool::create_pool(
		RuntimeOrigin::signed(1),
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
		RuntimeOrigin::signed(1),
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

#[test]
fn modify_a_argument_error_failed() {
	env_logger::try_init().unwrap_or(());
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let (_coin0, _coin1, _pool_asset, _swap_id) = create_pool();

		assert_noop!(
			StableAsset::modify_a(RuntimeOrigin::signed(1), 0, 100, 0),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsError
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
			nutsfinance_stable_asset::Error::<Test>::ArgumentsError
		);
	});
}

#[test]
fn create_pool_successful() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		let coin0 = DOT;
		let coin1 = VDOT;
		assert_eq!(StableAsset::pool_count(), 0);
		assert_ok!(StablePool::create_pool(
			RuntimeOrigin::signed(1),
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
			StableAsset::pools(0),
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
		assert_ok!(VtokenMinting::mint(Some(3).into(), DOT, 100_000_000, BoundedVec::default()));
		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		System::set_block_number(2);
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::signed(1),
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
			nutsfinance_stable_asset::Error::<Test>::MintUnderMin
		);
		assert_ok!(StablePool::mint_inner(&3, 0, amounts, 0));
		// assert_ok!(StableAsset::mint(RuntimeOrigin::signed(3), 0, amounts.clone(), 0));
		assert_eq!(
			StableAsset::pools(0),
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
		let pool_account: u128 = StableAssetPalletId::get().into_account_truncating();
		let vtoken_issuance = <Test as crate::Config>::MultiCurrency::total_issuance(pool_asset);
		log::debug!("pool_account{:?}vtoken_issuance{:?}", pool_account, vtoken_issuance);
		// assert_eq!(
		// 	Tokens::free_balance(pool_asset, &pool_account),
		// 	199800000000000000u128 - BALANCE_OFF
		// );
		assert_eq!(Tokens::free_balance(pool_asset, &3), 199800000000000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(pool_asset, &2), 200000000000000u128 - BALANCE_OFF); // fee_recipient
	});
}

#[test]
fn swap_successful() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		assert_ok!(VtokenMinting::mint(Some(3).into(), DOT, 100_000_000, BoundedVec::default()));
		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10000000u128, 20000000u128];
		assert_ok!(StableAsset::mint(RuntimeOrigin::signed(3), 0, amounts, 0));
		assert_ok!(StableAsset::swap(RuntimeOrigin::signed(3), 0, 0, 1, 5000000u128, 0, 2));
		assert_eq!(
			StableAsset::pools(0),
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
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		assert_ok!(VtokenMinting::mint(Some(3).into(), DOT, 100_000_000, BoundedVec::default()));
		// assert_ok!(<Test as crate::Config>::MultiCurrency::transfer(VDOT, &BRUCE, &CATHI, 50));
		// assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), 3, VDOT, 90_000_000, 0));

		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::signed(1),
			0,
			vec![(coin0, (1, 1)), (coin1, (1, 1))]
		));
		let amounts = vec![10000000u128, 20000000u128];
		let vtoken_issuance = <Test as crate::Config>::MultiCurrency::total_issuance(pool_asset);
		log::debug!("vtoken_issuance{:?}", vtoken_issuance);
		// assert_ok!(StableAsset::mint(RuntimeOrigin::signed(3), 0, amounts, 0));
		assert_ok!(StablePool::mint_inner(&3, 0, amounts, 0));
		let vtoken_issuance2 = <Test as crate::Config>::MultiCurrency::total_issuance(pool_asset);
		log::debug!("vtoken_issuance2{:?}", vtoken_issuance2);
		let swap_out = StableAsset::get_swap_output_amount(0, 0, 1, 5000000u128);
		log::debug!("swap_out{:?}StableAsset::pools(0){:?}", swap_out, StableAsset::pools(0));
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 1, 1, 5000000u128, 0),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 3, 0, 1, 5000000u128, 0),
			nutsfinance_stable_asset::Error::<Test>::PoolNotFound
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 2, 1, 5000000u128, 0),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 0, 2, 5000000u128, 0),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 0, 1, 0u128, 0),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 50000000000000000u128),
			Error::<Test>::SwapUnderMin
		);
		assert_noop!(
			StablePool::on_swap(&3u128, 0, 0, 1, 500000000u128, 0u128),
			orml_tokens::Error::<Test>::BalanceTooLow
		);
		assert_eq!(
			StableAsset::pools(0),
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
fn mint_swap_redeem1() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		// assert_ok!(StableAsset::set_token_rate(0, vec![(VDOT, (90_000_000, 100_000_000))]));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::signed(1),
			0,
			vec![(DOT, (1, 1)), (VDOT, (90_000_000, 100_000_000))]
		));
		assert_ok!(VtokenMinting::mint(Some(3).into(), DOT, 100_000_000, BoundedVec::default()));
		// assert_ok!(<Test as crate::Config>::MultiCurrency::transfer(VDOT, &BRUCE, &CATHI, 50));
		assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), 3, VDOT, 90_000_000, 0));

		let (coin0, coin1, pool_asset, swap_id) = create_pool();
		System::set_block_number(2);

		let amounts = vec![10_000_000u128, 20_000_000u128];
		let vtoken_issuance = <Test as crate::Config>::MultiCurrency::total_issuance(pool_asset);
		log::debug!("vtoken_issuance{:?}", vtoken_issuance);
		// assert_ok!(StableAsset::mint(RuntimeOrigin::signed(3), 0, amounts, 0));
		assert_ok!(StablePool::mint_inner(&3, 0, amounts, 0));
		let vtoken_issuance2 = <Test as crate::Config>::MultiCurrency::total_issuance(pool_asset);
		log::debug!("vtoken_issuance2{:?}", vtoken_issuance2);
		let swap_out = StableAsset::get_swap_output_amount(0, 0, 1, 5000000u128);
		log::debug!("swap_out{:?}StableAsset::pools(0){:?}", swap_out, StableAsset::pools(0));
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		log::debug!("swap_out{:?}StableAsset::pools(0){:?}", swap_out, StableAsset::pools(0));
		// assert_ok!(StableAsset::swap(RuntimeOrigin::signed(3), 0, 0, 1, 5000000u128, 0,
		// 2));
		// assert_eq!(
		// 	StableAsset::pools(0),
		// 	Some(StableAssetPoolInfo {
		// 		pool_asset,
		// 		assets: vec![coin0, coin1],
		// 		precisions: vec![10000000000u128, 10000000000u128],
		// 		mint_fee: 10000000u128,
		// 		swap_fee: 20000000u128,
		// 		redeem_fee: 50000000u128,
		// 		total_supply: 300006989999594867u128,
		// 		a: 10000u128,
		// 		a_block: 0,
		// 		future_a: 10000u128,
		// 		future_a_block: 0,
		// 		balances: vec![150000000000000000u128, 150006990000000000u128],
		// 		fee_recipient: 2,
		// 		account_id: swap_id,
		// 		yield_recipient: 1,
		// 		precision: 1000000000000000000u128,
		// 	})
		// );
		assert_eq!(Tokens::free_balance(coin0, &3), 85000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &3), 74502743u128 - BALANCE_OFF); // 90_000_000 - 22_222_222 + 4_502_743
		assert_eq!(Tokens::free_balance(coin0, &swap_id), 15000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &swap_id), 15497257u128 - BALANCE_OFF);
		assert_ok!(StablePool::on_swap(&4u128, 0, 0, 1, 15_000_000u128, 0));
		log::debug!(
			"swap_out2{:?}StableAsset::pools(0){:?}==={:?}",
			swap_out,
			StableAsset::pools(0),
			Tokens::free_balance(coin1, &3)
		);
		assert_ok!(StablePool::on_swap(&1u128, 0, 0, 1, 500_000_000u128, 0));
		log::debug!(
			"swap_out3{:?}StableAsset::pools(0){:?}==={:?}pool_asset{:?}",
			swap_out,
			StableAsset::pools(0),
			Tokens::free_balance(coin1, &1),
			Tokens::free_balance(pool_asset, &3)
		);
		// log::debug!("===pools{:?}", StableAsset::pools(0));
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 0, 15_000_000u128, vec![0]),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_ok!(StablePool::redeem_proportion_inner(&3, 0, 15_000_000_000_000u128, vec![0, 0]));
		log::debug!(
			"swap_out4{:?}StableAsset::pools(0){:?}==={:?}pool_asset{:?}",
			swap_out,
			StableAsset::pools(0),
			Tokens::free_balance(coin1, &1),
			Tokens::free_balance(pool_asset, &3)
		);
	});
}

#[test]
fn mint_swap_redeem2() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		// assert_ok!(StableAsset::set_token_rate(0, vec![(VDOT, (90_000_000, 100_000_000))]));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::signed(1),
			0,
			vec![(DOT, (1, 1)), (VDOT, (90_000_000, 100_000_000))]
		));
		assert_ok!(VtokenMinting::mint(Some(3).into(), DOT, 100_000_000, BoundedVec::default()));
		assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), 3, VDOT, 90_000_000, 0));
		let (coin0, coin1, pool_asset, swap_id) = create_pool2();

		let amounts = vec![10_000_000u128, 20_000_000u128];
		assert_ok!(StablePool::mint_inner(&3, 0, amounts, 0));
		assert_eq!(Tokens::free_balance(coin1, &3), 70_000_000 - BALANCE_OFF); // 90_000_000 - 20_000_000
		log::debug!(
			"StableAsset::pools(0){:?}==={:?},{:?}+{:?},{:?}pool_asset{:?},{:?}",
			StableAsset::pools(0),
			Tokens::free_balance(coin0, &swap_id),
			Tokens::free_balance(coin1, &swap_id),
			Tokens::free_balance(coin0, &3),
			Tokens::free_balance(coin1, &3),
			Tokens::free_balance(pool_asset, &3),
			Tokens::free_balance(pool_asset, &2)
		);
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(Tokens::free_balance(coin0, &3), 85000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &3), 74502743u128 - BALANCE_OFF); // 90_000_000 - 20_000_000 + 4502743
		assert_eq!(Tokens::free_balance(coin0, &swap_id), 15000000u128 - BALANCE_OFF);
		assert_eq!(Tokens::free_balance(coin1, &swap_id), 15497257u128 - BALANCE_OFF);
		// assert_ok!(StablePool::on_swap(&4u128, 0, 0, 1, 15_000_000u128, 0));
		// assert_ok!(StablePool::on_swap(&1u128, 0, 0, 1, 500_000_000u128, 0));
		log::debug!(
			"StableAsset::pools(0){:?}==={:?},{:?}+{:?},{:?}pool_asset{:?},{:?}",
			StableAsset::pools(0),
			Tokens::free_balance(coin0, &swap_id),
			Tokens::free_balance(coin1, &swap_id),
			Tokens::free_balance(coin0, &3),
			Tokens::free_balance(coin1, &3),
			Tokens::free_balance(pool_asset, &3),
			Tokens::free_balance(pool_asset, &2)
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 0, 15_000_000u128, vec![0]),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 0, 0u128, vec![0u128, 0u128]),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsError
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 0, 15_000_000u128, vec![0, 0, 0]),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(
				&3,
				0,
				15_000_000u128,
				vec![100000000000000000u128, 0]
			),
			nutsfinance_stable_asset::Error::<Test>::RedeemUnderMin
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 3, 15_000_000u128, vec![0, 0]),
			nutsfinance_stable_asset::Error::<Test>::PoolNotFound
		);
		assert_ok!(StablePool::redeem_proportion_inner(&3, 0, 32176560, vec![0, 0]));
		// assert_ok!(StablePool::redeem_proportion_inner(&3, 0, 30_500_608, vec![0, 0]));

		let redeem_proportion_amount =
			StableAsset::get_redeem_proportion_amount(&StableAsset::pools(0).unwrap(), 32176560);
		log::debug!(
			"get_redeem_proportion_amount{:?}StableAsset::pools(0){:?}",
			redeem_proportion_amount,
			StableAsset::pools(0)
		);

		let vtoken_issuance2 = <Test as crate::Config>::MultiCurrency::total_issuance(pool_asset);
		log::debug!("vtoken_issuance2:{:?}", vtoken_issuance2);
		log::debug!(
			"StableAsset::pools(0){:?}==={:?},{:?}+{:?},{:?}pool_asset{:?},{:?}",
			StableAsset::pools(0),
			Tokens::free_balance(coin0, &swap_id),
			Tokens::free_balance(coin1, &swap_id),
			Tokens::free_balance(coin0, &3),
			Tokens::free_balance(coin1, &3),
			Tokens::free_balance(pool_asset, &3),
			Tokens::free_balance(pool_asset, &2)
		);
	});
}

#[test]
fn mint_swap_redeem_for_precisions() {
	ExtBuilder::default().new_test_ext().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(1), DOT, 0));
		assert_ok!(VtokenMinting::mint(Some(3).into(), DOT, 100_000_000, BoundedVec::default()));
		assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), 3, VDOT, 90_000_000, 0));
		let (coin0, coin1, pool_asset, swap_id) = create_pool2();
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::signed(1),
			0,
			vec![(DOT, (1, 1)), (VDOT, (90_000_000, 100_000_000))]
		));
		let amounts = vec![10_000_000u128, 20_000_000u128];
		assert_ok!(StablePool::mint_inner(&3, 0, amounts, 0));
		assert_eq!(Tokens::free_balance(coin1, &3), 70_000_000 - BALANCE_OFF); // 90_000_000 - 20_000_000
		log::debug!(
			"StableAsset::pools(0){:?}==={:?},{:?}+{:?},{:?}pool_asset{:?},{:?}",
			StableAsset::pools(0),
			Tokens::free_balance(coin0, &swap_id),
			Tokens::free_balance(coin1, &swap_id),
			Tokens::free_balance(coin0, &3),
			Tokens::free_balance(coin1, &3),
			Tokens::free_balance(pool_asset, &3),
			Tokens::free_balance(pool_asset, &2)
		);
		assert_ok!(StablePool::on_swap(&3u128, 0, 0, 1, 5000000u128, 0));
		assert_eq!(Tokens::free_balance(coin0, &3), 85000000u128 - BALANCE_OFF);
		log::debug!(
			"StableAsset::pools(0){:?}==={:?},{:?}+{:?},{:?}pool_asset{:?},{:?}",
			StableAsset::pools(0),
			Tokens::free_balance(coin0, &swap_id),
			Tokens::free_balance(coin1, &swap_id),
			Tokens::free_balance(coin0, &3),
			Tokens::free_balance(coin1, &3),
			Tokens::free_balance(pool_asset, &3),
			Tokens::free_balance(pool_asset, &2)
		);
		assert_noop!(
			StablePool::redeem_proportion_inner(&3, 0, 15_000_000u128, vec![0]),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_ok!(StablePool::redeem_proportion_inner(&3, 0, 32176560, vec![0, 0]));

		let redeem_proportion_amount =
			StableAsset::get_redeem_proportion_amount(&StableAsset::pools(0).unwrap(), 32176560);
		log::debug!(
			"get_redeem_proportion_amount{:?}StableAsset::pools(0){:?}",
			redeem_proportion_amount,
			StableAsset::pools(0)
		);

		let vtoken_issuance2 = <Test as crate::Config>::MultiCurrency::total_issuance(pool_asset);
		log::debug!("vtoken_issuance2:{:?}", vtoken_issuance2);
		log::debug!(
			"StableAsset::pools(0){:?}==={:?},{:?}+{:?},{:?}pool_asset{:?},{:?}",
			StableAsset::pools(0),
			Tokens::free_balance(coin0, &swap_id),
			Tokens::free_balance(coin1, &swap_id),
			Tokens::free_balance(coin0, &3),
			Tokens::free_balance(coin1, &3),
			Tokens::free_balance(pool_asset, &3),
			Tokens::free_balance(pool_asset, &2)
		);
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
			RuntimeOrigin::signed(1),
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
			RuntimeOrigin::signed(1),
			0,
			vec![(DOT, (1, 1)), (VDOT, (10, 11))]
		));
		assert_ok!(StablePool::add_liquidity(RuntimeOrigin::signed(6).into(), 0, amounts, 0));
		assert_eq!(Tokens::free_balance(coin0, &6), 900_000_000_000u128);
		assert_eq!(Tokens::free_balance(coin1, &6), 900_000_000_000u128);
		assert_eq!(Tokens::free_balance(pool_asset, &6), 209_955_833_377);

		let vtoken_issuance2 = <Test as crate::Config>::MultiCurrency::total_issuance(pool_asset);
		log::debug!(
			"vtoken_issuance2:
		{:?}",
			vtoken_issuance2
		);
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
			nutsfinance_stable_asset::Error::<Test>::ArgumentsError
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
			nutsfinance_stable_asset::Error::<Test>::Math
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
			nutsfinance_stable_asset::Error::<Test>::RedeemUnderMin
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
			nutsfinance_stable_asset::Error::<Test>::ArgumentsError
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
			nutsfinance_stable_asset::Error::<Test>::PoolNotFound
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
			RuntimeOrigin::signed(1),
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
			RuntimeOrigin::signed(1),
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
			nutsfinance_stable_asset::Error::<Test>::ArgumentsMismatch
		);
		assert_noop!(
			StablePool::add_liquidity(RuntimeOrigin::signed(1), 3, amounts2, 0),
			nutsfinance_stable_asset::Error::<Test>::PoolNotFound
		);
		let amounts_has_zero = vec![0u128, 20000000u128];
		assert_noop!(
			StablePool::add_liquidity(RuntimeOrigin::signed(1), 0, amounts_has_zero, 0),
			nutsfinance_stable_asset::Error::<Test>::ArgumentsError
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
			nutsfinance_stable_asset::Error::<Test>::Math
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

		// assert_ok!(<Test as crate::Config>::MultiCurrency::deposit(
		// 	coin0.into(),
		// 	&6,
		// 	1_000_000_000_000u128
		// ));
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
			RuntimeOrigin::signed(1),
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
			RuntimeOrigin::signed(1),
			0,
			vec![(BNC, (1, 1)), (VBNC, (10, 11))]
		));
		assert_eq!(Tokens::free_balance(pool_asset, &6), 0);
		assert_ok!(StablePool::add_liquidity(RuntimeOrigin::signed(6).into(), 0, amounts, 0));
		assert_eq!(Tokens::free_balance(pool_asset, &6), 209_955_833_377);
	});
}

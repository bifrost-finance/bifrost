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

#![cfg(test)]

use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError::BadOrigin;

use crate::{mock::*, *};

fn initialize_pool() {
	assert_ok!(LighteningRedeem::edit_release_start_and_end_block(
		pallet_collective::RawOrigin::Members(2, 3).into(),
		10,
		15000
	));
	assert_ok!(LighteningRedeem::edit_release_per_day(
		pallet_collective::RawOrigin::Members(2, 3).into(),
		BalanceOf::<Runtime>::unique_saturated_from(50)
	));
	assert_ok!(LighteningRedeem::add_ksm_to_pool(RuntimeOrigin::signed(ALICE), 100));
}

#[test]
fn edit_release_start_and_end_block_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		// Charlie doesn't have the permission to edit.
		assert_noop!(
			LighteningRedeem::edit_release_start_and_end_block(
				RuntimeOrigin::signed(CHARLIE),
				10,
				15000
			),
			BadOrigin
		);

		assert_ok!(LighteningRedeem::edit_release_start_and_end_block(
			pallet_collective::RawOrigin::Members(2, 3).into(),
			10,
			15000
		));

		let (start, end) = LighteningRedeem::get_start_and_end_release_block();
		assert_eq!(start, 10);
		assert_eq!(end, 15000);
	});
}

#[test]
fn edit_exchange_price_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let original_price = LighteningRedeem::get_exchange_price_discount()
			.mul_floor(BalanceOf::<Runtime>::unique_saturated_from(100u128));
		assert_eq!(original_price, 90);

		// Charlie doesn't have the permission to edit.
		assert_noop!(
			LighteningRedeem::edit_exchange_price(RuntimeOrigin::signed(CHARLIE), 80),
			BadOrigin
		);

		assert_ok!(LighteningRedeem::edit_exchange_price(
			pallet_collective::RawOrigin::Members(2, 3).into(),
			80
		));

		let current_price = LighteningRedeem::get_exchange_price_discount()
			.mul_floor(BalanceOf::<Runtime>::unique_saturated_from(100u128));
		assert_eq!(current_price, 80);
	});
}

#[test]
fn edit_release_per_day_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		let originla_amount_per_day = LighteningRedeem::get_token_release_per_round();
		assert_eq!(
			originla_amount_per_day,
			BalanceOf::<Runtime>::unique_saturated_from(30 * TRILLION)
		);

		// Charlie doesn't have the permission to edit.
		assert_noop!(
			LighteningRedeem::edit_release_per_day(
				RuntimeOrigin::signed(CHARLIE),
				BalanceOf::<Runtime>::unique_saturated_from(50 * TRILLION)
			),
			BadOrigin
		);

		assert_ok!(LighteningRedeem::edit_release_per_day(
			pallet_collective::RawOrigin::Members(2, 3).into(),
			BalanceOf::<Runtime>::unique_saturated_from(50 * TRILLION)
		));

		let current_amount_per_day = LighteningRedeem::get_token_release_per_round();
		assert_eq!(
			current_amount_per_day,
			BalanceOf::<Runtime>::unique_saturated_from(50 * TRILLION)
		);
	});
}

#[test]
fn add_ksm_to_pool_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		// Charlie doesn't have KSM.
		assert_noop!(
			LighteningRedeem::add_ksm_to_pool(RuntimeOrigin::signed(CHARLIE), 80),
			Error::<Runtime>::NotEnoughBalance
		);

		// Charlie succuessfully issue 800 unit of ZLK to Alice account
		assert_ok!(LighteningRedeem::add_ksm_to_pool(RuntimeOrigin::signed(ALICE), 80));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 20);

		let pool_account = <Runtime as crate::Config>::PalletId::get().into_account_truncating();
		assert_eq!(Tokens::free_balance(KSM, &pool_account), 80);
	});
}

#[test]
fn exchange_for_ksm_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		run_to_block(2);
		initialize_pool();
		run_to_block(3);

		// In block 9, the pool still dosn't have any KSM which can be redeemed.
		run_to_block(20);
		assert_noop!(
			LighteningRedeem::exchange_for_ksm(RuntimeOrigin::signed(CHARLIE), 90),
			Error::<Runtime>::ExceedPoolAmount
		);

		run_to_block(7300);
		assert_eq!(LighteningRedeem::get_pool_amount(), 50);
		// Charlie doesn't have vsKSM and vsBond.
		assert_noop!(
			LighteningRedeem::exchange_for_ksm(RuntimeOrigin::signed(CHARLIE), 30),
			Error::<Runtime>::NotEnoughBalance
		);

		let pool_account = <Runtime as crate::Config>::PalletId::get().into_account_truncating();

		// Before doing exchange
		assert_eq!(Tokens::free_balance(vsKSM, &BOB), 100);
		assert_eq!(Tokens::free_balance(vsBond, &BOB), 100);
		assert_eq!(Tokens::free_balance(KSM, &BOB), 0);

		assert_eq!(Tokens::free_balance(KSM, &pool_account), 100);
		assert_eq!(Tokens::free_balance(vsKSM, &pool_account), 0);
		assert_eq!(Tokens::free_balance(vsBond, &pool_account), 0);

		run_to_block(14900);
		assert_eq!(LighteningRedeem::get_pool_amount(), 100);

		// perform the exchange
		assert_ok!(LighteningRedeem::exchange_for_ksm(RuntimeOrigin::signed(BOB), 90));
		assert_eq!(Tokens::free_balance(vsKSM, &BOB), 0);
		assert_eq!(Tokens::free_balance(vsBond, &BOB), 0);
		assert_eq!(Tokens::free_balance(KSM, &BOB), 90);

		assert_eq!(Tokens::free_balance(KSM, &pool_account), 10);
		assert_eq!(Tokens::free_balance(vsKSM, &pool_account), 100);
		assert_eq!(Tokens::free_balance(vsBond, &pool_account), 100);

		assert_eq!(LighteningRedeem::get_pool_amount(), 10);
	});
}

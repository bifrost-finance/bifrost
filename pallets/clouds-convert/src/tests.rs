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

// Ensure we're `no_std` when compiling for Wasm.

#![cfg(test)]

use crate::{mock::*, *};
use bifrost_primitives::currency::{CLOUD, VBNC};
use frame_support::{assert_noop, assert_ok};

#[test]
fn clouds_to_vebnc_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		// Bob convert 100 clouds to vebnc
		assert_noop!(
			CloudsConvert::clouds_to_vebnc(RuntimeOrigin::signed(BOB), 100, 0),
			Error::<Runtime>::NotEnoughBalance
		);

		// deposit 500 clouds to Bob
		assert_ok!(Tokens::deposit(CLOUD, &BOB, 500));

		// expect too much vBNC
		assert_noop!(
			CloudsConvert::clouds_to_vebnc(RuntimeOrigin::signed(BOB), 100, 100000),
			Error::<Runtime>::LessThanExpected
		);
		// convert too little clouds
		assert_noop!(
			CloudsConvert::clouds_to_vebnc(RuntimeOrigin::signed(BOB), 1, 0),
			Error::<Runtime>::LessThanExistentialDeposit
		);

		// pool does not have enough vBNC
		assert_noop!(
			CloudsConvert::clouds_to_vebnc(RuntimeOrigin::signed(BOB), 100, 1),
			Error::<Runtime>::LessThanExpected
		);
		// deposit some vBNC to Pool
		assert_ok!(Tokens::deposit(VBNC, &CloudsConvert::clouds_pool_account(), 100000000000));

		// check the veBNC balance of Bob
		let bob_old_vebnc_balance =
			<Runtime as crate::Config>::BbBNC::balance_of(&BOB, None).unwrap();

		// check the old pool balance
		let old_pool_balance = <Runtime as crate::Config>::MultiCurrency::free_balance(
			VBNC,
			&CloudsConvert::clouds_pool_account(),
		);

		// Bob convert 100 clouds to vebnc
		assert_ok!(CloudsConvert::clouds_to_vebnc(RuntimeOrigin::signed(BOB), 100, 1));

		// check the veBNC balance of Bob
		assert_eq!(
			<Runtime as crate::Config>::BbBNC::balance_of(&BOB, None).unwrap(),
			bob_old_vebnc_balance + 20034907200
		);

		// check the new pool balance
		assert_eq!(
			<Runtime as crate::Config>::MultiCurrency::free_balance(
				VBNC,
				&CloudsConvert::clouds_pool_account()
			),
			old_pool_balance - 20000000000
		);
	});
}

#[test]
fn charge_vbnc_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		// check the vbnc balance of the pool
		let old_pool_balance = <Runtime as crate::Config>::MultiCurrency::free_balance(
			VBNC,
			&CloudsConvert::clouds_pool_account(),
		);

		// Bob charge 100 vbnc to the pool
		assert_noop!(
			CloudsConvert::charge_vbnc(RuntimeOrigin::signed(BOB), 100),
			Error::<Runtime>::NotEnoughBalance
		);

		// deposit 100 vbnc to Bob
		assert_ok!(Tokens::deposit(VBNC, &BOB, 500));

		// Bob charge 100 vbnc to the pool
		assert_ok!(CloudsConvert::charge_vbnc(RuntimeOrigin::signed(BOB), 100));

		// check the new pool balance
		assert_eq!(
			<Runtime as crate::Config>::MultiCurrency::free_balance(
				VBNC,
				&CloudsConvert::clouds_pool_account()
			),
			old_pool_balance + 100
		);
	});
}

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
use bifrost_primitives::{
	currency::{VBNC, VBNC_P},
	BNC,
};
use frame_support::{assert_noop, assert_ok};

#[test]
fn convert_to_vbnc_p_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tokens::deposit(VBNC, &BOB, 5000));
		assert_ok!(Tokens::deposit(VBNC_P, &VBNCConvert::vbnc_p_pool_account(), 5000));

		assert_ok!(VBNCConvert::convert_to_vbnc_p(RuntimeOrigin::signed(BOB), VBNC, 1000));
		System::assert_last_event(RuntimeEvent::VBNCConvert(Event::VBNCPConverted {
			to: BOB,
			value: 1000,
		}));

		assert_eq!(<Runtime as crate::Config>::MultiCurrency::free_balance(VBNC, &BOB,), 4000);
		assert_eq!(<Runtime as crate::Config>::MultiCurrency::free_balance(VBNC_P, &BOB,), 1000);
		assert_eq!(
			<Runtime as crate::Config>::MultiCurrency::free_balance(
				VBNC_P,
				&VBNCConvert::vbnc_p_pool_account()
			),
			4000
		);
	});
}

#[test]
fn convert_to_vbnc_p_should_fail_with_wrong_currency() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tokens::deposit(BNC, &BOB, 5000));
		assert_ok!(Tokens::deposit(VBNC_P, &BOB, 5000));

		assert_noop!(
			VBNCConvert::convert_to_vbnc_p(RuntimeOrigin::signed(BOB), BNC, 1000),
			Error::<Runtime>::CurrencyNotSupport
		);
		assert_noop!(
			VBNCConvert::convert_to_vbnc_p(RuntimeOrigin::signed(BOB), VBNC_P, 1000),
			Error::<Runtime>::CurrencyNotSupport
		);
	});
}

#[test]
fn convert_to_vbnc_p_should_fail_with_account_balance_poor() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			VBNCConvert::convert_to_vbnc_p(RuntimeOrigin::signed(BOB), VBNC, 100),
			Error::<Runtime>::NotEnoughBalance
		);
	});
}

#[test]
fn convert_to_vbnc_p_should_fail_with_pool_balance_poor() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tokens::deposit(VBNC, &BOB, 500));

		assert_noop!(
			VBNCConvert::convert_to_vbnc_p(RuntimeOrigin::signed(BOB), VBNC, 100),
			Error::<Runtime>::NotEnoughBalance
		);
	});
}

#[test]
fn convert_to_vbnc_p_should_fail_with_less_than_existential_depositr() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tokens::deposit(VBNC, &BOB, 500));
		assert_ok!(Tokens::deposit(VBNC_P, &VBNCConvert::vbnc_p_pool_account(), 500));

		assert_noop!(
			VBNCConvert::convert_to_vbnc_p(RuntimeOrigin::signed(BOB), VBNC, 1),
			Error::<Runtime>::LessThanExistentialDeposit
		);
	});
}

#[test]
fn charge_vbnc_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Tokens::deposit(VBNC_P, &BOB, 500));

		assert_ok!(VBNCConvert::charge_vbnc_p(RuntimeOrigin::signed(BOB), 100));
		System::assert_last_event(RuntimeEvent::VBNCConvert(Event::VbncPCharged {
			who: BOB,
			value: 100,
		}));

		assert_eq!(<Runtime as crate::Config>::MultiCurrency::free_balance(VBNC_P, &BOB), 400);
		assert_eq!(
			<Runtime as crate::Config>::MultiCurrency::free_balance(
				VBNC_P,
				&VBNCConvert::vbnc_p_pool_account()
			),
			100
		);
	});
}

#[test]
fn charge_vbnc_should_fail_with_account_balance_poor() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			VBNCConvert::charge_vbnc_p(RuntimeOrigin::signed(BOB), 100),
			Error::<Runtime>::NotEnoughBalance
		);
	});
}

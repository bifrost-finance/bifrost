// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};

use crate as salp;
use crate::mock::*;

#[test]
fn create_fund_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 0, SlotLength::get() - 1,));
		assert_ok!(Salp::funds(3_000).ok_or(()));
		assert_eq!(Salp::current_trie_index(), 1);
	});
}

#[test]
fn create_fund_under_non_signed_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Salp::create(Origin::root(), 3_000, 1_000, 0, SlotLength::get() - 1,),
			DispatchError::BadOrigin,
		);

		assert_noop!(
			Salp::create(Origin::none(), 3_000, 1_000, 0, SlotLength::get() - 1,),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn create_fund_existed_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 0, SlotLength::get() - 1,),);

		assert_noop!(
			Salp::create(Some(ALICE).into(), 3_000, 1_000, 0, SlotLength::get() - 1,),
			salp::Error::<Test>::FundExisted,
		);
	});
}

#[test]
fn create_fund_exceed_slot_limit_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Salp::create(Some(ALICE).into(), 3_000, 1_000, 0, SlotLength::get()),
			salp::Error::<Test>::LastSlotTooFarInFuture,
		);
	});
}

#[test]
fn create_fund_first_slot_bigger_than_last_slot_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Salp::create(Some(ALICE).into(), 3_000, 1_000, SlotLength::get() - 1, 0),
			salp::Error::<Test>::LastSlotBeforeFirstSlot,
		);
	});
}

#[test]
fn contribute_should_work() {}

#[test]
fn contribute_under_non_signed_should_fail() {}

#[test]
fn contribute_to_non_exist_parachain_should_fail() {}

#[test]
// Utilities Test
#[test]
fn check_next_trie_index() {
	new_test_ext().execute_with(|| {
		for i in 0 .. 100 {
			assert_eq!(Salp::current_trie_index(), i);
			assert_ok!(Salp::next_trie_index());
		}
	});
}

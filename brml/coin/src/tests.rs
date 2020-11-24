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
use node_primitives::CoinTrait;
use frame_support::{assert_noop, assert_ok};

fn common() {
	assert_ok!(crate::Module::<Test>::calculate_bnc(60));
	assert_eq!(60, BncAmount::<Test>::get());
}

fn coin() {
	let (alice, a_count) = (11111111 as u64, 100);
	let (bob, b_count) = (22222222 as u64, 100);
	assert_ok!(crate::Module::<Test>::coin_bnc(alice,a_count));
	assert_ok!(crate::Module::<Test>::coin_bnc(bob,b_count));
	assert_ok!(crate::Module::<Test>::coin_bnc(bob,b_count));
	assert_eq!(100, BncCoin::<Test>::get(&alice));
	assert_eq!(200, BncCoin::<Test>::get(&bob));
}

#[test]
fn record_integral_should_be_ok() {
	new_test_ext().execute_with(|| {
		common();
	});
}

#[test]
fn coin_bnc_should_be_ok() {
	new_test_ext().execute_with(|| {
		coin();
	});
}

#[test]
fn issue_bnc_should_be_ok() {
	new_test_ext().execute_with(|| {
		let (alice, bob) = (11111111, 22222222);
		assert_noop!(
			crate::Module::<Test>::issue_bnc(),
			Error::<Test>::CoinerNotExist
		);
		common();
		coin();
		assert_ok!(crate::Module::<Test>::issue_bnc());
		assert_eq!(20, BncReward::<Test>::get(alice));
		assert_eq!(40, BncReward::<Test>::get(bob));
		// clear
		assert_eq!(0, BncAmount::<Test>::get());
		assert_eq!(0, BncCoin::<Test>::get(alice));
		assert_eq!(0, BncCoin::<Test>::get(bob));
		assert_eq!(0, BncCoin::<Test>::get(bob));
	});
}

#[test]
fn query_bnc_should_be_ok() {
	new_test_ext().execute_with(|| {
		let (alice, bob) = (11111111, 22222222);
		common();
		coin();
		
		assert_ok!(crate::Module::<Test>::query_bnc(alice));
		assert_ok!(Result::<u64, Test>::Ok(100), crate::Module::<Test>::query_bnc(alice).unwrap());
		
		assert_ok!(crate::Module::<Test>::query_bnc(bob));
		assert_ok!(Result::<u64, Test>::Ok(200), crate::Module::<Test>::query_bnc(bob).unwrap());
	});
}
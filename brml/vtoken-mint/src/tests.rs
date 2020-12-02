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
use node_primitives::MintTrait;
use frame_support::{assert_ok, traits::OnFinalize};

fn generate() {
	crate::Module::<Test>::count_bnc(10);
	crate::Module::<Test>::count_bnc(20);
	crate::Module::<Test>::count_bnc(30);
	
	assert_eq!(60, BncSum::<Test>::get());
}

fn mint() {
	let (alice, a_count) = (11111111 as u64, 100);
	let (bob, b_count) = (22222222 as u64, 100);
	assert_eq!(0, BncMint::<Test>::get(&alice));
	assert_eq!(0, BncMint::<Test>::get(&bob));
	
	let (_, max_bnc_amount) = BncMonitor::<Test>::get();
	assert_eq!(0, max_bnc_amount);
	
	assert_ok!(crate::Module::<Test>::mint_bnc(alice, a_count));
	let (_, max_bnc_amount) = BncMonitor::<Test>::get();
	assert_eq!(100, max_bnc_amount);
	
	assert_ok!(crate::Module::<Test>::mint_bnc(bob, b_count - 50));
	let (_, max_bnc_amount) = BncMonitor::<Test>::get();
	assert_eq!(100, max_bnc_amount);
	
	assert_ok!(crate::Module::<Test>::mint_bnc(bob, b_count + 50));
	let (_, max_bnc_amount) = BncMonitor::<Test>::get();
	assert_eq!(150, max_bnc_amount);
	
	assert_eq!(100, BncMint::<Test>::get(&alice));
	assert_eq!(200, BncMint::<Test>::get(&bob));
}

#[test]
fn on_finalize_should_ok() {
	new_test_ext().execute_with(|| {
		// initial genesis block_number and bnc_price
		BncPrice::<Test>::put((0, 200));
		crate::Module::<Test>::on_finalize(INTERVAL.into());
		assert_eq!(100, BncPrice::<Test>::get().1);
		// it doesn't mint and it doesn't issue
		let ((block_numer, bnc_amount), max_bnc_amount) = BncMonitor::<Test>::get();
		assert_eq!(0, block_numer);
		assert_eq!(0, bnc_amount);
		assert_eq!(0, max_bnc_amount);
		
		// mint but no issue
		mint();
		crate::Module::<Test>::on_finalize(INTERVAL.into());
		let ((block_numer, bnc_amount), max_bnc_amount) = BncMonitor::<Test>::get();
		assert_eq!(10519200, block_numer);
		assert_eq!(150, bnc_amount);
		assert_eq!(150, max_bnc_amount);
		
		// issue
		crate::Module::<Test>::on_finalize((INTERVAL + 50).into());
		let ((block_numer, bnc_amount), max_bnc_amount) = BncMonitor::<Test>::get();
		assert_eq!(0, block_numer);
		assert_eq!(0, bnc_amount);
		assert_eq!(0, max_bnc_amount);
	});
}

#[test]
fn count_bnc_should_be_ok() {
	new_test_ext().execute_with(|| {
		generate();
	});
}

#[test]
fn mint_bnc_should_be_ok() {
	new_test_ext().execute_with(|| {
		mint();
	});
}

#[test]
fn issue_bnc_should_be_ok() {
	new_test_ext().execute_with(|| {
		let (alice, bob) = (11111111, 22222222);
		// generate 60 BNC
		generate();
		assert_eq!(60, BncSum::<Test>::get());
		// alice:200 point  /  bob:100 point
		mint();
		let (_, max_bnc_amount) = BncMonitor::<Test>::get();
		assert_eq!(150, max_bnc_amount);
		// issue bnc
		assert_ok!(crate::Module::<Test>::issue_bnc());
		assert_eq!(20, BncReward::<Test>::get(alice));
		assert_eq!(40, BncReward::<Test>::get(bob));
		
		assert_eq!(0, BncSum::<Test>::get());
		assert_eq!(0, BncMint::<Test>::get(alice));
		assert_eq!(0, BncMint::<Test>::get(bob));
		assert_eq!(0, BncMint::<Test>::get(bob));
		
		let ((block_numer, bnc_amount), max_bnc_amount) = BncMonitor::<Test>::get();
		assert_eq!(0, block_numer);
		assert_eq!(0, bnc_amount);
		assert_eq!(0, max_bnc_amount);
	});
}

#[test]
fn query_bnc_should_be_ok() {
	new_test_ext().execute_with(|| {
		let (alice, bob) = (11111111, 22222222);
		generate();
		mint();
		
		assert_ok!(crate::Module::<Test>::query_bnc(alice));
		assert_ok!(Result::<u64, Test>::Ok(100), crate::Module::<Test>::query_bnc(alice).unwrap());
		
		assert_ok!(crate::Module::<Test>::query_bnc(bob));
		assert_ok!(Result::<u64, Test>::Ok(200), crate::Module::<Test>::query_bnc(bob).unwrap());
	});
}
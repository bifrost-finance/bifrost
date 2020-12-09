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
		let (
			alice,
			bob
		) = (11111111, 22222222);
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

		let ((block_number, bnc_amount), max_bnc_amount) = BncMonitor::<Test>::get();
		assert_eq!(0, block_number);
		assert_eq!(0, bnc_amount);
		assert_eq!(0, max_bnc_amount);
	});
}

// Currency weight model issue
#[test]
fn init_v_token_score_should_be_ok() {
	new_test_ext().execute_with(|| {
		let (asset_id, score) = (1, 10);
		assert_eq!(false, VtokenWeightScore::<Test>::contains_key(&asset_id));

		// Initial v_token score
		crate::Module::<Test>::init_v_token_score(asset_id, score);
		assert_eq!(10, VtokenWeightScore::<Test>::get(asset_id));
	});
}

#[test]
fn on_finalize_should_ok() {
	new_test_ext().execute_with(|| {
		// initial genesis block_number and bnc_price
		BncPrice::<Test>::put((0, 60));
		// bnc total reward amount : 30
		crate::Module::<Test>::on_finalize(INTERVAL.into());
		assert_eq!(30, BncPrice::<Test>::get().1);
		// it doesn't mint and it doesn't issue
		let ((block_number, bnc_amount), max_bnc_amount) = BncMonitor::<Test>::get();
		assert_eq!(0, block_number);
		assert_eq!(0, bnc_amount);
		assert_eq!(0, max_bnc_amount);

		// mint but no issue
		let (
			alice,
			bob,
			mint_amount,
			asset_id1,
			asset_id2,
			score,
			pledge_amount
		) = (11111111, 22222222, 100, 1, 2, 10, 1536);

		/* Initial v_token score and mint :
				alice : asset_id1 -> point  100     weight: 10
						asset_id2 -> point  300     weight: 10
				 bob :  asset_id1 -> point  200
		 */
		weight_mint(alice, bob, mint_amount, asset_id1, asset_id2, score);

		// bnc total reward amount : 60
		crate::Module::<Test>::on_finalize(INTERVAL.into());
		let ((block_numer, bnc_amount), max_bnc_amount) = BncMonitor::<Test>::get();
		assert_eq!(10519200, block_numer);
		assert_eq!(300, bnc_amount);
		assert_eq!(300, max_bnc_amount);

		/* adjust weight: asset_id1 10 -> asset_id1 20
				alice : asset_id1 -> point  100     weight: 20
						asset_id2 -> point  300     weight: 10
				 bob :  asset_id1 -> point  200
		 */
		assert_ok!(crate::Module::<Test>::adjust_v_token_weight(asset_id1, pledge_amount));

		/* issue 90 bnc (total reward amount):
			asset_id1 -> 60 bnc ->  (alice: 20, bob: 40)
			asset_id2 -> 30 bnc -> (alice: 30)
		 */
		crate::Module::<Test>::on_finalize((INTERVAL + 50).into());
		assert_eq!(50, BncReward::<Test>::get(alice));
		assert_eq!(40, BncReward::<Test>::get(bob));

		let ((block_numer, bnc_amount), max_bnc_amount) = BncMonitor::<Test>::get();
		assert_eq!(0, block_numer);
		assert_eq!(0, bnc_amount);
		assert_eq!(0, max_bnc_amount);
	});
}

#[test]
fn mint_bnc_by_weight_should_be_ok() {
	new_test_ext().execute_with(|| {
		let (
			alice,
			bob,
			mint_amount,
			asset_id1,
			asset_id2,
			score
		) = (11111111, 22222222, 100, 1, 2, 10);
		weight_mint(alice,bob, mint_amount, asset_id1, asset_id2, score);
	});
}

#[test]
fn issue_bnc_by_weight_should_be_ok() {
	new_test_ext().execute_with(|| {
		assert!(crate::Module::<Test>::issue_bnc_by_weight().is_err());
		//  60 BNC
		crate::Module::<Test>::count_bnc(60);
		let (
			alice,
			bob,
			mint_amount,
			asset_id1,
			asset_id2,
			score
		) = (11111111, 22222222, 100, 1, 2, 10);
		/* Initial v_token score and mint :
				alice : asset_id1 -> point  100     weight: 10
						asset_id2 -> point  300
				bob : asset_id1 -> 200              weight: 10
		 */
		weight_mint(alice, bob, mint_amount, asset_id1, asset_id2, score);

		// issue bnc
		assert_ok!(crate::Module::<Test>::issue_bnc_by_weight());
		assert_eq!(40, BncReward::<Test>::get(alice));
		assert_eq!(20, BncReward::<Test>::get(bob));

		assert_eq!(0, BncSum::<Test>::get());
		assert_eq!(0, BncMint::<Test>::get(alice));
		assert_eq!(0, BncMint::<Test>::get(bob));

		let ((block_number, bnc_amount), max_bnc_amount) = BncMonitor::<Test>::get();
		assert_eq!(0, block_number);
		assert_eq!(0, bnc_amount);
		assert_eq!(0, max_bnc_amount);
	});
}

#[test]
fn adjust_v_token_weight_should_be_ok() {
	new_test_ext().execute_with(|| {
		let (asset_id, pledge_amount) = (1, 512);
		assert_eq!(0, VtokenWeightScore::<Test>::get(asset_id));
		assert!(crate::Module::<Test>::adjust_v_token_weight(asset_id, pledge_amount).is_err());

		let pledge_amount = 518;
		assert_ok!(crate::Module::<Test>::adjust_v_token_weight(asset_id, pledge_amount));
		assert_eq!(2, VtokenWeightScore::<Test>::get(asset_id));

		assert_ok!(crate::Module::<Test>::adjust_v_token_weight(asset_id, pledge_amount));
		assert_eq!(4, VtokenWeightScore::<Test>::get(asset_id));

	});
}

fn weight_mint(
	alice: u64,
	bob: u64,
	mint_amount: u64,
	asset_id1: u32,
	asset_id2: u32,
	score: u64
) {
	assert!(crate::Module::<Test>::mint_bnc_by_weight(alice, mint_amount, asset_id1).is_err());

	// Initial v_token score and mint
	crate::Module::<Test>::init_v_token_score(asset_id1, score);
	assert_ok!(crate::Module::<Test>::mint_bnc_by_weight(alice, mint_amount, asset_id1));
	assert_eq!(100, VtokenWeightMint::<Test>::get(asset_id1, alice));

	assert_ok!(crate::Module::<Test>::mint_bnc_by_weight(bob, mint_amount - 50, asset_id1));
	assert_eq!(100, BncMonitor::<Test>::get().1);

	assert_ok!(crate::Module::<Test>::mint_bnc_by_weight(bob, mint_amount + 50, asset_id1));
	assert_eq!(150, BncMonitor::<Test>::get().1);

	crate::Module::<Test>::init_v_token_score(asset_id2, score);
	assert_ok!(crate::Module::<Test>::mint_bnc_by_weight(alice, mint_amount + 200, asset_id2));
	assert_eq!(300, BncMonitor::<Test>::get().1);
}
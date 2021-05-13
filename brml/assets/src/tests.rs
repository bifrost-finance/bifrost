// Copyright 2019-2021 Liebi Technologies.
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

use super::*;
use crate::mock::*;
use crate::mock::Event;
use frame_support::{assert_ok, assert_noop};
use system::{EventRecord, Phase};

#[test]
fn create_asset_should_work() {
	new_test_ext().execute_with(|| {
		let token1 = Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
			token_type: TokenType::Native,
			pair: None,
		};

		let id1 = Assets::next_asset_id();

		System::set_block_number(1);

		assert_ok!(Assets::create(Origin::root(), vec![0x12, 0x34], 8, TokenType::Native));
		assert_eq!(Assets::next_asset_id(), id1 + 1);
		assert_eq!(Assets::token_details(id1), token1);

		let token2 = Token {
			symbol: vec![0x56, 0x68, 0x90],
			precision: 4,
			total_supply: 0,
			token_type: TokenType::Native,
			pair: None,
		};

		let id2 = Assets::next_asset_id();
		assert_ok!(Assets::create(Origin::root(), vec![0x56, 0x68, 0x90], 4, TokenType::Native)); // take 2 as asset id
		assert_eq!(Assets::next_asset_id(), id2 + 1);
		assert_eq!(Assets::token_details(id2), token2);

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::Initialization,
				event: Event::assets(RawEvent::Created(id1, token1)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::Initialization,
				event: Event::assets(RawEvent::Created(id2, token2)),
				topics: vec![],
			}
		]);

	});

		
}

#[test]
fn issuing_asset_units_to_issuer_should_work() {
	new_test_ext().execute_with(|| {
		let alice = 1;

		let ausd_id = Assets::next_asset_id();
		assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18, TokenType::Stable));

		let dot_id = Assets::next_asset_id();
		assert_ok!(Assets::create(Origin::root(), b"DOT".to_vec(), 12, TokenType::Token));

		let vdot_id = Assets::next_asset_id();
		assert_ok!(Assets::create(Origin::root(), b"vDOT".to_vec(), 12, TokenType::VToken));

		System::set_block_number(1);

		assert_ok!(Assets::issue(Origin::root(), dot_id, alice, 10000));
		assert_eq!(Assets::token_details(dot_id), Token::new(b"DOT".to_vec(), 12, 10000, TokenType::Token));
		assert_eq!(Assets::account_assets((dot_id, alice)).balance, 10000);

		assert_ok!(Assets::issue(Origin::root(), vdot_id, alice, 20000));
		assert_eq!(Assets::token_details(vdot_id), Token::new(b"vDOT".to_vec(), 12, 20000, TokenType::VToken));
		assert_eq!(Assets::account_assets((vdot_id, alice)).balance, 20000);

		let bob = 2;
		// issue bob balances twice
		assert_ok!(Assets::issue(Origin::root(), ausd_id, bob, 20000));
		assert_ok!(Assets::issue(Origin::root(), ausd_id, bob, 30000));
		assert_eq!(Assets::token_details(ausd_id), Token::new(b"aUSD".to_vec(), 18, 50000, TokenType::Stable));
		assert_eq!(Assets::account_assets((ausd_id, bob)).balance, 50000);

		assert_eq!(System::events(), vec![
			EventRecord { phase: Phase::Initialization, event: Event::assets(RawEvent::Issued(dot_id, 1, 10000)), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: Event::assets(RawEvent::Issued(vdot_id, 1, 20000)), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: Event::assets(RawEvent::Issued(ausd_id, 2, 20000)), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: Event::assets(RawEvent::Issued(ausd_id, 2, 30000)), topics: vec![] },
		]);
	});
}

#[test]
fn issuing_before_creating_should_now_work() {
	new_test_ext().execute_with(|| {
		let asset_id = 10;
		assert_noop!(
			Assets::issue(Origin::root(), asset_id, 1, 10000),
			Error::<Test>::TokenNotExist
		);
	});
}

#[test]
fn transferring_amount_above_available_balance_should_work() {
	new_test_ext().execute_with(|| {
		let alice = 1;
		let bob = 2;
		let ausd_id = Assets::next_asset_id();
		assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18, TokenType::Stable));

		System::set_block_number(1);

		assert_ok!(Assets::issue(Origin::root(), ausd_id, alice, 10000));

		assert_ok!(Assets::transfer(Origin::signed(alice), ausd_id, bob, 1000));
		assert_eq!(Assets::account_asset_ids(bob), vec![ausd_id]);
		assert_eq!(Assets::token_details(ausd_id), Token::new(b"aUSD".to_vec(), 18, 10000, TokenType::Stable));

		assert_eq!(Assets::account_assets((ausd_id, alice)).balance, 9000);
		assert_eq!(Assets::account_assets((ausd_id, bob)).balance, 1000);

		assert_eq!(System::events(), vec![
			EventRecord { phase: Phase::Initialization, event: Event::assets(RawEvent::Issued(ausd_id, 1, 10000)), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: Event::assets(RawEvent::Transferred(ausd_id, 1, 2, 1000)), topics: vec![] }
		]);
	});
}

#[test]
fn transferring_amount_less_than_available_balance_should_not_work() {
	new_test_ext().execute_with(|| {
		let ausd_id = Assets::next_asset_id();
		assert_ok!(Assets::create(Origin::root(), vec![0x12, 0x34], 8, TokenType::Stable));
		let (alice, bob) = (1, 2);
		assert_noop!(
			Assets::transfer(Origin::signed(alice), ausd_id, bob, 1000),
			Error::<Test>::InsufficientBalanceForTransaction
		);
	});
}

#[test]
fn transferring_less_than_one_unit_should_not_work() {
	new_test_ext().execute_with(|| {
		let ausd_id = Assets::next_asset_id();
		assert_ok!(Assets::create(Origin::root(), vec![0x12, 0x34], 8, TokenType::Stable));
		let (alice, bob) = (1, 2);
		assert_noop!(
			Assets::transfer(Origin::signed(alice), ausd_id, bob, 0),
			Error::<Test>::ZeroAmountOfBalance
		);
	});
}

#[test]
fn destroying_asset_balance_with_positive_balance_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let alice = 1;
		let ausd_id = Assets::next_asset_id();
		assert_ok!(Assets::create(Origin::root(), b"aUSD".to_vec(), 18, TokenType::Stable));

		System::set_block_number(1);

		assert_ok!(Assets::issue(Origin::root(), ausd_id, alice, 10000));
		assert_ok!(Assets::destroy(Origin::signed(alice), ausd_id, 1000));

		assert_eq!(Assets::token_details(ausd_id), Token::new(b"aUSD".to_vec(), 18, 9000, TokenType::Stable));

		assert_eq!(Assets::account_assets((ausd_id, alice)).balance, 9000);

		assert_eq!(System::events(), vec![
			EventRecord { phase: Phase::Initialization, event: Event::assets(RawEvent::Created(0, Token { symbol: b"aUSD".to_vec(), precision: 18, total_supply: 0, token_type: TokenType::Stable, pair: None })), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: Event::assets(RawEvent::Issued(ausd_id, 1, 10000)), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: Event::assets(RawEvent::Destroyed(ausd_id, 1, 1000)), topics: vec![] }
		]);
	});
}

#[test]
fn destroying_asset_balance_with_zero_balance_should_not_work() {
	new_test_ext().execute_with(|| {
		let id = Assets::next_asset_id();
		assert_ok!(Assets::create(Origin::root(), vec![0x12, 0x34], 8, TokenType::Stable));
		let alice = 1;
		assert_ok!(Assets::issue(Origin::root(), id, alice, 100));
		assert_noop!(
			Assets::destroy(Origin::signed(alice), id, 100 + 1),
			Error::<Test>::InsufficientBalanceForTransaction
		);
	});
}

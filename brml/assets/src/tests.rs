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

use super::*;
use crate::mock::*;
use frame_support::{assert_ok, assert_noop, dispatch::DispatchError};
use system::{EventRecord, Phase};

#[test]
fn create_asset_should_work() {
	new_test_ext().execute_with(|| {
		let token1 = Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
		};

		let id1 = Assets::next_asset_id();
		let token_type1 = TokenSymbol::from(id1);

		System::set_block_number(1);

		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_eq!(Assets::next_asset_id(), id1 + 1);
		assert_eq!(Assets::token_details(token_type1), token1);

		let token2 = Token {
			symbol: vec![0x56, 0x68, 0x90],
			precision: 4,
			total_supply: 0,
		};

		let id2 = Assets::next_asset_id();
		let token_type2 = TokenSymbol::from(id2);
		assert_ok!(Assets::create(Origin::ROOT, vec![0x56, 0x68, 0x90], 4)); // take 2 as asset id
		assert_eq!(Assets::next_asset_id(), id2 + 1);
		assert_eq!(Assets::token_details(token_type2), token2);

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::Initialization,
				event: TestEvent::assets(RawEvent::Created(id1, token1)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::Initialization,
				event: TestEvent::assets(RawEvent::Created(id2, token2)),
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
		let ausd_type = TokenSymbol::from(ausd_id);
		assert_ok!(Assets::create(Origin::ROOT, b"aUSD".to_vec(), 18));

		let dot_id = Assets::next_asset_id();
		let dot_type = TokenSymbol::from(dot_id);
		assert_ok!(Assets::create(Origin::ROOT, b"DOT".to_vec(), 12));

		let vdot_id = Assets::next_asset_id();
		let vdot_type = TokenSymbol::from(vdot_id);
		assert_ok!(Assets::create(Origin::ROOT, b"vDOT".to_vec(), 12));

		System::set_block_number(1);

		assert_ok!(Assets::issue(Origin::ROOT, dot_type, alice, 10000));
		assert_eq!(Assets::token_details(dot_type), Token::new(b"DOT".to_vec(), 12, 10000));
		assert_eq!(Assets::account_assets((dot_type, alice)).balance, 10000);

		assert_ok!(Assets::issue(Origin::ROOT, vdot_type, alice, 20000));
		assert_eq!(Assets::token_details(vdot_type), Token::new(b"vDOT".to_vec(), 12, 20000));
		assert_eq!(Assets::account_assets((vdot_type, alice)).balance, 20000);

		let bob = 2;
		// issue bob balances twice
		assert_ok!(Assets::issue(Origin::ROOT, ausd_type, bob, 20000));
		assert_ok!(Assets::issue(Origin::ROOT, ausd_type, bob, 30000));
		assert_eq!(Assets::token_details(ausd_type), Token::new(b"aUSD".to_vec(), 18, 50000));
		assert_eq!(Assets::account_assets((ausd_type, bob)).balance, 50000);

		// creare a exsited token
		assert_eq!(
			Assets::create(Origin::ROOT, b"vDOT".to_vec(), 12),
			Err(DispatchError::Module { index: 0, error: 0, message: Some("TokenExisted") })
		);

		assert_eq!(System::events(), vec![
			EventRecord { phase: Phase::Initialization, event: TestEvent::assets(RawEvent::Issued(TokenSymbol::DOT, 1, 10000)), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: TestEvent::assets(RawEvent::Issued(TokenSymbol::vDOT, 1, 20000)), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: TestEvent::assets(RawEvent::Issued(TokenSymbol::aUSD, 2, 20000)), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: TestEvent::assets(RawEvent::Issued(TokenSymbol::aUSD, 2, 30000)), topics: vec![] },
		]);
	});
}

#[test]
fn issuing_before_creating_should_now_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Assets::issue(Origin::ROOT, TokenSymbol::DOT, 1, 10000),
			AssetsError::TokenNotExist
		);
	});
}

#[test]
fn transferring_amount_above_available_balance_should_work() {
	new_test_ext().execute_with(|| {
		let alice = 1;
		let bob = 2;
		let ausd_id = Assets::next_asset_id();
		let ausd_type = TokenSymbol::from(ausd_id);
		assert_ok!(Assets::create(Origin::ROOT, b"aUSD".to_vec(), 18));

		System::set_block_number(1);

		assert_ok!(Assets::issue(Origin::ROOT, ausd_type, alice, 10000));

		assert_ok!(Assets::transfer(Origin::signed(alice), ausd_type, bob, 1000));
		assert_eq!(Assets::account_asset_ids(bob), vec![ausd_type]);
		assert_eq!(Assets::token_details(ausd_type), Token::new(b"aUSD".to_vec(), 18, 10000));

		assert_eq!(Assets::account_assets((ausd_type, alice)).balance, 9000);
		assert_eq!(Assets::account_assets((ausd_type, bob)).balance, 1000);

		assert_eq!(System::events(), vec![
			EventRecord { phase: Phase::Initialization, event: TestEvent::assets(RawEvent::Issued(ausd_type, 1, 10000)), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: TestEvent::assets(RawEvent::Transferred(ausd_type, 1, 2, 1000)), topics: vec![] }
		]);
	});
}

#[test]
fn transferring_amount_less_than_available_balance_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		let (alice, bob) = (1, 2);
		assert_noop!(
			Assets::transfer(Origin::signed(alice), TokenSymbol::aUSD, bob, 1000),
			AssetsError::InvalidBalanceForTransaction
		);
	});
}

#[test]
fn transferring_less_than_one_unit_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		let (alice, bob) = (1, 2);
		assert_noop!(
			Assets::transfer(Origin::signed(alice), TokenSymbol::aUSD, bob, 0),
			AssetsError::ZeroAmountOfBalance
		);
	});
}

#[test]
fn destroying_asset_balance_with_positive_balance_should_work() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);

		let alice = 1;
		let ausd_id = Assets::next_asset_id();
		let ausd_type = TokenSymbol::from(ausd_id);
		assert_ok!(Assets::create(Origin::ROOT, b"aUSD".to_vec(), 18));

		System::set_block_number(1);

		assert_ok!(Assets::issue(Origin::ROOT, ausd_type, alice, 10000));
		assert_ok!(Assets::destroy(Origin::signed(alice), ausd_type, 1000));

		assert_eq!(Assets::token_details(ausd_type), Token::new(b"aUSD".to_vec(), 18, 9000));

		assert_eq!(Assets::account_assets((ausd_type, alice)).balance, 9000);

		assert_eq!(System::events(), vec![
			EventRecord { phase: Phase::Initialization, event: TestEvent::assets(RawEvent::Created(0, Token { symbol: b"aUSD".to_vec(), precision: 18, total_supply: 0 })), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: TestEvent::assets(RawEvent::Issued(ausd_type, 1, 10000)), topics: vec![] },
			EventRecord { phase: Phase::Initialization, event: TestEvent::assets(RawEvent::Destroyed(ausd_type, 1, 1000)), topics: vec![] }
		]);
	});
}

#[test]
fn destroying_asset_balance_with_zero_balance_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		let alice = 1;
		let id = Assets::next_asset_id();
		let token_symbol = TokenSymbol::from(id - 1);
		assert_ok!(Assets::issue(Origin::ROOT, token_symbol, alice, 100));
		assert_noop!(
			Assets::destroy(Origin::signed(alice), token_symbol, 100 + 1),
			AssetsError::InvalidBalanceForTransaction
		);
	});
}

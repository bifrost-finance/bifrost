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
use frame_support::{assert_ok, assert_noop};
use system::{EventRecord, Phase};

#[test]
fn create_asset_should_work() {
	new_test_ext().execute_with(|| {
		let token1 = Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
		};
		let vtoken1 = token1.clone();
		let token_pair1 = TokenPair::new(token1, vtoken1);
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_eq!(Assets::next_asset_id(), 1);
		assert_eq!(Assets::token_details(0), token_pair1);

		let token2 = Token {
			symbol: vec![0x56, 0x68, 0x90],
			precision: 4,
			total_supply: 0,
		};
		let vtoken2 = token2.clone();
		let token_pair2 = TokenPair::new(token2, vtoken2);
		assert_ok!(Assets::create(Origin::ROOT, vec![0x56, 0x68, 0x90], 4));
		assert_eq!(Assets::next_asset_id(), 2);
		assert_eq!(Assets::token_details(1), token_pair2);

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Created(0, token_pair1)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Created(1, token_pair2)),
				topics: vec![],
			}
		]);
	});
}

#[test]
fn issuing_asset_units_to_issuer_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));

		let token = Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
		};
		let token_pair = TokenPair::new(token.clone(), token.clone());
		assert_ok!(Assets::issue(Origin::ROOT, 0, TokenType::VToken, 1, 10000));
		assert_eq!(Assets::token_details(0), TokenPair::new(
			token.clone(),
			Token { total_supply: 10000, ..token.clone() }
		));
		assert_eq!(Assets::balances((0, TokenType::VToken, 1)), 10000);

		assert_ok!(Assets::issue(Origin::ROOT, 0, TokenType::Token, 2, 20000));
		assert_eq!(Assets::token_details(0), TokenPair::new(
			Token { total_supply: 20000, ..token.clone() },
			Token { total_supply: 10000, ..token.clone() }
		));
		assert_eq!(Assets::balances((0, TokenType::Token, 2)), 20000);

		assert_ok!(Assets::issue(Origin::ROOT, 0, TokenType::Token, 2, 30000));
		assert_eq!(Assets::token_details(0), TokenPair::new(
			Token { total_supply: 50000, ..token.clone() },
			Token { total_supply: 10000, ..token.clone() }
		));
		assert_eq!(Assets::balances((0, TokenType::Token, 2)), 50000);

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Created(0, token_pair)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Issued(0, TokenType::VToken, 1, 10000)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Issued(0, TokenType::Token, 2, 20000)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Issued(0, TokenType::Token, 2, 30000)),
				topics: vec![],
			}
		]);
	});
}

#[test]
fn issuing_before_creating_should_now_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Assets::issue(Origin::ROOT, 0, TokenType::Token, 1, 10000),
			AssetsError::TokenNotExist
		);
	});
}

#[test]
fn transferring_amount_above_available_balance_should_work() {
	new_test_ext().execute_with(|| {
		let token = Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
		};
		let token_pair = TokenPair::new(token.clone(), token.clone());

		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::issue(Origin::ROOT, 0, TokenType::VToken, 1, 10000));

		assert_ok!(Assets::transfer(Origin::signed(1), 0, TokenType::VToken, 2, 1000));
		assert_eq!(Assets::token_details(0), TokenPair::new(
			token.clone(),
			Token { total_supply: 10000, ..token.clone() }
		));
		assert_eq!(Assets::balances((0, TokenType::VToken, 1)), 9000);
		assert_eq!(Assets::balances((0, TokenType::VToken, 2)), 1000);

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Created(0, token_pair)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Issued(0, TokenType::VToken, 1, 10000)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Transferred(0, TokenType::VToken, 1, 2, 1000)),
				topics: vec![],
			}
		]);
	});
}

#[test]
fn transferring_amount_less_than_available_balance_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_noop!(
			Assets::transfer(Origin::signed(1), 0, TokenType::VToken, 1, 1000),
			AssetsError::InvalidBalanceForTransaction
		);
	});
}

#[test]
fn transferring_less_than_one_unit_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_noop!(
			Assets::transfer(Origin::signed(1), 0, TokenType::VToken, 1, 0),
			AssetsError::ZeroAmountOfBalance
		);
	});
}

#[test]
fn destroying_asset_balance_with_positive_balance_should_work() {
	new_test_ext().execute_with(|| {
		let token = Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
		};
		let token_pair = TokenPair::new(token.clone(), token.clone());

		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::issue(Origin::ROOT, 0, TokenType::VToken, 1, 10000));
		assert_ok!(Assets::destroy(Origin::signed(1), 0, TokenType::VToken, 1000));
		assert_eq!(Assets::token_details(0), TokenPair::new(
			token.clone(),
			Token { total_supply: 9000, ..token.clone() }
		));
		assert_eq!(Assets::balances((0, TokenType::VToken, 1)), 9000);

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Created(0, token_pair)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Issued(0, TokenType::VToken, 1, 10000)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Destroyed(0, TokenType::VToken, 1, 1000)),
				topics: vec![],
			}
		]);
	});
}

#[test]
fn destroying_asset_balance_with_zero_balance_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::issue(Origin::ROOT, 0, TokenType::VToken, 1, 100));
		assert_noop!(
			Assets::destroy(Origin::signed(1), 0, TokenType::VToken, 200),
			AssetsError::InvalidBalanceForTransaction
		);
	});
}

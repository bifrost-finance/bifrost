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
use crate::mock::{Assets, Origin, System, TestEvent, new_test_ext};
use frame_support::{assert_ok, assert_noop};
use system::{EventRecord, Phase};

#[test]
fn create_asset_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_eq!(Assets::next_asset_id(), 1);
		assert_eq!(Assets::token_details(0), Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 0,
		});

		assert_ok!(Assets::create(Origin::ROOT, vec![0x56, 0x68, 0x90], 4));
		assert_eq!(Assets::next_asset_id(), 2);
		assert_eq!(Assets::token_details(1), Token {
			symbol: vec![0x56, 0x68, 0x90],
			precision: 4,
			total_supply: 0,
		});

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Created(0, Token {
					symbol: vec![0x12, 0x34],
					precision: 8,
					total_supply: 0,
				})),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Created(1, Token {
					symbol: vec![0x56, 0x68, 0x90],
					precision: 4,
					total_supply: 0,
				})),
				topics: vec![],
			}
		]);
	});
}

#[test]
fn issuing_asset_units_to_issuer_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));

		assert_ok!(Assets::issue(Origin::ROOT, 0, 1, 10000));
		assert_eq!(Assets::token_details(0), Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 10000,
		});
		assert_eq!(Assets::balances((0, 1)), 10000);

		assert_ok!(Assets::issue(Origin::ROOT, 0, 2, 20000));
		assert_eq!(Assets::token_details(0), Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 30000,
		});
		assert_eq!(Assets::balances((0, 2)), 20000);

		assert_ok!(Assets::issue(Origin::ROOT, 0, 2, 30000));
		assert_eq!(Assets::token_details(0), Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 60000,
		});
		assert_eq!(Assets::balances((0, 2)), 50000);

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Created(0, Token {
					symbol: vec![0x12, 0x34],
					precision: 8,
					total_supply: 0,
				})),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Issued(0, 1, 10000)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Issued(0, 2, 20000)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Issued(0, 2, 30000)),
				topics: vec![],
			}
		]);
	});
}

#[test]
fn issuing_before_creating_should_now_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(Assets::issue(Origin::ROOT, 0, 1, 10000), "asset should be created first");
	});
}

#[test]
fn transferring_amount_above_available_balance_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::issue(Origin::ROOT, 0, 1, 10000));

		assert_ok!(Assets::transfer(Origin::signed(1), 0, 2, 1000));
		assert_eq!(Assets::token_details(0), Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 10000,
		});
		assert_eq!(Assets::balances((0, 1)), 9000);
		assert_eq!(Assets::balances((0, 2)), 1000);

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Created(0, Token {
					symbol: vec![0x12, 0x34],
					precision: 8,
					total_supply: 0,
				})),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Issued(0, 1, 10000)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Transferred(0, 1, 2, 1000)),
				topics: vec![],
			}
		]);
	});
}

#[test]
fn transferring_amount_less_than_available_balance_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_noop!(Assets::transfer(Origin::signed(1), 0, 1, 1000),
			"origin account balance must be greater than or equal to the transfer amount");
	});
}

#[test]
fn transferring_less_than_one_unit_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_noop!(Assets::transfer(Origin::signed(1), 0, 1, 0),
			"transfer amount should be non-zero");
	});
}

#[test]
fn destroying_asset_balance_with_positive_balance_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::issue(Origin::ROOT, 0, 1, 10000));
		assert_ok!(Assets::destroy(Origin::signed(1), 0, 1000));
		assert_eq!(Assets::token_details(0), Token {
			symbol: vec![0x12, 0x34],
			precision: 8,
			total_supply: 9000,
		});
		assert_eq!(Assets::balances((0, 1)), 9000);

		assert_eq!(System::events(), vec![
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Created(0, Token {
					symbol: vec![0x12, 0x34],
					precision: 8,
					total_supply: 0,
				})),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Issued(0, 1, 10000)),
				topics: vec![],
			},
			EventRecord {
				phase: Phase::ApplyExtrinsic(0),
				event: TestEvent::assets(RawEvent::Destroyed(0, 1, 1000)),
				topics: vec![],
			}
		]);
	});
}

#[test]
fn destroying_asset_balance_with_zero_balance_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));
		assert_ok!(Assets::issue(Origin::ROOT, 0, 1, 100));
		assert_noop!(Assets::destroy(Origin::signed(1), 0, 200),
			"amount should be less than or equal to origin balance");
	});
}

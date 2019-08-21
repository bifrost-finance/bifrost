// Copyright 2019 Liebi Technologies.
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
use crate::mock::{Assets, Settlement, Origin, System, new_test_ext};
use runtime_io::with_externalities;
use srml_support::{assert_ok, assert_noop};
use system::{EventRecord, Phase};
use sr_primitives::traits::OnInitialize;
use sr_primitives::traits::OnFinalize;

const SETTLEMENT_PERIOD: u64 = 24 * 60 * 10;

#[test]
fn issuing_asset_clearing_should_work() {
	with_externalities(&mut new_test_ext(), || {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));

		assert_ok!(Assets::issue(Origin::ROOT, 0, 1, 10000));
		assert_eq!(Settlement::clearing_assets((0, 1, 0)), BalanceDuration {
			last_block: 1,
			last_balance: 10000,
			value: 0,
		});
		assert_eq!(Settlement::clearing_tokens((0, 0)), BalanceDuration {
			last_block: 1,
			last_balance: 10000,
			value: 0,
		});

		System::set_block_number(100);
		assert_ok!(Assets::issue(Origin::ROOT, 0, 2, 20000));
		assert_eq!(Settlement::clearing_assets((0, 2, 0)), BalanceDuration {
			last_block: 100,
			last_balance: 20000,
			value: 0,
		});
		assert_eq!(Settlement::clearing_tokens((0, 0)), BalanceDuration {
			last_block: 100,
			last_balance: 30000,
			value: 10000 * (100 - 1),
		});

		System::set_block_number(200);
		assert_ok!(Assets::issue(Origin::ROOT, 0, 2, 30000));
		assert_eq!(Settlement::clearing_assets((0, 2, 0)), BalanceDuration {
			last_block: 200,
			last_balance: 50000,
			value: 20000 * (200 - 100),
		});
		assert_eq!(Settlement::clearing_tokens((0, 0)), BalanceDuration {
			last_block: 200,
			last_balance: 60000,
			value: 10000 * (100 - 1) + 30000 * (200 - 100),
		});
	});
}

#[test]
fn transfer_asset_clearing_should_work() {
	with_externalities(&mut new_test_ext(), || {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));

		assert_ok!(Assets::issue(Origin::ROOT, 0, 1, 10000));
		assert_eq!(Settlement::clearing_assets((0, 1, 0)), BalanceDuration {
			last_block: 1,
			last_balance: 10000,
			value: 0,
		});

		System::set_block_number(100);
		assert_ok!(Assets::transfer(Origin::signed(1), 0, 2, 1000));
		assert_eq!(Settlement::clearing_assets((0, 1, 0)), BalanceDuration {
			last_block: 100,
			last_balance: 9000,
			value: 10000 * (100 - 1),
		});
		assert_eq!(Settlement::clearing_assets((0, 2, 0)), BalanceDuration {
			last_block: 100,
			last_balance: 1000,
			value: 0,
		});
	});
}

#[test]
fn destroy_asset_clearing_should_work() {
	with_externalities(&mut new_test_ext(), || {
		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));

		assert_ok!(Assets::issue(Origin::ROOT, 0, 1, 10000));
		assert_eq!(Settlement::clearing_assets((0, 1, 0)), BalanceDuration {
			last_block: 1,
			last_balance: 10000,
			value: 0,
		});

		System::set_block_number(100);
		assert_ok!(Assets::destroy(Origin::signed(1), 0, 1000));
		assert_eq!(Settlement::clearing_assets((0, 1, 0)), BalanceDuration {
			last_block: 100,
			last_balance: 9000,
			value: 10000 * (100 - 1),
		});

		System::set_block_number(200);
		assert_ok!(Assets::destroy(Origin::signed(1), 0, 500));
		assert_eq!(Settlement::clearing_assets((0, 1, 0)), BalanceDuration {
			last_block: 200,
			last_balance: 8500,
			value: 10000 * (100 - 1) + 9000 * (200 - 100),
		});
	});
}

#[test]
fn new_settlement_should_work() {
	with_externalities(&mut new_test_ext(), || {
		Settlement::on_initialize(0);
		assert_eq!(Settlement::next_settlement_id(), 1);

		Settlement::on_initialize(SETTLEMENT_PERIOD);
		assert_eq!(Settlement::next_settlement_id(), 2);

		Settlement::on_initialize(SETTLEMENT_PERIOD * 2);
		assert_eq!(Settlement::next_settlement_id(), 3);
	});
}

#[test]
fn destroy_clearing_record_should_work() {
	with_externalities(&mut new_test_ext(), || {
		Settlement::on_initialize(0);

		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));

		assert_ok!(Assets::issue(Origin::ROOT, 0, 1, 10000));
		assert_eq!(Settlement::clearing_assets((0, 1, 0)), BalanceDuration {
			last_block: 1,
			last_balance: 10000,
			value: 0,
		});

		System::set_block_number(100);
		assert_ok!(Assets::destroy(Origin::signed(1), 0, 1000));
		assert_eq!(Settlement::clearing_assets((0, 1, Settlement::current_settlement_id())),
			BalanceDuration {
				last_block: 100,
				last_balance: 9000,
				value: 10000 * (100 - 1),
			}
		);

		System::set_block_number(SETTLEMENT_PERIOD + 100);
		Settlement::on_initialize(SETTLEMENT_PERIOD);
		let curr_stl_id = Settlement::current_settlement_id();
		assert_eq!(curr_stl_id, 1);
		assert_eq!(Settlement::next_settlement_id(), 2);
		//		System::set_block_number(200);
		assert_ok!(Assets::destroy(Origin::signed(1), 0, 500));
		assert_eq!(Settlement::clearing_assets((0, 1, curr_stl_id)), BalanceDuration {
			last_block: SETTLEMENT_PERIOD + 100,
			last_balance: 8500,
			value: 9000 * 100,
		});
	});
}

#[test]
fn enumerate_should_work() {
	with_externalities(&mut new_test_ext(), || {
		Settlement::on_initialize(0);

		assert_ok!(Assets::create(Origin::ROOT, vec![0x12, 0x34], 8));

		assert_ok!(Assets::issue(Origin::ROOT, 0, 1, 10000));
		assert_eq!(Settlement::clearing_assets((0, 1, 0)), BalanceDuration {
			last_block: 1,
			last_balance: 10000,
			value: 0,
		});

		assert_ok!(Assets::issue(Origin::ROOT, 0, 2, 5000));
		assert_eq!(Settlement::clearing_assets((0, 2, 0)), BalanceDuration {
			last_block: 1,
			last_balance: 5000,
			value: 0,
		});
		Settlement::on_finalize(0);

		const SETTLEMENT_PERIOD: u64 = 24 * 60 * 10;
		System::set_block_number(SETTLEMENT_PERIOD);
		Settlement::on_initialize(SETTLEMENT_PERIOD);

		assert_ok!(Assets::issue(Origin::ROOT, 0, 2, 5000));
		assert_eq!(Settlement::clearing_assets((0, 2, 1)), BalanceDuration {
			last_block: 14400,
			last_balance: 10000,
			value: 0,
		});

//		Settlement::on_finalize(SETTLEMENT_PERIOD);
	});
}

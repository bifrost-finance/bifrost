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

use crate::mock::{Assets, Origin, new_test_ext};
use frame_support::assert_ok;
use sr_primitives::traits::OnFinalize;

#[test]
fn set_default_exchange_rate_should_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(Assets::get_exchange_rate(), 1);
		assert_eq!(Assets::get_rate_per_block(), 0);
	});
}

#[test]
fn update_exhange_rate_should_work() {
	new_test_ext().execute_with(|| {
		assert_eq!(Assets::get_exchange_rate(), 1);
		assert_eq!(Assets::get_rate_per_block(), 0);
		// set a new rate and exchange
		assert_ok!(Assets::set_rate(Origin::ROOT, 2));
		assert_ok!(Assets::set_exchange(Origin::ROOT, 10));
		Assets::on_finalize(7);
		assert_eq!(Assets::get_rate_per_block(), 2);
		assert_eq!(Assets::get_exchange_rate(), 8);
	});
}

#[test]
fn update_rate_by_max_u64_should_error() {
	new_test_ext().execute_with(|| {
		assert_eq!(Assets::get_exchange_rate(), 1);
		assert_eq!(Assets::get_rate_per_block(), 0);
		// set max rate
		assert_ok!(Assets::set_rate(Origin::ROOT, u64::max_value()));
		Assets::on_finalize(9);
		// because rate is set as max value, exchange should be zero due to overflow
		assert_eq!(Assets::get_exchange_rate(), 0);
	});
}

#[test]
fn update_rate_multiple_times() {
	new_test_ext().execute_with(|| {
		assert_eq!(Assets::get_exchange_rate(), 1);
		assert_eq!(Assets::get_rate_per_block(), 0);
		// set rate and exchange
		assert_ok!(Assets::set_rate(Origin::ROOT, 4));
		assert_ok!(Assets::set_exchange(Origin::ROOT, 20));
		// calculate 3 times, 20 - 3 * 4
		Assets::on_finalize(9);
		Assets::on_finalize(9);
		Assets::on_finalize(9);
		assert_eq!(Assets::get_exchange_rate(), 8);
	});
}

#[test]
fn update_rate_multiple_times_until_overflow() {
	new_test_ext().execute_with(|| {
		assert_eq!(Assets::get_exchange_rate(), 1);
		assert_eq!(Assets::get_rate_per_block(), 0);
		// set rate and exchange
		assert_ok!(Assets::set_rate(Origin::ROOT, 4));
		assert_ok!(Assets::set_exchange(Origin::ROOT, 12));
		// calculate 3 times, 12 - 4 * 4
		Assets::on_finalize(9);
		Assets::on_finalize(9);
		Assets::on_finalize(9);
		Assets::on_finalize(9);
		assert_eq!(Assets::get_exchange_rate(), 0);
	});
}

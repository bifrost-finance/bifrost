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

//! Test utilities

#![cfg(test)]
#![allow(non_upper_case_globals)]

use crate::*;
use crate::mock::*;
use frame_support::{assert_ok, assert_noop};

pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		MinterReward::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		MinterReward::on_initialize(System::block_number());
	}
}

#[test]
fn minter_reward_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			run_to_block(2);

			let to_sell_vdot = 20;
			let to_sell_ksm = 20;

			assert_ok!(MinterReward::mint(Origin::signed(ALICE), DOT, to_sell_vdot));
			// assert_eq!(MinterReward::current_round_start_at(), 2);
			run_to_block(10);
			
			// assert_eq!(MinterReward::current_round(), 3);
			// assert_eq!(MinterReward::reward_by_one_block(), 75);
			dbg!(MinterReward::maximum_vtoken_minted());
			
			run_to_block(12);
			assert_ok!(MinterReward::mint(Origin::signed(BOB), DOT, to_sell_vdot + 40));
			// assert_ok!(MinterReward::mint(Origin::signed(ALICE), DOT, to_sell_vdot + 20));
			run_to_block(23);
			// run_to_block(17);
		});
}
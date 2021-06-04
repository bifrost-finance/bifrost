// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Test utilities

#![cfg(test)]
#![allow(non_upper_case_globals)]

use crate::*;
use crate::mock::*;
use frame_support::{assert_ok};
use std::convert::TryFrom;

pub(crate) fn run_to_block(n: u64) {
	while System::block_number() < n {
		MinterReward::on_finalize(System::block_number());
		System::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		MinterReward::on_initialize(System::block_number());
	}
}

// The following test is ignored due to some bugs on zenlink. It can be reopened after the bug is fixed.frame_system
// The functionality has already been tested.
#[test]
#[ignore]
fn minter_reward_should_work() {
	ExtBuilder::default()
		.ten_thousand_for_alice()
		.build()
		.execute_with(|| {
			run_to_block(2);

			let to_sell_vdot = 20;
			// let to_sell_ksm = 20;

			// create DEX pair
			let ausd_asset_id: AssetId = AssetId::try_from(CurrencyId::Stable(TokenSymbol::AUSD)).unwrap();
			let dot_asset_id: AssetId = AssetId::try_from(CurrencyId::Token(TokenSymbol::DOT)).unwrap();
			let vdot_asset_id: AssetId = AssetId::try_from(CurrencyId::VToken(TokenSymbol::DOT)).unwrap();
			let ksm_asset_id: AssetId = AssetId::try_from(CurrencyId::Token(TokenSymbol::KSM)).unwrap();
			let vksm_asset_id: AssetId = AssetId::try_from(CurrencyId::VToken(TokenSymbol::KSM)).unwrap();

			assert_ok!(ZenlinkProtocol::create_pair(Origin::signed(ALICE), ausd_asset_id, dot_asset_id));
			assert_ok!(ZenlinkProtocol::create_pair(Origin::signed(ALICE), ausd_asset_id, vdot_asset_id));
			assert_ok!(ZenlinkProtocol::create_pair(Origin::signed(ALICE), ausd_asset_id, ksm_asset_id));
			assert_ok!(ZenlinkProtocol::create_pair(Origin::signed(ALICE), ausd_asset_id, vksm_asset_id));

			let deadline: BlockNumberFor<Runtime> = <frame_system::Pallet<Runtime>>::block_number() + <Runtime as frame_system::Config>::BlockNumber::from(100u32);
			assert_ok!(ZenlinkProtocol::add_liquidity(Origin::signed(ALICE), ausd_asset_id, dot_asset_id, 1000, 1000, 1, 1, deadline));
			assert_ok!(ZenlinkProtocol::add_liquidity(Origin::signed(ALICE), ausd_asset_id, vdot_asset_id, 1000, 1000, 1, 1, deadline));
			assert_ok!(ZenlinkProtocol::add_liquidity(Origin::signed(ALICE), ausd_asset_id, ksm_asset_id, 1000, 1000, 1, 1, deadline));
			assert_ok!(ZenlinkProtocol::add_liquidity(Origin::signed(ALICE), ausd_asset_id, vksm_asset_id, 1000, 1000, 1, 1, deadline));

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
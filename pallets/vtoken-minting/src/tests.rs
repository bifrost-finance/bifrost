// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

// Ensure we're `no_std` when compiling for Wasm.

#![cfg(test)]

use frame_support::{assert_noop, assert_ok};

use crate::{mock::*, *};

#[test]
fn initialize_mint() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		VtokenMinting::increase_token_pool(KSM, 1000);
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 10));
	});
}

// #[test]
// fn add_to_issue_whitelist_should_work() {
// 	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
// 		// Charlie is not allowed to issue ZLK.
// 		assert_noop!(
// 			TokenIssuer::issue(Origin::signed(CHARLIE), ALICE, ZLK, 800),
// 			Error::<Runtime>::NotAllowed
// 		);
// 		// Chalie is added to the issue whitelist to have the ability of issuing ZLK.
// 		assert_ok!(TokenIssuer::add_to_issue_whitelist(
// 			pallet_collective::RawOrigin::Members(2, 3).into(),
// 			ZLK,
// 			CHARLIE
// 		));
// 		assert_eq!(TokenIssuer::get_issue_whitelist(ZLK), Some(vec![CHARLIE]));
// 		// Charlie succuessfully issue 800 unit of ZLK to Alice account
// 		assert_ok!(TokenIssuer::issue(Origin::signed(CHARLIE), ALICE, ZLK, 800));
// 		assert_eq!(Tokens::free_balance(ZLK, &ALICE), 800);
// 	});
// }

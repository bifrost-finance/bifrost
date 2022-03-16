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
fn mint() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		VtokenMinting::increase_token_pool(KSM, 1000);
		assert_ok!(VtokenMinting::set_minimum_mint(Origin::root(), KSM, 1000));
		assert_noop!(
			VtokenMinting::mint(Some(BOB).into(), KSM, 100),
			Error::<Runtime>::BelowMinimumMint
		);
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 1000));
		assert_eq!(VtokenMinting::token_pool(KSM), 2000);
		assert_eq!(VtokenMinting::token_to_add(KSM), 1000);
		assert_eq!(VtokenMinting::minimum_mint(KSM), 1000);
		let (entrance_account, exit_account) = VtokenMinting::get_entrance_and_exit_accounts();
		assert_eq!(Tokens::free_balance(KSM, &entrance_account), 1000);
	});
}

#[test]
fn redeem() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		VtokenMinting::increase_token_pool(KSM, 1000);
		// assert_eq!(VtokenMinting::token_pool(KSM), 1100);
		VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1));
		assert_ok!(VtokenMinting::set_minimum_redeem(Origin::root(), KSM, 90));
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 100));
		assert_noop!(
			VtokenMinting::redeem(Some(BOB).into(), KSM, 80),
			Error::<Runtime>::BelowMinimumRedeem
		);
		assert_ok!(VtokenMinting::redeem(Some(BOB).into(), KSM, 100));
		assert_eq!(VtokenMinting::token_pool(KSM), 1100);
		assert_eq!(VtokenMinting::token_to_add(KSM), 100);
		let (entrance_account, exit_account) = VtokenMinting::get_entrance_and_exit_accounts();
		assert_eq!(Tokens::free_balance(KSM, &entrance_account), 100);
	});
}

#[test]
fn rebond() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		VtokenMinting::increase_token_pool(KSM, 1000);
		VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1));

		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 100));
		assert_ok!(VtokenMinting::redeem(Some(BOB).into(), KSM, 100));
		assert_noop!(
			VtokenMinting::rebond(Some(BOB).into(), KSM, 100),
			Error::<Runtime>::InvalidRebondToken
		);
		assert_ok!(VtokenMinting::add_support_rebond_token(Origin::root(), KSM));
		assert_ok!(VtokenMinting::rebond(Some(BOB).into(), KSM, 100));
		assert_eq!(VtokenMinting::token_pool(KSM), 1100);
		assert_eq!(VtokenMinting::token_to_add(KSM), 100);
		let (entrance_account, exit_account) = VtokenMinting::get_entrance_and_exit_accounts();
		assert_eq!(Tokens::free_balance(KSM, &entrance_account), 100);
	});
}

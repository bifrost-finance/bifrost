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

use frame_support::{assert_noop, assert_ok, BoundedVec};

use crate::{mock::*, *};

#[test]
fn mint() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_minimum_mint(Origin::root(), KSM, 200));
		assert_ok!(VtokenMinting::set_fees(Origin::root(), 600, 20));
		assert_noop!(
			VtokenMinting::mint(Some(BOB).into(), KSM, 100),
			Error::<Runtime>::BelowMinimumMint
		);
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 1000));
		assert_eq!(VtokenMinting::token_pool(KSM), 400);
		assert_eq!(VtokenMinting::token_to_add(KSM), 400);
		assert_eq!(VtokenMinting::minimum_mint(KSM), 200);
		assert_eq!(Tokens::total_issuance(vKSM), 1400);

		let (entrance_account, _exit_account) = VtokenMinting::get_entrance_and_exit_accounts();
		assert_eq!(Tokens::free_balance(KSM, &entrance_account), 400);
		let fee_account: AccountId = <Runtime as Config>::FeeAccount::get();
		assert_eq!(Tokens::free_balance(KSM, &fee_account), 600);
	});
}

#[test]
fn redeem() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_fees(Origin::root(), 20, 20));
		assert_ok!(VtokenMinting::set_unlock_duration(Origin::root(), KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::increase_token_pool(KSM, 1000));
		assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::set_minimum_redeem(Origin::root(), KSM, 90));
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 1000));
		assert_noop!(
			VtokenMinting::redeem(Some(BOB).into(), vKSM, 80),
			Error::<Runtime>::BelowMinimumRedeem
		);
		assert_noop!(
			VtokenMinting::redeem(Some(BOB).into(), KSM, 80),
			Error::<Runtime>::NotSupportTokenType
		);
		assert_ok!(VtokenMinting::redeem(Some(BOB).into(), vKSM, 100));
		assert_ok!(VtokenMinting::redeem(Some(BOB).into(), vKSM, 200));
		assert_eq!(VtokenMinting::token_pool(KSM), 1720); // 1000 + 980 - 80 - 180
		assert_eq!(VtokenMinting::token_to_add(KSM), 980);
		assert_eq!(VtokenMinting::currency_unlocking_total(), 260); // 80 + 180
		let (entrance_account, _exit_account) = VtokenMinting::get_entrance_and_exit_accounts();
		assert_eq!(Tokens::free_balance(KSM, &entrance_account), 980);
		let mut ledger_list_origin = BoundedVec::default();
		assert_ok!(ledger_list_origin.try_push(0));
		assert_ok!(ledger_list_origin.try_push(1));
		assert_eq!(
			VtokenMinting::user_unlock_ledger(BOB, KSM),
			Some((260, ledger_list_origin.clone()))
		);
		assert_eq!(VtokenMinting::token_unlock_ledger(KSM, 0), Some((BOB, 80, TimeUnit::Era(2))));
		assert_eq!(
			VtokenMinting::time_unit_unlock_ledger(TimeUnit::Era(2), KSM),
			Some((260, ledger_list_origin, KSM))
		);
	});
}

#[test]
fn rebond() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_unlock_duration(Origin::root(), KSM, TimeUnit::Era(0)));
		assert_ok!(VtokenMinting::increase_token_pool(KSM, 1000));
		assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		let mut ledger_list_origin = BoundedVec::default();
		assert_ok!(ledger_list_origin.try_push(0));
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 200));
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 100));
		assert_ok!(VtokenMinting::redeem(Some(BOB).into(), vKSM, 200));
		assert_ok!(VtokenMinting::redeem(Some(BOB).into(), vKSM, 100));
		assert_eq!(VtokenMinting::token_unlock_ledger(KSM, 1), Some((BOB, 100, TimeUnit::Era(1))));
		assert_noop!(
			VtokenMinting::rebond(Some(BOB).into(), KSM, 100),
			Error::<Runtime>::InvalidRebondToken
		);
		assert_ok!(VtokenMinting::add_support_rebond_token(Origin::root(), KSM));
		assert_ok!(VtokenMinting::rebond(Some(BOB).into(), KSM, 200));
		assert_eq!(
			VtokenMinting::time_unit_unlock_ledger(TimeUnit::Era(1), KSM),
			Some((100, ledger_list_origin.clone(), KSM))
		);
		assert_eq!(
			VtokenMinting::user_unlock_ledger(BOB, KSM),
			Some((100, ledger_list_origin.clone()))
		);
		assert_eq!(VtokenMinting::token_unlock_ledger(KSM, 0), Some((BOB, 100, TimeUnit::Era(1))));
		assert_eq!(VtokenMinting::token_unlock_ledger(KSM, 1), None);
		assert_eq!(VtokenMinting::token_pool(KSM), 1200);
		assert_eq!(VtokenMinting::token_to_add(KSM), 500);
		assert_eq!(VtokenMinting::currency_unlocking_total(), 100); // 200 + 100 - 200
		let (entrance_account, _exit_account) = VtokenMinting::get_entrance_and_exit_accounts();
		assert_eq!(Tokens::free_balance(KSM, &entrance_account), 300);
	});
}

#[test]
fn hook() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_eq!(VtokenMinting::min_time_unit(KSM), TimeUnit::Era(1));
		assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(3)));
		assert_eq!(VtokenMinting::ongoing_time_unit(KSM), Some(TimeUnit::Era(3)));
		assert_ok!(VtokenMinting::set_unlock_duration(Origin::root(), KSM, TimeUnit::Era(1)));
		assert_ok!(VtokenMinting::set_hook_iteration_limit(Origin::root(), 1));
		assert_eq!(VtokenMinting::unlock_duration(KSM), Some(TimeUnit::Era(1)));
		VtokenMinting::on_initialize(100);
		VtokenMinting::on_initialize(100);
		VtokenMinting::on_initialize(100);
		VtokenMinting::on_initialize(100);
		VtokenMinting::on_initialize(100);
		assert_eq!(VtokenMinting::min_time_unit(KSM), TimeUnit::Era(4));
		assert_ok!(VtokenMinting::increase_token_pool(KSM, 1000));
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 200));
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 100));
		assert_ok!(VtokenMinting::redeem(Some(BOB).into(), vKSM, 200));
		assert_ok!(VtokenMinting::redeem(Some(BOB).into(), vKSM, 100));
		assert_eq!(VtokenMinting::token_to_add(KSM), 300);
		assert_eq!(VtokenMinting::currency_unlocking_total(), 300); // 200 + 100
		assert_eq!(VtokenMinting::token_to_deduct(KSM), 300);
		assert_noop!(
			VtokenMinting::rebond(Some(BOB).into(), KSM, 100),
			Error::<Runtime>::InvalidRebondToken
		);
		assert_ok!(VtokenMinting::add_support_rebond_token(Origin::root(), KSM));
		let (entrance_account, _exit_account) = VtokenMinting::get_entrance_and_exit_accounts();
		assert_eq!(Tokens::free_balance(KSM, &entrance_account), 300);
		VtokenMinting::on_initialize(100);
		assert_eq!(VtokenMinting::min_time_unit(KSM), TimeUnit::Era(4));
		VtokenMinting::on_initialize(100);
		assert_eq!(VtokenMinting::token_unlock_ledger(KSM, 0), None);
		assert_eq!(VtokenMinting::token_unlock_ledger(KSM, 1), None);
		assert_eq!(VtokenMinting::time_unit_unlock_ledger(TimeUnit::Era(4), KSM), None);
		assert_eq!(VtokenMinting::time_unit_unlock_ledger(TimeUnit::Era(5), KSM), None);
		assert_eq!(VtokenMinting::user_unlock_ledger(BOB, KSM), None);
		assert_eq!(VtokenMinting::token_pool(KSM), 1000);
		assert_eq!(Tokens::free_balance(KSM, &entrance_account), 0);
		assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(5)));
		VtokenMinting::on_initialize(100);
		VtokenMinting::on_initialize(0);
		VtokenMinting::on_initialize(1);
		assert_eq!(VtokenMinting::min_time_unit(KSM), TimeUnit::Era(6));
		assert_eq!(VtokenMinting::token_to_add(KSM), 0);
		assert_eq!(VtokenMinting::token_to_deduct(KSM), 0);
		assert_eq!(VtokenMinting::currency_unlocking_total(), 0);
	});
}

#[test]
fn rebond_by_unlock_id() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_ok!(VtokenMinting::set_unlock_duration(Origin::root(), KSM, TimeUnit::Era(0)));
		assert_ok!(VtokenMinting::increase_token_pool(KSM, 1000));
		assert_ok!(VtokenMinting::update_ongoing_time_unit(KSM, TimeUnit::Era(1)));
		let mut ledger_list_origin = BoundedVec::default();
		assert_ok!(ledger_list_origin.try_push(1));
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 200));
		assert_ok!(VtokenMinting::mint(Some(BOB).into(), KSM, 100));
		assert_ok!(VtokenMinting::redeem(Some(BOB).into(), vKSM, 200));
		assert_ok!(VtokenMinting::redeem(Some(BOB).into(), vKSM, 100));
		assert_eq!(VtokenMinting::token_pool(KSM), 1000);
		assert_noop!(
			VtokenMinting::rebond_by_unlock_id(Some(BOB).into(), KSM, 0),
			Error::<Runtime>::InvalidRebondToken
		);
		assert_ok!(VtokenMinting::add_support_rebond_token(Origin::root(), KSM));
		assert_ok!(VtokenMinting::rebond_by_unlock_id(Some(BOB).into(), KSM, 0));
		assert_eq!(
			VtokenMinting::time_unit_unlock_ledger(TimeUnit::Era(1), KSM),
			Some((100, ledger_list_origin.clone(), KSM))
		);
		assert_eq!(
			VtokenMinting::user_unlock_ledger(BOB, KSM),
			Some((100, ledger_list_origin.clone()))
		);
		assert_eq!(VtokenMinting::token_unlock_ledger(KSM, 0), None);
		assert_eq!(VtokenMinting::token_unlock_ledger(KSM, 1), Some((BOB, 100, TimeUnit::Era(1))));
		assert_eq!(VtokenMinting::token_pool(KSM), 1200);
		assert_eq!(VtokenMinting::token_to_add(KSM), 500);
		assert_eq!(VtokenMinting::currency_unlocking_total(), 100); // 200 + 100 - 200
		let (entrance_account, _exit_account) = VtokenMinting::get_entrance_and_exit_accounts();
		assert_eq!(Tokens::free_balance(KSM, &entrance_account), 300);
	});
}

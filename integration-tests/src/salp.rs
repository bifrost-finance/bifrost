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

use crate::{integration_tests::*, kusama_test_net::*};
use bifrost_kusama_runtime::MinContribution;
use bifrost_salp::{Error, FundStatus};
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use xcm_emulator::TestExt;

#[test]
fn create_fund_should_work() {
	SalpTest::execute_with(|| {
		assert_ok!(Salp::create(
			RawOrigin::Root.into(),
			3_000,
			100_000_000_000,
			1,
			SlotLength::get()
		));
		assert_ok!(Salp::funds(3_000).ok_or(()));
		assert_eq!(Salp::current_trie_index(), 1);
	});
}

#[test]
fn edit_fund_should_work() {
	SalpTest::execute_with(|| {
		assert_ok!(Salp::create(
			RawOrigin::Root.into(),
			3_000,
			100_000_000_000,
			1,
			SlotLength::get()
		));

		assert_ok!(Salp::edit(
			RawOrigin::Root.into(),
			3_000,
			100_000_000_000,
			150,
			2,
			SlotLength::get() + 1,
			Some(FundStatus::Ongoing)
		));
	});
}

#[test]
fn contribute_should_work() {
	SalpTest::execute_with(|| {
		assert_ok!(Salp::create(
			RawOrigin::Root.into(),
			3_000,
			1000_000_000_000,
			1,
			SlotLength::get()
		));
		assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 100_000_000_000));
		assert!(Salp::funds(3_000).is_some());
	});
}

#[test]
fn double_contribute_should_work() {
	SalpTest::execute_with(|| {
		assert_ok!(Salp::create(
			RawOrigin::Root.into(),
			3_000,
			1000_000_000_000,
			1,
			SlotLength::get()
		));
		assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 100_000_000_000));
		assert!(Salp::funds(3_000).is_some());

		assert_ok!(Salp::confirm_contribute(
			Origin::signed(AccountId::new(ALICE)),
			AccountId::new(BOB),
			3_000,
			true,
			CONTRIBUTON_INDEX
		));

		assert_noop!(
			Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 1000_000_000_001),
			Error::<Runtime>::CapExceeded
		);

		assert_noop!(
			Salp::contribute(
				Origin::signed(AccountId::new(BOB)),
				3_000,
				MinContribution::get() - 1
			),
			Error::<Runtime>::ContributionTooSmall
		);

		assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 100_000_000_000));
	});
}

#[test]
fn withdraw_should_work() {
	SalpTest::execute_with(|| {
		assert_ok!(Salp::create(
			RawOrigin::Root.into(),
			3_000,
			1000_000_000_000,
			1,
			SlotLength::get()
		));
		assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 100_000_000_000));
		assert_ok!(Salp::confirm_contribute(
			Origin::signed(AccountId::new(ALICE)),
			AccountId::new(BOB),
			3_000,
			true,
			CONTRIBUTON_INDEX
		));
		assert_ok!(Salp::fund_success(RawOrigin::Root.into(), 3_000));
		assert_ok!(Salp::fund_retire(RawOrigin::Root.into(), 3_000));
		assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));
	});
}

#[test]
fn refund_should_work() {
	SalpTest::execute_with(|| {
		assert_ok!(Salp::create(
			RawOrigin::Root.into(),
			3_000,
			1000_000_000_000,
			1,
			SlotLength::get()
		));
		assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 100_000_000_000));
		assert_ok!(Salp::confirm_contribute(
			Origin::signed(AccountId::new(ALICE)),
			AccountId::new(BOB),
			3_000,
			true,
			CONTRIBUTON_INDEX
		));
		assert_ok!(Salp::fund_fail(RawOrigin::Root.into(), 3_000));
		assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));
		assert_ok!(Salp::refund(
			Origin::signed(AccountId::new(BOB)),
			3_000,
			1,
			SlotLength::get(),
			100_000_000_000
		));

		assert_noop!(
			Salp::refund(Origin::signed(AccountId::new(BOB)), 3_000, 1, SlotLength::get(), 100),
			Error::<Runtime>::NotEnoughBalanceInFund
		);
	});
}

#[test]
fn dissolve_should_work() {
	SalpTest::execute_with(|| {
		assert_ok!(Salp::create(
			RawOrigin::Root.into(),
			3_000,
			1000_000_000_000,
			1,
			SlotLength::get()
		));
		assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 100_000_000_000));
		assert_ok!(Salp::confirm_contribute(
			Origin::signed(AccountId::new(ALICE)),
			AccountId::new(BOB),
			3_000,
			true,
			CONTRIBUTON_INDEX
		));
		assert_ok!(Salp::fund_success(RawOrigin::Root.into(), 3_000));
		assert_ok!(Salp::fund_retire(RawOrigin::Root.into(), 3_000));
		assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));
		assert_ok!(Salp::fund_end(RawOrigin::Root.into(), 3_000));

		assert_ok!(Salp::dissolve(RawOrigin::Root.into(), 3_000));

		assert!(Salp::funds(3_000).is_none());
	});
}

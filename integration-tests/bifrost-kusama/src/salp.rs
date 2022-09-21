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

use crate::{kusama_integration_tests::*, kusama_test_net::*};
use bifrost_kusama_runtime::{LeasePeriod, MinContribution};
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

#[test]
fn redeem_should_work() {
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
		assert_ok!(Salp::unlock(Origin::signed(AccountId::new(ALICE)), AccountId::new(BOB), 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem);

		assert_ok!(Salp::fund_retire(RawOrigin::Root.into(), 3_000));
		assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 3_000));
		let vs_token = <Runtime as bifrost_salp::Config>::CurrencyIdConversion::convert_to_vstoken(
			RelayCurrencyId::get(),
		)
		.unwrap();
		let vs_bond = <Runtime as bifrost_salp::Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			3_000,
			1,
			SlotLength::get(),
		)
		.unwrap();

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(
			vs_token,
			&AccountId::new(BOB),
			&AccountId::new(CATHI),
			500_000_000
		));
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(
			vs_bond,
			&AccountId::new(BOB),
			&AccountId::new(CATHI),
			500_000_000
		));

		assert_ok!(Salp::redeem(Origin::signed(AccountId::new(BOB)), 3_000, 500_000_000));
		assert_ok!(Salp::redeem(Origin::signed(AccountId::new(CATHI)), 3_000, 500_000_000));
	});
}

#[test]
fn redeem_with_speical_vsbond_should_work() {
	SalpTest::execute_with(|| {
		assert_ok!(Salp::create(RawOrigin::Root.into(), 2001, 1000_000_000_000, 13, 20));
		assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 2001, 100_000_000_000));
		assert_ok!(Salp::confirm_contribute(
			Origin::signed(AccountId::new(ALICE)),
			AccountId::new(BOB),
			2001,
			true,
			CONTRIBUTON_INDEX
		));

		assert_ok!(Salp::fund_success(RawOrigin::Root.into(), 2001));
		assert_ok!(Salp::unlock(Origin::signed(AccountId::new(ALICE)), AccountId::new(BOB), 2001));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem);

		assert_ok!(Salp::fund_retire(RawOrigin::Root.into(), 2001));
		assert_ok!(Salp::withdraw(RawOrigin::Root.into(), 2001));

		let vs_token = <Runtime as bifrost_salp::Config>::CurrencyIdConversion::convert_to_vstoken(
			RelayCurrencyId::get(),
		)
		.unwrap();
		let vs_bond = <Runtime as bifrost_salp::Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			2001,
			13,
			20,
		)
		.unwrap();

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(
			vs_token,
			&AccountId::new(BOB),
			&AccountId::new(CATHI),
			500_000_000
		));
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(
			vs_bond,
			&AccountId::new(BOB),
			&AccountId::new(CATHI),
			500_000_000
		));
		assert_ok!(Salp::redeem(Origin::signed(AccountId::new(BOB)), 2001, 500_000_000));
		assert_ok!(Salp::redeem(Origin::signed(AccountId::new(CATHI)), 2001, 500_000_000));
	});
}

#[test]
fn batch_unlock_should_work() {
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
		assert_ok!(Salp::batch_unlock(Origin::signed(AccountId::new(ALICE)), 3_000));
	})
}

#[test]
fn unlock_when_fund_ongoing_should_work() {
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
		assert_ok!(Salp::unlock(Origin::signed(AccountId::new(BOB)), AccountId::new(BOB), 3_000));
	});
}

#[test]
fn set_confirmor_should_work() {
	SalpTest::execute_with(|| {
		assert_ok!(Salp::create(
			RawOrigin::Root.into(),
			3_000,
			1000_000_000_000,
			1,
			SlotLength::get()
		));
		assert_ok!(Salp::contribute(Origin::signed(AccountId::new(BOB)), 3_000, 100_000_000_000));
		assert_noop!(
			Salp::confirm_contribute(
				Origin::signed(AccountId::new(BOB)),
				AccountId::new(BOB),
				3_000,
				true,
				CONTRIBUTON_INDEX
			),
			DispatchError::BadOrigin,
		);
		assert_ok!(Salp::set_multisig_confirm_account(RawOrigin::Root.into(), AccountId::new(BOB)));
		assert_ok!(Salp::confirm_contribute(
			Origin::signed(AccountId::new(BOB)),
			AccountId::new(BOB),
			3_000,
			true,
			CONTRIBUTON_INDEX
		));
	});
}

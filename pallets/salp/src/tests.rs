// This file is part of Bifrost.

// Copyright (C) Liebi Technologies PTE. LTD.
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

use crate::{mock::*, Error, FundStatus, *};
use bifrost_primitives::{
	BuybackPalletId, CurrencyId, TokenSymbol, TryConvertFrom, KSM, VKSM, VSKSM,
};
use bifrost_xcm_interface::SalpHelper;
use frame_support::{assert_noop, assert_ok};
use frame_system::pallet_prelude::BlockNumberFor;
use orml_traits::MultiCurrency;
use sp_runtime::{traits::AccountIdConversion, DispatchError};
use zenlink_protocol::AssetId;

#[test]
fn set_fund_retire_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));

		// Check status
		let fund = Funds::<Test>::get(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Retired);
	});
}

#[test]
fn set_fund_retire_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_noop!(Salp::fund_retire(RuntimeOrigin::none(), 3_000), DispatchError::BadOrigin);
	});
}

#[test]
fn set_fund_retire_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_noop!(Salp::fund_retire(Some(ALICE).into(), 4_000), Error::<Test>::InvalidParaId);
	});
}

#[test]
fn set_fund_retire_with_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(
			Salp::fund_retire(Some(ALICE).into(), 3_000),
			Error::<Test>::InvalidFundStatus
		);
	});
}

#[test]
fn set_fund_end_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_end(Some(ALICE).into(), 3_000));

		// Check storage
		let fund = Funds::<Test>::get(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::End);
	});
}

#[test]
fn set_fund_end_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::fund_end(RuntimeOrigin::none(), 3_000), DispatchError::BadOrigin);
	});
}

#[test]
fn set_fund_end_with_wrong_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::fund_end(Some(ALICE).into(), 4_000), Error::<Test>::InvalidParaId);
	});
}

#[test]
fn set_fund_end_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::fund_end(Some(ALICE).into(), 3_000), Error::<Test>::InvalidFundStatus);
	});
}

#[test]
fn withdraw_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		let fund = Funds::<Test>::get(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::RedeemWithdrew);

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 4_000, 100));
		Salp::bind_query_id_and_contribution(0, 4_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));

		let fund = Funds::<Test>::get(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::RefundWithdrew);
	});
}

#[test]
fn withdraw_when_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		let fund = Funds::<Test>::get(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::RedeemWithdrew);

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 4_000, 100));
		Salp::bind_query_id_and_contribution(0, 4_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));

		let fund = Funds::<Test>::get(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::RefundWithdrew);
	});
}

#[test]
fn double_withdraw_same_fund_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_noop!(Salp::withdraw(Some(ALICE).into(), 3_000), Error::<Test>::InvalidFundStatus);

		let fund = Funds::<Test>::get(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::RedeemWithdrew);

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 4_000, 100));
		Salp::bind_query_id_and_contribution(0, 4_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_noop!(Salp::withdraw(Some(ALICE).into(), 4_000), Error::<Test>::InvalidFundStatus);

		let fund = Funds::<Test>::get(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::RefundWithdrew);
	});
}

#[test]
fn double_withdraw_same_fund_when_one_of_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_noop!(Salp::withdraw(Some(ALICE).into(), 3_000), Error::<Test>::InvalidFundStatus);
	});
}

#[test]
fn withdraw_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::withdraw(RuntimeOrigin::none(), 3_000), DispatchError::BadOrigin);
	});
}

#[test]
fn withdraw_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::withdraw(Some(ALICE).into(), 4_000), Error::<Test>::InvalidParaId);
	});
}

#[test]
fn withdraw_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::withdraw(Some(ALICE).into(), 3_000), Error::<Test>::InvalidFundStatus);
	});
}

#[test]
fn withdraw_with_when_ump_wrong_should_fail() {
	// TODO: Require an solution to settle with parallel test workflow
}

#[test]
fn refund_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 100));

		let vs_token =
			<Test as Config>::CurrencyIdConversion::convert_to_vstoken(RelayCurrencyId::get())
				.unwrap();
		let vs_bond = <Test as Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			3_000,
			1,
			SlotLength::get(),
		)
		.unwrap();
		assert_eq!(Tokens::accounts(BRUCE, vs_token).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_token).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_token).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).free, INIT_BALANCE);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).reserved, 0);
	});
}

#[test]
fn refund_when_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 100));

		let vs_token =
			<Test as Config>::CurrencyIdConversion::convert_to_vstoken(RelayCurrencyId::get())
				.unwrap();
		let vs_bond = <Test as Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			3_000,
			1,
			SlotLength::get(),
		)
		.unwrap();
		assert_eq!(Tokens::accounts(BRUCE, vs_token).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_token).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_token).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).reserved, 0);
	});
}

#[test]
fn double_refund_when_one_of_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 100));
		assert_noop!(
			Salp::refund(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 100),
			Error::<Test>::NotEnoughBalanceInFund
		);
	});
}

#[test]
fn refund_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		assert_noop!(
			Salp::refund(RuntimeOrigin::root(), 3_000, 1, SlotLength::get(), 100),
			DispatchError::BadOrigin
		);
		assert_noop!(
			Salp::refund(RuntimeOrigin::none(), 3_000, 1, SlotLength::get(), 100),
			DispatchError::BadOrigin
		);

		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 100));
	});
}

#[test]
fn refund_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		assert_noop!(
			Salp::refund(Some(BRUCE).into(), 4_000, 1, SlotLength::get(), 100),
			Error::<Test>::InvalidFundNotExist
		);
	});
}

#[test]
fn dissolve_should_work() {
	new_test_ext().execute_with(|| {
		let remove_times = 4;
		let contribute_account_num = remove_times * RemoveKeysLimit::get();

		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 10_000, 1, SlotLength::get()));
		for i in 0..contribute_account_num {
			let ract = AccountId::new([(i as u8); 32]);
			assert_ok!(Tokens::deposit(RelayCurrencyId::get(), &ract, 10));
			assert_ok!(Salp::contribute(Some(ract.clone()).into(), 3_000, 10));
			Salp::bind_query_id_and_contribution(0, 3_000, ract, 10);
			assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		}
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_end(Some(ALICE).into(), 3_000));

		for _ in 0..remove_times {
			assert_ok!(Salp::dissolve(Some(ALICE).into(), 3_000));
		}

		assert!(Funds::<Test>::get(3_000).is_none());
		assert!(Salp::contribution_iterator(0).next().is_none());
	});
}

#[test]
fn dissolve_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_end(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::dissolve(RuntimeOrigin::none(), 3_000), DispatchError::BadOrigin);
	});
}

#[test]
fn dissolve_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_end(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::dissolve(Some(ALICE).into(), 4_000), Error::<Test>::InvalidParaId);
	});
}

#[test]
fn dissolve_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::dissolve(Some(ALICE).into(), 3_000), Error::<Test>::InvalidFundStatus);
	});
}

#[test]
fn redeem_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));

		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).free, INIT_BALANCE - 100);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).reserved, 0);

		assert_eq!(
			Tokens::accounts(Salp::fund_account_id(3_000), RelayCurrencyId::get()).free,
			100
		);
		assert_eq!(
			Tokens::accounts(Salp::fund_account_id(3_000), RelayCurrencyId::get()).frozen,
			0
		);
		assert_eq!(
			Tokens::accounts(Salp::fund_account_id(3_000), RelayCurrencyId::get()).reserved,
			0
		);

		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem.into());

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		let vs_token =
			<Test as Config>::CurrencyIdConversion::convert_to_vstoken(RelayCurrencyId::get())
				.unwrap();
		let vs_bond = <Test as Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			3_000,
			1,
			SlotLength::get(),
		)
		.unwrap();

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vs_token, &BRUCE, &CATHI, 50));
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vs_bond, &BRUCE, &CATHI, 50));

		assert_ok!(Salp::redeem(Some(BRUCE).into(), 3_000, 50));

		assert_eq!(Tokens::accounts(BRUCE, vs_token).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_token).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_token).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).free, INIT_BALANCE - 50);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).reserved, 0);

		assert_ok!(Salp::redeem(Some(CATHI).into(), 3_000, 50));

		assert_eq!(Tokens::accounts(CATHI, vs_token).free, 0);
		assert_eq!(Tokens::accounts(CATHI, vs_token).frozen, 0);
		assert_eq!(Tokens::accounts(CATHI, vs_token).reserved, 0);
		assert_eq!(Tokens::accounts(CATHI, vs_bond).free, 0);
		assert_eq!(Tokens::accounts(CATHI, vs_bond).frozen, 0);
		assert_eq!(Tokens::accounts(CATHI, vs_bond).reserved, 0);
		assert_eq!(Tokens::accounts(CATHI, RelayCurrencyId::get()).free, INIT_BALANCE + 50);
		assert_eq!(Tokens::accounts(CATHI, RelayCurrencyId::get()).frozen, 0);
		assert_eq!(Tokens::accounts(CATHI, RelayCurrencyId::get()).reserved, 0);
	});
}

#[test]
fn redeem_with_speical_vsbond_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 2001, 1_000, 13, 20));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 2001, 100));
		Salp::bind_query_id_and_contribution(0, 2001, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));

		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).free, INIT_BALANCE - 100);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).reserved, 0);

		assert_eq!(Tokens::accounts(Salp::fund_account_id(2001), RelayCurrencyId::get()).free, 100);
		assert_eq!(Tokens::accounts(Salp::fund_account_id(2001), RelayCurrencyId::get()).frozen, 0);
		assert_eq!(
			Tokens::accounts(Salp::fund_account_id(2001), RelayCurrencyId::get()).reserved,
			0
		);

		assert_ok!(Salp::fund_success(Some(ALICE).into(), 2001));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 2001));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem.into());

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 2001));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 2001));

		let vs_token =
			<Test as Config>::CurrencyIdConversion::convert_to_vstoken(RelayCurrencyId::get())
				.unwrap();
		let vs_bond = <Test as Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			2001,
			13,
			20,
		)
		.unwrap();

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vs_token, &BRUCE, &CATHI, 50));
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vs_bond, &BRUCE, &CATHI, 50));

		assert_ok!(Salp::redeem(Some(BRUCE).into(), 2001, 50));

		assert_eq!(Tokens::accounts(BRUCE, vs_token).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_token).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_token).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).free, INIT_BALANCE - 50);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, RelayCurrencyId::get()).reserved, 0);

		assert_ok!(Salp::redeem(Some(CATHI).into(), 2001, 50));

		assert_eq!(Tokens::accounts(CATHI, vs_token).free, 0);
		assert_eq!(Tokens::accounts(CATHI, vs_token).frozen, 0);
		assert_eq!(Tokens::accounts(CATHI, vs_token).reserved, 0);
		assert_eq!(Tokens::accounts(CATHI, vs_bond).free, 0);
		assert_eq!(Tokens::accounts(CATHI, vs_bond).frozen, 0);
		assert_eq!(Tokens::accounts(CATHI, vs_bond).reserved, 0);
		assert_eq!(Tokens::accounts(CATHI, RelayCurrencyId::get()).free, INIT_BALANCE + 50);
		assert_eq!(Tokens::accounts(CATHI, RelayCurrencyId::get()).frozen, 0);
		assert_eq!(Tokens::accounts(CATHI, RelayCurrencyId::get()).reserved, 0);
	});
}

#[test]
fn redeem_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem.into());

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::redeem(RuntimeOrigin::root(), 3_000, 50), DispatchError::BadOrigin);
		assert_noop!(Salp::redeem(RuntimeOrigin::none(), 3_000, 50), DispatchError::BadOrigin);
	});
}

#[test]
fn redeem_with_not_redeemable_vsbond_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_not_redeemable = LeasePeriod::get();
		System::set_block_number(block_not_redeemable.into());

		let vs_token =
			<Test as Config>::CurrencyIdConversion::convert_to_vstoken(RelayCurrencyId::get())
				.unwrap();
		let vs_bond = <Test as Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			3_000,
			1,
			SlotLength::get(),
		)
		.unwrap();

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vs_token, &BRUCE, &CATHI, 50));
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vs_bond, &BRUCE, &CATHI, 50));

		assert_noop!(Salp::redeem(Some(BRUCE).into(), 3_000, 50), Error::<Test>::InvalidFundStatus);

		assert_noop!(Salp::redeem(Some(CATHI).into(), 3_000, 50), Error::<Test>::InvalidFundStatus);
	});
}

#[test]
fn redeem_without_enough_vsassets_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem.into());

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		let vs_token =
			<Test as Config>::CurrencyIdConversion::convert_to_vstoken(RelayCurrencyId::get())
				.unwrap();
		let vs_bond = <Test as Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			3_000,
			1,
			SlotLength::get(),
		)
		.unwrap();

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vs_token, &BRUCE, &CATHI, 50));
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vs_bond, &BRUCE, &CATHI, 50));

		assert_noop!(
			Salp::redeem(Some(BRUCE).into(), 3_000, 60),
			Error::<Test>::NotEnoughFreeAssetsToRedeem
		);

		assert_noop!(
			Salp::redeem(Some(CATHI).into(), 3_000, 60),
			Error::<Test>::NotEnoughFreeAssetsToRedeem
		);
	});
}

#[test]
fn redeem_without_enough_balance_in_pool_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem.into());

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		// Before withdraw
		assert_noop!(
			Salp::redeem(Some(BRUCE).into(), 3_000, 200),
			Error::<Test>::NotEnoughBalanceInRedeemPool
		);
	});
}

#[test]
fn redeem_with_when_ump_wrong_should_fail() {
	// TODO: Require an solution to settle with parallel test workflow
}

#[test]
fn release_from_redeem_to_bancor_should_work() {
	fn run_to_block(n: BlockNumber) {
		use frame_support::traits::Hooks;
		while System::block_number() <= n.into() {
			Salp::on_finalize(System::block_number());
			System::on_finalize(System::block_number());
			System::set_block_number(System::block_number() + 1);
			System::on_initialize(System::block_number());
			Salp::on_initialize(System::block_number());
		}
	}

	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));

		run_to_block(ReleaseCycle::get());

		// TODO: Check the balance of bancor(Waiting Bancor to Support..)
	});
}

// Utilities Test
#[test]
fn check_next_trie_index() {
	new_test_ext().execute_with(|| {
		for i in 0..100 {
			assert_eq!(CurrentTrieIndex::<Test>::get(), i);
			assert_ok!(Salp::next_trie_index());
		}
	});
}

#[test]
fn unlock_when_fund_ongoing_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		let vs_token =
			<Test as Config>::CurrencyIdConversion::convert_to_vstoken(RelayCurrencyId::get())
				.unwrap();
		let vs_bond = <Test as Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			3_000,
			1,
			SlotLength::get(),
		)
		.unwrap();

		assert_eq!(Tokens::accounts(BRUCE, vs_token).free, 100);
		assert_eq!(Tokens::accounts(BRUCE, vs_token).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_token).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).free, 100);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond).reserved, 0);
	});
}

#[test]
fn refund_meanwhile_issue_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		let vs_bond_old = <Test as Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			3_000,
			1,
			SlotLength::get(),
		)
		.unwrap();
		assert_eq!(Tokens::accounts(BRUCE, vs_bond_old).free, 100);
		assert_eq!(
			<Test as Config>::CurrencyIdRegister::check_vsbond_registered(
				TokenSymbol::KSM,
				3_000,
				2,
				SlotLength::get() + 1
			),
			false
		);
		assert_ok!(Salp::continue_fund(Some(ALICE).into(), 3_000, 2, SlotLength::get() + 1));
		assert_eq!(
			<Test as Config>::CurrencyIdRegister::check_vsbond_registered(
				TokenSymbol::KSM,
				3_000,
				2,
				SlotLength::get() + 1
			),
			true
		);
		let old_fund = FailedFundsToRefund::<Test>::get((3_000, 1, SlotLength::get())).unwrap();
		assert_eq!(old_fund.first_slot, 1);
		assert_eq!(old_fund.raised, 100);
		let mut new_fund = Funds::<Test>::get(3_000).unwrap();
		assert_eq!(new_fund.first_slot, 2);
		assert_eq!(new_fund.raised, 100);
		let vs_bond_new = <Test as Config>::CurrencyIdConversion::convert_to_vsbond(
			RelayCurrencyId::get(),
			3_000,
			2,
			SlotLength::get() + 1,
		)
		.unwrap();
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		new_fund = Funds::<Test>::get(3_000).unwrap();
		assert_eq!(new_fund.raised, 200);
		assert_eq!(Tokens::accounts(BRUCE, vs_bond_new).free, 100);
		// refund from old failed fund should success
		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 50));
		assert_eq!(Tokens::accounts(BRUCE, vs_bond_old).free, 50);
		// refund from new fund should failed
		assert_noop!(
			Salp::refund(Some(BRUCE).into(), 3_000, 2, SlotLength::get() + 1, 100),
			Error::<Test>::InvalidRefund,
		);
		// refund from not exist fund should failed
		assert_noop!(
			Salp::refund(Some(BRUCE).into(), 4_000, 2, SlotLength::get() + 1, 100),
			Error::<Test>::InvalidFundNotExist,
		);
		// after dissolve failed fund refund from old should fail
		assert_ok!(Salp::dissolve_refunded(Some(ALICE).into(), 3_000, 1, SlotLength::get()));
		assert_noop!(
			Salp::refund(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 50),
			Error::<Test>::InvalidRefund,
		);
		// after new fund finally success redeem should success
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_eq!(
			Tokens::accounts(Salp::fund_account_id(3_000), RelayCurrencyId::get()).free,
			150
		);
		assert_ok!(Salp::redeem(Some(BRUCE).into(), 3_000, 50));
		assert_eq!(Tokens::accounts(BRUCE, vs_bond_new).free, 50);
		// after fund dissolved redeem should fail
		assert_ok!(Salp::fund_end(Some(ALICE).into(), 3_000));
		assert_eq!(
			Tokens::accounts(Salp::fund_account_id(3_000), RelayCurrencyId::get()).free,
			100
		);
		assert_ok!(Salp::dissolve(Some(ALICE).into(), 3_000));
		assert_eq!(Tokens::accounts(Salp::fund_account_id(3_000), RelayCurrencyId::get()).free, 0);
		let treasury_account: AccountId = TreasuryAccount::get();
		assert_eq!(Tokens::accounts(treasury_account, RelayCurrencyId::get()).free, 25);
		let buyback_account: AccountId = BuybackPalletId::get().into_account_truncating();
		assert_eq!(Tokens::accounts(buyback_account.clone(), RelayCurrencyId::get()).free, 75);
		assert_noop!(Salp::redeem(Some(BRUCE).into(), 3_000, 50), Error::<Test>::InvalidParaId);

		let para_id = 2001u32;
		let asset_0_currency_id: AssetId =
			AssetId::try_convert_from(RelayCurrencyId::get(), para_id).unwrap();
		let asset_1_currency_id: AssetId =
			AssetId::try_convert_from(CurrencyId::VSToken(TokenSymbol::KSM), para_id).unwrap();
		assert_ok!(ZenlinkProtocol::create_pair(
			RuntimeOrigin::root(),
			asset_0_currency_id,
			asset_1_currency_id,
			ALICE
		));
		let deadline: BlockNumberFor<Test> =
			<frame_system::Pallet<Test>>::block_number() + BlockNumberFor::<Test>::from(100u32);
		assert_ok!(ZenlinkProtocol::add_liquidity(
			RuntimeOrigin::signed(ALICE),
			asset_0_currency_id,
			asset_1_currency_id,
			1000,
			2200,
			1,
			1,
			deadline
		));

		let amounts = vec![1_000u128, 1_000u128];
		assert_ok!(StablePool::create_pool(
			RuntimeOrigin::signed(ALICE),
			vec![KSM, VKSM],
			vec![1u128.into(), 1u128.into()],
			0u128.into(),
			0u128.into(),
			0u128.into(),
			220u128.into(),
			ALICE,
			ALICE,
			1000000000000u128.into()
		));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::signed(ALICE),
			0,
			vec![(KSM, (1, 1)), (VKSM, (10, 11))]
		));
		assert_ok!(StablePool::add_liquidity(
			RuntimeOrigin::signed(ALICE).into(),
			0,
			amounts.clone(),
			0
		));
		assert_ok!(StablePool::create_pool(
			RuntimeOrigin::signed(ALICE),
			vec![KSM, VSKSM],
			vec![1u128.into(), 1u128.into()],
			0u128.into(),
			0u128.into(),
			0u128.into(),
			220u128.into(),
			ALICE,
			ALICE,
			1000000000000u128.into()
		));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::signed(ALICE),
			1,
			vec![(VSKSM, (1, 1)), (KSM, (10, 30))]
		));
		assert_ok!(StablePool::add_liquidity(
			RuntimeOrigin::signed(ALICE).into(),
			1,
			amounts.clone(),
			0
		));
		assert_ok!(StablePool::create_pool(
			RuntimeOrigin::signed(ALICE),
			vec![VSKSM, VKSM],
			vec![1u128.into(), 1u128.into()],
			0u128.into(),
			0u128.into(),
			0u128.into(),
			220u128.into(),
			ALICE,
			ALICE,
			1000000000000u128.into()
		));
		assert_ok!(StablePool::edit_token_rate(
			RuntimeOrigin::signed(ALICE),
			2,
			vec![(VSKSM, (1, 1)), (VKSM, (10, 11))]
		));
		assert_ok!(StablePool::add_liquidity(RuntimeOrigin::signed(ALICE).into(), 2, amounts, 0));

		assert_ok!(VtokenMinting::set_minimum_mint(RuntimeOrigin::signed(ALICE), KSM, 0));
		assert_ok!(VtokenMinting::mint(
			Some(ALICE).into(),
			KSM,
			2_000,
			BoundedVec::default(),
			None
		));
		assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), ALICE, VKSM, 0, 0));
		assert_noop!(
			Salp::buyback_vstoken_by_stable_pool(Some(ALICE).into(), 0, VKSM, 70),
			Error::<Test>::ArgumentsError
		);
		assert_noop!(
			Salp::buyback_vstoken_by_stable_pool(Some(ALICE).into(), 1, KSM, 100),
			orml_tokens::Error::<Test>::BalanceTooLow
		);
		let token_value = VtokenMinting::get_v_currency_amount_by_currency_amount(KSM, VKSM, 100);
		assert_eq!(token_value, Ok(100));
		assert_eq!(Tokens::free_balance(KSM, &ALICE), 95000);
		assert_ok!(Tokens::set_balance(RuntimeOrigin::root(), buyback_account, KSM, 100, 0));

		assert_ok!(Salp::buyback_vstoken_by_stable_pool(Some(BRUCE).into(), 1, KSM, 100));
		assert_eq!(Tokens::free_balance(VSKSM, &BRUCE), 100);
	});
}

#[test]
fn edit_fund_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		Salp::bind_query_id_and_contribution(0, 3_000, BRUCE, 100);
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), 0, true,));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		let mut fund = Funds::<Test>::get(3_000).unwrap();
		assert_eq!(fund.raised, 100);

		assert_ok!(Salp::edit(
			Some(ALICE).into(),
			3_000,
			1_000,
			150,
			2,
			SlotLength::get() + 1,
			Some(FundStatus::Ongoing)
		));
		fund = Funds::<Test>::get(3_000).unwrap();
		assert_eq!(fund.raised, 150);
		assert_eq!(fund.status, FundStatus::Ongoing);
	});
}

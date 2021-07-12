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

// Ensure we're `no_std` when compiling for Wasm.

#![cfg(test)]

use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};

use crate::{mock::*, Error, FundStatus};

#[test]
fn create_fund_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::funds(3_000).ok_or(()));
		assert_eq!(Salp::current_trie_index(), 1);
	});
}

#[test]
fn create_fund_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Salp::create(Origin::root(), 3_000, 1_000, 1, SlotLength::get()),
			DispatchError::BadOrigin,
		);

		assert_noop!(
			Salp::create(Origin::none(), 3_000, 1_000, 1, SlotLength::get()),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn create_fund_existed_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));

		assert_noop!(
			Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()),
			Error::<Test>::FundAlreadyCreated,
		);
	});
}

#[test]
fn create_fund_exceed_slot_limit_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Salp::create(Some(ALICE).into(), 3_000, 1_000, 0, SlotLength::get()),
			Error::<Test>::LastSlotTooFarInFuture,
		);
	});
}

#[test]
fn create_fund_first_slot_bigger_than_last_slot_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Salp::create(Some(ALICE).into(), 3_000, 1_000, SlotLength::get(), 0),
			Error::<Test>::LastSlotBeforeFirstSlot,
		);
	});
}

#[test]
fn set_fund_success_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));

		// Check status
		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Success);
	});
}

#[test]
fn set_fund_success_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(Salp::fund_success(Origin::root(), 3_000), DispatchError::BadOrigin);
		assert_noop!(Salp::fund_success(Origin::none(), 3_000), DispatchError::BadOrigin);
		assert_noop!(
			Salp::fund_success(Some(BRUCE).into(), 3_000),
			Error::<Test>::UnauthorizedAccount
		);
	})
}

#[test]
fn set_fund_success_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(Salp::fund_success(Some(ALICE).into(), 4_000), Error::<Test>::InvalidParaId);
	});
}

#[test]
fn set_fund_success_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_noop!(
			Salp::fund_success(Some(ALICE).into(), 3_000),
			Error::<Test>::InvalidFundStatus
		);
	});
}

#[test]
fn set_fund_fail_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));

		// Check status
		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Failed);
	});
}

#[test]
fn set_fund_fail_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(Salp::fund_fail(Origin::root(), 3_000), DispatchError::BadOrigin);
		assert_noop!(Salp::fund_fail(Origin::none(), 3_000), DispatchError::BadOrigin);
		assert_noop!(
			Salp::fund_fail(Some(BRUCE).into(), 3_000),
			Error::<Test>::UnauthorizedAccount
		);
	});
}

#[test]
fn set_fund_fail_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(Salp::fund_fail(Some(ALICE).into(), 4_000), Error::<Test>::InvalidParaId);
	});
}

#[test]
fn set_fund_fail_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_noop!(Salp::fund_fail(Some(ALICE).into(), 3_000), Error::<Test>::InvalidFundStatus);
	});
}

#[test]
fn set_fund_retire_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));

		// Check status
		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Retired);
	});
}

#[test]
fn set_fund_retire_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_noop!(Salp::fund_retire(Origin::root(), 3_000), DispatchError::BadOrigin);
		assert_noop!(Salp::fund_retire(Origin::none(), 3_000), DispatchError::BadOrigin);
		assert_noop!(
			Salp::fund_retire(Some(BRUCE).into(), 3_000),
			Error::<Test>::UnauthorizedAccount
		);
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
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_ok!(Salp::fund_end(Some(ALICE).into(), 3_000));

		// Check storage
		let fund = Salp::funds(3_000).unwrap();
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
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		assert_noop!(Salp::fund_end(Origin::root(), 3_000), DispatchError::BadOrigin);
		assert_noop!(Salp::fund_end(Origin::none(), 3_000), DispatchError::BadOrigin);
		assert_noop!(Salp::fund_end(Some(BRUCE).into(), 3_000), Error::<Test>::UnauthorizedAccount);
	});
}

#[test]
fn set_fund_end_with_wrong_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

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
fn contribute_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));

		let fund = Salp::funds(3_000).unwrap();
		let (contributed, contributing) = Salp::contribution_get(fund.trie_index, &BRUCE);
		assert_eq!(fund.raised, 100);
		assert_eq!(contributed, 100);
		assert_eq!(contributing, 0);

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());
		assert_eq!(Tokens::accounts(BRUCE, vsToken).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).reserved, 100);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).reserved, 100);
	});
}

#[test]
fn double_contribute_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));

		// Check the contribution
		let fund = Salp::funds(3_000).unwrap();
		let (contributed, contributing) = Salp::contribution_get(fund.trie_index, &BRUCE);
		assert_eq!(fund.raised, 200);
		assert_eq!(contributed, 200);
		assert_eq!(contributing, 0);

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());
		assert_eq!(Tokens::accounts(BRUCE, vsToken).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).reserved, 200);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).reserved, 200);
	});
}

#[test]
fn contribute_when_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, false));

		let fund = Salp::funds(3_000).unwrap();
		let (contributed, contributing) = Salp::contribution_get(fund.trie_index, &BRUCE);
		assert_eq!(fund.raised, 0);
		assert_eq!(contributed, 0);
		assert_eq!(contributing, 0);

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());
		assert_eq!(Tokens::accounts(BRUCE, vsToken).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).reserved, 0);
	});
}

#[test]
fn confirm_contribute_later_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));

		let fund = Salp::funds(3_000).unwrap();
		let (contributed, contributing) = Salp::contribution_get(fund.trie_index, &BRUCE);
		assert_eq!(fund.raised, 100);
		assert_eq!(contributed, 100);
		assert_eq!(contributing, 0);

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());
		assert_eq!(Tokens::accounts(BRUCE, vsToken).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).reserved, 100);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).reserved, 100);
	});
}

#[test]
fn contribute_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(Salp::contribute(Origin::root(), 3_000, 100), DispatchError::BadOrigin);
		assert_noop!(Salp::contribute(Origin::none(), 3_000, 100), DispatchError::BadOrigin);

		assert_noop!(
			Salp::confirm_contribute(Origin::root(), BRUCE, 3000, true),
			DispatchError::BadOrigin,
		);
		assert_noop!(
			Salp::confirm_contribute(Origin::none(), BRUCE, 3000, true),
			DispatchError::BadOrigin,
		);
		assert_noop!(
			Salp::confirm_contribute(Some(BRUCE).into(), BRUCE, 3000, true),
			Error::<Test>::UnauthorizedAccount,
		);
	});
}

#[test]
fn contribute_with_low_contribution_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 3_000, MinContribution::get() - 1),
			Error::<Test>::ContributionTooSmall
		);
	})
}

#[test]
fn contribute_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 4_000, 100),
			Error::<Test>::InvalidParaId
		);
	})
}

#[test]
fn contribute_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000,));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 3_000, 100),
			Error::<Test>::InvalidFundStatus
		);
	})
}

#[test]
fn contribute_exceed_cap_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 3_000, 1_001),
			Error::<Test>::CapExceeded
		);
	})
}

#[test]
fn contribute_when_contributing_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_noop!(Salp::contribute(Some(BRUCE).into(), 3_000, 100), Error::<Test>::Contributing,);
	})
}

#[test]
fn contribute_with_when_ump_wrong_should_fail() {
	// TODO: Require an solution to settle with parallel test workflow
}

#[test]
fn withdraw_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Withdrew);

		// TODO: Check the balance of `redeem-pool`

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 4_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 4_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, true));

		let fund = Salp::funds(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::Withdrew);

		assert_eq!(Salp::refund_pool(), 100);
	});
}

#[test]
fn withdraw_when_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, false));

		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Retired);

		// TODO: Check the balance of `redeem-pool`

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 4_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 4_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, false));

		let fund = Salp::funds(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::Failed);

		assert_eq!(Salp::refund_pool(), 0);
	});
}

#[test]
fn double_withdraw_same_fund_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_noop!(Salp::withdraw(Some(ALICE).into(), 3_000), Error::<Test>::InvalidFundStatus);

		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Withdrew);

		// TODO: Check the balance of `redeem-pool`

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 4_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 4_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, true));
		assert_noop!(Salp::withdraw(Some(ALICE).into(), 4_000), Error::<Test>::InvalidFundStatus);

		let fund = Salp::funds(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::Withdrew);

		assert_eq!(Salp::refund_pool(), 100);
	});
}

#[test]
fn double_withdraw_same_fund_when_one_of_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, false));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Withdrew);

		// TODO: Check the balance of `redeem-pool`

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 4_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 4_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, false));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, true));

		let fund = Salp::funds(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::Withdrew);

		assert_eq!(Salp::refund_pool(), 100);
	});
}

#[test]
fn withdraw_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::withdraw(Origin::root(), 3_000), DispatchError::BadOrigin);
		assert_noop!(Salp::withdraw(Origin::none(), 3_000), DispatchError::BadOrigin);
		assert_noop!(Salp::withdraw(Some(BRUCE).into(), 3_000), Error::<Test>::UnauthorizedAccount);
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

// TODO: Add the unit-tests of `refund`

#[test]
fn dissolve_should_work() {
	new_test_ext().execute_with(|| {
		let remove_times = 4;
		let contribute_account_num = remove_times * RemoveKeysLimit::get();

		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 10_000, 1, SlotLength::get()));
		for i in 0 .. contribute_account_num {
			let ract = AccountId::new([(i as u8); 32]);
			assert_ok!(Salp::contribute(Some(ract.clone()).into(), 3_000, 10));
			assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), ract, 3_000, true));
		}
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_ok!(Salp::fund_end(Some(ALICE).into(), 3_000));

		// TODO: Check the balance of `redeem-pool`

		for _ in 0 .. remove_times {
			assert_ok!(Salp::dissolve(Some(ALICE).into(), 3_000));
		}

		assert!(Salp::funds(3_000).is_none());

		for i in 0 .. contribute_account_num {
			let ract = AccountId::new([(i as u8); 32]);
			let (contributed, contributing) = Salp::contribution_get(3_000, &ract);
			assert_eq!(contributed, 0);
			assert_eq!(contributing, 0);
		}
	});
}

#[test]
fn dissolve_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_ok!(Salp::fund_end(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::dissolve(Origin::root(), 3_000), DispatchError::BadOrigin);
		assert_noop!(Salp::dissolve(Origin::none(), 3_000), DispatchError::BadOrigin);
		assert_noop!(Salp::dissolve(Some(BRUCE).into(), 3_000), Error::<Test>::UnauthorizedAccount);
	});
}

#[test]
fn dissolve_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
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
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		assert_noop!(Salp::dissolve(Some(ALICE).into(), 3_000), Error::<Test>::InvalidFundStatus);
	});
}

// TODO: Add unit-tests for checking the `Hooks`

// Utilities Test
#[test]
fn check_next_trie_index() {
	new_test_ext().execute_with(|| {
		for i in 0 .. 100 {
			assert_eq!(Salp::current_trie_index(), i);
			assert_ok!(Salp::next_trie_index());
		}
	});
}

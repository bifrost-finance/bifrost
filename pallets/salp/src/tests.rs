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

use frame_support::{assert_noop, assert_ok, dispatch::DispatchError, traits::BalanceStatus as BS};
use orml_traits::{MultiCurrency, MultiReservableCurrency};

use crate::{mock::*, ContributionStatus, Error, FundStatus};

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
fn unlock_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());

		assert_eq!(Tokens::accounts(BRUCE, vsToken).free, 100);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).free, 100);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).reserved, 0);
	});
}

#[test]
fn unlock_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));

		assert_noop!(
			Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000),
			Error::<Test>::InvalidFundStatus
		);
	});
}

#[test]
fn unlock_when_already_unlocked_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		assert_noop!(
			Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000),
			Error::<Test>::InvalidContributionStatus
		);
	});
}

#[test]
fn unlock_without_enough_reserved_vsassets_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));

		// ABSOLUTELY NOT HAPPEN AT NORMAL PROCESS!
		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());
		Tokens::slash_reserved(vsToken, &BRUCE, 50);
		Tokens::slash_reserved(vsBond, &BRUCE, 50);

		// ```
		// // The following code will produce a supernatural bug.
		// // DONT ASK WHY, I DONT KNOW!
		// assert_noop!(
		// Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000),
		// Error::<Test>::NotEnoughBalanceToUnlock
		// );
		// ```
		let result = Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000);
		assert_noop!(result, Error::<Test>::NotEnoughBalanceToUnlock);
	});
}

#[test]
fn contribute_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));

		let fund = Salp::funds(3_000).unwrap();
		let (contributed, status) = Salp::contribution(fund.trie_index, &BRUCE);
		assert_eq!(fund.raised, 100);
		assert_eq!(contributed, 100);
		assert_eq!(status, ContributionStatus::Idle);

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
		let (contributed, status) = Salp::contribution(fund.trie_index, &BRUCE);
		assert_eq!(fund.raised, 200);
		assert_eq!(contributed, 200);
		assert_eq!(status, ContributionStatus::Idle);

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
		let (contributed, status) = Salp::contribution(fund.trie_index, &BRUCE);
		assert_eq!(fund.raised, 0);
		assert_eq!(contributed, 0);
		assert_eq!(status, ContributionStatus::Idle);

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
		let (contributed, status) = Salp::contribution(fund.trie_index, &BRUCE);
		assert_eq!(fund.raised, 100);
		assert_eq!(contributed, 100);
		assert_eq!(status, ContributionStatus::Idle);

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
	});
}

#[test]
fn contribute_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 4_000, 100),
			Error::<Test>::InvalidParaId
		);
	});
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
	});
}

#[test]
fn contribute_exceed_cap_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 3_000, 1_001),
			Error::<Test>::CapExceeded
		);
	});
}

#[test]
fn contribute_when_contributing_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_noop!(
			Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true),
			Error::<Test>::InvalidContributionStatus
		);
	});
}

#[test]
fn confirm_contribute_when_not_in_contributing_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));

		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 3_000, 100),
			Error::<Test>::InvalidContributionStatus
		);
	});
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
		assert_eq!(fund.status, FundStatus::RedeemWithdrew);

		assert_eq!(Salp::redeem_pool(), 100);

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 4_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 4_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, true));

		let fund = Salp::funds(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::RefundWithdrew);

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

		assert_eq!(Salp::redeem_pool(), 0);

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
		assert_eq!(fund.status, FundStatus::RedeemWithdrew);

		assert_eq!(Salp::redeem_pool(), 100);

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 4_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 4_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, true));
		assert_noop!(Salp::withdraw(Some(ALICE).into(), 4_000), Error::<Test>::InvalidFundStatus);

		let fund = Salp::funds(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::RefundWithdrew);

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
		assert_eq!(fund.status, FundStatus::RedeemWithdrew);

		assert_eq!(Salp::redeem_pool(), 100);

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 4_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 4_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, false));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, true));

		let fund = Salp::funds(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::RefundWithdrew);

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

#[test]
fn refund_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000));
		assert_ok!(Salp::confirm_refund(Some(ALICE).into(), BRUCE, 3_000, true));

		assert_eq!(Salp::refund_pool(), 0);

		let fund = Salp::funds(3_000).unwrap();
		let (contributed, status) = Salp::contribution(fund.trie_index, &BRUCE);
		assert_eq!(contributed, 100);
		assert_eq!(status, ContributionStatus::Refunded);

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
fn refund_when_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000));
		assert_ok!(Salp::confirm_refund(Some(ALICE).into(), BRUCE, 3_000, false));

		assert_eq!(Salp::refund_pool(), 100);

		let fund = Salp::funds(3_000).unwrap();
		let (contributed, status) = Salp::contribution(fund.trie_index, &BRUCE);
		assert_eq!(contributed, 100);
		assert_eq!(status, ContributionStatus::Idle);

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
fn double_refund_when_one_of_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000));
		assert_ok!(Salp::confirm_refund(Some(ALICE).into(), BRUCE, 3_000, false));
		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000));
		assert_ok!(Salp::confirm_refund(Some(ALICE).into(), BRUCE, 3_000, true));

		assert_eq!(Salp::refund_pool(), 0);

		let fund = Salp::funds(3_000).unwrap();
		let (contributed, status) = Salp::contribution(fund.trie_index, &BRUCE);
		assert_eq!(contributed, 100);
		assert_eq!(status, ContributionStatus::Refunded);

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
fn refund_without_enough_vsassets_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());
		assert_ok!(Tokens::repatriate_reserved(vsToken, &BRUCE, &ALICE, 50, BS::Reserved));
		assert_ok!(Tokens::repatriate_reserved(vsBond, &BRUCE, &ALICE, 50, BS::Reserved));

		assert_noop!(
			Salp::refund(Some(BRUCE).into(), 3_000),
			Error::<Test>::NotEnoughReservedAssetsToRefund
		);
	});
}

/// ABSOLUTELY NOT HAPPEN AT NORMAL PROCESS!
#[test]
fn refund_without_enough_balance_in_pool_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		// ABSOLUTELY NOT HAPPEN AT NORMAL PROCESS!
		crate::pallet::RefundPool::<Test>::set(50);

		assert_noop!(
			Salp::refund(Some(BRUCE).into(), 3_000),
			Error::<Test>::NotEnoughBalanceInRefundPool
		);
	});
}

#[test]
fn confirm_refund_without_enough_reserved_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000));

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());
		assert_ok!(Tokens::repatriate_reserved(vsToken, &BRUCE, &ALICE, 50, BS::Reserved));
		assert_ok!(Tokens::repatriate_reserved(vsBond, &BRUCE, &ALICE, 50, BS::Reserved));

		// ```
		// // The following code will produce a supernatural bug.
		// // DONT ASK WHY, I DONT KNOW!
		// assert_noop!(
		// Salp::confirm_refund(Some(ALICE).into(), BRUCE, 3_000, true),
		// Error::<Test>::NotEnoughReservedAssetsToRefund
		// );
		// ```
		let result = Salp::confirm_refund(Some(ALICE).into(), BRUCE, 3_000, true);
		assert_noop!(result, Error::<Test>::NotEnoughReservedAssetsToRefund);
	});
}

#[test]
fn refund_when_refunding_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000));

		assert_noop!(
			Salp::refund(Some(BRUCE).into(), 3_000),
			Error::<Test>::InvalidContributionStatus
		);
	});
}

#[test]
fn confirm_refund_when_not_in_refunding_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		assert_noop!(
			Salp::confirm_refund(Some(ALICE).into(), BRUCE, 3_000, true),
			Error::<Test>::InvalidContributionStatus,
		);
	});
}

#[test]
fn refund_with_zero_contribution_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		assert_noop!(Salp::refund(Some(BRUCE).into(), 3_000), Error::<Test>::ZeroContribution);
	});
}

#[test]
fn refund_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		assert_noop!(Salp::refund(Origin::root(), 3_000), DispatchError::BadOrigin);
		assert_noop!(Salp::refund(Origin::none(), 3_000), DispatchError::BadOrigin);

		assert_ok!(Salp::refund(Some(BRUCE).into(), 3_000));
		assert_noop!(
			Salp::confirm_refund(Origin::root(), BRUCE, 3_000, true),
			DispatchError::BadOrigin
		);
		assert_noop!(
			Salp::confirm_refund(Origin::none(), BRUCE, 3_000, true),
			DispatchError::BadOrigin
		);
		assert_noop!(
			Salp::confirm_refund(Some(BRUCE).into(), BRUCE, 3_000, true),
			Error::<Test>::UnauthorizedAccount
		);
	});
}

#[test]
fn refund_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		assert_noop!(Salp::refund(Some(BRUCE).into(), 4_000), Error::<Test>::InvalidParaId);
	});
}

#[test]
fn refund_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_ok!(Salp::fund_end(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::refund(Some(BRUCE).into(), 3_000), Error::<Test>::InvalidFundStatus);

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, true));

		assert_noop!(Salp::refund(Some(BRUCE).into(), 4_000), Error::<Test>::InvalidFundStatus);
	});
}

#[test]
fn refund_with_when_ump_wrong_should_fail() {
	// TODO: Require an solution to settle with parallel test workflow
}

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

		assert_eq!(Salp::redeem_pool(), (10 * contribute_account_num) as u128);

		for _ in 0 .. remove_times {
			assert_ok!(Salp::dissolve(Some(ALICE).into(), 3_000));
		}

		assert!(Salp::funds(3_000).is_none());
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

#[test]
fn redeem_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem);

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vsToken, &BRUCE, &CATHI, 50));
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vsBond, &BRUCE, &CATHI, 50));

		assert_ok!(Salp::redeem(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 50));
		assert_ok!(Salp::confirm_redeem(Origin::root(), BRUCE, 3_000, 1, SlotLength::get(), true));

		assert_eq!(Salp::redeem_pool(), 50);

		assert_eq!(Tokens::accounts(BRUCE, vsToken).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).free, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).reserved, 0);

		assert_ok!(Salp::redeem(Some(CATHI).into(), 3_000, 1, SlotLength::get(), 50));
		assert_ok!(Salp::confirm_redeem(Origin::root(), CATHI, 3_000, 1, SlotLength::get(), true));

		assert_eq!(Salp::redeem_pool(), 0);

		assert_eq!(Tokens::accounts(CATHI, vsToken).free, 0);
		assert_eq!(Tokens::accounts(CATHI, vsToken).frozen, 0);
		assert_eq!(Tokens::accounts(CATHI, vsToken).reserved, 0);
		assert_eq!(Tokens::accounts(CATHI, vsBond).free, 0);
		assert_eq!(Tokens::accounts(CATHI, vsBond).frozen, 0);
		assert_eq!(Tokens::accounts(CATHI, vsBond).reserved, 0);
	});
}

#[test]
fn redeem_when_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem);

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());

		assert_ok!(Salp::redeem(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 50));
		assert_ok!(Salp::confirm_redeem(Origin::root(), BRUCE, 3_000, 1, SlotLength::get(), false));

		assert_eq!(Salp::redeem_pool(), 100);

		assert_eq!(Tokens::accounts(BRUCE, vsToken).free, 100);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsToken).reserved, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).free, 100);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).frozen, 0);
		assert_eq!(Tokens::accounts(BRUCE, vsBond).reserved, 0);
	});
}

#[test]
fn redeem_when_redeeming_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem);

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_ok!(Salp::redeem(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 50));

		let result = Salp::redeem(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 50);
		assert_noop!(result, Error::<Test>::InvalidRedeemStatus);
	});
}

#[test]
fn confirm_redeem_when_not_in_redeeming_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem);

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		assert_noop!(
			Salp::confirm_redeem(Origin::root(), BRUCE, 3_000, 1, SlotLength::get(), true),
			Error::<Test>::InvalidRedeemStatus
		);
	});
}

#[test]
fn redeem_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem);

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		assert_noop!(
			Salp::redeem(Origin::root(), 3_000, 1, SlotLength::get(), 50),
			DispatchError::BadOrigin
		);
		assert_noop!(
			Salp::redeem(Origin::none(), 3_000, 1, SlotLength::get(), 50),
			DispatchError::BadOrigin
		);

		assert_noop!(
			Salp::confirm_redeem(Origin::none(), BRUCE, 3_000, 1, SlotLength::get(), true),
			Error::<Test>::UnauthorizedAccount
		);
		assert_noop!(
			Salp::confirm_redeem(Some(ALICE).into(), BRUCE, 3_000, 1, SlotLength::get(), true),
			Error::<Test>::UnauthorizedAccount
		);
	});
}

#[test]
fn redeem_with_expired_vsbond_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		let block_end_redeem = block_begin_redeem + VSBondValidPeriod::get();
		System::set_block_number(block_end_redeem);

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vsToken, &BRUCE, &CATHI, 50));
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vsBond, &BRUCE, &CATHI, 50));

		assert_noop!(
			Salp::redeem(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 50),
			Error::<Test>::VSBondExpired
		);

		assert_noop!(
			Salp::redeem(Some(CATHI).into(), 3_000, 1, SlotLength::get(), 50),
			Error::<Test>::VSBondExpired
		);
	});
}

#[test]
fn redeem_with_not_redeemable_vsbond_should_fail() {
	new_test_ext().execute_with(|| {
		// Mock redeem-pool already had some balance
		crate::pallet::RedeemPool::<Test>::set(100);

		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_not_redeemable = LeasePeriod::get();
		System::set_block_number(block_not_redeemable);

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vsToken, &BRUCE, &CATHI, 50));
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vsBond, &BRUCE, &CATHI, 50));

		assert_noop!(
			Salp::redeem(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 50),
			Error::<Test>::UnRedeemableNow
		);

		assert_noop!(
			Salp::redeem(Some(CATHI).into(), 3_000, 1, SlotLength::get(), 50),
			Error::<Test>::UnRedeemableNow
		);
	});
}

#[test]
fn redeem_without_enough_vsassets_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem);

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		#[allow(non_snake_case)]
		let (vsToken, vsBond) = Salp::vsAssets(3_000, 1, SlotLength::get());

		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vsToken, &BRUCE, &CATHI, 50));
		assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(vsBond, &BRUCE, &CATHI, 50));

		assert_noop!(
			Salp::redeem(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 60),
			Error::<Test>::NotEnoughFreeAssetsToRedeem
		);

		assert_noop!(
			Salp::redeem(Some(CATHI).into(), 3_000, 1, SlotLength::get(), 60),
			Error::<Test>::NotEnoughFreeAssetsToRedeem
		);
	});
}

#[test]
fn redeem_without_enough_balance_in_pool_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 100));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));

		// Mock the BlockNumber
		let block_begin_redeem = (SlotLength::get() + 1) * LeasePeriod::get();
		System::set_block_number(block_begin_redeem);

		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));

		// Before withdraw
		assert_noop!(
			Salp::redeem(Some(BRUCE).into(), 3_000, 1, SlotLength::get(), 50),
			Error::<Test>::NotEnoughBalanceInRedeemPool
		);

		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
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
		while System::block_number() <= n {
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
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, true));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::unlock(Some(BRUCE).into(), BRUCE, 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		run_to_block(ReleaseCycle::get());

		let release_amount = ReleaseRatio::get() * 100u128;
		let redeem_pool_left = 100 - release_amount;
		assert_eq!(Salp::redeem_pool(), redeem_pool_left);

		// TODO: Check the balance of bancor(Waiting Bancor to Support..)
	});
}

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

#[test]
fn check_is_expired() {
	new_test_ext().execute_with(|| {
		let slot = 10;
		let block_end_of_slot = Salp::block_end_of_lease_period_index(slot);
		let block_expired = block_end_of_slot + VSBondValidPeriod::get();

		assert!(Salp::is_expired(block_expired, slot));
	});
}

#[test]
fn check_can_redeem() {
	new_test_ext().execute_with(|| {
		let slot = 10;
		let block_end_of_slot = Salp::block_end_of_lease_period_index(slot);
		let block_redeemable = block_end_of_slot + VSBondValidPeriod::get() / 2;

		assert!(Salp::can_redeem(block_redeemable, slot));
	});
}

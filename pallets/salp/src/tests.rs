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

use crate::{mock::*, ContributionStatus, Error, FundStatus};

#[test]
fn create_fund_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get(),));
		assert_ok!(Salp::funds(3_000).ok_or(()));
		assert_eq!(Salp::current_trie_index(), 1);
	});
}

#[test]
fn create_fund_under_non_signed_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Salp::create(Origin::root(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()),
			DispatchError::BadOrigin,
		);

		assert_noop!(
			Salp::create(Origin::none(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn create_fund_existed_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get(),),);

		assert_noop!(
			Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get(),),
			Error::<Test>::FundExisted,
		);
	});
}

#[test]
fn create_fund_exceed_slot_limit_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 0, SlotLength::get()),
			Error::<Test>::LastSlotTooFarInFuture,
		);
	});
}

#[test]
fn create_fund_first_slot_bigger_than_last_slot_should_fail() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, SlotLength::get(), 0),
			Error::<Test>::LastSlotBeforeFirstSlot,
		);
	});
}

#[test]
fn set_fund_success_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));

		// Check status
		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Success);
	});
}

#[test]
fn set_fund_success_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_noop!(Salp::fund_success(Origin::root(), 3_000), Error::<Test>::InvalidOrigin);
		assert_noop!(Salp::fund_success(Origin::none(), 3_000), Error::<Test>::InvalidOrigin);
		assert_noop!(Salp::fund_success(Some(BRUCE).into(), 3_000), Error::<Test>::InvalidOrigin);
	})
}

#[test]
fn set_fund_success_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_noop!(Salp::fund_success(Some(ALICE).into(), 4_000), Error::<Test>::InvalidParaId);
	});
}

#[test]
fn set_fund_success_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
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
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 3_000));

		// Check status
		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Failed);
	});
}

#[test]
fn set_fund_fail_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_noop!(Salp::fund_fail(Origin::root(), 3_000), Error::<Test>::InvalidOrigin);
		assert_noop!(Salp::fund_fail(Origin::none(), 3_000), Error::<Test>::InvalidOrigin);
		assert_noop!(Salp::fund_fail(Some(BRUCE).into(), 3_000), Error::<Test>::InvalidOrigin);
	});
}

#[test]
fn set_fund_fail_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_noop!(Salp::fund_fail(Some(ALICE).into(), 4_000), Error::<Test>::InvalidParaId);
	});
}

#[test]
fn set_fund_fail_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_noop!(Salp::fund_fail(Some(ALICE).into(), 3_000), Error::<Test>::InvalidFundStatus);
	});
}

#[test]
fn set_fund_retire_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
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
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_noop!(Salp::fund_retire(Origin::root(), 3_000), Error::<Test>::InvalidOrigin);
		assert_noop!(Salp::fund_retire(Origin::none(), 3_000), Error::<Test>::InvalidOrigin);
		assert_noop!(Salp::fund_retire(Some(BRUCE).into(), 3_000), Error::<Test>::InvalidOrigin);
	});
}

#[test]
fn set_fund_retire_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_noop!(Salp::fund_retire(Some(ALICE).into(), 4_000), Error::<Test>::InvalidParaId);
	});
}

#[test]
fn set_fund_retire_with_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_noop!(
			Salp::fund_retire(Some(ALICE).into(), 3_000),
			Error::<Test>::InvalidFundStatus
		);
	});
}

#[test]
fn set_fund_end_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
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
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		assert_noop!(Salp::fund_end(Origin::root(), 3_000), Error::<Test>::InvalidOrigin);
		assert_noop!(Salp::fund_end(Origin::none(), 3_000), Error::<Test>::InvalidOrigin);
		assert_noop!(Salp::fund_end(Some(BRUCE).into(), 3_000), Error::<Test>::InvalidOrigin);
	});
}

#[test]
fn set_fund_end_with_wrong_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
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
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));

		assert_noop!(Salp::fund_end(Some(ALICE).into(), 3_000), Error::<Test>::InvalidFundStatus);
	});
}

#[test]
fn contribute_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));

		let fund_origin = Salp::funds(3_000).unwrap();
		let (balance, status) = Salp::contribution_get(fund_origin.trie_index, &BRUCE);

		// Check the init status
		assert_eq!(balance, 0 * DOLLARS);
		assert_eq!(status, ContributionStatus::Contributed);

		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 10 * DOLLARS));

		let fund_after = Salp::funds(3_000).unwrap();
		let (balance, status) = Salp::contribution_get(fund_after.trie_index, &BRUCE);

		// Ensure `Salp::contribute` not change the state(data in storage)
		assert_eq!(fund_origin, fund_after);
		assert_eq!(balance, 0 * DOLLARS);
		assert_eq!(status, ContributionStatus::Contributing);

		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, 10 * DOLLARS, true));

		let fund = Salp::funds(3_000).unwrap();
		let (balance, status) = Salp::contribution_get(fund.trie_index, &BRUCE);

		// Check the contribution
		assert_eq!(balance, 10 * DOLLARS);
		assert_eq!(status, ContributionStatus::Contributed);

		// Check the fund raised
		let raised_delta = fund.raised.saturating_sub(fund_origin.raised);
		assert_eq!(raised_delta, balance);

		// TODO: Check the status of vsToken/vsBond issued
	});
}

#[test]
fn double_contribute_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 10 * DOLLARS));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, 10 * DOLLARS, true));

		// Check the contribution
		let fund = Salp::funds(3_000).unwrap();
		let (balance, status) = Salp::contribution_get(fund.trie_index, &BRUCE);
		assert_eq!(balance, 10 * DOLLARS);
		assert_eq!(status, ContributionStatus::Contributed);

		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 10 * DOLLARS));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, 10 * DOLLARS, true));

		// Check the contribution
		let fund = Salp::funds(3_000).unwrap();
		let (balance, status) = Salp::contribution_get(fund.trie_index, &BRUCE);
		assert_eq!(balance, 20 * DOLLARS);
		assert_eq!(status, ContributionStatus::Contributed);

		// TODO: Check the status of vsToken/vsBond issued
	});
}

#[test]
fn contribute_when_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 10 * DOLLARS));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, 10 * DOLLARS, false));

		let fund = Salp::funds(3_000).unwrap();
		let (balance, status) = Salp::contribution_get(fund.trie_index, &BRUCE);

		assert_eq!(balance, 0 * DOLLARS);
		assert_eq!(status, ContributionStatus::Contributed);
		assert_eq!(fund.raised, 0 * DOLLARS);

		// TODO: Check the status of vsToken/vsBond issued
	});
}

#[test]
fn double_contribute_when_one_of_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 10 * DOLLARS));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, 10 * DOLLARS, false));

		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 10 * DOLLARS));
		assert_ok!(Salp::confirm_contribute(Some(ALICE).into(), BRUCE, 3_000, 10 * DOLLARS, true));

		let fund = Salp::funds(3_000).unwrap();
		let (balance, status) = Salp::contribution_get(fund.trie_index, &BRUCE);

		assert_eq!(balance, 10 * DOLLARS);
		assert_eq!(status, ContributionStatus::Contributed);
		assert_eq!(fund.raised, balance);

		// TODO: Check the status of vsToken/vsBond issued
	});
}

#[test]
fn contribute_with_wrong_origin_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_noop!(
			Salp::contribute(Origin::root(), 3_000, 10 * DOLLARS),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn contribute_with_low_contribution_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 3_000, MinContribution::get() - 1),
			Error::<Test>::ContributionTooSmall
		);
	})
}

#[test]
fn contribute_with_wrong_para_id_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 4_000, 10 * DOLLARS),
			Error::<Test>::InvalidParaId
		);
	})
}

#[test]
fn contribute_with_wrong_fund_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000,));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 3_000, 10 * DOLLARS),
			Error::<Test>::InvalidFundStatus
		);
	})
}

#[test]
fn contribute_exceed_cap_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 3_000, 1_001 * DOLLARS),
			Error::<Test>::CapExceeded
		);
	})
}

#[test]
fn contribute_with_wrong_contribution_status_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::contribute(Some(BRUCE).into(), 3_000, 10 * DOLLARS));
		assert_noop!(
			Salp::contribute(Some(BRUCE).into(), 3_000, 10 * DOLLARS),
			Error::<Test>::ContributionInvalid
		);
	});
}

#[test]
fn contribute_when_ump_wrong_should_fail() {
	// TODO: NEED SERIAL EXEC
	// new_test_ext().execute_with(|| {
	// 	assert_ok!(Salp::create(
	// 		Some(ALICE).into(),
	// 		3_000,
	// 		1_000 * DOLLARS,
	// 		1,
	// 		SlotLength::get()
	// 	));
	//
	// 	unsafe {
	// 		MOCK_XCM_RESULT = (false, false);
	// 	}
	//
	// 	assert_noop!(
	// 		Salp::contribute(Some(BRUCE).into(), 3_000, 10 * DOLLARS),
	// 		Error::<Test>::XcmFailed,
	// 	);
	//
	// 	unsafe {
	// 		MOCK_XCM_RESULT = (true, true);
	// 	}
	// })
}

#[test]
fn withdraw_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));

		// Check storage
		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Withdrew);

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, true));

		// Check storage
		let fund = Salp::funds(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::Withdrew);
	});
}

#[test]
fn withdraw_when_xcm_error_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, false));

		// Check storage
		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Retired);

		assert_ok!(Salp::create(Some(ALICE).into(), 4_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_fail(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 4_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 4_000, false));

		// Check storage
		let fund = Salp::funds(4_000).unwrap();
		assert_eq!(fund.status, FundStatus::Failed);
	});
}

#[test]
fn double_withdraw_same_fund_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(Salp::create(Some(ALICE).into(), 3_000, 1_000 * DOLLARS, 1, SlotLength::get()));
		assert_ok!(Salp::fund_success(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::fund_retire(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::withdraw(Some(ALICE).into(), 3_000));
		assert_ok!(Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true));
		assert_noop!(
			Salp::confirm_withdraw(Some(ALICE).into(), 3_000, true),
			Error::<Test>::InvalidFundStatus
		);

		// Check storage
		let fund = Salp::funds(3_000).unwrap();
		assert_eq!(fund.status, FundStatus::Withdrew);
	});
}

#[test]
fn double_withdraw_same_fund_when_xcm_error_should_work() {
	
}

#[test]
fn double_withdraw_same_fund_when_one_of_xcm_error_should_work() {

}

#[test]
fn withdraw_with_wrong_para_id_should_fail() {

}

#[test]
fn withdraw_with_wrong_fund_status_should_fail() {

}

#[test]
fn withdraw_with_when_ump_wrong_should_fail() {

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

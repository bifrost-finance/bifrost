// Copyright 2019-2022 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

//! traits for parachain-staking
use frame_support::pallet_prelude::DispatchResultWithPostInfo;

pub trait OnCollatorPayout<AccountId, Balance> {
	fn on_collator_payout(
		for_round: crate::RoundIndex,
		collator_id: AccountId,
		amount: Balance,
	) -> frame_support::pallet_prelude::Weight;
}
impl<AccountId, Balance> OnCollatorPayout<AccountId, Balance> for () {
	fn on_collator_payout(
		_for_round: crate::RoundIndex,
		_collator_id: AccountId,
		_amount: Balance,
	) -> frame_support::pallet_prelude::Weight {
		0
	}
}

pub trait OnNewRound {
	fn on_new_round(round_index: crate::RoundIndex) -> frame_support::pallet_prelude::Weight;
}
impl OnNewRound for () {
	fn on_new_round(_round_index: crate::RoundIndex) -> frame_support::pallet_prelude::Weight {
		0
	}
}

pub trait ParachainStakingInterface<AccountId, Balance> {
	fn delegate(
		delegator: AccountId,
		candidate: AccountId,
		amount: Balance,
		candidate_delegation_count: u32,
		delegation_count: u32,
	) -> DispatchResultWithPostInfo;

	fn delegator_bond_more(
		delegator: AccountId,
		candidate: AccountId,
		more: Balance,
	) -> DispatchResultWithPostInfo;

	fn schedule_delegator_bond_less(
		delegator: AccountId,
		candidate: AccountId,
		less: Balance,
	) -> DispatchResultWithPostInfo;

	fn schedule_leave_delegators(delegator: AccountId) -> DispatchResultWithPostInfo;

	fn cancel_delegation_request(
		delegator: AccountId,
		candidate: AccountId,
	) -> DispatchResultWithPostInfo;

	fn schedule_revoke_delegation(
		delegator: AccountId,
		collator: AccountId,
	) -> DispatchResultWithPostInfo;

	fn cancel_leave_delegators(delegator: AccountId) -> DispatchResultWithPostInfo;

	fn execute_leave_delegators(
		delegator: AccountId,
		delegation_count: u32,
	) -> DispatchResultWithPostInfo;

	fn execute_delegation_request(
		delegator: AccountId,
		candidate: AccountId,
	) -> DispatchResultWithPostInfo;

	fn get_delegation_count(delegator: AccountId, candidate: AccountId) -> (u32, u32);
}

impl<AccountId, Balance> ParachainStakingInterface<AccountId, Balance> for () {
	fn delegate(
		_delegator: AccountId,
		_candidate: AccountId,
		_amount: Balance,
		_candidate_delegation_count: u32,
		_delegation_count: u32,
	) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn delegator_bond_more(
		_delegator: AccountId,
		_candidate: AccountId,
		_more: Balance,
	) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn schedule_delegator_bond_less(
		_delegator: AccountId,
		_candidate: AccountId,
		_less: Balance,
	) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn schedule_leave_delegators(_delegator: AccountId) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn cancel_delegation_request(
		_delegator: AccountId,
		_candidate: AccountId,
	) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn schedule_revoke_delegation(
		_delegator: AccountId,
		_collator: AccountId,
	) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn cancel_leave_delegators(_delegator: AccountId) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn execute_leave_delegators(
		_delegator: AccountId,
		_delegation_count: u32,
	) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn execute_delegation_request(
		_delegator: AccountId,
		_candidate: AccountId,
	) -> DispatchResultWithPostInfo {
		Ok(().into())
	}

	fn get_delegation_count(_delegator: AccountId, _candidate: AccountId) -> (u32, u32) {
		(0, 0)
	}
}

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

use crate::{AccountVote, DispatchResult, PollClass, PollIndex, *};
use bifrost_primitives::{CurrencyId, DerivativeIndex};
use sp_std::vec::Vec;

/// Abstraction over a voting agent for a certain parachain.
pub trait VotingAgent<Balance, AccountId, Error> {
	fn vote(
		&self,
		who: AccountId,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<Balance>)>,
		poll_index: PollIndex,
		vtoken: CurrencyId,
		submitted: bool,
		maybe_old_vote: Option<(AccountVote<Balance>, Balance)>,
	) -> DispatchResult;

	fn remove_vote(
		&self,
		class: PollClass,
		poll_index: PollIndex,
		vtoken: CurrencyId,
		derivative_index: DerivativeIndex,
	) -> DispatchResult;
}

pub trait ConvictionVotingCall<T: Config> {
	fn vote(poll_index: PollIndex, vote: AccountVote<BalanceOf<T>>) -> Self;

	fn remove_vote(class: Option<PollClass>, poll_index: PollIndex) -> Self;
}

pub trait UtilityCall<Call> {
	fn as_derivative(derivative_index: DerivativeIndex, call: Call) -> Call;

	fn batch_all(calls: Vec<Call>) -> Call;
}

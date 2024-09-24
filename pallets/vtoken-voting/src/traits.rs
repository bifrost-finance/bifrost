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

use crate::{AccountVote, PollClass, PollIndex, *};
use bifrost_primitives::DerivativeIndex;
use sp_std::vec::Vec;

/// Abstraction over a voting agent for a certain parachain.
pub trait VotingAgent<T: Config> {
	fn vtoken(&self) -> CurrencyIdOf<T>;
	fn location(&self) -> Location;
	fn delegate_vote(
		&self,
		who: AccountIdOf<T>,
		vtoken: CurrencyIdOf<T>,
		poll_index: PollIndex,
		submitted: bool,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
		maybe_old_vote: Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>,
	) -> DispatchResult;
	fn vote_call_encode(
		&self,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
		poll_index: PollIndex,
		derivative_index: DerivativeIndex,
	) -> Result<Vec<u8>, Error<T>>;
	fn delegate_remove_delegator_vote(
		&self,
		vtoken: CurrencyIdOf<T>,
		poll_index: PollIndex,
		class: PollClass,
		derivative_index: DerivativeIndex,
	) -> DispatchResult;
	fn remove_delegator_vote_call_encode(
		&self,
		class: PollClass,
		poll_index: PollIndex,
		derivative_index: DerivativeIndex,
	) -> Result<Vec<u8>, Error<T>>;
}

pub trait ConvictionVotingCall<T: Config> {
	fn vote(poll_index: PollIndex, vote: AccountVote<BalanceOf<T>>) -> Self;

	fn remove_vote(class: Option<PollClass>, poll_index: PollIndex) -> Self;
}

pub trait UtilityCall<Call> {
	fn as_derivative(derivative_index: DerivativeIndex, call: Call) -> Call;

	fn batch_all(calls: Vec<Call>) -> Call;
}

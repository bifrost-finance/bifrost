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
///
/// This trait defines the operations a voting agent should implement for handling votes and
/// delegator-related actions within a certain parachain's voting system.
pub trait VotingAgent<T: Config> {
	/// Retrieves the voting token (`vtoken`) associated with the agent.
	///
	/// This function should return the currency ID representing the token used for voting.
	fn vtoken(&self) -> CurrencyIdOf<T>;

	/// Retrieves the location of the agent.
	///
	/// This function returns the location associated with the agent, which could be used to
	/// identify the origin or context within the parachain system.
	fn location(&self) -> Location;

	/// Delegate a vote on behalf of a user.
	///
	/// - `who`: The account for which the vote is being delegated.
	/// - `vtoken`: The token used for voting.
	/// - `poll_index`: The index of the poll on which the vote is being cast.
	/// - `submitted`: A flag indicating whether the vote was already submitted.
	/// - `new_delegator_votes`: A vector of delegator votes, represented by the index of the
	///   derivative and the account's vote.
	/// - `maybe_old_vote`: An optional tuple representing the old vote and its associated balance,
	///   in case an old vote exists.
	///
	/// This function handles the delegation of votes for the specified account and updates the
	/// voting state accordingly. It returns a `DispatchResult` to indicate success or failure.
	fn delegate_vote(
		&self,
		who: AccountIdOf<T>,
		vtoken: CurrencyIdOf<T>,
		poll_index: PollIndex,
		submitted: bool,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
		maybe_old_vote: Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>,
	) -> DispatchResult;

	/// Encode the call data for voting.
	///
	/// - `new_delegator_votes`: A vector of new delegator votes to be encoded.
	/// - `poll_index`: The index of the poll.
	/// - `derivative_index`: The index of the derivative (delegator) involved in the voting
	///   process.
	///
	/// This function encodes the call for a vote delegation action, returning the byte-encoded
	/// representation of the call data. In case of errors during encoding, an `Error<T>` is
	/// returned.
	fn vote_call_encode(
		&self,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
		poll_index: PollIndex,
		derivative_index: DerivativeIndex,
	) -> Result<Vec<u8>, Error<T>>;

	/// Remove a delegator's vote.
	///
	/// - `vtoken`: The token associated with the vote.
	/// - `poll_index`: The index of the poll from which the vote is being removed.
	/// - `class`: The class/type of the poll.
	/// - `derivative_index`: The index of the delegator's derivative whose vote is being removed.
	///
	/// This function handles the removal of a delegator's vote for the specified poll and class,
	/// returning a `DispatchResult` to indicate success or failure.
	fn delegate_remove_delegator_vote(
		&self,
		vtoken: CurrencyIdOf<T>,
		poll_index: PollIndex,
		class: PollClass,
		derivative_index: DerivativeIndex,
	) -> DispatchResult;

	/// Encode the call data for removing a delegator's vote.
	///
	/// - `class`: The class/type of the poll.
	/// - `poll_index`: The index of the poll from which the vote is being removed.
	/// - `derivative_index`: The index of the delegator's derivative involved in the vote removal.
	///
	/// This function encodes the call for removing a delegator's vote, returning the byte-encoded
	/// representation of the call data. In case of errors during encoding, an `Error<T>` is
	/// returned.
	fn remove_delegator_vote_call_encode(
		&self,
		class: PollClass,
		poll_index: PollIndex,
		derivative_index: DerivativeIndex,
	) -> Result<Vec<u8>, Error<T>>;
}

/// Trait defining calls related to conviction voting mechanisms.
pub trait ConvictionVotingCall<T: Config> {
	/// Casts a vote in a poll.
	///
	/// - `poll_index`: The index of the poll where the vote is being cast.
	/// - `vote`: The vote being cast, which includes the voter's balance and conviction.
	fn vote(poll_index: PollIndex, vote: AccountVote<BalanceOf<T>>) -> Self;

	/// Removes a vote from a poll.
	///
	/// - `class`: Optionally specify the class of the poll (if applicable).
	/// - `poll_index`: The index of the poll from which the vote is being removed.
	fn remove_vote(class: Option<PollClass>, poll_index: PollIndex) -> Self;
}

/// Trait defining utility calls for handling batch or derivative actions.
pub trait UtilityCall<Call> {
	/// Executes a call as a derivative of another account.
	///
	/// - `derivative_index`: The index representing a specific derivative account (usually for
	///   staking or delegation purposes).
	/// - `call`: The call that will be executed by the derivative account.
	fn as_derivative(derivative_index: DerivativeIndex, call: Call) -> Call;

	/// Executes a batch of calls where all must succeed or none will be executed.
	///
	/// - `calls`: A vector of calls that will be executed in batch. If any call fails, none of the
	///   calls will be applied.
	fn batch_all(calls: Vec<Call>) -> Call;
}

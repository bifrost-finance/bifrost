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

use bifrost_primitives::{
	CurrencyId, DerivativeIndex, XcmDestWeightAndFeeHandler, XcmOperationType,
};
use core::marker::PhantomData;
use frame_support::{dispatch::DispatchResult, ensure};

use crate::{
	pallet::{Error, Pallet},
	traits::VotingAgent,
	AccountIdOf, AccountVote, BalanceOf, Call, Config, ConvictionVotingCall, CurrencyIdOf,
	PendingReferendumInfo, PendingVotingInfo, PollIndex, RelayCall, UtilityCall,
};

/// StakingAgent implementation for Astar
pub struct RelaychainAgent<T>(PhantomData<T>);

impl<T> RelaychainAgent<T> {
	pub fn new() -> Self {
		RelaychainAgent(PhantomData::<T>)
	}
}

impl<T: Config> VotingAgent<BalanceOf<T>, AccountIdOf<T>, Error<T>> for RelaychainAgent<T> {
	fn vote(
		&self,
		who: AccountIdOf<T>,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
		poll_index: PollIndex,
		vtoken: CurrencyIdOf<T>,
		submitted: bool,
		maybe_old_vote: Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>,
	) -> DispatchResult {
		// send XCM message
		let vote_calls = new_delegator_votes
			.iter()
			.map(|(_derivative_index, vote)| {
				<RelayCall<T> as ConvictionVotingCall<T>>::vote(poll_index, *vote)
			})
			.collect::<Vec<_>>();
		let vote_call = if vote_calls.len() == 1 {
			vote_calls.into_iter().nth(0).ok_or(Error::<T>::NoData)?
		} else {
			ensure!(false, Error::<T>::NoPermissionYet);
			<RelayCall<T> as UtilityCall<RelayCall<T>>>::batch_all(vote_calls)
		};
		let notify_call = Call::<T>::notify_vote { query_id: 0, response: Default::default() };
		let (weight, extra_fee) = T::XcmDestWeightAndFee::get_operation_weight_and_fee(
			CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?,
			XcmOperationType::Vote,
		)
		.ok_or(Error::<T>::NoData)?;

		let derivative_index = new_delegator_votes[0].0;
		Pallet::<T>::send_xcm_with_notify(
			derivative_index,
			vote_call,
			notify_call,
			weight,
			extra_fee,
			|query_id| {
				if !submitted {
					PendingReferendumInfo::<T>::insert(query_id, (vtoken, poll_index));
				}
				PendingVotingInfo::<T>::insert(
					query_id,
					(vtoken, poll_index, derivative_index, who.clone(), maybe_old_vote),
				)
			},
		)?;
		Ok(())
	}
}

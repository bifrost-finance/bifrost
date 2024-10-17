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

use crate::*;
use bifrost_primitives::{CurrencyId, DerivativeIndex};
use frame_support::pallet_prelude::*;
use xcm::v4::Location;

use crate::{agents::bifrost_agent::BifrostCall, pallet::Error, traits::*};

/// VotingAgent implementation for Bifrost
pub struct BifrostAgent<T: Config> {
	vtoken: CurrencyIdOf<T>,
	location: Location,
}

impl<T: Config> BifrostAgent<T> {
	// Only polkadot networks are supported.
	pub fn new(vtoken: CurrencyId) -> Result<Self, Error<T>> {
		if cfg!(feature = "polkadot") {
			let location = Pallet::<T>::convert_vtoken_to_dest_location(vtoken)?;
			Ok(Self { vtoken, location })
		} else {
			Err(Error::<T>::VTokenNotSupport)
		}
	}
}

impl<T: Config> VotingAgent<T> for BifrostAgent<T> {
	fn vtoken(&self) -> CurrencyIdOf<T> {
		self.vtoken
	}

	fn location(&self) -> Location {
		self.location.clone()
	}

	fn delegate_vote(
		&self,
		who: AccountIdOf<T>,
		vtoken: CurrencyIdOf<T>,
		poll_index: PollIndex,
		_submitted: bool,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
		maybe_old_vote: Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>,
	) -> DispatchResult {
		// Get the derivative index from the first delegator vote.
		let derivative_index = new_delegator_votes[0].0;
		let call_encode =
			self.vote_call_encode(new_delegator_votes, poll_index, derivative_index)?;
		let vote_call: <T as frame_system::Config>::RuntimeCall =
			<T as frame_system::Config>::RuntimeCall::decode(&mut &*call_encode)
				.map_err(|_| Error::<T>::CallDecodeFailed)?;

		let token = CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?;
		let delegator: AccountIdOf<T> =
			T::DerivativeAccount::get_account_id(token, derivative_index)
				.ok_or(Error::<T>::NoData)?;
		let origin = RawOrigin::Signed(delegator).into();
		let success = vote_call.dispatch(origin).is_ok();
		Pallet::<T>::handle_vote_result(
			success,
			who,
			vtoken,
			poll_index,
			maybe_old_vote,
			derivative_index,
		)?;

		if success {
			Ok(())
		} else {
			Err(Error::<T>::InvalidCallDispatch.into())
		}
	}

	fn vote_call_encode(
		&self,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
		poll_index: PollIndex,
		_derivative_index: DerivativeIndex,
	) -> Result<Vec<u8>, Error<T>> {
		let vote_calls = new_delegator_votes
			.iter()
			.map(|(_derivative_index, vote)| {
				<BifrostCall<T> as ConvictionVotingCall<T>>::vote(poll_index, *vote)
			})
			.collect::<Vec<_>>();
		let vote_call = if vote_calls.len() == 1 {
			vote_calls.into_iter().nth(0).ok_or(Error::<T>::NoData)?
		} else {
			ensure!(false, Error::<T>::NoPermissionYet);
			<BifrostCall<T> as UtilityCall<BifrostCall<T>>>::batch_all(vote_calls)
		};

		Ok(vote_call.encode())
	}

	fn delegate_remove_delegator_vote(
		&self,
		vtoken: CurrencyIdOf<T>,
		poll_index: PollIndex,
		class: PollClass,
		derivative_index: DerivativeIndex,
	) -> DispatchResult {
		let call_encode =
			self.remove_delegator_vote_call_encode(class, poll_index, derivative_index)?;
		let call = <T as frame_system::Config>::RuntimeCall::decode(&mut &*call_encode)
			.map_err(|_| Error::<T>::CallDecodeFailed)?;

		let token = CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?;
		let delegator: AccountIdOf<T> =
			T::DerivativeAccount::get_account_id(token, derivative_index)
				.ok_or(Error::<T>::NoData)?;
		let origin = RawOrigin::Signed(delegator).into();
		let success = call.dispatch(origin).is_ok();

		if success {
			Pallet::<T>::handle_remove_delegator_vote_success(vtoken, poll_index);
			Ok(())
		} else {
			Err(Error::<T>::InvalidCallDispatch.into())
		}
	}

	fn remove_delegator_vote_call_encode(
		&self,
		class: PollClass,
		poll_index: PollIndex,
		_derivative_index: DerivativeIndex,
	) -> Result<Vec<u8>, Error<T>> {
		Ok(<BifrostCall<T> as ConvictionVotingCall<T>>::remove_vote(Some(class), poll_index)
			.encode())
	}
}

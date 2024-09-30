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
use frame_support::{ensure, pallet_prelude::*};
use xcm::v4::Location;

use crate::{agents::relaychain_agent::call::*, pallet::Error, traits::*};

/// VotingAgent implementation for relay chain
pub struct RelaychainAgent<T: Config> {
	vtoken: CurrencyIdOf<T>,
	location: Location,
}
impl<T: Config> RelaychainAgent<T> {
	pub fn new(vtoken: CurrencyId) -> Result<Self, Error<T>> {
		let location = Pallet::<T>::convert_vtoken_to_dest_location(vtoken)?;
		Ok(Self { vtoken, location })
	}
}

impl<T: Config> VotingAgent<T> for RelaychainAgent<T> {
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
		submitted: bool,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
		maybe_old_vote: Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>,
	) -> DispatchResult {
		Pallet::<T>::send_xcm_vote_message(
			who,
			vtoken,
			poll_index,
			submitted,
			new_delegator_votes,
			maybe_old_vote,
		)
	}

	fn vote_call_encode(
		&self,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
		poll_index: PollIndex,
		derivative_index: DerivativeIndex,
	) -> Result<Vec<u8>, Error<T>> {
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

		let encode_call =
			<RelayCall<T> as UtilityCall<RelayCall<T>>>::as_derivative(derivative_index, vote_call)
				.encode();

		Ok(encode_call)
	}

	fn delegate_remove_delegator_vote(
		&self,
		vtoken: CurrencyIdOf<T>,
		poll_index: PollIndex,
		class: PollClass,
		derivative_index: DerivativeIndex,
	) -> DispatchResult {
		Pallet::<T>::send_xcm_remove_delegator_vote_message(
			vtoken,
			poll_index,
			class,
			derivative_index,
		)
	}

	fn remove_delegator_vote_call_encode(
		&self,
		class: PollClass,
		poll_index: PollIndex,
		derivative_index: DerivativeIndex,
	) -> Result<Vec<u8>, Error<T>> {
		let remove_vote_call =
			<RelayCall<T> as ConvictionVotingCall<T>>::remove_vote(Some(class), poll_index);
		Ok(<RelayCall<T> as UtilityCall<RelayCall<T>>>::as_derivative(
			derivative_index,
			remove_vote_call,
		)
		.encode())
	}
}

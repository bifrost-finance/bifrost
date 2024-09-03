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

use crate::{agents::bifrost_agent::BifrostCall, *};
use bifrost_primitives::{CurrencyId, DerivativeIndex};
use core::marker::PhantomData;
use frame_support::{ensure, pallet_prelude::*};
use xcm::v4::Location;

use crate::{pallet::Error, traits::*};

/// VotingAgent implementation for Bifrost
pub struct BifrostAgent<T> {
	location: Location,
	_marker: PhantomData<T>,
}

impl<T: pallet::Config> BifrostAgent<T> {
	pub fn new(vtoken: CurrencyId) -> Result<Self, Error<T>> {
		return if cfg!(feature = "polkadot") {
			let location = Pallet::<T>::convert_vtoken_to_dest_location(vtoken)?;
			Ok(Self { location, _marker: PhantomData })
		} else {
			Err(Error::<T>::VTokenNotSupport)
		}
	}
}

impl<T: Config> VotingAgent<BalanceOf<T>, AccountIdOf<T>, Error<T>> for BifrostAgent<T> {
	fn location(&self) -> Location {
		self.location.clone()
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
				<BifrostCall<T> as ConvictionVotingCall<T>>::vote(poll_index, *vote)
			})
			.collect::<Vec<_>>();
		let vote_call = if vote_calls.len() == 1 {
			vote_calls.into_iter().nth(0).ok_or(Error::<T>::NoData)?
		} else {
			ensure!(false, Error::<T>::NoPermissionYet);
			<BifrostCall<T> as UtilityCall<BifrostCall<T>>>::batch_all(vote_calls)
		};

		let encode_call = <BifrostCall<T> as UtilityCall<BifrostCall<T>>>::as_derivative(
			derivative_index,
			vote_call,
		)
		.encode();

		Ok(encode_call)
	}

	fn remove_delegator_vote_call_encode(
		&self,
		class: PollClass,
		poll_index: PollIndex,
		derivative_index: DerivativeIndex,
	) -> Vec<u8> {
		let remove_vote_call =
			<BifrostCall<T> as ConvictionVotingCall<T>>::remove_vote(Some(class), poll_index);
		<BifrostCall<T> as UtilityCall<BifrostCall<T>>>::as_derivative(
			derivative_index,
			remove_vote_call,
		)
		.encode()
	}
}

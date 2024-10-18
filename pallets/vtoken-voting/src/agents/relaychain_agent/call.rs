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
#![allow(ambiguous_glob_imports)]
#![allow(ambiguous_glob_reexports)]
#![allow(unused_imports)]

use crate::{traits::*, *};
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{traits::StaticLookup, RuntimeDebug};

#[cfg(feature = "kusama")]
pub(in crate::agents::relaychain_agent) use kusama::*;

#[cfg(feature = "polkadot")]
pub(in crate::agents::relaychain_agent) use polkadot::*;

#[cfg(feature = "kusama")]
pub(in crate::agents::relaychain_agent) mod kusama {
	use crate::agents::relaychain_agent::call::*;

	#[derive(Encode, Decode, RuntimeDebug)]
	pub(in crate::agents::relaychain_agent) enum RelayCall<T: Config> {
		#[codec(index = 20)]
		ConvictionVoting(ConvictionVoting<T>),
		#[codec(index = 24)]
		Utility(Utility<Self>),
	}
}

#[cfg(feature = "polkadot")]
pub(in crate::agents::relaychain_agent) mod polkadot {
	use crate::agents::relaychain_agent::call::*;

	#[derive(Encode, Decode, RuntimeDebug)]
	pub(in crate::agents::relaychain_agent) enum RelayCall<T: Config> {
		#[codec(index = 20)]
		ConvictionVoting(ConvictionVoting<T>),
		#[codec(index = 26)]
		Utility(Utility<Self>),
	}
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub(in crate::agents::relaychain_agent) enum ConvictionVoting<T: Config> {
	#[codec(index = 0)]
	Vote(#[codec(compact)] PollIndex, AccountVote<BalanceOf<T>>),
	#[codec(index = 3)]
	Unlock(PollClass, <T::Lookup as StaticLookup>::Source),
	#[codec(index = 4)]
	RemoveVote(Option<PollClass>, PollIndex),
}

impl<T: Config> ConvictionVotingCall<T> for RelayCall<T> {
	fn vote(poll_index: PollIndex, vote: AccountVote<BalanceOf<T>>) -> Self {
		Self::ConvictionVoting(ConvictionVoting::Vote(poll_index, vote))
	}

	fn remove_vote(class: Option<PollClass>, poll_index: PollIndex) -> Self {
		Self::ConvictionVoting(ConvictionVoting::RemoveVote(class, poll_index))
	}
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub(in crate::agents::relaychain_agent) enum Utility<Call> {
	#[codec(index = 1)]
	AsDerivative(DerivativeIndex, Box<Call>),
	#[codec(index = 2)]
	BatchAll(Vec<Call>),
}

impl<T: Config> UtilityCall<RelayCall<T>> for RelayCall<T> {
	fn as_derivative(derivative_index: DerivativeIndex, call: Self) -> Self {
		Self::Utility(Utility::AsDerivative(derivative_index, Box::new(call)))
	}

	fn batch_all(calls: Vec<Self>) -> Self {
		Self::Utility(Utility::BatchAll(calls))
	}
}

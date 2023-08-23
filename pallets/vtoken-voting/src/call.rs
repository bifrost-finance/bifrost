// This file is part of Bifrost.

// Copyright (C) 2019-2022 Liebi Technologies (UK) Ltd.
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

use crate::{BalanceOf, Config, PollIndexOf};
use codec::{Decode, Encode};
use frame_support::RuntimeDebug;
use pallet_conviction_voting::AccountVote;
use sp_runtime::traits::StaticLookup;
use sp_std::boxed::Box;

#[cfg(feature = "kusama")]
#[derive(Encode, Decode, RuntimeDebug)]
pub enum RelayCall<T: Config> {
	#[codec(index = 20)]
	ConvictionVoting(ConvictionVotingCall<T>),
	#[codec(index = 24)]
	Utility(UtilityCall<Self>),
}

#[cfg(any(feature = "kusama", feature = "polkadot"))]
impl<T: Config> RelayCall<T> {
	pub fn get_derivative_call(derivative_index: u16, call: Self) -> Self {
		Self::Utility(UtilityCall::AsDerivative(derivative_index, Box::new(call)))
	}
}

#[cfg(feature = "polkadot")]
#[derive(Encode, Decode, RuntimeDebug)]
pub enum RelayCall<T: Config> {
	#[codec(index = 20)]
	ConvictionVoting(ConvictionVotingCall<T>),
	#[codec(index = 26)]
	Utility(UtilityCall<Self>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum ConvictionVotingCall<T: Config> {
	#[codec(index = 0)]
	Vote(#[codec(compact)] PollIndexOf<T>, AccountVote<BalanceOf<T>>),
	#[codec(index = 3)]
	Unlock(u16, <T::Lookup as StaticLookup>::Source),
	#[codec(index = 4)]
	RemoveVote(Option<u16>, PollIndexOf<T>),
}

#[derive(Encode, Decode, RuntimeDebug, Clone)]
pub enum UtilityCall<Call> {
	#[codec(index = 1)]
	AsDerivative(u16, Box<Call>),
}

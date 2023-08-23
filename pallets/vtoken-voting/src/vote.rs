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

use codec::{Decode, Encode, EncodeLike, MaxEncodedLen};
use frame_support::pallet_prelude::*;
use pallet_conviction_voting::{AccountVote, Conviction};
use sp_std::{fmt::Debug, prelude::*};

/// Info regarding a referendum, present or past.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ReferendumInfo<
	TrackId: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
	Moment: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone + EncodeLike,
	Tally: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
> {
	/// Referendum has been submitted and is being voted on.
	Ongoing(ReferendumStatus<TrackId, Moment, Tally>),
	/// Referendum finished.
	Completed(Moment),
	/// Referendum finished with a kill.
	Killed,
}

/// Info regarding an ongoing referendum.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ReferendumStatus<
	TrackId: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
	Moment: Parameter + Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone + EncodeLike,
	Tally: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
> {
	/// The track of this referendum.
	pub track: TrackId,
	/// The time of submission. Once `UndecidingTimeout` passes, it may be closed by anyone if
	/// `deciding` is `None`.
	pub submitted: Moment,
	/// The current tally of votes in this referendum.
	pub tally: Tally,
}

/// A vote for a referendum of a particular account.
#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum VoteRole {
	/// A standard vote, one-way (approve or reject) with a given amount of conviction.
	Standard { aye: bool, conviction: Conviction },
	/// A split vote with balances given for both ways, and with no conviction, useful for
	/// parachains when voting.
	Split,
	/// A split vote with balances given for both ways as well as abstentions, and with no
	/// conviction, useful for parachains when voting, other off-chain aggregate accounts and
	/// individuals who wish to abstain.
	SplitAbstain,
}

impl<Balance> From<AccountVote<Balance>> for VoteRole {
	fn from(a: AccountVote<Balance>) -> VoteRole {
		match a {
			AccountVote::Standard { vote, balance: _ } =>
				VoteRole::Standard { aye: vote.aye, conviction: vote.conviction },
			AccountVote::Split { .. } => VoteRole::Split,
			AccountVote::SplitAbstain { .. } => VoteRole::SplitAbstain,
		}
	}
}

impl TryFrom<u8> for VoteRole {
	type Error = ();
	fn try_from(i: u8) -> Result<VoteRole, ()> {
		Ok(match i {
			0 => VoteRole::Standard { aye: true, conviction: Conviction::None },
			1 => VoteRole::Standard { aye: true, conviction: Conviction::Locked1x },
			2 => VoteRole::Standard { aye: true, conviction: Conviction::Locked2x },
			3 => VoteRole::Standard { aye: true, conviction: Conviction::Locked3x },
			4 => VoteRole::Standard { aye: true, conviction: Conviction::Locked4x },
			5 => VoteRole::Standard { aye: true, conviction: Conviction::Locked5x },
			6 => VoteRole::Standard { aye: true, conviction: Conviction::Locked6x },
			10 => VoteRole::Standard { aye: false, conviction: Conviction::None },
			11 => VoteRole::Standard { aye: false, conviction: Conviction::Locked1x },
			12 => VoteRole::Standard { aye: false, conviction: Conviction::Locked2x },
			13 => VoteRole::Standard { aye: false, conviction: Conviction::Locked3x },
			14 => VoteRole::Standard { aye: false, conviction: Conviction::Locked4x },
			15 => VoteRole::Standard { aye: false, conviction: Conviction::Locked5x },
			16 => VoteRole::Standard { aye: false, conviction: Conviction::Locked6x },
			20 => VoteRole::Split,
			21 => VoteRole::SplitAbstain,
			_ => return Err(()),
		})
	}
}

pub enum PollStatus<Tally, Moment> {
	None,
	Ongoing(Tally),
	Completed(Moment, bool),
}

impl<Tally, Moment> PollStatus<Tally, Moment> {
	pub fn ensure_ongoing(self) -> Option<Tally> {
		match self {
			Self::Ongoing(t) => Some(t),
			_ => None,
		}
	}
}

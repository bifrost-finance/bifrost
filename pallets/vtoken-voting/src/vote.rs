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

use bifrost_primitives::DerivativeIndex;
use frame_support::{
	pallet_prelude::*, traits::Get, CloneNoBound, EqNoBound, PartialEqNoBound, RuntimeDebugNoBound,
};
use pallet_conviction_voting::{Conviction, Delegations, Vote};
use parity_scale_codec::{Codec, Decode, Encode, EncodeLike, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, EnsureDivAssign, EnsureMulAssign, One, Zero},
	ArithmeticError, Saturating,
};
use sp_std::{fmt::Debug, prelude::*};

/// Info regarding a referendum, present or past.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ReferendumInfo<
	Moment: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone + EncodeLike,
	Tally: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
> {
	/// Referendum has been submitted and is being voted on.
	Ongoing(ReferendumStatus<Moment, Tally>),
	/// Referendum finished.
	Completed(Moment),
	/// Referendum finished with a kill.
	Killed(Moment),
}

/// Info regarding an ongoing referendum.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ReferendumStatus<
	Moment: Parameter + Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone + EncodeLike,
	Tally: Eq + PartialEq + Debug + Encode + Decode + TypeInfo + Clone,
> {
	/// The time of submission. Once `UndecidingTimeout` passes, it may be closed by anyone if
	/// `deciding` is `None`.
	pub submitted: Option<Moment>,
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

impl<Balance: Zero> From<VoteRole> for AccountVote<Balance> {
	fn from(v: VoteRole) -> AccountVote<Balance> {
		match v {
			VoteRole::Standard { aye, conviction } =>
				AccountVote::Standard { vote: Vote { aye, conviction }, balance: Zero::zero() },
			VoteRole::Split => AccountVote::Split { aye: Zero::zero(), nay: Zero::zero() },
			VoteRole::SplitAbstain => AccountVote::SplitAbstain {
				aye: Zero::zero(),
				nay: Zero::zero(),
				abstain: Zero::zero(),
			},
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
	Killed(Moment),
}

impl<Tally, Moment> PollStatus<Tally, Moment> {
	pub fn ensure_ongoing(self) -> Option<Tally> {
		match self {
			Self::Ongoing(t) => Some(t),
			_ => None,
		}
	}
}

/// A vote for a referendum of a particular account.
#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum AccountVote<Balance> {
	/// A standard vote, one-way (approve or reject) with a given amount of conviction.
	Standard { vote: Vote, balance: Balance },
	/// A split vote with balances given for both ways, and with no conviction, useful for
	/// parachains when voting.
	Split { aye: Balance, nay: Balance },
	/// A split vote with balances given for both ways as well as abstentions, and with no
	/// conviction, useful for parachains when voting, other off-chain aggregate accounts and
	/// individuals who wish to abstain.
	SplitAbstain { aye: Balance, nay: Balance, abstain: Balance },
}

impl<Balance: Saturating> AccountVote<Balance> {
	pub fn new_standard(vote: Vote, balance: Balance) -> Self {
		AccountVote::Standard { vote, balance }
	}

	pub fn as_standard_vote(&self) -> Option<Vote> {
		match self {
			AccountVote::Standard { vote, .. } => Some(*vote),
			_ => None,
		}
	}

	/// Returns `Some` of the lock periods that the account is locked for, assuming that the
	/// referendum passed iff `approved` is `true`.
	pub fn locked_if(self, _approved: bool) -> Option<(u32, Balance)> {
		// winning side: can only be removed after the lock period ends.
		match self {
			AccountVote::Standard { vote: Vote { conviction: Conviction::None, .. }, .. } => None,
			AccountVote::Standard { vote, balance } /* if vote.aye == _approved */ =>
				Some((vote.conviction.lock_periods(), balance)),
			_ => None,
		}
	}

	/// The total balance involved in this vote.
	pub fn balance(self) -> Balance {
		match self {
			AccountVote::Standard { balance, .. } => balance,
			AccountVote::Split { aye, nay } => aye.saturating_add(nay),
			AccountVote::SplitAbstain { aye, nay, abstain } =>
				aye.saturating_add(nay).saturating_add(abstain),
		}
	}

	/// Returns `Some` with whether the vote is an aye vote if it is standard, otherwise `None` if
	/// it is split.
	pub fn as_standard(self) -> Option<bool> {
		match self {
			AccountVote::Standard { vote, .. } => Some(vote.aye),
			_ => None,
		}
	}

	pub fn checked_add(&mut self, vote: AccountVote<Balance>) -> Result<(), ()>
	where
		Balance: One,
	{
		match (self, vote) {
			(
				AccountVote::Standard { vote: v1, balance: b1 },
				AccountVote::Standard { vote: v2, balance: b2 },
			) if *v1 == v2 => b1.saturating_accrue(b2),
			(AccountVote::Split { aye: a1, nay: n1 }, AccountVote::Split { aye: a2, nay: n2 }) => {
				a1.saturating_accrue(a2);
				n1.saturating_accrue(n2);
			},
			(
				AccountVote::SplitAbstain { aye: a1, nay: n1, abstain: ab1 },
				AccountVote::SplitAbstain { aye: a2, nay: n2, abstain: ab2 },
			) => {
				a1.saturating_accrue(a2);
				n1.saturating_accrue(n2);
				ab1.saturating_accrue(ab2);
			},
			_ => return Err(()),
		}
		Ok(())
	}

	pub fn checked_sub(&mut self, vote: AccountVote<Balance>) -> Result<(), ()>
	where
		Balance: One,
	{
		match (self, vote) {
			(
				AccountVote::Standard { vote: v1, balance: b1 },
				AccountVote::Standard { vote: v2, balance: b2 },
			) if *v1 == v2 => b1.saturating_reduce(b2),
			(AccountVote::Split { aye: a1, nay: n1 }, AccountVote::Split { aye: a2, nay: n2 }) => {
				a1.saturating_reduce(a2);
				n1.saturating_reduce(n2);
			},
			(
				AccountVote::SplitAbstain { aye: a1, nay: n1, abstain: ab1 },
				AccountVote::SplitAbstain { aye: a2, nay: n2, abstain: ab2 },
			) => {
				a1.saturating_reduce(a2);
				n1.saturating_reduce(n2);
				ab1.saturating_reduce(ab2);
			},
			_ => return Err(()),
		}
		Ok(())
	}

	pub fn checked_mul(&mut self, balance: Balance) -> Result<(), ArithmeticError>
	where
		Balance: Copy + EnsureMulAssign,
	{
		match self {
			AccountVote::Standard { vote: _, balance: b1 } => b1.ensure_mul_assign(balance)?,
			AccountVote::Split { aye: a1, nay: n1 } => {
				a1.ensure_mul_assign(balance)?;
				n1.ensure_mul_assign(balance)?;
			},
			AccountVote::SplitAbstain { aye: a1, nay: n1, abstain: ab1 } => {
				a1.ensure_mul_assign(balance)?;
				n1.ensure_mul_assign(balance)?;
				ab1.ensure_mul_assign(balance)?;
			},
		}
		Ok(())
	}

	pub fn checked_div(&mut self, balance: Balance) -> Result<(), ArithmeticError>
	where
		Balance: Copy + EnsureDivAssign,
	{
		match self {
			AccountVote::Standard { vote: _, balance: b1 } => b1.ensure_div_assign(balance)?,
			AccountVote::Split { aye: a1, nay: n1 } => {
				a1.ensure_div_assign(balance)?;
				n1.ensure_div_assign(balance)?;
			},
			AccountVote::SplitAbstain { aye: a1, nay: n1, abstain: ab1 } => {
				a1.ensure_div_assign(balance)?;
				n1.ensure_div_assign(balance)?;
				ab1.ensure_div_assign(balance)?;
			},
		}
		Ok(())
	}
}

/// A "prior" lock, i.e. a lock for some now-forgotten reason.
#[derive(
	Encode,
	Decode,
	Default,
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PriorLock<BlockNumber, Balance>(BlockNumber, Balance);

impl<BlockNumber: Ord + Copy + Zero, Balance: Ord + Copy + Zero> PriorLock<BlockNumber, Balance> {
	/// Accumulates an additional lock.
	pub fn accumulate(&mut self, until: BlockNumber, amount: Balance) {
		self.0 = self.0.max(until);
		self.1 = self.1.max(amount);
	}

	pub fn locked(&self) -> Balance {
		self.1
	}

	pub fn rejig(&mut self, now: BlockNumber) {
		if now >= self.0 {
			self.0 = Zero::zero();
			self.1 = Zero::zero();
		}
	}
}

/// Information concerning the delegation of some voting power.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Delegating<Balance, AccountId, BlockNumber> {
	/// The amount of balance delegated.
	pub balance: Balance,
	/// The account to which the voting power is delegated.
	pub target: AccountId,
	/// The conviction with which the voting power is delegated. When this gets undelegated, the
	/// relevant lock begins.
	pub conviction: Conviction,
	/// The total amount of delegations that this account has received, post-conviction-weighting.
	pub delegations: Delegations<Balance>,
	/// Any pre-existing locks from past voting/delegating activity.
	pub prior: PriorLock<BlockNumber, Balance>,
}

/// Information concerning the direct vote-casting of some voting power.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxVotes))]
#[codec(mel_bound(Balance: MaxEncodedLen, BlockNumber: MaxEncodedLen, PollIndex: MaxEncodedLen))]
pub struct Casting<Balance, BlockNumber, PollIndex, MaxVotes>
where
	MaxVotes: Get<u32>,
{
	/// The current votes of the account.
	pub votes: BoundedVec<(PollIndex, AccountVote<Balance>, DerivativeIndex, Balance), MaxVotes>,
	/// The total amount of delegations that this account has received, post-conviction-weighting.
	pub delegations: Delegations<Balance>,
	/// Any pre-existing locks from past voting/delegating activity.
	pub prior: PriorLock<BlockNumber, Balance>,
}

/// An indicator for what an account is doing; it can either be delegating or voting.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[scale_info(skip_type_params(MaxVotes))]
#[codec(mel_bound(
	Balance: MaxEncodedLen, AccountId: MaxEncodedLen, BlockNumber: MaxEncodedLen,
	PollIndex: MaxEncodedLen,
))]
pub enum Voting<Balance, AccountId, BlockNumber, PollIndex, MaxVotes>
where
	MaxVotes: Get<u32>,
{
	/// The account is voting directly.
	Casting(Casting<Balance, BlockNumber, PollIndex, MaxVotes>),
	/// The account is delegating `balance` of its balance to a `target` account with `conviction`.
	Delegating(Delegating<Balance, AccountId, BlockNumber>),
}

impl<Balance: Default, AccountId, BlockNumber: Zero, PollIndex, MaxVotes> Default
	for Voting<Balance, AccountId, BlockNumber, PollIndex, MaxVotes>
where
	MaxVotes: Get<u32>,
{
	fn default() -> Self {
		Voting::Casting(Casting {
			votes: Default::default(),
			delegations: Default::default(),
			prior: PriorLock(Zero::zero(), Default::default()),
		})
	}
}

impl<Balance, AccountId, BlockNumber, PollIndex, MaxVotes> AsMut<PriorLock<BlockNumber, Balance>>
	for Voting<Balance, AccountId, BlockNumber, PollIndex, MaxVotes>
where
	MaxVotes: Get<u32>,
{
	fn as_mut(&mut self) -> &mut PriorLock<BlockNumber, Balance> {
		match self {
			Voting::Casting(Casting { prior, .. }) => prior,
			Voting::Delegating(Delegating { prior, .. }) => prior,
		}
	}
}

impl<
		Balance: Saturating + Ord + Zero + Copy,
		BlockNumber: Ord + Copy + Zero,
		AccountId,
		PollIndex,
		MaxVotes,
	> Voting<Balance, AccountId, BlockNumber, PollIndex, MaxVotes>
where
	MaxVotes: Get<u32>,
{
	pub fn rejig(&mut self, now: BlockNumber) {
		AsMut::<PriorLock<BlockNumber, Balance>>::as_mut(self).rejig(now);
	}

	/// The amount of this account's balance that must currently be locked due to voting.
	pub fn locked_balance(&self) -> Balance {
		match self {
			Voting::Casting(Casting { votes, prior, .. }) =>
				votes.iter().map(|i| i.3).fold(prior.locked(), |a, i| a.max(i)),
			Voting::Delegating(Delegating { balance, prior, .. }) => *balance.max(&prior.locked()),
		}
	}

	pub fn locked_vtoken_balance(&self) -> Balance {
		match self {
			Voting::Casting(Casting { votes, .. }) =>
				votes.iter().map(|i| i.3).fold(Zero::zero(), |a, i| a.max(i)),
			Voting::Delegating(Delegating { .. }) => Zero::zero(),
		}
	}

	pub fn set_common(
		&mut self,
		delegations: Delegations<Balance>,
		prior: PriorLock<BlockNumber, Balance>,
	) {
		let (d, p) = match self {
			Voting::Casting(Casting { ref mut delegations, ref mut prior, .. }) =>
				(delegations, prior),
			Voting::Delegating(Delegating { ref mut delegations, ref mut prior, .. }) =>
				(delegations, prior),
		};
		*d = delegations;
		*p = prior;
	}
}

/// Info regarding an ongoing referendum.
#[derive(
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
	RuntimeDebugNoBound,
	TypeInfo,
	Encode,
	Decode,
	MaxEncodedLen,
)]
#[scale_info(skip_type_params(Total))]
#[codec(mel_bound(Votes: MaxEncodedLen))]
pub struct Tally<Votes: Clone + PartialEq + Eq + Debug + TypeInfo + Codec, Total> {
	/// The number of aye votes, expressed in terms of post-conviction lock-vote.
	pub ayes: Votes,
	/// The number of nay votes, expressed in terms of post-conviction lock-vote.
	pub nays: Votes,
	/// The basic number of aye votes, expressed pre-conviction.
	pub support: Votes,
	/// Dummy.
	dummy: PhantomData<Total>,
}

impl<
		Votes: Clone + Default + PartialEq + Eq + Debug + Copy + AtLeast32BitUnsigned + TypeInfo + Codec,
		Total: Get<Votes>,
	> Tally<Votes, Total>
{
	/// Create a new tally.
	pub fn from_vote(vote: Vote, balance: Votes) -> Self {
		let Delegations { votes, capital } = vote.conviction.votes(balance);
		Self {
			ayes: if vote.aye { votes } else { Zero::zero() },
			nays: if vote.aye { Zero::zero() } else { votes },
			support: capital,
			dummy: PhantomData,
		}
	}

	pub fn from_parts(
		ayes_with_conviction: Votes,
		nays_with_conviction: Votes,
		ayes: Votes,
	) -> Self {
		Self {
			ayes: ayes_with_conviction,
			nays: nays_with_conviction,
			support: ayes,
			dummy: PhantomData,
		}
	}

	pub fn account_vote(&self, conviction: Conviction) -> AccountVote<Votes> {
		if self.ayes >= self.nays {
			AccountVote::Standard {
				vote: Vote { aye: true, conviction },
				balance: self.ayes - self.nays,
			}
		} else {
			AccountVote::Standard {
				vote: Vote { aye: false, conviction },
				balance: self.nays - self.ayes,
			}
		}
	}

	/// Add an account's vote into the tally.
	pub fn add(&mut self, vote: AccountVote<Votes>) -> Option<()> {
		match vote {
			AccountVote::Standard { vote, balance } => {
				let Delegations { votes, capital } = vote.conviction.votes(balance);
				match vote.aye {
					true => {
						self.support = self.support.checked_add(&capital)?;
						self.ayes = self.ayes.checked_add(&votes)?
					},
					false => self.nays = self.nays.checked_add(&votes)?,
				}
			},
			AccountVote::Split { aye, nay } => {
				let aye = Conviction::None.votes(aye);
				let nay = Conviction::None.votes(nay);
				self.support = self.support.checked_add(&aye.capital)?;
				self.ayes = self.ayes.checked_add(&aye.votes)?;
				self.nays = self.nays.checked_add(&nay.votes)?;
			},
			AccountVote::SplitAbstain { aye, nay, abstain } => {
				let aye = Conviction::None.votes(aye);
				let nay = Conviction::None.votes(nay);
				let abstain = Conviction::None.votes(abstain);
				self.support =
					self.support.checked_add(&aye.capital)?.checked_add(&abstain.capital)?;
				self.ayes = self.ayes.checked_add(&aye.votes)?;
				self.nays = self.nays.checked_add(&nay.votes)?;
			},
		}
		Some(())
	}

	/// Remove an account's vote from the tally.
	pub fn remove(&mut self, vote: AccountVote<Votes>) -> Option<()> {
		match vote {
			AccountVote::Standard { vote, balance } => {
				let Delegations { votes, capital } = vote.conviction.votes(balance);
				match vote.aye {
					true => {
						self.support = self.support.checked_sub(&capital)?;
						self.ayes = self.ayes.checked_sub(&votes)?
					},
					false => self.nays = self.nays.checked_sub(&votes)?,
				}
			},
			AccountVote::Split { aye, nay } => {
				let aye = Conviction::None.votes(aye);
				let nay = Conviction::None.votes(nay);
				self.support = self.support.checked_sub(&aye.capital)?;
				self.ayes = self.ayes.checked_sub(&aye.votes)?;
				self.nays = self.nays.checked_sub(&nay.votes)?;
			},
			AccountVote::SplitAbstain { aye, nay, abstain } => {
				let aye = Conviction::None.votes(aye);
				let nay = Conviction::None.votes(nay);
				let abstain = Conviction::None.votes(abstain);
				self.support =
					self.support.checked_sub(&aye.capital)?.checked_sub(&abstain.capital)?;
				self.ayes = self.ayes.checked_sub(&aye.votes)?;
				self.nays = self.nays.checked_sub(&nay.votes)?;
			},
		}
		Some(())
	}

	/// Increment some amount of votes.
	pub fn increase(&mut self, approve: bool, delegations: Delegations<Votes>) {
		match approve {
			true => {
				self.support = self.support.saturating_add(delegations.capital);
				self.ayes = self.ayes.saturating_add(delegations.votes);
			},
			false => self.nays = self.nays.saturating_add(delegations.votes),
		}
	}

	/// Decrement some amount of votes.
	pub fn reduce(&mut self, approve: bool, delegations: Delegations<Votes>) {
		match approve {
			true => {
				self.support = self.support.saturating_sub(delegations.capital);
				self.ayes = self.ayes.saturating_sub(delegations.votes);
			},
			false => self.nays = self.nays.saturating_sub(delegations.votes),
		}
	}
}

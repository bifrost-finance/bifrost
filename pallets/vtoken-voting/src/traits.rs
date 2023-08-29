use crate::{vote::AccountVote, BalanceOf, Config, CurrencyIdOf, DerivativeIndex};
use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::{
	traits::{Get, VoteTally},
	CloneNoBound, EqNoBound, PartialEqNoBound, RuntimeDebugNoBound,
};
use pallet_conviction_voting::{Conviction, Delegations, Vote};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Zero},
	Perbill,
};
use std::{fmt::Debug, marker::PhantomData};
use xcm::v3::{MultiLocation, Weight as XcmWeight};

pub trait XcmDestWeightAndFeeHandler<T: Config> {
	fn get_vote(token: CurrencyIdOf<T>) -> Option<(XcmWeight, BalanceOf<T>)>;

	fn get_remove_vote(token: CurrencyIdOf<T>) -> Option<(XcmWeight, BalanceOf<T>)>;
}

pub trait DerivativeAccountHandler<T: Config> {
	fn check_derivative_index_exists(
		token: CurrencyIdOf<T>,
		derivative_index: DerivativeIndex,
	) -> bool;

	fn get_multilocation(
		token: CurrencyIdOf<T>,
		derivative_index: DerivativeIndex,
	) -> Option<MultiLocation>;

	fn get_stake_info(
		token: CurrencyIdOf<T>,
		derivative_index: DerivativeIndex,
	) -> Option<(BalanceOf<T>, BalanceOf<T>)>;
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
		Class,
	> VoteTally<Votes, Class> for Tally<Votes, Total>
{
	fn new(_: Class) -> Self {
		Self { ayes: Zero::zero(), nays: Zero::zero(), support: Zero::zero(), dummy: PhantomData }
	}

	fn ayes(&self, _: Class) -> Votes {
		self.ayes
	}

	fn support(&self, _: Class) -> Perbill {
		Perbill::from_rational(self.support, Total::get())
	}

	fn approval(&self, _: Class) -> Perbill {
		Perbill::from_rational(self.ayes, self.ayes.saturating_add(self.nays))
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn unanimity(_: Class) -> Self {
		Self { ayes: Total::get(), nays: Zero::zero(), support: Total::get(), dummy: PhantomData }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn rejection(_: Class) -> Self {
		Self { ayes: Zero::zero(), nays: Total::get(), support: Total::get(), dummy: PhantomData }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn from_requirements(support: Perbill, approval: Perbill, _: Class) -> Self {
		let support = support.mul_ceil(Total::get());
		let ayes = approval.mul_ceil(support);
		Self { ayes, nays: support - ayes, support, dummy: PhantomData }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn setup(_: Class, _: Perbill) {}
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

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

// Ensure we're ,no_std, when compiling for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod agents;
mod vote;

// pub mod migration;
pub mod traits;
pub mod weights;

pub use crate::vote::{AccountVote, PollStatus, ReferendumInfo, ReferendumStatus, VoteRole};
use crate::{
	agents::{BifrostAgent, RelaychainAgent},
	traits::VotingAgent,
	vote::{Casting, Tally, Voting},
};
use bifrost_primitives::{
	currency::{BNC, DOT, KSM, VBNC, VDOT, VKSM},
	traits::{DerivativeAccountHandler, VTokenSupplyProvider, XcmDestWeightAndFeeHandler},
	CurrencyId, DerivativeIndex, XcmOperationType,
};
use cumulus_primitives_core::{ParaId, QueryId, Response};
use frame_support::{
	dispatch::{GetDispatchInfo, PostDispatchInfo},
	pallet_prelude::*,
	traits::{Get, LockIdentifier},
};
use frame_system::{
	pallet_prelude::{BlockNumberFor, *},
	RawOrigin,
};
use orml_traits::{MultiCurrency, MultiLockableCurrency};
pub use pallet::*;
pub use pallet_conviction_voting::AccountVote as ConvictionVotingAccountVote;
use pallet_conviction_voting::{Conviction, UnvoteScope, Vote};
use sp_runtime::{
	traits::{
		BlockNumberProvider, Bounded, CheckedDiv, CheckedMul, Dispatchable, Saturating,
		UniqueSaturatedInto, Zero,
	},
	ArithmeticError, Perbill,
};
use sp_std::{boxed::Box, vec::Vec};
pub use weights::WeightInfo;
use xcm::v4::{prelude::*, Location, Weight as XcmWeight};

const CONVICTION_VOTING_ID: LockIdentifier = *b"vtvoting";

type PollIndex = u32;
type PollClass = u16;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::CurrencyId;

pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type TallyOf<T> = Tally<BalanceOf<T>, ()>;

type VotingOf<T> =
	Voting<BalanceOf<T>, AccountIdOf<T>, BlockNumberFor<T>, PollIndex, <T as Config>::MaxVotes>;

pub type ReferendumInfoOf<T> = ReferendumInfo<BlockNumberFor<T>, TallyOf<T>>;

type VotingAgentBoxType<T> = Box<dyn VotingAgent<T>>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::traits::CallerTrait;

	/// The current storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_xcm::Config {
		type RuntimeEvent: IsType<<Self as frame_system::Config>::RuntimeEvent> + From<Event<Self>>;

		type RuntimeOrigin: IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>;

		type RuntimeCall: IsType<<Self as pallet_xcm::Config>::RuntimeCall>
			+ From<Call<Self>>
			+ GetDispatchInfo
			+ Dispatchable<
				RuntimeOrigin = <Self as Config>::RuntimeOrigin,
				PostInfo = PostDispatchInfo,
			> + Parameter;

		type MultiCurrency: MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type ControlOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		type ResponseOrigin: EnsureOrigin<
			<Self as frame_system::Config>::RuntimeOrigin,
			Success = Location,
		>;

		type XcmDestWeightAndFee: XcmDestWeightAndFeeHandler<CurrencyIdOf<Self>, BalanceOf<Self>>;

		type DerivativeAccount: DerivativeAccountHandler<
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
			AccountIdOf<Self>,
		>;

		type RelaychainBlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

		type VTokenSupplyProvider: VTokenSupplyProvider<CurrencyIdOf<Self>, BalanceOf<Self>>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;

		/// The maximum number of concurrent votes an account may have.
		#[pallet::constant]
		type MaxVotes: Get<u32>;

		#[pallet::constant]
		type QueryTimeout: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type ReferendumCheckInterval: Get<BlockNumberFor<Self>>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		type PalletsOrigin: CallerTrait<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A vote has been cast.
		///
		/// - `who`: The account that cast the vote.
		/// - `vtoken`: The token used for voting.
		/// - `poll_index`: The index of the poll being voted on.
		/// - `token_vote`: The vote cast using the token.
		/// - `delegator_vote`: The vote cast by a delegator.
		Voted {
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			token_vote: AccountVote<BalanceOf<T>>,
			delegator_vote: AccountVote<BalanceOf<T>>,
		},

		/// A user's vote has been unlocked, allowing them to retrieve their tokens.
		///
		/// - `who`: The account whose tokens are unlocked.
		/// - `vtoken`: The token that was locked during voting.
		/// - `poll_index`: The index of the poll associated with the unlocking.
		Unlocked { who: AccountIdOf<T>, vtoken: CurrencyIdOf<T>, poll_index: PollIndex },

		/// A delegator's vote has been removed.
		///
		/// - `who`: The account that dispatched remove_delegator_vote.
		/// - `vtoken`: The token associated with the delegator's vote.
		/// - `derivative_index`: The index of the derivative.
		DelegatorVoteRemoved {
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			derivative_index: DerivativeIndex,
		},

		/// A delegator has been added.
		///
		/// - `vtoken`: The token associated with the delegator.
		/// - `derivative_index`: The index of the derivative being added for the delegator.
		DelegatorAdded { vtoken: CurrencyIdOf<T>, derivative_index: DerivativeIndex },

		/// A new referendum information has been created.
		///
		/// - `vtoken`: The token associated with the referendum.
		/// - `poll_index`: The index of the poll.
		/// - `info`: The referendum information (details about the poll).
		ReferendumInfoCreated {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			info: ReferendumInfoOf<T>,
		},

		/// Referendum information has been updated.
		///
		/// - `vtoken`: The token associated with the referendum.
		/// - `poll_index`: The index of the poll.
		/// - `info`: The updated referendum information.
		ReferendumInfoSet {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			info: ReferendumInfoOf<T>,
		},

		/// The vote locking period has been set.
		///
		/// - `vtoken`: The token for which the locking period is being set.
		/// - `locking_period`: The period for which votes will be locked (in block numbers).
		VoteLockingPeriodSet { vtoken: CurrencyIdOf<T>, locking_period: BlockNumberFor<T> },

		/// The undeciding timeout period has been set.
		///
		/// - `vtoken`: The token associated with the timeout.
		/// - `undeciding_timeout`: The period of time before a poll is considered undecided.
		UndecidingTimeoutSet { vtoken: CurrencyIdOf<T>, undeciding_timeout: BlockNumberFor<T> },

		/// A referendum has been killed (cancelled or ended).
		///
		/// - `vtoken`: The token associated with the referendum.
		/// - `poll_index`: The index of the poll being killed.
		ReferendumKilled { vtoken: CurrencyIdOf<T>, poll_index: PollIndex },

		/// A notification about the result of a vote has been sent.
		///
		/// - `vtoken`: The token associated with the poll.
		/// - `poll_index`: The index of the poll.
		/// - `success`: Whether the notification was successful or not.
		VoteNotified { vtoken: CurrencyIdOf<T>, poll_index: PollIndex, success: bool },

		/// A notification about the removal of a delegator's vote has been sent.
		///
		/// - `vtoken`: The token associated with the poll.
		/// - `poll_index`: The index of the poll.
		/// - `success`: Whether the notification was successful or not.
		DelegatorVoteRemovedNotified {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			success: bool,
		},

		/// A response has been received from a specific location.
		///
		/// - `responder`: The location that sent the response.
		/// - `query_id`: The ID of the query that was responded to.
		/// - `response`: The content of the response.
		ResponseReceived { responder: Location, query_id: QueryId, response: Response },

		/// The vote cap ratio has been set.
		///
		/// - `vtoken`: The token associated with the cap.
		/// - `vote_cap_ratio`: The maximum allowed ratio for the vote.
		VoteCapRatioSet { vtoken: CurrencyIdOf<T>, vote_cap_ratio: Perbill },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// XCM execution Failure
		XcmFailure,
		/// The given currency is not supported.
		VTokenNotSupport,
		/// Derivative index occupied.
		DerivativeIndexOccupied,
		/// Another vote is pending.
		PendingVote,
		/// Another update referendum status is pending.
		PendingUpdateReferendumStatus,
		/// No data available in storage.
		NoData,
		/// Poll is not ongoing.
		NotOngoing,
		/// Poll is not completed.
		NotCompleted,
		/// Poll is not killed.
		NotKilled,
		/// Poll is not expired.
		NotExpired,
		/// The given account did not vote on the poll.
		NotVoter,
		/// The actor has no permission to conduct the action.
		NoPermission,
		/// The actor has no permission to conduct the action right now but will do in the future.
		NoPermissionYet,
		/// The account is already delegating.
		AlreadyDelegating,
		/// Too high a balance was provided that the account cannot afford.
		InsufficientFunds,
		/// Maximum number of votes reached.
		MaxVotesReached,
		/// Maximum number of items reached.
		TooMany,
		/// The given vote is not Standard vote.
		NotStandardVote,
		/// The given conviction is not valid.
		InvalidConviction,
		/// The given value is out of range.
		OutOfRange,
		InvalidCallDispatch,
		CallDecodeFailed,
	}

	/// Information concerning any given referendum.
	#[pallet::storage]
	pub type ReferendumInfoFor<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		CurrencyIdOf<T>,
		Twox64Concat,
		PollIndex,
		ReferendumInfoOf<T>,
	>;

	/// All voting for a particular voter in a particular voting class. We store the balance for the
	/// number of votes that we have recorded.
	#[pallet::storage]
	pub type VotingFor<T: Config> =
		StorageMap<_, Twox64Concat, AccountIdOf<T>, VotingOf<T>, ValueQuery>;

	/// The voting classes which have a non-zero lock requirement and the lock amounts which they
	/// require. The actual amount locked on behalf of this pallet should always be the maximum of
	/// this list.
	#[pallet::storage]
	pub type ClassLocksFor<T: Config> = StorageMap<
		_,
		Twox64Concat,
		AccountIdOf<T>,
		BoundedVec<(CurrencyIdOf<T>, BalanceOf<T>), T::MaxVotes>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type PendingReferendumInfo<T: Config> =
		StorageMap<_, Twox64Concat, QueryId, (CurrencyIdOf<T>, PollIndex)>;

	#[pallet::storage]
	pub type PendingVotingInfo<T: Config> = StorageMap<
		_,
		Twox64Concat,
		QueryId,
		(
			CurrencyIdOf<T>,
			PollIndex,
			DerivativeIndex,
			AccountIdOf<T>,
			Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>,
		),
	>;

	#[pallet::storage]
	pub type PendingRemoveDelegatorVote<T: Config> =
		StorageMap<_, Twox64Concat, QueryId, (CurrencyIdOf<T>, PollIndex, DerivativeIndex)>;

	#[pallet::storage]
	pub type VoteLockingPeriod<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BlockNumberFor<T>>;

	#[pallet::storage]
	pub type UndecidingTimeout<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BlockNumberFor<T>>;

	#[pallet::storage]
	pub type Delegators<T: Config> = StorageMap<
		_,
		Twox64Concat,
		CurrencyIdOf<T>,
		BoundedVec<DerivativeIndex, ConstU32<100>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type VoteCapRatio<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, Perbill, ValueQuery>;

	#[pallet::storage]
	pub type DelegatorVotes<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		CurrencyIdOf<T>,
		Twox64Concat,
		PollIndex,
		BoundedVec<(DerivativeIndex, AccountVote<BalanceOf<T>>), ConstU32<100>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type PendingDelegatorVotes<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		CurrencyIdOf<T>,
		Twox64Concat,
		PollIndex,
		BoundedVec<(DerivativeIndex, AccountVote<BalanceOf<T>>), ConstU32<100>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type ReferendumTimeout<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BlockNumberFor<T>,
		BoundedVec<(CurrencyIdOf<T>, PollIndex), ConstU32<50>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type VoteDelegatorFor<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, AccountIdOf<T>>,
			NMapKey<Twox64Concat, CurrencyIdOf<T>>,
			NMapKey<Twox64Concat, PollIndex>,
		),
		DerivativeIndex,
	>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub delegators: Vec<(CurrencyIdOf<T>, Vec<DerivativeIndex>)>,
		pub undeciding_timeouts: Vec<(CurrencyIdOf<T>, BlockNumberFor<T>)>,
		pub vote_cap_ratio: Vec<(CurrencyIdOf<T>, Perbill)>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			self.delegators.iter().for_each(|(vtoken, delegators)| {
				Delegators::<T>::insert(vtoken, BoundedVec::truncate_from(delegators.clone()));
			});

			self.undeciding_timeouts.iter().for_each(|(vtoken, undeciding_timeout)| {
				UndecidingTimeout::<T>::insert(vtoken, undeciding_timeout);
			});

			self.vote_cap_ratio.iter().for_each(|(vtoken, cap_ratio)| {
				VoteCapRatio::<T>::insert(vtoken, cap_ratio);
			});
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(n: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let db_weight = T::DbWeight::get();
			let mut used_weight = db_weight.reads(3);
			if remaining_weight.any_lt(used_weight) ||
				n % T::ReferendumCheckInterval::get() != Zero::zero()
			{
				return Weight::zero();
			}
			let relay_current_block_number =
				T::RelaychainBlockNumberProvider::current_block_number();

			for relay_block_number in ReferendumTimeout::<T>::iter_keys() {
				if relay_current_block_number >= relay_block_number {
					let info_list = ReferendumTimeout::<T>::get(relay_block_number);
					let len = info_list.len() as u64;
					let temp_weight = db_weight.reads_writes(len, len) + db_weight.writes(1);
					if remaining_weight.any_lt(used_weight + temp_weight) {
						return used_weight;
					}
					used_weight += temp_weight;
					for (vtoken, poll_index) in info_list.iter() {
						ReferendumInfoFor::<T>::mutate(vtoken, poll_index, |maybe_info| {
							match maybe_info {
								Some(info) =>
									if let ReferendumInfo::Ongoing(_) = info {
										*info =
											ReferendumInfo::Completed(relay_current_block_number);
									},
								None => {},
							}
						});
					}
					ReferendumTimeout::<T>::remove(relay_block_number);
				}
			}

			used_weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(
			<T as Config>::WeightInfo::vote_new().max(<T as Config>::WeightInfo::vote_existing())
			+ <T as Config>::WeightInfo::notify_vote()
		)]
		pub fn vote(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			vtoken_vote: AccountVote<BalanceOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			ensure!(UndecidingTimeout::<T>::contains_key(vtoken), Error::<T>::NoData);
			Self::ensure_no_pending_vote(vtoken, poll_index)?;

			let token_vote = Self::compute_token_vote(vtoken, vtoken_vote)?;

			// create referendum if not exist
			let mut submitted = false;
			if !ReferendumInfoFor::<T>::contains_key(vtoken, poll_index) {
				ReferendumInfoFor::<T>::insert(
					vtoken,
					poll_index,
					ReferendumInfo::Ongoing(ReferendumStatus {
						submitted: None,
						tally: TallyOf::<T>::from_parts(Zero::zero(), Zero::zero(), Zero::zero()),
					}),
				);
			} else {
				Self::ensure_referendum_ongoing(vtoken, poll_index)?;
				submitted = true;
			}

			// record vote info
			let (maybe_old_vote, maybe_total_vote) =
				Self::try_vote(&who, vtoken, poll_index, token_vote, vtoken_vote.balance())?;

			let delegator_total_vote = Self::compute_delegator_total_vote(
				vtoken,
				maybe_total_vote.ok_or(Error::<T>::NoData)?,
			)?;
			let new_delegator_votes =
				Self::allocate_delegator_votes(vtoken, poll_index, delegator_total_vote)?;

			PendingDelegatorVotes::<T>::try_mutate(vtoken, poll_index, |item| -> DispatchResult {
				for (derivative_index, vote) in new_delegator_votes.iter() {
					item.try_push((*derivative_index, *vote)).map_err(|_| Error::<T>::TooMany)?;
				}
				Ok(())
			})?;

			let voting_agent = Self::get_voting_agent(&vtoken)?;
			voting_agent.delegate_vote(
				who.clone(),
				vtoken,
				poll_index,
				submitted,
				new_delegator_votes.clone(),
				maybe_old_vote,
			)?;

			Self::deposit_event(Event::<T>::Voted {
				who,
				vtoken,
				poll_index,
				token_vote,
				delegator_vote: new_delegator_votes[0].1,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::unlock())]
		pub fn unlock(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] poll_index: PollIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			Self::ensure_referendum_completed(vtoken, poll_index)
				.or(Self::ensure_referendum_killed(vtoken, poll_index))
				.map_err(|_| Error::<T>::NoPermissionYet)?;
			Self::ensure_no_pending_vote(vtoken, poll_index)?;

			Self::try_remove_vote(&who, vtoken, poll_index, UnvoteScope::OnlyExpired)?;
			Self::update_lock(&who, vtoken)?;

			Self::deposit_event(Event::<T>::Unlocked { who, vtoken, poll_index });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(
			<T as Config>::WeightInfo::remove_delegator_vote()
			+ <T as Config>::WeightInfo::notify_remove_delegator_vote()
		)]
		pub fn remove_delegator_vote(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] class: PollClass,
			#[pallet::compact] poll_index: PollIndex,
			#[pallet::compact] derivative_index: DerivativeIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			ensure!(DelegatorVotes::<T>::get(vtoken, poll_index).len() > 0, Error::<T>::NoData);
			Self::ensure_referendum_expired(vtoken, poll_index)?;

			let voting_agent = Self::get_voting_agent(&vtoken)?;
			voting_agent.delegate_remove_delegator_vote(
				vtoken,
				poll_index,
				class,
				derivative_index,
			)?;

			Self::deposit_event(Event::<T>::DelegatorVoteRemoved { who, vtoken, derivative_index });

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::kill_referendum())]
		pub fn kill_referendum(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] poll_index: PollIndex,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			Self::ensure_referendum_completed(vtoken, poll_index)?;

			ReferendumInfoFor::<T>::insert(
				vtoken,
				poll_index,
				ReferendumInfo::Killed(T::RelaychainBlockNumberProvider::current_block_number()),
			);

			Self::deposit_event(Event::<T>::ReferendumKilled { vtoken, poll_index });

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::add_delegator())]
		pub fn add_delegator(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] derivative_index: DerivativeIndex,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			let token = CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?;
			ensure!(
				T::DerivativeAccount::check_derivative_index_exists(token, derivative_index),
				Error::<T>::NoData
			);
			ensure!(
				!Delegators::<T>::get(vtoken).contains(&derivative_index),
				Error::<T>::DerivativeIndexOccupied
			);

			Delegators::<T>::try_mutate(vtoken, |vec| -> DispatchResult {
				vec.try_push(derivative_index).map_err(|_| Error::<T>::TooMany)?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::DelegatorAdded { vtoken, derivative_index });

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::set_referendum_status())]
		pub fn set_referendum_status(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] poll_index: PollIndex,
			info: ReferendumInfoOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_vtoken(&vtoken)?;

			ensure!(ReferendumInfoFor::<T>::contains_key(vtoken, poll_index), Error::<T>::NoData);
			ReferendumInfoFor::<T>::insert(vtoken, poll_index, info.clone());

			Self::deposit_event(Event::<T>::ReferendumInfoSet { vtoken, poll_index, info });

			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::set_vote_locking_period())]
		pub fn set_vote_locking_period(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			locking_period: BlockNumberFor<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			VoteLockingPeriod::<T>::insert(vtoken, locking_period);
			Self::deposit_event(Event::<T>::VoteLockingPeriodSet { vtoken, locking_period });

			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::set_undeciding_timeout())]
		pub fn set_undeciding_timeout(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			undeciding_timeout: BlockNumberFor<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			UndecidingTimeout::<T>::insert(vtoken, undeciding_timeout);
			Self::deposit_event(Event::<T>::UndecidingTimeoutSet { vtoken, undeciding_timeout });

			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::notify_vote())]
		pub fn notify_vote(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			let responder = Self::ensure_xcm_response_or_governance(origin)?;
			let success = Response::DispatchResult(MaybeErrorCode::Success) == response;

			if let Some((vtoken, poll_index, derivative_index, who, maybe_old_vote)) =
				PendingVotingInfo::<T>::get(query_id)
			{
				Self::handle_vote_result(
					success,
					who,
					vtoken,
					poll_index,
					maybe_old_vote,
					derivative_index,
				)?;

				PendingVotingInfo::<T>::remove(query_id);
				Self::deposit_event(Event::<T>::VoteNotified { vtoken, poll_index, success });
			}

			if let Some((vtoken, poll_index)) = PendingReferendumInfo::<T>::get(query_id) {
				if success {
					ReferendumInfoFor::<T>::try_mutate_exists(
						vtoken,
						poll_index,
						|maybe_info| -> DispatchResult {
							if let Some(info) = maybe_info {
								if let ReferendumInfo::Ongoing(status) = info {
									let relay_current_block_number =
										T::RelaychainBlockNumberProvider::current_block_number();
									status.submitted = Some(relay_current_block_number);
									ReferendumTimeout::<T>::mutate(
										relay_current_block_number.saturating_add(
											UndecidingTimeout::<T>::get(vtoken)
												.ok_or(Error::<T>::NoData)?,
										),
										|ref_vec| {
											ref_vec
												.try_push((vtoken, poll_index))
												.map_err(|_| Error::<T>::TooMany)
										},
									)?;
									Self::deposit_event(Event::<T>::ReferendumInfoCreated {
										vtoken,
										poll_index,
										info: info.clone(),
									});
								}
							}
							Ok(())
						},
					)?;
				} else {
					ReferendumInfoFor::<T>::remove(vtoken, poll_index);
				}
				PendingReferendumInfo::<T>::remove(query_id);
			}

			Self::deposit_event(Event::<T>::ResponseReceived { responder, query_id, response });

			Ok(())
		}

		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::notify_remove_delegator_vote())]
		pub fn notify_remove_delegator_vote(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			let responder = Self::ensure_xcm_response_or_governance(origin)?;
			if let Some((vtoken, poll_index, _derivative_index)) =
				PendingRemoveDelegatorVote::<T>::get(query_id)
			{
				let success = Response::DispatchResult(MaybeErrorCode::Success) == response;
				if success {
					Self::handle_remove_delegator_vote_success(vtoken, poll_index);
				}
				PendingRemoveDelegatorVote::<T>::remove(query_id);
				Self::deposit_event(Event::<T>::DelegatorVoteRemovedNotified {
					vtoken,
					poll_index,
					success,
				});
			}
			Self::deposit_event(Event::<T>::ResponseReceived { responder, query_id, response });

			Ok(())
		}

		#[pallet::call_index(11)]
		#[pallet::weight(<T as Config>::WeightInfo::set_vote_cap_ratio())]
		pub fn set_vote_cap_ratio(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			vote_cap_ratio: Perbill,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			VoteCapRatio::<T>::insert(vtoken, vote_cap_ratio);
			Self::deposit_event(Event::<T>::VoteCapRatioSet { vtoken, vote_cap_ratio });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn handle_remove_delegator_vote_success(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
		) {
			DelegatorVotes::<T>::remove(vtoken, poll_index);
		}

		pub(crate) fn handle_vote_result(
			success: bool,
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			maybe_old_vote: Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>,
			derivative_index: DerivativeIndex,
		) -> DispatchResult {
			if !success {
				// rollback vote
				let _ = PendingDelegatorVotes::<T>::clear(u32::MAX, None);
				Self::try_remove_vote(&who, vtoken, poll_index, UnvoteScope::Any)?;
				Self::update_lock(&who, vtoken)?;
				if let Some((old_vote, vtoken_balance)) = maybe_old_vote {
					Self::try_vote(&who, vtoken, poll_index, old_vote, vtoken_balance)?;
				}
			} else {
				if !VoteDelegatorFor::<T>::contains_key((&who, vtoken, poll_index)) {
					VoteDelegatorFor::<T>::insert((&who, vtoken, poll_index), derivative_index);
				}
				DelegatorVotes::<T>::remove(vtoken, poll_index);
				DelegatorVotes::<T>::try_mutate(vtoken, poll_index, |item| -> DispatchResult {
					for (derivative_index, vote) in
						PendingDelegatorVotes::<T>::take(vtoken, poll_index).iter()
					{
						item.try_push((*derivative_index, *vote))
							.map_err(|_| Error::<T>::TooMany)?;
					}
					Ok(())
				})?;
			}

			Ok(())
		}

		pub(crate) fn send_xcm_vote_message(
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			submitted: bool,
			new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
			maybe_old_vote: Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>,
		) -> DispatchResult {
			let notify_call = Call::<T>::notify_vote { query_id: 0, response: Default::default() };
			let (weight, extra_fee) = T::XcmDestWeightAndFee::get_operation_weight_and_fee(
				CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?,
				XcmOperationType::Vote,
			)
			.ok_or(Error::<T>::NoData)?;

			let derivative_index = new_delegator_votes[0].0;

			let voting_agent = Self::get_voting_agent(&vtoken)?;
			let encode_call = voting_agent.vote_call_encode(
				new_delegator_votes.clone(),
				poll_index,
				derivative_index,
			)?;

			Self::send_xcm_with_notify(
				voting_agent.location(),
				encode_call,
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

		pub(crate) fn send_xcm_remove_delegator_vote_message(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			class: PollClass,
			derivative_index: DerivativeIndex,
		) -> DispatchResult {
			let voting_agent = Self::get_voting_agent(&vtoken)?;
			let encode_call = voting_agent.remove_delegator_vote_call_encode(
				class,
				poll_index,
				derivative_index,
			)?;
			let notify_call = Call::<T>::notify_remove_delegator_vote {
				query_id: 0,
				response: Default::default(),
			};

			let (weight, extra_fee) = T::XcmDestWeightAndFee::get_operation_weight_and_fee(
				CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?,
				XcmOperationType::RemoveVote,
			)
			.ok_or(Error::<T>::NoData)?;

			Self::send_xcm_with_notify(
				voting_agent.location(),
				encode_call,
				notify_call,
				weight,
				extra_fee,
				|query_id| {
					PendingRemoveDelegatorVote::<T>::insert(
						query_id,
						(vtoken, poll_index, derivative_index),
					);
				},
			)?;

			Ok(())
		}

		pub(crate) fn send_xcm_with_notify(
			responder_location: Location,
			encode_call: Vec<u8>,
			notify_call: Call<T>,
			transact_weight: XcmWeight,
			extra_fee: BalanceOf<T>,
			f: impl FnOnce(QueryId) -> (),
		) -> DispatchResult {
			let now = frame_system::Pallet::<T>::block_number();
			let timeout = now.saturating_add(T::QueryTimeout::get());
			let notify_runtime_call = <T as Config>::RuntimeCall::from(notify_call);
			let notify_call_weight = notify_runtime_call.get_dispatch_info().weight;
			let query_id = pallet_xcm::Pallet::<T>::new_notify_query(
				responder_location.clone(),
				notify_runtime_call,
				timeout,
				xcm::v4::Junctions::Here,
			);
			f(query_id);

			let xcm_message = Self::construct_xcm_message(
				encode_call,
				extra_fee,
				transact_weight,
				notify_call_weight,
				query_id,
			)?;

			xcm::v4::send_xcm::<T::XcmRouter>(responder_location, xcm_message)
				.map_err(|_| Error::<T>::XcmFailure)?;

			Ok(())
		}

		pub(crate) fn construct_xcm_message(
			call: Vec<u8>,
			extra_fee: BalanceOf<T>,
			transact_weight: XcmWeight,
			notify_call_weight: XcmWeight,
			query_id: QueryId,
		) -> Result<Xcm<()>, Error<T>> {
			let para_id = T::ParachainId::get().into();
			let asset = Asset {
				id: AssetId(Location::here()),
				fun: Fungible(UniqueSaturatedInto::<u128>::unique_saturated_into(extra_fee)),
			};
			let xcm_message = sp_std::vec![
				WithdrawAsset(asset.clone().into()),
				BuyExecution { fees: asset, weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: transact_weight,
					call: call.into(),
				},
				ReportTransactStatus(QueryResponseInfo {
					destination: Location::from(Parachain(para_id)),
					query_id,
					max_weight: notify_call_weight,
				}),
				RefundSurplus,
				DepositAsset {
					assets: All.into(),
					beneficiary: Location::new(0, [Parachain(para_id)]),
				},
			];

			Ok(Xcm(xcm_message))
		}

		fn try_vote(
			who: &AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			vote: AccountVote<BalanceOf<T>>,
			vtoken_balance: BalanceOf<T>,
		) -> Result<
			(Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>, Option<AccountVote<BalanceOf<T>>>),
			DispatchError,
		> {
			ensure!(
				vtoken_balance <= T::MultiCurrency::total_balance(vtoken, who),
				Error::<T>::InsufficientFunds
			);
			let mut old_vote = None;
			let mut total_vote = None;
			Self::try_access_poll(vtoken, poll_index, |poll_status| {
				let tally = poll_status.ensure_ongoing().ok_or(Error::<T>::NotOngoing)?;
				VotingFor::<T>::try_mutate(who, |voting| {
					if let Voting::Casting(Casting { ref mut votes, delegations, .. }) = voting {
						match votes.binary_search_by_key(&poll_index, |i| i.0) {
							Ok(i) => {
								// Shouldn't be possible to fail, but we handle it gracefully.
								tally.remove(votes[i].1).ok_or(ArithmeticError::Underflow)?;
								old_vote = Some((votes[i].1, votes[i].3));
								if let Some(approve) = votes[i].1.as_standard() {
									tally.reduce(approve, *delegations);
								}
								votes[i].1 = vote;
								votes[i].2 = 0; // Deprecated: derivative_index
								votes[i].3 = vtoken_balance;
							},
							Err(i) => {
								votes
									.try_insert(
										i,
										// Deprecated: derivative_index
										(poll_index, vote, 0, vtoken_balance),
									)
									.map_err(|_| Error::<T>::MaxVotesReached)?;
							},
						}
						// Shouldn't be possible to fail, but we handle it gracefully.
						tally.add(vote).ok_or(ArithmeticError::Overflow)?;
						if let Some(approve) = vote.as_standard() {
							tally.increase(approve, *delegations);
						}
						total_vote = Some(tally.account_vote(Conviction::Locked1x));
					} else {
						return Err(Error::<T>::AlreadyDelegating.into());
					}
					// Extend the lock to `balance` (rather than setting it) since we don't know
					// what other votes are in place.
					Self::set_lock(&who, vtoken, voting.locked_vtoken_balance())?;
					Ok((old_vote, total_vote))
				})
			})
		}

		/// Remove the account's vote for the given poll if possible. This is possible when:
		/// - The poll has not finished.
		/// - The poll has finished and the voter lost their direction.
		/// - The poll has finished and the voter's lock period is up.
		///
		/// This will generally be combined with a call to `unlock`.
		pub(crate) fn try_remove_vote(
			who: &AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			scope: UnvoteScope,
		) -> DispatchResult {
			VotingFor::<T>::try_mutate(who, |voting| {
				if let Voting::Casting(Casting { ref mut votes, delegations, ref mut prior }) =
					voting
				{
					let i = votes
						.binary_search_by_key(&poll_index, |i| i.0)
						.map_err(|_| Error::<T>::NotVoter)?;
					let v = votes.remove(i);

					Self::try_access_poll(vtoken, poll_index, |poll_status| match poll_status {
						PollStatus::Ongoing(tally) => {
							ensure!(matches!(scope, UnvoteScope::Any), Error::<T>::NoPermission);
							// Shouldn't be possible to fail, but we handle it gracefully.
							tally.remove(v.1).ok_or(ArithmeticError::Underflow)?;
							if let Some(approve) = v.1.as_standard() {
								tally.reduce(approve, *delegations);
							}
							Ok(())
						},
						PollStatus::Completed(end, approved) => {
							if let Some((lock_periods, _)) = v.1.locked_if(approved) {
								let unlock_at = end.saturating_add(
									VoteLockingPeriod::<T>::get(vtoken)
										.ok_or(Error::<T>::NoData)?
										.saturating_mul(lock_periods.into()),
								);
								let now = T::RelaychainBlockNumberProvider::current_block_number();
								if now < unlock_at {
									ensure!(
										matches!(scope, UnvoteScope::Any),
										Error::<T>::NoPermissionYet
									);
									// v.3 is the actual locked vtoken balance
									prior.accumulate(unlock_at, v.3)
								}
							}
							Ok(())
						},
						PollStatus::Killed(_) => Ok(()), // Poll was killed.
						PollStatus::None => Ok(()),      // Poll was cancelled.
					})
				} else {
					Ok(())
				}
			})
		}

		/// Rejig the lock on an account. It will never get more stringent (since that would
		/// indicate a security hole) but may be reduced from what they are currently.
		pub(crate) fn update_lock(who: &AccountIdOf<T>, vtoken: CurrencyIdOf<T>) -> DispatchResult {
			let lock_needed = VotingFor::<T>::mutate(who, |voting| {
				voting.rejig(T::RelaychainBlockNumberProvider::current_block_number());
				voting.locked_balance()
			});

			if lock_needed.is_zero() {
				ClassLocksFor::<T>::mutate(who, |locks| {
					locks.retain(|x| x.0 != vtoken);
				});
				T::MultiCurrency::remove_lock(CONVICTION_VOTING_ID, vtoken, who)
			} else {
				ClassLocksFor::<T>::mutate(who, |locks| {
					match locks.iter().position(|x| x.0 == vtoken) {
						Some(i) => locks[i].1 = lock_needed,
						None => {
							let ok = locks.try_push((vtoken, lock_needed)).is_ok();
							debug_assert!(
								ok,
								"Vec bounded by number of classes; \
						all items in Vec associated with a unique class; \
						qed"
							);
						},
					}
				});
				T::MultiCurrency::set_lock(CONVICTION_VOTING_ID, vtoken, who, lock_needed)
			}
		}

		pub(crate) fn set_lock(
			who: &AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ClassLocksFor::<T>::mutate(who, |locks| {
				match locks.iter().position(|x| x.0 == vtoken) {
					Some(i) => locks[i].1 = amount,
					None => {
						let ok = locks.try_push((vtoken, amount)).is_ok();
						debug_assert!(
							ok,
							"Vec bounded by number of classes; \
						all items in Vec associated with a unique class; \
						qed"
						);
					},
				}
			});
			if amount.is_zero() {
				T::MultiCurrency::remove_lock(CONVICTION_VOTING_ID, vtoken, who)
			} else {
				T::MultiCurrency::set_lock(CONVICTION_VOTING_ID, vtoken, who, amount)
			}
		}

		fn ensure_vtoken(vtoken: &CurrencyIdOf<T>) -> Result<(), DispatchError> {
			ensure!([VKSM, VDOT].contains(vtoken), Error::<T>::VTokenNotSupport);
			Ok(())
		}

		fn ensure_no_pending_vote(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
		) -> DispatchResult {
			ensure!(
				!PendingVotingInfo::<T>::iter()
					.any(|(_, (v, p, _, _, _))| v == vtoken && p == poll_index),
				Error::<T>::PendingVote
			);
			Ok(())
		}

		pub fn ensure_referendum_ongoing(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
		) -> Result<ReferendumStatus<BlockNumberFor<T>, TallyOf<T>>, DispatchError> {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Ongoing(status)) => Ok(status),
				_ => Err(Error::<T>::NotOngoing.into()),
			}
		}

		fn ensure_referendum_completed(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
		) -> DispatchResult {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Completed(_)) => Ok(()),
				_ => Err(Error::<T>::NotCompleted.into()),
			}
		}

		fn ensure_referendum_expired(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
		) -> DispatchResult {
			let delegator_votes = DelegatorVotes::<T>::get(vtoken, poll_index).into_inner();
			let (_derivative_index, delegator_vote) =
				delegator_votes.first().ok_or(Error::<T>::NoData)?;
			match (ReferendumInfoFor::<T>::get(vtoken, poll_index), delegator_vote.locked_if(true))
			{
				(Some(ReferendumInfo::Completed(moment)), Some((lock_periods, _balance))) => {
					let locking_period =
						VoteLockingPeriod::<T>::get(vtoken).ok_or(Error::<T>::NoData)?;
					ensure!(
						T::RelaychainBlockNumberProvider::current_block_number() >=
							moment.saturating_add(
								locking_period.saturating_mul(lock_periods.into())
							),
						Error::<T>::NotExpired
					);
					Ok(())
				},
				(Some(ReferendumInfo::Completed(moment)), None) => {
					ensure!(
						T::RelaychainBlockNumberProvider::current_block_number() >= moment,
						Error::<T>::NotExpired
					);
					Ok(())
				},
				_ => Err(Error::<T>::NotExpired.into()),
			}
		}

		fn ensure_referendum_killed(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
		) -> DispatchResult {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Killed(_)) => Ok(()),
				_ => Err(Error::<T>::NotKilled.into()),
			}
		}

		fn ensure_xcm_response_or_governance(
			origin: OriginFor<T>,
		) -> Result<Location, DispatchError> {
			let responder = T::ResponseOrigin::ensure_origin(origin.clone()).or_else(|_| {
				T::ControlOrigin::ensure_origin(origin).map(|_| xcm::v4::Junctions::Here.into())
			})?;
			Ok(responder)
		}

		fn try_access_poll<R>(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			f: impl FnOnce(PollStatus<&mut TallyOf<T>, BlockNumberFor<T>>) -> Result<R, DispatchError>,
		) -> Result<R, DispatchError> {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Ongoing(mut status)) => {
					let result = f(PollStatus::Ongoing(&mut status.tally))?;
					ReferendumInfoFor::<T>::insert(
						vtoken,
						poll_index,
						ReferendumInfo::Ongoing(status),
					);
					Ok(result)
				},
				Some(ReferendumInfo::Completed(end)) => f(PollStatus::Completed(end, false)),
				Some(ReferendumInfo::Killed(end)) => f(PollStatus::Killed(end)),
				_ => f(PollStatus::None),
			}
		}

		fn compute_token_vote(
			vtoken: CurrencyIdOf<T>,
			vote: AccountVote<BalanceOf<T>>,
		) -> Result<AccountVote<BalanceOf<T>>, DispatchError> {
			let token = CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?;
			let vtoken_supply =
				T::VTokenSupplyProvider::get_vtoken_supply(vtoken).ok_or(Error::<T>::NoData)?;
			let token_supply =
				T::VTokenSupplyProvider::get_token_supply(token).ok_or(Error::<T>::NoData)?;
			let mut new_vote = vote;
			new_vote
				.checked_mul(token_supply)
				.and_then(|_| new_vote.checked_div(vtoken_supply))?;

			Ok(new_vote)
		}

		pub(crate) fn vote_cap(vtoken: CurrencyIdOf<T>) -> Result<BalanceOf<T>, DispatchError> {
			let token = CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?;
			let token_supply =
				T::VTokenSupplyProvider::get_token_supply(token).ok_or(Error::<T>::NoData)?;
			let vote_cap_ratio = VoteCapRatio::<T>::get(vtoken);

			Ok(vote_cap_ratio * token_supply)
		}

		pub(crate) fn vote_to_capital(conviction: Conviction, vote: BalanceOf<T>) -> BalanceOf<T> {
			let capital = match conviction {
				Conviction::None =>
					vote.checked_mul(&10u8.into()).unwrap_or_else(BalanceOf::<T>::max_value),
				x => vote.checked_div(&u8::from(x).into()).unwrap_or_else(Zero::zero),
			};
			capital
		}

		pub(crate) fn compute_delegator_total_vote(
			vtoken: CurrencyIdOf<T>,
			vote: AccountVote<BalanceOf<T>>,
		) -> Result<AccountVote<BalanceOf<T>>, DispatchError> {
			let aye = vote.as_standard().ok_or(Error::<T>::NotStandardVote)?;
			let conviction_votes = vote
				.as_standard_vote()
				.ok_or(Error::<T>::NotStandardVote)?
				.conviction
				.votes(vote.balance())
				.votes;
			let vote_cap = Self::vote_cap(vtoken)?;
			for i in 0..=6 {
				let conviction =
					Conviction::try_from(i).map_err(|_| Error::<T>::InvalidConviction)?;
				let capital = Self::vote_to_capital(conviction, conviction_votes);
				if capital <= vote_cap {
					return Ok(AccountVote::new_standard(Vote { aye, conviction }, capital));
				}
			}

			Err(Error::<T>::InsufficientFunds.into())
		}

		pub(crate) fn allocate_delegator_votes(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			delegator_total_vote: AccountVote<BalanceOf<T>>,
		) -> Result<Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>, DispatchError> {
			let vote_role: VoteRole = delegator_total_vote.into();
			let mut delegator_total_vote = delegator_total_vote;

			let token = CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?;
			let mut delegator_votes = DelegatorVotes::<T>::get(vtoken, poll_index).into_inner();
			let delegator_vote_keys =
				delegator_votes.iter().map(|(index, _)| *index).collect::<Vec<_>>();
			for derivative_index in Delegators::<T>::get(vtoken) {
				if !delegator_vote_keys.contains(&derivative_index) {
					delegator_votes
						.push((derivative_index, AccountVote::<BalanceOf<T>>::from(vote_role)));
				}
			}
			let data = delegator_votes
				.into_iter()
				.map(|(derivative_index, _)| {
					let (_, available_vote) =
						T::DerivativeAccount::get_stake_info(token, derivative_index)
							.unwrap_or_default();
					(derivative_index, available_vote)
				})
				.collect::<Vec<_>>();

			let mut delegator_votes = Vec::new();
			for (derivative_index, available_vote) in data {
				if available_vote >= delegator_total_vote.balance() {
					delegator_votes.push((derivative_index, delegator_total_vote));
					return Ok(delegator_votes);
				} else {
					let account_vote = AccountVote::new_standard(
						delegator_total_vote.as_standard_vote().ok_or(Error::<T>::NoData)?,
						available_vote,
					);
					delegator_votes.push((derivative_index, account_vote));
					delegator_total_vote
						.checked_sub(account_vote)
						.map_err(|_| ArithmeticError::Underflow)?
				}
			}
			if delegator_total_vote.balance() != Zero::zero() {
				return Err(Error::<T>::OutOfRange.into());
			}

			Ok(delegator_votes)
		}

		pub(crate) fn get_voting_agent(
			currency_id: &CurrencyIdOf<T>,
		) -> Result<VotingAgentBoxType<T>, Error<T>> {
			match *currency_id {
				VKSM | VDOT => Ok(Box::new(RelaychainAgent::<T>::new(*currency_id)?)),
				VBNC => Ok(Box::new(BifrostAgent::<T>::new(*currency_id)?)),
				_ => Err(Error::<T>::VTokenNotSupport),
			}
		}

		pub(crate) fn convert_vtoken_to_dest_location(
			vtoken: CurrencyId,
		) -> Result<Location, Error<T>> {
			let token = CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?;
			match token {
				KSM | DOT => Ok(Location::parent()),
				BNC => Ok(Location::new(1, [Parachain(T::ParachainId::get().into())])),
				_ => Err(Error::<T>::VTokenNotSupport),
			}
		}
	}
}

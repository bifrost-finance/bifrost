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

// Ensure we're ,no_std, when compiling for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

mod call;
mod vote;
pub mod weights;

pub use crate::{
	call::{ConvictionVotingCall, KusamaCall},
	vote::{ReferendumInfo, ReferendumStatus, VoteRole},
};
use codec::{Decode, Encode, HasCompact, MaxEncodedLen};
use cumulus_primitives_core::{ParaId, QueryId, Response};
use frame_support::{
	pallet_prelude::*,
	traits::{Get, LockIdentifier, PollStatus, VoteTally},
};
use frame_system::pallet_prelude::{BlockNumberFor, *};
use node_primitives::{
	currency::{VDOT, VKSM},
	CurrencyId,
};
use orml_traits::{MultiCurrency, MultiLockableCurrency};
pub use pallet::*;
use pallet_conviction_voting::{AccountVote, Casting, Tally, UnvoteScope, Voting};
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{
		AccountIdConversion, Saturating, StaticLookup, TrailingZeroInput, UniqueSaturatedInto, Zero,
	},
	ArithmeticError,
};
use sp_std::prelude::*;
use weights::WeightInfo;
use xcm::v3::{prelude::*, Weight as XcmWeight};

const CONVICTION_VOTING_ID: LockIdentifier = *b"vtvoting";

type DerivativeIndex = u16;

type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::CurrencyId;
type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

type ClassOf<T> = <T as Config>::Class;

type PollIndexOf<T> = <T as Config>::PollIndex;

pub type TallyOf<T> = Tally<BalanceOf<T>, ()>;

type VotingOf<T> = Voting<
	BalanceOf<T>,
	AccountIdOf<T>,
	BlockNumberFor<T>,
	PollIndexOf<T>,
	<T as Config>::MaxVotes,
>;

pub type ReferendumInfoOf<T> = ReferendumInfo<ClassOf<T>, BlockNumberFor<T>, TallyOf<T>>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_xcm::Config {
		type RuntimeEvent: IsType<<Self as frame_system::Config>::RuntimeEvent> + From<Event<Self>>;

		type RuntimeOrigin: IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>;

		type RuntimeCall: IsType<<Self as pallet_xcm::Config>::RuntimeCall> + From<Call<Self>>;

		type MultiCurrency: MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type ControlOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		type ResponseOrigin: EnsureOrigin<
			<Self as frame_system::Config>::RuntimeOrigin,
			Success = MultiLocation,
		>;

		type Class: Parameter + Member + Ord + Copy + MaxEncodedLen + Zero;

		type PollIndex: Parameter + Member + Ord + Copy + MaxEncodedLen + HasCompact;

		/// The maximum number of concurrent votes an account may have.
		///
		/// Also used to compute weight, an overly large value can lead to extrinsics with large
		/// weight estimation: see `delegate` for instance.
		#[pallet::constant]
		type MaxVotes: Get<u32>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Voted {
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
			vote: AccountVote<BalanceOf<T>>,
		},
		VoteNotified {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
			success: bool,
		},
		Unlocked {
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		},
		DelegatorRoleSet {
			vtoken: CurrencyIdOf<T>,
			role: VoteRole,
			derivative_index: DerivativeIndex,
		},
		DelegatorTokenUnlocked {
			vtoken: CurrencyIdOf<T>,
			delegator: AccountIdOf<T>,
		},
		ReferendumInfoSet {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
			info: ReferendumInfoOf<T>,
		},
		ReferendumStatusUpdated {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		},
		ReferendumStatusUpdateNotified {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
			success: bool,
		},
		VoteLockingPeriodSet {
			vtoken: CurrencyIdOf<T>,
			locking_period: BlockNumberFor<T>,
		},
		ReferendumKilled {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		},
		ResponseReceived {
			responder: MultiLocation,
			query_id: QueryId,
			response: Response,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// XCM execution Failure
		XcmFailure,
		/// The given currency is not supported.
		VTokenNotSupport,
		NoData,
		/// Poll is not ongoing.
		NotOngoing,
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
		/// The class must be supplied since it is not easily determinable from the state.
		ClassNeeded,
	}

	/// Information concerning any given referendum.
	#[pallet::storage]
	pub type ReferendumInfoFor<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		CurrencyIdOf<T>,
		Blake2_128Concat,
		PollIndexOf<T>,
		ReferendumInfoOf<T>,
	>;

	/// All voting for a particular voter in a particular voting class. We store the balance for the
	/// number of votes that we have recorded.
	#[pallet::storage]
	pub type VotingFor<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		AccountIdOf<T>,
		Twox64Concat,
		ClassOf<T>,
		VotingOf<T>,
		ValueQuery,
	>;

	/// The voting classes which have a non-zero lock requirement and the lock amounts which they
	/// require. The actual amount locked on behalf of this pallet should always be the maximum of
	/// this list.
	#[pallet::storage]
	pub type ClassLocksFor<T: Config> = StorageMap<
		_,
		Twox64Concat,
		AccountIdOf<T>,
		BoundedVec<(ClassOf<T>, BalanceOf<T>), ConstU32<100>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type PendingVotingInfo<T: Config> =
		StorageMap<_, Twox64Concat, QueryId, (CurrencyIdOf<T>, PollIndexOf<T>, AccountIdOf<T>)>;

	#[pallet::storage]
	pub type PendingReferendumStatus<T: Config> =
		StorageMap<_, Twox64Concat, QueryId, (CurrencyIdOf<T>, PollIndexOf<T>)>;

	#[pallet::storage]
	pub type VoteLockingPeriod<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BlockNumberFor<T>>;

	#[pallet::storage]
	pub type DelegatorRole<T: Config> =
		StorageDoubleMap<_, Twox64Concat, CurrencyIdOf<T>, Twox64Concat, VoteRole, DerivativeIndex>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub roles: Vec<(CurrencyIdOf<T>, u8, DerivativeIndex)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { roles: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.roles.iter().for_each(|(vtoken, role, derivative_index)| {
				DelegatorRole::<T>::insert(
					vtoken,
					VoteRole::try_from(*role).unwrap(),
					derivative_index,
				);
			});
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::vote())]
		pub fn vote(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] poll_index: PollIndexOf<T>,
			vote: AccountVote<BalanceOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;

			// create referendum if not exist
			if !ReferendumInfoFor::<T>::contains_key(vtoken, poll_index) {
				let class = Zero::zero();
				let info = ReferendumInfo::Ongoing(ReferendumStatus {
					track: class,
					tally: TallyOf::<T>::new(class),
				});
				ReferendumInfoFor::<T>::insert(vtoken, poll_index, info.clone());
				Self::deposit_event(Event::<T>::ReferendumInfoSet { vtoken, poll_index, info });
			}

			// record vote info
			Self::try_vote(&who, vtoken, poll_index, vote)?;

			// send XCM message
			let vote_call = KusamaCall::<T>::ConvictionVoting(ConvictionVotingCall::<T>::Vote(
				poll_index, vote,
			));
			let derivative_index =
				DelegatorRole::<T>::get(vtoken, VoteRole::from(vote)).ok_or(Error::<T>::NoData)?;
			let call = KusamaCall::<T>::get_derivative_call(derivative_index, vote_call);

			let notify_call = Call::<T>::notify_vote { query_id: 0, response: Default::default() };
			let (query_id, xcm_message) = Self::build_xcm(call, notify_call)?;
			PendingVotingInfo::<T>::insert(query_id, (vtoken, poll_index, who.clone()));
			send_xcm::<T::XcmRouter>(Parent.into(), xcm_message)
				.map_err(|_| Error::<T>::XcmFailure)?;

			Self::deposit_event(Event::<T>::Voted { who, vtoken, poll_index, vote });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::unlock())]
		pub fn unlock(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] poll_index: PollIndexOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;

			let class = Self::try_remove_vote(&who, vtoken, poll_index, None, UnvoteScope::Any)?;
			Self::update_lock(&who, vtoken, &class)?;

			Self::deposit_event(Event::<T>::Unlocked { who, vtoken, poll_index });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::update_referendum_status())]
		pub fn update_referendum_status(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] poll_index: PollIndexOf<T>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;

			let notify_call = Call::<T>::notify_update_referendum_status {
				query_id: 0,
				response: Default::default(),
			};

			let remove_vote_call = KusamaCall::<T>::ConvictionVoting(
				ConvictionVotingCall::<T>::RemoveVote(None, poll_index),
			);
			let derivative_index =
				Self::find_derivative_index_by_role(vtoken, VoteRole::SplitAbstain)
					.ok_or(Error::<T>::NoData)?;
			let call = KusamaCall::<T>::get_derivative_call(derivative_index, remove_vote_call);

			let (query_id, xcm_message) = Self::build_xcm(call, notify_call.clone())?;
			PendingReferendumStatus::<T>::insert(query_id, (vtoken, poll_index));
			send_xcm::<T::XcmRouter>(Parent.into(), xcm_message)
				.map_err(|_| Error::<T>::XcmFailure)?;

			let bifrost_para_account: AccountIdOf<T> =
				T::ParachainId::get().into_account_truncating();
			let bifrost_para_subaccount: AccountIdOf<T> =
				Self::derivative_account_id(bifrost_para_account.clone(), derivative_index);
			let unlock_call = KusamaCall::<T>::ConvictionVoting(ConvictionVotingCall::<T>::Unlock(
				Zero::zero(),
				T::Lookup::unlookup(bifrost_para_subaccount),
			));
			let call = KusamaCall::<T>::get_derivative_call(derivative_index, unlock_call);
			let (query_id, xcm_message) = Self::build_xcm(call, notify_call)?;
			PendingReferendumStatus::<T>::insert(query_id, (vtoken, poll_index));
			send_xcm::<T::XcmRouter>(Parent.into(), xcm_message)
				.map_err(|_| Error::<T>::XcmFailure)?;

			Self::deposit_event(Event::<T>::ReferendumStatusUpdated { vtoken, poll_index });

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::unlock_delegator_token())]
		pub fn unlock_delegator_token(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			delegator: AccountIdLookupOf<T>,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;

			let delegator = T::Lookup::lookup(delegator)?;

			Self::deposit_event(Event::<T>::DelegatorTokenUnlocked { vtoken, delegator });

			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::kill_referendum())]
		pub fn kill_referendum(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] poll_index: PollIndexOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_vtoken(&vtoken)?;

			let _status = Self::ensure_ongoing(vtoken, poll_index)?;
			ReferendumInfoFor::<T>::insert(vtoken, poll_index, ReferendumInfo::Killed);

			Self::deposit_event(Event::<T>::ReferendumKilled { vtoken, poll_index });

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::set_delegator_role())]
		pub fn set_delegator_role(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			derivative_index: DerivativeIndex,
			role: VoteRole,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_vtoken(&vtoken)?;

			DelegatorRole::<T>::insert(vtoken, role, derivative_index);

			Self::deposit_event(Event::<T>::DelegatorRoleSet { vtoken, role, derivative_index });

			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::set_referendum_status())]
		pub fn set_referendum_status(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] poll_index: PollIndexOf<T>,
			info: ReferendumInfoOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_vtoken(&vtoken)?;

			ensure!(ReferendumInfoFor::<T>::contains_key(vtoken, poll_index), Error::<T>::NoData);
			ReferendumInfoFor::<T>::insert(vtoken, poll_index, info.clone());

			Self::deposit_event(Event::<T>::ReferendumInfoSet { vtoken, poll_index, info });

			Ok(())
		}

		#[pallet::call_index(7)]
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

		#[pallet::call_index(100)]
		#[pallet::weight(<T as Config>::WeightInfo::notify_vote())]
		pub fn notify_vote(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			let responder = T::ResponseOrigin::ensure_origin(origin)?;
			if let Some((vtoken, poll_index, who)) = PendingVotingInfo::<T>::take(query_id) {
				let success = Response::DispatchResult(MaybeErrorCode::Success) == response;
				if !success {
					// rollback vote
					let class =
						Self::try_remove_vote(&who, vtoken, poll_index, None, UnvoteScope::Any)?;
					Self::update_lock(&who, vtoken, &class)?;
				}
				Self::deposit_event(Event::<T>::VoteNotified { vtoken, poll_index, success });
			}
			Self::deposit_event(Event::<T>::ResponseReceived { responder, query_id, response });

			Ok(())
		}

		#[pallet::call_index(101)]
		#[pallet::weight(<T as Config>::WeightInfo::notify_update_referendum_status())]
		pub fn notify_update_referendum_status(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			let responder = T::ResponseOrigin::ensure_origin(origin)?;
			if let Some((vtoken, poll_index)) = PendingReferendumStatus::<T>::take(query_id) {
				let success = Response::DispatchResult(MaybeErrorCode::Success) == response;
				if success {
					ReferendumInfoFor::<T>::insert(
						vtoken,
						poll_index,
						ReferendumInfo::Completed(<frame_system::Pallet<T>>::block_number()),
					);
				}
				Self::deposit_event(Event::<T>::ReferendumStatusUpdateNotified {
					vtoken,
					poll_index,
					success,
				});
			}
			Self::deposit_event(Event::<T>::ResponseReceived { responder, query_id, response });

			Ok(())
		}

		#[pallet::call_index(102)]
		#[pallet::weight(<T as Config>::WeightInfo::notify_unlock_delegator_token())]
		pub fn notify_unlock_delegator_token(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			let responder = T::ResponseOrigin::ensure_origin(origin)?;

			Self::deposit_event(Event::<T>::ResponseReceived { responder, query_id, response });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn try_vote(
			who: &AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
			vote: AccountVote<BalanceOf<T>>,
		) -> DispatchResult {
			ensure!(
				vote.balance() <= T::MultiCurrency::total_balance(vtoken, who),
				Error::<T>::InsufficientFunds
			);
			Self::try_access_poll(vtoken, poll_index, |poll_status| {
				let (tally, class) = poll_status.ensure_ongoing().ok_or(Error::<T>::NotOngoing)?;
				VotingFor::<T>::try_mutate(who, &class, |voting| {
					if let Voting::Casting(Casting { ref mut votes, delegations, .. }) = voting {
						match votes.binary_search_by_key(&poll_index, |i| i.0) {
							Ok(i) => {
								// Shouldn't be possible to fail, but we handle it gracefully.
								tally.remove(votes[i].1).ok_or(ArithmeticError::Underflow)?;
								if let Some(approve) = votes[i].1.as_standard() {
									tally.reduce(approve, *delegations);
								}
								votes[i].1 = vote;
							},
							Err(i) => {
								votes
									.try_insert(i, (poll_index, vote))
									.map_err(|_| Error::<T>::MaxVotesReached)?;
							},
						}
						// Shouldn't be possible to fail, but we handle it gracefully.
						tally.add(vote).ok_or(ArithmeticError::Overflow)?;
						if let Some(approve) = vote.as_standard() {
							tally.increase(approve, *delegations);
						}
					} else {
						return Err(Error::<T>::AlreadyDelegating.into());
					}
					// Extend the lock to `balance` (rather than setting it) since we don't know
					// what other votes are in place.
					Self::extend_lock(&who, vtoken, &class, vote.balance())?;
					Ok(())
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
			poll_index: PollIndexOf<T>,
			class_hint: Option<ClassOf<T>>,
			scope: UnvoteScope,
		) -> Result<ClassOf<T>, DispatchError> {
			let class = class_hint
				.or_else(|| Some(Self::as_ongoing(vtoken, poll_index)?.1))
				.ok_or(Error::<T>::ClassNeeded)?;
			VotingFor::<T>::try_mutate(who, class, |voting| {
				if let Voting::Casting(Casting { ref mut votes, delegations, ref mut prior }) =
					voting
				{
					let i = votes
						.binary_search_by_key(&poll_index, |i| i.0)
						.map_err(|_| Error::<T>::NotVoter)?;
					let v = votes.remove(i);

					Self::try_access_poll(vtoken, poll_index, |poll_status| match poll_status {
						PollStatus::Ongoing(tally, _) => {
							ensure!(matches!(scope, UnvoteScope::Any), Error::<T>::NoPermission);
							// Shouldn't be possible to fail, but we handle it gracefully.
							tally.remove(v.1).ok_or(ArithmeticError::Underflow)?;
							if let Some(approve) = v.1.as_standard() {
								tally.reduce(approve, *delegations);
							}
							Ok(class)
						},
						PollStatus::Completed(end, approved) => {
							if let Some((lock_periods, balance)) = v.1.locked_if(approved) {
								let unlock_at = end.saturating_add(
									VoteLockingPeriod::<T>::get(vtoken)
										.ok_or(Error::<T>::NoData)?
										.saturating_mul(lock_periods.into()),
								);
								let now = frame_system::Pallet::<T>::block_number();
								if now < unlock_at {
									ensure!(
										matches!(scope, UnvoteScope::Any),
										Error::<T>::NoPermissionYet
									);
									prior.accumulate(unlock_at, balance)
								}
							}
							Ok(class)
						},
						PollStatus::None => Ok(class), // Poll was cancelled.
					})
				} else {
					Ok(class)
				}
			})
		}

		/// Rejig the lock on an account. It will never get more stringent (since that would
		/// indicate a security hole) but may be reduced from what they are currently.
		pub(crate) fn update_lock(
			who: &AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			class: &ClassOf<T>,
		) -> DispatchResult {
			let class_lock_needed = VotingFor::<T>::mutate(who, class, |voting| {
				voting.rejig(frame_system::Pallet::<T>::block_number());
				voting.locked_balance()
			});
			let lock_needed = ClassLocksFor::<T>::mutate(who, |locks| {
				locks.retain(|x| &x.0 != class);
				if !class_lock_needed.is_zero() {
					let ok = locks.try_push((*class, class_lock_needed)).is_ok();
					debug_assert!(
						ok,
						"Vec bounded by number of classes; \
					all items in Vec associated with a unique class; \
					qed"
					);
				}
				locks.iter().map(|x| x.1).max().unwrap_or(Zero::zero())
			});
			if lock_needed.is_zero() {
				T::MultiCurrency::remove_lock(CONVICTION_VOTING_ID, vtoken, who)
			} else {
				T::MultiCurrency::set_lock(CONVICTION_VOTING_ID, vtoken, who, lock_needed)
			}
		}

		fn extend_lock(
			who: &AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			class: &ClassOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ClassLocksFor::<T>::mutate(who, |locks| {
				match locks.iter().position(|x| &x.0 == class) {
					Some(i) => locks[i].1 = locks[i].1.max(amount),
					None => {
						let ok = locks.try_push((*class, amount)).is_ok();
						debug_assert!(
							ok,
							"Vec bounded by number of classes; \
						all items in Vec associated with a unique class; \
						qed"
						);
					},
				}
			});
			T::MultiCurrency::extend_lock(CONVICTION_VOTING_ID, vtoken, who, amount)
		}

		fn build_xcm(
			call: KusamaCall<T>,
			notify_call: Call<T>,
		) -> Result<(QueryId, Xcm<()>), Error<T>> {
			let responder = MultiLocation::parent();
			let now = frame_system::Pallet::<T>::block_number();
			let timeout = now.saturating_add(100u32.into());
			let query_id = pallet_xcm::Pallet::<T>::new_notify_query(
				responder,
				<T as Config>::RuntimeCall::from(notify_call),
				timeout,
				Here,
			);

			let xcm_message = Self::construct_xcm_message(
				call.encode(),
				4000000000u32.into(),
				Weight::from_parts(4000000000, 100000),
				Some(query_id),
			)?;

			Ok((query_id, xcm_message))
		}

		fn construct_xcm_message(
			call: Vec<u8>,
			extra_fee: BalanceOf<T>,
			weight: XcmWeight,
			query_id: Option<QueryId>,
		) -> Result<Xcm<()>, Error<T>> {
			let para_id = T::ParachainId::get();
			let asset = MultiAsset {
				id: Concrete(MultiLocation::here()),
				fun: Fungible(UniqueSaturatedInto::<u128>::unique_saturated_into(extra_fee)),
			};

			let mut xcm_message = vec![
				WithdrawAsset(asset.clone().into()),
				BuyExecution { fees: asset, weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: weight,
					call: call.into(),
				},
				RefundSurplus,
				DepositAsset {
					assets: All.into(),
					beneficiary: MultiLocation {
						parents: 0,
						interior: X1(Parachain(para_id.into())),
					},
				},
			];

			if let Some(query_id) = query_id {
				xcm_message.insert(
					3,
					ReportTransactStatus(QueryResponseInfo {
						destination: MultiLocation::from(X1(Parachain(para_id.into()))),
						query_id,
						max_weight: weight,
					}),
				);
			}

			Ok(Xcm(xcm_message))
		}

		fn ensure_vtoken(vtoken: &CurrencyIdOf<T>) -> Result<(), DispatchError> {
			ensure!([VKSM, VDOT].contains(vtoken), Error::<T>::VTokenNotSupport);
			Ok(())
		}

		/// `Some` if the referendum `index` can be voted on, along with the tally and class of
		/// referendum.
		///
		/// Don't use this if you might mutate - use `try_access_poll` instead.
		pub fn as_ongoing(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		) -> Option<(TallyOf<T>, ClassOf<T>)> {
			Self::ensure_ongoing(vtoken, poll_index).ok().map(|x| (x.tally, x.track))
		}

		fn ensure_ongoing(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		) -> Result<ReferendumStatus<ClassOf<T>, TallyOf<T>>, DispatchError> {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Ongoing(status)) => Ok(status),
				_ => Err(Error::<T>::NotOngoing.into()),
			}
		}

		fn try_access_poll<R>(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
			f: impl FnOnce(
				PollStatus<&mut TallyOf<T>, BlockNumberFor<T>, ClassOf<T>>,
			) -> Result<R, DispatchError>,
		) -> Result<R, DispatchError> {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Ongoing(mut status)) => {
					let result = f(PollStatus::Ongoing(&mut status.tally, status.track))?;
					ReferendumInfoFor::<T>::insert(
						vtoken,
						poll_index,
						ReferendumInfo::Ongoing(status),
					);
					Ok(result)
				},
				Some(ReferendumInfo::Completed(end)) => f(PollStatus::Completed(end, false)),
				_ => f(PollStatus::None),
			}
		}

		pub fn derivative_account_id(who: T::AccountId, index: DerivativeIndex) -> T::AccountId {
			let entropy = (b"modlpy/utilisuba", who, index).using_encoded(blake2_256);
			Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
				.expect("infinite length input; no invalid inputs for type; qed")
		}

		fn find_derivative_index_by_role(
			vtoken: CurrencyIdOf<T>,
			target: VoteRole,
		) -> Option<DerivativeIndex> {
			DelegatorRole::<T>::iter_prefix(vtoken).into_iter().find_map(|(role, index)| {
				if role == target {
					Some(index)
				} else {
					None
				}
			})
		}
	}
}

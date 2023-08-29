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
pub mod traits;
mod vote;
pub mod weights;

pub use crate::{
	call::*,
	vote::{AccountVote, PollStatus, ReferendumInfo, ReferendumStatus, VoteRole},
};
use crate::{
	traits::Tally,
	vote::{Casting, Voting},
};
use codec::{Encode, HasCompact, MaxEncodedLen};
use cumulus_primitives_core::{ParaId, QueryId, Response};
use frame_support::{
	pallet_prelude::*,
	traits::{Get, LockIdentifier, VoteTally},
};
use frame_system::pallet_prelude::{BlockNumberFor, *};
use node_primitives::{
	currency::{VDOT, VKSM},
	CurrencyId,
};
use orml_traits::{MultiCurrency, MultiLockableCurrency};
pub use pallet::*;
use pallet_conviction_voting::UnvoteScope;
use sp_runtime::{
	traits::{BlockNumberProvider, CheckedSub, Saturating, UniqueSaturatedInto, Zero},
	ArithmeticError,
};
use sp_std::prelude::*;
use traits::{DerivativeAccountHandler, XcmDestWeightAndFeeHandler};
use weights::WeightInfo;
use xcm::v3::{prelude::*, Weight as XcmWeight};

const CONVICTION_VOTING_ID: LockIdentifier = *b"vtvoting";

pub type DerivativeIndex = u16;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::CurrencyId;

pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

type PollIndexOf<T> = <T as Config>::PollIndex;

pub type TallyOf<T> = Tally<BalanceOf<T>, ()>;

type VotingOf<T> = Voting<
	BalanceOf<T>,
	AccountIdOf<T>,
	BlockNumberFor<T>,
	PollIndexOf<T>,
	<T as Config>::MaxVotes,
>;

pub type ReferendumInfoOf<T> = ReferendumInfo<BlockNumberFor<T>, TallyOf<T>>;

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

		type PollIndex: Parameter + Member + Ord + Copy + MaxEncodedLen + HasCompact;

		type XcmDestWeightAndFee: XcmDestWeightAndFeeHandler<Self>;

		type DerivativeAccount: DerivativeAccountHandler<Self>;

		type RelaychainBlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

		/// The maximum number of concurrent votes an account may have.
		///
		/// Also used to compute weight, an overly large value can lead to extrinsics with large
		/// weight estimation: see `delegate` for instance.
		#[pallet::constant]
		type MaxVotes: Get<u32>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;

		#[pallet::constant]
		type QueryTimeout: Get<BlockNumberFor<Self>>;

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
		Unlocked {
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		},
		ReferendumStatusUpdated {
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		},
		DelegatorTokenUnlocked {
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			derivative_index: DerivativeIndex,
		},
		DelegatorRoleSet {
			vtoken: CurrencyIdOf<T>,
			role: VoteRole,
			derivative_index: DerivativeIndex,
		},
		ReferendumInfoSet {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
			info: ReferendumInfoOf<T>,
		},
		VoteLockingPeriodSet {
			vtoken: CurrencyIdOf<T>,
			locking_period: BlockNumberFor<T>,
		},
		ReferendumKilled {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		},
		VoteNotified {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
			success: bool,
		},
		ReferendumStatusUpdateNotified {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
			success: bool,
		},
		DelegatorTokenUnlockNotified {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
			success: bool,
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
		DerivativeIndexOccupied,
		PendingVote,
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
		BoundedVec<(PollIndexOf<T>, BalanceOf<T>), ConstU32<100>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type PendingReferendumInfo<T: Config> =
		StorageMap<_, Twox64Concat, QueryId, (CurrencyIdOf<T>, PollIndexOf<T>, BlockNumberFor<T>)>;

	#[pallet::storage]
	pub type PendingVotingInfo<T: Config> = StorageMap<
		_,
		Twox64Concat,
		QueryId,
		(CurrencyIdOf<T>, PollIndexOf<T>, DerivativeIndex, AccountIdOf<T>, BlockNumberFor<T>),
	>;

	#[pallet::storage]
	pub type PendingReferendumStatus<T: Config> =
		StorageMap<_, Twox64Concat, QueryId, (CurrencyIdOf<T>, PollIndexOf<T>, BlockNumberFor<T>)>;

	#[pallet::storage]
	pub type PendingUnlockDelegatorToken<T: Config> =
		StorageMap<_, Twox64Concat, QueryId, (CurrencyIdOf<T>, PollIndexOf<T>, BlockNumberFor<T>)>;

	#[pallet::storage]
	pub type VoteLockingPeriod<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BlockNumberFor<T>>;

	#[pallet::storage]
	pub type DelegatorRole<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Twox64Concat, CurrencyIdOf<T>>,
			NMapKey<Twox64Concat, VoteRole>,
			NMapKey<Twox64Concat, DerivativeIndex>,
		),
		AccountVote<BalanceOf<T>>,
	>;

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
				let vote_role = VoteRole::try_from(*role).unwrap();
				DelegatorRole::<T>::insert(
					(vtoken, vote_role.clone(), derivative_index),
					AccountVote::<BalanceOf<T>>::from(vote_role),
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
			let vote_role = VoteRole::from(vote);
			let derivative_index =
				Self::select_derivative_index(vtoken, vote_role, vote.balance())?;
			Self::ensure_no_pending_vote(&vtoken, &poll_index, &derivative_index)?;

			// create referendum if not exist
			let mut confirmed = false;
			if !ReferendumInfoFor::<T>::contains_key(vtoken, poll_index) {
				let info = ReferendumInfo::Ongoing(ReferendumStatus {
					submitted: T::RelaychainBlockNumberProvider::current_block_number(),
					tally: TallyOf::<T>::new(0u16),
					confirmed,
				});
				ReferendumInfoFor::<T>::insert(vtoken, poll_index, info.clone());
				Self::deposit_event(Event::<T>::ReferendumInfoSet { vtoken, poll_index, info });
			} else {
				confirmed = true;
			}

			// record vote info
			Self::try_vote(&who, vtoken, poll_index, vote)?;

			let new_vote = DelegatorRole::<T>::try_mutate_exists(
				(vtoken, VoteRole::from(vote), derivative_index),
				|maybe_vote| {
					if let Some(inner_vote) = maybe_vote {
						inner_vote.checked_add(vote).map_err(|_| Error::<T>::NoData)?;
						Ok(inner_vote.clone())
					} else {
						Err(Error::<T>::NoData)
					}
				},
			)?;

			// send XCM message
			let vote_call = RelayCall::<T>::ConvictionVoting(ConvictionVotingCall::<T>::Vote(
				poll_index, new_vote,
			));
			let notify_call = Call::<T>::notify_vote { query_id: 0, response: Default::default() };
			let (weight, extra_fee) = T::XcmDestWeightAndFee::get_vote(
				CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?,
			)
			.ok_or(Error::<T>::NoData)?;
			Self::send_xcm_with_notify(
				derivative_index,
				vote_call,
				notify_call,
				weight,
				extra_fee,
				|query_id| {
					let expired_block_number = frame_system::Pallet::<T>::block_number()
						.saturating_add(T::QueryTimeout::get());
					if !confirmed {
						PendingReferendumInfo::<T>::insert(
							query_id,
							(vtoken, poll_index, expired_block_number),
						);
					}
					PendingVotingInfo::<T>::insert(
						query_id,
						(vtoken, poll_index, derivative_index, who.clone(), expired_block_number),
					)
				},
			)?;

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
			Self::ensure_referendum_expired(vtoken, poll_index)
				.or(Self::ensure_referendum_killed(vtoken, poll_index))
				.map_err(|_| Error::<T>::NotExpired)?;

			Self::try_remove_vote(&who, vtoken, poll_index, UnvoteScope::Any)?;
			Self::update_lock(&who, vtoken, &poll_index)?;

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
			let who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			Self::ensure_no_pending_update_referendum_status(&vtoken, &poll_index)?;
			Self::ensure_referendum_ongoing(vtoken, poll_index)?;

			let notify_call = Call::<T>::notify_update_referendum_status {
				query_id: 0,
				response: Default::default(),
			};
			let derivative_index =
				Self::find_derivative_index_by_role(vtoken, VoteRole::SplitAbstain)
					.ok_or(Error::<T>::NoData)?;
			let remove_vote_call = RelayCall::<T>::ConvictionVoting(
				ConvictionVotingCall::<T>::RemoveVote(None, poll_index),
			);
			let (weight, extra_fee) = T::XcmDestWeightAndFee::get_remove_vote(
				CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?,
			)
			.ok_or(Error::<T>::NoData)?;
			Self::send_xcm_with_notify(
				derivative_index,
				remove_vote_call,
				notify_call,
				weight,
				extra_fee,
				|query_id| {
					PendingReferendumStatus::<T>::insert(
						query_id,
						(
							vtoken,
							poll_index,
							frame_system::Pallet::<T>::block_number()
								.saturating_add(T::QueryTimeout::get()),
						),
					);
				},
			)?;

			Self::deposit_event(Event::<T>::ReferendumStatusUpdated { who, vtoken, poll_index });

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::unlock_delegator_token())]
		pub fn remove_delegator_vote(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] poll_index: PollIndexOf<T>,
			derivative_index: DerivativeIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			Self::ensure_referendum_expired(vtoken, poll_index)?;

			let notify_call = Call::<T>::notify_remove_delegator_vote {
				query_id: 0,
				response: Default::default(),
			};
			let remove_vote_call = RelayCall::<T>::ConvictionVoting(
				ConvictionVotingCall::<T>::RemoveVote(None, poll_index),
			);
			let (weight, extra_fee) = T::XcmDestWeightAndFee::get_remove_vote(
				CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?,
			)
			.ok_or(Error::<T>::NoData)?;
			Self::send_xcm_with_notify(
				derivative_index,
				remove_vote_call,
				notify_call,
				weight,
				extra_fee,
				|query_id| {
					PendingUnlockDelegatorToken::<T>::insert(
						query_id,
						(
							vtoken,
							poll_index,
							frame_system::Pallet::<T>::block_number()
								.saturating_add(T::QueryTimeout::get()),
						),
					);
				},
			)?;

			Self::deposit_event(Event::<T>::DelegatorTokenUnlocked {
				who,
				vtoken,
				derivative_index,
			});

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
			Self::ensure_referendum_completed(vtoken, poll_index)?;

			ReferendumInfoFor::<T>::insert(
				vtoken,
				poll_index,
				ReferendumInfo::Killed(T::RelaychainBlockNumberProvider::current_block_number()),
			);

			Self::deposit_event(Event::<T>::ReferendumKilled { vtoken, poll_index });

			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::set_delegator_role())]
		pub fn set_delegator_role(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			derivative_index: DerivativeIndex,
			vote_role: VoteRole,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			let token = CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?;
			ensure!(
				T::DerivativeAccount::check_derivative_index_exists(token, derivative_index),
				Error::<T>::NoData
			);
			ensure!(
				!DelegatorRole::<T>::contains_key((vtoken, vote_role, derivative_index)),
				Error::<T>::DerivativeIndexOccupied
			);

			if let Some(((role, index), vote)) = DelegatorRole::<T>::iter_prefix((vtoken,))
				.find(|((_, i), _)| i == &derivative_index)
			{
				DelegatorRole::<T>::remove((vtoken, role, index));
				DelegatorRole::<T>::insert((vtoken, vote_role, derivative_index), vote);
			} else {
				DelegatorRole::<T>::insert(
					(vtoken, vote_role, derivative_index),
					AccountVote::<BalanceOf<T>>::from(vote_role),
				);
			}

			Self::deposit_event(Event::<T>::DelegatorRoleSet {
				vtoken,
				role: vote_role,
				derivative_index,
			});

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
			let responder = Self::ensure_xcm_response_or_governance(origin)?;
			let success = Response::DispatchResult(MaybeErrorCode::Success) == response;

			if let Some((vtoken, poll_index, _, who, _)) = PendingVotingInfo::<T>::take(query_id) {
				if !success {
					// rollback vote
					Self::try_remove_vote(&who, vtoken, poll_index, UnvoteScope::Any)?;
					Self::update_lock(&who, vtoken, &poll_index)?;
				}
				Self::deposit_event(Event::<T>::VoteNotified { vtoken, poll_index, success });
			}

			if let Some((vtoken, poll_index, _)) = PendingReferendumInfo::<T>::take(query_id) {
				if !success {
					ReferendumInfoFor::<T>::remove(vtoken, poll_index);
				} else {
					ReferendumInfoFor::<T>::try_mutate_exists(
						vtoken,
						poll_index,
						|info| -> DispatchResult {
							if let Some(ReferendumInfo::Ongoing(status)) = info {
								status.confirmed = true;
							}
							Ok(())
						},
					)?;
				}
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
			let responder = Self::ensure_xcm_response_or_governance(origin)?;
			if let Some((vtoken, poll_index, _)) = PendingReferendumStatus::<T>::take(query_id) {
				let success = Response::DispatchResult(MaybeErrorCode::Success) == response;
				if success {
					ReferendumInfoFor::<T>::try_mutate_exists(
						vtoken,
						poll_index,
						|maybe_info| -> DispatchResult {
							if let Some(info) = maybe_info {
								*info = ReferendumInfo::Completed(
									T::RelaychainBlockNumberProvider::current_block_number(),
								);
							}
							Ok(())
						},
					)?;
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
		#[pallet::weight(<T as Config>::WeightInfo::notify_remove_delegator_vote())]
		pub fn notify_remove_delegator_vote(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			let responder = Self::ensure_xcm_response_or_governance(origin)?;
			if let Some((vtoken, poll_index, _)) = PendingUnlockDelegatorToken::<T>::take(query_id)
			{
				let success = Response::DispatchResult(MaybeErrorCode::Success) == response;
				if !success {
					// rollback vote
					let class =
						Self::try_remove_vote(&who, vtoken, poll_index, None, UnvoteScope::Any)?;
					Self::update_lock(&who, vtoken, &class)?;
				}
				Self::deposit_event(Event::<T>::DelegatorTokenUnlockNotified {
					vtoken,
					poll_index,
					success,
				});
			}
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
				let tally = poll_status.ensure_ongoing().ok_or(Error::<T>::NotOngoing)?;
				VotingFor::<T>::try_mutate(who, |voting| {
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
					Self::extend_lock(&who, vtoken, &poll_index, vote.balance())?;
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
							if let Some((lock_periods, balance)) = v.1.locked_if(approved) {
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
									prior.accumulate(unlock_at, balance)
								}
							}
							Ok(())
						},
						PollStatus::None => Ok(()), // Poll was cancelled.
					})
				} else {
					Ok(())
				}
			})
		}

		/// Rejig the lock on an account. It will never get more stringent (since that would
		/// indicate a security hole) but may be reduced from what they are currently.
		pub(crate) fn update_lock(
			who: &AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: &PollIndexOf<T>,
		) -> DispatchResult {
			let class_lock_needed = VotingFor::<T>::mutate(who, |voting| {
				voting.rejig(frame_system::Pallet::<T>::block_number());
				voting.locked_balance()
			});
			let lock_needed = ClassLocksFor::<T>::mutate(who, |locks| {
				locks.retain(|x| &x.0 != poll_index);
				if !class_lock_needed.is_zero() {
					let ok = locks.try_push((*poll_index, class_lock_needed)).is_ok();
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
			poll_index: &PollIndexOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ClassLocksFor::<T>::mutate(who, |locks| {
				match locks.iter().position(|x| &x.0 == poll_index) {
					Some(i) => locks[i].1 = locks[i].1.max(amount),
					None => {
						let ok = locks.try_push((*poll_index, amount)).is_ok();
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

		fn send_xcm_with_notify(
			derivative_index: DerivativeIndex,
			call: RelayCall<T>,
			notify_call: Call<T>,
			weight: XcmWeight,
			extra_fee: BalanceOf<T>,
			f: impl FnOnce(QueryId) -> (),
		) -> DispatchResult {
			let responder = MultiLocation::parent();
			let now = frame_system::Pallet::<T>::block_number();
			let timeout = now.saturating_add(T::QueryTimeout::get());
			let query_id = pallet_xcm::Pallet::<T>::new_notify_query(
				responder,
				<T as Config>::RuntimeCall::from(notify_call),
				timeout,
				Here,
			);
			f(query_id);

			let xcm_message = Self::construct_xcm_message(
				RelayCall::<T>::get_derivative_call(derivative_index, call).encode(),
				extra_fee,
				weight,
				query_id,
			)?;

			send_xcm::<T::XcmRouter>(Parent.into(), xcm_message)
				.map_err(|_| Error::<T>::XcmFailure)?;

			Ok(())
		}

		fn construct_xcm_message(
			call: Vec<u8>,
			extra_fee: BalanceOf<T>,
			weight: XcmWeight,
			query_id: QueryId,
		) -> Result<Xcm<()>, Error<T>> {
			let para_id = T::ParachainId::get().into();
			let asset = MultiAsset {
				id: Concrete(MultiLocation::here()),
				fun: Fungible(UniqueSaturatedInto::<u128>::unique_saturated_into(extra_fee)),
			};
			let xcm_message = vec![
				WithdrawAsset(asset.clone().into()),
				BuyExecution { fees: asset, weight_limit: Unlimited },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: weight,
					call: call.into(),
				},
				ReportTransactStatus(QueryResponseInfo {
					destination: MultiLocation::from(X1(Parachain(para_id))),
					query_id,
					max_weight: weight,
				}),
				RefundSurplus,
				DepositAsset {
					assets: All.into(),
					beneficiary: MultiLocation { parents: 0, interior: X1(Parachain(para_id)) },
				},
			];

			Ok(Xcm(xcm_message))
		}

		fn ensure_vtoken(vtoken: &CurrencyIdOf<T>) -> Result<(), DispatchError> {
			ensure!([VKSM, VDOT].contains(vtoken), Error::<T>::VTokenNotSupport);
			Ok(())
		}

		fn ensure_no_pending_vote(
			vtoken: &CurrencyIdOf<T>,
			poll_index: &PollIndexOf<T>,
			derivative_index: &DerivativeIndex,
		) -> DispatchResult {
			ensure!(
				PendingVotingInfo::<T>::iter()
					.find(|(_, (v, p, _, _, _))| v == vtoken && p == poll_index)
					.is_none(),
				Error::<T>::PendingVote
			);
			Ok(())
		}

		fn ensure_no_pending_update_referendum_status(
			vtoken: &CurrencyIdOf<T>,
			poll_index: &PollIndexOf<T>,
		) -> DispatchResult {
			ensure!(
				PendingReferendumStatus::<T>::iter()
					.find(|(_, (v, p, _))| v == vtoken && p == poll_index)
					.is_none(),
				Error::<T>::PendingUpdateReferendumStatus
			);
			Ok(())
		}

		pub fn ensure_referendum_ongoing(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		) -> Result<ReferendumStatus<BlockNumberFor<T>, TallyOf<T>>, DispatchError> {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Ongoing(status)) => Ok(status),
				_ => Err(Error::<T>::NotOngoing.into()),
			}
		}

		fn ensure_referendum_completed(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		) -> Result<BlockNumberFor<T>, DispatchError> {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Completed(moment)) => Ok(moment),
				_ => Err(Error::<T>::NotCompleted.into()),
			}
		}

		fn ensure_referendum_expired(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		) -> Result<BlockNumberFor<T>, DispatchError> {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Completed(moment)) => {
					let locking_period =
						VoteLockingPeriod::<T>::get(vtoken).ok_or(Error::<T>::NoData)?;
					Ok(moment.saturating_add(locking_period))
				},
				_ => Err(Error::<T>::NotCompleted.into()),
			}
		}

		fn ensure_referendum_killed(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
		) -> Result<BlockNumberFor<T>, DispatchError> {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Killed(moment)) => Ok(moment),
				_ => Err(Error::<T>::NotKilled.into()),
			}
		}

		fn ensure_xcm_response_or_governance(
			origin: OriginFor<T>,
		) -> Result<MultiLocation, DispatchError> {
			let responder = T::ResponseOrigin::ensure_origin(origin.clone())
				.or_else(|_| T::ControlOrigin::ensure_origin(origin).map(|_| Here.into()))?;
			Ok(responder)
		}

		fn try_access_poll<R>(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndexOf<T>,
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
				_ => f(PollStatus::None),
			}
		}

		fn select_derivative_index(
			vtoken: CurrencyIdOf<T>,
			role: VoteRole,
			vote_amount: BalanceOf<T>,
		) -> Result<DerivativeIndex, DispatchError> {
			let token = CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?;

			let mut data = DelegatorRole::<T>::iter_prefix((vtoken, role))
				.map(|(index, vote)| {
					let (_, active) = T::DerivativeAccount::get_stake_info(token, index)
						.unwrap_or(Default::default());
					(active, vote, index)
				})
				.collect::<Vec<_>>();
			data.sort_by(|a, b| {
				(b.0.saturating_sub(b.1.balance())).cmp(&(a.0.saturating_sub(a.1.balance())))
			});

			let (active, vote, index) = data.first().ok_or(Error::<T>::NoData)?;
			active
				.checked_sub(&vote.balance())
				.ok_or(ArithmeticError::Underflow)?
				.checked_sub(&vote_amount)
				.ok_or(ArithmeticError::Underflow)?;

			Ok(*index)
		}

		fn find_derivative_index_by_role(
			vtoken: CurrencyIdOf<T>,
			target_role: VoteRole,
		) -> Option<DerivativeIndex> {
			DelegatorRole::<T>::iter_prefix((vtoken,)).into_iter().find_map(
				|((role, index), vote)| if role == target_role { Some(index) } else { None },
			)
		}
	}
}
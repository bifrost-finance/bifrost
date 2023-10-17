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

use crate::vote::{Casting, Tally, Voting};
pub use crate::{
	call::*,
	vote::{AccountVote, PollStatus, ReferendumInfo, ReferendumStatus, VoteRole},
};
use cumulus_primitives_core::{ParaId, QueryId, Response};
use frame_support::{
	dispatch::GetDispatchInfo,
	pallet_prelude::*,
	traits::{Get, LockIdentifier},
};
use frame_system::pallet_prelude::{BlockNumberFor, *};
use node_primitives::{
	currency::{VDOT, VKSM},
	traits::{DerivativeAccountHandler, VTokenSupplyProvider, XcmDestWeightAndFeeHandler},
	CurrencyId, DerivativeIndex, XcmOperationType,
};
use orml_traits::{MultiCurrency, MultiLockableCurrency};
pub use pallet::*;
use pallet_conviction_voting::UnvoteScope;
use sp_runtime::{
	traits::{BlockNumberProvider, CheckedSub, Saturating, UniqueSaturatedInto, Zero},
	ArithmeticError,
};
use sp_std::prelude::*;
pub use weights::WeightInfo;
use xcm::v3::{prelude::*, Weight as XcmWeight};

const CONVICTION_VOTING_ID: LockIdentifier = *b"vtvoting";

type PollIndex = u32;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::CurrencyId;

pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type TallyOf<T> = Tally<BalanceOf<T>, ()>;

type VotingOf<T> =
	Voting<BalanceOf<T>, AccountIdOf<T>, BlockNumberFor<T>, PollIndex, <T as Config>::MaxVotes>;

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

		type RuntimeCall: IsType<<Self as pallet_xcm::Config>::RuntimeCall>
			+ From<Call<Self>>
			+ GetDispatchInfo;

		type MultiCurrency: MultiLockableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type ControlOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		type ResponseOrigin: EnsureOrigin<
			<Self as frame_system::Config>::RuntimeOrigin,
			Success = MultiLocation,
		>;

		type XcmDestWeightAndFee: XcmDestWeightAndFeeHandler<CurrencyIdOf<Self>, BalanceOf<Self>>;

		type DerivativeAccount: DerivativeAccountHandler<CurrencyIdOf<Self>, BalanceOf<Self>>;

		type RelaychainBlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

		type VTokenSupplyProvider: VTokenSupplyProvider<CurrencyIdOf<Self>, BalanceOf<Self>>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;

		/// The maximum number of concurrent votes an account may have.
		#[pallet::constant]
		type MaxVotes: Get<u32>;

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
			poll_index: PollIndex,
			new_vote: AccountVote<BalanceOf<T>>,
			delegator_vote: AccountVote<BalanceOf<T>>,
		},
		Unlocked {
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
		},
		DelegatorVoteRemoved {
			who: AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			derivative_index: DerivativeIndex,
		},
		DelegatorRoleSet {
			vtoken: CurrencyIdOf<T>,
			role: VoteRole,
			derivative_index: DerivativeIndex,
		},
		ReferendumInfoCreated {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			info: ReferendumInfoOf<T>,
		},
		ReferendumInfoSet {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			info: ReferendumInfoOf<T>,
		},
		VoteLockingPeriodSet {
			vtoken: CurrencyIdOf<T>,
			locking_period: BlockNumberFor<T>,
		},
		UndecidingTimeoutSet {
			vtoken: CurrencyIdOf<T>,
			undeciding_timeout: BlockNumberFor<T>,
		},
		ReferendumKilled {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
		},
		VoteNotified {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			success: bool,
		},
		DelegatorVoteRemovedNotified {
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
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
		/// Change delegator is not allowed.
		ChangeDelegator,
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
		BoundedVec<(PollIndex, BalanceOf<T>), T::MaxVotes>,
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
	pub type DelegatorVote<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		CurrencyIdOf<T>,
		Twox64Concat,
		DerivativeIndex,
		AccountVote<BalanceOf<T>>,
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
		pub delegator_votes: Vec<(CurrencyIdOf<T>, u8, DerivativeIndex)>,
		pub undeciding_timeouts: Vec<(CurrencyIdOf<T>, BlockNumberFor<T>)>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			self.delegator_votes.iter().for_each(|(vtoken, role, derivative_index)| {
				let vote_role = VoteRole::try_from(*role).unwrap();
				DelegatorVote::<T>::insert(
					vtoken,
					derivative_index,
					AccountVote::<BalanceOf<T>>::from(vote_role),
				);
			});
			self.undeciding_timeouts.iter().for_each(|(vtoken, undeciding_timeout)| {
				UndecidingTimeout::<T>::insert(vtoken, undeciding_timeout);
			});
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let mut weight = T::DbWeight::get().reads(1);
			let relay_current_block_number =
				T::RelaychainBlockNumberProvider::current_block_number();

			weight += T::DbWeight::get().reads(1);
			let timeout = ReferendumTimeout::<T>::get(relay_current_block_number);
			if !timeout.is_empty() {
				timeout.iter().for_each(|(vtoken, poll_index)| {
					ReferendumInfoFor::<T>::mutate(
						vtoken,
						poll_index,
						|maybe_info| match maybe_info {
							Some(info) =>
								if let ReferendumInfo::Ongoing(_) = info {
									*info = ReferendumInfo::Completed(
										relay_current_block_number.into(),
									);
								},
							None => {},
						},
					);
					weight += T::DbWeight::get().reads_writes(1, 1);
				});
				weight += T::DbWeight::get().reads_writes(1, 1);
				ReferendumTimeout::<T>::remove(relay_current_block_number);
			}

			weight
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
			#[pallet::compact] poll_index: PollIndex,
			vote: AccountVote<BalanceOf<T>>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			ensure!(UndecidingTimeout::<T>::contains_key(vtoken), Error::<T>::NoData);
			Self::ensure_no_pending_vote(vtoken, poll_index)?;

			let new_vote = Self::compute_new_vote(vtoken, vote)?;
			let derivative_index = Self::try_select_derivative_index(vtoken, new_vote)?;
			if let Some(d) = VoteDelegatorFor::<T>::get((&who, vtoken, poll_index)) {
				ensure!(d == derivative_index, Error::<T>::ChangeDelegator)
			}

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
			let maybe_old_vote = Self::try_vote(
				&who,
				vtoken,
				poll_index,
				derivative_index,
				new_vote,
				vote.balance(),
			)?;

			// send XCM message
			let delegator_vote =
				DelegatorVote::<T>::get(vtoken, derivative_index).ok_or(Error::<T>::NoData)?;
			let vote_call =
				<RelayCall<T> as ConvictionVotingCall<T>>::vote(poll_index, delegator_vote);
			let notify_call = Call::<T>::notify_vote { query_id: 0, response: Default::default() };
			let (weight, extra_fee) = T::XcmDestWeightAndFee::get_operation_weight_and_fee(
				CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?,
				XcmOperationType::Vote,
			)
			.ok_or(Error::<T>::NoData)?;
			Self::send_xcm_with_notify(
				derivative_index,
				vote_call,
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

			Self::deposit_event(Event::<T>::Voted {
				who,
				vtoken,
				poll_index,
				new_vote,
				delegator_vote,
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
			Self::ensure_referendum_expired(vtoken, poll_index)
				.or(Self::ensure_referendum_killed(vtoken, poll_index))
				.map_err(|_| Error::<T>::NotExpired)?;

			Self::try_remove_vote(&who, vtoken, poll_index, UnvoteScope::Any)?;
			Self::update_lock(&who, vtoken, &poll_index)?;

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
			#[pallet::compact] poll_index: PollIndex,
			#[pallet::compact] derivative_index: DerivativeIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::ensure_vtoken(&vtoken)?;
			Self::ensure_referendum_expired(vtoken, poll_index)?;
			ensure!(DelegatorVote::<T>::contains_key(vtoken, derivative_index), Error::<T>::NoData);

			let notify_call = Call::<T>::notify_remove_delegator_vote {
				query_id: 0,
				response: Default::default(),
			};
			let remove_vote_call =
				<RelayCall<T> as ConvictionVotingCall<T>>::remove_vote(None, poll_index);
			let (weight, extra_fee) = T::XcmDestWeightAndFee::get_operation_weight_and_fee(
				CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?,
				XcmOperationType::RemoveVote,
			)
			.ok_or(Error::<T>::NoData)?;
			Self::send_xcm_with_notify(
				derivative_index,
				remove_vote_call,
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
		#[pallet::weight(<T as Config>::WeightInfo::set_delegator_role())]
		pub fn set_delegator_role(
			origin: OriginFor<T>,
			vtoken: CurrencyIdOf<T>,
			#[pallet::compact] derivative_index: DerivativeIndex,
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
				!DelegatorVote::<T>::contains_key(vtoken, derivative_index),
				Error::<T>::DerivativeIndexOccupied
			);

			DelegatorVote::<T>::insert(
				vtoken,
				derivative_index,
				AccountVote::<BalanceOf<T>>::from(vote_role),
			);

			Self::deposit_event(Event::<T>::DelegatorRoleSet {
				vtoken,
				role: vote_role,
				derivative_index,
			});

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
				if !success {
					// rollback vote
					Self::try_remove_vote(&who, vtoken, poll_index, UnvoteScope::Any)?;
					Self::update_lock(&who, vtoken, &poll_index)?;
					if let Some((old_vote, vtoken_balance)) = maybe_old_vote {
						Self::try_vote(
							&who,
							vtoken,
							poll_index,
							derivative_index,
							old_vote,
							vtoken_balance,
						)?;
					}
				} else {
					if !VoteDelegatorFor::<T>::contains_key((&who, vtoken, poll_index)) {
						VoteDelegatorFor::<T>::insert((&who, vtoken, poll_index), derivative_index);
					}
				}
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
			if let Some((vtoken, poll_index, derivative_index)) =
				PendingRemoveDelegatorVote::<T>::get(query_id)
			{
				let success = Response::DispatchResult(MaybeErrorCode::Success) == response;
				if success {
					DelegatorVote::<T>::try_mutate_exists(
						vtoken,
						derivative_index,
						|maybe_vote| {
							if let Some(inner_vote) = maybe_vote {
								inner_vote
									.checked_sub(*inner_vote)
									.map_err(|_| Error::<T>::NoData)?;
								Ok(())
							} else {
								Err(Error::<T>::NoData)
							}
						},
					)?;
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
	}

	impl<T: Config> Pallet<T> {
		fn try_vote(
			who: &AccountIdOf<T>,
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
			derivative_index: DerivativeIndex,
			vote: AccountVote<BalanceOf<T>>,
			vtoken_balance: BalanceOf<T>,
		) -> Result<Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>, DispatchError> {
			ensure!(
				vote.balance() <= T::MultiCurrency::total_balance(vtoken, who),
				Error::<T>::InsufficientFunds
			);
			let mut old_vote = None;
			Self::try_access_poll(vtoken, poll_index, |poll_status| {
				let tally = poll_status.ensure_ongoing().ok_or(Error::<T>::NotOngoing)?;
				VotingFor::<T>::try_mutate(who, |voting| {
					if let Voting::Casting(Casting { ref mut votes, delegations, .. }) = voting {
						match votes.binary_search_by_key(&poll_index, |i| i.0) {
							Ok(i) => {
								// Shouldn't be possible to fail, but we handle it gracefully.
								tally.remove(votes[i].1).ok_or(ArithmeticError::Underflow)?;
								Self::try_sub_delegator_vote(vtoken, votes[i].2, votes[i].1)?;
								old_vote = Some((votes[i].1, votes[i].3));
								if let Some(approve) = votes[i].1.as_standard() {
									tally.reduce(approve, *delegations);
								}
								votes[i].1 = vote;
								votes[i].2 = derivative_index;
								votes[i].3 = vtoken_balance;
							},
							Err(i) => {
								votes
									.try_insert(
										i,
										(poll_index, vote, derivative_index, vtoken_balance),
									)
									.map_err(|_| Error::<T>::MaxVotesReached)?;
							},
						}
						// Shouldn't be possible to fail, but we handle it gracefully.
						tally.add(vote).ok_or(ArithmeticError::Overflow)?;
						Self::try_add_delegator_vote(vtoken, derivative_index, vote)?;
						if let Some(approve) = vote.as_standard() {
							tally.increase(approve, *delegations);
						}
					} else {
						return Err(Error::<T>::AlreadyDelegating.into());
					}
					// Extend the lock to `balance` (rather than setting it) since we don't know
					// what other votes are in place.
					Self::extend_lock(&who, vtoken, &poll_index, vtoken_balance)?;
					Ok(old_vote)
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
							Self::try_sub_delegator_vote(vtoken, v.2, v.1)?;
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
			poll_index: &PollIndex,
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
			poll_index: &PollIndex,
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
			transact_weight: XcmWeight,
			extra_fee: BalanceOf<T>,
			f: impl FnOnce(QueryId) -> (),
		) -> DispatchResult {
			let responder = MultiLocation::parent();
			let now = frame_system::Pallet::<T>::block_number();
			let timeout = now.saturating_add(T::QueryTimeout::get());
			let notify_runtime_call = <T as Config>::RuntimeCall::from(notify_call);
			let notify_call_weight = notify_runtime_call.get_dispatch_info().weight;
			let query_id = pallet_xcm::Pallet::<T>::new_notify_query(
				responder,
				notify_runtime_call,
				timeout,
				Here,
			);
			f(query_id);

			let xcm_message = Self::construct_xcm_message(
				<RelayCall<T> as UtilityCall<RelayCall<T>>>::as_derivative(derivative_index, call)
					.encode(),
				extra_fee,
				transact_weight,
				notify_call_weight,
				query_id,
			)?;

			send_xcm::<T::XcmRouter>(Parent.into(), xcm_message)
				.map_err(|_| Error::<T>::XcmFailure)?;

			Ok(())
		}

		fn construct_xcm_message(
			call: Vec<u8>,
			extra_fee: BalanceOf<T>,
			transact_weight: XcmWeight,
			notify_call_weight: XcmWeight,
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
					require_weight_at_most: transact_weight,
					call: call.into(),
				},
				ReportTransactStatus(QueryResponseInfo {
					destination: MultiLocation::from(X1(Parachain(para_id))),
					query_id,
					max_weight: notify_call_weight,
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
		) -> Result<BlockNumberFor<T>, DispatchError> {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Completed(moment)) => Ok(moment),
				_ => Err(Error::<T>::NotCompleted.into()),
			}
		}

		fn ensure_referendum_expired(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
		) -> Result<BlockNumberFor<T>, DispatchError> {
			match ReferendumInfoFor::<T>::get(vtoken, poll_index) {
				Some(ReferendumInfo::Completed(moment)) => {
					let locking_period =
						VoteLockingPeriod::<T>::get(vtoken).ok_or(Error::<T>::NoData)?;
					ensure!(
						T::RelaychainBlockNumberProvider::current_block_number() >=
							moment.saturating_add(locking_period),
						Error::<T>::NotExpired
					);
					Ok(moment.saturating_add(locking_period))
				},
				_ => Err(Error::<T>::NotExpired.into()),
			}
		}

		fn ensure_referendum_killed(
			vtoken: CurrencyIdOf<T>,
			poll_index: PollIndex,
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
				_ => f(PollStatus::None),
			}
		}

		fn try_select_derivative_index(
			vtoken: CurrencyIdOf<T>,
			my_vote: AccountVote<BalanceOf<T>>,
		) -> Result<DerivativeIndex, DispatchError> {
			let token = CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?;

			let mut data = DelegatorVote::<T>::iter_prefix(vtoken)
				.map(|(index, vote)| {
					let (_, active) = T::DerivativeAccount::get_stake_info(token, index)
						.unwrap_or(Default::default());
					(active, vote, index)
				})
				.filter(|(_, vote, _)| VoteRole::from(*vote) == VoteRole::from(my_vote))
				.collect::<Vec<_>>();
			data.sort_by(|a, b| {
				(b.0.saturating_sub(b.1.balance())).cmp(&(a.0.saturating_sub(a.1.balance())))
			});

			let (active, vote, index) = data.first().ok_or(Error::<T>::NoData)?;
			active
				.checked_sub(&vote.balance())
				.ok_or(ArithmeticError::Underflow)?
				.checked_sub(&my_vote.balance())
				.ok_or(ArithmeticError::Underflow)?;

			Ok(*index)
		}

		fn try_add_delegator_vote(
			vtoken: CurrencyIdOf<T>,
			derivative_index: DerivativeIndex,
			vote: AccountVote<BalanceOf<T>>,
		) -> Result<AccountVote<BalanceOf<T>>, DispatchError> {
			DelegatorVote::<T>::try_mutate_exists(vtoken, derivative_index, |maybe_vote| {
				match maybe_vote {
					Some(inner_vote) => {
						inner_vote.checked_add(vote).map_err(|_| ArithmeticError::Overflow)?;
						Ok(*inner_vote)
					},
					None => Err(Error::<T>::NoData.into()),
				}
			})
		}

		fn try_sub_delegator_vote(
			vtoken: CurrencyIdOf<T>,
			derivative_index: DerivativeIndex,
			vote: AccountVote<BalanceOf<T>>,
		) -> Result<AccountVote<BalanceOf<T>>, DispatchError> {
			DelegatorVote::<T>::try_mutate_exists(vtoken, derivative_index, |maybe_vote| {
				match maybe_vote {
					Some(inner_vote) => {
						inner_vote.checked_sub(vote).map_err(|_| ArithmeticError::Underflow)?;
						Ok(*inner_vote)
					},
					None => Err(Error::<T>::NoData.into()),
				}
			})
		}

		fn compute_new_vote(
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
	}
}

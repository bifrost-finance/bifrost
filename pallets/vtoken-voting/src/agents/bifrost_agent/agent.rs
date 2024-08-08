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

use crate::{agents::bifrost_agent::BifrostCall, pallet, AssetId, Xcm, *};
use bifrost_primitives::{
	CurrencyId, DerivativeIndex, XcmDestWeightAndFeeHandler, XcmOperationType,
};
use core::marker::PhantomData;
use cumulus_primitives_core::QueryId;
use frame_support::{
	dispatch::{DispatchResult, GetDispatchInfo},
	ensure,
	pallet_prelude::*,
	traits::Get,
};
use sp_runtime::traits::Saturating;
use xcm::v4::{Location, Weight as XcmWeight};

use crate::{pallet::Error, traits::*};

/// VotingAgent implementation for Bifrost
pub struct BifrostAgent<T> {
	location: Location,
	_marker: PhantomData<T>,
}

impl<T: pallet::Config> BifrostAgent<T> {
	pub fn new(vtoken: CurrencyId) -> Result<Self, Error<T>> {
		let location = Pallet::<T>::convert_vtoken_to_dest_location(vtoken)?;
		Ok(Self { location, _marker: PhantomData })
	}

	pub fn get_location(&self) -> &Location {
		&self.location
	}

	fn send_xcm_with_notify(
		&self,
		derivative_index: DerivativeIndex,
		call: BifrostCall<T>,
		notify_call: Call<T>,
		transact_weight: XcmWeight,
		extra_fee: BalanceOf<T>,
		f: impl FnOnce(QueryId) -> (),
	) -> DispatchResult {
		let responder = self.get_location().clone();
		let now = frame_system::Pallet::<T>::block_number();
		let timeout = now.saturating_add(T::QueryTimeout::get());
		let notify_runtime_call = <T as Config>::RuntimeCall::from(notify_call);
		let notify_call_weight = notify_runtime_call.get_dispatch_info().weight;
		let query_id = pallet_xcm::Pallet::<T>::new_notify_query(
			responder.clone(),
			notify_runtime_call,
			timeout,
			xcm::v4::Junctions::Here,
		);
		f(query_id);

		let xcm_message = self.construct_xcm_message(
			<BifrostCall<T> as UtilityCall<BifrostCall<T>>>::as_derivative(derivative_index, call)
				.encode(),
			extra_fee,
			transact_weight,
			notify_call_weight,
			query_id,
		)?;

		xcm::v4::send_xcm::<T::XcmRouter>(responder.into(), xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(())
	}

	fn construct_xcm_message(
		&self,
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
}

impl<T: Config> VotingAgent<BalanceOf<T>, AccountIdOf<T>, Error<T>> for BifrostAgent<T> {
	fn vote(
		&self,
		who: AccountIdOf<T>,
		new_delegator_votes: Vec<(DerivativeIndex, AccountVote<BalanceOf<T>>)>,
		poll_index: PollIndex,
		vtoken: CurrencyIdOf<T>,
		submitted: bool,
		maybe_old_vote: Option<(AccountVote<BalanceOf<T>>, BalanceOf<T>)>,
	) -> DispatchResult {
		// send XCM message
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
		let notify_call = Call::<T>::notify_vote { query_id: 0, response: Default::default() };
		let (weight, extra_fee) = T::XcmDestWeightAndFee::get_operation_weight_and_fee(
			CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?,
			XcmOperationType::Vote,
		)
		.ok_or(Error::<T>::NoData)?;

		let derivative_index = new_delegator_votes[0].0;
		self.send_xcm_with_notify(
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
		Ok(())
	}

	fn remove_vote(
		&self,
		class: PollClass,
		poll_index: PollIndex,
		vtoken: CurrencyId,
		derivative_index: DerivativeIndex,
	) -> DispatchResult {
		let notify_call =
			Call::<T>::notify_remove_delegator_vote { query_id: 0, response: Default::default() };
		let remove_vote_call =
			<BifrostCall<T> as ConvictionVotingCall<T>>::remove_vote(Some(class), poll_index);
		let (weight, extra_fee) = T::XcmDestWeightAndFee::get_operation_weight_and_fee(
			CurrencyId::to_token(&vtoken).map_err(|_| Error::<T>::NoData)?,
			XcmOperationType::RemoveVote,
		)
		.ok_or(Error::<T>::NoData)?;
		self.send_xcm_with_notify(
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

		Ok(())
	}
}

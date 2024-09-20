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

use crate::{
	astar_dapp_staking::types::{
		AstarCall, AstarDappStakingPendingStatus, AstarUnlockingRecord, AstarValidator, DappStaking,
	},
	common::types::{
		Delegator, DelegatorIndex, Ledger, PendingStatus, StakingProtocol, Validator, XcmTask,
	},
	Call, Config, ConfigurationByStakingProtocol, Error, Event,
	LedgerByStakingProtocolAndDelegator, Pallet, PendingStatusByQueryId,
	ValidatorsByStakingProtocolAndDelegator,
};
use bifrost_primitives::VtokenMintingOperator;
use frame_support::{dispatch::DispatchResultWithPostInfo, ensure};
use parity_scale_codec::Encode;
use sp_std::{cmp::Ordering, vec::Vec};
use xcm::v4::{opaque::Xcm, Location, QueryId};

pub const ASTAR_DAPP_STAKING: StakingProtocol = StakingProtocol::AstarDappStaking;

impl<T: Config> Pallet<T> {
	pub fn ensure_validator_exist(
		delegator: Delegator<T::AccountId>,
		validator: AstarValidator<T::AccountId>,
	) -> DispatchResultWithPostInfo {
		let validators =
			ValidatorsByStakingProtocolAndDelegator::<T>::get(ASTAR_DAPP_STAKING, delegator);
		let is_exist = validators.iter().any(|storage_validator| match storage_validator {
			Validator::AstarDappStaking(astar_validator) => *astar_validator == validator,
			_ => false,
		});
		ensure!(is_exist, Error::<T>::ValidatorNotFound);
		Ok(().into())
	}

	pub fn do_dapp_staking(
		delegator: Delegator<T::AccountId>,
		task: DappStaking<T::AccountId>,
	) -> DispatchResultWithPostInfo {
		let delegator_index = Self::ensure_delegator_exist(&ASTAR_DAPP_STAKING, &delegator)?;
		let (call, pending_status) = match task.clone() {
			DappStaking::Lock(amount) => (
				AstarCall::<T>::DappStaking(DappStaking::<T::AccountId>::Lock(amount)).encode(),
				Some(PendingStatus::AstarDappStaking(AstarDappStakingPendingStatus::Lock(
					delegator.clone(),
					amount,
				))),
			),
			DappStaking::Unlock(amount) => (
				AstarCall::<T>::DappStaking(DappStaking::<T::AccountId>::Unlock(amount)).encode(),
				Some(PendingStatus::AstarDappStaking(AstarDappStakingPendingStatus::UnLock(
					delegator.clone(),
					amount,
				))),
			),
			DappStaking::ClaimUnlocked => (
				AstarCall::<T>::DappStaking(DappStaking::<T::AccountId>::ClaimUnlocked).encode(),
				Some(PendingStatus::AstarDappStaking(
					AstarDappStakingPendingStatus::ClaimUnlocked(delegator.clone()),
				)),
			),
			DappStaking::Stake(validator, amount) => {
				Self::ensure_validator_exist(delegator.clone(), validator.clone())?;
				(
					AstarCall::<T>::DappStaking(DappStaking::<T::AccountId>::Stake(
						validator.clone(),
						amount,
					))
					.encode(),
					None,
				)
			},
			DappStaking::Unstake(validator, amount) => {
				Self::ensure_validator_exist(delegator.clone(), validator.clone())?;
				(
					AstarCall::<T>::DappStaking(DappStaking::<T::AccountId>::Unstake(
						validator.clone(),
						amount,
					))
					.encode(),
					None,
				)
			},
			DappStaking::ClaimStakerRewards => (
				AstarCall::<T>::DappStaking(DappStaking::<T::AccountId>::ClaimStakerRewards)
					.encode(),
				None,
			),
			DappStaking::ClaimBonusReward(validator) => {
				Self::ensure_validator_exist(delegator.clone(), validator.clone())?;
				(
					AstarCall::<T>::DappStaking(DappStaking::<T::AccountId>::ClaimBonusReward(
						validator,
					))
					.encode(),
					None,
				)
			},
			DappStaking::RelockUnlocking => (
				AstarCall::<T>::DappStaking(DappStaking::<T::AccountId>::RelockUnlocking).encode(),
				None,
			),
		};
		let (query_id, xcm_message) =
			Self::get_query_id_and_xcm_message(call, delegator_index, &pending_status)?;
		if let Some(query_id) = query_id {
			let pending_status = pending_status.clone().ok_or(Error::<T>::XcmFeeNotFound)?;
			PendingStatusByQueryId::<T>::insert(query_id, pending_status.clone());
		}
		Self::send_xcm_message(ASTAR_DAPP_STAKING, xcm_message)?;
		Self::deposit_event(Event::<T>::SendXcmTask {
			query_id,
			delegator,
			task: XcmTask::AstarDappStaking(task),
			pending_status,
			dest_location: ASTAR_DAPP_STAKING.info().remote_dest_location,
		});
		Ok(().into())
	}

	pub fn get_query_id_and_xcm_message(
		call: Vec<u8>,
		delegator_index: DelegatorIndex,
		pending_status: &Option<PendingStatus<T::AccountId>>,
	) -> Result<(Option<QueryId>, Xcm), Error<T>> {
		let call =
			Self::wrap_utility_as_derivative_call_data(&ASTAR_DAPP_STAKING, delegator_index, call);
		let mut query_id = None;
		let xcm_message;
		if pending_status.is_some() {
			let notify_call =
				<T as Config>::RuntimeCall::from(Call::<T>::notify_astar_dapp_staking {
					query_id: 0,
					response: Default::default(),
				});
			xcm_message = Self::wrap_xcm_message_with_notify(
				&ASTAR_DAPP_STAKING,
				call,
				notify_call,
				&mut query_id,
			)?;
		} else {
			xcm_message = Self::wrap_xcm_message(&ASTAR_DAPP_STAKING, call)?;
		};
		Ok((query_id, xcm_message))
	}

	pub fn do_notify_astar_dapp_staking(
		responder: Location,
		pending_status: PendingStatus<T::AccountId>,
	) -> Result<(), Error<T>> {
		let delegator = match pending_status.clone() {
			PendingStatus::AstarDappStaking(AstarDappStakingPendingStatus::Lock(delegator, _)) =>
				delegator,
			PendingStatus::AstarDappStaking(AstarDappStakingPendingStatus::UnLock(
				delegator,
				_,
			)) => delegator,
			PendingStatus::AstarDappStaking(AstarDappStakingPendingStatus::ClaimUnlocked(
				delegator,
			)) => delegator,
		};
		LedgerByStakingProtocolAndDelegator::<T>::mutate(
			ASTAR_DAPP_STAKING,
			delegator,
			|ledger| -> Result<(), Error<T>> {
				if let Some(Ledger::AstarDappStaking(mut pending_ledger)) = ledger.clone() {
					match pending_status.clone() {
						PendingStatus::AstarDappStaking(AstarDappStakingPendingStatus::Lock(
							_,
							amount,
						)) => {
							pending_ledger.add_lock_amount(amount);
						},
						PendingStatus::AstarDappStaking(AstarDappStakingPendingStatus::UnLock(
							_,
							amount,
						)) => {
							pending_ledger.subtract_lock_amount(amount);
							let currency_id = ASTAR_DAPP_STAKING.info().currency_id;
							let current_time_unit =
								T::VtokenMinting::get_ongoing_time_unit(currency_id)
									.ok_or(Error::<T>::TimeUnitNotFound)?;
							let configuration =
								ConfigurationByStakingProtocol::<T>::get(ASTAR_DAPP_STAKING)
									.ok_or(Error::<T>::ConfigurationNotFound)?;
							let unlock_time = current_time_unit
								.add(configuration.unlock_period)
								.ok_or(Error::<T>::TimeUnitNotFound)?;
							pending_ledger
								.unlocking
								.try_push(AstarUnlockingRecord { amount, unlock_time })
								.map_err(|_| Error::<T>::UnlockRecordOverflow)?;
						},
						PendingStatus::AstarDappStaking(
							AstarDappStakingPendingStatus::ClaimUnlocked(_),
						) => {
							let currency_id = ASTAR_DAPP_STAKING.info().currency_id;
							let current_time_unit =
								T::VtokenMinting::get_ongoing_time_unit(currency_id)
									.ok_or(Error::<T>::TimeUnitNotFound)?;
							pending_ledger.unlocking.retain(|record| {
								current_time_unit.cmp(&record.unlock_time) != Ordering::Greater
							});
						},
					};
					*ledger = Some(Ledger::AstarDappStaking(pending_ledger));
				};
				Ok(())
			},
		)?;
		Self::deposit_event(Event::<T>::NotifyResponseReceived { responder, pending_status });
		Ok(())
	}
}

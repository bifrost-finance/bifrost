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

use super::types::{
	MoonbeamCall, MoonbeamCurrencyId, MoonbeamParachainStakingCall, MoonbeamXtokensCall,
};
use crate::{
	agents::{MantaCall, MantaCurrencyId, MantaParachainStakingCall, MantaXtokensCall},
	pallet::{Error, Event},
	primitives::{
		Ledger, MoonbeamLedgerUpdateOperation, OneToManyDelegatorStatus, OneToManyLedger, QueryId,
	},
	traits::{QueryResponseManager, StakingAgent, XcmBuilder},
	AccountIdOf, BalanceOf, Config, DelegatorLedgerXcmUpdateQueue, DelegatorLedgers,
	DelegatorsMultilocation2Index, LedgerUpdateEntry, MinimumsAndMaximums, Pallet, TimeUnit,
	Validators, ValidatorsByDelegatorUpdateEntry,
};
use codec::alloc::collections::BTreeMap;
use core::marker::PhantomData;
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get};
use node_primitives::{
	currency::{GLMR, MANTA, MOVR},
	CurrencyId, VtokenMintingOperator, XcmOperationType,
};
use polkadot_parachain::primitives::Sibling;
use sp_arithmetic::Percent;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, CheckedSub, Convert, UniqueSaturatedInto, Zero},
	DispatchResult,
};
use sp_std::prelude::*;
use xcm::{
	latest::Weight,
	opaque::v3::{
		Instruction,
		Junction::{AccountId32, Parachain},
		Junctions::X1,
		MultiLocation, WeightLimit,
	},
	v3::{prelude::*, Weight as XcmWeight},
	VersionedMultiLocation,
};
use xcm_interface::traits::parachains;

/// StakingAgent implementation for Moonriver/Moonbeam
pub struct MoonbeamAgent<T>(PhantomData<T>);

impl<T> MoonbeamAgent<T> {
	pub fn new() -> Self {
		MoonbeamAgent(PhantomData::<T>)
	}
}

impl<T: Config>
	StakingAgent<
		BalanceOf<T>,
		AccountIdOf<T>,
		LedgerUpdateEntry<BalanceOf<T>>,
		ValidatorsByDelegatorUpdateEntry,
		Error<T>,
	> for MoonbeamAgent<T>
{
	fn initialize_delegator(
		&self,
		currency_id: CurrencyId,
		_delegator_location_op: Option<Box<MultiLocation>>,
	) -> Result<MultiLocation, Error<T>> {
		let new_delegator_id = Pallet::<T>::inner_initialize_delegator(currency_id)?;

		// Generate multi-location by id.
		let delegator_multilocation = T::AccountConverter::convert((new_delegator_id, currency_id));
		ensure!(delegator_multilocation != MultiLocation::default(), Error::<T>::FailToConvert);

		// Add the new delegator into storage
		Pallet::<T>::inner_add_delegator(new_delegator_id, &delegator_multilocation, currency_id)
			.map_err(|_| Error::<T>::FailToAddDelegator)?;

		Ok(delegator_multilocation)
	}

	/// First bond a new validator for a delegator. Moonriver/Moonbeam's corresponding function is
	/// "delegate".
	fn bond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		// First check if the delegator exists.
		// If not, check if amount is greater than minimum delegator stake. Afterwards, create the
		// delegator ledger.
		// If yes, check if amount is greater than minimum delegation requirement.
		let collator = validator.ok_or(Error::<T>::ValidatorNotProvided)?;
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		// Ensure amount is no less than delegation_amount_minimum.
		ensure!(amount >= mins_maxs.delegation_amount_minimum.into(), Error::<T>::LowerThanMinimum);

		// check if the validator is in the white list.
		let validator_list =
			Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;
		validator_list
			.iter()
			.position(|va| va == &collator)
			.ok_or(Error::<T>::ValidatorNotExist)?;

		let ledger_option = DelegatorLedgers::<T>::get(currency_id, who);
		if let Some(Ledger::Moonbeam(ledger)) = ledger_option {
			ensure!(
				ledger.status == OneToManyDelegatorStatus::Active,
				Error::<T>::DelegatorLeaving
			);

			// Ensure the bond after wont exceed delegator_active_staking_maximum
			let active_amount =
				ledger.total.checked_sub(&ledger.less_total).ok_or(Error::<T>::UnderFlow)?;
			let add_total = active_amount.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;
			ensure!(
				add_total <= mins_maxs.delegator_active_staking_maximum,
				Error::<T>::ExceedActiveMaximum
			);

			// check if the delegator-validator delegation exists.
			ensure!(!ledger.delegations.contains_key(&collator), Error::<T>::AlreadyBonded);

			// check if it will exceeds the delegation limit of the delegator.
			let new_deleagtions_count =
				ledger.delegations.len().checked_add(1).ok_or(Error::<T>::OverFlow)?;
			ensure!(
				(new_deleagtions_count as u32) <= mins_maxs.validators_back_maximum,
				Error::<T>::GreaterThanMaximum
			);

		// check if it will exceeds the delegation limit of the validator.
		} else {
			ensure!(amount >= mins_maxs.delegator_bonded_minimum, Error::<T>::LowerThanMinimum);

			// Ensure the bond doesn't exceeds delegator_active_staking_maximum
			ensure!(
				amount <= mins_maxs.delegator_active_staking_maximum,
				Error::<T>::ExceedActiveMaximum
			);

			// Create a new delegator ledger
			// The real bonded amount will be updated by services once the xcm transaction
			// succeeds.
			let empty_delegation_set: BTreeMap<MultiLocation, BalanceOf<T>> = BTreeMap::new();
			let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<T>)> =
				BTreeMap::new();
			let new_ledger = OneToManyLedger::<BalanceOf<T>> {
				account: *who,
				total: Zero::zero(),
				less_total: Zero::zero(),
				delegations: empty_delegation_set,
				requests: vec![],
				request_briefs: request_briefs_set,
				status: OneToManyDelegatorStatus::Active,
			};
			let moonbeam_ledger = Ledger::<BalanceOf<T>>::Moonbeam(new_ledger);

			DelegatorLedgers::<T>::insert(currency_id, who, moonbeam_ledger);
		}

		// prepare xcm call

		// Get the delegator account id in Moonriver/Moonbeam network
		let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;

		// Only allow bond with validators with maximum 1.3 times rewarded delegators. Otherwise,
		// it's too crowded.
		let additional_delegation_count = mins_maxs
			.validators_reward_maximum
			.checked_div(3)
			.ok_or(Error::<T>::Unexpected)?;
		let candidate_delegation_count: u32 = mins_maxs
			.validators_reward_maximum
			.checked_add(additional_delegation_count)
			.ok_or(Error::<T>::OverFlow)?;

		let delegation_count: u32 = mins_maxs.validators_back_maximum;
		// Construct xcm message.
		let call: Vec<u8> = match currency_id {
			MOVR | GLMR => {
				let validator_account_id_20 =
					Pallet::<T>::multilocation_to_h160_account(validator_multilocation)?;
				MoonbeamCall::Staking(MoonbeamParachainStakingCall::<T>::DelegateWithAutoCompound(
					validator_account_id_20,
					amount,
					Percent::from_percent(100),
					candidate_delegation_count,
					candidate_delegation_count,
					delegation_count,
				))
				.encode()
				.into()
			},
			MANTA => {
				let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
				let validator_account_id =
					Pallet::<T>::multilocation_to_account(validator_multilocation)?;
				MantaCall::ParachainStaking(MantaParachainStakingCall::<T>::Delegate(
					validator_account_id,
					amount,
					candidate_delegation_count,
					delegation_count,
				))
				.encode()
				.into()
			},
			_ => Err(Error::<T>::Unsupported)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Bond,
				call,
				who,
				currency_id,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Pallet::<T>::insert_delegator_ledger_update_entry(
			who,
			Some(collator),
			MoonbeamLedgerUpdateOperation::Bond,
			amount,
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Bond extra amount for a existing delegation.
	fn bond_extra(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.bond_extra_minimum, Error::<T>::LowerThanMinimum);

		// check if the delegator exists, if not, return error.
		let collator = (*validator).ok_or(Error::<T>::ValidatorNotProvided)?;

		// need to check if the validator is still in the validators list.
		let validators =
			Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;
		ensure!(validators.contains(&collator), Error::<T>::ValidatorError);

		let ledger_option = DelegatorLedgers::<T>::get(currency_id, who);
		if let Some(Ledger::Moonbeam(ledger)) = ledger_option {
			ensure!(
				ledger.status == OneToManyDelegatorStatus::Active,
				Error::<T>::DelegatorLeaving
			);
			// check if the delegation exists, if not, return error.
			ensure!(ledger.delegations.contains_key(&collator), Error::<T>::ValidatorNotBonded);
			// Ensure the bond after wont exceed delegator_active_staking_maximum
			let active_amount =
				ledger.total.checked_sub(&ledger.less_total).ok_or(Error::<T>::UnderFlow)?;
			let add_total = active_amount.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;
			ensure!(
				add_total <= mins_maxs.delegator_active_staking_maximum,
				Error::<T>::ExceedActiveMaximum
			);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}
		// bond extra amount to the existing delegation.
		// Construct xcm message.
		let call: Vec<u8> = match currency_id {
			MOVR | GLMR => {
				let validator_h160_account = Pallet::<T>::multilocation_to_h160_account(&collator)?;
				MoonbeamCall::Staking(MoonbeamParachainStakingCall::<T>::DelegatorBondMore(
					validator_h160_account,
					amount,
				))
				.encode()
				.into()
			},
			MANTA => {
				let validator_account = Pallet::<T>::multilocation_to_account(&collator)?;
				MantaCall::ParachainStaking(MantaParachainStakingCall::<T>::DelegatorBondMore(
					validator_account,
					amount,
				))
				.encode()
				.into()
			},
			_ => Err(Error::<T>::Unsupported)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::BondExtra,
				call,
				who,
				currency_id,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Pallet::<T>::insert_delegator_ledger_update_entry(
			who,
			Some(collator),
			MoonbeamLedgerUpdateOperation::Bond,
			amount,
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.unbond_minimum, Error::<T>::LowerThanMinimum);

		// check if the delegator exists, if not, return error.
		let collator = (*validator).ok_or(Error::<T>::ValidatorNotProvided)?;

		let ledger_option = DelegatorLedgers::<T>::get(currency_id, who);
		if let Some(Ledger::Moonbeam(ledger)) = ledger_option {
			ensure!(
				ledger.status == OneToManyDelegatorStatus::Active,
				Error::<T>::DelegatorLeaving
			);
			// check if the delegation exists, if not, return error.
			let old_delegate_amount =
				ledger.delegations.get(&collator).ok_or(Error::<T>::ValidatorNotBonded)?;

			// check if there is pending request
			ensure!(!ledger.request_briefs.contains_key(&collator), Error::<T>::AlreadyRequested);

			let delegated_amount_after =
				old_delegate_amount.checked_sub(&amount).ok_or(Error::<T>::UnderFlow)?;
			ensure!(
				delegated_amount_after >= mins_maxs.delegation_amount_minimum.into(),
				Error::<T>::LowerThanMinimum
			);

			// Ensure the unbond after wont below delegator_bonded_minimum
			let active_amount =
				ledger.total.checked_sub(&ledger.less_total).ok_or(Error::<T>::UnderFlow)?;
			let subtracted_total =
				active_amount.checked_sub(&amount).ok_or(Error::<T>::UnderFlow)?;
			ensure!(
				subtracted_total >= mins_maxs.delegator_bonded_minimum,
				Error::<T>::LowerThanMinimum
			);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		// Construct xcm message.
		let call: Vec<u8> = match currency_id {
			MOVR | GLMR => {
				let validator_h160_account = Pallet::<T>::multilocation_to_h160_account(&collator)?;
				MoonbeamCall::Staking(MoonbeamParachainStakingCall::<T>::ScheduleDelegatorBondLess(
					validator_h160_account,
					amount,
				))
				.encode()
				.into()
			},
			MANTA => {
				let validator_account = Pallet::<T>::multilocation_to_account(&collator)?;
				MantaCall::ParachainStaking(
					MantaParachainStakingCall::<T>::ScheduleDelegatorBondLess(
						validator_account,
						amount,
					),
				)
				.encode()
				.into()
			},
			_ => Err(Error::<T>::Unsupported)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Unbond,
				call,
				who,
				currency_id,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Pallet::<T>::insert_delegator_ledger_update_entry(
			who,
			Some(collator),
			MoonbeamLedgerUpdateOperation::BondLess,
			amount,
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Unbonding all amount of a delegator. Equivalent to leave delegator set. The same as Chill
	/// function.
	fn unbond_all(
		&self,
		_who: &MultiLocation,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Cancel pending request
	fn rebond(
		&self,
		who: &MultiLocation,
		_amount: Option<BalanceOf<T>>,
		validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		let collator = (*validator).ok_or(Error::<T>::ValidatorNotProvided)?;

		let ledger_option = DelegatorLedgers::<T>::get(currency_id, who);
		if let Some(Ledger::Moonbeam(ledger)) = ledger_option {
			ensure!(
				ledger.status == OneToManyDelegatorStatus::Active,
				Error::<T>::DelegatorLeaving
			);

			let (_, rebond_amount) =
				ledger.request_briefs.get(&collator).ok_or(Error::<T>::RequestNotExist)?;

			// check if the pending request amount plus active amount greater than delegator minimum
			// request.
			let active =
				ledger.total.checked_sub(&ledger.less_total).ok_or(Error::<T>::UnderFlow)?;
			let rebond_after_amount =
				active.checked_add(&rebond_amount).ok_or(Error::<T>::OverFlow)?;

			// ensure the rebond after amount meet the delegator bond requirement.
			ensure!(
				rebond_after_amount >= mins_maxs.delegator_bonded_minimum,
				Error::<T>::LowerThanMinimum
			);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		// Construct xcm message.
		let call: Vec<u8> = match currency_id {
			MOVR | GLMR => {
				let validator_h160_account = Pallet::<T>::multilocation_to_h160_account(&collator)?;
				MoonbeamCall::Staking(MoonbeamParachainStakingCall::<T>::CancelDelegationRequest(
					validator_h160_account,
				))
				.encode()
				.into()
			},
			MANTA => {
				let validator_account = Pallet::<T>::multilocation_to_account(&collator)?;
				MantaCall::ParachainStaking(
					MantaParachainStakingCall::<T>::CancelDelegationRequest(validator_account),
				)
				.encode()
				.into()
			},
			_ => Err(Error::<T>::Unsupported)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Rebond,
				call,
				who,
				currency_id,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Pallet::<T>::insert_delegator_ledger_update_entry(
			who,
			Some(collator),
			MoonbeamLedgerUpdateOperation::CancelRequest,
			Zero::zero(),
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Delegate to some validators. For Moonriver/Moonbeam, it equals function Nominate.
	fn delegate(
		&self,
		_who: &MultiLocation,
		_targets: &Vec<MultiLocation>,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Revoke a delegation relationship. Only deal with the first validator in the vec.
	fn undelegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		let validator = targets.first().ok_or(Error::<T>::ValidatorNotProvided)?;

		// First, check if the delegator exists.
		let ledger_option = DelegatorLedgers::<T>::get(currency_id, who);
		if let Some(Ledger::Moonbeam(ledger)) = ledger_option {
			ensure!(
				ledger.status == OneToManyDelegatorStatus::Active,
				Error::<T>::DelegatorLeaving
			);
			// Second, check the validators one by one to see if all exist.
			ensure!(ledger.delegations.contains_key(validator), Error::<T>::ValidatorNotBonded);
			ensure!(!ledger.request_briefs.contains_key(validator), Error::<T>::AlreadyRequested);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		// Construct xcm message.
		let call: Vec<u8> = match currency_id {
			MOVR | GLMR => {
				let validator_h160_account =
					Pallet::<T>::multilocation_to_h160_account(&validator)?;
				MoonbeamCall::Staking(MoonbeamParachainStakingCall::<T>::ScheduleRevokeDelegation(
					validator_h160_account,
				))
				.encode()
				.into()
			},
			MANTA => {
				let validator_account = Pallet::<T>::multilocation_to_account(&validator)?;
				MantaCall::ParachainStaking(
					MantaParachainStakingCall::<T>::ScheduleRevokeDelegation(validator_account),
				)
				.encode()
				.into()
			},
			_ => Err(Error::<T>::Unsupported)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Undelegate,
				call,
				who,
				currency_id,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Pallet::<T>::insert_delegator_ledger_update_entry(
			who,
			Some(*validator),
			MoonbeamLedgerUpdateOperation::Revoke,
			Zero::zero(),
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Cancel leave delegator set.
	fn redelegate(
		&self,
		_who: &MultiLocation,
		_targets: &Option<Vec<MultiLocation>>,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Initiate payout for a certain delegator.
	fn payout(
		&self,
		_who: &MultiLocation,
		_validator: &MultiLocation,
		_when: &Option<TimeUnit>,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(
		&self,
		who: &MultiLocation,
		_when: &Option<TimeUnit>,
		validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
		_amount: Option<BalanceOf<T>>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is in the delegator set.
		let collator = (*validator).ok_or(Error::<T>::ValidatorNotProvided)?;
		let mut leaving = false;
		let now = T::VtokenMinting::get_ongoing_time_unit(currency_id)
			.ok_or(Error::<T>::TimeUnitNotExist)?;

		let ledger_option = DelegatorLedgers::<T>::get(currency_id, who);
		let mut due_amount = Zero::zero();
		if let Some(Ledger::Moonbeam(ledger)) = ledger_option {
			// check if the delegator is in the state of leaving. If yes, execute leaving.
			if let OneToManyDelegatorStatus::Leaving(leaving_time) = ledger.status {
				ensure!(now >= leaving_time, Error::<T>::LeavingNotDue);
				leaving = true;
			} else {
				// check if the validator has a delegation request.
				ensure!(ledger.delegations.contains_key(&collator), Error::<T>::ValidatorNotBonded);
				// check whether the request is already due.
				let request_info =
					ledger.request_briefs.get(&collator).ok_or(Error::<T>::RequestNotExist)?;
				let due_time = &request_info.0;
				due_amount = request_info.1;
				ensure!(now >= due_time.clone(), Error::<T>::RequestNotDue);
			}
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		// Construct xcm message.
		let call: Vec<u8> = match currency_id {
			MOVR | GLMR => {
				let delegator_h160_account = Pallet::<T>::multilocation_to_h160_account(who)?;
				let validator_h160_account = Pallet::<T>::multilocation_to_h160_account(&collator)?;
				MoonbeamCall::Staking(MoonbeamParachainStakingCall::<T>::ExecuteDelegationRequest(
					delegator_h160_account,
					validator_h160_account,
				))
				.encode()
				.into()
			},
			MANTA => {
				let delegator_account = Pallet::<T>::multilocation_to_account(who)?;
				let validator_account = Pallet::<T>::multilocation_to_account(&collator)?;
				MantaCall::ParachainStaking(
					MantaParachainStakingCall::<T>::ExecuteDelegationRequest(
						delegator_account,
						validator_account,
					),
				)
				.encode()
				.into()
			},
			_ => Err(Error::<T>::Unsupported)?,
		};

		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Liquidize,
				call,
				who,
				currency_id,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		if leaving {
			Pallet::<T>::insert_delegator_ledger_update_entry(
				who,
				Some(collator),
				MoonbeamLedgerUpdateOperation::ExecuteLeave,
				Zero::zero(),
				query_id,
				timeout,
				currency_id,
			)?;
		} else {
			Pallet::<T>::insert_delegator_ledger_update_entry(
				who,
				Some(collator),
				MoonbeamLedgerUpdateOperation::ExecuteRequest,
				due_amount,
				query_id,
				timeout,
				currency_id,
			)?;
		}

		// Send out the xcm message.
		let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// The same as unbondAll, leaving delegator set.
	fn chill(&self, _who: &MultiLocation, _currency_id: CurrencyId) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Make token transferred back to Bifrost chain account.
	fn transfer_back(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// Ensure amount is greater than zero.
		ensure!(!amount.is_zero(), Error::<T>::AmountZero);

		// Check if from is one of our delegators. If not, return error.
		DelegatorsMultilocation2Index::<T>::get(currency_id, from)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Make sure the receiving account is the Exit_account from vtoken-minting module.
		let to_account_id = Pallet::<T>::multilocation_to_account(&to)?;

		let (_, exit_account) = T::VtokenMinting::get_entrance_and_exit_accounts();
		ensure!(to_account_id == exit_account, Error::<T>::InvalidAccount);

		// Prepare parameter dest and beneficiary.
		let to_32: [u8; 32] = Pallet::<T>::multilocation_to_account_32(&to)?;
		let dest = Box::new(VersionedMultiLocation::from(MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(T::ParachainId::get().into()),
				AccountId32 { network: None, id: to_32 },
			),
		}));

		// Construct xcm message.
		let call: Vec<u8> = match currency_id {
			MOVR | GLMR => MoonbeamCall::Xtokens(MoonbeamXtokensCall::<T>::Transfer(
				MoonbeamCurrencyId::SelfReserve,
				amount.unique_saturated_into(),
				dest,
				WeightLimit::Unlimited,
			))
			.encode()
			.into(),
			MANTA => MantaCall::Xtokens(MantaXtokensCall::<T>::Transfer(
				MantaCurrencyId::MantaCurrency(1),
				amount.unique_saturated_into(),
				dest,
				WeightLimit::Unlimited,
			))
			.encode()
			.into(),
			_ => Err(Error::<T>::Unsupported)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let fee = Pallet::<T>::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperationType::TransferBack,
			call,
			from,
			currency_id,
		)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		Ok(())
	}

	/// Make token from Bifrost chain account to the staking chain account.
	/// Receiving account must be one of the MOVR/GLMR delegators.
	fn transfer_to(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// Make sure receiving account is one of the MOVR/GLMR delegators.
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(currency_id, to),
			Error::<T>::DelegatorNotExist
		);

		// Make sure from account is the entrance account of vtoken-minting module.
		let from_account_id = Pallet::<T>::multilocation_to_account(&from)?;
		let (entrance_account, _) = T::VtokenMinting::get_entrance_and_exit_accounts();
		ensure!(from_account_id == entrance_account, Error::<T>::InvalidAccount);

		Pallet::<T>::do_transfer_to(from, to, amount, currency_id)?;

		Ok(())
	}

	// Convert token to another token.
	fn convert_asset(
		&self,
		_who: &MultiLocation,
		_amount: BalanceOf<T>,
		_currency_id: CurrencyId,
		_if_from_currency: bool,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn tune_vtoken_exchange_rate(
		&self,
		_who: &Option<MultiLocation>,
		token_amount: BalanceOf<T>,
		_vtoken_amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		ensure!(!token_amount.is_zero(), Error::<T>::AmountZero);

		// Tune the vtoken exchange rate.
		T::VtokenMinting::increase_token_pool(currency_id, token_amount)
			.map_err(|_| Error::<T>::IncreaseTokenPoolError)?;

		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		// Get the delegator ledger
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		let total = if let Ledger::Moonbeam(moonbeam_ledger) = ledger {
			moonbeam_ledger.total
		} else {
			Err(Error::<T>::Unexpected)?
		};

		// Check if ledger total amount is zero. If not, return error.
		ensure!(total.is_zero(), Error::<T>::AmountNotZero);

		Pallet::<T>::inner_remove_delegator(who, currency_id)
	}

	/// Charge hosting fee.
	fn charge_hosting_fee(
		&self,
		amount: BalanceOf<T>,
		_from: &MultiLocation,
		to: &MultiLocation,
		currency_id: CurrencyId,
	) -> DispatchResult {
		// Get current VMOVR/MOVR、VGLMR/GLMR exchange rate.
		let vtoken = currency_id.to_vtoken().map_err(|_| Error::<T>::NotSupportedCurrencyId)?;

		let charge_amount =
			Pallet::<T>::inner_calculate_vtoken_hosting_fee(amount, vtoken, currency_id)?;

		Pallet::<T>::inner_charge_hosting_fee(charge_amount, to, vtoken)
	}

	/// Deposit some amount as fee to nominator accounts.
	fn supplement_fee_reserve(
		&self,
		amount: BalanceOf<T>,
		from: &MultiLocation,
		to: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		Pallet::<T>::do_transfer_to(from, to, amount, currency_id)?;

		Ok(())
	}

	fn check_delegator_ledger_query_response(
		&self,
		query_id: QueryId,
		entry: LedgerUpdateEntry<BalanceOf<T>>,
		manual_mode: bool,
		currency_id: CurrencyId,
	) -> Result<bool, Error<T>> {
		// If this is manual mode, it is always updatable.
		let should_update = if manual_mode {
			true
		} else {
			T::SubstrateResponseManager::get_query_response_record(query_id)
		};

		// Update corresponding storages.
		if should_update {
			Pallet::<T>::update_ledger_query_response_storage(
				query_id,
				entry.clone(),
				currency_id,
			)?;

			Pallet::<T>::update_all_occupied_status_storage(currency_id)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorLedgerQueryResponseConfirmed {
				query_id,
				entry,
			});
		}

		Ok(should_update)
	}

	fn check_validators_by_delegator_query_response(
		&self,
		_query_id: QueryId,
		_entry: ValidatorsByDelegatorUpdateEntry,
		_manual_mode: bool,
	) -> Result<bool, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn fail_delegator_ledger_query_response(&self, query_id: QueryId) -> Result<(), Error<T>> {
		// delete pallet_xcm query
		T::SubstrateResponseManager::remove_query_record(query_id);

		// delete update entry
		DelegatorLedgerXcmUpdateQueue::<T>::remove(query_id);

		// Deposit event.
		Pallet::<T>::deposit_event(Event::DelegatorLedgerQueryResponseFailed { query_id });

		Ok(())
	}

	fn fail_validators_by_delegator_query_response(
		&self,
		_query_id: QueryId,
	) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
	}
}

/// Internal functions.
impl<T: Config> MoonbeamAgent<T> {
	fn get_glmr_local_multilocation(currency_id: CurrencyId) -> Result<MultiLocation, Error<T>> {
		match currency_id {
			MOVR => Ok(MultiLocation {
				parents: 0,
				interior: X1(PalletInstance(parachains::moonriver::PALLET_ID)),
			}),
			GLMR => Ok(MultiLocation {
				parents: 0,
				interior: X1(PalletInstance(parachains::moonbeam::PALLET_ID)),
			}),
			_ => Err(Error::<T>::NotSupportedCurrencyId),
		}
	}

	fn inner_construct_xcm_message(
		currency_id: CurrencyId,
		extra_fee: BalanceOf<T>,
	) -> Result<Vec<Instruction>, Error<T>> {
		let multi = Self::get_glmr_local_multilocation(currency_id)?;

		let asset =
			MultiAsset { id: Concrete(multi), fun: Fungible(extra_fee.unique_saturated_into()) };

		let self_sibling_parachain_account: [u8; 20] =
			Sibling::from(T::ParachainId::get()).into_account_truncating();

		Ok(vec![
			WithdrawAsset(asset.clone().into()),
			BuyExecution { fees: asset, weight_limit: Unlimited },
			RefundSurplus,
			DepositAsset {
				assets: AllCounted(8).into(),
				beneficiary: MultiLocation {
					parents: 0,
					interior: X1(AccountKey20 {
						network: None,
						key: self_sibling_parachain_account,
					}),
				},
			},
		])
	}

	fn get_report_transact_status_instruct(query_id: QueryId, max_weight: Weight) -> Instruction {
		ReportTransactStatus(QueryResponseInfo {
			destination: MultiLocation::new(1, X1(Parachain(u32::from(T::ParachainId::get())))),
			query_id,
			max_weight,
		})
	}
}

/// Trait XcmBuilder implementation for Moonriver/Moonbeam
impl<T: Config>
	XcmBuilder<
		BalanceOf<T>,
		MoonbeamCall<T>,
		Error<T>,
		// , MultiLocation,
	> for MoonbeamAgent<T>
{
	fn construct_xcm_message(
		call: MoonbeamCall<T>,
		extra_fee: BalanceOf<T>,
		weight: XcmWeight,
		currency_id: CurrencyId,
		query_id: Option<QueryId>,
	) -> Result<Xcm<()>, Error<T>> {
		let mut xcm_message = Self::inner_construct_xcm_message(currency_id, extra_fee)?;
		let transact = Transact {
			origin_kind: OriginKind::SovereignAccount,
			require_weight_at_most: weight,
			call: call.encode().into(),
		};
		xcm_message.insert(2, transact);
		if let Some(query_id) = query_id {
			let report_transact_status_instruct =
				Self::get_report_transact_status_instruct(query_id, weight);
			xcm_message.insert(3, report_transact_status_instruct);
		}
		Ok(Xcm(xcm_message))
	}
}

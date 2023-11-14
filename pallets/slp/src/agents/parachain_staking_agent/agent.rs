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
		Ledger, OneToManyDelegationAction, OneToManyDelegatorStatus, OneToManyLedger,
		OneToManyScheduledRequest, ParachainStakingLedgerUpdateEntry,
		ParachainStakingLedgerUpdateOperation, QueryId,
	},
	traits::{QueryResponseManager, StakingAgent},
	AccountIdOf, BalanceOf, Config, DelegatorLedgerXcmUpdateQueue, DelegatorLedgers,
	DelegatorsMultilocation2Index, LedgerUpdateEntry, MinimumsAndMaximums, Pallet, TimeUnit,
	Validators, ValidatorsByDelegatorUpdateEntry, BNC,
};
use codec::alloc::collections::BTreeMap;
use core::marker::PhantomData;
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get};
use node_primitives::{
	currency::{GLMR, MANTA, MOVR},
	CurrencyId, VtokenMintingOperator, XcmOperationType,
};
use orml_traits::MultiCurrency;
use parachain_staking::ParachainStakingInterface;
use sp_arithmetic::Percent;
use sp_runtime::{
	traits::{CheckedAdd, CheckedSub, Convert, UniqueSaturatedInto, Zero},
	DispatchResult,
};
use sp_std::prelude::*;
use xcm::{
	opaque::v3::{
		Junction::{AccountId32, Parachain},
		MultiLocation, WeightLimit,
	},
	v3::prelude::*,
	VersionedMultiLocation,
};

/// StakingAgent implementation for Moonriver/Moonbeam
pub struct ParachainStakingAgent<T>(PhantomData<T>);

impl<T> ParachainStakingAgent<T> {
	pub fn new() -> Self {
		ParachainStakingAgent(PhantomData::<T>)
	}
}

impl<T: Config>
	StakingAgent<
		BalanceOf<T>,
		AccountIdOf<T>,
		LedgerUpdateEntry<BalanceOf<T>>,
		ValidatorsByDelegatorUpdateEntry,
		Error<T>,
	> for ParachainStakingAgent<T>
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
		if let Some(Ledger::ParachainStaking(ledger)) = ledger_option {
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
			let moonbeam_ledger = Ledger::<BalanceOf<T>>::ParachainStaking(new_ledger);

			DelegatorLedgers::<T>::insert(currency_id, who, moonbeam_ledger);
		}

		// prepare xcm call

		// Get the delegator account id in Moonriver/Moonbeam network
		let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;

		let mut query_index = 0;
		if currency_id == BNC {
			let validator_account_id =
				Pallet::<T>::multilocation_to_account(validator_multilocation)?;
			let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;

			let (delegation_count, candidate_delegation_count) =
				T::ParachainStaking::get_delegation_count(
					delegator_account_id.clone(),
					validator_account_id.clone(),
				);

			T::ParachainStaking::delegate(
				delegator_account_id,
				validator_account_id,
				amount,
				candidate_delegation_count,
				delegation_count,
			)
			.map_err(|_| Error::<T>::Unexpected)?;

			DelegatorLedgers::<T>::mutate_exists(
				currency_id,
				who,
				|old_ledger_opt| -> Result<(), Error<T>> {
					if let Some(Ledger::ParachainStaking(ref mut old_ledger)) = old_ledger_opt {
						// first bond and bond more operations
						// If this is a bonding operation.
						// Increase the total amount and add the delegation relationship.
						ensure!(
							old_ledger.status == OneToManyDelegatorStatus::Active,
							Error::<T>::DelegatorLeaving
						);
						old_ledger.total =
							old_ledger.total.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;

						let amount_rs = old_ledger.delegations.get(validator_multilocation);
						let original_amount =
							if let Some(amt) = amount_rs { *amt } else { Zero::zero() };

						let new_amount =
							original_amount.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;
						old_ledger.delegations.insert(*validator_multilocation, new_amount);
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)
					}
				},
			)?;

			Pallet::<T>::update_all_occupied_status_storage(currency_id)?;
		} else {
			// Only allow bond with validators with maximum 1.3 times rewarded delegators.
			// Otherwise, it's too crowded.
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
					MoonbeamCall::Staking(
						MoonbeamParachainStakingCall::<T>::DelegateWithAutoCompound(
							validator_account_id_20,
							amount,
							Percent::from_percent(100),
							candidate_delegation_count,
							candidate_delegation_count,
							delegation_count,
						),
					)
					.encode()
					.into()
				},
				MANTA => {
					let validator_multilocation =
						validator.as_ref().ok_or(Error::<T>::Unexpected)?;
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
				ParachainStakingLedgerUpdateOperation::Bond,
				amount,
				query_id,
				timeout,
				currency_id,
			)?;

			// Send out the xcm message.
			let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
			send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

			query_index = query_id;
		}

		Ok(query_index)
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
		if let Some(Ledger::ParachainStaking(ledger)) = ledger_option {
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

		let mut query_index = 0;
		if currency_id == BNC {
			// bond extra amount to the existing delegation.
			let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
			let validator_account_id =
				Pallet::<T>::multilocation_to_account(validator_multilocation)?;
			let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;

			T::ParachainStaking::delegator_bond_more(
				delegator_account_id,
				validator_account_id,
				amount,
			)
			.map_err(|_| Error::<T>::Unexpected)?;

			DelegatorLedgers::<T>::mutate_exists(
				currency_id,
				who,
				|old_ledger_opt| -> Result<(), Error<T>> {
					if let Some(Ledger::ParachainStaking(ref mut old_ledger)) = old_ledger_opt {
						// first bond and bond more operations
						// If this is a bonding operation.
						// Increase the total amount and add the delegation relationship.
						ensure!(
							old_ledger.status == OneToManyDelegatorStatus::Active,
							Error::<T>::DelegatorLeaving
						);
						old_ledger.total =
							old_ledger.total.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;

						let amount_rs = old_ledger.delegations.get(validator_multilocation);
						let original_amount =
							if let Some(amt) = amount_rs { *amt } else { Zero::zero() };

						let new_amount =
							original_amount.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;
						old_ledger.delegations.insert(*validator_multilocation, new_amount);
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)
					}
				},
			)?;

			Pallet::<T>::update_all_occupied_status_storage(currency_id)?;
		} else {
			// bond extra amount to the existing delegation.
			// Construct xcm message.
			let call: Vec<u8> = match currency_id {
				MOVR | GLMR => {
					let validator_h160_account =
						Pallet::<T>::multilocation_to_h160_account(&collator)?;
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
				ParachainStakingLedgerUpdateOperation::Bond,
				amount,
				query_id,
				timeout,
				currency_id,
			)?;

			// Send out the xcm message.
			let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
			send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;
			query_index = query_id;
		}

		Ok(query_index)
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
		if let Some(Ledger::ParachainStaking(ledger)) = ledger_option {
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

		let mut query_index = 0;
		if currency_id == BNC {
			let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
			let validator_account_id =
				Pallet::<T>::multilocation_to_account(validator_multilocation)?;
			let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;

			T::ParachainStaking::schedule_delegator_bond_less(
				delegator_account_id,
				validator_account_id,
				amount,
			)
			.map_err(|_| Error::<T>::Unexpected)?;

			DelegatorLedgers::<T>::mutate_exists(
				currency_id,
				who,
				|old_ledger_opt| -> Result<(), Error<T>> {
					if let Some(Ledger::ParachainStaking(ref mut old_ledger)) = old_ledger_opt {
						ensure!(
							old_ledger.status == OneToManyDelegatorStatus::Active,
							Error::<T>::DelegatorLeaving
						);

						old_ledger.less_total = old_ledger
							.less_total
							.checked_add(&amount)
							.ok_or(Error::<T>::OverFlow)?;

						let unlock_time_unit =
							Pallet::<T>::get_unlocking_time_unit_from_current(false, currency_id)?
								.ok_or(Error::<T>::TimeUnitNotExist)?;

						// add a new entry in requests and request_briefs
						let new_request = OneToManyScheduledRequest {
							validator: *validator_multilocation,
							when_executable: unlock_time_unit.clone(),
							action: OneToManyDelegationAction::<BalanceOf<T>>::Decrease(amount),
						};
						old_ledger.requests.push(new_request);
						old_ledger
							.request_briefs
							.insert(*validator_multilocation, (unlock_time_unit, amount));
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)
					}
				},
			)?;

			Pallet::<T>::update_all_occupied_status_storage(currency_id)?;
		} else {
			// Construct xcm message.
			let call: Vec<u8> = match currency_id {
				MOVR | GLMR => {
					let validator_h160_account =
						Pallet::<T>::multilocation_to_h160_account(&collator)?;
					MoonbeamCall::Staking(
						MoonbeamParachainStakingCall::<T>::ScheduleDelegatorBondLess(
							validator_h160_account,
							amount,
						),
					)
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
				ParachainStakingLedgerUpdateOperation::BondLess,
				amount,
				query_id,
				timeout,
				currency_id,
			)?;

			// Send out the xcm message.
			let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
			send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;
			query_index = query_id;
		}

		Ok(query_index)
	}

	/// Unbonding all amount of a delegator. Equivalent to leave delegator set. The same as Chill
	/// function.
	fn unbond_all(
		&self,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		ensure!(currency_id == BNC, Error::<T>::Unsupported);

		// check if the delegator exists.
		let ledger_option = DelegatorLedgers::<T>::get(currency_id, who);

		if let Some(Ledger::ParachainStaking(ledger)) = ledger_option {
			// check if the delegator is in the state of leaving.
			ensure!(ledger.status == OneToManyDelegatorStatus::Active, Error::<T>::AlreadyLeaving);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;

		T::ParachainStaking::schedule_leave_delegators(delegator_account_id)
			.map_err(|_| Error::<T>::Unexpected)?;

		DelegatorLedgers::<T>::mutate_exists(
			currency_id,
			who,
			|old_ledger_opt| -> Result<(), Error<T>> {
				if let Some(Ledger::ParachainStaking(ref mut old_ledger)) = old_ledger_opt {
					ensure!(
						old_ledger.status == OneToManyDelegatorStatus::Active,
						Error::<T>::DelegatorAlreadyLeaving
					);

					old_ledger.less_total = old_ledger.total;
					let unlock_time =
						Pallet::<T>::get_unlocking_time_unit_from_current(false, currency_id)?
							.ok_or(Error::<T>::TimeUnitNotExist)?;
					old_ledger.status = OneToManyDelegatorStatus::Leaving(unlock_time.clone());

					let mut new_requests = vec![];
					let new_request_briefs: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<T>)> =
						BTreeMap::new();
					for (vali, amt) in old_ledger.delegations.iter() {
						let request_entry = OneToManyScheduledRequest {
							validator: *vali,
							when_executable: unlock_time.clone(),
							action: OneToManyDelegationAction::<BalanceOf<T>>::Revoke(*amt),
						};
						new_requests.push(request_entry);

						old_ledger.request_briefs.insert(*vali, (unlock_time.clone(), *amt));
					}

					old_ledger.requests = new_requests;
					old_ledger.request_briefs = new_request_briefs;
					Ok(())
				} else {
					Err(Error::<T>::Unexpected)
				}
			},
		)?;

		Pallet::<T>::update_all_occupied_status_storage(currency_id)?;

		Ok(0)
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
		if let Some(Ledger::ParachainStaking(ledger)) = ledger_option {
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

		let mut query_index = 0;
		if currency_id == BNC {
			let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
			let validator_account_id =
				Pallet::<T>::multilocation_to_account(validator_multilocation)?;
			let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;

			T::ParachainStaking::cancel_delegation_request(
				delegator_account_id,
				validator_account_id,
			)
			.map_err(|_| Error::<T>::Unexpected)?;

			DelegatorLedgers::<T>::mutate_exists(
				currency_id,
				who,
				|old_ledger_opt| -> Result<(), Error<T>> {
					if let Some(Ledger::ParachainStaking(ref mut old_ledger)) = old_ledger_opt {
						ensure!(
							old_ledger.status == OneToManyDelegatorStatus::Active,
							Error::<T>::DelegatorLeaving
						);

						let (_, cancel_amount) = old_ledger
							.request_briefs
							.get(validator_multilocation)
							.ok_or(Error::<T>::Unexpected)?;

						old_ledger.less_total = old_ledger
							.less_total
							.checked_sub(&cancel_amount)
							.ok_or(Error::<T>::UnderFlow)?;

						let request_index = old_ledger
							.requests
							.iter()
							.position(|rqst| rqst.validator == *validator_multilocation)
							.ok_or(Error::<T>::Unexpected)?;
						old_ledger.requests.remove(request_index);

						old_ledger.request_briefs.remove(validator_multilocation);
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)
					}
				},
			)?;

			Pallet::<T>::update_all_occupied_status_storage(currency_id)?;
		} else {
			// Construct xcm message.
			let call: Vec<u8> = match currency_id {
				MOVR | GLMR => {
					let validator_h160_account =
						Pallet::<T>::multilocation_to_h160_account(&collator)?;
					MoonbeamCall::Staking(
						MoonbeamParachainStakingCall::<T>::CancelDelegationRequest(
							validator_h160_account,
						),
					)
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
				ParachainStakingLedgerUpdateOperation::CancelRequest,
				Zero::zero(),
				query_id,
				timeout,
				currency_id,
			)?;

			// Send out the xcm message.
			let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
			send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;
			query_index = query_id;
		}

		Ok(query_index)
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
		if let Some(Ledger::ParachainStaking(ledger)) = ledger_option {
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

		let mut query_index = 0;
		if currency_id == BNC {
			let validator_account_id = Pallet::<T>::multilocation_to_account(validator)?;
			let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;

			T::ParachainStaking::schedule_revoke_delegation(
				delegator_account_id,
				validator_account_id,
			)
			.map_err(|_| Error::<T>::Unexpected)?;

			DelegatorLedgers::<T>::mutate_exists(
				currency_id,
				who,
				|old_ledger_opt| -> Result<(), Error<T>> {
					if let Some(Ledger::ParachainStaking(ref mut old_ledger)) = old_ledger_opt {
						ensure!(
							old_ledger.status == OneToManyDelegatorStatus::Active,
							Error::<T>::DelegatorLeaving
						);

						let revoke_amount =
							old_ledger.delegations.get(validator).ok_or(Error::<T>::Unexpected)?;

						old_ledger.less_total = old_ledger
							.less_total
							.checked_add(&revoke_amount)
							.ok_or(Error::<T>::OverFlow)?;

						let unlock_time_unit =
							Pallet::<T>::get_unlocking_time_unit_from_current(false, currency_id)?
								.ok_or(Error::<T>::TimeUnitNotExist)?;

						// add a new entry in requests and request_briefs
						let new_request = OneToManyScheduledRequest {
							validator: *validator,
							when_executable: unlock_time_unit.clone(),
							action: OneToManyDelegationAction::<BalanceOf<T>>::Revoke(
								*revoke_amount,
							),
						};
						old_ledger.requests.push(new_request);
						old_ledger
							.request_briefs
							.insert(*validator, (unlock_time_unit, *revoke_amount));
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)
					}
				},
			)?;

			Pallet::<T>::update_all_occupied_status_storage(currency_id)?;
		} else {
			// Construct xcm message.
			let call: Vec<u8> = match currency_id {
				MOVR | GLMR => {
					let validator_h160_account =
						Pallet::<T>::multilocation_to_h160_account(&validator)?;
					MoonbeamCall::Staking(
						MoonbeamParachainStakingCall::<T>::ScheduleRevokeDelegation(
							validator_h160_account,
						),
					)
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
				ParachainStakingLedgerUpdateOperation::Revoke,
				Zero::zero(),
				query_id,
				timeout,
				currency_id,
			)?;

			// Send out the xcm message.
			let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
			send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

			query_index = query_id;
		}
		Ok(query_index)
	}

	/// Cancel leave delegator set.
	fn redelegate(
		&self,
		who: &MultiLocation,
		_targets: &Option<Vec<MultiLocation>>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		ensure!(currency_id == BNC, Error::<T>::Unsupported);

		// first check if the delegator exists.
		let ledger_option = DelegatorLedgers::<T>::get(currency_id, who);
		if let Some(Ledger::ParachainStaking(ledger)) = ledger_option {
			// check if the delegator is in the state of leaving.
			match ledger.status {
				OneToManyDelegatorStatus::Leaving(_) => Ok(()),
				_ => Err(Error::<T>::DelegatorNotLeaving),
			}?;
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;

		T::ParachainStaking::cancel_leave_delegators(delegator_account_id)
			.map_err(|_| Error::<T>::Unexpected)?;

		DelegatorLedgers::<T>::mutate_exists(
			currency_id,
			who,
			|old_ledger_opt| -> Result<(), Error<T>> {
				if let Some(Ledger::ParachainStaking(ref mut old_ledger)) = old_ledger_opt {
					let leaving = matches!(old_ledger.status, OneToManyDelegatorStatus::Leaving(_));
					ensure!(leaving, Error::<T>::DelegatorNotLeaving);

					old_ledger.less_total = Zero::zero();
					old_ledger.status = OneToManyDelegatorStatus::Active;

					old_ledger.requests = vec![];
					old_ledger.request_briefs = BTreeMap::new();
					Ok(())
				} else {
					Err(Error::<T>::Unexpected)
				}
			},
		)?;

		Pallet::<T>::update_all_occupied_status_storage(currency_id)?;

		Ok(0)
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
		if let Some(Ledger::ParachainStaking(ledger)) = ledger_option {
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

		let mut query_index = 0;
		if currency_id == BNC {
			let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
			let validator_account_id =
				Pallet::<T>::multilocation_to_account(validator_multilocation)?;
			let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;
			let mins_maxs =
				MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;

			if leaving {
				T::ParachainStaking::execute_leave_delegators(
					delegator_account_id,
					mins_maxs.validators_back_maximum,
				)
				.map_err(|_| Error::<T>::Unexpected)?;
				DelegatorLedgers::<T>::mutate_exists(
					currency_id,
					who,
					|old_ledger_opt| -> Result<(), Error<T>> {
						if let Some(Ledger::ParachainStaking(ref mut old_ledger)) = old_ledger_opt {
							// make sure leaving time is less than or equal to current time.
							let scheduled_time =
								if let OneToManyDelegatorStatus::Leaving(scheduled_time_unit) =
									old_ledger.clone().status
								{
									if let TimeUnit::Round(tu) = scheduled_time_unit {
										tu
									} else {
										Err(Error::<T>::InvalidTimeUnit)?
									}
								} else {
									Err(Error::<T>::DelegatorNotLeaving)?
								};

							let current_time_unit =
								Pallet::<T>::get_unlocking_time_unit_from_current(
									false,
									currency_id,
								)?
								.ok_or(Error::<T>::TimeUnitNotExist)?;

							if let TimeUnit::Round(current_time) = current_time_unit {
								ensure!(current_time >= scheduled_time, Error::<T>::LeavingNotDue);
							} else {
								Err(Error::<T>::InvalidTimeUnit)?;
							}

							let empty_delegation_set: BTreeMap<MultiLocation, BalanceOf<T>> =
								BTreeMap::new();
							let request_briefs_set: BTreeMap<
								MultiLocation,
								(TimeUnit, BalanceOf<T>),
							> = BTreeMap::new();
							let new_ledger = OneToManyLedger::<BalanceOf<T>> {
								account: old_ledger.clone().account,
								total: Zero::zero(),
								less_total: Zero::zero(),
								delegations: empty_delegation_set,
								requests: vec![],
								request_briefs: request_briefs_set,
								status: OneToManyDelegatorStatus::Active,
							};
							let parachain_staking_ledger =
								Ledger::<BalanceOf<T>>::ParachainStaking(new_ledger);

							*old_ledger_opt = Some(parachain_staking_ledger);
							Ok(())
						} else {
							Err(Error::<T>::Unexpected)
						}
					},
				)?;

				Pallet::<T>::update_all_occupied_status_storage(currency_id)?;
			} else {
				T::ParachainStaking::execute_delegation_request(
					delegator_account_id,
					validator_account_id,
				)
				.map_err(|_| Error::<T>::Unexpected)?;
				DelegatorLedgers::<T>::mutate_exists(
					currency_id,
					who,
					|old_ledger_opt| -> Result<(), Error<T>> {
						if let Some(Ledger::ParachainStaking(ref mut old_ledger)) = old_ledger_opt {
							ensure!(
								old_ledger.status == OneToManyDelegatorStatus::Active,
								Error::<T>::DelegatorLeaving
							);

							// ensure current round is no less than executable time.
							let execute_time_unit =
								Pallet::<T>::get_unlocking_time_unit_from_current(
									false,
									currency_id,
								)?
								.ok_or(Error::<T>::TimeUnitNotExist)?;

							let execute_round =
								if let TimeUnit::Round(current_round) = execute_time_unit {
									current_round
								} else {
									Err(Error::<T>::InvalidTimeUnit)?
								};

							let request_time_unit = old_ledger
								.request_briefs
								.get(validator_multilocation)
								.ok_or(Error::<T>::RequestNotExist)?;

							let request_round =
								if let TimeUnit::Round(req_round) = request_time_unit.0 {
									req_round
								} else {
									Err(Error::<T>::InvalidTimeUnit)?
								};

							ensure!(execute_round >= request_round, Error::<T>::RequestNotDue);

							let (_, execute_amount) = old_ledger
								.request_briefs
								.remove(validator_multilocation)
								.ok_or(Error::<T>::Unexpected)?;
							old_ledger.total = old_ledger
								.total
								.checked_sub(&execute_amount)
								.ok_or(Error::<T>::UnderFlow)?;

							old_ledger.less_total = old_ledger
								.less_total
								.checked_sub(&execute_amount)
								.ok_or(Error::<T>::UnderFlow)?;

							let request_index = old_ledger
								.requests
								.iter()
								.position(|rqst| rqst.validator == *validator_multilocation)
								.ok_or(Error::<T>::RequestNotExist)?;

							old_ledger.requests.remove(request_index);

							let old_delegate_amount = old_ledger
								.delegations
								.get(validator_multilocation)
								.ok_or(Error::<T>::ValidatorNotBonded)?;
							let new_delegate_amount = old_delegate_amount
								.checked_sub(&execute_amount)
								.ok_or(Error::<T>::UnderFlow)?;

							if new_delegate_amount == Zero::zero() {
								old_ledger
									.delegations
									.remove(validator_multilocation)
									.ok_or(Error::<T>::Unexpected)?;
							} else {
								old_ledger
									.delegations
									.insert(*validator_multilocation, new_delegate_amount);
							}
							Ok(())
						} else {
							Err(Error::<T>::Unexpected)
						}
					},
				)?;

				Pallet::<T>::update_all_occupied_status_storage(currency_id)?;
			}
		} else {
			// Construct xcm message.
			let call: Vec<u8> = match currency_id {
				MOVR | GLMR => {
					let delegator_h160_account = Pallet::<T>::multilocation_to_h160_account(who)?;
					let validator_h160_account =
						Pallet::<T>::multilocation_to_h160_account(&collator)?;
					MoonbeamCall::Staking(
						MoonbeamParachainStakingCall::<T>::ExecuteDelegationRequest(
							delegator_h160_account,
							validator_h160_account,
						),
					)
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
					ParachainStakingLedgerUpdateOperation::ExecuteLeave,
					Zero::zero(),
					query_id,
					timeout,
					currency_id,
				)?;
			} else {
				Pallet::<T>::insert_delegator_ledger_update_entry(
					who,
					Some(collator),
					ParachainStakingLedgerUpdateOperation::ExecuteRequest,
					due_amount,
					query_id,
					timeout,
					currency_id,
				)?;
			}

			// Send out the xcm message.
			let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
			send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

			query_index = query_id;
		}
		Ok(query_index)
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

		if currency_id == BNC {
			let from_account = Pallet::<T>::multilocation_to_account(from)?;
			T::MultiCurrency::transfer(currency_id, &from_account, &to_account_id, amount)
				.map_err(|_| Error::<T>::Unexpected)?;
		} else {
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
		}

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

		if currency_id == BNC {
			let to_account = Pallet::<T>::multilocation_to_account(to)?;
			T::MultiCurrency::transfer(currency_id, &from_account_id, &to_account, amount)
				.map_err(|_| Error::<T>::Unexpected)?;
		} else {
			Pallet::<T>::do_transfer_to(from, to, amount, currency_id)?;
		}

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

		let total = if let Ledger::ParachainStaking(moonbeam_ledger) = ledger {
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
		if currency_id == BNC {
			ensure!(!amount.is_zero(), Error::<T>::AmountZero);
			let from_account_id = Pallet::<T>::multilocation_to_account(from)?;
			let to_account_id = Pallet::<T>::multilocation_to_account(to)?;
			T::MultiCurrency::transfer(currency_id, &from_account_id, &to_account_id, amount)
				.map_err(|_e| Error::<T>::MultiCurrencyError)?;
		} else {
			Pallet::<T>::do_transfer_to(from, to, amount, currency_id)?;
		}

		Ok(())
	}

	fn check_delegator_ledger_query_response(
		&self,
		query_id: QueryId,
		entry: LedgerUpdateEntry<BalanceOf<T>>,
		manual_mode: bool,
		currency_id: CurrencyId,
	) -> Result<bool, Error<T>> {
		ensure!(currency_id != BNC, Error::<T>::Unsupported);
		// If this is manual mode, it is always updatable.
		let should_update = if manual_mode {
			true
		} else {
			T::SubstrateResponseManager::get_query_response_record(query_id)
		};

		// Update corresponding storages.
		if should_update {
			Self::update_ledger_query_response_storage(query_id, entry.clone(), currency_id)?;

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

impl<T: Config> ParachainStakingAgent<T> {
	fn update_ledger_query_response_storage(
		query_id: QueryId,
		query_entry: LedgerUpdateEntry<BalanceOf<T>>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		use ParachainStakingLedgerUpdateOperation::{
			Bond, BondLess, CancelLeave, CancelRequest, ExecuteLeave, ExecuteRequest,
			LeaveDelegator, Revoke,
		};
		// update DelegatorLedgers<T> storage
		if let LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
			currency_id: _,
			delegator_id,
			validator_id: validator_id_op,
			update_operation,
			amount,
			unlock_time,
		}) = query_entry
		{
			DelegatorLedgers::<T>::mutate_exists(
				currency_id,
				delegator_id,
				|old_ledger_opt| -> Result<(), Error<T>> {
					if let Some(Ledger::ParachainStaking(ref mut old_ledger)) = old_ledger_opt {
						match update_operation {
							// first bond and bond more operations
							Bond => {
								let validator_id =
									validator_id_op.ok_or(Error::<T>::ValidatorError)?;

								// If this is a bonding operation.
								// Increase the total amount and add the delegation relationship.
								ensure!(
									old_ledger.status == OneToManyDelegatorStatus::Active,
									Error::<T>::DelegatorLeaving
								);
								old_ledger.total = old_ledger
									.total
									.checked_add(&amount)
									.ok_or(Error::<T>::OverFlow)?;

								let amount_rs = old_ledger.delegations.get(&validator_id);
								let original_amount =
									if let Some(amt) = amount_rs { *amt } else { Zero::zero() };

								let new_amount = original_amount
									.checked_add(&amount)
									.ok_or(Error::<T>::OverFlow)?;
								old_ledger.delegations.insert(validator_id, new_amount);
							},
							// schedule bond less request
							BondLess => {
								let validator_id =
									validator_id_op.ok_or(Error::<T>::ValidatorError)?;

								ensure!(
									old_ledger.status == OneToManyDelegatorStatus::Active,
									Error::<T>::DelegatorLeaving
								);

								old_ledger.less_total = old_ledger
									.less_total
									.checked_add(&amount)
									.ok_or(Error::<T>::OverFlow)?;

								let unlock_time_unit =
									unlock_time.ok_or(Error::<T>::TimeUnitNotExist)?;

								// add a new entry in requests and request_briefs
								let new_request = OneToManyScheduledRequest {
									validator: validator_id,
									when_executable: unlock_time_unit.clone(),
									action: OneToManyDelegationAction::<BalanceOf<T>>::Decrease(
										amount,
									),
								};
								old_ledger.requests.push(new_request);
								old_ledger
									.request_briefs
									.insert(validator_id, (unlock_time_unit, amount));
							},
							// schedule revoke request
							Revoke => {
								let validator_id =
									validator_id_op.ok_or(Error::<T>::ValidatorError)?;

								ensure!(
									old_ledger.status == OneToManyDelegatorStatus::Active,
									Error::<T>::DelegatorLeaving
								);

								let revoke_amount = old_ledger
									.delegations
									.get(&validator_id)
									.ok_or(Error::<T>::Unexpected)?;

								old_ledger.less_total = old_ledger
									.less_total
									.checked_add(&revoke_amount)
									.ok_or(Error::<T>::OverFlow)?;

								let unlock_time_unit =
									unlock_time.ok_or(Error::<T>::TimeUnitNotExist)?;

								// add a new entry in requests and request_briefs
								let new_request = OneToManyScheduledRequest {
									validator: validator_id,
									when_executable: unlock_time_unit.clone(),
									action: OneToManyDelegationAction::<BalanceOf<T>>::Revoke(
										*revoke_amount,
									),
								};
								old_ledger.requests.push(new_request);
								old_ledger
									.request_briefs
									.insert(validator_id, (unlock_time_unit, *revoke_amount));
							},
							// cancel bond less or revoke request
							CancelRequest => {
								let validator_id =
									validator_id_op.ok_or(Error::<T>::ValidatorError)?;

								ensure!(
									old_ledger.status == OneToManyDelegatorStatus::Active,
									Error::<T>::DelegatorLeaving
								);

								let (_, cancel_amount) = old_ledger
									.request_briefs
									.get(&validator_id)
									.ok_or(Error::<T>::Unexpected)?;

								old_ledger.less_total = old_ledger
									.less_total
									.checked_sub(&cancel_amount)
									.ok_or(Error::<T>::UnderFlow)?;

								let request_index = old_ledger
									.requests
									.iter()
									.position(|request| request.validator == validator_id)
									.ok_or(Error::<T>::Unexpected)?;
								old_ledger.requests.remove(request_index);

								old_ledger.request_briefs.remove(&validator_id);
							},
							// schedule leave
							LeaveDelegator => {
								ensure!(
									old_ledger.status == OneToManyDelegatorStatus::Active,
									Error::<T>::DelegatorAlreadyLeaving
								);

								old_ledger.less_total = old_ledger.total;
								let unlock_time =
									unlock_time.ok_or(Error::<T>::TimeUnitNotExist)?;
								old_ledger.status =
									OneToManyDelegatorStatus::Leaving(unlock_time.clone());

								let mut new_requests = vec![];
								let new_request_briefs: BTreeMap<
									MultiLocation,
									(TimeUnit, BalanceOf<T>),
								> = BTreeMap::new();
								for (vali, amt) in old_ledger.delegations.iter() {
									let request_entry = OneToManyScheduledRequest {
										validator: *vali,
										when_executable: unlock_time.clone(),
										action: OneToManyDelegationAction::<BalanceOf<T>>::Revoke(
											*amt,
										),
									};
									new_requests.push(request_entry);

									old_ledger
										.request_briefs
										.insert(*vali, (unlock_time.clone(), *amt));
								}

								old_ledger.requests = new_requests;
								old_ledger.request_briefs = new_request_briefs;
							},
							// cancel leave
							CancelLeave => {
								let leaving = matches!(
									old_ledger.status,
									OneToManyDelegatorStatus::Leaving(_)
								);
								ensure!(leaving, Error::<T>::DelegatorNotLeaving);

								old_ledger.less_total = Zero::zero();
								old_ledger.status = OneToManyDelegatorStatus::Active;

								old_ledger.requests = vec![];
								old_ledger.request_briefs = BTreeMap::new();
							},
							// execute leaving
							ExecuteLeave => {
								// make sure leaving time is less than or equal to current time.
								let scheduled_time =
									if let OneToManyDelegatorStatus::Leaving(scheduled_time_unit) =
										old_ledger.clone().status
									{
										if let TimeUnit::Round(tu) = scheduled_time_unit {
											tu
										} else {
											Err(Error::<T>::InvalidTimeUnit)?
										}
									} else {
										Err(Error::<T>::DelegatorNotLeaving)?
									};

								let current_time_unit =
									unlock_time.ok_or(Error::<T>::TimeUnitNotExist)?;

								if let TimeUnit::Round(current_time) = current_time_unit {
									ensure!(
										current_time >= scheduled_time,
										Error::<T>::LeavingNotDue
									);
								} else {
									Err(Error::<T>::InvalidTimeUnit)?;
								}

								let empty_delegation_set: BTreeMap<MultiLocation, BalanceOf<T>> =
									BTreeMap::new();
								let request_briefs_set: BTreeMap<
									MultiLocation,
									(TimeUnit, BalanceOf<T>),
								> = BTreeMap::new();
								let new_ledger = OneToManyLedger::<BalanceOf<T>> {
									account: old_ledger.clone().account,
									total: Zero::zero(),
									less_total: Zero::zero(),
									delegations: empty_delegation_set,
									requests: vec![],
									request_briefs: request_briefs_set,
									status: OneToManyDelegatorStatus::Active,
								};
								let moonbeam_ledger =
									Ledger::<BalanceOf<T>>::ParachainStaking(new_ledger);

								*old_ledger_opt = Some(moonbeam_ledger);
								// execute request
							},
							ExecuteRequest => {
								let validator_id =
									validator_id_op.ok_or(Error::<T>::ValidatorError)?;

								ensure!(
									old_ledger.status == OneToManyDelegatorStatus::Active,
									Error::<T>::DelegatorLeaving
								);

								// ensure current round is no less than executable time.
								let execute_time_unit =
									unlock_time.ok_or(Error::<T>::InvalidTimeUnit)?;

								let execute_round =
									if let TimeUnit::Round(current_round) = execute_time_unit {
										current_round
									} else {
										Err(Error::<T>::InvalidTimeUnit)?
									};

								let request_time_unit = old_ledger
									.request_briefs
									.get(&validator_id)
									.ok_or(Error::<T>::RequestNotExist)?;

								let request_round =
									if let TimeUnit::Round(req_round) = request_time_unit.0 {
										req_round
									} else {
										Err(Error::<T>::InvalidTimeUnit)?
									};

								ensure!(execute_round >= request_round, Error::<T>::RequestNotDue);

								let (_, execute_amount) = old_ledger
									.request_briefs
									.remove(&validator_id)
									.ok_or(Error::<T>::Unexpected)?;
								old_ledger.total = old_ledger
									.total
									.checked_sub(&execute_amount)
									.ok_or(Error::<T>::UnderFlow)?;

								old_ledger.less_total = old_ledger
									.less_total
									.checked_sub(&execute_amount)
									.ok_or(Error::<T>::UnderFlow)?;

								let request_index = old_ledger
									.requests
									.iter()
									.position(|rqst| rqst.validator == validator_id)
									.ok_or(Error::<T>::RequestNotExist)?;
								old_ledger.requests.remove(request_index);

								let old_delegate_amount = old_ledger
									.delegations
									.get(&validator_id)
									.ok_or(Error::<T>::ValidatorNotBonded)?;
								let new_delegate_amount = old_delegate_amount
									.checked_sub(&execute_amount)
									.ok_or(Error::<T>::UnderFlow)?;

								if new_delegate_amount == Zero::zero() {
									old_ledger
										.delegations
										.remove(&validator_id)
										.ok_or(Error::<T>::Unexpected)?;
								} else {
									old_ledger
										.delegations
										.insert(validator_id, new_delegate_amount);
								}
							},
						}
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)
					}
				},
			)?;

			// Delete the DelegatorLedgerXcmUpdateQueue<T> query
			DelegatorLedgerXcmUpdateQueue::<T>::remove(query_id);

			// Delete the query in pallet_xcm.
			T::SubstrateResponseManager::remove_query_record(query_id);

			Ok(())
		} else {
			Err(Error::<T>::Unexpected)
		}
	}
}

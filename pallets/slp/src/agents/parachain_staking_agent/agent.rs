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

use crate::{
	primitives::{OneToManyDelegationAction, OneToManyLedger, OneToManyScheduledRequest},
	DelegationsOccupied,
};
use codec::{alloc::collections::BTreeMap, Encode};
use core::marker::PhantomData;
use cumulus_primitives_core::relay_chain::HashT;
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Len};
use parachain_staking::ParachainStakingInterface;
// use frame_system::pallet_prelude::BlockNumberFor;
use node_primitives::{CurrencyId, TokenSymbol, VtokenMintingOperator};
use orml_traits::MultiCurrency;
use sp_core::U256;
use sp_runtime::{
	traits::{CheckedAdd, CheckedSub, Convert, UniqueSaturatedFrom, UniqueSaturatedInto, Zero},
	DispatchResult,
};
use sp_std::prelude::*;
use xcm::opaque::latest::MultiLocation;

use crate::{
	pallet::Error,
	primitives::{Ledger, OneToManyDelegatorStatus, ValidatorsByDelegatorUpdateEntry, BNC},
	traits::StakingAgent,
	AccountIdOf, BalanceOf, Config, CurrencyDelays, DelegatorLedgers, DelegatorNextIndex,
	DelegatorsIndex2Multilocation, DelegatorsMultilocation2Index, Hash, LedgerUpdateEntry,
	MinimumsAndMaximums, Pallet, QueryId, TimeUnit, Validators,
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
		MultiLocation,
		MultiLocation,
		BalanceOf<T>,
		TimeUnit,
		AccountIdOf<T>,
		MultiLocation,
		QueryId,
		LedgerUpdateEntry<BalanceOf<T>, MultiLocation, MultiLocation>,
		ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation, Hash<T>>,
		Error<T>,
	> for ParachainStakingAgent<T>
{
	fn initialize_delegator(&self, currency_id: CurrencyId) -> Result<MultiLocation, Error<T>> {
		let new_delegator_id = DelegatorNextIndex::<T>::get(currency_id);
		DelegatorNextIndex::<T>::mutate(currency_id, |id| -> Result<(), Error<T>> {
			let option_new_id = id.checked_add(1).ok_or(Error::<T>::OverFlow)?;
			*id = option_new_id;
			Ok(())
		})?;

		// Generate multi-location by id.
		let delegator_multilocation = T::AccountConverter::convert((new_delegator_id, currency_id));

		// Add the new delegator into storage
		Self::add_delegator(&self, new_delegator_id, &delegator_multilocation, currency_id)
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
		let collator = validator.clone().ok_or(Error::<T>::ValidatorNotProvided)?;
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		// Ensure amount is no less than delegation_amount_minimum.
		ensure!(amount >= mins_maxs.delegation_amount_minimum.into(), Error::<T>::LowerThanMinimum);

		// check if the validator is in the white list.
		let multi_hash = T::Hashing::hash(&collator.encode());
		let validator_list =
			Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;
		validator_list
			.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash)
			.map_err(|_| Error::<T>::ValidatorSetNotExist)?;

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
			let new_ledger = OneToManyLedger::<MultiLocation, MultiLocation, BalanceOf<T>> {
				account: who.clone(),
				total: Zero::zero(),
				less_total: Zero::zero(),
				delegations: empty_delegation_set,
				requests: vec![],
				request_briefs: request_briefs_set,
				status: OneToManyDelegatorStatus::Active,
			};
			let parachain_staking_ledger =
				Ledger::<MultiLocation, BalanceOf<T>, MultiLocation>::ParachainStaking(new_ledger);

			DelegatorLedgers::<T>::insert(currency_id, who, parachain_staking_ledger);
		}

		// Get the delegator account id in Moonriver/Moonbeam network
		let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
		let validator_account_id = Pallet::<T>::multilocation_to_account(validator_multilocation)?;
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
						if let Some(amt) = amount_rs { amt.clone() } else { Zero::zero() };

					let new_amount =
						original_amount.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;
					old_ledger.delegations.insert((*validator_multilocation).clone(), new_amount);
					Ok(())
				} else {
					Err(Error::<T>::Unexpected)
				}
			},
		)?;

		Self::update_all_occupied_status_storage(currency_id)?;

		Ok(0)
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
		let collator = validator.clone().ok_or(Error::<T>::ValidatorNotProvided)?;

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
		// bond extra amount to the existing delegation.
		let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
		let validator_account_id = Pallet::<T>::multilocation_to_account(validator_multilocation)?;
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
						if let Some(amt) = amount_rs { amt.clone() } else { Zero::zero() };

					let new_amount =
						original_amount.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;
					old_ledger.delegations.insert((*validator_multilocation).clone(), new_amount);
					Ok(())
				} else {
					Err(Error::<T>::Unexpected)
				}
			},
		)?;

		Self::update_all_occupied_status_storage(currency_id)?;

		Ok(0)
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
		let collator = validator.clone().ok_or(Error::<T>::ValidatorNotProvided)?;

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

		let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
		let validator_account_id = Pallet::<T>::multilocation_to_account(validator_multilocation)?;
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

					old_ledger.less_total =
						old_ledger.less_total.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;

					let unlock_time_unit =
						Self::get_unlocking_round_from_current(false, currency_id)?
							.ok_or(Error::<T>::TimeUnitNotExist)?;

					// add a new entry in requests and request_briefs
					let new_request = OneToManyScheduledRequest {
						validator: (*validator_multilocation).clone(),
						when_executable: unlock_time_unit.clone(),
						action: OneToManyDelegationAction::<BalanceOf<T>>::Decrease(amount),
					};
					old_ledger.requests.push(new_request);
					old_ledger
						.request_briefs
						.insert((*validator_multilocation).clone(), (unlock_time_unit, amount));
					Ok(())
				} else {
					Err(Error::<T>::Unexpected)
				}
			},
		)?;

		Self::update_all_occupied_status_storage(currency_id)?;

		Ok(0)
	}

	/// Unbonding all amount of a delegator. Equivalent to leave delegator set. The same as Chill
	/// function.
	fn unbond_all(
		&self,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
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
					let unlock_time = Self::get_unlocking_round_from_current(false, currency_id)?
						.ok_or(Error::<T>::TimeUnitNotExist)?;
					old_ledger.status = OneToManyDelegatorStatus::Leaving(unlock_time.clone());

					let mut new_requests = vec![];
					let new_request_briefs: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<T>)> =
						BTreeMap::new();
					for (vali, amt) in old_ledger.delegations.iter() {
						let request_entry = OneToManyScheduledRequest {
							validator: vali.clone(),
							when_executable: unlock_time.clone(),
							action: OneToManyDelegationAction::<BalanceOf<T>>::Revoke(amt.clone()),
						};
						new_requests.push(request_entry);

						old_ledger
							.request_briefs
							.insert(vali.clone(), (unlock_time.clone(), amt.clone()));
					}

					old_ledger.requests = new_requests;
					old_ledger.request_briefs = new_request_briefs;
					Ok(())
				} else {
					Err(Error::<T>::Unexpected)
				}
			},
		)?;

		Self::update_all_occupied_status_storage(currency_id)?;

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
		let collator = validator.clone().ok_or(Error::<T>::ValidatorNotProvided)?;

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

		let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
		let validator_account_id = Pallet::<T>::multilocation_to_account(validator_multilocation)?;
		let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;

		T::ParachainStaking::cancel_delegation_request(delegator_account_id, validator_account_id)
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
						.binary_search_by_key(validator_multilocation, |request| {
							request.validator.clone()
						})
						.map_err(|_| Error::<T>::Unexpected)?;
					old_ledger.requests.remove(request_index);

					old_ledger.request_briefs.remove(validator_multilocation);
					Ok(())
				} else {
					Err(Error::<T>::Unexpected)
				}
			},
		)?;

		Self::update_all_occupied_status_storage(currency_id)?;

		Ok(0)
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
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
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
			let unbond_amount = ledger.delegations.get(&validator).ok_or(Error::<T>::OverFlow)?;

			// Check after undelegating all these validators, if the delegator still meets the
			// requirement.
			let active =
				ledger.total.checked_sub(&ledger.less_total).ok_or(Error::<T>::UnderFlow)?;
			let unbond_after_amount =
				active.checked_sub(&unbond_amount).ok_or(Error::<T>::UnderFlow)?;
			ensure!(
				unbond_after_amount >= mins_maxs.delegator_bonded_minimum,
				Error::<T>::LowerThanMinimum
			);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		let validator_account_id = Pallet::<T>::multilocation_to_account(validator)?;
		let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;

		T::ParachainStaking::schedule_revoke_delegation(delegator_account_id, validator_account_id)
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
						Self::get_unlocking_round_from_current(false, currency_id)?
							.ok_or(Error::<T>::TimeUnitNotExist)?;

					// add a new entry in requests and request_briefs
					let new_request = OneToManyScheduledRequest {
						validator: (*validator).clone(),
						when_executable: unlock_time_unit.clone(),
						action: OneToManyDelegationAction::<BalanceOf<T>>::Revoke(
							revoke_amount.clone(),
						),
					};
					old_ledger.requests.push(new_request);
					old_ledger
						.request_briefs
						.insert((*validator).clone(), (unlock_time_unit, revoke_amount.clone()));
					Ok(())
				} else {
					Err(Error::<T>::Unexpected)
				}
			},
		)?;

		Self::update_all_occupied_status_storage(currency_id)?;

		Ok(0)
	}

	/// Cancel leave delegator set.
	fn redelegate(
		&self,
		who: &MultiLocation,
		_targets: &Option<Vec<MultiLocation>>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
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

		Self::update_all_occupied_status_storage(currency_id)?;

		Ok(0)
	}

	/// Initiate payout for a certain delegator.
	fn payout(
		&self,
		_who: &MultiLocation,
		_validator: &MultiLocation,
		_when: &Option<TimeUnit>,
		_currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(
		&self,
		who: &MultiLocation,
		_when: &Option<TimeUnit>,
		validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		// Check if it is in the delegator set.
		let collator = validator.clone().ok_or(Error::<T>::ValidatorNotProvided)?;
		let mut leaving = false;
		let now = T::VtokenMinting::get_ongoing_time_unit(currency_id)
			.ok_or(Error::<T>::TimeUnitNotExist)?;
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;

		let ledger_option = DelegatorLedgers::<T>::get(currency_id, who);
		// let mut due_amount = Zero::zero();
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
				// due_amount = request_info.1;
				ensure!(now >= due_time.clone(), Error::<T>::RequestNotDue);
			}
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
		let validator_account_id = Pallet::<T>::multilocation_to_account(validator_multilocation)?;
		let delegator_account_id = Pallet::<T>::multilocation_to_account(who)?;

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
							Self::get_unlocking_round_from_current(false, currency_id)?
								.ok_or(Error::<T>::TimeUnitNotExist)?;

						if let TimeUnit::Round(current_time) = current_time_unit {
							ensure!(current_time >= scheduled_time, Error::<T>::LeavingNotDue);
						} else {
							Err(Error::<T>::InvalidTimeUnit)?;
						}

						let empty_delegation_set: BTreeMap<MultiLocation, BalanceOf<T>> =
							BTreeMap::new();
						let request_briefs_set: BTreeMap<MultiLocation, (TimeUnit, BalanceOf<T>)> =
							BTreeMap::new();
						let new_ledger =
							OneToManyLedger::<MultiLocation, MultiLocation, BalanceOf<T>> {
								account: old_ledger.clone().account,
								total: Zero::zero(),
								less_total: Zero::zero(),
								delegations: empty_delegation_set,
								requests: vec![],
								request_briefs: request_briefs_set,
								status: OneToManyDelegatorStatus::Active,
							};
						let parachain_staking_ledger =
							Ledger::<MultiLocation, BalanceOf<T>, MultiLocation>::ParachainStaking(
								new_ledger,
							);

						*old_ledger_opt = Some(parachain_staking_ledger);
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)
					}
				},
			)?;

			Self::update_all_occupied_status_storage(currency_id)?;
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
							Self::get_unlocking_round_from_current(false, currency_id)?
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

						let request_round = if let TimeUnit::Round(req_round) = request_time_unit.0
						{
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
							.binary_search_by_key(validator_multilocation, |rqst| {
								rqst.validator.clone()
							})
							.map_err(|_| Error::<T>::RequestNotExist)?;
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
								.insert((*validator_multilocation).clone(), new_delegate_amount);
						}
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)
					}
				},
			)?;

			Self::update_all_occupied_status_storage(currency_id)?;
		}

		Ok(0)
	}

	/// The same as unbondAll, leaving delegator set.
	fn chill(&self, who: &MultiLocation, currency_id: CurrencyId) -> Result<QueryId, Error<T>> {
		Self::unbond_all(&self, who, currency_id)
	}

	/// Make token transferred back to Bifrost chain account.
	fn transfer_back(
		&self,
		_from: &MultiLocation,
		_to: &MultiLocation,
		_amount: BalanceOf<T>,
		_currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Make token from Bifrost chain account to the staking chain account.
	/// Receiving account must be one of the KSM delegators.
	fn transfer_to(
		&self,
		_from: &MultiLocation,
		_to: &MultiLocation,
		_amount: BalanceOf<T>,
		_currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
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

	/// Add a new serving delegator for a particular currency.
	fn add_delegator(
		&self,
		index: u16,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> DispatchResult {
		// Check if the delegator already exists. If yes, return error.
		ensure!(
			!DelegatorsIndex2Multilocation::<T>::contains_key(currency_id, index),
			Error::<T>::AlreadyExist
		);

		// Ensure delegators count is not greater than maximum.
		let delegators_count = DelegatorNextIndex::<T>::get(currency_id);
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(delegators_count < mins_maxs.delegators_maximum, Error::<T>::GreaterThanMaximum);

		// Revise two delegator storages.
		DelegatorsIndex2Multilocation::<T>::insert(currency_id, index, who);
		DelegatorsMultilocation2Index::<T>::insert(currency_id, who, index);

		// create ledger.

		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		// Check if the delegator exists.
		let index = DelegatorsMultilocation2Index::<T>::get(currency_id, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Get the delegator ledger
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		let total = if let Ledger::ParachainStaking(parachain_staking_ledger) = ledger {
			parachain_staking_ledger.total
		} else {
			Err(Error::<T>::Unexpected)?
		};

		// Check if ledger total amount is zero. If not, return error.
		ensure!(total.is_zero(), Error::<T>::AmountNotZero);

		// Remove corresponding storage.
		DelegatorsIndex2Multilocation::<T>::remove(currency_id, index);
		DelegatorsMultilocation2Index::<T>::remove(currency_id, who);
		DelegatorLedgers::<T>::remove(currency_id, who);

		Ok(())
	}

	/// Add a new serving delegator for a particular currency.
	fn add_validator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		let multi_hash = T::Hashing::hash(&who.encode());
		// Check if the validator already exists.
		let validators_set = Validators::<T>::get(currency_id);

		// Ensure validator candidates in the whitelist is not greater than maximum.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(
			validators_set.len() as u16 <= mins_maxs.validators_maximum,
			Error::<T>::GreaterThanMaximum
		);

		if validators_set.is_none() {
			Validators::<T>::insert(currency_id, vec![(who, multi_hash)]);
		} else {
			// Change corresponding storage.
			Validators::<T>::mutate(currency_id, |validator_vec| -> Result<(), Error<T>> {
				if let Some(ref mut validator_list) = validator_vec {
					let rs =
						validator_list.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash);

					if let Err(index) = rs {
						validator_list.insert(index, (who.clone(), multi_hash));
					} else {
						Err(Error::<T>::AlreadyExist)?
					}
				}
				Ok(())
			})?;
		}

		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_validator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		// Check if the validator already exists.
		let validators_set =
			Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;

		let multi_hash = T::Hashing::hash(&who.encode());
		ensure!(validators_set.contains(&(who.clone(), multi_hash)), Error::<T>::ValidatorNotExist);

		// Check all the delegators' delegations, to see whether this specific validator is in use.
		for (_, ledger) in DelegatorLedgers::<T>::iter_prefix(currency_id) {
			if let Ledger::ParachainStaking(parachain_staking_ledger) = ledger {
				ensure!(
					!parachain_staking_ledger.delegations.contains_key(who),
					Error::<T>::ValidatorStillInUse
				);
			} else {
				Err(Error::<T>::ProblematicLedger)?;
			}
		}

		// Update corresponding storage.
		Validators::<T>::mutate(currency_id, |validator_vec| {
			if let Some(ref mut validator_list) = validator_vec {
				let rs = validator_list.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash);

				if let Ok(index) = rs {
					validator_list.remove(index);
				}
			}
		});

		Ok(())
	}

	/// Charge hosting fee.
	fn charge_hosting_fee(
		&self,
		amount: BalanceOf<T>,
		_from: &MultiLocation,
		to: &MultiLocation,
		currency_id: CurrencyId,
	) -> DispatchResult {
		ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

		// Get current VKSM/KSM exchange rate.
		let vtoken = match currency_id {
			BNC => Ok(CurrencyId::VToken(TokenSymbol::BNC)),
			_ => Err(Error::<T>::NotSupportedCurrencyId),
		}?;

		let vtoken_issuance = T::MultiCurrency::total_issuance(vtoken);
		let token_pool = T::VtokenMinting::get_token_pool(currency_id);
		// Calculate how much vksm the beneficiary account can get.
		let amount: u128 = amount.unique_saturated_into();
		let vtoken_issuance: u128 = vtoken_issuance.unique_saturated_into();
		let token_pool: u128 = token_pool.unique_saturated_into();
		let can_get_vtoken = U256::from(amount)
			.checked_mul(U256::from(vtoken_issuance))
			.and_then(|n| n.checked_div(U256::from(token_pool)))
			.and_then(|n| TryInto::<u128>::try_into(n).ok())
			.unwrap_or_else(Zero::zero);

		let beneficiary = Pallet::<T>::multilocation_to_account(&to)?;
		// Issue corresponding vksm to beneficiary account.
		T::MultiCurrency::deposit(
			vtoken,
			&beneficiary,
			BalanceOf::<T>::unique_saturated_from(can_get_vtoken),
		)?;
		Ok(())
	}

	/// Deposit some amount as fee to nominator accounts.
	fn supplement_fee_reserve(
		&self,
		_amount: BalanceOf<T>,
		_from: &MultiLocation,
		_to: &MultiLocation,
		_currency_id: CurrencyId,
	) -> DispatchResult {
		Ok(())
	}

	fn check_delegator_ledger_query_response(
		&self,
		_query_id: QueryId,
		_entry: LedgerUpdateEntry<BalanceOf<T>, MultiLocation, MultiLocation>,
		_manual_mode: bool,
		_currency_id: CurrencyId,
	) -> Result<bool, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn check_validators_by_delegator_query_response(
		&self,
		_query_id: QueryId,
		_entry: ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation, Hash<T>>,
		_manual_mode: bool,
	) -> Result<bool, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn fail_delegator_ledger_query_response(&self, _query_id: QueryId) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn fail_validators_by_delegator_query_response(
		&self,
		_query_id: QueryId,
	) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
	}
}

/// Internal functions.
impl<T: Config> ParachainStakingAgent<T> {
	fn get_unlocking_round_from_current(
		if_leave: bool,
		currency_id: CurrencyId,
	) -> Result<Option<TimeUnit>, Error<T>> {
		let current_time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
			.ok_or(Error::<T>::TimeUnitNotExist)?;
		let delays = CurrencyDelays::<T>::get(currency_id).ok_or(Error::<T>::DelaysNotExist)?;

		let unlock_round = if let TimeUnit::Round(current_round) = current_time_unit {
			let mut delay = delays.unlock_delay;
			if if_leave {
				delay = delays.leave_delegators_delay;
			}

			if let TimeUnit::Round(delay_round) = delay {
				current_round.checked_add(delay_round).ok_or(Error::<T>::OverFlow)
			} else {
				Err(Error::<T>::InvalidDelays)
			}
		} else {
			Err(Error::<T>::InvalidTimeUnit)
		}?;

		let unlock_time_unit = TimeUnit::Round(unlock_round);
		Ok(Some(unlock_time_unit))
	}

	fn update_all_occupied_status_storage(currency_id: CurrencyId) -> Result<(), Error<T>> {
		let mut all_occupied = true;

		for (_, ledger) in DelegatorLedgers::<T>::iter_prefix(currency_id) {
			if let Ledger::ParachainStaking(parachain_staking_ledger) = ledger {
				if parachain_staking_ledger.delegations.len() >
					parachain_staking_ledger.request_briefs.len()
				{
					all_occupied = false;
					break;
				}
			} else {
				Err(Error::<T>::Unexpected)?;
			}
		}
		let original_status = DelegationsOccupied::<T>::get(currency_id);

		match original_status {
			Some(status) if status == all_occupied => (),
			_ => DelegationsOccupied::<T>::insert(currency_id, all_occupied),
		};

		Ok(())
	}
}

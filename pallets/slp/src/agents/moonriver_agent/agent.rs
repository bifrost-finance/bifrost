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

use core::marker::PhantomData;
use xcm_interface::traits::parachains;

use super::types::{
	MoonriverBalancesCall, MoonriverCall, MoonriverCurrencyId, MoonriverParachainStakingCall,
	MoonriverUtilityCall, MoonriverXtokensCall,
};
use crate::primitives::{
	OneToManyDelegationAction::{Decrease, Revoke},
	OneToManyLedger, OneToManyScheduledRequest,
};
use codec::{alloc::collections::BTreeMap, Decode, Encode};
use cumulus_primitives_core::relay_chain::HashT;
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get, weights::Weight};
use frame_system::pallet_prelude::BlockNumberFor;
use node_primitives::{CurrencyId, TokenSymbol, VtokenMintingOperator};
use orml_traits::MultiCurrency;
use sp_core::{H160, U256};
use sp_io::hashing::blake2_256;
use sp_runtime::{
	traits::{
		CheckedAdd, CheckedSub, Convert, Saturating, StaticLookup, TrailingZeroInput,
		UniqueSaturatedFrom, UniqueSaturatedInto, Zero,
	},
	DispatchResult,
};
use sp_std::prelude::*;
use xcm::{
	latest::prelude::*,
	opaque::latest::{
		Junction::{AccountId32, Parachain},
		Junctions::X1,
		MultiLocation,
	},
	VersionedMultiAssets, VersionedMultiLocation,
};

use crate::{
	agents::SystemCall,
	pallet::{Error, Event},
	primitives::{
		Ledger, MoonriverLedgerUpdateEntry, OneToManyDelegatorStatus, SubstrateLedger,
		SubstrateValidatorsByDelegatorUpdateEntry, UnlockChunk, ValidatorsByDelegatorUpdateEntry,
		XcmOperation, MOVR,
	},
	traits::{QueryResponseManager, StakingAgent, XcmBuilder},
	AccountIdOf, BalanceOf, Config, CurrencyDelays, DelegatorLedgerXcmUpdateQueue,
	DelegatorLedgers, DelegatorNextIndex, DelegatorsIndex2Multilocation,
	DelegatorsMultilocation2Index, Hash, LedgerUpdateEntry, MinimumsAndMaximums, Pallet, QueryId,
	TimeUnit, Validators, ValidatorsByDelegator, ValidatorsByDelegatorXcmUpdateQueue,
	XcmDestWeightAndFee, TIMEOUT_BLOCKS,
};

/// StakingAgent implementation for Moonriver
pub struct MoonriverAgent<T>(PhantomData<T>);

impl<T> MoonriverAgent<T> {
	pub fn new() -> Self {
		MoonriverAgent(PhantomData::<T>)
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
	> for MoonriverAgent<T>
{
	fn initialize_delegator(&self) -> Result<MultiLocation, Error<T>> {
		let new_delegator_id = DelegatorNextIndex::<T>::get(MOVR);
		DelegatorNextIndex::<T>::mutate(MOVR, |id| -> Result<(), Error<T>> {
			let option_new_id = id.checked_add(1).ok_or(Error::<T>::OverFlow)?;
			*id = option_new_id;
			Ok(())
		})?;

		// Generate multi-location by id.
		let delegator_multilocation = T::AccountConverter::convert((new_delegator_id, MOVR));

		// Add the new delegator into storage
		Self::add_delegator(&self, new_delegator_id, &delegator_multilocation)
			.map_err(|_| Error::<T>::FailToAddDelegator)?;

		Ok(delegator_multilocation)
	}

	/// First bond a new validator for a delegator. In the Moonriver context, corresponding part
	/// is "delegate" function.
	fn bond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		validator: &Option<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		// First check if the delegator exists.
		// If not, check if amount is greater than minimum delegator stake. Afterwards, create the
		// delegator ledger.
		// If yes, check if amount is greater than minimum delegation requirement.
		let collator = validator.clone().ok_or(Error::<T>::ValidatorNotProvided)?;
		let mins_maxs = MinimumsAndMaximums::<T>::get(MOVR).ok_or(Error::<T>::NotExist)?;
		// Ensure amount is no less than delegation_amount_minimum.
		ensure!(amount >= mins_maxs.delegation_amount_minimum.into(), Error::<T>::LowerThanMinimum);

		// check if the validator is in the white list.
		let multi_hash = T::Hashing::hash(&collator.encode());
		let validator_list = Validators::<T>::get(MOVR).ok_or(Error::<T>::ValidatorSetNotExist)?;
		validator_list
			.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash)
			.map_err(|_| Error::<T>::ValidatorSetNotExist)?;

		let ledger_option = DelegatorLedgers::<T>::get(MOVR, who);
		if let Some(Ledger::Moonriver(ledger)) = ledger_option {
			ensure!(amount >= mins_maxs.bond_extra_minimum, Error::<T>::LowerThanMinimum);

			// Ensure the bond after wont exceed delegator_active_staking_maximum
			let add_total = ledger.total.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;
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
			let movr_ledger =
				Ledger::<MultiLocation, BalanceOf<T>, MultiLocation>::Moonriver(new_ledger);

			DelegatorLedgers::<T>::insert(MOVR, who, movr_ledger);
		}

		// prepare xcm call

		// Get the delegator account id in Moonriver network
		let validator_multilocation = validator.as_ref().ok_or(Error::<T>::Unexpected)?;
		let validator_account_id_20 =
			Pallet::<T>::multilocation_to_h160_account(validator_multilocation)?;

		let candidate_delegation_count: u32 = mins_maxs.validators_back_maximum;
		let delegation_count: u32 = mins_maxs.validators_back_maximum;
		// Construct xcm message.
		let call = MoonriverCall::Staking(MoonriverParachainStakingCall::Delegate(
			validator_account_id_20,
			amount,
			candidate_delegation_count,
			delegation_count,
		));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) =
			Self::construct_xcm_as_subaccount_with_query_id(XcmOperation::Bond, call, who)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			Some(&collator),
			true,
			false,
			false,
			false,
			false,
			false,
			false,
			amount,
			query_id,
			timeout,
		)?;

		// Send out the xcm message.
		let dest = Self::get_moonriver_para_multilocation();
		T::XcmRouter::send_xcm(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Bond extra amount for a existing delegation.
	fn bond_extra(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		validator: &Option<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(MOVR).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.bond_extra_minimum, Error::<T>::LowerThanMinimum);

		// check if the delegator exists, if not, return error.
		let collator = validator.clone().ok_or(Error::<T>::ValidatorNotProvided)?;

		let ledger_option = DelegatorLedgers::<T>::get(MOVR, who);
		if let Some(Ledger::Moonriver(ledger)) = ledger_option {
			// check if the delegation exists, if not, return error.
			ensure!(ledger.delegations.contains_key(&collator), Error::<T>::ValidatorNotBonded);
			// Ensure the bond after wont exceed delegator_active_staking_maximum
			let add_total = ledger.total.checked_add(&amount).ok_or(Error::<T>::OverFlow)?;
			ensure!(
				add_total <= mins_maxs.delegator_active_staking_maximum,
				Error::<T>::ExceedActiveMaximum
			);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}
		// bond extra amount to the existing delegation.
		// Construct xcm message..
		let validator_h160_account = Pallet::<T>::multilocation_to_h160_account(&collator)?;
		let call = MoonriverCall::Staking(MoonriverParachainStakingCall::DelegatorBondMore(
			validator_h160_account,
			amount,
		));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) =
			Self::construct_xcm_as_subaccount_with_query_id(XcmOperation::BondExtra, call, who)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			Some(&collator),
			true,
			false,
			false,
			false,
			false,
			false,
			false,
			amount,
			query_id,
			timeout,
		)?;

		// Send out the xcm message.
		let dest = Self::get_moonriver_para_multilocation();
		T::XcmRouter::send_xcm(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		validator: &Option<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(MOVR).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.unbond_minimum, Error::<T>::LowerThanMinimum);

		// check if the delegator exists, if not, return error.
		let collator = validator.clone().ok_or(Error::<T>::ValidatorNotProvided)?;

		let ledger_option = DelegatorLedgers::<T>::get(MOVR, who);
		if let Some(Ledger::Moonriver(ledger)) = ledger_option {
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
			let subtracted_total =
				ledger.total.checked_sub(&amount).ok_or(Error::<T>::UnderFlow)?;
			ensure!(
				subtracted_total >= mins_maxs.delegator_bonded_minimum,
				Error::<T>::LowerThanMinimum
			);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		// Construct xcm message.
		let validator_h160_account = Pallet::<T>::multilocation_to_h160_account(&collator)?;
		let call =
			MoonriverCall::Staking(MoonriverParachainStakingCall::ScheduleDelegatorBondLess(
				validator_h160_account,
				amount,
			));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) =
			Self::construct_xcm_as_subaccount_with_query_id(XcmOperation::Unbond, call, who)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			Some(&collator),
			false,
			true,
			false,
			false,
			false,
			false,
			false,
			amount,
			query_id,
			timeout,
		)?;

		// Send out the xcm message.
		let dest = Self::get_moonriver_para_multilocation();
		T::XcmRouter::send_xcm(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Unbonding all amount of a delegator. Equivalent to leave delegator set. The same as Chill
	/// function.
	fn unbond_all(&self, who: &MultiLocation) -> Result<QueryId, Error<T>> {
		// check if the delegator exists.
		let ledger_option = DelegatorLedgers::<T>::get(MOVR, who);

		if let Some(Ledger::Moonriver(ledger)) = ledger_option {
			// check if the delegator is in the state of leaving.
			ensure!(ledger.status == OneToManyDelegatorStatus::Active, Error::<T>::AlreadyLeaving);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		// Construct xcm message.
		let call = MoonriverCall::Staking(MoonriverParachainStakingCall::ScheduleLeaveDelegators);

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) =
			Self::construct_xcm_as_subaccount_with_query_id(XcmOperation::Chill, call, who)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			None,
			false,
			false,
			false,
			false,
			true,
			false,
			false,
			Zero::zero(),
			query_id,
			timeout,
		)?;

		// Send out the xcm message.
		let dest = Self::get_moonriver_para_multilocation();
		T::XcmRouter::send_xcm(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Cancel pending request
	fn rebond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		validator: &Option<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		let mins_maxs = MinimumsAndMaximums::<T>::get(MOVR).ok_or(Error::<T>::NotExist)?;
		let collator = validator.clone().ok_or(Error::<T>::ValidatorNotProvided)?;

		let ledger_option = DelegatorLedgers::<T>::get(MOVR, who);
		if let Some(Ledger::Moonriver(ledger)) = ledger_option {
			// check if there is pending request
			ensure!(ledger.request_briefs.contains_key(&collator), Error::<T>::RequestNotExist);

			// get pending request amount.
			let when_executable;
			let mut rebond_amount = BalanceOf::<T>::from(0u32);
			// let request_iter = ledger.requests.iter().ok_or(Error::<T>::Unexpected)?;
			for OneToManyScheduledRequest::<MultiLocation, BalanceOf<T>> {
				validator: vali,
				when_executable: when,
				action: act,
			} in ledger.requests.iter()
			{
				if *vali == collator {
					when_executable = when;
					rebond_amount = match act {
						Revoke(revoke_balance) => *revoke_balance,
						Decrease(decrease_balance) => *decrease_balance,
					};

					break;
				}
			}

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

			// ensure the rebond after amount meet the basic delegation requirement.
			let old_delegate_amount =
				ledger.delegations.get(&collator).ok_or(Error::<T>::ValidatorNotBonded)?;
			let new_delegation_amount =
				old_delegate_amount.checked_add(&rebond_amount).ok_or(Error::<T>::OverFlow)?;
			ensure!(
				new_delegation_amount >= mins_maxs.delegation_amount_minimum.into(),
				Error::<T>::LowerThanMinimum
			);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		// Construct xcm message.
		let validator_h160_account = Pallet::<T>::multilocation_to_h160_account(&collator)?;
		let call = MoonriverCall::Staking(MoonriverParachainStakingCall::CancelDelegationRequest(
			validator_h160_account,
		));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) =
			Self::construct_xcm_as_subaccount_with_query_id(XcmOperation::Rebond, call, who)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			Some(&collator),
			false,
			false,
			false,
			true,
			false,
			false,
			false,
			amount,
			query_id,
			timeout,
		)?;

		// Send out the xcm message.
		let dest = Self::get_moonriver_para_multilocation();
		T::XcmRouter::send_xcm(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Delegate to some validators. For Moonriver, it equals function Nominate.
	fn delegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Revoke a delegation relationship. Only deal with the first validator in the vec.
	fn undelegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		let mins_maxs = MinimumsAndMaximums::<T>::get(MOVR).ok_or(Error::<T>::NotExist)?;
		let validator = targets.first().ok_or(Error::<T>::ValidatorNotProvided)?;

		// First, check if the delegator exists.
		let ledger_option = DelegatorLedgers::<T>::get(MOVR, who);
		if let Some(Ledger::Moonriver(ledger)) = ledger_option {
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

		// Do the undelegating work.
		// Construct xcm message.
		let validator_h160_account = Pallet::<T>::multilocation_to_h160_account(&validator)?;
		let call = MoonriverCall::Staking(MoonriverParachainStakingCall::ScheduleRevokeDelegation(
			validator_h160_account,
		));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) =
			Self::construct_xcm_as_subaccount_with_query_id(XcmOperation::Undelegate, call, who)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			Some(validator),
			false,
			false,
			true,
			false,
			false,
			false,
			false,
			Zero::zero(),
			query_id,
			timeout,
		)?;

		// Send out the xcm message.
		let dest = Self::get_moonriver_para_multilocation();
		T::XcmRouter::send_xcm(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Cancel leave delegator set.
	fn redelegate(
		&self,
		who: &MultiLocation,
		_targets: &Vec<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		// first check if the delegator exists.
		let ledger_option = DelegatorLedgers::<T>::get(MOVR, who);
		if let Some(Ledger::Moonriver(ledger)) = ledger_option {
			// check if the delegator is in the state of leaving.
			match ledger.status {
				OneToManyDelegatorStatus::Leaving(_) => Ok(()),
				_ => Err(Error::<T>::DelegatorNotLeaving),
			}?;
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}
		// do the cancellation.
		// Construct xcm message.
		let call = MoonriverCall::Staking(MoonriverParachainStakingCall::CancelLeaveDelegators);

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) =
			Self::construct_xcm_as_subaccount_with_query_id(XcmOperation::CancelLeave, call, who)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			None,
			false,
			false,
			false,
			false,
			false,
			true,
			false,
			Zero::zero(),
			query_id,
			timeout,
		)?;

		// Send out the xcm message.
		let dest = Self::get_moonriver_para_multilocation();
		T::XcmRouter::send_xcm(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Initiate payout for a certain delegator.
	fn payout(
		&self,
		who: &MultiLocation,
		validator: &MultiLocation,
		when: &Option<TimeUnit>,
	) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(
		&self,
		who: &MultiLocation,
		when: &Option<TimeUnit>,
		validator: &Option<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is in the delegator set.
		let collator = validator.clone().ok_or(Error::<T>::ValidatorNotProvided)?;
		let mut leaving = false;
		let now =
			T::VtokenMinting::get_ongoing_time_unit(MOVR).ok_or(Error::<T>::TimeUnitNotExist)?;
		let mins_maxs = MinimumsAndMaximums::<T>::get(MOVR).ok_or(Error::<T>::NotExist)?;

		let ledger_option = DelegatorLedgers::<T>::get(MOVR, who);
		let mut due_amount = Zero::zero();
		if let Some(Ledger::Moonriver(ledger)) = ledger_option {
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
		let delegator_h160_account = Pallet::<T>::multilocation_to_h160_account(who)?;
		let call;
		if leaving {
			call = MoonriverCall::Staking(MoonriverParachainStakingCall::ExecuteLeaveDelegators(
				delegator_h160_account,
				mins_maxs.validators_back_maximum,
			));

			let (query_id, timeout, xcm_message) = Self::construct_xcm_as_subaccount_with_query_id(
				XcmOperation::Liquidize,
				call.clone(),
				who,
			)?;
		} else {
			let validator_h160_account = Pallet::<T>::multilocation_to_h160_account(&collator)?;
			call = MoonriverCall::Staking(MoonriverParachainStakingCall::ExecuteDelegationRequest(
				delegator_h160_account,
				validator_h160_account,
			));

			let (query_id, timeout, xcm_message) = Self::construct_xcm_as_subaccount_with_query_id(
				XcmOperation::Liquidize,
				call.clone(),
				who,
			)?;
		}

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) =
			Self::construct_xcm_as_subaccount_with_query_id(XcmOperation::Liquidize, call, who)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		if leaving {
			Self::insert_delegator_ledger_update_entry(
				who,
				Some(&collator),
				false,
				false,
				false,
				false,
				false,
				false,
				true,
				Zero::zero(),
				query_id,
				timeout,
			)?;
		} else {
			Self::insert_delegator_ledger_update_entry(
				who,
				Some(&collator),
				false,
				false,
				false,
				false,
				false,
				false,
				false,
				due_amount,
				query_id,
				timeout,
			)?;
		}

		// Send out the xcm message.
		let dest = Self::get_moonriver_para_multilocation();
		T::XcmRouter::send_xcm(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// The same as unbondAll, leaving delegator set.
	fn chill(&self, who: &MultiLocation) -> Result<QueryId, Error<T>> {
		Self::unbond_all(&self, who)
	}

	/// Make token transferred back to Bifrost chain account.
	fn transfer_back(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		// Ensure amount is greater than zero.
		ensure!(!amount.is_zero(), Error::<T>::AmountZero);

		// Check if from is one of our delegators. If not, return error.
		DelegatorsMultilocation2Index::<T>::get(MOVR, from).ok_or(Error::<T>::DelegatorNotExist)?;

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
				AccountId32 { network: Any, id: to_32 },
			),
		}));

		let (weight, _) = XcmDestWeightAndFee::<T>::get(MOVR, XcmOperation::XtokensTransferBack)
			.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		// Construct xcm message.
		let call = MoonriverCall::Xtokens(MoonriverXtokensCall::Transfer(
			MoonriverCurrencyId::SelfReserve,
			amount.unique_saturated_into(),
			dest,
			weight,
		));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperation::TransferBack,
			call,
			from,
		)?;

		Ok(())
	}

	/// Make token from Bifrost chain account to the staking chain account.
	/// Receiving account must be one of the KSM delegators.
	fn transfer_to(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		// Make sure receiving account is one of the KSM delegators.
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(MOVR, to),
			Error::<T>::DelegatorNotExist
		);

		// Make sure from account is the entrance account of vtoken-minting module.
		let from_account_id = Pallet::<T>::multilocation_to_account(&from)?;
		let (entrance_account, _) = T::VtokenMinting::get_entrance_and_exit_accounts();
		ensure!(from_account_id == entrance_account, Error::<T>::InvalidAccount);

		Self::do_transfer_to(from, to, amount)?;

		Ok(())
	}

	fn tune_vtoken_exchange_rate(
		&self,
		who: &MultiLocation,
		token_amount: BalanceOf<T>,
		_vtoken_amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		ensure!(!token_amount.is_zero(), Error::<T>::AmountZero);

		// Check whether "who" is an existing delegator.
		ensure!(DelegatorLedgers::<T>::contains_key(MOVR, who), Error::<T>::DelegatorNotBonded);

		// Tune the vtoken exchange rate.
		T::VtokenMinting::increase_token_pool(MOVR, token_amount)
			.map_err(|_| Error::<T>::IncreaseTokenPoolError)?;

		Ok(())
	}

	/// Add a new serving delegator for a particular currency.
	fn add_delegator(&self, index: u16, who: &MultiLocation) -> DispatchResult {
		// Check if the delegator already exists. If yes, return error.
		ensure!(
			!DelegatorsIndex2Multilocation::<T>::contains_key(MOVR, index),
			Error::<T>::AlreadyExist
		);

		// Revise two delegator storages.
		DelegatorsIndex2Multilocation::<T>::insert(MOVR, index, who);
		DelegatorsMultilocation2Index::<T>::insert(MOVR, who, index);

		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(&self, who: &MultiLocation) -> DispatchResult {
		// Check if the delegator exists.
		let index = DelegatorsMultilocation2Index::<T>::get(MOVR, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Get the delegator ledger
		let ledger = DelegatorLedgers::<T>::get(MOVR, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		let total = if let Ledger::Moonriver(moonriver_ledger) = ledger {
			moonriver_ledger.total
		} else {
			Err(Error::<T>::Unexpected)?
		};

		// Check if ledger total amount is zero. If not, return error.
		ensure!(total.is_zero(), Error::<T>::AmountNotZero);

		// Remove corresponding storage.
		DelegatorsIndex2Multilocation::<T>::remove(MOVR, index);
		DelegatorsMultilocation2Index::<T>::remove(MOVR, who);
		DelegatorLedgers::<T>::remove(MOVR, who);

		Ok(())
	}

	/// Add a new serving delegator for a particular currency.
	fn add_validator(&self, who: &MultiLocation) -> DispatchResult {
		unimplemented!()
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_validator(&self, who: &MultiLocation) -> DispatchResult {
		unimplemented!()
	}

	/// Charge hosting fee.
	fn charge_hosting_fee(
		&self,
		amount: BalanceOf<T>,
		_from: &MultiLocation,
		to: &MultiLocation,
	) -> DispatchResult {
		// Get current VKSM/KSM exchange rate.
		let vtoken_issuance =
			T::MultiCurrency::total_issuance(CurrencyId::VToken(TokenSymbol::MOVR));
		let token_pool = T::VtokenMinting::get_token_pool(MOVR);
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
			CurrencyId::VToken(TokenSymbol::MOVR),
			&beneficiary,
			BalanceOf::<T>::unique_saturated_from(can_get_vtoken),
		)?;

		Ok(())
	}

	/// Deposit some amount as fee to nominator accounts.
	fn supplement_fee_reserve(
		&self,
		amount: BalanceOf<T>,
		from: &MultiLocation,
		to: &MultiLocation,
	) -> DispatchResult {
		unimplemented!()
	}

	fn check_delegator_ledger_query_response(
		&self,
		query_id: QueryId,
		entry: LedgerUpdateEntry<BalanceOf<T>, MultiLocation, MultiLocation>,
		manual_mode: bool,
	) -> Result<bool, Error<T>> {
		unimplemented!()
	}

	fn check_validators_by_delegator_query_response(
		&self,
		query_id: QueryId,
		entry: ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation, Hash<T>>,
		manual_mode: bool,
	) -> Result<bool, Error<T>> {
		unimplemented!()
	}

	fn fail_delegator_ledger_query_response(&self, query_id: QueryId) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn fail_validators_by_delegator_query_response(
		&self,
		query_id: QueryId,
	) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
	}
}

/// Internal functions.
impl<T: Config> MoonriverAgent<T> {
	fn get_moonriver_para_multilocation() -> MultiLocation {
		MultiLocation { parents: 1, interior: Junctions::X1(Parachain(parachains::moonriver::ID)) }
	}

	fn get_movr_local_multilocation() -> MultiLocation {
		MultiLocation { parents: 0, interior: X1(PalletInstance(parachains::moonriver::PALLET_ID)) }
	}

	fn get_movr_multilocation() -> MultiLocation {
		MultiLocation { parents: 1, interior: X1(PalletInstance(parachains::moonriver::PALLET_ID)) }
	}

	fn construct_xcm_as_subaccount_with_query_id(
		operation: XcmOperation,
		call: MoonriverCall<T>,
		who: &MultiLocation,
	) -> Result<(QueryId, BlockNumberFor<T>, Xcm<()>), Error<T>> {
		// prepare the query_id for reporting back transact status
		let responder = Self::get_moonriver_para_multilocation();
		let now = frame_system::Pallet::<T>::block_number();
		let timeout = T::BlockNumber::from(TIMEOUT_BLOCKS).saturating_add(now);
		let query_id = T::SubstrateResponseManager::create_query_record(&responder, timeout);

		let (call_as_subaccount, fee, weight) =
			Self::prepare_send_as_subaccount_call_params_with_query_id(
				operation, call, who, query_id,
			)?;

		let xcm_message =
			Self::construct_xcm_message_with_query_id(call_as_subaccount, fee, weight, query_id);

		Ok((query_id, timeout, xcm_message))
	}

	fn construct_xcm_and_send_as_subaccount_without_query_id(
		operation: XcmOperation,
		call: MoonriverCall<T>,
		who: &MultiLocation,
	) -> Result<(), Error<T>> {
		let (call_as_subaccount, fee, weight) =
			Self::prepare_send_as_subaccount_call_params_without_query_id(operation, call, who)?;

		let xcm_message =
			Self::construct_xcm_message_without_query_id(call_as_subaccount, fee, weight);

		let dest = Self::get_moonriver_para_multilocation();
		T::XcmRouter::send_xcm(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(())
	}

	fn prepare_send_as_subaccount_call_params_with_query_id(
		operation: XcmOperation,
		call: MoonriverCall<T>,
		who: &MultiLocation,
		query_id: QueryId,
	) -> Result<(MoonriverCall<T>, BalanceOf<T>, Weight), Error<T>> {
		// Get the delegator sub-account index.
		let sub_account_index = DelegatorsMultilocation2Index::<T>::get(MOVR, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Temporary wrapping remark event in Moonriver for ease use of backend service.
		let remark_call =
			MoonriverCall::System(SystemCall::RemarkWithEvent(Box::new(query_id.encode())));

		let call_batched_with_remark =
			MoonriverCall::Utility(Box::new(MoonriverUtilityCall::BatchAll(Box::new(vec![
				Box::new(call),
				Box::new(remark_call),
			]))));

		let call_as_subaccount =
			MoonriverCall::Utility(Box::new(MoonriverUtilityCall::AsDerivative(
				sub_account_index,
				Box::new(call_batched_with_remark),
			)));

		let (weight, fee) = XcmDestWeightAndFee::<T>::get(MOVR, operation)
			.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		Ok((call_as_subaccount, fee, weight))
	}

	fn prepare_send_as_subaccount_call_params_without_query_id(
		operation: XcmOperation,
		call: MoonriverCall<T>,
		who: &MultiLocation,
	) -> Result<(MoonriverCall<T>, BalanceOf<T>, Weight), Error<T>> {
		// Get the delegator sub-account index.
		let sub_account_index = DelegatorsMultilocation2Index::<T>::get(MOVR, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		let call_as_subaccount = MoonriverCall::Utility(Box::new(
			MoonriverUtilityCall::AsDerivative(sub_account_index, Box::new(call)),
		));

		let (weight, fee) = XcmDestWeightAndFee::<T>::get(MOVR, operation)
			.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		Ok((call_as_subaccount, fee, weight))
	}

	fn insert_delegator_ledger_update_entry(
		who: &MultiLocation,
		validator: Option<&MultiLocation>,
		if_bond: bool,
		if_unlock: bool,
		if_revoke: bool,
		if_cancel: bool,
		if_leave: bool,
		if_cancel_leave: bool,
		if_execute_leave: bool,
		amount: BalanceOf<T>,
		query_id: QueryId,
		timeout: BlockNumberFor<T>,
	) -> Result<(), Error<T>> {
		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.

		// First to see if the delegation relationship exist.
		// If not, create one. If yes,

		let unlock_time = if if_unlock || if_revoke || if_leave || if_execute_leave {
			Self::get_unlocking_round_from_current(if_leave)?
		} else if if_bond || if_cancel || if_cancel_leave {
			None
		//liquidize operation
		} else {
			T::VtokenMinting::get_ongoing_time_unit(MOVR)
		};

		let collator = validator.ok_or(Error::<T>::Unexpected)?;
		let entry = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: who.clone(),
			validator_id: collator.clone(),
			if_bond,
			if_unlock,
			if_revoke,
			if_cancel,
			if_leave,
			amount,
			unlock_time,
		});
		DelegatorLedgerXcmUpdateQueue::<T>::insert(query_id, (entry, timeout));

		Ok(())
	}

	fn get_unlocking_round_from_current(if_leave: bool) -> Result<Option<TimeUnit>, Error<T>> {
		let current_time_unit =
			T::VtokenMinting::get_ongoing_time_unit(MOVR).ok_or(Error::<T>::TimeUnitNotExist)?;
		let delays = CurrencyDelays::<T>::get(MOVR).ok_or(Error::<T>::DelaysNotExist)?;

		let unlock_round = if let TimeUnit::Round(current_round) = current_time_unit {
			let mut delay = delays.unlock_delay;
			if if_leave {
				delay = delays.leave_delegators_delay;
			}

			if let TimeUnit::Round(delay_round) = delay {
				current_round.checked_add(delay_round).ok_or(Error::<T>::OverFlow)
			} else {
				Err(Error::<T>::InvalidTimeUnit)
			}
		} else {
			Err(Error::<T>::InvalidTimeUnit)
		}?;

		let unlock_time_unit = TimeUnit::Round(unlock_round);
		Ok(Some(unlock_time_unit))
	}

	fn do_transfer_to(
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		// Ensure amount is greater than zero.
		ensure!(!amount.is_zero(), Error::<T>::AmountZero);

		// Ensure the from account is located within Bifrost chain. Otherwise, the xcm massage will
		// not succeed.
		ensure!(from.parents.is_zero(), Error::<T>::InvalidTransferSource);

		let (weight, fee_amount) = XcmDestWeightAndFee::<T>::get(MOVR, XcmOperation::TransferTo)
			.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		// Prepare parameter dest and beneficiary.
		let dest = Self::get_moonriver_para_multilocation();
		let beneficiary = Pallet::<T>::multilocation_to_local_multilocation(to)?;

		let movr_location = Self::get_movr_multilocation();
		// Prepare parameter assets.
		let asset = MultiAsset {
			fun: Fungible(amount.unique_saturated_into()),
			id: Concrete(movr_location),
		};
		let assets = MultiAssets::from(asset);

		// Prepare fee asset.
		let movr_local_location = Self::get_movr_local_multilocation();
		let fee_asset = MultiAsset {
			fun: Fungible(fee_amount.unique_saturated_into()),
			id: Concrete(Self::get_movr_local_multilocation()),
		};

		// prepare for xcm message
		let msg = Xcm(vec![
			WithdrawAsset(assets.clone()),
			InitiateReserveWithdraw {
				assets: All.into(),
				reserve: dest.clone(),
				xcm: Xcm(vec![
					BuyExecution { fees: fee_asset, weight_limit: WeightLimit::Limited(weight) },
					DepositAsset { assets: All.into(), max_assets: 1, beneficiary },
				]),
			},
		]);

		// Execute the xcm message.
		T::XcmExecutor::execute_xcm_in_credit(from.clone(), msg, weight, weight)
			.ensure_complete()
			.map_err(|_| Error::<T>::XcmExecutionFailed)?;

		Ok(())
	}
}

/// Trait XcmBuilder implementation for Moonriver
impl<T: Config>
	XcmBuilder<
		BalanceOf<T>,
		MoonriverCall<T>, // , MultiLocation,
	> for MoonriverAgent<T>
{
	fn construct_xcm_message_with_query_id(
		call: MoonriverCall<T>,
		extra_fee: BalanceOf<T>,
		weight: Weight,
		_query_id: QueryId,
	) -> Xcm<()> {
		let asset = MultiAsset {
			id: Concrete(Self::get_movr_local_multilocation()),
			fun: Fungibility::Fungible(extra_fee.unique_saturated_into()),
		};

		Xcm(vec![
			WithdrawAsset(asset.clone().into()),
			BuyExecution { fees: asset, weight_limit: Unlimited },
			Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: weight,
				call: call.encode().into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: All.into(),
				max_assets: u32::max_value(),
				beneficiary: MultiLocation {
					parents: 0,
					interior: X1(Parachain(T::ParachainId::get().into())),
				},
			},
		])
	}

	fn construct_xcm_message_without_query_id(
		call: MoonriverCall<T>,
		extra_fee: BalanceOf<T>,
		weight: Weight,
	) -> Xcm<()> {
		let asset = MultiAsset {
			id: Concrete(Self::get_movr_local_multilocation()),
			fun: Fungibility::Fungible(extra_fee.unique_saturated_into()),
		};

		Xcm(vec![
			WithdrawAsset(asset.clone().into()),
			BuyExecution { fees: asset, weight_limit: Unlimited },
			Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: weight,
				call: call.encode().into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: All.into(),
				max_assets: u32::max_value(),
				beneficiary: MultiLocation {
					parents: 0,
					interior: X1(Parachain(T::ParachainId::get().into())),
				},
			},
		])
	}
}

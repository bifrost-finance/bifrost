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
use super::{RewardDestination, XcmCall};
use crate::{
	agents::{KusamaCall, PolkadotCall, StakingCall},
	pallet::{Error, Event},
	primitives::{
		Ledger, QueryId, SubstrateLedger, SubstrateLedgerUpdateEntry,
		SubstrateLedgerUpdateOperation, SubstrateValidatorsByDelegatorUpdateEntry, UnlockChunk,
		ValidatorsByDelegatorUpdateEntry,
	},
	traits::{QueryResponseManager, StakingAgent},
	AccountIdOf, BalanceOf, BoundedVec, Config, DelegatorLedgerXcmUpdateQueue, DelegatorLedgers,
	DelegatorsMultilocation2Index, LedgerUpdateEntry, MinimumsAndMaximums, Pallet, TimeUnit,
	ValidatorsByDelegator, ValidatorsByDelegatorXcmUpdateQueue,
};
use bifrost_primitives::{
	currency::KSM, CurrencyId, VtokenMintingOperator, XcmDestWeightAndFeeHandler, XcmOperationType,
	DOT,
};
use core::marker::PhantomData;
use frame_support::{ensure, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use parity_scale_codec::Encode;
use sp_runtime::{
	traits::{CheckedAdd, CheckedSub, Convert, StaticLookup, UniqueSaturatedInto, Zero},
	DispatchResult,
};
use sp_std::prelude::*;
use xcm::{opaque::v3::MultiLocation, v3::prelude::*, VersionedAssets, VersionedLocation};

/// StakingAgent implementation for Kusama/Polkadot
pub struct PolkadotAgent<T>(PhantomData<T>);

impl<T> PolkadotAgent<T> {
	pub fn new() -> Self {
		PolkadotAgent(PhantomData::<T>)
	}
}

impl<T: Config>
	StakingAgent<
		BalanceOf<T>,
		AccountIdOf<T>,
		LedgerUpdateEntry<BalanceOf<T>>,
		ValidatorsByDelegatorUpdateEntry,
		Error<T>,
	> for PolkadotAgent<T>
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

	/// First time bonding some amount to a delegator.
	fn bond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		ensure!(!DelegatorLedgers::<T>::contains_key(currency_id, who), Error::<T>::AlreadyBonded);

		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.delegator_bonded_minimum, Error::<T>::LowerThanMinimum);

		// Ensure the bond doesn't exceeds delegator_active_staking_maximum
		ensure!(
			amount <= mins_maxs.delegator_active_staking_maximum,
			Error::<T>::ExceedActiveMaximum
		);

		// Construct xcm message.
		let call = match currency_id {
			KSM => KusamaCall::Staking(StakingCall::<T>::Bond(
				amount,
				RewardDestination::<AccountIdOf<T>>::Staked,
			))
			.encode(),
			DOT => PolkadotCall::Staking(StakingCall::<T>::Bond(
				amount,
				RewardDestination::<AccountIdOf<T>>::Staked,
			))
			.encode(),
			_ => Err(Error::<T>::NotSupportedCurrencyId)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Bond,
				call,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Create a new delegator ledger
		// The real bonded amount will be updated by services once the xcm transaction succeeds.
		let ledger = SubstrateLedger::<BalanceOf<T>> {
			account: *who,
			total: Zero::zero(),
			active: Zero::zero(),
			unlocking: vec![],
		};
		let sub_ledger = Ledger::<BalanceOf<T>>::Substrate(ledger);

		DelegatorLedgers::<T>::insert(currency_id, who, sub_ledger);

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			SubstrateLedgerUpdateOperation::Bond,
			amount,
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Bond extra amount to a delegator.
	fn bond_extra(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.bond_extra_minimum, Error::<T>::LowerThanMinimum);

		// Check if the new_add_amount + active_staking_amount doesn't exceeds
		// delegator_active_staking_maximum
		if let Ledger::Substrate(substrate_ledger) = ledger {
			let active = substrate_ledger.active;

			let total = amount.checked_add(&active).ok_or(Error::<T>::OverFlow)?;
			ensure!(
				total <= mins_maxs.delegator_active_staking_maximum,
				Error::<T>::ExceedActiveMaximum
			);
		} else {
			Err(Error::<T>::Unexpected)?;
		}
		// Construct xcm message..
		let call = match currency_id {
			KSM => KusamaCall::Staking(StakingCall::<T>::BondExtra(amount)).encode(),
			DOT => PolkadotCall::Staking(StakingCall::<T>::BondExtra(amount)).encode(),
			_ => Err(Error::NotSupportedCurrencyId)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::BondExtra,
				call,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			SubstrateLedgerUpdateOperation::Bond,
			amount,
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		if let Ledger::Substrate(substrate_ledger) = ledger {
			let (active_staking, unlocking_num) =
				(substrate_ledger.active, substrate_ledger.unlocking.len() as u32);

			// Check if the unbonding amount exceeds minimum requirement.
			let mins_maxs =
				MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
			ensure!(amount >= mins_maxs.unbond_minimum, Error::<T>::LowerThanMinimum);

			// Check if the remaining active balance is enough for (unbonding amount + minimum
			// bonded amount)
			let remaining =
				active_staking.checked_sub(&amount).ok_or(Error::<T>::NotEnoughToUnbond)?;
			ensure!(remaining >= mins_maxs.delegator_bonded_minimum, Error::<T>::NotEnoughToUnbond);

			// Check if this unbonding will exceed the maximum unlocking records bound for a single
			// delegator.
			ensure!(
				unlocking_num < mins_maxs.unbond_record_maximum,
				Error::<T>::ExceedUnlockingRecords
			);
		} else {
			Err(Error::<T>::Unexpected)?;
		}

		// Construct xcm message.
		let call = match currency_id {
			KSM => KusamaCall::Staking(StakingCall::<T>::Unbond(amount)).encode(),
			DOT => PolkadotCall::Staking(StakingCall::<T>::Unbond(amount)).encode(),
			_ => Err(Error::NotSupportedCurrencyId)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Unbond,
				call,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			SubstrateLedgerUpdateOperation::Unlock,
			amount,
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Unbonding all amount of a delegator. Differentiate from regular unbonding.
	fn unbond_all(
		&self,
		who: &MultiLocation,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Get the active amount of a delegator.
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		if let Ledger::Substrate(substrate_ledger) = ledger {
			let amount = substrate_ledger.active;

			// Construct xcm message.
			let call = match currency_id {
				KSM => KusamaCall::Staking(StakingCall::<T>::Unbond(amount)).encode(),
				DOT => PolkadotCall::Staking(StakingCall::<T>::Unbond(amount)).encode(),
				_ => Err(Error::NotSupportedCurrencyId)?,
			};

			// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
			// send it out.
			let (query_id, timeout, fee, xcm_message) =
				Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
					XcmOperationType::Unbond,
					call,
					who,
					currency_id,
					weight_and_fee,
				)?;

			// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
			// process.
			Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

			// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
			Self::insert_delegator_ledger_update_entry(
				who,
				SubstrateLedgerUpdateOperation::Unlock,
				amount,
				query_id,
				timeout,
				currency_id,
			)?;

			// Send out the xcm message.
			let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
			xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
				.map_err(|_e| Error::<T>::XcmFailure)?;

			Ok(query_id)
		} else {
			Err(Error::<T>::Unexpected)?
		}
	}

	/// Cancel some unbonding amount.
	fn rebond(
		&self,
		who: &MultiLocation,
		amount: Option<BalanceOf<T>>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		let amount = amount.ok_or(Error::<T>::AmountNone)?;
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if the rebonding amount exceeds minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.rebond_minimum, Error::<T>::LowerThanMinimum);

		// Get the delegator ledger
		if let Ledger::Substrate(substrate_ledger) = ledger {
			let unlock_chunk_list = substrate_ledger.unlocking;

			// Check if the delegator unlocking amount is greater than or equal to the rebond
			// amount.
			let mut total_unlocking: BalanceOf<T> = Zero::zero();
			for UnlockChunk { value, unlock_time: _ } in unlock_chunk_list.iter() {
				total_unlocking = total_unlocking.checked_add(value).ok_or(Error::<T>::OverFlow)?;
			}
			ensure!(total_unlocking >= amount, Error::<T>::RebondExceedUnlockingAmount);
		} else {
			Err(Error::<T>::Unexpected)?;
		}

		// Construct xcm message.
		let call = match currency_id {
			KSM => KusamaCall::Staking(StakingCall::<T>::Rebond(amount)).encode(),
			DOT => PolkadotCall::Staking(StakingCall::<T>::Rebond(amount)).encode(),
			_ => Err(Error::NotSupportedCurrencyId)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Rebond,
				call,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			SubstrateLedgerUpdateOperation::Rebond,
			amount,
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Delegate to some validators. For Kusama/Polkadot, it equals function Nominate.
	fn delegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		ensure!(
			DelegatorLedgers::<T>::contains_key(currency_id, who),
			Error::<T>::DelegatorNotBonded
		);

		// Check if targets vec is empty.
		let vec_len = targets.len() as u32;
		ensure!(vec_len > 0, Error::<T>::VectorEmpty);

		// Check if targets exceeds validators_back_maximum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(vec_len <= mins_maxs.validators_back_maximum, Error::<T>::GreaterThanMaximum);

		// remove duplicates
		let dedup_list = Pallet::<T>::remove_validators_duplicates(currency_id, targets)?;

		// Convert vec of multilocations into accounts.
		let mut accounts = vec![];
		for multilocation_account in dedup_list.iter() {
			let account = Pallet::<T>::multilocation_to_account(multilocation_account)?;
			let unlookup_account = T::Lookup::unlookup(account);
			accounts.push(unlookup_account);
		}

		// Construct xcm message.
		let call = match currency_id {
			KSM => KusamaCall::Staking(StakingCall::<T>::Nominate(accounts)).encode(),
			DOT => PolkadotCall::Staking(StakingCall::<T>::Nominate(accounts)).encode(),
			_ => Err(Error::NotSupportedCurrencyId)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Delegate,
				call,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a query record to the ValidatorsByDelegatorXcmUpdateQueue<T> storage.
		Self::insert_validators_by_delegator_update_entry(
			who,
			dedup_list,
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Remove delegation relationship with some validators.
	fn undelegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		ensure!(
			DelegatorLedgers::<T>::contains_key(currency_id, who),
			Error::<T>::DelegatorNotBonded
		);

		// Check if targets vec is empty.
		let vec_len = targets.len() as u32;
		ensure!(vec_len > 0, Error::<T>::VectorEmpty);

		// Get the original delegated validators.
		let original_set = ValidatorsByDelegator::<T>::get(currency_id, who)
			.ok_or(Error::<T>::ValidatorSetNotExist)?;

		// Remove targets from the original set to make a new set.
		let mut new_set: Vec<MultiLocation> = vec![];
		for acc in original_set.iter() {
			if !targets.contains(acc) {
				new_set.push(*acc)
			}
		}

		// Ensure new set is not empty.
		ensure!(new_set.len() > 0, Error::<T>::VectorEmpty);

		// Convert new targets into account vec.
		let mut accounts = vec![];
		for multilocation_account in new_set.iter() {
			let account = Pallet::<T>::multilocation_to_account(multilocation_account)?;
			let unlookup_account = T::Lookup::unlookup(account);
			accounts.push(unlookup_account);
		}

		// Construct xcm message.
		let call = match currency_id {
			KSM => KusamaCall::Staking(StakingCall::<T>::Nominate(accounts)).encode(),
			DOT => PolkadotCall::Staking(StakingCall::<T>::Nominate(accounts)).encode(),
			_ => Err(Error::NotSupportedCurrencyId)?,
		};
		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Delegate,
				call,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a query record to the ValidatorsByDelegatorXcmUpdateQueue<T> storage.
		Self::insert_validators_by_delegator_update_entry(
			who,
			new_set,
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(
		&self,
		who: &MultiLocation,
		targets: &Option<Vec<MultiLocation>>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		let targets = targets.as_ref().ok_or(Error::<T>::ValidatorSetNotExist)?;
		let query_id = Self::delegate(self, who, targets, currency_id, weight_and_fee)?;
		Ok(query_id)
	}

	/// Initiate payout for a certain delegator.
	fn payout(
		&self,
		who: &MultiLocation,
		validator: &MultiLocation,
		when: &Option<TimeUnit>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Get the validator account
		let validator_account = Pallet::<T>::multilocation_to_account(validator)?;

		// Get the payout era
		let payout_era = if let Some(TimeUnit::Era(payout_era)) = *when {
			payout_era
		} else {
			Err(Error::<T>::InvalidTimeUnit)?
		};
		// Construct xcm message.
		let call = match currency_id {
			KSM =>
				KusamaCall::Staking(StakingCall::<T>::PayoutStakers(validator_account, payout_era))
					.encode(),
			DOT => PolkadotCall::Staking(StakingCall::<T>::PayoutStakers(
				validator_account,
				payout_era,
			))
			.encode(),
			_ => Err(Error::NotSupportedCurrencyId)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let fee = Pallet::<T>::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperationType::Payout,
			call,
			who,
			currency_id,
			weight_and_fee,
		)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Both tokenpool increment and delegator ledger update need to be conducted by backend
		// services.

		Ok(Zero::zero())
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(
		&self,
		who: &MultiLocation,
		when: &Option<TimeUnit>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
		_amount: Option<BalanceOf<T>>,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is in the delegator set.
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(currency_id, who),
			Error::<T>::DelegatorNotExist
		);

		// Get the slashing span param.
		let num_slashing_spans = if let Some(TimeUnit::SlashingSpan(num_slashing_spans)) = *when {
			num_slashing_spans
		} else {
			Err(Error::<T>::InvalidTimeUnit)?
		};

		// Construct xcm message.
		let call = match currency_id {
			KSM =>
				KusamaCall::Staking(StakingCall::<T>::WithdrawUnbonded(num_slashing_spans)).encode(),
			DOT => PolkadotCall::Staking(StakingCall::<T>::WithdrawUnbonded(num_slashing_spans))
				.encode(),
			_ => Err(Error::NotSupportedCurrencyId)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Liquidize,
				call,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			SubstrateLedgerUpdateOperation::Liquidize,
			Zero::zero(),
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Chill self. Cancel the identity of delegator in the Relay chain side.
	/// Unbonding all the active amount should be done before or after chill,
	/// so that we can collect back all the bonded amount.
	fn chill(
		&self,
		who: &MultiLocation,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is in the delegator set.
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(currency_id, who),
			Error::<T>::DelegatorNotExist
		);

		// Construct xcm message.
		let call = match currency_id {
			KSM => KusamaCall::Staking(StakingCall::<T>::Chill).encode(),
			DOT => PolkadotCall::Staking(StakingCall::<T>::Chill).encode(),
			_ => Err(Error::NotSupportedCurrencyId)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Chill,
				call,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Get active amount, if not zero, create an update entry.
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		if let Ledger::Substrate(substrate_ledger) = ledger {
			let amount = substrate_ledger.active;

			// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
			Self::insert_delegator_ledger_update_entry(
				who,
				SubstrateLedgerUpdateOperation::Unlock,
				amount,
				query_id,
				timeout,
				currency_id,
			)?;
		} else {
			Err(Error::<T>::Unexpected)?;
		}

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Make token transferred back to Bifrost chain account.
	fn transfer_back(
		&self,
		from: &MultiLocation,
		_to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<(), Error<T>> {
		// Ensure amount is greater than zero.
		ensure!(!amount.is_zero(), Error::<T>::AmountZero);

		// Check if from is one of our delegators. If not, return error.
		DelegatorsMultilocation2Index::<T>::get(currency_id, from)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Make sure the receiving account is the Exit_account from vtoken-minting module.
		let (entrance_account, _) = T::VtokenMinting::get_entrance_and_exit_accounts();

		// Prepare parameter dest and beneficiary.
		let dest = Box::new(VersionedLocation::V3(Location::from([Parachain(
			T::ParachainId::get().into(),
		)])));

		let beneficiary = Box::new(VersionedLocation::V3(Location::from([AccountId32 {
			network: None,
			id: entrance_account.encode().try_into().map_err(|_| Error::<T>::FailToConvert)?,
		}])));

		// Prepare parameter assets.
		let asset = MultiAsset {
			fun: Fungible(amount.unique_saturated_into()),
			id: Concrete(MultiLocation { parents: 0, interior: Here }),
		};
		let assets: Box<VersionedAssets> = Box::new(VersionedAssets::V3(MultiAssets::from(asset)));

		// Prepare parameter fee_asset_item.
		let fee_asset_item: u32 = 0;

		let (weight_limit, _) = T::XcmWeightAndFeeHandler::get_operation_weight_and_fee(
			currency_id,
			XcmOperationType::TransferBack,
		)
		.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		// Construct xcm message.
		let call = match currency_id {
			KSM => KusamaCall::<T>::Xcm(Box::new(XcmCall::LimitedReserveTransferAssets(
				dest,
				beneficiary,
				assets,
				fee_asset_item,
				Limited(weight_limit),
			)))
			.encode(),
			DOT => PolkadotCall::<T>::Xcm(Box::new(XcmCall::LimitedReserveTransferAssets(
				dest,
				beneficiary,
				assets,
				fee_asset_item,
				Limited(weight_limit),
			)))
			.encode(),
			_ => Err(Error::NotSupportedCurrencyId)?,
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let fee = Pallet::<T>::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperationType::TransferBack,
			call,
			from,
			currency_id,
			weight_and_fee,
		)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		Ok(())
	}

	/// Make token from Bifrost chain account to the staking chain account.
	/// Receiving account must be one of the currency_id delegators.
	fn transfer_to(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// Make sure receiving account is one of the currency_id delegators.
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(currency_id, to),
			Error::<T>::DelegatorNotExist
		);

		// Make sure from account is the entrance account of vtoken-minting module.
		let from_account_id = Pallet::<T>::multilocation_to_account(from)?;
		let (entrance_account, _) = T::VtokenMinting::get_entrance_and_exit_accounts();
		ensure!(from_account_id == entrance_account, Error::<T>::InvalidAccount);

		// transfer supplementary fee from treasury to the "from" account. Return the added up
		// amount
		let amount = Pallet::<T>::get_transfer_to_added_amount_and_supplement(
			from_account_id,
			amount,
			currency_id,
		)?;

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
		_weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn tune_vtoken_exchange_rate(
		&self,
		who: &Option<MultiLocation>,
		token_amount: BalanceOf<T>,
		_vtoken_amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		let who = who.as_ref().ok_or(Error::<T>::DelegatorNotExist)?;

		Pallet::<T>::tune_vtoken_exchange_rate_without_update_ledger(
			who,
			token_amount,
			currency_id,
		)?;

		// update delegator ledger
		DelegatorLedgers::<T>::mutate(currency_id, who, |old_ledger| -> Result<(), Error<T>> {
			if let Some(Ledger::Substrate(ref mut old_sub_ledger)) = old_ledger {
				// Increase both the active and total amount.
				old_sub_ledger.active =
					old_sub_ledger.active.checked_add(&token_amount).ok_or(Error::<T>::OverFlow)?;

				old_sub_ledger.total =
					old_sub_ledger.total.checked_add(&token_amount).ok_or(Error::<T>::OverFlow)?;
				Ok(())
			} else {
				Err(Error::<T>::Unexpected)?
			}
		})?;

		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		// Get the delegator ledger
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		if let Ledger::Substrate(substrate_ledger) = ledger {
			let total = substrate_ledger.total;

			// Check if ledger total amount is zero. If not, return error.
			ensure!(total.is_zero(), Error::<T>::AmountNotZero);
		} else {
			Err(Error::<T>::Unexpected)?;
		}

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
		// Get current VKSM/KSM or VDOT/DOT exchange rate.
		let vtoken = currency_id.to_vtoken().map_err(|_| Error::<T>::NotSupportedCurrencyId)?;

		let charge_amount =
			Pallet::<T>::inner_calculate_vtoken_hosting_fee(amount, vtoken, currency_id)?;

		Pallet::<T>::inner_charge_hosting_fee(charge_amount, to, vtoken)
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
			Self::update_ledger_query_response_storage(query_id, entry.clone(), currency_id)?;

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
		query_id: QueryId,
		entry: ValidatorsByDelegatorUpdateEntry,
		manual_mode: bool,
	) -> Result<bool, Error<T>> {
		let should_update = if manual_mode {
			true
		} else {
			T::SubstrateResponseManager::get_query_response_record(query_id)
		};

		// Update corresponding storages.
		if should_update {
			Self::update_validators_by_delegator_query_response_storage(query_id, entry.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsByDelegatorQueryResponseConfirmed {
				query_id,
				entry,
			});
		}

		Ok(should_update)
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
		query_id: QueryId,
	) -> Result<(), Error<T>> {
		// delete pallet_xcm query
		T::SubstrateResponseManager::remove_query_record(query_id);

		// delete update entry
		ValidatorsByDelegatorXcmUpdateQueue::<T>::remove(query_id);

		// Deposit event.
		Pallet::<T>::deposit_event(Event::ValidatorsByDelegatorQueryResponseFailed { query_id });

		Ok(())
	}
}

/// Internal functions.
impl<T: Config> PolkadotAgent<T> {
	fn update_ledger_query_response_storage(
		query_id: QueryId,
		query_entry: LedgerUpdateEntry<BalanceOf<T>>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		use crate::primitives::SubstrateLedgerUpdateOperation::{Bond, Liquidize, Rebond, Unlock};
		// update DelegatorLedgers<T> storage
		if let LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
			currency_id: _,
			delegator_id,
			update_operation,
			amount,
			unlock_time,
		}) = query_entry
		{
			DelegatorLedgers::<T>::mutate(
				currency_id,
				delegator_id,
				|old_ledger| -> Result<(), Error<T>> {
					if let Some(Ledger::Substrate(ref mut old_sub_ledger)) = old_ledger {
						// If this an unlocking xcm message update record
						// Decrease the active amount and add an unlocking record.
						match update_operation {
							Bond => {
								// If this is a bonding operation.
								// Increase both the active and total amount.
								old_sub_ledger.active = old_sub_ledger
									.active
									.checked_add(&amount)
									.ok_or(Error::<T>::OverFlow)?;

								old_sub_ledger.total = old_sub_ledger
									.total
									.checked_add(&amount)
									.ok_or(Error::<T>::OverFlow)?;
							},
							Unlock => {
								old_sub_ledger.active = old_sub_ledger
									.active
									.checked_sub(&amount)
									.ok_or(Error::<T>::UnderFlow)?;

								let unlock_time_unit =
									unlock_time.ok_or(Error::<T>::TimeUnitNotExist)?;

								let new_unlock_record =
									UnlockChunk { value: amount, unlock_time: unlock_time_unit };

								old_sub_ledger.unlocking.push(new_unlock_record);
							},
							Rebond => {
								// If it is a rebonding operation.
								// Reduce the unlocking records.
								let mut remaining_amount = amount;

								#[allow(clippy::while_let_loop)]
								loop {
									if let Some(record) = old_sub_ledger.unlocking.pop() {
										if remaining_amount >= record.value {
											remaining_amount -= record.value;
										} else {
											let remain_unlock_chunk = UnlockChunk {
												value: record.value - remaining_amount,
												unlock_time: record.unlock_time,
											};
											old_sub_ledger.unlocking.push(remain_unlock_chunk);
											break;
										}
									} else {
										break;
									}
								}

								// Increase the active amount.
								old_sub_ledger.active = old_sub_ledger
									.active
									.checked_add(&amount)
									.ok_or(Error::<T>::OverFlow)?;
							},
							Liquidize => {
								// If it is a liquidize operation.
								let unlock_unit = unlock_time.ok_or(Error::<T>::InvalidTimeUnit)?;
								let unlock_era = if let TimeUnit::Era(unlock_era) = unlock_unit {
									unlock_era
								} else {
									Err(Error::<T>::InvalidTimeUnit)?
								};

								let mut accumulated: BalanceOf<T> = Zero::zero();
								let mut pop_first_num = 0;

								// for each unlocking record, check whether its unlocking era is
								// smaller or equal to unlock_time. If yes, pop it out and
								// accumulate its amount.
								for record in old_sub_ledger.unlocking.iter() {
									if let TimeUnit::Era(due_era) = record.unlock_time {
										if due_era <= unlock_era {
											accumulated = accumulated
												.checked_add(&record.value)
												.ok_or(Error::<T>::OverFlow)?;

											pop_first_num = pop_first_num
												.checked_add(&1)
												.ok_or(Error::<T>::OverFlow)?;
										} else {
											break;
										}
									} else {
										Err(Error::<T>::Unexpected)?;
									}
								}

								// Remove the first pop_first_num elements from unlocking records.
								old_sub_ledger.unlocking.drain(0..pop_first_num);

								// Finally deduct the accumulated amount from ledger total field.
								old_sub_ledger.total = old_sub_ledger
									.total
									.checked_sub(&accumulated)
									.ok_or(Error::<T>::OverFlow)?;
							},
						}
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)
					}
				},
			)?;
		} else {
			Err(Error::<T>::Unexpected)?;
		}

		// Delete the DelegatorLedgerXcmUpdateQueue<T> query
		DelegatorLedgerXcmUpdateQueue::<T>::remove(query_id);

		// Delete the query in pallet_xcm.
		T::SubstrateResponseManager::remove_query_record(query_id);

		Ok(())
	}

	/// confirm_validators_by_delegator_query_response successfully
	fn update_validators_by_delegator_query_response_storage(
		query_id: QueryId,
		query_entry: ValidatorsByDelegatorUpdateEntry,
	) -> Result<(), Error<T>> {
		// update ValidatorsByDelegator<T> storage
		let ValidatorsByDelegatorUpdateEntry::Substrate(
			SubstrateValidatorsByDelegatorUpdateEntry { currency_id, delegator_id, validators },
		) = query_entry;

		// ensure the length of validators does not exceed MaxLengthLimit
		ensure!(
			validators.len() <= T::MaxLengthLimit::get() as usize,
			Error::<T>::ExceedMaxLengthLimit
		);

		let bounded_validators =
			BoundedVec::try_from(validators).map_err(|_| Error::<T>::FailToConvert)?;
		ValidatorsByDelegator::<T>::insert(currency_id, delegator_id, bounded_validators);

		// update ValidatorsByDelegatorXcmUpdateQueue<T> storage
		ValidatorsByDelegatorXcmUpdateQueue::<T>::remove(query_id);

		Ok(())
	}

	/// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
	fn insert_delegator_ledger_update_entry(
		who: &MultiLocation,
		update_operation: SubstrateLedgerUpdateOperation,
		amount: BalanceOf<T>,
		query_id: QueryId,
		timeout: BlockNumberFor<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		use crate::primitives::SubstrateLedgerUpdateOperation::{Liquidize, Unlock};
		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		let unlock_time = match &update_operation {
			Unlock => Pallet::<T>::get_unlocking_time_unit_from_current(false, currency_id)?,
			Liquidize => T::VtokenMinting::get_ongoing_time_unit(currency_id),
			_ => None,
		};

		let entry = LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
			currency_id,
			delegator_id: *who,
			update_operation,
			amount,
			unlock_time,
		});
		DelegatorLedgerXcmUpdateQueue::<T>::insert(query_id, (entry, timeout));

		Ok(())
	}

	/// Insert a query record to the ValidatorsByDelegatorXcmUpdateQueue<T> storage.
	fn insert_validators_by_delegator_update_entry(
		who: &MultiLocation,
		validator_list: Vec<MultiLocation>,
		query_id: QueryId,
		timeout: BlockNumberFor<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		let entry = ValidatorsByDelegatorUpdateEntry::Substrate(
			SubstrateValidatorsByDelegatorUpdateEntry {
				currency_id,
				delegator_id: *who,
				validators: validator_list,
			},
		);
		ValidatorsByDelegatorXcmUpdateQueue::<T>::insert(query_id, (entry, timeout));

		Ok(())
	}
}

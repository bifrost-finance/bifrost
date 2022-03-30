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

use codec::Encode;
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get, transactional, weights::Weight};
use frame_system::pallet_prelude::BlockNumberFor;
use node_primitives::{CurrencyId, TokenSymbol, VtokenMintingOperator};
use orml_traits::MultiCurrency;
use sp_core::{H256, U256};
use sp_runtime::{
	traits::{
		CheckedAdd, CheckedSub, Convert, Saturating, StaticLookup, UniqueSaturatedFrom,
		UniqueSaturatedInto, Zero,
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
	agents::{KusamaCall, RewardDestination, StakingCall, UtilityCall, XcmCall},
	pallet::{Error, Event},
	primitives::{
		Ledger, SubstrateLedger, SubstrateLedgerUpdateEntry,
		SubstrateValidatorsByDelegatorUpdateEntry, UnlockChunk, ValidatorsByDelegatorUpdateEntry,
		XcmOperation, KSM,
	},
	traits::{
		DelegatorManager, QueryResponseChecker, QueryResponseManager, StakingAgent,
		StakingFeeManager, XcmBuilder,
	},
	AccountIdOf, BalanceOf, Config, CurrencyDelays, DelegatorLedgerXcmUpdateQueue,
	DelegatorLedgers, DelegatorNextIndex, DelegatorsIndex2Multilocation,
	DelegatorsMultilocation2Index, LedgerUpdateEntry, MinimumsAndMaximums, Pallet, QueryId,
	TimeUnit, ValidatorManager, Validators, ValidatorsByDelegator,
	ValidatorsByDelegatorXcmUpdateQueue, XcmDestWeightAndFee, TIMEOUT_BLOCKS,
};

/// StakingAgent implementation for Kusama
pub struct KusamaAgent<T>(PhantomData<T>);

impl<T> KusamaAgent<T> {
	pub fn new() -> Self {
		KusamaAgent(PhantomData::<T>)
	}
}

impl<T: Config>
	StakingAgent<
		MultiLocation,
		MultiLocation,
		BalanceOf<T>,
		TimeUnit,
		AccountIdOf<T>,
		QueryId,
		Error<T>,
	> for KusamaAgent<T>
{
	fn initialize_delegator(&self) -> Result<MultiLocation, Error<T>> {
		let new_delegator_id = DelegatorNextIndex::<T>::get(KSM);
		DelegatorNextIndex::<T>::mutate(KSM, |id| -> Result<(), Error<T>> {
			let option_new_id = id.checked_add(1).ok_or(Error::<T>::OverFlow)?;
			*id = option_new_id;
			Ok(())
		})?;

		// Generate multi-location by id.
		let delegator_multilocation = T::AccountConverter::convert(new_delegator_id);

		// Add the new delegator into storage
		Self::add_delegator(&self, new_delegator_id, &delegator_multilocation)
			.map_err(|_| Error::<T>::FailToAddDelegator)?;

		Ok(delegator_multilocation)
	}

	/// First time bonding some amount to a delegator.
	fn bond(&self, who: MultiLocation, amount: BalanceOf<T>) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		ensure!(DelegatorLedgers::<T>::get(KSM, who.clone()).is_none(), Error::<T>::AlreadyBonded);

		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.delegator_bonded_minimum, Error::<T>::LowerThanMinimum);

		// Ensure the bond doesn't exceeds delegator_active_staking_maximum
		ensure!(
			amount <= mins_maxs.delegator_active_staking_maximum,
			Error::<T>::ExceedActiveMaximum
		);

		// Get the delegator account id in Kusama network
		let delegator_account = Pallet::<T>::multilocation_to_account(&who)?;

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Bond(
			T::Lookup::unlookup(delegator_account.clone()),
			amount,
			RewardDestination::<AccountIdOf<T>>::Staked,
		));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout) = Self::construct_xcm_and_send_as_subaccount_with_query_id(
			XcmOperation::Bond,
			call,
			who.clone(),
		)?;

		// Create a new delegator ledger
		// The real bonded amount will be updated by services once the xcm transaction succeeds.
		let ledger = SubstrateLedger::<MultiLocation, BalanceOf<T>> {
			account: who.clone(),
			total: Zero::zero(),
			active: Zero::zero(),
			unlocking: vec![],
		};
		let sub_ledger = Ledger::<MultiLocation, BalanceOf<T>>::Substrate(ledger);

		DelegatorLedgers::<T>::insert(KSM, who.clone(), sub_ledger);

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who, true, false, false, amount, query_id, timeout,
		)?;

		Ok(query_id)
	}

	/// Bond extra amount to a delegator.
	fn bond_extra(&self, who: MultiLocation, amount: BalanceOf<T>) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.bond_extra_minimum, Error::<T>::LowerThanMinimum);

		// Check if the new_add_amount + active_staking_amount doesn't exceeds
		// delegator_active_staking_maximum
		let active = if let Ledger::Substrate(substrate_ledger) = ledger {
			substrate_ledger.active
		} else {
			Err(Error::<T>::Unexpected)?
		};

		let total = amount.checked_add(&active).ok_or(Error::<T>::OverFlow)?;
		ensure!(
			total <= mins_maxs.delegator_active_staking_maximum,
			Error::<T>::ExceedActiveMaximum
		);
		// Construct xcm message..
		let call = KusamaCall::Staking(StakingCall::BondExtra(amount));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout) = Self::construct_xcm_and_send_as_subaccount_with_query_id(
			XcmOperation::BondExtra,
			call,
			who.clone(),
		)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who, true, false, false, amount, query_id, timeout,
		)?;

		Ok(query_id)
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(&self, who: MultiLocation, amount: BalanceOf<T>) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		let (active_staking, unlocking_num) = if let Ledger::Substrate(substrate_ledger) = ledger {
			(substrate_ledger.active, substrate_ledger.unlocking.len() as u32)
		} else {
			Err(Error::<T>::Unexpected)?
		};

		// Check if the unbonding amount exceeds minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.unbond_minimum, Error::<T>::LowerThanMinimum);

		// Check if the remaining active balance is enough for (unbonding amount + minimum
		// bonded amount)
		let remaining = active_staking.checked_sub(&amount).ok_or(Error::<T>::NotEnoughToUnbond)?;
		ensure!(remaining >= mins_maxs.delegator_bonded_minimum, Error::<T>::NotEnoughToUnbond);

		// Check if this unbonding will exceed the maximum unlocking records bound for a single
		// delegator.
		ensure!(
			unlocking_num < mins_maxs.unbond_record_maximum,
			Error::<T>::ExceedUnlockingRecords
		);

		// Send unbond xcm message
		let (query_id, timeout) = Self::do_unbond(&who, amount)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who, false, true, false, amount, query_id, timeout,
		)?;

		Ok(query_id)
	}

	/// Unbonding all amount of a delegator. Differentiate from regular unbonding.
	fn unbond_all(&self, who: MultiLocation) -> Result<QueryId, Error<T>> {
		// Get the active amount of a delegator.
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		let amount = if let Ledger::Substrate(substrate_ledger) = ledger {
			substrate_ledger.active
		} else {
			Err(Error::<T>::Unexpected)?
		};

		// Send unbond xcm message
		let (query_id, timeout) = Self::do_unbond(&who, amount)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who, false, true, false, amount, query_id, timeout,
		)?;

		Ok(query_id)
	}

	/// Cancel some unbonding amount.
	fn rebond(&self, who: MultiLocation, amount: BalanceOf<T>) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if the rebonding amount exceeds minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.rebond_minimum, Error::<T>::LowerThanMinimum);

		// Get the delegator ledger
		let unlock_chunk_list = if let Ledger::Substrate(substrate_ledger) = ledger {
			substrate_ledger.unlocking
		} else {
			Err(Error::<T>::Unexpected)?
		};

		// Check if the delegator unlocking amount is greater than or equal to the rebond amount.
		let mut total_unlocking: BalanceOf<T> = Zero::zero();
		for UnlockChunk { value, unlock_time: _ } in unlock_chunk_list.iter() {
			total_unlocking = total_unlocking.checked_add(value).ok_or(Error::<T>::OverFlow)?;
		}
		ensure!(total_unlocking >= amount, Error::<T>::RebondExceedUnlockingAmount);

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Rebond(amount));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout) = Self::construct_xcm_and_send_as_subaccount_with_query_id(
			XcmOperation::Rebond,
			call,
			who.clone(),
		)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who, false, false, true, amount, query_id, timeout,
		)?;

		Ok(query_id)
	}

	/// Delegate to some validators. For Kusama, it equals function Nominate.
	fn delegate(
		&self,
		who: MultiLocation,
		targets: Vec<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		let _ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if targets vec is empty.
		let vec_len = targets.len() as u32;
		ensure!(vec_len > Zero::zero(), Error::<T>::VectorEmpty);

		// Check if targets exceeds validators_back_maximum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(vec_len <= mins_maxs.validators_back_maximum, Error::<T>::GreaterThanMaximum);

		// Sort validators and remove duplicates
		let sorted_dedup_list = Pallet::<T>::sort_validators_and_remove_duplicates(KSM, &targets)?;

		// Convert vec of multilocations into accounts.
		let mut accounts = vec![];
		for (multilocation_account, _hash) in sorted_dedup_list.clone().iter() {
			let account = Pallet::<T>::multilocation_to_account(multilocation_account)?;
			let unlookup_account = T::Lookup::unlookup(account);
			accounts.push(unlookup_account);
		}

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Nominate(accounts));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout) = Self::construct_xcm_and_send_as_subaccount_with_query_id(
			XcmOperation::Delegate,
			call,
			who.clone(),
		)?;

		// Insert a query record to the ValidatorsByDelegatorXcmUpdateQueue<T> storage.
		Self::insert_validators_by_delegator_update_entry(
			who,
			sorted_dedup_list,
			query_id,
			timeout,
		)?;

		Ok(query_id)
	}

	/// Remove delegation relationship with some validators.
	#[transactional]
	fn undelegate(
		&self,
		who: MultiLocation,
		targets: Vec<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		let _ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if targets vec is empty.
		let vec_len = targets.len() as u32;
		ensure!(vec_len > Zero::zero(), Error::<T>::VectorEmpty);

		// Get the original delegated validators.
		let original_set = ValidatorsByDelegator::<T>::get(KSM, who.clone())
			.ok_or(Error::<T>::ValidatorSetNotExist)?;

		// Remove targets from the original set to make a new set.
		let mut new_set: Vec<(MultiLocation, H256)> = vec![];
		for (acc, acc_hash) in original_set.iter() {
			if !targets.contains(acc) {
				new_set.push((acc.clone(), acc_hash.clone()))
			}
		}

		// Ensure new set is not empty.
		ensure!(new_set.len() > Zero::zero(), Error::<T>::VectorEmpty);

		// Convert new targets into account vec.
		let mut accounts = vec![];
		for (multilocation_account, _hash) in new_set.iter() {
			let account = Pallet::<T>::multilocation_to_account(multilocation_account)?;
			let unlookup_account = T::Lookup::unlookup(account);
			accounts.push(unlookup_account);
		}

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Nominate(accounts));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout) = Self::construct_xcm_and_send_as_subaccount_with_query_id(
			XcmOperation::Delegate,
			call,
			who.clone(),
		)?;

		// Insert a query record to the ValidatorsByDelegatorXcmUpdateQueue<T> storage.
		Self::insert_validators_by_delegator_update_entry(who, new_set, query_id, timeout)?;

		Ok(query_id)
	}

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(
		&self,
		who: MultiLocation,
		targets: Vec<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		let query_id = Self::delegate(&self, who, targets)?;
		Ok(query_id)
	}

	/// Initiate payout for a certain delegator.
	fn payout(
		&self,
		who: MultiLocation,
		validator: MultiLocation,
		when: Option<TimeUnit>,
	) -> Result<(), Error<T>> {
		// Get the validator account
		let validator_account = Pallet::<T>::multilocation_to_account(&validator)?;

		// Get the payout era
		let payout_era = if let Some(TimeUnit::Era(payout_era)) = when {
			payout_era
		} else {
			Err(Error::<T>::InvalidTimeUnit)?
		};
		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::PayoutStakers(validator_account, payout_era));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperation::Payout,
			call,
			who,
		)?;

		// Both tokenpool increment and delegator ledger update need to be conducted by backend
		// services.

		Ok(())
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(&self, who: MultiLocation, when: Option<TimeUnit>) -> Result<QueryId, Error<T>> {
		// Check if it is in the delegator set.
		DelegatorsMultilocation2Index::<T>::get(KSM, who.clone())
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Get the slashing span param.
		let num_slashing_spans = if let Some(TimeUnit::SlashingSpan(num_slashing_spans)) = when {
			num_slashing_spans
		} else {
			Err(Error::<T>::InvalidTimeUnit)?
		};

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::WithdrawUnbonded(num_slashing_spans));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout) = Self::construct_xcm_and_send_as_subaccount_with_query_id(
			XcmOperation::Liquidize,
			call,
			who.clone(),
		)?;
		// Delegator ledger update needs to be conducted by backend services.

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			false,
			false,
			false,
			Zero::zero(),
			query_id,
			timeout,
		)?;

		Ok(query_id)
	}

	/// Chill self. Cancel the identity of delegator in the Relay chain side.
	/// Unbonding all the active amount should be done before or after chill,
	/// so that we can collect back all the bonded amount.
	fn chill(&self, who: MultiLocation) -> Result<QueryId, Error<T>> {
		// Check if it is in the delegator set.
		DelegatorsMultilocation2Index::<T>::get(KSM, who.clone())
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Chill);

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout) = Self::construct_xcm_and_send_as_subaccount_with_query_id(
			XcmOperation::Chill,
			call,
			who.clone(),
		)?;

		// Get active amount, if not zero, create an update entry.
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		let amount = if let Ledger::Substrate(substrate_ledger) = ledger {
			substrate_ledger.active
		} else {
			Err(Error::<T>::Unexpected)?
		};

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who, false, true, false, amount, query_id, timeout,
		)?;

		Ok(query_id)
	}

	/// Make token transferred back to Bifrost chain account.
	fn transfer_back(
		&self,
		from: MultiLocation,
		to: AccountIdOf<T>,
		amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		// Ensure amount is greater than zero.
		ensure!(amount >= Zero::zero(), Error::<T>::AmountZero);

		// Check if from is one of our delegators. If not, return error.
		DelegatorsMultilocation2Index::<T>::get(KSM, from.clone())
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Prepare parameter dest and beneficiary.
		let to_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(to)?;

		let dest =
			Box::new(VersionedMultiLocation::from(X1(Parachain(T::ParachainId::get().into()))));
		let beneficiary =
			Box::new(VersionedMultiLocation::from(X1(AccountId32 { network: Any, id: to_32 })));

		// Prepare parameter assets.
		let asset = MultiAsset {
			fun: Fungible(amount.unique_saturated_into()),
			id: Concrete(MultiLocation { parents: 0, interior: Here }),
		};
		let assets: Box<VersionedMultiAssets> =
			Box::new(VersionedMultiAssets::from(MultiAssets::from(asset)));

		// Prepare parameter fee_asset_item.
		let fee_asset_item: u32 = 0;

		// Construct xcm message.
		let call = KusamaCall::Xcm(XcmCall::ReserveTransferAssets(
			dest,
			beneficiary,
			assets,
			fee_asset_item,
		));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperation::TransferBack,
			call,
			from.clone(),
		)?;

		Ok(())
	}

	/// Make token from Bifrost chain account to the staking chain account.
	fn transfer_to(
		&self,
		from: AccountIdOf<T>,
		to: MultiLocation,
		amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		// Ensure amount is greater than zero.
		ensure!(amount >= Zero::zero(), Error::<T>::AmountZero);

		let (weight, fee_amount) = XcmDestWeightAndFee::<T>::get(KSM, XcmOperation::TransferTo)
			.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		// "from" AccountId to MultiLocation
		let from_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(from)?;
		let from_location = Pallet::<T>::account_32_to_local_location(from_32)?;

		// Prepare parameter dest and beneficiary.
		let dest = MultiLocation::parent();
		let to_32: [u8; 32] = Pallet::<T>::multilocation_to_account_32(&to)?;
		let beneficiary = Pallet::<T>::account_32_to_local_location(to_32)?;

		// Prepare parameter assets.
		let asset = MultiAsset {
			fun: Fungible(amount.unique_saturated_into()),
			id: Concrete(MultiLocation::parent()),
		};
		let assets = MultiAssets::from(asset);

		// Prepare fee asset.
		let fee_asset = MultiAsset {
			fun: Fungible(fee_amount.unique_saturated_into()),
			id: Concrete(MultiLocation { parents: 0, interior: Here }),
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

		let now = frame_system::Pallet::<T>::block_number();
		let timeout = T::BlockNumber::from(TIMEOUT_BLOCKS).saturating_add(now);

		//【For xcm v3】
		// let query_id = T::SubstrateResponseManager::create_query_record(dest.clone(), timeout);
		// // Report the Error message of the xcm.
		// // from the responder's point of view to get Here's MultiLocation.
		// let destination = T::UniversalLocation::get()
		// 	.invert_target(&dest)
		// 	.map_err(|()| XcmError::MultiLocationNotInvertible)?;

		// // Set the error reporting.
		// let response_info = QueryResponseInfo { destination, query_id, max_weight: 0 };
		// let report_error = Xcm(vec![ReportError(response_info)]);
		// msg.0.insert(0, SetAppendix(report_error));

		// Execute the xcm message.
		T::XcmExecutor::execute_xcm_in_credit(from_location, msg, weight, weight)
			.ensure_complete()
			.map_err(|_| Error::<T>::XcmExecutionFailed)?;

		Ok(())
	}
}

/// DelegatorManager implementation for Kusama
impl<T: Config> DelegatorManager<MultiLocation, SubstrateLedger<MultiLocation, BalanceOf<T>>>
	for KusamaAgent<T>
{
	/// Add a new serving delegator for a particular currency.
	#[transactional]
	fn add_delegator(&self, index: u16, who: &MultiLocation) -> DispatchResult {
		// Check if the delegator already exists. If yes, return error.
		ensure!(
			DelegatorsIndex2Multilocation::<T>::get(KSM, index).is_none(),
			Error::<T>::AlreadyExist
		);

		// Revise two delegator storages.
		DelegatorsIndex2Multilocation::<T>::insert(KSM, index, who);
		DelegatorsMultilocation2Index::<T>::insert(KSM, who, index);

		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	#[transactional]
	fn remove_delegator(&self, who: &MultiLocation) -> DispatchResult {
		// Check if the delegator exists.
		let index = DelegatorsMultilocation2Index::<T>::get(KSM, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Get the delegator ledger
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		let total = if let Ledger::Substrate(substrate_ledger) = ledger {
			substrate_ledger.total
		} else {
			Err(Error::<T>::Unexpected)?
		};

		// Check if ledger total amount is zero. If not, return error.
		ensure!(total == Zero::zero(), Error::<T>::AmountNotZero);

		// Remove corresponding storage.
		DelegatorsIndex2Multilocation::<T>::remove(KSM, index);
		DelegatorsMultilocation2Index::<T>::remove(KSM, who.clone());
		DelegatorLedgers::<T>::remove(KSM, who);

		Ok(())
	}
}

/// ValidatorManager implementation for Kusama
impl<T: Config> ValidatorManager<MultiLocation> for KusamaAgent<T> {
	/// Add a new serving delegator for a particular currency.
	fn add_validator(&self, who: &MultiLocation) -> DispatchResult {
		let multi_hash = Pallet::<T>::get_hash(&who);
		// Check if the validator already exists.
		let validators_set = Validators::<T>::get(KSM);
		if validators_set.is_none() {
			Validators::<T>::insert(KSM, vec![(who.clone(), multi_hash)]);
		} else {
			// Change corresponding storage.
			Validators::<T>::mutate(KSM, |validator_vec| -> Result<(), Error<T>> {
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
	fn remove_validator(&self, who: &MultiLocation) -> DispatchResult {
		// Check if the validator already exists.
		let validators_set = Validators::<T>::get(KSM).ok_or(Error::<T>::ValidatorSetNotExist)?;

		let multi_hash = Pallet::<T>::get_hash(&who);
		ensure!(validators_set.contains(&(who.clone(), multi_hash)), Error::<T>::ValidatorNotExist);

		//  Check if ValidatorsByDelegator<T> involves this validator. If yes, return error.
		for validator_list in ValidatorsByDelegator::<T>::iter_prefix_values(KSM) {
			if validator_list.contains(&(who.clone(), multi_hash)) {
				Err(Error::<T>::ValidatorStillInUse)?;
			}
		}
		// Update corresponding storage.
		Validators::<T>::mutate(KSM, |validator_vec| {
			if let Some(ref mut validator_list) = validator_vec {
				let rs = validator_list.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash);

				if let Ok(index) = rs {
					validator_list.remove(index);
				}
			}
		});

		Ok(())
	}
}

/// Abstraction over a fee manager for charging fee from the origin chain(Bifrost)
/// or deposit fee reserves for the destination chain nominator accounts.
impl<T: Config> StakingFeeManager<MultiLocation, BalanceOf<T>> for KusamaAgent<T> {
	/// Charge hosting fee.
	fn charge_hosting_fee(
		&self,
		amount: BalanceOf<T>,
		_from: MultiLocation,
		to: MultiLocation,
	) -> DispatchResult {
		// Get current VKSM/KSM exchange rate.
		let vksm_issuance = T::MultiCurrency::total_issuance(KSM);
		let ksm_pool = T::VtokenMinting::get_token_pool(KSM);
		// Calculate how much vksm the beneficiary account can get.
		let amount: u128 = amount.unique_saturated_into();
		let vksm_issuance: u128 = vksm_issuance.unique_saturated_into();
		let ksm_pool: u128 = ksm_pool.unique_saturated_into();
		let can_get_vksm = U256::from(amount)
			.checked_mul(U256::from(vksm_issuance))
			.and_then(|n| n.checked_div(U256::from(ksm_pool)))
			.and_then(|n| TryInto::<u128>::try_into(n).ok())
			.unwrap_or_else(Zero::zero);

		let beneficiary = Pallet::<T>::multilocation_to_account(&to)?;
		// Issue corresponding vksm to beneficiary account.
		T::MultiCurrency::deposit(
			CurrencyId::VToken(TokenSymbol::KSM),
			&beneficiary,
			BalanceOf::<T>::unique_saturated_from(can_get_vksm),
		)?;

		Ok(())
	}

	/// Deposit some amount as fee to nominator accounts.
	fn supplement_fee_reserve(
		&self,
		amount: BalanceOf<T>,
		from: MultiLocation,
		to: MultiLocation,
	) -> DispatchResult {
		// Ensure amount is greater than zero.
		ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

		let source_account = Pallet::<T>::multilocation_to_account(&from)?;
		self.transfer_to(source_account, to, amount)?;

		Ok(())
	}
}

/// Trait XcmBuilder implementation for Kusama
impl<T: Config>
	XcmBuilder<
		BalanceOf<T>,
		KusamaCall<T>, // , MultiLocation,
	> for KusamaAgent<T>
{
	fn construct_xcm_message_with_query_id(
		call: KusamaCall<T>,
		extra_fee: BalanceOf<T>,
		weight: Weight,
		query_id: QueryId,
		// response_back_location: MultiLocation
	) -> Xcm<()> {
		let asset = MultiAsset {
			id: Concrete(MultiLocation::here()),
			fun: Fungibility::Fungible(extra_fee.unique_saturated_into()),
		};

		//【For xcm v3】
		// 	// Add one more field for reporting transact status
		// 	Xcm(vec![
		// 		WithdrawAsset(asset.clone().into()),
		// 		BuyExecution { fees: asset, weight_limit: Unlimited },
		// 		Transact {
		// 			origin_type: OriginKind::SovereignAccount,
		// 			require_weight_at_most: weight,
		// 			call: call.encode().into(),
		// 		},
		// 		ReportTransactStatus(QueryResponseInfo {query_id, response_back_location, max_weight:0}),
		// 		RefundSurplus,
		// 		DepositAsset {
		// 			assets: All.into(),
		// 			max_assets: u32::max_value(),
		// 			beneficiary: MultiLocation {
		// 				parents: 0,
		// 				interior: X1(Parachain(T::ParachainId::get().into())),
		// 			},
		// 		},
		// 	])

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
		// }
	}

	fn construct_xcm_message_without_query_id(
		call: KusamaCall<T>,
		extra_fee: BalanceOf<T>,
		weight: Weight,
	) -> Xcm<()> {
		let asset = MultiAsset {
			id: Concrete(MultiLocation::here()),
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

impl<T: Config>
	QueryResponseChecker<
		QueryId,
		LedgerUpdateEntry<BalanceOf<T>, MultiLocation>,
		ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation>,
		Error<T>,
	> for KusamaAgent<T>
{
	fn check_delegator_ledger_query_response(
		&self,
		query_id: QueryId,
		entry: LedgerUpdateEntry<BalanceOf<T>, MultiLocation>,
		manual_mode: bool,
	) -> Result<bool, Error<T>> {
		// If this is manual mode, it is always updatable.
		let should_update = if manual_mode {
			true
		} else {
			T::SubstrateResponseManager::get_query_response_record(query_id)
		};

		// Update corresponding storages.
		if should_update {
			Self::update_ledger_query_response_storage(query_id, entry.clone())?;

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
		entry: ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation>,
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
		Pallet::<T>::deposit_event(Event::DelegatorLedgerQueryResponseFailSuccessfully {
			query_id,
		});

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
		Pallet::<T>::deposit_event(Event::ValidatorsByDelegatorQueryResponseFailSuccessfully {
			query_id,
		});

		Ok(())
	}
}

/// Internal functions.
impl<T: Config> KusamaAgent<T> {
	fn prepare_send_as_subaccount_call_params(
		operation: XcmOperation,
		call: KusamaCall<T>,
		who: MultiLocation,
	) -> Result<(KusamaCall<T>, BalanceOf<T>, Weight), Error<T>> {
		// Get the delegator sub-account index.
		let sub_account_index = DelegatorsMultilocation2Index::<T>::get(KSM, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		let call_as_subaccount =
			KusamaCall::Utility(Box::new(UtilityCall::AsDerivative(sub_account_index, call)));

		let (weight, fee) = XcmDestWeightAndFee::<T>::get(KSM, operation)
			.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		Ok((call_as_subaccount, fee, weight))
	}

	fn construct_xcm_and_send_as_subaccount_with_query_id(
		operation: XcmOperation,
		call: KusamaCall<T>,
		who: MultiLocation,
	) -> Result<(QueryId, BlockNumberFor<T>), Error<T>> {
		let (call_as_subaccount, fee, weight) =
			Self::prepare_send_as_subaccount_call_params(operation, call, who)?;

		// prepare the query_id for reporting back transact status
		let responder = MultiLocation::parent();
		let now = frame_system::Pallet::<T>::block_number();
		let timeout = T::BlockNumber::from(TIMEOUT_BLOCKS).saturating_add(now);
		let query_id = T::SubstrateResponseManager::create_query_record(responder.clone(), timeout);

		let xcm_message =
			Self::construct_xcm_message_with_query_id(call_as_subaccount, fee, weight, query_id);

		//【For xcm v3】
		// let response_back_location = T::UniversalLocation::get()
		// 	.invert_target(&responder)
		// 	.map_err(|()| XcmError::MultiLocationNotInvertible)?;

		// let xcm_message = Self::construct_xcm_message(
		// 	call_as_subaccount,
		// 	fee,
		// 	weight,
		// 	query_id,
		// 	response_back_location,
		// );
		T::XcmRouter::send_xcm(Parent, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok((query_id, timeout))
	}

	fn construct_xcm_and_send_as_subaccount_without_query_id(
		operation: XcmOperation,
		call: KusamaCall<T>,
		who: MultiLocation,
	) -> Result<(), Error<T>> {
		let (call_as_subaccount, fee, weight) =
			Self::prepare_send_as_subaccount_call_params(operation, call, who)?;

		let xcm_message =
			Self::construct_xcm_message_without_query_id(call_as_subaccount, fee, weight);

		T::XcmRouter::send_xcm(Parent, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(())
	}

	fn do_unbond(
		who: &MultiLocation,
		amount: BalanceOf<T>,
	) -> Result<(QueryId, BlockNumberFor<T>), Error<T>> {
		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Unbond(amount));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout) = Self::construct_xcm_and_send_as_subaccount_with_query_id(
			XcmOperation::Unbond,
			call,
			who.clone(),
		)?;

		Ok((query_id, timeout))
	}

	fn update_ledger_query_response_storage(
		query_id: QueryId,
		query_entry: LedgerUpdateEntry<BalanceOf<T>, MultiLocation>,
	) -> Result<(), Error<T>> {
		// update DelegatorLedgers<T> storage
		if let LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
			currency_id: _,
			delegator_id,
			if_bond,
			if_unlock,
			if_rebond,
			amount,
			unlock_time,
		}) = query_entry
		{
			DelegatorLedgers::<T>::mutate(
				KSM,
				delegator_id.clone(),
				|old_ledger| -> Result<(), Error<T>> {
					if let Some(Ledger::Substrate(mut old_sub_ledger)) = old_ledger.clone() {
						// If this an unlocking xcm message update record
						// Decrease the active amount and add an unlocking record.
						if if_bond {
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
						} else if if_unlock {
							old_sub_ledger.active = old_sub_ledger
								.active
								.checked_sub(&amount)
								.ok_or(Error::<T>::UnderFlow)?;

							let unlock_time_unit =
								unlock_time.ok_or(Error::<T>::TimeUnitNotExist)?;

							let new_unlock_record =
								UnlockChunk { value: amount, unlock_time: unlock_time_unit };

							old_sub_ledger.unlocking.push(new_unlock_record);
						} else if if_rebond {
							// If it is a rebonding operation.
							// Reduce the unlocking records.
							let mut remaining_amount = amount;

							loop {
								let record = old_sub_ledger
									.unlocking
									.pop()
									.ok_or(Error::<T>::UnlockingRecordNotExist)?;

								if remaining_amount >= record.value {
									remaining_amount = remaining_amount - record.value;
								} else {
									let remain_unlock_chunk = UnlockChunk {
										value: record.value - remaining_amount,
										unlock_time: record.unlock_time.clone(),
									};
									old_sub_ledger.unlocking.push(remain_unlock_chunk);
									break;
								}
							}

							// Increase the active amount.
							old_sub_ledger.active = old_sub_ledger
								.active
								.checked_add(&amount)
								.ok_or(Error::<T>::OverFlow)?;
						} else {
							// If it is a liquidize operation.
							let unlock_unit = unlock_time.ok_or(Error::<T>::InvalidTimeUnit)?;
							let unlock_era = if let TimeUnit::Era(unlock_era) = unlock_unit {
								unlock_era
							} else {
								Err(Error::<T>::InvalidTimeUnit)?
							};

							let mut accumulated: BalanceOf<T> = Zero::zero();
							let mut pop_first_num = 0;

							// for each unlocking record, check whether its unlocking era is smaller
							// or equal to unlock_time. If yes, pop it out and accumulate its
							// amount.
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
						}
					}
					Ok(())
				},
			)?;

			// Delete the DelegatorLedgerXcmUpdateQueue<T> query
			DelegatorLedgerXcmUpdateQueue::<T>::remove(query_id);

			// Delete the query in pallet_xcm.
			ensure!(
				T::SubstrateResponseManager::remove_query_record(query_id),
				Error::<T>::QueryResponseRemoveError
			);

			Ok(())
		} else {
			Err(Error::<T>::Unexpected)
		}
	}

	fn update_validators_by_delegator_query_response_storage(
		query_id: QueryId,
		query_entry: ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation>,
	) -> Result<(), Error<T>> {
		// update ValidatorsByDelegator<T> storage
		if let ValidatorsByDelegatorUpdateEntry::Substrate(
			SubstrateValidatorsByDelegatorUpdateEntry { currency_id, delegator_id, validators },
		) = query_entry
		{
			ValidatorsByDelegator::<T>::insert(currency_id, delegator_id, validators);

			// update ValidatorsByDelegatorXcmUpdateQueue<T> storage
			ValidatorsByDelegatorXcmUpdateQueue::<T>::remove(query_id);

			// Delete the query in pallet_xcm.

			ensure!(
				T::SubstrateResponseManager::remove_query_record(query_id),
				Error::<T>::QueryResponseRemoveError
			);

			Ok(())
		} else {
			Err(Error::<T>::Unexpected)
		}
	}

	fn get_unlocking_era_from_current() -> Result<Option<TimeUnit>, Error<T>> {
		let current_time_unit =
			T::VtokenMinting::get_ongoing_time_unit(KSM).ok_or(Error::<T>::TimeUnitNotExist)?;
		let delays = CurrencyDelays::<T>::get(KSM).ok_or(Error::<T>::DelaysNotExist)?;

		let unlock_era = if let TimeUnit::Era(current_era) = current_time_unit {
			if let TimeUnit::Era(delay_era) = delays.unlock_delay {
				current_era.checked_add(delay_era).ok_or(Error::<T>::OverFlow)
			} else {
				Err(Error::<T>::InvalidTimeUnit)
			}
		} else {
			Err(Error::<T>::InvalidTimeUnit)
		}?;

		let unlock_time_unit = TimeUnit::Era(unlock_era);
		Ok(Some(unlock_time_unit))
	}

	fn insert_delegator_ledger_update_entry(
		who: MultiLocation,
		if_bond: bool,
		if_unlock: bool,
		if_rebond: bool,
		amount: BalanceOf<T>,
		query_id: QueryId,
		timeout: BlockNumberFor<T>,
	) -> Result<(), Error<T>> {
		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		let unlock_time = if if_unlock {
			Self::get_unlocking_era_from_current()?
		} else if if_bond || if_rebond {
			None
		} else {
			T::VtokenMinting::get_ongoing_time_unit(KSM)
		};

		let entry = LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
			currency_id: KSM,
			delegator_id: who.clone(),
			if_bond,
			if_unlock,
			if_rebond,
			amount,
			unlock_time,
		});
		DelegatorLedgerXcmUpdateQueue::<T>::insert(query_id, (entry, timeout));

		Ok(())
	}

	fn insert_validators_by_delegator_update_entry(
		who: MultiLocation,
		validator_list: Vec<(MultiLocation, H256)>,
		query_id: QueryId,
		timeout: BlockNumberFor<T>,
	) -> Result<(), Error<T>> {
		// Insert a query record to the ValidatorsByDelegatorXcmUpdateQueue<T> storage.
		let entry = ValidatorsByDelegatorUpdateEntry::Substrate(
			SubstrateValidatorsByDelegatorUpdateEntry {
				currency_id: KSM,
				delegator_id: who,
				validators: validator_list,
			},
		);
		ValidatorsByDelegatorXcmUpdateQueue::<T>::insert(query_id, (entry, timeout));

		Ok(())
	}
}

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
use sp_core::H256;
use sp_runtime::{
	traits::{CheckedAdd, CheckedSub, Convert, StaticLookup, UniqueSaturatedInto, Zero},
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
	AccountIdOf, BalanceOf, Config, DelegatorLedgerXcmUpdateQueue, DelegatorLedgers,
	DelegatorNextIndex, DelegatorsIndex2Multilocation, DelegatorsMultilocation2Index, IfXcmV3Ready,
	LedgerUpdateEntry, MinimumsAndMaximums, Pallet, TimeUnit, ValidatorManager, Validators,
	ValidatorsByDelegator, ValidatorsByDelegatorXcmUpdateQueue, XcmDestWeightAndFee, XcmQueryId,
};

/// StakingAgent implementation for Kusama
pub struct KusamaAgent<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>(
	PhantomData<(T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager)>,
);

impl<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
	KusamaAgent<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
{
	pub fn new() -> Self {
		KusamaAgent(
			PhantomData::<(T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager)>,
		)
	}
}

impl<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
	StakingAgent<MultiLocation, MultiLocation, BalanceOf<T>, TimeUnit, AccountIdOf<T>>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
	SubstrateResponseManager: QueryResponseManager<XcmQueryId, MultiLocation, BlockNumberFor<T>>,
{
	fn initialize_delegator(&self) -> Option<MultiLocation> {
		let new_delegator_id = DelegatorNextIndex::<T>::get(KSM);
		let rs = DelegatorNextIndex::<T>::mutate(KSM, |id| -> DispatchResult {
			let option_new_id = id.checked_add(1).ok_or(Error::<T>::OverFlow)?;
			*id = option_new_id;
			Ok(())
		});

		if rs.is_ok() {
			// Generate multi-location by id.
			let delegator_multilocation = AccountConverter::convert(new_delegator_id);

			// Add the new delegator into storage
			Self::add_delegator(&self, new_delegator_id, &delegator_multilocation).ok()?;

			Some(delegator_multilocation)
		} else {
			None
		}
	}

	/// First time bonding some amount to a delegator.
	fn bond(&self, who: MultiLocation, amount: BalanceOf<T>) -> DispatchResult {
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
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Bond, call, who.clone())?;

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

		Ok(())
	}

	/// Bond extra amount to a delegator.
	fn bond_extra(&self, who: MultiLocation, amount: BalanceOf<T>) -> DispatchResult {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.bond_extra_minimum, Error::<T>::LowerThanMinimum);

		// Check if the new_add_amount + active_staking_amount doesn't exceeds
		// delegator_active_staking_maximum
		let Ledger::Substrate(substrate_ledger) = ledger;

		let total = amount.checked_add(&substrate_ledger.active).ok_or(Error::<T>::OverFlow)?;
		ensure!(
			total <= mins_maxs.delegator_active_staking_maximum,
			Error::<T>::ExceedActiveMaximum
		);
		// Construct xcm message..
		let call = KusamaCall::Staking(StakingCall::BondExtra(amount));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::BondExtra, call, who.clone())?;

		Ok(())
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(&self, who: MultiLocation, amount: BalanceOf<T>) -> DispatchResult {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;
		let Ledger::Substrate(substrate_ledger) = ledger;
		let active_staking = substrate_ledger.active;

		// Check if the unbonding amount exceeds minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.unbond_minimum, Error::<T>::LowerThanMinimum);

		// Check if the remaining active balance is enough for (unbonding amount + minimum
		// bonded amount)
		let remaining = active_staking.checked_sub(&amount).ok_or(Error::<T>::NotEnoughToUnbond)?;
		ensure!(remaining >= mins_maxs.delegator_bonded_minimum, Error::<T>::NotEnoughToUnbond);

		// Check if this unbonding will exceed the maximum unlocking records bound for a single
		// delegator.
		let unlocking_num = substrate_ledger.unlocking.len() as u32;
		ensure!(
			unlocking_num < mins_maxs.unbond_record_maximum,
			Error::<T>::ExceedUnlockingRecords
		);

		// Send unbond xcm message
		Self::do_unbond(&who, amount)?;

		Ok(())
	}

	/// Unbonding all amount of a delegator. Differentiate from regular unbonding.
	fn unbond_all(&self, who: MultiLocation) -> DispatchResult {
		// Get the active amount of a delegator.
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;
		let Ledger::Substrate(substrate_ledger) = ledger;
		let amount = substrate_ledger.active;

		// Send unbond xcm message
		Self::do_unbond(&who, amount)?;

		Ok(())
	}

	/// Cancel some unbonding amount.
	fn rebond(&self, who: MultiLocation, amount: BalanceOf<T>) -> DispatchResult {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if the rebonding amount exceeds minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.rebond_minimum, Error::<T>::LowerThanMinimum);

		// Get the delegator ledger
		let Ledger::Substrate(substrate_ledger) = ledger;

		// Check if the delegator unlocking amount is greater than or equal to the rebond amount.
		let unlock_chunk_list = substrate_ledger.unlocking;
		let mut total_unlocking: BalanceOf<T> = Zero::zero();
		for UnlockChunk { value, unlock_time: _ } in unlock_chunk_list.iter() {
			total_unlocking = total_unlocking.checked_add(value).ok_or(Error::<T>::OverFlow)?;
		}
		ensure!(total_unlocking >= amount, Error::<T>::RebondExceedUnlockingAmount);

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Rebond(amount));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Rebond, call, who.clone())?;

		Ok(())
	}

	/// Delegate to some validators. For Kusama, it equals function Nominate.
	fn delegate(&self, who: MultiLocation, targets: Vec<MultiLocation>) -> DispatchResult {
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
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Delegate, call, who.clone())?;

		// Update ValidatorsByDelegator storage
		ValidatorsByDelegator::<T>::insert(KSM, who.clone(), sorted_dedup_list);

		Ok(())
	}

	/// Remove delegation relationship with some validators.
	#[transactional]
	fn undelegate(&self, who: MultiLocation, targets: Vec<MultiLocation>) -> DispatchResult {
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
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Delegate, call, who.clone())?;

		// Update ValidatorsByDelegator storage
		ValidatorsByDelegator::<T>::insert(KSM, who.clone(), new_set.clone());

		Ok(())
	}

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(&self, who: MultiLocation, targets: Vec<MultiLocation>) -> DispatchResult {
		Self::delegate(&self, who, targets)?;
		Ok(())
	}

	/// Initiate payout for a certain delegator.
	fn payout(
		&self,
		who: MultiLocation,
		validator: MultiLocation,
		when: Option<TimeUnit>,
	) -> DispatchResult {
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
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Payout, call, who)?;
		Ok(())
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(&self, who: MultiLocation, when: Option<TimeUnit>) -> DispatchResult {
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
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Liquidize, call, who)?;
		Ok(())
	}

	/// Chill self. Cancel the identity of delegator in the Relay chain side.
	/// Unbonding all the active amount should be done before or after chill,
	/// so that we can collect back all the bonded amount.
	fn chill(&self, who: MultiLocation) -> DispatchResult {
		// Check if it is in the delegator set.
		DelegatorsMultilocation2Index::<T>::get(KSM, who.clone())
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Chill);

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Chill, call, who)?;

		Ok(())
	}

	/// Make token transferred back to Bifrost chain account.
	fn transfer_back(
		&self,
		from: MultiLocation,
		to: AccountIdOf<T>,
		amount: BalanceOf<T>,
	) -> DispatchResult {
		// Ensure amount is greater than zero.
		ensure!(amount >= Zero::zero(), Error::<T>::AmountZero);

		// Check if from is one of our delegators. If not, return error.
		DelegatorsMultilocation2Index::<T>::get(KSM, from.clone())
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Prepare parameter dest and beneficiary.
		let to_32: [u8; 32] = Pallet::<T>::account_id_to_account_32(to)?;

		let dest = Box::new(VersionedMultiLocation::from(X1(Parachain(ParachainId::get().into()))));
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
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::TransferBack, call, from.clone())?;

		Ok(())
	}

	/// Make token from Bifrost chain account to the staking chain account.
	fn transfer_to(
		&self,
		from: AccountIdOf<T>,
		to: MultiLocation,
		amount: BalanceOf<T>,
	) -> DispatchResult {
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
				reserve: dest,
				xcm: Xcm(vec![
					BuyExecution { fees: fee_asset, weight_limit: WeightLimit::Limited(weight) },
					DepositAsset { assets: All.into(), max_assets: 1, beneficiary },
				]),
			},
		]);

		T::XcmExecutor::execute_xcm_in_credit(from_location, msg, weight, weight)
			.ensure_complete()
			.map_err(|_| Error::<T>::XcmExecutionFailed)?;

		Ok(())
	}
}

/// DelegatorManager implementation for Kusama
impl<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
	DelegatorManager<MultiLocation, SubstrateLedger<MultiLocation, BalanceOf<T>>>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
	SubstrateResponseManager: QueryResponseManager<XcmQueryId, MultiLocation, BlockNumberFor<T>>,
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
		let Ledger::Substrate(substrate_ledger) = ledger;

		// Check if ledger total amount is zero. If not, return error.
		ensure!(substrate_ledger.total == Zero::zero(), Error::<T>::AmountNotZero);

		// Remove corresponding storage.
		DelegatorsIndex2Multilocation::<T>::remove(KSM, index);
		DelegatorsMultilocation2Index::<T>::remove(KSM, who.clone());
		DelegatorLedgers::<T>::remove(KSM, who);

		Ok(())
	}
}

/// ValidatorManager implementation for Kusama
impl<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
	ValidatorManager<MultiLocation>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
	SubstrateResponseManager: QueryResponseManager<XcmQueryId, MultiLocation, BlockNumberFor<T>>,
{
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
impl<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
	StakingFeeManager<MultiLocation, BalanceOf<T>>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
	SubstrateResponseManager: QueryResponseManager<XcmQueryId, MultiLocation, BlockNumberFor<T>>,
{
	/// Charge hosting fee.
	fn charge_hosting_fee(
		&self,
		_amount: BalanceOf<T>,
		_from: MultiLocation,
		_to: MultiLocation,
	) -> DispatchResult {
		// No need to implement this method for Kusama. The hosting fee deduction will be calculated
		// in the backend service.alloc
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
impl<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
	XcmBuilder<BalanceOf<T>, KusamaCall<T>>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
	SubstrateResponseManager: QueryResponseManager<XcmQueryId, MultiLocation, BlockNumberFor<T>>,
{
	fn construct_xcm_message(
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
					interior: X1(Parachain(ParachainId::get().into())),
				},
			},
		])
	}
}

impl<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
	QueryResponseChecker<
		XcmQueryId,
		LedgerUpdateEntry<BalanceOf<T>, MultiLocation>,
		ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation>,
	> for KusamaAgent<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
	SubstrateResponseManager: QueryResponseManager<XcmQueryId, MultiLocation, BlockNumberFor<T>>,
{
	fn check_delegator_ledger_query_response(
		&self,
		query_id: XcmQueryId,
		entry: LedgerUpdateEntry<BalanceOf<T>, MultiLocation>,
	) -> DispatchResult {
		let mut should_update = false;

		// First to confirm whether we got the response from Kusama. This is only for xcm v3. If xcm
		// v3 is not ready, then we will skip this part.
		if IfXcmV3Ready::<T>::get() {
		} else {
			should_update = true;
		}

		// Update corresponding storages.
		if should_update {
			Self::update_ledger_query_response_storage(query_id, entry.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorLedgerQueryResponseConfirmed {
				query_id,
				entry,
			});
		}

		Ok(())
	}
	fn check_validators_by_delegator_query_response(
		&self,
		query_id: XcmQueryId,
		entry: ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation>,
	) -> DispatchResult {
		let mut should_update = false;

		// First to confirm whether we got the response from Kusama. This is only for xcm v3. If xcm
		// v3 is not ready, then we will skip this part.
		if IfXcmV3Ready::<T>::get() {
		} else {
			should_update = true;
		}

		// Update corresponding storages.
		if should_update {
			Self::update_validators_by_delegator_query_response_storage(query_id, entry.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsByDelegatorQueryResponseConfirmed {
				query_id,
				entry,
			});
		}

		Ok(())
	}
}

/// Internal functions.
impl<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
	KusamaAgent<T, AccountConverter, ParachainId, XcmSender, SubstrateResponseManager>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
	SubstrateResponseManager: QueryResponseManager<XcmQueryId, MultiLocation, BlockNumberFor<T>>,
{
	fn construct_xcm_and_send_as_subaccount(
		operation: XcmOperation,
		call: KusamaCall<T>,
		who: MultiLocation,
	) -> DispatchResult {
		// Get the delegator sub-account index.
		let sub_account_index = DelegatorsMultilocation2Index::<T>::get(KSM, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		let call_as_subaccount =
			KusamaCall::Utility(Box::new(UtilityCall::AsDerivative(sub_account_index, call)));

		let (weight, fee) = XcmDestWeightAndFee::<T>::get(KSM, operation)
			.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		let xcm_message = Self::construct_xcm_message(call_as_subaccount, fee, weight);
		XcmSender::send_xcm(Parent, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(())
	}

	fn do_unbond(who: &MultiLocation, amount: BalanceOf<T>) -> DispatchResult {
		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Unbond(amount));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Unbond, call, who.clone())?;

		Ok(())
	}

	fn update_ledger_query_response_storage(
		query_id: XcmQueryId,
		query_entry: LedgerUpdateEntry<BalanceOf<T>, MultiLocation>,
	) -> Result<(), Error<T>> {
		// update DelegatorLedgers<T> storage
		if let LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
			currency_id: _,
			delegator_id,
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
						if if_unlock {
							old_sub_ledger.active = old_sub_ledger
								.active
								.checked_sub(&amount)
								.ok_or(Error::<T>::UnderFlow)?;

							let unlock_time_unit =
								unlock_time.ok_or(Error::<T>::TimeUnitNotExist)?;

							let new_unlock_record =
								UnlockChunk { value: amount, unlock_time: unlock_time_unit };

							old_sub_ledger.unlocking.push(new_unlock_record);
						} else {
							if if_rebond {
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
							}
						}
					}
					Ok(())
				},
			)?;

			// Delete the DelegatorLedgerXcmUpdateQueue<T> query
			DelegatorLedgerXcmUpdateQueue::<T>::remove(query_id);

			Ok(())
		} else {
			Err(Error::<T>::Unexpected)
		}
	}

	fn update_validators_by_delegator_query_response_storage(
		query_id: XcmQueryId,
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
			Ok(())
		} else {
			Err(Error::<T>::Unexpected)
		}
	}
}

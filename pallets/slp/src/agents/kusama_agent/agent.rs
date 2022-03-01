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

use codec::{Decode, Encode};
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get, weights::Weight};
use sp_runtime::{
	traits::{CheckedAdd, CheckedSub, Convert, UniqueSaturatedInto, Zero},
	DispatchResult,
};
use sp_std::prelude::*;
use xcm::{
	latest::prelude::*,
	opaque::latest::{Junction::AccountId32, Junctions::X1, MultiLocation},
};

use crate::{
	agents::{KusamaCall, StakingCall, UtilityCall},
	pallet::Error,
	primitives::{Ledger, SubstrateLedger, UnlockChunk, XcmOperation, KSM},
	traits::{DelegatorManager, StakingAgent, XcmBuilder},
	AccountIdOf, BalanceOf, Config, DelegatorLedgers, DelegatorNextIndex,
	DelegatorsIndex2Multilocation, DelegatorsMultilocation2Index, MinimumsAndMaximums,
	ValidatorManager, ValidatorsByDelegator, XcmDestWeightAndFee,
};

/// StakingAgent implementation for Kusama
pub struct KusamaAgent<T, AccountConverter, ParachainId, XcmSender>(
	PhantomData<(T, AccountConverter, ParachainId, XcmSender)>,
);

impl<T, AccountConverter, ParachainId, XcmSender>
	KusamaAgent<T, AccountConverter, ParachainId, XcmSender>
{
	pub fn new() -> Self {
		KusamaAgent(PhantomData::<(T, AccountConverter, ParachainId, XcmSender)>)
	}
}

impl<T, AccountConverter, ParachainId, XcmSender>
	StakingAgent<MultiLocation, MultiLocation, BalanceOf<T>>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
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
		DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::AlreadyBonded)?;

		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.delegator_bonded_minimum, Error::<T>::LowerThanMinimum);

		// Ensure the bond doesn't exceeds delegator_active_staking_maximum
		ensure!(
			amount <= mins_maxs.delegator_active_staking_maximum,
			Error::<T>::ExceedActiveMaximum
		);

		// Get the delegator account id in Kusama network
		let delegator_account = Self::multilocation_to_account(&who)?;

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Bond(
			delegator_account.clone(),
			amount,
			delegator_account,
		));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Bond, call, who.clone())?;

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

		// Check if the unbonding amount exceeds minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.unbond_minimum, Error::<T>::LowerThanMinimum);

		// Get the delegator ledger
		let Ledger::Substrate(substrate_ledger) = ledger;

		// Check if the remaining active balance is enough for (unbonding amount + minimum bonded
		// amount)
		let active_staking = substrate_ledger.active;
		let remaining = active_staking.checked_sub(&amount).ok_or(Error::<T>::NotEnoughToUnbond)?;
		ensure!(remaining >= mins_maxs.delegator_bonded_minimum, Error::<T>::NotEnoughToUnbond);

		// Check if this unbonding will exceed the maximum unlocking records bound for a single
		// delegator.
		let unlocking_num = substrate_ledger.unlocking.len() as u32;
		ensure!(
			unlocking_num < mins_maxs.unbond_record_maximum,
			Error::<T>::ExceedUnlockingRecords
		);

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Unbond(amount));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Unbond, call, who.clone())?;

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
		for UnlockChunk { value, unlock_time } in unlock_chunk_list.iter() {
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
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if targets vec is empty.
		let vec_len = targets.len() as u32;
		ensure!(vec_len > Zero::zero(), Error::<T>::VectorEmpty);

		// Check if targets exceeds validators_back_maximum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
		ensure!(vec_len <= mins_maxs.validators_back_maximum, Error::<T>::GreaterThanMaximum);

		// Convert vec of multilocations into accounts.
		let mut accounts = vec![];
		for multilocation_account in targets.iter() {
			let account = Self::multilocation_to_account(multilocation_account)?;
			accounts.push(account);
		}

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Nominate(accounts));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Delegate, call, who.clone())?;

		Ok(())
	}

	/// Remove delegation relationship with some validators.
	fn undelegate(&self, who: MultiLocation, targets: Vec<MultiLocation>) -> DispatchResult {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(KSM, who.clone()).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if targets vec is empty.
		let vec_len = targets.len() as u32;
		ensure!(vec_len > Zero::zero(), Error::<T>::VectorEmpty);

		// Get the original delegated validators.
		let original_set = ValidatorsByDelegator::<T>::get(KSM, who.clone())
			.ok_or(Error::<T>::ValidatorSetNotExist)?;

		// Remove targets from the original set to make a new set.
		let mut new_set: Vec<MultiLocation> = vec![];
		for acc in original_set.iter() {
			if !targets.contains(acc) {
				new_set.push(acc.clone())
			}
		}

		// Ensure new set is not empty.
		ensure!(new_set.len() > Zero::zero(), Error::<T>::VectorEmpty);

		// Convert new targets into account vec.
		let mut accounts = vec![];
		for multilocation_account in new_set.iter() {
			let account = Self::multilocation_to_account(multilocation_account)?;
			accounts.push(account);
		}

		// Construct xcm message.
		let call = KusamaCall::Staking(StakingCall::Nominate(accounts));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount(XcmOperation::Delegate, call, who.clone())?;

		Ok(())
	}

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(&self, who: MultiLocation, targets: Vec<MultiLocation>) -> DispatchResult {
		Self::delegate(&self, who, targets)?;
		Ok(())
	}

	/// Initiate payout for a certain delegator.
	fn payout(&self, who: MultiLocation) -> BalanceOf<T> {
		unimplemented!()
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(&self, who: MultiLocation) -> BalanceOf<T> {
		unimplemented!()
	}

	/// Increase/decrease the token amount for the storage "token_pool" in the VtokenMining
	/// module. If the increase variable is true, then we increase token_pool by token_amount.
	/// If it is false, then we decrease token_pool by token_amount.
	fn increase_token_pool(&self, token_amount: BalanceOf<T>) -> DispatchResult {
		unimplemented!()
	}
	///
	fn decrease_token_pool(&self, token_amount: BalanceOf<T>) -> DispatchResult {
		unimplemented!()
	}
}

/// DelegatorManager implementation for Kusama
impl<T, AccountConverter, ParachainId, XcmSender>
	DelegatorManager<MultiLocation, SubstrateLedger<MultiLocation, BalanceOf<T>>>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
{
	/// Add a new serving delegator for a particular currency.
	fn add_delegator(&self, index: u16, who: &MultiLocation) -> DispatchResult {
		DelegatorsIndex2Multilocation::<T>::insert(KSM, index, who);
		DelegatorsMultilocation2Index::<T>::insert(KSM, who, index);
		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(&self, who: &MultiLocation) -> DispatchResult {
		unimplemented!()
	}
}

/// ValidatorManager implementation for Kusama
impl<T, AccountConverter, ParachainId, XcmSender> ValidatorManager<MultiLocation>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
{
	/// Add a new serving delegator for a particular currency.
	fn add_validator(&self, who: &MultiLocation) -> DispatchResult {
		unimplemented!()
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_validator(&self, who: &MultiLocation) -> DispatchResult {
		unimplemented!()
	}
}

/// Trait XcmBuilder implementation for Kusama
impl<T, AccountConverter, ParachainId, XcmSender> XcmBuilder<BalanceOf<T>, KusamaCall<T>>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
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

/// Internal functions.
impl<T, AccountConverter, ParachainId, XcmSender>
	KusamaAgent<T, AccountConverter, ParachainId, XcmSender>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
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

		let (weight, fee) = XcmDestWeightAndFee::<T>::get(KSM, operation);

		let xcm_message = Self::construct_xcm_message(call_as_subaccount, fee, weight);
		XcmSender::send_xcm(Parent, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(())
	}

	fn multilocation_to_account(who: &MultiLocation) -> Result<AccountIdOf<T>, Error<T>> {
		// Get the delegator account id in Kusama network
		let account_32 = match who {
			MultiLocation {
				parents: 1,
				interior: X1(AccountId32 { network: NetworkId, id: account_id }),
			} => account_id,
			_ => Err(Error::<T>::AccountNotExist)?,
		};
		let account =
			T::AccountId::decode(&mut &account_32[..]).map_err(|_| Error::<T>::DecodingError)?;

		Ok(account)
	}
}

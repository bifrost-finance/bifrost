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

use super::types::{AstarCall, AstarDappsStakingCall, AstarUtilityCall, SmartContract, XcmCall};
use crate::{
	pallet::{Error, Event},
	primitives::{
		Ledger, QueryId, SubstrateLedger, SubstrateLedgerUpdateEntry,
		SubstrateLedgerUpdateOperation, UnlockChunk, ValidatorsByDelegatorUpdateEntry,
		XcmOperation, TIMEOUT_BLOCKS,
	},
	traits::{QueryResponseManager, StakingAgent, XcmBuilder},
	AccountIdOf, BalanceOf, Config, CurrencyDelays, DelegatorLedgerXcmUpdateQueue,
	DelegatorLedgers, DelegatorsMultilocation2Index, LedgerUpdateEntry, MinimumsAndMaximums,
	Pallet, TimeUnit, Validators, XcmDestWeightAndFee, XcmWeight,
};
use codec::Encode;
use core::marker::PhantomData;
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use node_primitives::{CurrencyId, VtokenMintingOperator, ASTR_TOKEN_ID};
use polkadot_parachain::primitives::Sibling;
use sp_runtime::{
	traits::{
		AccountIdConversion, CheckedAdd, CheckedSub, Convert, Saturating, UniqueSaturatedInto, Zero,
	},
	DispatchResult, SaturatedConversion,
};
use sp_std::prelude::*;
use xcm::{
	opaque::v3::{Instruction, Junction::Parachain, Junctions::X1, MultiLocation},
	v3::{prelude::*, Weight},
	VersionedMultiAssets, VersionedMultiLocation,
};
use xcm_interface::traits::parachains;

/// StakingAgent implementation for Astar
pub struct AstarAgent<T>(PhantomData<T>);

impl<T> AstarAgent<T> {
	pub fn new() -> Self {
		AstarAgent(PhantomData::<T>)
	}
}

impl<T: Config>
	StakingAgent<
		BalanceOf<T>,
		AccountIdOf<T>,
		LedgerUpdateEntry<BalanceOf<T>>,
		ValidatorsByDelegatorUpdateEntry,
		Error<T>,
	> for AstarAgent<T>
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

	/// Bond some amount to a delegator.
	fn bond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		let contract_multilocation = validator.ok_or(Error::<T>::ValidatorNotProvided)?;
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		// Check if the amount exceeds the minimum requirement.
		ensure!(amount >= mins_maxs.bond_extra_minimum, Error::<T>::LowerThanMinimum);

		// check if the validator is in the white list.
		let validator_list =
			Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;
		validator_list
			.iter()
			.position(|va| va == &contract_multilocation)
			.ok_or(Error::<T>::ValidatorNotExist)?;

		if DelegatorLedgers::<T>::get(currency_id, who).is_none() {
			// Check if the amount exceeds the minimum requirement. The first bond requires 500 ASTR
			ensure!(amount >= mins_maxs.delegator_bonded_minimum, Error::<T>::LowerThanMinimum);

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
		}

		// Get the contract_h160
		let contract_h160 = Pallet::<T>::multilocation_to_h160_account(&contract_multilocation)?;
		let smart_contract = SmartContract::<T::AccountId>::Evm(contract_h160);

		// Construct xcm message.
		let call = AstarCall::Staking(AstarDappsStakingCall::BondAndStake(
			smart_contract,
			amount.saturated_into(),
		));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) = Self::construct_xcm_as_subaccount_with_query_id(
			XcmOperation::Bond,
			call,
			who,
			currency_id,
		)?;

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
		let dest = Self::get_astar_multilocation();
		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Bond extra amount to a delegator.
	fn bond_extra(
		&self,
		_who: &MultiLocation,
		_amount: BalanceOf<T>,
		_validator: &Option<MultiLocation>,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		// Check if the unbonding amount exceeds minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.unbond_minimum, Error::<T>::LowerThanMinimum);

		// check if the delegator exists, if not, return error.
		let contract_multilocation = (*validator).ok_or(Error::<T>::ValidatorNotProvided)?;

		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		if let Ledger::Substrate(substrate_ledger) = ledger {
			// Check if this unbonding will exceed the maximum unlocking records bound for a single
			let unlocking_num = substrate_ledger.unlocking.len() as u32;
			ensure!(
				unlocking_num < mins_maxs.unbond_record_maximum,
				Error::<T>::ExceedUnlockingRecords
			);
		} else {
			Err(Error::<T>::Unexpected)?;
		}

		// Construct xcm message.
		let contract_h160 = Pallet::<T>::multilocation_to_h160_account(&contract_multilocation)?;
		let smart_contract = SmartContract::<T::AccountId>::Evm(contract_h160);
		let call = AstarCall::Staking(AstarDappsStakingCall::UnbondAndUnstake(
			smart_contract,
			amount.saturated_into(),
		));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) = Self::construct_xcm_as_subaccount_with_query_id(
			XcmOperation::Unbond,
			call,
			who,
			currency_id,
		)?;

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
		let dest = Self::get_astar_multilocation();
		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Unbonding all amount of a delegator. Differentiate from regular unbonding.
	fn unbond_all(
		&self,
		_who: &MultiLocation,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Cancel some unbonding amount.
	fn rebond(
		&self,
		_who: &MultiLocation,
		_amount: Option<BalanceOf<T>>,
		_validator: &Option<MultiLocation>,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unexpected)
	}

	/// Delegate to some validators. For Kusama/Polkadot, it equals function Nominate.
	fn delegate(
		&self,
		_who: &MultiLocation,
		_targets: &Vec<MultiLocation>,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Remove delegation relationship with some validators.
	fn undelegate(
		&self,
		_who: &MultiLocation,
		_targets: &Vec<MultiLocation>,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(
		&self,
		_who: &MultiLocation,
		_targets: &Option<Vec<MultiLocation>>,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// claim staker
	fn payout(
		&self,
		who: &MultiLocation,
		validator: &MultiLocation,
		_when: &Option<TimeUnit>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		// Get the validator account
		let contract_h160 = Pallet::<T>::multilocation_to_h160_account(&validator)?;
		let smart_contract = SmartContract::<T::AccountId>::Evm(contract_h160);

		// Construct xcm message.
		let call = AstarCall::Staking(AstarDappsStakingCall::ClaimStaker(smart_contract));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperation::Payout,
			call,
			who,
			currency_id,
		)?;

		Ok(Zero::zero())
	}

	/// Withdraw the unbound amount.
	fn liquidize(
		&self,
		who: &MultiLocation,
		_when: &Option<TimeUnit>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
		_amount: Option<BalanceOf<T>>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is in the delegator set.
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(currency_id, who),
			Error::<T>::DelegatorNotExist
		);

		// Construct xcm message.
		let call = AstarCall::Staking(AstarDappsStakingCall::WithdrawUnbonded);

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, xcm_message) = Self::construct_xcm_as_subaccount_with_query_id(
			XcmOperation::Liquidize,
			call,
			who,
			currency_id,
		)?;

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
		let dest = Self::get_astar_multilocation();
		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Chill self. Cancel the identity of delegator in the Relay chain side.
	/// Unbonding all the active amount should be done before or after chill,
	/// so that we can collect back all the bonded amount.
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
		let to_account_id = Pallet::<T>::multilocation_to_account(to)?;
		let (_, exit_account) = T::VtokenMinting::get_entrance_and_exit_accounts();
		ensure!(to_account_id == exit_account, Error::<T>::InvalidAccount);

		// Prepare parameter dest and beneficiary.
		let to_32: [u8; 32] = Pallet::<T>::multilocation_to_account_32(to)?;

		let dest = Box::new(VersionedMultiLocation::from(MultiLocation::new(
			1,
			X1(Parachain(T::ParachainId::get().into())),
		)));

		let beneficiary =
			Box::new(VersionedMultiLocation::from(MultiLocation::from(X1(AccountId32 {
				network: None,
				id: to_32,
			}))));

		// Prepare parameter assets.
		let asset = MultiAsset {
			fun: Fungible(amount.unique_saturated_into()),
			id: Concrete(MultiLocation { parents: 0, interior: Here }),
		};
		let assets: Box<VersionedMultiAssets> =
			Box::new(VersionedMultiAssets::from(MultiAssets::from(asset)));

		// Prepare parameter fee_asset_item.
		let fee_asset_item: u32 = 0;

		let (weight_limit, _) =
			XcmDestWeightAndFee::<T>::get(currency_id, XcmOperation::TransferBack)
				.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		// Construct xcm message.
		let call = AstarCall::Xcm(Box::new(XcmCall::LimitedReserveTransferAssets(
			dest,
			beneficiary,
			assets,
			fee_asset_item,
			Limited(weight_limit),
		)));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		Self::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperation::TransferBack,
			call,
			from,
			currency_id,
		)?;

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

		Self::do_transfer_to(from, to, amount, currency_id)?;

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
		// Get current vASTR/ASTR exchange rate.
		let vtoken = CurrencyId::VToken2(ASTR_TOKEN_ID);

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
		Self::do_transfer_to(from, to, amount, currency_id)?;

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

/// Trait XcmBuilder implementation for ASTAR
impl<T: Config> XcmBuilder<BalanceOf<T>, AstarCall<T>, Error<T>> for AstarAgent<T> {
	fn construct_xcm_message(
		call: AstarCall<T>,
		extra_fee: BalanceOf<T>,
		weight: XcmWeight,
		_currency_id: CurrencyId,
		query_id: Option<QueryId>,
	) -> Result<Xcm<()>, Error<T>> {
		let mut xcm_message = Self::inner_construct_xcm_message(extra_fee);
		let transact_instruct = Transact {
			origin_kind: OriginKind::SovereignAccount,
			require_weight_at_most: weight,
			call: call.encode().into(),
		};
		xcm_message.insert(2, transact_instruct);
		if let Some(query_id) = query_id {
			let report_transact_status_instruct =
				Self::get_report_transact_status_instruct(query_id, weight);
			xcm_message.insert(3, report_transact_status_instruct);
		}
		Ok(Xcm(xcm_message))
	}
}

/// Internal functions.
impl<T: Config> AstarAgent<T> {
	fn get_astar_multilocation() -> MultiLocation {
		MultiLocation { parents: 1, interior: X1(Parachain(parachains::astar::ID)) }
	}

	fn prepare_send_as_subaccount_call(
		operation: XcmOperation,
		call: AstarCall<T>,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<(AstarCall<T>, BalanceOf<T>, XcmWeight), Error<T>> {
		// Get the delegator sub-account index.
		let sub_account_index = DelegatorsMultilocation2Index::<T>::get(currency_id, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		let call_as_subaccount = AstarCall::Utility(Box::new(AstarUtilityCall::AsDerivative(
			sub_account_index,
			Box::new(call),
		)));

		let (weight, fee) = XcmDestWeightAndFee::<T>::get(currency_id, operation)
			.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		Ok((call_as_subaccount, fee, weight))
	}

	fn construct_xcm_as_subaccount_with_query_id(
		operation: XcmOperation,
		call: AstarCall<T>,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<(QueryId, BlockNumberFor<T>, Xcm<()>), Error<T>> {
		// prepare the query_id for reporting back transact status
		let responder = Self::get_astar_multilocation();
		let now = frame_system::Pallet::<T>::block_number();
		let timeout = T::BlockNumber::from(TIMEOUT_BLOCKS).saturating_add(now);

		// Generate query_id need( responder,callback, timeout)
		let query_id = match operation {
			XcmOperation::Bond | XcmOperation::Unbond | XcmOperation::Liquidize =>
				T::SubstrateResponseManager::create_query_record(
					&responder,
					Some(Pallet::<T>::confirm_delegator_ledger_call()),
					timeout,
				),
			_ => {
				ensure!(false, Error::<T>::Unsupported);
				0
			},
		};

		let (call_as_subaccount, fee, weight) =
			Self::prepare_send_as_subaccount_call(operation, call, who, currency_id)?;

		let xcm_message = Self::construct_xcm_message(
			call_as_subaccount,
			fee,
			weight,
			currency_id,
			Some(query_id),
		)?;

		Ok((query_id, timeout, xcm_message))
	}

	fn construct_xcm_and_send_as_subaccount_without_query_id(
		operation: XcmOperation,
		call: AstarCall<T>,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		let (call_as_subaccount, fee, weight) =
			Self::prepare_send_as_subaccount_call(operation, call, who, currency_id)?;

		let xcm_message =
			Self::construct_xcm_message(call_as_subaccount, fee, weight, currency_id, None)?;

		let dest = Self::get_astar_multilocation();

		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(())
	}

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
								Err(Error::<T>::Unexpected)?;
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
		ensure!(
			T::SubstrateResponseManager::remove_query_record(query_id),
			Error::<T>::QueryResponseRemoveError
		);

		Ok(())
	}

	fn get_unlocking_era_from_current(
		currency_id: CurrencyId,
	) -> Result<Option<TimeUnit>, Error<T>> {
		let current_time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
			.ok_or(Error::<T>::TimeUnitNotExist)?;
		let delays = CurrencyDelays::<T>::get(currency_id).ok_or(Error::<T>::DelaysNotExist)?;

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
			Unlock => Self::get_unlocking_era_from_current(currency_id)?,
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

	fn do_transfer_to(
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// Prepare parameter dest and beneficiary.
		let dest = Self::get_astar_multilocation();

		// Prepare parameter assets.
		let assets = {
			let asset =
				MultiAsset { fun: Fungible(amount.unique_saturated_into()), id: Concrete(dest) };
			MultiAssets::from(asset)
		};

		Pallet::<T>::inner_do_transfer_to(from, to, amount, currency_id, assets, &dest)
	}

	fn inner_construct_xcm_message(extra_fee: BalanceOf<T>) -> Vec<Instruction> {
		let asset = MultiAsset {
			id: Concrete(MultiLocation::here()),
			fun: Fungible(extra_fee.unique_saturated_into()),
		};

		let self_sibling_parachain_account: [u8; 32] =
			Sibling::from(T::ParachainId::get()).into_account_truncating();

		vec![
			WithdrawAsset(asset.clone().into()),
			BuyExecution { fees: asset, weight_limit: Unlimited },
			RefundSurplus,
			DepositAsset {
				assets: AllCounted(8).into(),
				beneficiary: MultiLocation {
					parents: 0,
					interior: X1(AccountId32 { network: None, id: self_sibling_parachain_account }),
				},
			},
		]
	}

	fn get_report_transact_status_instruct(query_id: QueryId, max_weight: Weight) -> Instruction {
		ReportTransactStatus(QueryResponseInfo {
			destination: MultiLocation::new(1, X1(Parachain(u32::from(T::ParachainId::get())))),
			query_id,
			max_weight,
		})
	}
}

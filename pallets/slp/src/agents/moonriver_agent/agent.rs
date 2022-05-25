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

use super::types::{
	MoonriverBalancesCall, MoonriverCall, MoonriverCurrencyId, MoonriverParachainStakingCall,
	MoonriverUtilityCall, MoonriverXtokensCall,
};
use crate::primitives::OneToManyLedger;
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
		Ledger, MoonriverLedgerUpdateEntry, OneToManyBond, OneToManyDelegatorStatus,
		SubstrateLedger, SubstrateValidatorsByDelegatorUpdateEntry, UnlockChunk,
		ValidatorsByDelegatorUpdateEntry, XcmOperation, MOVR,
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
			// if exists, return error. If not, continue.
			ensure!(!ledger.delegations.contains_key(&collator), Error::<T>::AlreadyBonded);
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
			let new_ledger = OneToManyLedger::<MultiLocation, MultiLocation, BalanceOf<T>> {
				account: who.clone(),
				total: Zero::zero(),
				less_total: Zero::zero(),
				delegations: empty_delegation_set,
				requests: vec![],
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
			who, &collator, true, false, false, false, false, amount, query_id, timeout,
		)?;

		// Send out the xcm message.
		T::XcmRouter::send_xcm(Parent, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Bond extra amount for a existing delegation.
	fn bond_extra(&self, who: &MultiLocation, amount: BalanceOf<T>) -> Result<QueryId, Error<T>> {
		unimplemented!()
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(&self, who: &MultiLocation, amount: BalanceOf<T>) -> Result<QueryId, Error<T>> {
		unimplemented!()
	}

	/// Unbonding all amount of a delegator. Differentiate from regular unbonding.
	fn unbond_all(&self, who: &MultiLocation) -> Result<QueryId, Error<T>> {
		unimplemented!()
	}

	/// Cancel some unbonding amount.
	fn rebond(&self, who: &MultiLocation, amount: BalanceOf<T>) -> Result<QueryId, Error<T>> {
		unimplemented!()
	}

	/// Delegate to some validators. For Moonriver, it equals function Nominate.
	fn delegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		Error::<T>::Unsupported
	}

	/// Remove delegation relationship with some validators.
	fn undelegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		unimplemented!()
	}

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		unimplemented!()
	}

	/// Initiate payout for a certain delegator.
	fn payout(
		&self,
		who: &MultiLocation,
		validator: &MultiLocation,
		when: &Option<TimeUnit>,
	) -> Result<(), Error<T>> {
		unimplemented!()
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(&self, who: &MultiLocation, when: &Option<TimeUnit>) -> Result<QueryId, Error<T>> {
		unimplemented!()
	}

	/// Chill self. Cancel the identity of delegator in the Relay chain side.
	/// Unbonding all the active amount should be done before or after chill,
	/// so that we can collect back all the bonded amount.
	fn chill(&self, who: &MultiLocation) -> Result<QueryId, Error<T>> {
		unimplemented!()
	}

	/// Make token transferred back to Bifrost chain account.
	fn transfer_back(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		unimplemented!()
	}

	/// Make token from Bifrost chain account to the staking chain account.
	/// Receiving account must be one of the KSM delegators.
	fn transfer_to(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		unimplemented!()
	}

	fn tune_vtoken_exchange_rate(
		&self,
		who: &MultiLocation,
		token_amount: BalanceOf<T>,
		_vtoken_amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		unimplemented!()
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
		unimplemented!()
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
		unimplemented!()
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
		unimplemented!()
	}

	fn fail_validators_by_delegator_query_response(
		&self,
		query_id: QueryId,
	) -> Result<(), Error<T>> {
		unimplemented!()
	}
}

/// Internal functions.
impl<T: Config> MoonriverAgent<T> {
	fn construct_xcm_as_subaccount_with_query_id(
		operation: XcmOperation,
		call: MoonriverCall<T>,
		who: &MultiLocation,
	) -> Result<(QueryId, BlockNumberFor<T>, Xcm<()>), Error<T>> {
		// prepare the query_id for reporting back transact status
		let responder = MultiLocation { parents: 1, interior: X1(Parachain(2023)) };
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

	fn insert_delegator_ledger_update_entry(
		who: &MultiLocation,
		validator: &MultiLocation,
		if_bond: bool,
		if_unlock: bool,
		if_revoke: bool,
		if_cancel: bool,
		if_leave: bool,
		amount: BalanceOf<T>,
		query_id: QueryId,
		timeout: BlockNumberFor<T>,
	) -> Result<(), Error<T>> {
		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.

		// First to see if the delegation relationship exist.
		// If not, create one. If yes,

		let unlock_time = if if_unlock || if_revoke || if_leave {
			Self::get_unlocking_round_from_current(if_leave)?
		} else if if_bond || if_cancel {
			None
		//liquidize operation
		} else {
			T::VtokenMinting::get_ongoing_time_unit(MOVR)
		};

		let entry = LedgerUpdateEntry::Moonriver(MoonriverLedgerUpdateEntry {
			currency_id: MOVR,
			delegator_id: who.clone(),
			validator_id: validator.clone(),
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
		// }
	}

	fn construct_xcm_message_without_query_id(
		call: MoonriverCall<T>,
		extra_fee: BalanceOf<T>,
		weight: Weight,
	) -> Xcm<()> {
		unimplemented!()
	}
}

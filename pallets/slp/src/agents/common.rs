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
use crate::{
	pallet::Error,
	primitives::{
		ParachainStakingLedgerUpdateEntry, ParachainStakingLedgerUpdateOperation, TIMEOUT_BLOCKS,
	},
	traits::QueryResponseManager,
	vec, AccountIdOf, BalanceOf, BlockNumberFor, BoundedVec, Config, CurrencyDelays,
	DelegationsOccupied, DelegatorLatestTuneRecord, DelegatorLedgerXcmUpdateQueue,
	DelegatorLedgers, DelegatorNextIndex, DelegatorsIndex2Multilocation,
	DelegatorsMultilocation2Index, Encode, Event, FeeSources, Ledger, LedgerUpdateEntry,
	MinimumsAndMaximums, Pallet, TimeUnit, Validators, Vec, Weight, XcmOperationType, Zero, ASTR,
	BNC, DOT, GLMR, KSM, MANTA, MOVR, PHA,
};
use bifrost_primitives::{CurrencyId, VtokenMintingOperator, XcmDestWeightAndFeeHandler};
use frame_support::{dispatch::GetDispatchInfo, ensure, traits::Len};
use orml_traits::{MultiCurrency, XcmTransfer};
use polkadot_parachain_primitives::primitives::Sibling;
use sp_core::{Get, U256};
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, UniqueSaturatedFrom, UniqueSaturatedInto},
	DispatchResult, Saturating,
};
use xcm::v3::{prelude::*, MultiLocation};

// Some common business functions for all agents
impl<T: Config> Pallet<T> {
	pub(crate) fn inner_initialize_delegator(currency_id: CurrencyId) -> Result<u16, Error<T>> {
		let new_delegator_id = DelegatorNextIndex::<T>::get(currency_id);
		DelegatorNextIndex::<T>::mutate(currency_id, |id| -> Result<(), Error<T>> {
			let option_new_id = id.checked_add(1).ok_or(Error::<T>::OverFlow)?;
			*id = option_new_id;
			Ok(())
		})?;

		Ok(new_delegator_id)
	}

	/// Add a new serving delegator for a particular currency.
	pub(crate) fn inner_add_delegator(
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

		Ok(())
	}

	pub(crate) fn inner_add_validator(
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> DispatchResult {
		// Check if the validator already exists.
		let validators_set = Validators::<T>::get(currency_id);

		// Ensure validator candidates in the whitelist is not greater than maximum.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(
			validators_set.len() as u16 <= mins_maxs.validators_maximum,
			Error::<T>::GreaterThanMaximum
		);

		// ensure validator candidates are less than MaxLengthLimit
		ensure!(
			validators_set.len() < T::MaxLengthLimit::get() as usize,
			Error::<T>::ExceedMaxLengthLimit
		);

		let mut validators_vec;
		if let Some(validators_bounded_vec) = validators_set {
			validators_vec = validators_bounded_vec.to_vec();
			let rs = validators_vec.iter().position(|multi| multi == who);
			// Check if the validator is in the already exist.
			ensure!(rs.is_none(), Error::<T>::AlreadyExist);

			// If the validator is not in the whitelist, add it.
			validators_vec.push(*who);
		} else {
			validators_vec = vec![*who];
		}

		let bounded_list =
			BoundedVec::try_from(validators_vec).map_err(|_| Error::<T>::FailToConvert)?;

		Validators::<T>::insert(currency_id, bounded_list);

		// Deposit event.
		Self::deposit_event(Event::ValidatorsAdded { currency_id, validator_id: *who });

		Ok(())
	}

	pub(crate) fn inner_remove_delegator(
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> DispatchResult {
		// Check if the delegator exists.
		let index = DelegatorsMultilocation2Index::<T>::get(currency_id, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;
		// Remove corresponding storage.
		DelegatorsIndex2Multilocation::<T>::remove(currency_id, index);
		DelegatorsMultilocation2Index::<T>::remove(currency_id, who);
		DelegatorLedgers::<T>::remove(currency_id, who);

		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	pub(crate) fn inner_remove_validator(
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> DispatchResult {
		// Check if the validator already exists.
		let validators_set =
			Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;

		ensure!(validators_set.contains(who), Error::<T>::ValidatorNotExist);

		// Update corresponding storage.
		Validators::<T>::mutate(currency_id, |validator_vec| {
			if let Some(ref mut validator_list) = validator_vec {
				let index_op = validator_list.clone().iter().position(|va| va == who);

				if let Some(index) = index_op {
					validator_list.remove(index);

					Self::deposit_event(Event::ValidatorsRemoved {
						currency_id,
						validator_id: *who,
					});
				}
			}
		});

		Ok(())
	}

	/// Charge vtoken for hosting fee.
	pub(crate) fn inner_calculate_vtoken_hosting_fee(
		amount: BalanceOf<T>,
		vtoken: CurrencyId,
		currency_id: CurrencyId,
	) -> Result<BalanceOf<T>, Error<T>> {
		ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

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

		let charge_amount = BalanceOf::<T>::unique_saturated_from(can_get_vtoken);

		Ok(charge_amount)
	}

	pub(crate) fn inner_charge_hosting_fee(
		charge_amount: BalanceOf<T>,
		to: &MultiLocation,
		depoist_currency: CurrencyId,
	) -> DispatchResult {
		ensure!(charge_amount > Zero::zero(), Error::<T>::AmountZero);

		let beneficiary = Self::multilocation_to_account(&to)?;
		// Issue corresponding vksm to beneficiary account.
		T::MultiCurrency::deposit(depoist_currency, &beneficiary, charge_amount)?;

		Ok(())
	}

	pub(crate) fn tune_vtoken_exchange_rate_without_update_ledger(
		who: &MultiLocation,
		token_amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// ensure who is a valid delegator
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(currency_id, &who),
			Error::<T>::DelegatorNotExist
		);

		// Get current TimeUnit.
		let current_time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
			.ok_or(Error::<T>::TimeUnitNotExist)?;
		// Get DelegatorLatestTuneRecord for the currencyId.
		let latest_time_unit_op = DelegatorLatestTuneRecord::<T>::get(currency_id, &who);
		// ensure each delegator can only tune once per TimeUnit.
		ensure!(
			latest_time_unit_op != Some(current_time_unit.clone()),
			Error::<T>::DelegatorAlreadyTuned
		);

		ensure!(!token_amount.is_zero(), Error::<T>::AmountZero);

		// Check whether "who" is an existing delegator.
		ensure!(
			DelegatorLedgers::<T>::contains_key(currency_id, who),
			Error::<T>::DelegatorNotBonded
		);

		// Tune the vtoken exchange rate.
		T::VtokenMinting::increase_token_pool(currency_id, token_amount)
			.map_err(|_| Error::<T>::IncreaseTokenPoolError)?;

		// Update the DelegatorLatestTuneRecord<T> storage.
		DelegatorLatestTuneRecord::<T>::insert(currency_id, who, current_time_unit);

		Ok(())
	}

	pub(crate) fn burn_fee_from_source_account(
		fee: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// get fee source first
		let (source_location, reserved_fee) =
			FeeSources::<T>::get(currency_id).ok_or(Error::<T>::FeeSourceNotExist)?;

		// check if fee is too high to be covered.
		ensure!(fee <= reserved_fee, Error::<T>::FeeTooHigh);

		let source_account = Self::native_multilocation_to_account(&source_location)?;

		// withdraw. If withdraw fails, issue an event and continue.
		if let Err(_) = T::MultiCurrency::withdraw(currency_id, &source_account, fee) {
			// Deposit event
			Self::deposit_event(Event::BurnFeeFailed { currency_id, amount: fee });
		}

		Ok(())
	}

	pub(crate) fn inner_construct_xcm_message(
		currency_id: CurrencyId,
		extra_fee: BalanceOf<T>,
	) -> Result<Vec<xcm::v4::Instruction<()>>, Error<T>> {
		let remote_fee_location = Self::convert_currency_to_remote_fee_location(currency_id);

		let asset = xcm::v4::Asset {
			id: xcm::v4::AssetId(remote_fee_location),
			fun: xcm::v4::prelude::Fungible(extra_fee.unique_saturated_into()),
		};

		let refund_receiver = Self::convert_currency_to_refund_receiver(currency_id);

		Ok(vec![
			xcm::v4::prelude::WithdrawAsset(asset.clone().into()),
			xcm::v4::prelude::BuyExecution { fees: asset, weight_limit: Unlimited },
			xcm::v4::prelude::RefundSurplus,
			xcm::v4::prelude::DepositAsset {
				assets: xcm::v4::prelude::AllCounted(8).into(),
				beneficiary: xcm::v4::prelude::Location { parents: 0, interior: refund_receiver },
			},
		])
	}

	pub(crate) fn convert_currency_to_refund_receiver(
		currency_id: CurrencyId,
	) -> xcm::v4::Junctions {
		let interior = match currency_id {
			KSM | DOT => xcm::v4::Junctions::from([xcm::v4::prelude::Parachain(
				T::ParachainId::get().into(),
			)]),
			MOVR | GLMR => xcm::v4::Junctions::from([xcm::v4::prelude::AccountKey20 {
				network: None,
				key: Sibling::from(T::ParachainId::get()).into_account_truncating(),
			}]),
			_ => xcm::v4::Junctions::from([xcm::v4::prelude::AccountId32 {
				network: None,
				id: Sibling::from(T::ParachainId::get()).into_account_truncating(),
			}]),
		};

		return interior;
	}

	pub(crate) fn prepare_send_as_subaccount_call(
		call: Vec<u8>,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<Vec<u8>, Error<T>> {
		// Get the delegator sub-account index.
		let sub_account_index = DelegatorsMultilocation2Index::<T>::get(currency_id, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// call_as_subaccount = Utility(Box::new(AsDerivative(sub_account_index, Box::new(call))));
		let utility_call: u8 = match currency_id {
			MANTA => 40,
			MOVR | GLMR => 30,
			ASTR => 11,
			PHA => 3,
			KSM => 24,
			DOT => 26,
			_ => Err(Error::<T>::Unsupported)?,
		};

		let mut call_as_subaccount: Vec<u8> = utility_call.encode();
		// Since everyone use the same Utility pallet from Substrate repo, as_derivative function is
		// always indexed 1.
		call_as_subaccount.extend(1u8.encode());
		call_as_subaccount.extend(sub_account_index.encode());
		call_as_subaccount.extend(call);

		Ok(call_as_subaccount)
	}

	pub(crate) fn construct_xcm_as_subaccount_with_query_id(
		operation: XcmOperationType,
		call: Vec<u8>,
		who: &MultiLocation,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<(QueryId, BlockNumberFor<T>, BalanceOf<T>, xcm::v4::Xcm<()>), Error<T>> {
		// prepare the query_id for reporting back transact status
		let now = frame_system::Pallet::<T>::block_number();
		let timeout = BlockNumberFor::<T>::from(TIMEOUT_BLOCKS).saturating_add(now);
		let (query_id, notify_call_weight) =
			Self::get_query_id_and_notify_call_weight(currency_id, &operation)?;

		let (transact_weight, withdraw_fee) = match weight_and_fee {
			Some((weight, fee)) => (weight, fee),
			_ => T::XcmWeightAndFeeHandler::get_operation_weight_and_fee(currency_id, operation)
				.ok_or(Error::<T>::WeightAndFeeNotExists)?,
		};

		let call_as_subaccount = Self::prepare_send_as_subaccount_call(call, who, currency_id)?;

		let xcm_message = Self::construct_xcm_message(
			call_as_subaccount,
			withdraw_fee,
			transact_weight,
			currency_id,
			Some(query_id),
			Some(notify_call_weight),
		)?;

		Ok((query_id, timeout, withdraw_fee, xcm_message))
	}

	pub(crate) fn get_query_id_and_notify_call_weight(
		currency_id: CurrencyId,
		operation: &XcmOperationType,
	) -> Result<(QueryId, Weight), Error<T>> {
		let now = frame_system::Pallet::<T>::block_number();
		let timeout = BlockNumberFor::<T>::from(TIMEOUT_BLOCKS).saturating_add(now);
		let responder = Self::convert_currency_to_dest_location(currency_id)?;

		let (notify_call_weight, callback_option) = match (currency_id, operation) {
			(DOT, &XcmOperationType::Delegate) |
			(DOT, &XcmOperationType::Undelegate) |
			(KSM, &XcmOperationType::Delegate) |
			(KSM, &XcmOperationType::Undelegate) => {
				let notify_call = Self::confirm_validators_by_delegator_call();
				(notify_call.get_dispatch_info().weight, Some(notify_call))
			},
			_ => {
				let notify_call = Self::confirm_delegator_ledger_call();
				(notify_call.get_dispatch_info().weight, Some(notify_call))
			},
		};

		let query_id =
			T::SubstrateResponseManager::create_query_record(responder, callback_option, timeout);

		return Ok((query_id, notify_call_weight));
	}

	pub(crate) fn construct_xcm_and_send_as_subaccount_without_query_id(
		operation: XcmOperationType,
		call: Vec<u8>,
		who: &MultiLocation,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<BalanceOf<T>, Error<T>> {
		let (transact_weight, withdraw_fee) = match weight_and_fee {
			Some((weight, fee)) => (weight, fee),
			_ => T::XcmWeightAndFeeHandler::get_operation_weight_and_fee(currency_id, operation)
				.ok_or(Error::<T>::WeightAndFeeNotExists)?,
		};

		let call_as_subaccount = Self::prepare_send_as_subaccount_call(call, who, currency_id)?;

		let xcm_message = Self::construct_xcm_message(
			call_as_subaccount,
			withdraw_fee,
			transact_weight,
			currency_id,
			None,
			None,
		)?;

		let dest_location = Self::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(withdraw_fee)
	}

	pub(crate) fn get_report_transact_status_instruct(
		query_id: QueryId,
		max_weight: Weight,
		currency_id: CurrencyId,
	) -> xcm::v4::Instruction<()> {
		let dest_location = match currency_id {
			DOT | KSM => xcm::v4::Location::new(
				0,
				[xcm::v4::prelude::Parachain(u32::from(T::ParachainId::get()))],
			),
			_ => xcm::v4::Location::new(
				1,
				[xcm::v4::prelude::Parachain(u32::from(T::ParachainId::get()))],
			),
		};

		xcm::v4::prelude::ReportTransactStatus(xcm::v4::prelude::QueryResponseInfo {
			destination: dest_location,
			query_id,
			max_weight,
		})
	}

	pub(crate) fn insert_delegator_ledger_update_entry(
		who: &MultiLocation,
		validator: Option<MultiLocation>,
		update_operation: ParachainStakingLedgerUpdateOperation,
		amount: BalanceOf<T>,
		query_id: QueryId,
		timeout: BlockNumberFor<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		use ParachainStakingLedgerUpdateOperation::{
			BondLess, ExecuteLeave, ExecuteRequest, LeaveDelegator, Revoke,
		};
		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.

		// First to see if the delegation relationship exist.
		// If not, create one. If yes,
		let unlock_time = match &update_operation {
			BondLess | Revoke => Self::get_unlocking_time_unit_from_current(false, currency_id)?,
			LeaveDelegator => Self::get_unlocking_time_unit_from_current(true, currency_id)?,
			ExecuteRequest | ExecuteLeave => T::VtokenMinting::get_ongoing_time_unit(currency_id),
			_ => None,
		};

		let entry = LedgerUpdateEntry::ParachainStaking(ParachainStakingLedgerUpdateEntry {
			currency_id,
			delegator_id: *who,
			validator_id: validator,
			update_operation,
			amount,
			unlock_time,
		});
		DelegatorLedgerXcmUpdateQueue::<T>::insert(query_id, (entry, timeout));

		Ok(())
	}

	pub(crate) fn do_transfer_to(
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// Ensure amount is greater than zero.
		ensure!(!amount.is_zero(), Error::<T>::AmountZero);

		// Ensure the from account is located within Bifrost chain. Otherwise, the xcm massage will
		// not succeed.
		ensure!(from.parents.is_zero(), Error::<T>::InvalidTransferSource);

		let from_account = Pallet::<T>::multilocation_to_account(from)?;
		let v4_location = (*to).try_into().map_err(|()| Error::<T>::FailToConvert)?;
		T::XcmTransfer::transfer(from_account, currency_id, amount, v4_location, Unlimited)
			.map_err(|_| Error::<T>::TransferToError)?;

		Ok(())
	}

	pub(crate) fn update_all_occupied_status_storage(
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		let mut all_occupied = true;

		for (_, ledger) in DelegatorLedgers::<T>::iter_prefix(currency_id) {
			if let Ledger::ParachainStaking(moonbeam_ledger) = ledger {
				if moonbeam_ledger.delegations.len() > moonbeam_ledger.request_briefs.len() {
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

	pub(crate) fn construct_xcm_message(
		call: Vec<u8>,
		extra_fee: BalanceOf<T>,
		transact_weight: Weight,
		currency_id: CurrencyId,
		query_id: Option<QueryId>,
		notify_call_weight: Option<Weight>,
	) -> Result<xcm::v4::Xcm<()>, Error<T>> {
		let mut xcm_message = Self::inner_construct_xcm_message(currency_id, extra_fee)?;
		let transact = xcm::v4::prelude::Transact {
			origin_kind: OriginKind::SovereignAccount,
			require_weight_at_most: transact_weight,
			call: call.into(),
		};
		xcm_message.insert(2, transact);
		match (query_id, notify_call_weight) {
			(Some(query_id), Some(notify_call_weight)) => {
				let report_transact_status_instruct = Self::get_report_transact_status_instruct(
					query_id,
					notify_call_weight,
					currency_id,
				);
				xcm_message.insert(3, report_transact_status_instruct);
			},
			_ => {},
		};
		Ok(xcm::v4::Xcm(xcm_message))
	}

	pub(crate) fn get_unlocking_time_unit_from_current(
		if_leave: bool,
		currency_id: CurrencyId,
	) -> Result<Option<TimeUnit>, Error<T>> {
		let current_time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
			.ok_or(Error::<T>::TimeUnitNotExist)?;
		let delays = CurrencyDelays::<T>::get(currency_id).ok_or(Error::<T>::DelaysNotExist)?;

		let unlock_time_unit = match (currency_id, current_time_unit) {
			(ASTR, TimeUnit::Era(current_era)) |
			(KSM, TimeUnit::Era(current_era)) |
			(DOT, TimeUnit::Era(current_era)) =>
				if let TimeUnit::Era(delay_era) = delays.unlock_delay {
					let unlock_era =
						current_era.checked_add(delay_era).ok_or(Error::<T>::OverFlow)?;
					TimeUnit::Era(unlock_era)
				} else {
					Err(Error::<T>::InvalidTimeUnit)?
				},
			(PHA, TimeUnit::Hour(current_hour)) => {
				if let TimeUnit::Hour(delay_hour) = delays.unlock_delay {
					let unlock_hour =
						current_hour.checked_add(delay_hour).ok_or(Error::<T>::OverFlow)?;
					TimeUnit::Hour(unlock_hour)
				} else {
					Err(Error::<T>::InvalidTimeUnit)?
				}
			},
			(BNC, TimeUnit::Round(current_round)) |
			(MOVR, TimeUnit::Round(current_round)) |
			(GLMR, TimeUnit::Round(current_round)) |
			(MANTA, TimeUnit::Round(current_round)) => {
				let mut delay = delays.unlock_delay;
				if if_leave {
					delay = delays.leave_delegators_delay;
				}
				if let TimeUnit::Round(delay_round) = delay {
					let unlock_round =
						current_round.checked_add(delay_round).ok_or(Error::<T>::OverFlow)?;
					TimeUnit::Round(unlock_round)
				} else {
					Err(Error::<T>::InvalidTimeUnit)?
				}
			},
			_ => Err(Error::<T>::InvalidTimeUnit)?,
		};

		Ok(Some(unlock_time_unit))
	}

	pub(crate) fn get_transfer_to_added_amount_and_supplement(
		from: AccountIdOf<T>,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<BalanceOf<T>, Error<T>> {
		// get transfer_to extra fee
		let (_weight, supplementary_fee) = T::XcmWeightAndFeeHandler::get_operation_weight_and_fee(
			currency_id,
			XcmOperationType::TransferTo,
		)
		.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		// transfer supplementary_fee from treasury to "from" account
		// get fee source first
		let (source_location, _reserved_fee) =
			FeeSources::<T>::get(currency_id).ok_or(Error::<T>::FeeSourceNotExist)?;
		let source_account = Self::native_multilocation_to_account(&source_location)?;

		// transfer supplementary_fee from treasury to "from" account
		T::MultiCurrency::transfer(currency_id, &source_account, &from, supplementary_fee)
			.map_err(|_| Error::<T>::Unexpected)?;
		let added_amount = amount.checked_add(&supplementary_fee).ok_or(Error::<T>::OverFlow)?;

		Ok(added_amount)
	}
}

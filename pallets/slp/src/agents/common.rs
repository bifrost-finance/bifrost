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
	pallet::Error,
	primitives::{
		ParachainStakingLedgerUpdateEntry, ParachainStakingLedgerUpdateOperation, OneToManyDelegationAction,
		OneToManyDelegatorStatus, OneToManyLedger, OneToManyScheduledRequest, TIMEOUT_BLOCKS,
	},
	traits::QueryResponseManager,
	vec, BalanceOf, BlockNumberFor, BoundedVec, Box, Config, CurrencyDelays, DelegationsOccupied,
	DelegatorLatestTuneRecord, DelegatorLedgerXcmUpdateQueue, DelegatorLedgers, DelegatorNextIndex,
	DelegatorsIndex2Multilocation, DelegatorsMultilocation2Index, Encode, Event, FeeSources,
	Junction::{AccountId32, Parachain},
	Junctions::{Here, X1},
	Ledger, LedgerUpdateEntry, MinimumsAndMaximums, MultiLocation, Pallet, TimeUnit, Validators,
	Weight, Xcm, XcmOperationType, XcmWeight, Zero, ASTR, DOT, GLMR, KSM, MANTA, MOVR, PHA,
};
use codec::alloc::collections::BTreeMap;
use frame_support::{ensure, traits::Len};
use node_primitives::{CurrencyId, VtokenMintingOperator, XcmDestWeightAndFeeHandler};
use orml_traits::MultiCurrency;
use polkadot_parachain::primitives::Sibling;
use sp_core::{Get, U256};
use sp_runtime::{
	traits::{
		AccountIdConversion, CheckedAdd, CheckedSub, UniqueSaturatedFrom, UniqueSaturatedInto,
	},
	DispatchResult, Saturating,
};
use xcm::{opaque::v3::Instruction, v3::prelude::*, VersionedMultiLocation};

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

	pub(crate) fn get_transfer_back_dest_and_beneficiary(
		from: &MultiLocation,
		to: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<(Box<VersionedMultiLocation>, Box<VersionedMultiLocation>), Error<T>> {
		// Check if from is one of our delegators. If not, return error.
		DelegatorsMultilocation2Index::<T>::get(currency_id, from)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Make sure the receiving account is the Exit_account from vtoken-minting module.
		let to_account_id = Self::multilocation_to_account(to)?;
		let (_, exit_account) = T::VtokenMinting::get_entrance_and_exit_accounts();
		ensure!(to_account_id == exit_account, Error::<T>::InvalidAccount);

		// Prepare parameter dest and beneficiary.
		let to_32: [u8; 32] = Self::multilocation_to_account_32(to)?;

		let dest = Box::new(VersionedMultiLocation::from(MultiLocation::from(X1(Parachain(
			T::ParachainId::get().into(),
		)))));

		let beneficiary =
			Box::new(VersionedMultiLocation::from(MultiLocation::from(X1(AccountId32 {
				network: None,
				id: to_32,
			}))));

		Ok((dest, beneficiary))
	}

	pub(crate) fn inner_do_transfer_to(
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
		assets: MultiAssets,
		dest: &MultiLocation,
	) -> Result<(), Error<T>> {
		// Ensure amount is greater than zero.
		ensure!(!amount.is_zero(), Error::<T>::AmountZero);

		// Ensure the from account is located within Bifrost chain. Otherwise, the xcm massage will
		// not succeed.
		ensure!(from.parents.is_zero(), Error::<T>::InvalidTransferSource);

		let (weight, fee_amount) = T::XcmWeightAndFeeHandler::get_operation_weight_and_fee(
			currency_id,
			XcmOperationType::TransferTo,
		)
		.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		// Prepare parameter beneficiary.
		let to_32: [u8; 32] = Self::multilocation_to_account_32(to)?;
		let beneficiary = Self::account_32_to_local_location(to_32)?;

		// Prepare fee asset.
		let fee_asset = MultiAsset {
			fun: Fungible(fee_amount.unique_saturated_into()),
			id: Concrete(MultiLocation { parents: 0, interior: Here }),
		};

		// prepare for xcm message
		let msg = Xcm(vec![
			WithdrawAsset(assets),
			InitiateReserveWithdraw {
				assets: All.into(),
				reserve: *dest,
				xcm: Xcm(vec![
					BuyExecution { fees: fee_asset, weight_limit: WeightLimit::Limited(weight) },
					DepositAsset { assets: AllCounted(1).into(), beneficiary },
				]),
			},
		]);
		let hash = msg.using_encoded(sp_io::hashing::blake2_256);
		// Execute the xcm message.
		T::XcmExecutor::execute_xcm_in_credit(*from, msg, hash, weight, weight)
			.ensure_complete()
			.map_err(|_| Error::<T>::XcmFailure)?;

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
		// ensure the fee source account has the balance of currency_id
		T::MultiCurrency::ensure_can_withdraw(currency_id, &source_account, fee)
			.map_err(|_| Error::<T>::NotEnoughBalance)?;

		// withdraw
		T::MultiCurrency::withdraw(currency_id, &source_account, fee)
			.map_err(|_| Error::<T>::NotEnoughBalance)?;

		Ok(())
	}

	pub(crate) fn inner_construct_xcm_message(
		currency_id: CurrencyId,
		extra_fee: BalanceOf<T>,
	) -> Result<Vec<Instruction>, Error<T>> {
		let multi = Self::get_currency_local_multilocation(currency_id);

		let asset =
			MultiAsset { id: Concrete(multi), fun: Fungible(extra_fee.unique_saturated_into()) };

		let interior = Self::get_interior_by_currency_id(currency_id);

		Ok(vec![
			WithdrawAsset(asset.clone().into()),
			BuyExecution { fees: asset, weight_limit: Unlimited },
			RefundSurplus,
			DepositAsset {
				assets: AllCounted(8).into(),
				beneficiary: MultiLocation { parents: 0, interior },
			},
		])
	}

	pub(crate) fn get_interior_by_currency_id(currency_id: CurrencyId) -> Junctions {
		let interior = match currency_id {
			KSM | DOT => X1(Parachain(T::ParachainId::get().into())),
			MOVR | GLMR => X1(AccountKey20 {
				network: None,
				key: Sibling::from(T::ParachainId::get()).into_account_truncating(),
			}),
			_ => X1(AccountId32 {
				network: None,
				id: Sibling::from(T::ParachainId::get()).into_account_truncating(),
			}),
		};

		return interior;
	}

	pub(crate) fn prepare_send_as_subaccount_call(
		operation: XcmOperationType,
		call: Vec<u8>,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<(Vec<u8>, BalanceOf<T>, XcmWeight), Error<T>> {
		// Get the delegator sub-account index.
		let sub_account_index = DelegatorsMultilocation2Index::<T>::get(currency_id, who)
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// call_as_subaccount = Utility(Box::new(AsDerivative(sub_account_index, Box::new(call))));
		let utility_call: u8 = match currency_id {
			MANTA => 10,
			MOVR | GLMR => 30,
			ASTR => 11,
			PHA => 3,
			_ => Err(Error::<T>::Unsupported)?,
		};

		let mut call_as_subaccount: Vec<u8> = utility_call.encode();
		// Since everyone use the same Utility pallet from Substrate repo, as_derivative function is
		// always indexed 1.
		call_as_subaccount.extend(1.encode());
		call_as_subaccount.extend(sub_account_index.encode());
		call_as_subaccount.extend(call);

		let (weight, fee) =
			T::XcmWeightAndFeeHandler::get_operation_weight_and_fee(currency_id, operation)
				.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		Ok((call_as_subaccount, fee, weight))
	}

	pub(crate) fn construct_xcm_as_subaccount_with_query_id(
		operation: XcmOperationType,
		call: Vec<u8>,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<(QueryId, BlockNumberFor<T>, BalanceOf<T>, Xcm<()>), Error<T>> {
		// prepare the query_id for reporting back transact status
		let now = frame_system::Pallet::<T>::block_number();
		let timeout = T::BlockNumber::from(TIMEOUT_BLOCKS).saturating_add(now);
		let query_id = Self::get_query_id(currency_id, &operation)?;

		let (call_as_subaccount, fee, weight) =
			Self::prepare_send_as_subaccount_call(operation, call, who, currency_id)?;

		let xcm_message = Self::construct_xcm_message(
			call_as_subaccount,
			fee,
			weight,
			currency_id,
			Some(query_id),
		)?;

		Ok((query_id, timeout, fee, xcm_message))
	}

	pub(crate) fn get_query_id(
		currency_id: CurrencyId,
		operation: &XcmOperationType,
	) -> Result<QueryId, Error<T>> {
		let now = frame_system::Pallet::<T>::block_number();
		let timeout = T::BlockNumber::from(TIMEOUT_BLOCKS).saturating_add(now);
		let responder = Self::get_para_multilocation_by_currency_id(currency_id)?;

		let callback_option = match (currency_id, operation) {
			(PHA, _) => None,
			(DOT, &XcmOperationType::Delegate) |
			(DOT, &XcmOperationType::Undelegate) |
			(KSM, &XcmOperationType::Delegate) |
			(KSM, &XcmOperationType::Undelegate) => Some(Self::confirm_validators_by_delegator_call()),
			_ => Some(Self::confirm_delegator_ledger_call()),
		};

		let query_id =
			T::SubstrateResponseManager::create_query_record(&responder, callback_option, timeout);

		return Ok(query_id);
	}

	pub(crate) fn construct_xcm_and_send_as_subaccount_without_query_id(
		operation: XcmOperationType,
		call: Vec<u8>,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<BalanceOf<T>, Error<T>> {
		let (call_as_subaccount, fee, weight) =
			Self::prepare_send_as_subaccount_call(operation, call, who, currency_id)?;

		let xcm_message =
			Self::construct_xcm_message(call_as_subaccount, fee, weight, currency_id, None)?;

		let dest = Self::get_para_multilocation_by_currency_id(currency_id)?;
		send_xcm::<T::XcmRouter>(dest, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(fee)
	}

	pub(crate) fn get_report_transact_status_instruct(
		query_id: QueryId,
		max_weight: Weight,
	) -> Instruction {
		ReportTransactStatus(QueryResponseInfo {
			destination: MultiLocation::new(1, X1(Parachain(u32::from(T::ParachainId::get())))),
			query_id,
			max_weight,
		})
	}

	pub(crate) fn get_unlocking_round_from_current(
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
			BondLess | Revoke => Pallet::<T>::get_unlocking_round_from_current(false, currency_id)?,
			LeaveDelegator => Pallet::<T>::get_unlocking_round_from_current(true, currency_id)?,
			ExecuteRequest | ExecuteLeave => T::VtokenMinting::get_ongoing_time_unit(currency_id),
			_ => None,
		};

		let entry = LedgerUpdateEntry::Moonbeam(ParachainStakingLedgerUpdateEntry {
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

		let (weight, fee_amount) = T::XcmWeightAndFeeHandler::get_operation_weight_and_fee(
			currency_id,
			XcmOperationType::TransferTo,
		)
		.ok_or(Error::<T>::WeightAndFeeNotExists)?;

		// Prepare parameter dest and beneficiary.
		let dest = Pallet::<T>::get_para_multilocation_by_currency_id(currency_id)?;
		let beneficiary = Pallet::<T>::multilocation_to_local_multilocation(to)?;

		let currency_location = Pallet::<T>::get_currency_full_multilocation(currency_id)?;
		// Prepare parameter assets.
		let asset = MultiAsset {
			fun: Fungible(amount.unique_saturated_into()),
			id: Concrete(currency_location),
		};
		let assets = MultiAssets::from(asset);

		// Prepare fee asset.
		let currency_local_location = Pallet::<T>::get_currency_local_multilocation(currency_id);
		let fee_asset = MultiAsset {
			fun: Fungible(fee_amount.unique_saturated_into()),
			id: Concrete(currency_local_location),
		};

		// prepare for xcm message
		let msg = Xcm(vec![
			WithdrawAsset(assets.clone()),
			InitiateReserveWithdraw {
				assets: All.into(),
				reserve: dest,
				xcm: Xcm(vec![
					BuyExecution { fees: fee_asset, weight_limit: Limited(weight) },
					DepositAsset { assets: AllCounted(1).into(), beneficiary },
				]),
			},
		]);
		let hash = msg.using_encoded(sp_io::hashing::blake2_256);
		// Execute the xcm message.
		T::XcmExecutor::execute_xcm_in_credit(*from, msg, hash, weight, weight)
			.ensure_complete()
			.map_err(|_| Error::<T>::XcmFailure)?;

		Ok(())
	}

	pub(crate) fn update_ledger_query_response_storage(
		query_id: QueryId,
		query_entry: LedgerUpdateEntry<BalanceOf<T>>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		use ParachainStakingLedgerUpdateOperation::{
			Bond, BondLess, CancelLeave, CancelRequest, ExecuteLeave, ExecuteRequest,
			LeaveDelegator, Revoke,
		};
		// update DelegatorLedgers<T> storage
		if let LedgerUpdateEntry::Moonbeam(ParachainStakingLedgerUpdateEntry {
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
					if let Some(Ledger::Moonbeam(ref mut old_ledger)) = old_ledger_opt {
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
								let moonbeam_ledger = Ledger::<BalanceOf<T>>::Moonbeam(new_ledger);

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

	pub(crate) fn update_all_occupied_status_storage(
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		let mut all_occupied = true;

		for (_, ledger) in DelegatorLedgers::<T>::iter_prefix(currency_id) {
			if let Ledger::Moonbeam(moonbeam_ledger) = ledger {
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
		weight: XcmWeight,
		currency_id: CurrencyId,
		query_id: Option<QueryId>,
	) -> Result<Xcm<()>, Error<T>> {
		let mut xcm_message = Self::inner_construct_xcm_message(currency_id, extra_fee)?;
		let transact = Transact {
			origin_kind: OriginKind::SovereignAccount,
			require_weight_at_most: weight,
			call: call.into(),
		};
		xcm_message.insert(2, transact);
		if let Some(query_id) = query_id {
			let report_transact_status_instruct =
				Self::get_report_transact_status_instruct(query_id, weight);
			xcm_message.insert(3, report_transact_status_instruct);
		}
		Ok(Xcm(xcm_message))
	}
}

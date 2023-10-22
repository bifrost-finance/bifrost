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
	pallet::{Error, Event},
	primitives::{FilecoinLedger, Ledger},
	traits::StakingAgent,
	AccountIdOf, BalanceOf, BoundedVec, Config, CurrencyLatestTuneRecord, DelegatorLedgers,
	LedgerUpdateEntry, MinimumsAndMaximums, MultiLocation, Pallet, TimeUnit, Validators,
	ValidatorsByDelegator, ValidatorsByDelegatorUpdateEntry,
};
use core::marker::PhantomData;
pub use cumulus_primitives_core::ParaId;
use frame_support::ensure;
use node_primitives::{
	BridgeOperator, CurrencyId, VtokenMintingOperator, XcmOperationType, CROSSCHAIN_ACCOUNT_LENGTH,
	CROSSCHAIN_AMOUNT_LENGTH, CROSSCHAIN_CURRENCY_ID_LENGTH, CROSSCHAIN_OPERATION_LENGTH, FIL,
};
use orml_traits::MultiCurrency;
use sp_core::{Get, U256};
use sp_runtime::{
	traits::{CheckedAdd, CheckedDiv, CheckedSub, UniqueSaturatedFrom, Zero},
	DispatchResult, SaturatedConversion,
};
use sp_std::prelude::*;
use xcm::v3::prelude::*;

/// StakingAgent implementation for Filecoin
pub struct FilecoinAgent<T>(PhantomData<T>);

impl<T> FilecoinAgent<T> {
	pub fn new() -> Self {
		FilecoinAgent(PhantomData::<T>)
	}
}

impl<T: Config>
	StakingAgent<
		BalanceOf<T>,
		AccountIdOf<T>,
		LedgerUpdateEntry<BalanceOf<T>>,
		ValidatorsByDelegatorUpdateEntry,
		Error<T>,
	> for FilecoinAgent<T>
{
	// In filecoin world, delegator means miner. Validator is used to store worker info.
	fn initialize_delegator(
		&self,
		currency_id: CurrencyId,
		delegator_location_op: Option<Box<MultiLocation>>,
	) -> Result<MultiLocation, Error<T>> {
		// Filecoin delegator(miner) account is passed in, not automatically generated.
		let delegator_multilocation = delegator_location_op.ok_or(Error::<T>::NotExist)?;
		let new_delegator_id = Pallet::<T>::inner_initialize_delegator(currency_id)?;

		// Add the new delegator into storage
		Pallet::<T>::inner_add_delegator(new_delegator_id, &delegator_multilocation, currency_id)
			.map_err(|_| Error::<T>::FailToAddDelegator)?;

		Ok(*delegator_multilocation)
	}

	/// First time stake some amount to a miner.
	/// Since Filecoin will bond after the real staking happens, it just needs to update the ledger.
	fn bond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
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

		// Check if the delegator(miner) has bonded an worker.
		let miners = ValidatorsByDelegator::<T>::get(currency_id, who)
			.ok_or(Error::<T>::ValidatorNotBonded)?;
		ensure!(miners.len() == 1, Error::<T>::VectorTooLong);

		// Create a new delegator ledger
		let ledger = FilecoinLedger::<BalanceOf<T>> { account: *who, initial_pledge: amount };
		let filecoin_ledger = Ledger::<BalanceOf<T>>::Filecoin(ledger);

		DelegatorLedgers::<T>::insert(currency_id, who, filecoin_ledger);
		let query_id = Zero::zero();

		Ok(query_id)
	}

	/// Bond extra amount to a delegator.
	/// Since Filecoin will bond after the real staking happens, it just needs to update the ledger.
	fn bond_extra(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.bond_extra_minimum, Error::<T>::LowerThanMinimum);

		if let Ledger::Filecoin(filecoin_ledger) = ledger {
			let initial_pledge = filecoin_ledger.initial_pledge;

			let total = amount.checked_add(&initial_pledge).ok_or(Error::<T>::OverFlow)?;
			ensure!(
				total <= mins_maxs.delegator_active_staking_maximum,
				Error::<T>::ExceedActiveMaximum
			);

			// update delegator ledger
			DelegatorLedgers::<T>::mutate(
				currency_id,
				who,
				|old_ledger| -> Result<(), Error<T>> {
					if let Some(Ledger::Filecoin(ref mut old_fil_ledger)) = old_ledger {
						old_fil_ledger.initial_pledge = old_fil_ledger
							.initial_pledge
							.checked_add(&amount)
							.ok_or(Error::<T>::OverFlow)?;
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)?
					}
				},
			)?;
		} else {
			Err(Error::<T>::Unexpected)?;
		}

		let query_id = Zero::zero();
		Ok(query_id)
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		if let Ledger::Filecoin(filecoin_ledger) = ledger {
			let initial_pledge = filecoin_ledger.initial_pledge;

			// Check if the unbonding amount exceeds minimum requirement.
			let mins_maxs =
				MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
			ensure!(amount >= mins_maxs.unbond_minimum, Error::<T>::LowerThanMinimum);

			let remaining =
				initial_pledge.checked_sub(&amount).ok_or(Error::<T>::NotEnoughToUnbond)?;
			ensure!(remaining >= mins_maxs.delegator_bonded_minimum, Error::<T>::NotEnoughToUnbond);

			// update delegator ledger
			DelegatorLedgers::<T>::mutate(
				currency_id,
				who,
				|old_ledger| -> Result<(), Error<T>> {
					if let Some(Ledger::Filecoin(ref mut old_fil_ledger)) = old_ledger {
						old_fil_ledger.initial_pledge = old_fil_ledger
							.initial_pledge
							.checked_sub(&amount)
							.ok_or(Error::<T>::OverFlow)?;
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)?
					}
				},
			)?;
		} else {
			Err(Error::<T>::Unexpected)?;
		}

		let query_id = Zero::zero();
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
		Err(Error::<T>::Unsupported)
	}

	/// One delegator(miner) can only map to a validator(worker), so targets vec can only contains 1
	/// item.
	fn delegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		ensure!(targets.len() == 1, Error::<T>::VectorTooLong);
		let worker = &targets[0];

		// Need to check whether this validator is in the whitelist.
		let validators_vec =
			Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;
		ensure!(validators_vec.contains(worker), Error::<T>::ValidatorNotExist);

		// ensure the length of validators_vec does not exceed the MaxLengthLimit.
		ensure!(
			validators_vec.len() <= T::MaxLengthLimit::get() as usize,
			Error::<T>::ExceedMaxLengthLimit
		);

		let validators_list =
			BoundedVec::try_from(vec![*worker]).map_err(|_| Error::<T>::FailToConvert)?;

		// update ledger
		ValidatorsByDelegator::<T>::insert(currency_id, *who, validators_list.clone());

		// query_id is nonsense for filecoin.
		let query_id = Zero::zero();

		// Deposit event.
		Pallet::<T>::deposit_event(Event::ValidatorsByDelegatorSet {
			currency_id,
			validators_list: validators_list.to_vec(),
			delegator_id: *who,
		});

		Ok(query_id)
	}

	/// Remove delegation relationship with some validators.
	fn undelegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		// Check if it is bonded already.
		ensure!(
			DelegatorLedgers::<T>::contains_key(currency_id, who),
			Error::<T>::DelegatorNotBonded
		);

		// Check if the delegator's ledger still has staking balance.
		// It can be undelegated only if there is none.
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		if let Ledger::Filecoin(filecoin_ledger) = ledger {
			let amount = filecoin_ledger.initial_pledge;
			ensure!(amount == Zero::zero(), Error::<T>::AmountNotZero);

			let validators_by_delegator_vec = ValidatorsByDelegator::<T>::get(currency_id, who)
				.ok_or(Error::<T>::ValidatorNotBonded)?;
			ensure!(targets[0] == validators_by_delegator_vec[0], Error::<T>::ValidatorError);

			// remove entry.
			ValidatorsByDelegator::<T>::remove(currency_id, who);
			// query_id is nonsense to filecoin.
			let query_id = Zero::zero();

			// deposit event
			Pallet::<T>::deposit_event(Event::ValidatorsByDelegatorSet {
				currency_id,
				validators_list: vec![],
				delegator_id: *who,
			});

			Ok(query_id)
		} else {
			Err(Error::<T>::Unexpected)?
		}
	}

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(
		&self,
		who: &MultiLocation,
		targets: &Option<Vec<MultiLocation>>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		let targets = targets.as_ref().ok_or(Error::<T>::ValidatorSetNotExist)?;
		let query_id = Self::delegate(self, who, targets, currency_id)?;
		Ok(query_id)
	}

	fn payout(
		&self,
		_who: &MultiLocation,
		_validator: &MultiLocation,
		_when: &Option<TimeUnit>,
		_currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn liquidize(
		&self,
		_who: &MultiLocation,
		_when: &Option<TimeUnit>,
		_validator: &Option<MultiLocation>,
		_currency_id: CurrencyId,
		_amount: Option<BalanceOf<T>>,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn chill(&self, _who: &MultiLocation, _currency_id: CurrencyId) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
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

	/// For filecoin, transfer_to means to mint from Bifrost network by delegating the
	/// delegate-staking contract. It does two things. Firstly, it cross-transfer corresponding
	/// amount from entrance_account to the delegate-staking contract in Filecoin network. Secondly,
	/// it send out cross message to call "goMint" function in the delegate-staking contract in
	/// Filecoin network to mint vtoken.
	fn transfer_to(
		&self,
		from: &MultiLocation,
		_to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// "from" account must be entrance account
		let from_account = Pallet::<T>::native_multilocation_to_account(from)?;
		let (entrance_account, _) = T::VtokenMinting::get_entrance_and_exit_accounts();
		ensure!(from_account == entrance_account, Error::<T>::InvalidAccount);

		// We don't use the sent-in "to".
		// "to" must be delegate-staking contract, which should be already mapped
		// to the entrance account in the storage AccountToOuterMultilocation
		let to_location = T::BridgeOperator::get_registered_outer_multilocation_from_account(
			FIL,
			entrance_account.clone(),
		)
		.map_err(|_| Error::<T>::MultilocationNotExist)?;

		// burn the transfer amount
		T::MultiCurrency::withdraw(currency_id, &entrance_account, amount)
			.map_err(|_e| Error::<T>::NotEnoughBalance)?;

		// set the fee_payer to be the hosting fee receiver, which is treasury account
		let (_, fee_payer_location) =
			Pallet::<T>::get_hosting_fee(currency_id).ok_or(Error::<T>::InvalidHostingFee)?;

		let fee_payer = Pallet::<T>::multilocation_to_account(&fee_payer_location)?;
		// first message, send transfer message
		Pallet::<T>::send_message(
			XcmOperationType::TransferTo,
			fee_payer.clone(),
			&to_location,
			amount,
			currency_id,
			currency_id,
		)?;

		// second message，to call the goMint method in delegate-staking contract
		Pallet::<T>::send_message(
			XcmOperationType::Mint,
			fee_payer,
			&to_location,
			amount,
			currency_id,
			currency_id,
		)?;

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

	/// For filecoin, instead of delegator(miner) account, "who" should be a
	/// validator(worker) account, since we tune extrange rate once per worker by
	/// aggregating all its miner accounts' interests.
	// Filecoin use TimeUnit::Kblock, which means 1000 blocks. Filecoin produces
	// one block per 30 seconds . Kblock takes around 8.33 hours.
	fn tune_vtoken_exchange_rate(
		&self,
		_who: &Option<MultiLocation>,
		nominator: BalanceOf<T>,
		denominator: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// ensure amount valid
		ensure!(nominator > Zero::zero(), Error::<T>::AmountZero);
		ensure!(denominator > Zero::zero(), Error::<T>::AmountZero);

		// ensure within tune limit of exchange rate
		let (_, max_permill) = Pallet::<T>::get_currency_tune_exchange_rate_limit(currency_id)
			.ok_or(Error::<T>::TuneExchangeRateLimitNotSet)?;
		ensure!(nominator <= max_permill.mul_floor(denominator), Error::<T>::GreaterThanMaximum);
		let (old_nominator, old_denominator) =
			T::VtokenMinting::get_special_vtoken_exchange_rate(currency_id)
				.ok_or(Error::<T>::ExchangeRateNotExist)?;
		ensure!(old_denominator > Zero::zero(), Error::<T>::AmountZero);

		// ensure new exchange rate is greater than old excahnge rate
		let old_rate = old_nominator.checked_div(&old_denominator).ok_or(Error::<T>::OverFlow)?;
		let new_rate = nominator.checked_div(&denominator).ok_or(Error::<T>::OverFlow)?;
		ensure!(new_rate >= old_rate, Error::<T>::LessThanOldExchangeRate);

		// Ensure this tune is within limit.
		// Get current TimeUnit.
		let current_time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
			.ok_or(Error::<T>::TimeUnitNotExist)?;
		// Check whether tuning times exceed limit. And get the new tune_num
		let new_tune_num = Pallet::<T>::check_tuning_limit(currency_id)?;
		// Update the CurrencyLatestTuneRecord<T> storage.
		CurrencyLatestTuneRecord::<T>::insert(currency_id, (current_time_unit, new_tune_num));

		// update SpecialVtokenExchangeRate
		T::VtokenMinting::update_special_vtoken_exchange_rate(
			currency_id,
			Some((nominator, denominator)),
		)
		.map_err(|_| Error::<T>::FailToUpdateExchangeRate)?;

		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		// Get the delegator ledger
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		if let Ledger::Filecoin(filecoin_ledger) = ledger {
			let initial_pledge = filecoin_ledger.initial_pledge;

			// Check if ledger initial_pledge amount is zero. If not, return error.
			ensure!(initial_pledge.is_zero(), Error::<T>::AmountNotZero);
		} else {
			Err(Error::<T>::Unexpected)?;
		}

		Pallet::<T>::inner_remove_delegator(who, currency_id)
	}

	/// FIL not supporting charge hosting fee on Bifrost network. It is charged on Filecoin network.
	fn charge_hosting_fee(
		&self,
		_amount: BalanceOf<T>,
		_from: &MultiLocation,
		_to: &MultiLocation,
		_currency_id: CurrencyId,
	) -> DispatchResult {
		Err(Error::<T>::Unsupported)?;
		Ok(())
	}

	/// For Filecoin, from and to should be accounts in Bifrost network.
	/// `From` is treasury account, `to` should be an account which is registered in
	/// AccountToOuterMultilocation storage.
	fn supplement_fee_reserve(
		&self,
		amount: BalanceOf<T>,
		from: &MultiLocation,
		to: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// Ensure `to` registered in AccountToOuterMultilocation storage.
		let to_account = Pallet::<T>::native_multilocation_to_account(&to)?;
		let dest_location = T::BridgeOperator::get_registered_outer_multilocation_from_account(
			currency_id,
			to_account.clone(),
		)
		.map_err(|_| Error::<T>::MultilocationNotExist)?;

		let source_account = Pallet::<T>::native_multilocation_to_account(&from)?;

		// cross out transfer from dest_account to dest location in Filecoin network
		// burn the transfer amount
		T::MultiCurrency::withdraw(currency_id, &source_account, amount)
			.map_err(|_e| Error::<T>::NotEnoughBalance)?;

		// send transfer message
		Pallet::<T>::send_message(
			XcmOperationType::TransferTo,
			source_account,
			&dest_location,
			amount,
			currency_id,
			currency_id,
		)?;

		Ok(())
	}

	fn check_delegator_ledger_query_response(
		&self,
		_query_id: QueryId,
		_entry: LedgerUpdateEntry<BalanceOf<T>>,
		_manual_mode: bool,
		_currency_id: CurrencyId,
	) -> Result<bool, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn check_validators_by_delegator_query_response(
		&self,
		_query_id: QueryId,
		_entry: ValidatorsByDelegatorUpdateEntry,
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

	fn execute_crosschain_operation(
		&self,
		currency_id: CurrencyId,
		payload: &[u8],
	) -> Result<(), Error<T>> {
		// decode XcmOperationType from the first 32 bytes
		let operation_u8: u8 = U256::from_big_endian(&payload[0..32])
			.try_into()
			.map_err(|_| Error::<T>::FailToConvert)?;
		let operation: XcmOperationType =
			XcmOperationType::try_from(operation_u8).map_err(|_| Error::<T>::FailToConvert)?;

		match operation {
			XcmOperationType::UpdateDelegatorLedger => {
				let max_len = CROSSCHAIN_OPERATION_LENGTH +
					CROSSCHAIN_CURRENCY_ID_LENGTH +
					CROSSCHAIN_AMOUNT_LENGTH +
					CROSSCHAIN_ACCOUNT_LENGTH;
				ensure!(payload.len() == max_len, Error::<T>::InvalidPayloadLength);
				Self::renew_delegator_ledger(self, currency_id, &payload)
			},
			XcmOperationType::PassExchangeRateBack => {
				let max_len = CROSSCHAIN_OPERATION_LENGTH +
					CROSSCHAIN_CURRENCY_ID_LENGTH +
					2 * CROSSCHAIN_AMOUNT_LENGTH;
				ensure!(payload.len() == max_len, Error::<T>::InvalidPayloadLength);
				Self::update_exchange_rate(self, &payload)
			},
			_ => Err(Error::<T>::InvalidXcmOperation),
		}?;

		Ok(())
	}
}

impl<T: Config> FilecoinAgent<T> {
	fn renew_delegator_ledger(
		&self,
		currency_id: CurrencyId,
		payload: &[u8],
	) -> Result<(), Error<T>> {
		// get initial_pledge
		let initial_pledge: u128 = U256::from_big_endian(&payload[64..96])
			.try_into()
			.map_err(|_| Error::<T>::FailToConvert)?;
		let initial_pledge: BalanceOf<T> = BalanceOf::<T>::unique_saturated_from(initial_pledge);

		let miner_actor_id: u64 = U256::from_big_endian(&payload[96..128])
			.try_into()
			.map_err(|_| Error::<T>::FailToConvert)?;

		// transform account into MultiLocation
		let filecoin_multilocation =
			Pallet::<T>::filecoin_miner_id_to_multilocation(miner_actor_id)?;

		// renew delegator ledger
		let ledger = DelegatorLedgers::<T>::get(currency_id, &filecoin_multilocation);

		if ledger.is_none() {
			Self::bond(self, &filecoin_multilocation, initial_pledge, &None, currency_id)?;
		} else {
			if let Some(Ledger::Filecoin(filecoin_ledger)) = ledger {
				let original_initial_pledge = filecoin_ledger.initial_pledge;
				if original_initial_pledge < initial_pledge {
					let bond_extra_amount = initial_pledge
						.checked_sub(&original_initial_pledge)
						.ok_or(Error::<T>::OverFlow)?;
					Self::bond_extra(
						self,
						&filecoin_multilocation,
						bond_extra_amount,
						&None,
						currency_id,
					)?;
				} else if original_initial_pledge > initial_pledge {
					let unbond_amount = original_initial_pledge
						.checked_sub(&initial_pledge)
						.ok_or(Error::<T>::OverFlow)?;
					Self::unbond(self, &filecoin_multilocation, unbond_amount, &None, currency_id)?;
				}
			} else {
				Err(Error::<T>::Unexpected)?
			}
		}

		Ok(())
	}

	pub fn update_exchange_rate(&self, payload: &[u8]) -> Result<(), Error<T>> {
		// get currency_id from payload. The second 32 bytes are currency_id.
		let currency_id_u64: u64 = U256::from_big_endian(&payload[32..64])
			.try_into()
			.map_err(|_| Error::<T>::FailToConvert)?;
		let currency_id =
			CurrencyId::try_from(currency_id_u64).map_err(|_| Error::<T>::FailToConvert)?;

		ensure!(currency_id == FIL, Error::<T>::NotSupportedCurrencyId);

		// get exchange_rate nominator and denominator from payload
		let nominator: u128 = U256::from_big_endian(&payload[64..96])
			.try_into()
			.map_err(|_| Error::<T>::FailToConvert)?;
		let nominator: BalanceOf<T> = nominator.saturated_into::<BalanceOf<T>>();

		let denominator: u128 = U256::from_big_endian(&payload[96..128])
			.try_into()
			.map_err(|_| Error::<T>::FailToConvert)?;
		let denominator: BalanceOf<T> = denominator.saturated_into::<BalanceOf<T>>();

		Self::tune_vtoken_exchange_rate(self, &None, nominator, denominator, currency_id)?;

		Ok(())
	}
}

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
	AccountIdOf, BalanceOf, Config, DelegatorLatestTuneRecord, DelegatorLedgers,
	DelegatorsMultilocation2Index, Hash, LedgerUpdateEntry, MinimumsAndMaximums, MultiLocation,
	Pallet, TimeUnit, Validators, ValidatorsByDelegator, ValidatorsByDelegatorUpdateEntry,
};
use codec::Encode;
use core::marker::PhantomData;
use cumulus_primitives_core::relay_chain::HashT;
pub use cumulus_primitives_core::ParaId;
use frame_support::ensure;
use node_primitives::{CurrencyId, VtokenMintingOperator};
use orml_traits::MultiCurrency;
use sp_runtime::{
	traits::{CheckedAdd, CheckedSub, Zero},
	DispatchResult,
};
use sp_std::prelude::*;
use xcm::latest::prelude::*;

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
		ValidatorsByDelegatorUpdateEntry<Hash<T>>,
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
		Self::add_delegator(self, new_delegator_id, &delegator_multilocation, currency_id)
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
		let ledger =
			FilecoinLedger::<BalanceOf<T>> { account: who.clone(), initial_pledge: amount };
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
		let multi_hash = T::Hashing::hash(&worker.encode());
		ensure!(
			validators_vec.contains(&(worker.clone(), multi_hash)),
			Error::<T>::ValidatorNotExist
		);

		let validators_list = vec![(worker.clone(), multi_hash)];
		// update ledger
		ValidatorsByDelegator::<T>::insert(currency_id, who.clone(), validators_list.clone());

		// query_id is nonsense for filecoin.
		let query_id = Zero::zero();

		// Deposit event.
		Pallet::<T>::deposit_event(Event::ValidatorsByDelegatorSet {
			currency_id,
			validators_list,
			delegator_id: who.clone(),
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
			ensure!(targets[0] == validators_by_delegator_vec[0].0, Error::<T>::ValidatorError);

			// remove entry.
			ValidatorsByDelegator::<T>::remove(currency_id, who);
			// query_id is nonsense to filecoin.
			let query_id = Zero::zero();

			// deposit event
			Pallet::<T>::deposit_event(Event::ValidatorsByDelegatorSet {
				currency_id,
				validators_list: vec![],
				delegator_id: who.clone(),
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
	) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn liquidize(
		&self,
		_who: &MultiLocation,
		_when: &Option<TimeUnit>,
		_validator: &Option<MultiLocation>,
		_currency_id: CurrencyId,
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

	/// For filecoin, transfer_to means transfering newly minted amount to miner
	/// accounts. It actually burn/withdraw the corresponding amount from entrance_account.
	fn transfer_to(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// "from" account must be entrance account
		let from_account = Pallet::<T>::native_multilocation_to_account(from)?;
		let (entrance_account, _) = T::VtokenMinting::get_entrance_and_exit_accounts();
		ensure!(from_account == entrance_account, Error::<T>::InvalidAccount);

		// "to" account must be one of the delegators(miners) accounts
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(currency_id, to),
			Error::<T>::DelegatorNotExist
		);

		// burn the amount
		T::MultiCurrency::withdraw(currency_id, &entrance_account, amount)
			.map_err(|_e| Error::<T>::NotEnoughBalance)?;

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
		who: &Option<MultiLocation>,
		token_amount: BalanceOf<T>,
		_vtoken_amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		let who = who.as_ref().ok_or(Error::<T>::DelegatorNotExist)?;
		let multi_hash = T::Hashing::hash(&who.encode());

		// ensure "who" is a valid validator
		if let Some(validator_vec) = Validators::<T>::get(currency_id) {
			ensure!(
				validator_vec.contains(&(who.clone(), multi_hash)),
				Error::<T>::ValidatorNotExist
			);
		} else {
			Err(Error::<T>::ValidatorNotExist)?;
		}

		// Get current TimeUnit.
		let current_time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
			.ok_or(Error::<T>::TimeUnitNotExist)?;
		// Get DelegatorLatestTuneRecord for the currencyId.
		let latest_time_unit_op = DelegatorLatestTuneRecord::<T>::get(currency_id, &who);
		// ensure each delegator can only tune once per TimeUnit at most.
		ensure!(
			latest_time_unit_op != Some(current_time_unit.clone()),
			Error::<T>::DelegatorAlreadyTuned
		);

		ensure!(!token_amount.is_zero(), Error::<T>::AmountZero);

		// issue the increased interest amount to the entrance account
		// Get charged fee value
		let (fee_permill, _beneficiary) =
			Pallet::<T>::get_hosting_fee(currency_id).ok_or(Error::<T>::InvalidHostingFee)?;
		let fee_to_charge = fee_permill.mul_floor(token_amount);
		let amount_to_increase =
			token_amount.checked_sub(&fee_to_charge).ok_or(Error::<T>::UnderFlow)?;

		if amount_to_increase > Zero::zero() {
			// Tune the vtoken exchange rate.
			T::VtokenMinting::increase_token_pool(currency_id, amount_to_increase)
				.map_err(|_| Error::<T>::IncreaseTokenPoolError)?;

			// Deposit token to entrance account
			let (entrance_account, _) = T::VtokenMinting::get_entrance_and_exit_accounts();
			T::MultiCurrency::deposit(currency_id, &entrance_account, amount_to_increase)
				.map_err(|_e| Error::<T>::MultiCurrencyError)?;

			// Update the DelegatorLatestTuneRecord<T> storage.
			DelegatorLatestTuneRecord::<T>::insert(currency_id, who, current_time_unit);
		}

		Ok(())
	}

	/// Add a new serving delegator for a particular currency.
	fn add_delegator(
		&self,
		index: u16,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> DispatchResult {
		Pallet::<T>::inner_add_delegator(index, who, currency_id)
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

	/// Add a new worker for a particular currency.
	fn add_validator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		Pallet::<T>::inner_add_validator(who, currency_id)
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_validator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		let multi_hash = T::Hashing::hash(&who.encode());

		//  Check if ValidatorsByDelegator<T> involves this validator. If yes, return error.
		for validator_list in ValidatorsByDelegator::<T>::iter_prefix_values(currency_id) {
			if validator_list.contains(&(who.clone(), multi_hash)) {
				Err(Error::<T>::ValidatorStillInUse)?;
			}
		}
		// Update corresponding storage.
		Pallet::<T>::inner_remove_validator(who, currency_id)
	}

	/// Charge hosting fee.
	fn charge_hosting_fee(
		&self,
		amount: BalanceOf<T>,
		_from: &MultiLocation,
		to: &MultiLocation,
		currency_id: CurrencyId,
	) -> DispatchResult {
		Pallet::<T>::inner_charge_hosting_fee(amount, to, currency_id)
	}

	/// Deposit some amount as fee to nominator accounts.
	fn supplement_fee_reserve(
		&self,
		_amount: BalanceOf<T>,
		_from: &MultiLocation,
		_to: &MultiLocation,
		_currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
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
		_entry: ValidatorsByDelegatorUpdateEntry<Hash<T>>,
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
}

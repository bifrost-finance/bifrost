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
	pallet::Error, vec, BalanceOf, Config, DelegatorLedgers, DelegatorNextIndex,
	DelegatorsIndex2Multilocation, DelegatorsMultilocation2Index, Encode, MinimumsAndMaximums,
	MultiLocation, Pallet, Validators, Zero,
};
use cumulus_primitives_core::relay_chain::HashT;
use frame_support::{ensure, traits::Len};
use node_primitives::{CurrencyId, VtokenMintingOperator};
use orml_traits::MultiCurrency;
use sp_core::U256;
use sp_runtime::{
	traits::{UniqueSaturatedFrom, UniqueSaturatedInto},
	DispatchResult,
};

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
		let multi_hash = T::Hashing::hash(&who.encode());
		// Check if the validator already exists.
		let validators_set = Validators::<T>::get(currency_id);

		// Ensure validator candidates in the whitelist is not greater than maximum.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(
			validators_set.len() as u16 <= mins_maxs.validators_maximum,
			Error::<T>::GreaterThanMaximum
		);

		if validators_set.is_none() {
			Validators::<T>::insert(currency_id, vec![(who, multi_hash)]);
		} else {
			// Change corresponding storage.
			Validators::<T>::mutate(currency_id, |validator_vec| -> Result<(), Error<T>> {
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

		let multi_hash = T::Hashing::hash(&who.encode());
		ensure!(validators_set.contains(&(who.clone(), multi_hash)), Error::<T>::ValidatorNotExist);

		// Update corresponding storage.
		Validators::<T>::mutate(currency_id, |validator_vec| {
			if let Some(ref mut validator_list) = validator_vec {
				let rs = validator_list.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash);

				if let Ok(index) = rs {
					validator_list.remove(index);
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

		let beneficiary = Pallet::<T>::multilocation_to_account(&to)?;
		// Issue corresponding vksm to beneficiary account.
		T::MultiCurrency::deposit(depoist_currency, &beneficiary, charge_amount)?;

		Ok(())
	}
}

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
	blake2_256, pallet::Error, AccountIdOf, Config, Decode, DelegatorLedgerXcmUpdateQueue, Hash,
	LedgerUpdateEntry, Pallet, TrailingZeroInput, Validators, ValidatorsByDelegatorUpdateEntry,
	ValidatorsByDelegatorXcmUpdateQueue, H160,
};
use codec::Encode;
use cumulus_primitives_core::relay_chain::HashT;
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get};
use node_primitives::CurrencyId;
use sp_std::prelude::*;
use xcm::{
	latest::prelude::*,
	opaque::latest::{
		Junction::{AccountId32, Parachain},
		Junctions::X1,
		MultiLocation,
	},
};

// Some untilities.
impl<T: Config> Pallet<T> {
	/// Convert native multiLocation to account.
	pub fn native_multilocation_to_account(
		who: &MultiLocation,
	) -> Result<AccountIdOf<T>, Error<T>> {
		// Get the delegator account id in Kusama/Polkadot network
		let account_32 = match who {
			MultiLocation {
				parents: 0,
				interior: X1(AccountId32 { network: _network_id, id: account_id }),
			} => account_id,
			_ => Err(Error::<T>::AccountNotExist)?,
		};

		let account =
			T::AccountId::decode(&mut &account_32[..]).map_err(|_| Error::<T>::DecodingError)?;

		Ok(account)
	}

	pub fn sort_validators_and_remove_duplicates(
		currency_id: CurrencyId,
		validators: &Vec<MultiLocation>,
	) -> Result<Vec<(MultiLocation, Hash<T>)>, Error<T>> {
		let validators_set =
			Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;
		let mut validators_list: Vec<(MultiLocation, Hash<T>)> = vec![];
		for validator in validators.iter() {
			// Check if the validator is in the validator whitelist
			let multi_hash = <T as frame_system::Config>::Hashing::hash(&validator.encode());
			ensure!(
				validators_set.contains(&(validator.clone(), multi_hash)),
				Error::<T>::ValidatorNotExist
			);

			// sort the validators and remove duplicates
			let rs = validators_list.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash);

			if let Err(index) = rs {
				validators_list.insert(index, (validator.clone(), multi_hash));
			}
		}

		Ok(validators_list)
	}

	pub fn multilocation_to_account(who: &MultiLocation) -> Result<AccountIdOf<T>, Error<T>> {
		// Get the delegator account id in Kusama/Polkadot network
		let account_32 = Self::multilocation_to_account_32(who)?;
		let account =
			T::AccountId::decode(&mut &account_32[..]).map_err(|_| Error::<T>::DecodingError)?;
		Ok(account)
	}

	pub fn multilocation_to_account_32(who: &MultiLocation) -> Result<[u8; 32], Error<T>> {
		// Get the delegator account id in Kusama/Polkadot network
		let account_32 = match who {
			MultiLocation {
				parents: _,
				interior: X1(AccountId32 { network: _network_id, id: account_id }),
			} => account_id,
			_ => Err(Error::<T>::AccountNotExist)?,
		};
		Ok(*account_32)
	}

	pub fn account_id_to_account_32(account_id: AccountIdOf<T>) -> Result<[u8; 32], Error<T>> {
		let account_32 = T::AccountId::encode(&account_id)
			.try_into()
			.map_err(|_| Error::<T>::EncodingError)?;

		Ok(account_32)
	}

	pub fn account_32_to_local_location(account_32: [u8; 32]) -> Result<MultiLocation, Error<T>> {
		let local_location = MultiLocation {
			parents: 0,
			interior: X1(AccountId32 { network: Any, id: account_32 }),
		};

		Ok(local_location)
	}

	pub fn multilocation_to_local_multilocation(
		location: &MultiLocation,
	) -> Result<MultiLocation, Error<T>> {
		let inside: Junction = match location {
			MultiLocation {
				parents: _p,
				interior: X2(Parachain(_para_id), AccountId32 { network: Any, id: account_32 }),
			} => AccountId32 { network: Any, id: *account_32 },
			MultiLocation {
				parents: _p,
				interior: X2(Parachain(_para_id), AccountKey20 { network: Any, key: account_20 }),
			} => AccountKey20 { network: Any, key: *account_20 },
			MultiLocation {
				parents: _p,
				interior: X1(AccountId32 { network: Any, id: account_32 }),
			} => AccountId32 { network: Any, id: *account_32 },
			_ => Err(Error::<T>::Unsupported)?,
		};

		let local_location = MultiLocation { parents: 0, interior: X1(inside) };

		Ok(local_location)
	}

	pub fn account_32_to_parent_location(account_32: [u8; 32]) -> Result<MultiLocation, Error<T>> {
		let parent_location = MultiLocation {
			parents: 1,
			interior: X1(AccountId32 { network: Any, id: account_32 }),
		};

		Ok(parent_location)
	}

	pub fn account_32_to_parachain_location(
		account_32: [u8; 32],
		chain_id: u32,
	) -> Result<MultiLocation, Error<T>> {
		let parachain_location = MultiLocation {
			parents: 1,
			interior: X2(Parachain(chain_id), AccountId32 { network: Any, id: account_32 }),
		};

		Ok(parachain_location)
	}

	pub fn multilocation_to_account_20(who: &MultiLocation) -> Result<[u8; 20], Error<T>> {
		// Get the delegator account id in Moonriver/Moonbeam network
		let account_20 = match who {
			MultiLocation {
				parents: _,
				interior: X2(Parachain(_), AccountKey20 { network: _network_id, key: account_id }),
			} => account_id,
			_ => Err(Error::<T>::AccountNotExist)?,
		};
		Ok(*account_20)
	}

	pub fn multilocation_to_h160_account(who: &MultiLocation) -> Result<H160, Error<T>> {
		// Get the delegator account id in Moonriver/Moonbeam network
		let account_20 = Self::multilocation_to_account_20(who)?;
		let account_h160 =
			H160::decode(&mut &account_20[..]).map_err(|_| Error::<T>::DecodingError)?;
		Ok(account_h160)
	}

	/// **************************************/
	/// ****** XCM confirming Functions ******/
	/// **************************************/
	pub fn process_query_entry_records() -> Result<u32, Error<T>> {
		let mut counter = 0u32;

		// Deal with DelegatorLedgerXcmUpdateQueue storage
		for query_id in DelegatorLedgerXcmUpdateQueue::<T>::iter_keys() {
			if counter >= T::MaxTypeEntryPerBlock::get() {
				break;
			}

			let updated = Self::get_ledger_update_agent_then_process(query_id, false)?;
			if updated {
				counter = counter.saturating_add(1);
			}
		}

		// Deal with ValidatorsByDelegator storage
		for query_id in ValidatorsByDelegatorXcmUpdateQueue::<T>::iter_keys() {
			if counter >= T::MaxTypeEntryPerBlock::get() {
				break;
			}
			let updated =
				Self::get_validators_by_delegator_update_agent_then_process(query_id, false)?;

			if updated {
				counter = counter.saturating_add(1);
			}
		}

		Ok(counter)
	}

	pub fn get_ledger_update_agent_then_process(
		query_id: QueryId,
		manual_mode: bool,
	) -> Result<bool, Error<T>> {
		// See if the query exists. If it exists, call corresponding chain storage update
		// function.
		let (entry, timeout) =
			Self::get_delegator_ledger_update_entry(query_id).ok_or(Error::<T>::QueryNotExist)?;

		let now = frame_system::Pallet::<T>::block_number();
		let mut updated = true;
		if now <= timeout {
			let currency_id = match entry.clone() {
				LedgerUpdateEntry::Substrate(substrate_entry) => Some(substrate_entry.currency_id),
				LedgerUpdateEntry::Moonbeam(moonbeam_entry) => Some(moonbeam_entry.currency_id),
				LedgerUpdateEntry::ParachainStaking(parachain_staking_entry) =>
					Some(parachain_staking_entry.currency_id),
			}
			.ok_or(Error::<T>::NotSupportedCurrencyId)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			updated = staking_agent.check_delegator_ledger_query_response(
				query_id,
				entry,
				manual_mode,
				currency_id,
			)?;
		} else {
			Self::do_fail_delegator_ledger_query_response(query_id)?;
		}

		Ok(updated)
	}

	pub fn get_validators_by_delegator_update_agent_then_process(
		query_id: QueryId,
		manual_mode: bool,
	) -> Result<bool, Error<T>> {
		// See if the query exists. If it exists, call corresponding chain storage update
		// function.
		let (entry, timeout) = Self::get_validators_by_delegator_update_entry(query_id)
			.ok_or(Error::<T>::QueryNotExist)?;

		let now = frame_system::Pallet::<T>::block_number();
		let mut updated = true;
		if now <= timeout {
			let currency_id = match entry.clone() {
				ValidatorsByDelegatorUpdateEntry::Substrate(substrate_entry) =>
					Some(substrate_entry.currency_id),
			}
			.ok_or(Error::<T>::NotSupportedCurrencyId)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			updated = staking_agent.check_validators_by_delegator_query_response(
				query_id,
				entry,
				manual_mode,
			)?;
		} else {
			Self::do_fail_validators_by_delegator_query_response(query_id)?;
		}
		Ok(updated)
	}

	pub(crate) fn do_fail_delegator_ledger_query_response(
		query_id: QueryId,
	) -> Result<(), Error<T>> {
		// See if the query exists. If it exists, call corresponding chain storage update
		// function.
		let (entry, _) =
			Self::get_delegator_ledger_update_entry(query_id).ok_or(Error::<T>::QueryNotExist)?;
		let currency_id = match entry {
			LedgerUpdateEntry::Substrate(substrate_entry) => Some(substrate_entry.currency_id),
			LedgerUpdateEntry::Moonbeam(moonbeam_entry) => Some(moonbeam_entry.currency_id),
			LedgerUpdateEntry::ParachainStaking(parachain_staking_entry) =>
				Some(parachain_staking_entry.currency_id),
		}
		.ok_or(Error::<T>::NotSupportedCurrencyId)?;

		let staking_agent = Self::get_currency_staking_agent(currency_id)?;
		staking_agent.fail_delegator_ledger_query_response(query_id)?;

		Ok(())
	}

	pub(crate) fn do_fail_validators_by_delegator_query_response(
		query_id: QueryId,
	) -> Result<(), Error<T>> {
		// See if the query exists. If it exists, call corresponding chain storage update
		// function.
		let (entry, _) = Self::get_validators_by_delegator_update_entry(query_id)
			.ok_or(Error::<T>::QueryNotExist)?;
		let currency_id = match entry {
			ValidatorsByDelegatorUpdateEntry::Substrate(substrate_entry) =>
				Some(substrate_entry.currency_id),
		}
		.ok_or(Error::<T>::NotSupportedCurrencyId)?;

		let staking_agent = Self::get_currency_staking_agent(currency_id)?;
		staking_agent.fail_validators_by_delegator_query_response(query_id)?;

		Ok(())
	}

	pub fn derivative_account_id_20(who: [u8; 20], index: u16) -> H160 {
		let entropy = (b"modlpy/utilisuba", who, index).using_encoded(blake2_256);
		let sub_id: [u8; 20] = Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
			.expect("infinite length input; no invalid inputs for type; qed");

		H160::from_slice(sub_id.as_slice())
	}
}

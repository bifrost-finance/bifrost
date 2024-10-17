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
	blake2_256, pallet::Error, AccountIdOf, Config, Decode, DelegatorLedgerXcmUpdateQueue,
	LedgerUpdateEntry, MinimumsAndMaximums, Pallet, TrailingZeroInput, Validators,
	ValidatorsByDelegatorUpdateEntry, ValidatorsByDelegatorXcmUpdateQueue, ASTR, DOT, GLMR, H160,
	KSM, MANTA, MOVR, PHA,
};
use bifrost_primitives::{
	AstarChainId, CurrencyId, MantaChainId, MoonbeamChainId, MoonriverChainId, PhalaChainId,
};
use frame_support::ensure;
use parity_scale_codec::Encode;
use sp_core::Get;
use sp_std::prelude::*;
use xcm::v3::{prelude::*, MultiLocation};

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

	pub fn remove_validators_duplicates(
		currency_id: CurrencyId,
		validators: &Vec<MultiLocation>,
	) -> Result<Vec<MultiLocation>, Error<T>> {
		let validators_set =
			Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;
		let mut validators_list: Vec<MultiLocation> = vec![];
		for validator in validators.iter() {
			// Check if the validator is in the validator whitelist
			ensure!(validators_set.contains(&validator), Error::<T>::ValidatorNotExist);
			if !validators_list.contains(&validator) {
				validators_list.push(*validator);
			}
		}

		Ok(validators_list)
	}

	pub fn check_length_and_deduplicate(
		currency_id: CurrencyId,
		validator_list: Vec<MultiLocation>,
	) -> Result<Vec<MultiLocation>, Error<T>> {
		ensure!(!validator_list.is_empty(), Error::<T>::ValidatorNotProvided);

		// Ensure validator candidates in the whitelist is not greater than maximum.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;

		// ensure validator candidates in the whitelist does not exceed MaxLengthLimit.
		ensure!(
			validator_list.len() <= T::MaxLengthLimit::get() as usize,
			Error::<T>::ExceedMaxLengthLimit
		);

		ensure!(
			validator_list.len() as u16 <= mins_maxs.validators_maximum,
			Error::<T>::GreaterThanMaximum
		);

		// deduplicate validator list.
		let mut validator_set = validator_list.clone();
		validator_set.sort();
		validator_set.dedup();

		Ok(validator_set)
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
			MultiLocation {
				parents: _,
				interior: X2(_, AccountId32 { network: _network_id, id: account_id }),
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
			interior: X1(AccountId32 { network: None, id: account_32 }),
		};

		Ok(local_location)
	}

	pub fn multilocation_to_local_multilocation(
		location: &MultiLocation,
	) -> Result<MultiLocation, Error<T>> {
		let inside: Junction = match location {
			MultiLocation {
				parents: _p,
				interior: X2(Parachain(_para_id), AccountId32 { network: None, id: account_32 }),
			} => AccountId32 { network: None, id: *account_32 },
			MultiLocation {
				parents: _p,
				interior: X2(Parachain(_para_id), AccountKey20 { network: None, key: account_20 }),
			} => AccountKey20 { network: None, key: *account_20 },
			MultiLocation {
				parents: _p,
				interior: X1(AccountId32 { network: None, id: account_32 }),
			} => AccountId32 { network: None, id: *account_32 },
			_ => Err(Error::<T>::Unsupported)?,
		};

		let local_location = MultiLocation { parents: 0, interior: X1(inside) };

		Ok(local_location)
	}

	pub fn account_32_to_parent_location(account_32: [u8; 32]) -> Result<MultiLocation, Error<T>> {
		let parent_location = MultiLocation {
			parents: 1,
			interior: X1(AccountId32 { network: None, id: account_32 }),
		};

		Ok(parent_location)
	}

	pub fn account_32_to_parachain_location(
		account_32: [u8; 32],
		chain_id: u32,
	) -> Result<MultiLocation, Error<T>> {
		let parachain_location = MultiLocation {
			parents: 1,
			interior: X2(Parachain(chain_id), AccountId32 { network: None, id: account_32 }),
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

	/// **************************************
	/// ****** XCM confirming Functions ******
	/// **************************************
	pub fn get_ledger_update_agent_then_process(
		query_id: QueryId,
		manual_mode: bool,
	) -> Result<bool, Error<T>> {
		// See if the query exists. If it exists, call corresponding chain storage update
		// function.
		let (entry, timeout) =
			DelegatorLedgerXcmUpdateQueue::<T>::get(query_id).ok_or(Error::<T>::QueryNotExist)?;

		let now = frame_system::Pallet::<T>::block_number();
		let mut updated = true;
		if now <= timeout {
			let currency_id = match entry.clone() {
				LedgerUpdateEntry::Substrate(substrate_entry) => Some(substrate_entry.currency_id),
				LedgerUpdateEntry::ParachainStaking(moonbeam_entry) =>
					Some(moonbeam_entry.currency_id),
				_ => None,
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
		let (entry, timeout) = ValidatorsByDelegatorXcmUpdateQueue::<T>::get(query_id)
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
			DelegatorLedgerXcmUpdateQueue::<T>::get(query_id).ok_or(Error::<T>::QueryNotExist)?;
		let currency_id = match entry {
			LedgerUpdateEntry::Substrate(substrate_entry) => Some(substrate_entry.currency_id),
			LedgerUpdateEntry::ParachainStaking(moonbeam_entry) => Some(moonbeam_entry.currency_id),
			_ => None,
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
		let (entry, _) = ValidatorsByDelegatorXcmUpdateQueue::<T>::get(query_id)
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

	pub fn convert_currency_to_dest_location(
		currency_id: CurrencyId,
	) -> Result<xcm::v4::Location, Error<T>> {
		match currency_id {
			KSM | DOT => Ok(xcm::v4::Location::parent()),
			MOVR => Ok(xcm::v4::Location::new(
				1,
				[xcm::v4::prelude::Parachain(MoonriverChainId::get())],
			)),
			GLMR =>
				Ok(xcm::v4::Location::new(1, [xcm::v4::prelude::Parachain(MoonbeamChainId::get())])),
			ASTR =>
				Ok(xcm::v4::Location::new(1, [xcm::v4::prelude::Parachain(AstarChainId::get())])),
			MANTA =>
				Ok(xcm::v4::Location::new(1, [xcm::v4::prelude::Parachain(MantaChainId::get())])),
			PHA =>
				Ok(xcm::v4::Location::new(1, [xcm::v4::prelude::Parachain(PhalaChainId::get())])),
			_ => Err(Error::<T>::NotSupportedCurrencyId),
		}
	}

	pub fn get_currency_full_multilocation(
		currency_id: CurrencyId,
	) -> Result<MultiLocation, Error<T>> {
		match currency_id {
			MOVR => Ok(MultiLocation {
				parents: 1,
				interior: X2(Parachain(MoonriverChainId::get()), PalletInstance(10)),
			}),
			GLMR => Ok(MultiLocation {
				parents: 1,
				interior: X2(Parachain(MoonbeamChainId::get()), PalletInstance(10)),
			}),
			MANTA => Ok(MultiLocation { parents: 1, interior: X1(Parachain(MantaChainId::get())) }),
			_ => Err(Error::<T>::NotSupportedCurrencyId),
		}
	}

	pub fn convert_currency_to_remote_fee_location(currency_id: CurrencyId) -> xcm::v4::Location {
		match currency_id {
			MOVR => xcm::v4::Location::new(0, [xcm::v4::prelude::PalletInstance(10)]),
			GLMR => xcm::v4::Location::new(0, [xcm::v4::prelude::PalletInstance(10)]),
			_ => xcm::v4::Location::here(),
		}
	}
}

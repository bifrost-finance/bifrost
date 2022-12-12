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
	agents::{StakingCall, SubstrateCall, SystemCall, XcmCall},
	pallet::{Error, Event},
	primitives::{
		Ledger, PhalaLedger, QueryId, SubstrateLedgerUpdateEntry, SubstrateLedgerUpdateOperation,
		XcmOperation,
	},
	traits::{InstructionBuilder, QueryResponseManager, StakingAgent, XcmBuilder},
	AccountIdOf, BalanceOf, Config, CurrencyDelays, CurrencyId, DelegatorLatestTuneRecord,
	DelegatorLedgerXcmUpdateQueue, DelegatorLedgers, DelegatorsMultilocation2Index, Hash,
	LedgerUpdateEntry, MinimumsAndMaximums, Pallet, TimeUnit, Validators,
	ValidatorsByDelegatorUpdateEntry, XcmDestWeightAndFee, XcmWeight,
};
use codec::Encode;
use core::marker::PhantomData;
use cumulus_primitives_core::relay_chain::HashT;
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::{
	traits::{
		CheckedAdd, CheckedSub, Convert, Saturating, StaticLookup, UniqueSaturatedFrom,
		UniqueSaturatedInto, Zero,
	},
	DispatchResult,
};
use sp_std::prelude::*;
use xcm::{
	latest::prelude::*,
	opaque::latest::{
		Instruction,
		Junction::{AccountId32, GeneralIndex, Parachain},
		Junctions::X1,
		MultiLocation,
	},
	VersionedMultiAssets, VersionedMultiLocation,
};

/// StakingAgent implementation for Phala
pub struct PhalaAgent<T>(PhantomData<T>);

impl<T> PhalaAgent<T> {
	pub fn new() -> Self {
		PhalaAgent(PhantomData::<T>)
	}
}

impl<T: Config>
	StakingAgent<
		BalanceOf<T>,
		AccountIdOf<T>,
		LedgerUpdateEntry<BalanceOf<T>>,
		ValidatorsByDelegatorUpdateEntry<Hash<T>>,
		Error<T>,
	> for PhalaAgent<T>
{
	fn initialize_delegator(
		&self,
		currency_id: CurrencyId,
		_delegator_location_op: Option<Box<MultiLocation>>,
	) -> Result<MultiLocation, Error<T>> {
		let new_delegator_id = Pallet::<T>::inner_initialize_delegator(currency_id)?;

		// Generate multi-location by id.
		let delegator_multilocation = T::AccountConverter::convert((new_delegator_id, currency_id));

		// Add the new delegator into storage
		Self::add_delegator(self, new_delegator_id, &delegator_multilocation, currency_id)
			.map_err(|_| Error::<T>::FailToAddDelegator)?;

		Ok(delegator_multilocation)
	}

	/// First time bonding some amount to a delegator.
	fn bond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Ok(Zero::zero())
	}

	/// Bond extra amount to a delegator.
	fn bond_extra(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Ok(Zero::zero())
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Ok(Zero::zero())
	}

	/// Unbonding all amount of a delegator. Differentiate from regular unbonding.
	fn unbond_all(
		&self,
		who: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Ok(Zero::zero())
	}

	/// Cancel some unbonding amount.
	fn rebond(
		&self,
		who: &MultiLocation,
		amount: Option<BalanceOf<T>>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Ok(Zero::zero())
	}

	/// Delegate to some validators. In Phala context, the passed in Multilocation
	/// should contain validator bonded pool id and NFT collection id. Only deal
	/// with the first item in the vec.
	fn delegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		// Check if it is in the delegator set.
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(currency_id, who.clone()),
			Error::<T>::DelegatorNotExist
		);

		// Check if targets vec is empty.
		let vec_len = targets.len() as u32;
		ensure!(vec_len > 0, Error::<T>::VectorEmpty);

		// Get the first item of the vec
		let candidate = &targets[0];

		if let &MultiLocation {
			parents: 1,
			interior: X2(GeneralIndex(pool_id), GeneralIndex(collection_id)),
		} = candidate
		{
			// Ensure the candidate is in the validator whitelist.
			let validators_set =
				Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;

			let multi_hash = T::Hashing::hash(&candidate.encode());
			ensure!(
				validators_set.contains(&(candidate.clone(), multi_hash)),
				Error::<T>::ValidatorNotExist
			);

			// if the delegator is new, create a ledger for it
			if !DelegatorLedgers::<T>::contains_key(currency_id, &who.clone()) {
				// Create a new delegator ledger\
				let ledger = PhalaLedger::<BalanceOf<T>> {
					account: who.clone(),
					active_shares: Zero::zero(),
					unlocking_shares: Zero::zero(),
					bonded_pool_id: None,
					bonded_pool_collection_id: None,
				};
				let phala_ledger = Ledger::<BalanceOf<T>>::Phala(ledger);

				DelegatorLedgers::<T>::insert(currency_id, who.clone(), phala_ledger);
			}

			DelegatorLedgers::<T>::mutate_exists(
				currency_id,
				who.clone(),
				|old_ledger_opt| -> Result<(), Error<T>> {
					if let Some(Ledger::Phala(ref mut ledger)) = old_ledger_opt {
						ensure!(ledger.active_shares == Zero::zero(), Error::<T>::AlreadyBonded);
						ensure!(ledger.unlocking_shares == Zero::zero(), Error::<T>::AlreadyBonded);

						// delegate the validator
						ledger.bonded_pool_id = Some(u64::unique_saturated_from(pool_id));
						ledger.bonded_pool_collection_id =
							Some(u32::unique_saturated_from(collection_id));
					} else {
						Err(Error::<T>::Unexpected)?;
					}
					Ok(())
				},
			)?;
		} else {
			Err(Error::<T>::ValidatorError)?;
		}

		Ok(Zero::zero())
	}

	/// Remove delegation relationship with some validators.
	fn undelegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Ok(Zero::zero())
	}

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(
		&self,
		who: &MultiLocation,
		targets: &Option<Vec<MultiLocation>>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Ok(Zero::zero())
	}

	/// Initiate payout for a certain delegator.
	fn payout(
		&self,
		who: &MultiLocation,
		validator: &MultiLocation,
		when: &Option<TimeUnit>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		Ok(())
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(
		&self,
		who: &MultiLocation,
		when: &Option<TimeUnit>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
	) -> Result<QueryId, Error<T>> {
		Ok(Zero::zero())
	}

	/// Chill self. Cancel the identity of delegator in the Relay chain side.
	/// Unbonding all the active amount should be done before or after chill,
	/// so that we can collect back all the bonded amount.
	fn chill(&self, who: &MultiLocation, currency_id: CurrencyId) -> Result<QueryId, Error<T>> {
		Ok(Zero::zero())
	}

	/// Make token transferred back to Bifrost chain account.
	fn transfer_back(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
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
		Ok(())
	}

	fn tune_vtoken_exchange_rate(
		&self,
		who: &Option<MultiLocation>,
		token_amount: BalanceOf<T>,
		_vtoken_amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
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
		Ok(())
	}

	/// Add a new serving delegator for a particular currency.
	fn add_validator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_validator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		Ok(())
	}

	/// Charge hosting fee.
	fn charge_hosting_fee(
		&self,
		amount: BalanceOf<T>,
		_from: &MultiLocation,
		to: &MultiLocation,
		currency_id: CurrencyId,
	) -> DispatchResult {
		Ok(())
	}

	/// Deposit some amount as fee to nominator accounts.
	fn supplement_fee_reserve(
		&self,
		amount: BalanceOf<T>,
		from: &MultiLocation,
		to: &MultiLocation,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		Ok(())
	}

	fn check_delegator_ledger_query_response(
		&self,
		query_id: QueryId,
		entry: LedgerUpdateEntry<BalanceOf<T>>,
		manual_mode: bool,
		currency_id: CurrencyId,
	) -> Result<bool, Error<T>> {
		Ok(true)
	}

	fn check_validators_by_delegator_query_response(
		&self,
		query_id: QueryId,
		entry: ValidatorsByDelegatorUpdateEntry<Hash<T>>,
		manual_mode: bool,
	) -> Result<bool, Error<T>> {
		Ok(true)
	}

	fn fail_delegator_ledger_query_response(&self, query_id: QueryId) -> Result<(), Error<T>> {
		Ok(())
	}

	fn fail_validators_by_delegator_query_response(
		&self,
		query_id: QueryId,
	) -> Result<(), Error<T>> {
		Ok(())
	}
}

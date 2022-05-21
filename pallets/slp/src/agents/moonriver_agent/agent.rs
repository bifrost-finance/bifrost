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
use codec::{Decode, Encode};
use cumulus_primitives_core::relay_chain::HashT;
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get, weights::Weight};
use frame_system::pallet_prelude::BlockNumberFor;
use node_primitives::{CurrencyId, TokenSymbol, VtokenMintingOperator};
use orml_traits::MultiCurrency;
use sp_core::{blake2_256, H160, U256};
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
		Ledger, SubstrateLedger, SubstrateLedgerUpdateEntry,
		SubstrateValidatorsByDelegatorUpdateEntry, UnlockChunk, ValidatorsByDelegatorUpdateEntry,
		XcmOperation, MOVR,
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
		LedgerUpdateEntry<BalanceOf<T>, MultiLocation>,
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

	/// First time bonding some amount to a delegator.
	fn bond(&self, who: &MultiLocation, amount: BalanceOf<T>) -> Result<QueryId, Error<T>> {
		unimplemented!()
	}

	/// Bond extra amount to a delegator.
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

	/// Delegate to some validators. For Kusama, it equals function Nominate.
	fn delegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
	) -> Result<QueryId, Error<T>> {
		unimplemented!()
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
		entry: LedgerUpdateEntry<BalanceOf<T>, MultiLocation>,
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

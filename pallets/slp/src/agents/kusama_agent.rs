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

use node_primitives::CurrencyId;
use sp_runtime::{traits::Convert, DispatchResult};
use xcm::opaque::latest::MultiLocation;

use crate::{
	pallet::Error,
	primitives::SubstrateLedger,
	traits::{DelegatorManager, StakingAgent},
	BalanceOf, Config, DelegatorNextIndex, Delegators, Event, Pallet,
};

/// StakingAgent implementation for Kusama
pub struct KusamaAgent<T, AccountConverter>(PhantomData<(T, AccountConverter)>);

impl<T, AccountConverter> StakingAgent<MultiLocation, MultiLocation>
	for KusamaAgent<T, AccountConverter>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
{
	type CurrencyId = CurrencyId;
	type Balance = BalanceOf<T>;

	fn initialize_delegator(currency_id: Self::CurrencyId) -> Option<MultiLocation> {
		let new_delegator_id = DelegatorNextIndex::<T>::get(currency_id);
		let rs = DelegatorNextIndex::<T>::mutate(currency_id, |id| -> DispatchResult {
			let option_new_id = id.checked_add(1).ok_or(Error::<T>::OverFlow)?;
			*id = option_new_id;
			Ok(())
		});

		if let Ok(_) = rs {
			// Generate multi-location by id.
			let delegator_multilocation = AccountConverter::convert(new_delegator_id);

			// Add the new delegator into storage
			let _ = Self::add_delegator(currency_id, new_delegator_id, &delegator_multilocation);

			Pallet::<T>::deposit_event(Event::DelegatorInitialized(
				currency_id,
				delegator_multilocation.clone(),
			));
			Some(delegator_multilocation)
		} else {
			None
		}
	}

	/// First time bonding some amount to a delegator.
	fn bond(
		currency_id: Self::CurrencyId,
		who: &MultiLocation,
		amount: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Bond extra amount to a delegator.
	fn bond_extra(
		currency_id: Self::CurrencyId,
		who: &MultiLocation,
		amount: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(
		currency_id: Self::CurrencyId,
		who: &MultiLocation,
		amount: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Cancel some unbonding amount.
	fn rebond(
		currency_id: Self::CurrencyId,
		who: &MultiLocation,
		amount: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Delegate to some validators.
	fn delegate(
		currency_id: Self::CurrencyId,
		who: &MultiLocation,
		targets: Vec<MultiLocation>,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Remove delegation relationship with some validators.
	fn undelegate(
		currency_id: Self::CurrencyId,
		who: &MultiLocation,
		targets: Vec<MultiLocation>,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(
		currency_id: Self::CurrencyId,
		who: &MultiLocation,
		targets: Vec<MultiLocation>,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Initiate payout for a certain delegator.
	fn payout(currency_id: Self::CurrencyId, who: &MultiLocation) -> Self::Balance {
		unimplemented!()
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(currency_id: Self::CurrencyId, who: &MultiLocation) -> Self::Balance {
		unimplemented!()
	}

	/// Increase/decrease the token amount for the storage "token_pool" in the VtokenMining
	/// module. If the increase variable is true, then we increase token_pool by token_amount.
	/// If it is false, then we decrease token_pool by token_amount.
	fn increase_token_pool(
		currency_id: Self::CurrencyId,
		token_amount: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}
	///
	fn decrease_token_pool(
		currency_id: Self::CurrencyId,
		token_amount: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}
}

/// DelegatorManager implementation for Kusama
impl<T, AccountConverter>
	DelegatorManager<MultiLocation, SubstrateLedger<MultiLocation, BalanceOf<T>>>
	for KusamaAgent<T, AccountConverter>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
{
	type CurrencyId = CurrencyId;

	/// Add a new serving delegator for a particular currency.
	fn add_delegator(
		currency_id: Self::CurrencyId,
		index: u16,
		who: &MultiLocation,
	) -> DispatchResult {
		Delegators::<T>::insert(currency_id, index, who);
		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(currency_id: Self::CurrencyId, who: &MultiLocation) -> DispatchResult {
		unimplemented!()
	}

	/// Get the list of currently serving delegators for a particular currency.
	fn get_delegators(currency_id: Self::CurrencyId) -> Vec<MultiLocation> {
		unimplemented!()
	}

	/// Get the ledger for a particular currency delegator.
	fn get_delegator_ledger(
		currency_id: Self::CurrencyId,
		who: &MultiLocation,
	) -> Option<SubstrateLedger<MultiLocation, BalanceOf<T>>> {
		unimplemented!()
	}
}

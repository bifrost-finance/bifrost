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
use sp_runtime::DispatchResult;
use xcm::opaque::latest::MultiLocation;

use crate::{
	primitives::SubstrateLedger,
	traits::{DelegatorManager, StakingAgent},
	BalanceOf, Config,
};

/// StakingAgent implementation for Kusama
pub struct KusamaAgent<T>(PhantomData<T>);

impl<T: Config> StakingAgent<MultiLocation, MultiLocation> for KusamaAgent<T> {
	type CurrencyId = CurrencyId;
	type Balance = BalanceOf<T>;

	fn initialize_delegator(currency_id: Self::CurrencyId) -> MultiLocation {
		unimplemented!()
	}
	// {
	// 	let new_delegator_id = Self::get_delegator_next_index(currency_id);
	// 	let rs = DelegatorNextIndex::<T>::mutate(currency_id, |id| -> DispatchResult {
	// 		let option_new_id = id.checked_add(1);
	// 		if let Some(new_id) = option_new_id {
	// 			id = new_id;
	// 			return Ok(());
	// 		} else {
	// 			return DispatchError;
	// 		}
	// 	});

	// if let

	// Self::add_delegator();
	// }

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
impl<T: Config> DelegatorManager<MultiLocation, SubstrateLedger<MultiLocation, BalanceOf<T>>>
	for KusamaAgent<T>
{
	type CurrencyId = CurrencyId;

	/// Add a new serving delegator for a particular currency.
	fn add_delegator(currency_id: Self::CurrencyId, who: &MultiLocation) -> DispatchResult {
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

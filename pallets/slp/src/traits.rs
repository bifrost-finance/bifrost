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

use sp_runtime::DispatchResult;
use sp_std::vec::Vec;

use crate::{Weight, Xcm};

/// Abstraction over a staking agent for a certain POS chain.
pub trait StakingAgent<DelegatorId, ValidatorId, Balance, TimeUnit, AccountId> {
	/// Delegator initialization work. Generate a new delegator and return its ID.
	fn initialize_delegator(&self) -> Option<DelegatorId>;

	/// First time bonding some amount to a delegator.
	fn bond(&self, who: DelegatorId, amount: Balance) -> DispatchResult;

	/// Bond extra amount to a delegator.
	fn bond_extra(&self, who: DelegatorId, amount: Balance) -> DispatchResult;

	/// Decrease the bonding amount of a delegator.
	fn unbond(&self, who: DelegatorId, amount: Balance) -> DispatchResult;

	/// Unbonding all amount of a delegator. Differentiate from regular unbonding.
	fn unbond_all(&self, who: DelegatorId) -> DispatchResult;

	/// Cancel some unbonding amount.
	fn rebond(&self, who: DelegatorId, amount: Balance) -> DispatchResult;

	/// Delegate to some validators.
	fn delegate(&self, who: DelegatorId, targets: Vec<ValidatorId>) -> DispatchResult;

	/// Remove delegation relationship with some validators.
	fn undelegate(&self, who: DelegatorId, targets: Vec<ValidatorId>) -> DispatchResult;

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(&self, who: DelegatorId, targets: Vec<ValidatorId>) -> DispatchResult;

	/// Initiate payout for a certain delegator.
	fn payout(
		&self,
		who: DelegatorId,
		validator: ValidatorId,
		when: Option<TimeUnit>,
	) -> DispatchResult;

	/// Withdraw the due payout into free balance.
	fn liquidize(&self, who: DelegatorId, when: Option<TimeUnit>) -> DispatchResult;

	/// Cancel the identity of delegator.
	fn chill(&self, who: DelegatorId) -> DispatchResult;

	/// Make token transferred back to Bifrost chain account.
	fn transfer_back(&self, from: DelegatorId, to: AccountId, amount: Balance) -> DispatchResult;

	/// Make token from Bifrost chain account to the staking chain account.
	fn transfer_to(&self, from: AccountId, to: DelegatorId, amount: Balance) -> DispatchResult;
}

/// Abstraction over a fee manager for charging fee from the origin chain(Bifrost)
/// or deposit fee reserves for the destination chain nominator accounts.
pub trait StakingFeeManager<AccountId, Balance> {
	/// Charge hosting fee.
	fn charge_hosting_fee(&self, amount: Balance, from: AccountId, to: AccountId)
		-> DispatchResult;

	/// Deposit some amount as fee to nominator accounts.
	fn supplement_fee_reserve(
		&self,
		amount: Balance,
		from: AccountId,
		to: AccountId,
	) -> DispatchResult;
}

/// Abstraction over a delegator manager.
pub trait DelegatorManager<DelegatorId, Ledger> {
	/// Add a new serving delegator for a particular currency.
	fn add_delegator(&self, index: u16, who: &DelegatorId) -> DispatchResult;

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(&self, who: &DelegatorId) -> DispatchResult;
}

/// Abstraction over a validator manager.
pub trait ValidatorManager<ValidatorId> {
	/// Add a new serving validator for a particular currency.
	fn add_validator(&self, who: &ValidatorId) -> DispatchResult;

	/// Remove an existing serving validator for a particular currency.
	fn remove_validator(&self, who: &ValidatorId) -> DispatchResult;
}

/// Helper to build xcm message
pub trait XcmBuilder<Balance, ChainCallType> {
	fn construct_xcm_message(call: ChainCallType, extra_fee: Balance, weight: Weight) -> Xcm<()>;
}

/// Helper to communicate with pallet_xcm's Queries storage for Substrate chains in runtime.
pub trait QueryResponseManager<XcmQueryId, AccountId, BlockNumber> {
	fn get_query_response_record(query_id: XcmQueryId) -> DispatchResult;
	fn create_query_record(
		query_id: XcmQueryId,
		responder: AccountId,
		match_querier: AccountId,
		timeout: BlockNumber,
	) -> DispatchResult;
	fn remove_query_record(query_id: XcmQueryId) -> DispatchResult;
}

/// Abstraction over a QueryResponseChecker.
pub trait QueryResponseChecker<XcmQueryId, LedgerUpdateEntry, ValidatorsByDelegatorUpdateEntry> {
	fn check_delegator_ledger_query_response(
		&self,
		query_id: XcmQueryId,
		query_entry: LedgerUpdateEntry,
	) -> DispatchResult;
	fn check_validators_by_delegator_query_response(
		&self,
		query_id: XcmQueryId,
		query_entry: ValidatorsByDelegatorUpdateEntry,
	) -> DispatchResult;
}

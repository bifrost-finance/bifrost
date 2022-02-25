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

use crate::{Weight, Xcm};

/// Abstraction over a staking agent for a certain POS chain.
pub trait StakingAgent<DelegatorId, ValidatorId, Balance> {
	/// Delegator initialization work. Generate a new delegator and return its ID.
	fn initialize_delegator() -> Option<DelegatorId>;

	/// First time bonding some amount to a delegator.
	fn bond(who: DelegatorId, amount: Balance) -> DispatchResult;

	/// Bond extra amount to a delegator.
	fn bond_extra(who: DelegatorId, amount: Balance) -> DispatchResult;

	/// Decrease bonding amount to a delegator.
	fn unbond(who: DelegatorId, amount: Balance) -> DispatchResult;

	/// Cancel some unbonding amount.
	fn rebond(who: DelegatorId, amount: Balance) -> DispatchResult;

	/// Delegate to some validators.
	fn delegate(who: DelegatorId, targets: Vec<ValidatorId>) -> DispatchResult;

	/// Remove delegation relationship with some validators.
	fn undelegate(who: DelegatorId, targets: Vec<ValidatorId>) -> DispatchResult;

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(who: DelegatorId, targets: Vec<ValidatorId>) -> DispatchResult;

	/// Initiate payout for a certain delegator.
	fn payout(who: DelegatorId) -> Balance;

	/// Withdraw the due payout into free balance.
	fn liquidize(who: DelegatorId) -> Balance;

	/// Increase/decrease the token amount for the storage "token_pool" in the VtokenMining module.
	/// If the increase variable is true, then we increase token_pool by token_amount. If it is
	/// false, then we decrease token_pool by token_amount.
	fn increase_token_pool(token_amount: Balance) -> DispatchResult;
	fn decrease_token_pool(token_amount: Balance) -> DispatchResult;
}

/// Abstraction over a fee manager for charging fee from the origin chain(Bifrost)
/// or deposit fee reserves for the destination chain nominator accounts.
pub trait StakingFeeManager<AccountId, Balance> {
	/// Charge hosting fee from an account in Bifrost chain.
	fn charge_hosting_fee(amount: Balance, from: &AccountId, to: &AccountId) -> DispatchResult;

	/// Deposit some amount as fee to nominator accounts.
	fn fill_cost_reserve(amount: Balance, from: &AccountId, to: &AccountId) -> DispatchResult;
}

/// Abstraction over a delegator manager.
pub trait DelegatorManager<DelegatorId, Ledger> {
	/// Add a new serving delegator for a particular currency.
	fn add_delegator(index: u16, who: &DelegatorId) -> DispatchResult;

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(who: &DelegatorId) -> DispatchResult;
}

/// Abstraction over a validator manager.
pub trait ValidatorManager<ValidatorId> {
	/// Add a new serving validator for a particular currency.
	fn add_validator(who: &ValidatorId) -> DispatchResult;

	/// Remove an existing serving validator for a particular currency.
	fn remove_validator(who: &ValidatorId) -> DispatchResult;
}

/// Abstraction over a user refund manager to refund user unlocking balance without waiting for the
/// maximum amount of time. It it for being called by other pallets such as VtokenMinting.
pub trait UserRefundManager<AccountId, CurrencyId, Balance> {
	/// Refund user unlocking balance without waiting for the maximum amount of time.
	fn refund_user_unbond(
		currency_id: CurrencyId,
		who: &AccountId,
		amount: Balance,
	) -> DispatchResult;
}

/// Helper to build xcm message
pub trait XcmBuilder<Balance, ChainCallType> {
	fn construct_xcm_message(call: ChainCallType, extra_fee: Balance, weight: Weight) -> Xcm<()>;
}

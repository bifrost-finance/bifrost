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

use codec::FullCodec;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize},
	DispatchResult,
};
use sp_std::fmt::Debug;

/// Abstraction over a staking agent for a certain POS chain.
pub trait StakingAgent<DelegatorId, ValidatorId> {
	/// The currency identifier.
	type CurrencyId: FullCodec
		+ Eq
		+ PartialEq
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ scale_info::TypeInfo;

	/// The balance of an account.
	type Balance: AtLeast32BitUnsigned
		+ FullCodec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default
		+ scale_info::TypeInfo;

	/// Delegator initialization work. Generate a new delegator and return its ID.
	fn initialize_delegator(currency_id: Self::CurrencyId) -> DelegatorId;

	/// First time bonding some amount to a delegator.
	fn bond(
		currency_id: Self::CurrencyId,
		who: &DelegatorId,
		amount: Self::Balance,
	) -> DispatchResult;

	/// Bond extra amount to a delegator.
	fn bond_extra(
		currency_id: Self::CurrencyId,
		who: &DelegatorId,
		amount: Self::Balance,
	) -> DispatchResult;

	/// Decrease bonding amount to a delegator.
	fn unbond(
		currency_id: Self::CurrencyId,
		who: &DelegatorId,
		amount: Self::Balance,
	) -> DispatchResult;

	/// Cancel some unbonding amount.
	fn rebond(
		currency_id: Self::CurrencyId,
		who: &DelegatorId,
		amount: Self::Balance,
	) -> DispatchResult;

	/// Delegate to some validators.
	fn delegate(
		currency_id: Self::CurrencyId,
		who: &DelegatorId,
		targets: Vec<ValidatorId>,
	) -> DispatchResult;

	/// Remove delegation relationship with some validators.
	fn undelegate(
		currency_id: Self::CurrencyId,
		who: &DelegatorId,
		targets: Vec<ValidatorId>,
	) -> DispatchResult;

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(
		currency_id: Self::CurrencyId,
		who: &DelegatorId,
		targets: Vec<ValidatorId>,
	) -> DispatchResult;

	/// Initiate payout for a certain delegator.
	fn payout(currency_id: Self::CurrencyId, who: &DelegatorId) -> Self::Balance;

	/// Withdraw the due payout into free balance.
	fn liquidize(currency_id: Self::CurrencyId, who: &DelegatorId) -> Self::Balance;

	/// Increase/decrease the token amount for the storage "token_pool" in the VtokenMining module.
	/// If the increase variable is true, then we increase token_pool by token_amount. If it is
	/// false, then we decrease token_pool by token_amount.
	fn increase_token_pool(
		currency_id: Self::CurrencyId,
		token_amount: Self::Balance,
	) -> DispatchResult;
	fn decrease_token_pool(
		currency_id: Self::CurrencyId,
		token_amount: Self::Balance,
	) -> DispatchResult;
}

/// Abstraction over a fee manager for charging fee from the origin chain(Bifrost)
/// or deposit fee reserves for the destination chain nominator accounts.
pub trait StakingFeeManager<AccountId> {
	/// The currency identifier.
	type CurrencyId: FullCodec
		+ Eq
		+ PartialEq
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ scale_info::TypeInfo;

	/// The balance of an account.
	type Balance: AtLeast32BitUnsigned
		+ FullCodec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default
		+ scale_info::TypeInfo;

	/// Charge hosting fee from an account in Bifrost chain.
	fn charge_hosting_fee(
		currency_id: Self::CurrencyId,
		amount: Self::Balance,
		from: &AccountId,
		to: &AccountId,
	) -> DispatchResult;

	/// Deposit some amount as fee to nominator accounts.
	fn fill_cost_reserve(
		currency_id: Self::CurrencyId,
		amount: Self::Balance,
		from: &AccountId,
		to: &AccountId,
	) -> DispatchResult;
}

/// Abstraction over a delegator manager.
pub trait DelegatorManager<DelegatorId, Ledger> {
	/// The currency identifier.
	type CurrencyId: FullCodec
		+ Eq
		+ PartialEq
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ scale_info::TypeInfo;

	/// Add a new serving delegator for a particular currency.
	fn add_delegator(currency_id: Self::CurrencyId, who: &DelegatorId) -> DispatchResult;

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(currency_id: Self::CurrencyId, who: &DelegatorId) -> DispatchResult;

	/// Get the list of currently serving delegators for a particular currency.
	fn get_delegators(currency_id: Self::CurrencyId) -> Vec<DelegatorId>;

	/// Get the ledger for a particular currency delegator.
	fn get_delegator_ledger(currency_id: Self::CurrencyId, who: &DelegatorId) -> Option<Ledger>;
}

/// Abstraction over a validator manager.
pub trait ValidatorManager<ValidatorId> {
	/// The currency identifier.
	type CurrencyId: FullCodec
		+ Eq
		+ PartialEq
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ scale_info::TypeInfo;

	/// Add a new serving validator for a particular currency.
	fn add_validator(currency_id: Self::CurrencyId, who: &ValidatorId) -> DispatchResult;

	/// Remove an existing serving validator for a particular currency.
	fn remove_validator(currency_id: Self::CurrencyId, who: &ValidatorId) -> DispatchResult;

	/// Get the list of currently serving validators for a particular currency.
	fn get_validators(currency_id: Self::CurrencyId) -> Vec<ValidatorId>;
}

/// Abstraction over a user refund manager to refund user unlocking balance without waiting for the
/// maximum amount of time.
pub trait UserRefundManager<AccountId> {
	/// The currency identifier.
	type CurrencyId: FullCodec
		+ Eq
		+ PartialEq
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ scale_info::TypeInfo;

	/// The balance of an account.
	type Balance: AtLeast32BitUnsigned
		+ FullCodec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default
		+ scale_info::TypeInfo;

	/// Refund user unlocking balance without waiting for the maximum amount of time.
	fn refund_user_unbond(
		currency_id: Self::CurrencyId,
		who: &AccountId,
		amount: Self::Balance,
	) -> DispatchResult;
}

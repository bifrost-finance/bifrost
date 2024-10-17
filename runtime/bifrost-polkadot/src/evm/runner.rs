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

//! EVM stack-based runner.
//! This runner is a wrapper around the default stack-based runner that adds possibility to charge
//! fees in different currencies and to validate transactions based on the account's fee payment
//! asset.
//!
//! Shamelessly copied from pallet-evm and modified to support multi-currency fees.

use crate::{evm::WethAssetId, Weight};
use bifrost_primitives::{
	AccountFeeCurrency, AccountFeeCurrencyBalanceInCurrency, Balance, CurrencyId,
	OraclePriceProvider,
};
use fp_evm::{Account, TransactionValidationError};
use frame_support::traits::{
	tokens::{Fortitude, Preservation},
	Get,
};
use pallet_evm::{
	runner::Runner, AddressMapping, CallInfo, Config, CreateInfo, FeeCalculator, RunnerError,
};
use primitive_types::{H160, H256, U256};
use sp_runtime::{traits::UniqueSaturatedInto, DispatchError};
use sp_std::{marker::PhantomData, vec::Vec};

/// AccountFeeCurrencyBalanceInCurrency implementation for the FeeAssetBalanceInCurrency.
/// Provides account's balance of fee asset currency in a given currency
pub struct FeeAssetBalanceInCurrency<T, Price, AC, I>(PhantomData<(T, Price, AC, I)>);

impl<T, Price, AC, I> AccountFeeCurrencyBalanceInCurrency<T::AccountId>
	for FeeAssetBalanceInCurrency<T, Price, AC, I>
where
	T: frame_system::Config,
	Price: OraclePriceProvider,
	AC: AccountFeeCurrency<T::AccountId>,
	I: frame_support::traits::fungibles::Inspect<
		T::AccountId,
		AssetId = CurrencyId,
		Balance = Balance,
	>,
{
	type Output = (Balance, Weight);
	type Error = DispatchError;

	fn get_balance_in_currency(
		to_currency: CurrencyId,
		account: &T::AccountId,
		fee: U256,
	) -> Result<Self::Output, DispatchError> {
		let from_currency = AC::get_fee_currency(account, fee)
			.map_err(|_| DispatchError::Other("Get Currency Error."))?;
		let account_balance =
			I::reducible_balance(from_currency, account, Preservation::Preserve, Fortitude::Polite);
		let price_weight = T::DbWeight::get().reads(2); // 1 read to get currency and 1 read to get balance

		if from_currency == to_currency {
			return Ok((account_balance, price_weight));
		}

		let Some((converted, _, _)) = Price::get_oracle_amount_by_currency_and_amount_in(
			&from_currency,
			account_balance,
			&to_currency,
		) else {
			return Ok((0, price_weight));
		};
		Ok((converted, price_weight))
	}
}

pub struct WrapRunner<T, R, B>(sp_std::marker::PhantomData<(T, R, B)>);

impl<T, R, B> Runner<T> for WrapRunner<T, R, B>
where
	T: Config,
	R: Runner<T>,
	<R as pallet_evm::Runner<T>>::Error: core::convert::From<TransactionValidationError>,
	B: AccountFeeCurrencyBalanceInCurrency<T::AccountId, Output = (Balance, Weight)>,
{
	type Error = R::Error;

	fn validate(
		source: H160,
		target: Option<H160>,
		input: Vec<u8>,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		is_transactional: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		evm_config: &evm::Config,
	) -> Result<(), RunnerError<Self::Error>> {
		let (base_fee, mut weight) = T::FeeCalculator::min_gas_price();

		let evm_currency = WethAssetId::get();
		let account_id = T::AddressMapping::into_account_id(source);
		let account_nonce = frame_system::Pallet::<T>::account_nonce(&account_id);

		let (balance, b_weight) = B::get_balance_in_currency(evm_currency, &account_id, base_fee)
			.map_err(|_| RunnerError {
			error: R::Error::from(TransactionValidationError::BalanceTooLow),
			weight,
		})?;

		let (source_account, inner_weight) = (
			Account {
				nonce: U256::from(UniqueSaturatedInto::<u128>::unique_saturated_into(
					account_nonce,
				)),
				balance: U256::from(UniqueSaturatedInto::<u128>::unique_saturated_into(balance)),
			},
			T::DbWeight::get().reads(1).saturating_add(b_weight),
		);
		weight = weight.saturating_add(inner_weight);

		let _ = fp_evm::CheckEvmTransaction::<Self::Error>::new(
			fp_evm::CheckEvmTransactionConfig {
				evm_config,
				block_gas_limit: T::BlockGasLimit::get(),
				base_fee,
				chain_id: T::ChainId::get(),
				is_transactional,
			},
			fp_evm::CheckEvmTransactionInput {
				chain_id: Some(T::ChainId::get()),
				to: target,
				input,
				nonce: nonce.unwrap_or(source_account.nonce),
				gas_limit: gas_limit.into(),
				gas_price: None,
				max_fee_per_gas,
				max_priority_fee_per_gas,
				value,
				access_list,
			},
			weight_limit,
			proof_size_base_cost,
		)
		.validate_in_block_for(&source_account)
		.and_then(|v| v.with_base_fee())
		.and_then(|v| v.with_balance_for(&source_account))
		.map_err(|error| RunnerError { error, weight })?;
		Ok(())
	}

	fn call(
		source: H160,
		target: H160,
		input: Vec<u8>,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		is_transactional: bool,
		validate: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		config: &evm::Config,
	) -> Result<CallInfo, RunnerError<Self::Error>> {
		if validate {
			Self::validate(
				source,
				Some(target),
				input.clone(),
				value,
				gas_limit,
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.clone(),
				is_transactional,
				weight_limit,
				proof_size_base_cost,
				config,
			)?;
		}
		// Validated, flag set to false
		R::call(
			source,
			target,
			input,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			is_transactional,
			false,
			weight_limit,
			proof_size_base_cost,
			config,
		)
	}

	fn create(
		source: H160,
		init: Vec<u8>,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		is_transactional: bool,
		validate: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		config: &evm::Config,
	) -> Result<CreateInfo, RunnerError<Self::Error>> {
		if validate {
			Self::validate(
				source,
				None,
				init.clone(),
				value,
				gas_limit,
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.clone(),
				is_transactional,
				weight_limit,
				proof_size_base_cost,
				config,
			)?;
		}
		// Validated, flag set to false
		R::create(
			source,
			init,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			is_transactional,
			false,
			weight_limit,
			proof_size_base_cost,
			config,
		)
	}

	fn create2(
		source: H160,
		init: Vec<u8>,
		salt: H256,
		value: U256,
		gas_limit: u64,
		max_fee_per_gas: Option<U256>,
		max_priority_fee_per_gas: Option<U256>,
		nonce: Option<U256>,
		access_list: Vec<(H160, Vec<H256>)>,
		is_transactional: bool,
		validate: bool,
		weight_limit: Option<Weight>,
		proof_size_base_cost: Option<u64>,
		config: &evm::Config,
	) -> Result<CreateInfo, RunnerError<Self::Error>> {
		if validate {
			Self::validate(
				source,
				None,
				init.clone(),
				value,
				gas_limit,
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.clone(),
				is_transactional,
				weight_limit,
				proof_size_base_cost,
				config,
			)?;
		}
		//Validated, flag set to false
		R::create2(
			source,
			init,
			salt,
			value,
			gas_limit,
			max_fee_per_gas,
			max_priority_fee_per_gas,
			nonce,
			access_list,
			is_transactional,
			false,
			weight_limit,
			proof_size_base_cost,
			config,
		)
	}
}

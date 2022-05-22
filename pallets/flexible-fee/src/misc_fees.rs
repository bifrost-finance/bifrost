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

// The swap pool algorithm implements Balancer protocol
// For more details, refer to https://balancer.finance/whitepaper/

use frame_support::traits::Contains;
use node_primitives::ExtraFeeName;

use super::*;
use crate::Config;

pub struct MiscFeeHandler<T, FeeCurrency, FeeAmount, FeeFilter>(
	PhantomData<(T, FeeCurrency, FeeAmount, FeeFilter)>,
);

impl<T: Config, FeeCurrency, FeeAmount, FeeFilter>
	FeeDeductor<T::AccountId, CurrencyIdOf<T>, PalletBalanceOf<T>, T::Call>
	for MiscFeeHandler<T, FeeCurrency, FeeAmount, FeeFilter>
where
	FeeCurrency: Get<CurrencyIdOf<T>>,
	FeeAmount: Get<PalletBalanceOf<T>>,
	FeeFilter: Contains<CallOf<T>>,
{
	fn deduct_fee(
		who: &T::AccountId,
		receiver: &T::AccountId,
		call: &T::Call,
	) -> Result<(CurrencyIdOf<T>, PalletBalanceOf<T>), DispatchError> {
		// If this call matches a specific extra-fee call
		if FeeFilter::contains(call) {
			let total_fee = FeeAmount::get();
			let fee_currency = FeeCurrency::get();

			<T as pallet::Config>::MultiCurrency::transfer(fee_currency, who, receiver, total_fee)?;
			Ok((fee_currency, total_fee))
		} else {
			Err(DispatchError::Other("Failed to deduct extra fee."))
		}
	}
}

pub trait FeeDeductor<AccountId, CurrencyId, Balance, Call> {
	fn deduct_fee(
		who: &AccountId,
		receiver: &AccountId,
		call: &Call,
	) -> Result<(CurrencyId, Balance), DispatchError>;
}

#[impl_trait_for_tuples::impl_for_tuples(30)]
impl<AccountId, CurrencyId, Balance, Call> FeeDeductor<AccountId, CurrencyId, Balance, Call>
	for Tuple
{
	fn deduct_fee(
		who: &AccountId,
		receiver: &AccountId,
		call: &Call,
	) -> Result<(CurrencyId, Balance), DispatchError> {
		for_tuples!(
			#(
				if let Ok(result) = Tuple::deduct_fee(who, receiver, call) {
					return Ok(result);
				}
			)*
		);

		Err(DispatchError::Other("Failed to deduct extra fee."))
	}
}

pub trait NameGetter<Call> {
	fn get_name(call: &Call) -> ExtraFeeName;
}

pub trait FeeGetter<Call> {
	fn get_fee_info(call: &Call) -> (ExtraFeeName, bool);
}

pub struct ExtraFeeMatcher<T, FeeNameGetter, AggregateExtraFeeFilter>(
	PhantomData<(T, FeeNameGetter, AggregateExtraFeeFilter)>,
);
impl<T: Config, FeeNameGetter, AggregateExtraFeeFilter> FeeGetter<CallOf<T>>
	for ExtraFeeMatcher<T, FeeNameGetter, AggregateExtraFeeFilter>
where
	FeeNameGetter: NameGetter<CallOf<T>>,
	AggregateExtraFeeFilter: Contains<CallOf<T>>,
{
	fn get_fee_info(call: &CallOf<T>) -> (ExtraFeeName, bool) {
		let fee_name = FeeNameGetter::get_name(call.clone());
		let if_extra_fee = AggregateExtraFeeFilter::contains(call);

		(fee_name, if_extra_fee)
	}
}

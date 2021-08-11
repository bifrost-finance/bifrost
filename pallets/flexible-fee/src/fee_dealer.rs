// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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

use sp_runtime::traits::UniqueSaturatedFrom;

use super::*;
use crate::Config;

pub trait FeeDealer<AccountId, Balance> {
	fn ensure_can_charge_fee(
		who: &AccountId,
		fee: Balance,
		reason: WithdrawReasons,
	) -> DispatchResult;
}

pub struct FixedCurrencyFeeRate<T: Config>(PhantomData<T>);

impl<T: Config> FeeDealer<T::AccountId, PalletBalanceOf<T>> for FixedCurrencyFeeRate<T> {
	/// Make sure there is enough BNC to be deducted if the user has assets in other form of tokens
	/// rather than BNC.
	fn ensure_can_charge_fee(
		who: &T::AccountId,
		fee: PalletBalanceOf<T>,
		reason: WithdrawReasons,
	) -> DispatchResult {
		// First, check if the user has enough BNC balance to be deducted.assert_eq!
		let existential_deposit = <<T as Config>::Currency as Currency<
			<T as frame_system::Config>::AccountId,
		>>::minimum_balance();
		// check native balance if is enough
		let native_is_enough = <<T as Config>::Currency as Currency<
			<T as frame_system::Config>::AccountId,
		>>::free_balance(who)
		.checked_sub(&(fee + existential_deposit.into()))
		.map_or(false, |new_free_balance| {
			<<T as Config>::Currency as Currency<
										<T as frame_system::Config>::AccountId,
									>>::ensure_can_withdraw(
										who, fee, reason, new_free_balance
									)
									.is_ok()
		});

		if !native_is_enough {
			// If the user doesn't has enough BNC, then we will use KSM as the fee currency.
			let fee_currency_id: CurrencyId = T::AlternativeFeeCurrencyId::get();
			let (fee_currency_base, native_currency_base): (u32, u32) =
				T::AltFeeCurrencyExchangeRate::get();

			let fee_currency_balance = T::MultiCurrency::free_balance(fee_currency_id, who);

			let consume_fee_currency_amount =
				fee.saturating_mul(fee_currency_base.into()) / native_currency_base.into();
			ensure!(
				consume_fee_currency_amount
					<= PalletBalanceOf::<T>::unique_saturated_from(fee_currency_balance.into()),
				Error::<T>::NotEnoughBalance
			);

			// deduct fee currency and increase native currency amount
			// This withdraw operation allows death. So it will succeed given the remaining amount
			// less than the existential deposit.
			T::MultiCurrency::withdraw(
				fee_currency_id,
				who,
				T::Balance::from(consume_fee_currency_amount),
			)?;
			T::MultiCurrency::deposit(T::NativeCurrencyId::get(), who, T::Balance::from(fee))?;

			crate::Pallet::<T>::deposit_event(Event::FixedRateFeeExchanged(
				fee_currency_id,
				consume_fee_currency_amount,
			));
		}

		Ok(())
	}
}

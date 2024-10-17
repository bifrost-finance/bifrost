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

use crate::{Config, ExtraFeeByCall, Pallet};
use bifrost_primitives::{Balance, CurrencyId, OraclePriceProvider, Price, BNC};
use orml_traits::MultiCurrency;
use pallet_transaction_payment::OnChargeTransaction;
use parity_scale_codec::Encode;
use sp_core::Get;
use sp_runtime::{
	traits::{DispatchInfoOf, PostDispatchInfoOf, Zero},
	transaction_validity::{InvalidTransaction, TransactionValidityError},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentInfo {
	Native(Balance),
	NonNative(Balance, CurrencyId, Price, Price),
}

/// Default implementation for a Currency and an OnUnbalanced handler.
impl<T> OnChargeTransaction<T> for Pallet<T>
where
	T: Config,
	T::MultiCurrency: MultiCurrency<T::AccountId, CurrencyId = CurrencyId>,
{
	type Balance = Balance;
	type LiquidityInfo = Option<PaymentInfo>;

	/// Withdraw the predicted fee from the transaction origin.
	///
	/// Note: The `fee` already includes the `tip`.
	fn withdraw_fee(
		who: &T::AccountId,
		call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		fee: Self::Balance,
		_tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		if fee.is_zero() {
			return Ok(None);
		}

		let (fee_currency, fee_amount, bnc_price, fee_currency_price) =
			Self::get_fee_currency_and_fee_amount(who, fee)
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

		// withdraw normal extrinsic fee
		T::MultiCurrency::withdraw(fee_currency, who, fee_amount)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

		for (call_name, (extra_fee_currency, extra_fee_amount, extra_fee_receiver)) in
			ExtraFeeByCall::<T>::iter()
		{
			let raw_call_name = call_name.to_vec();
			let raw_call_name_len = raw_call_name.len();
			if call.encode().len() >= raw_call_name_len {
				if call.encode()[0..raw_call_name_len].eq(&raw_call_name) {
					match Self::charge_extra_fee(
						who,
						extra_fee_currency,
						extra_fee_amount,
						&extra_fee_receiver,
					) {
						Ok(_) => {},
						Err(_) => {
							return Err(TransactionValidityError::Invalid(
								InvalidTransaction::Payment,
							));
						},
					}
				};
			}
		}

		if fee_currency == BNC {
			Ok(Some(PaymentInfo::Native(fee_amount)))
		} else {
			Ok(Some(PaymentInfo::NonNative(
				fee_amount,
				fee_currency,
				bnc_price,
				fee_currency_price,
			)))
		}
	}

	/// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
	/// Since the predicted fee might have been too high, parts of the fee may
	/// be refunded.
	///
	/// Note: The `fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &T::AccountId,
		_dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		_post_info: &PostDispatchInfoOf<T::RuntimeCall>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		if let Some(paid) = already_withdrawn {
			// Calculate how much refund we should return
			let (currency, refund, fee, tip) = match paid {
				PaymentInfo::Native(paid_fee) => (
					BNC,
					paid_fee.saturating_sub(corrected_fee),
					corrected_fee.saturating_sub(tip),
					tip,
				),
				PaymentInfo::NonNative(paid_fee, fee_currency, bnc_price, fee_currency_price) => {
					// calculate corrected_fee in the non-native currency
					let converted_corrected_fee = T::OraclePriceProvider::get_amount_by_prices(
						&BNC,
						corrected_fee,
						bnc_price,
						&fee_currency,
						fee_currency_price,
					)
					.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
					let refund = paid_fee.saturating_sub(converted_corrected_fee);
					let converted_tip = T::OraclePriceProvider::get_amount_by_prices(
						&BNC,
						tip,
						bnc_price,
						&fee_currency,
						fee_currency_price,
					)
					.ok_or(TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
					(
						fee_currency,
						refund,
						converted_corrected_fee.saturating_sub(converted_tip),
						converted_tip,
					)
				},
			};
			// refund to the account that paid the fees
			T::MultiCurrency::deposit(currency, who, refund)
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

			// deposit the fee
			T::MultiCurrency::deposit(currency, &T::TreasuryAccount::get(), fee + tip)
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
		}
		Ok(())
	}
}

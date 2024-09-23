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

use crate::Currencies;
use bifrost_primitives::{
	AccountFeeCurrency, Balance, CurrencyId, OraclePriceProvider, Price, WETH,
};
use frame_support::traits::TryDrop;
use orml_traits::MultiCurrency;
use pallet_evm::{AddressMapping, Error, OnChargeEVMTransaction};
use sp_core::{H160, U256};
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::marker::PhantomData;

#[derive(Copy, Debug, Clone, Default, PartialEq)]
pub struct EvmPaymentInfo {
	fee_amount: Balance,
	fee_currency: CurrencyId,
	fee_currency_price: Price,
	weth_price: Price,
}

impl EvmPaymentInfo {
	pub fn merge(self, other: Self) -> Self {
		EvmPaymentInfo {
			fee_amount: self.fee_amount.saturating_add(other.fee_amount),
			fee_currency: self.fee_currency,
			fee_currency_price: self.fee_currency_price,
			weth_price: self.weth_price,
		}
	}
}

impl TryDrop for EvmPaymentInfo {
	fn try_drop(self) -> Result<(), Self> {
		if self.fee_amount == 0 {
			Ok(())
		} else {
			Err(self)
		}
	}
}

/// Implements the transaction payment for EVM transactions.
/// Supports multi-currency fees based on what is provided by AccountFeeCurrency - account currency.
pub struct TransferEvmFees<AC, MC, Price>(PhantomData<(AC, MC, Price)>);

impl<T, AC, MC, Price> OnChargeEVMTransaction<T> for TransferEvmFees<AC, MC, Price>
where
	T: pallet_evm::Config,
	AC: AccountFeeCurrency<T::AccountId>, // AccountCurrency
	Price: OraclePriceProvider,           // PriceProvider
	MC: MultiCurrency<T::AccountId, CurrencyId = CurrencyId, Balance = Balance>,
	U256: UniqueSaturatedInto<Balance>,
	sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
{
	type LiquidityInfo = Option<EvmPaymentInfo>;

	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, Error<T>> {
		if fee.is_zero() {
			return Ok(None);
		}
		let account_id = T::AddressMapping::into_account_id(*who);

		let fee_currency =
			AC::get_fee_currency(&account_id, fee).map_err(|_| Error::<T>::BalanceLow)?;

		let Some((fee_amount, weth_price, fee_currency_price)) =
			Price::get_oracle_amount_by_currency_and_amount_in(
				&WETH,
				fee.unique_saturated_into(),
				&fee_currency,
			)
		else {
			return Err(Error::<T>::WithdrawFailed);
		};

		// Ensure that converted fee is not zero
		if fee_amount == 0 {
			return Err(Error::<T>::WithdrawFailed);
		}

		log::debug!(
			target: "evm",
			"Withdrew fee from account {:?} in currency {:?} amount {:?}",
			account_id,
			fee_currency,
			fee_amount
		);

		MC::withdraw(fee_currency, &account_id, fee_amount)
			.map_err(|_| Error::<T>::WithdrawFailed)?;

		Ok(Some(EvmPaymentInfo { fee_amount, fee_currency, fee_currency_price, weth_price }))
	}

	fn correct_and_deposit_fee(
		who: &H160,
		corrected_fee: U256,
		_base_fee: U256,
		already_withdrawn: Self::LiquidityInfo,
	) -> Self::LiquidityInfo {
		if let Some(payment_info) = already_withdrawn {
			let account_id = T::AddressMapping::into_account_id(*who);

			let adjusted_paid = if let Some(converted_corrected_fee) = Price::get_amount_by_prices(
				&WETH,
				corrected_fee.unique_saturated_into(),
				payment_info.weth_price,
				&payment_info.fee_currency,
				payment_info.fee_currency_price,
			) {
				// Calculate how much refund we should return
				let refund_amount = payment_info.fee_amount.saturating_sub(converted_corrected_fee);

				// refund to the account that paid the fees. If this fails, the
				// account might have dropped below the existential balance. In
				// that case we don't refund anything.
				let refund_imbalance =
					match MC::deposit(payment_info.fee_currency, &account_id, refund_amount) {
						Ok(_) => 0,
						Err(_) => refund_amount,
					};
				// figure out how much is left to mint back
				// refund_amount already minted back to account, imbalance is what is left to mint
				// if any
				payment_info
					.fee_amount
					.saturating_sub(refund_amount)
					.saturating_add(refund_imbalance)
			} else {
				// if conversion failed for some reason, we refund the whole amount back to treasury
				payment_info.fee_amount
			};

			// We can simply refund all the remaining amount back to treasury
			let result = Currencies::deposit(
				payment_info.fee_currency,
				&crate::BifrostTreasuryAccount::get(),
				adjusted_paid,
			);
			debug_assert_eq!(result, Ok(()));
		}
		None
	}

	fn pay_priority_fee(tip: Self::LiquidityInfo) {
		debug_assert_eq!(tip, None);
	}
}

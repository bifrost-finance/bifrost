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

use crate::{Currencies, ParachainInfo, ZenlinkProtocol};
use bifrost_primitives::{AccountFeeCurrency, Balance, CurrencyId, BNC};
use bifrost_runtime_common::Ratio;
use frame_support::traits::{
	tokens::{Fortitude, Precision},
	Get, OnUnbalanced, TryDrop,
};
use orml_traits::MultiCurrency;
use pallet_evm::{AddressMapping, Error, OnChargeEVMTransaction};
use sp_core::{H160, U256};
use sp_runtime::{
	helpers_128bit::multiply_by_rational_with_rounding,
	traits::{Convert, UniqueSaturatedInto},
	Rounding,
};
use sp_std::marker::PhantomData;
use zenlink_protocol::ExportZenlink;

#[derive(Copy, Clone, Default)]
pub struct EvmPaymentInfo {
	amount: Balance,
	currency_id: CurrencyId,
	price: Ratio,
}

impl EvmPaymentInfo {
	pub fn merge(self, other: Self) -> Self {
		EvmPaymentInfo {
			amount: self.amount.saturating_add(other.amount),
			currency_id: self.currency_id,
			price: self.price,
		}
	}
}

impl TryDrop for EvmPaymentInfo {
	fn try_drop(self) -> Result<(), Self> {
		if self.amount == 0 {
			Ok(())
		} else {
			Err(self)
		}
	}
}

/// Implements the transaction payment for EVM transactions.
/// Supports multi-currency fees based on what is provided by AC - account currency.
pub struct TransferEvmFees<OU, AC, EC, MC>(PhantomData<(OU, AC, EC, MC)>);

impl<T, OU, AC, EC, MC> OnChargeEVMTransaction<T> for TransferEvmFees<OU, AC, EC, MC>
where
	T: pallet_evm::Config,
	OU: OnUnbalanced<EvmPaymentInfo>,
	U256: UniqueSaturatedInto<Balance>,
	AC: AccountFeeCurrency<T::AccountId>, // AccountCurrency
	EC: Get<CurrencyId>,                  // Evm default fee asset
	U256: UniqueSaturatedInto<Balance>,
	MC: frame_support::traits::tokens::fungibles::Mutate<
			T::AccountId,
			AssetId = CurrencyId,
			Balance = Balance,
		> + frame_support::traits::tokens::fungibles::Inspect<
			T::AccountId,
			AssetId = CurrencyId,
			Balance = Balance,
		>,
	sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>
{
	type LiquidityInfo = Option<EvmPaymentInfo>;

	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, pallet_evm::Error<T>> {
		if fee.is_zero() {
			return Ok(None);
		}
		let account_id = T::AddressMapping::into_account_id(*who);
		let fee_currency = AC::get(&account_id);

		let path = [
			EC::get().to_asset_id(ParachainInfo::parachain_id().into()),
			fee_currency.to_asset_id(ParachainInfo::parachain_id().into()),
		];
		let amounts = ZenlinkProtocol::get_amount_in_by_path(fee.unique_saturated_into(), &path)
			.map_err(|_| Error::<T>::BalanceLow)?;

		// let amounts = sp_std::vec![
		// 	fee.unique_saturated_into(),
		// 	180000000000u128
		// ];

		log::debug!(target: "runtime", "===========================amounts{:?}", amounts);

		let converted = amounts[1];
		let price = Ratio::new(amounts[1], amounts[0]);

		// Ensure that converted fee is not zero
		if converted == 0u128 {
			return Err(Error::<T>::WithdrawFailed);
		}

		let burned = MC::burn_from(
			fee_currency,
			&account_id,
			converted,
			Precision::Exact,
			Fortitude::Polite,
		)
		.map_err(|_| Error::<T>::BalanceLow)?;

		Ok(Some(EvmPaymentInfo { amount: burned, currency_id: fee_currency, price }))
	}

	fn correct_and_deposit_fee(
		who: &H160,
		corrected_fee: U256,
		_base_fee: U256,
		already_withdrawn: Self::LiquidityInfo,
	) -> Self::LiquidityInfo {
		if let Some(paid) = already_withdrawn {
			let account_id = T::AddressMapping::into_account_id(*who);

			// fee / weth = amounts[1] / amounts[0]
			// fee =  weth * amounts[1] / amounts[0]
			let adjusted_paid = if let Some(converted_corrected_fee) =
				multiply_by_rational_with_rounding(
					corrected_fee.unique_saturated_into(),
					paid.price.n,
					paid.price.d,
					Rounding::Up,
				) {
				// Calculate how much refund we should return
				let refund_amount = paid.amount.saturating_sub(converted_corrected_fee);

				// refund to the account that paid the fees. If this fails, the
				// account might have dropped below the existential balance. In
				// that case we don't refund anything.
				let result = MC::mint_into(paid.currency_id, &account_id, refund_amount);

				let refund_imbalance = if let Ok(amount) = result {
					// Ensure that we minted all amount, in case of partial refund for some reason,
					// refund the difference back to treasury
					debug_assert_eq!(amount, refund_amount);
					refund_amount.saturating_sub(amount)
				} else {
					// If error, we refund the whole amount back to treasury
					refund_amount
				};
				// figure out how much is left to mint back
				// refund_amount already minted back to account, imbalance is what is left to mint
				// if any
				paid.amount.saturating_sub(refund_amount).saturating_add(refund_imbalance)
			} else {
				// if conversion failed for some reason, we refund the whole amount back to treasury
				paid.amount
			};

			// We can simply refund all the remaining amount back to treasury
			OU::on_unbalanced(EvmPaymentInfo {
				amount: adjusted_paid,
				currency_id: paid.currency_id,
				price: paid.price,
			});
			return None;
		}
		None
	}

	fn pay_priority_fee(tip: Self::LiquidityInfo) {
		if let Some(tip) = tip {
			OU::on_unbalanced(tip);
		}
	}
}

pub struct DepositEvmFeeToTreasury;
impl OnUnbalanced<EvmPaymentInfo> for DepositEvmFeeToTreasury {
	// this is called for substrate-based transactions
	fn on_unbalanceds<B>(amounts: impl Iterator<Item = EvmPaymentInfo>) {
		Self::on_unbalanced(amounts.fold(EvmPaymentInfo::default(), |i, x| x.merge(i)))
	}

	// this is called from pallet_evm for Ethereum-based transactions
	// (technically, it calls on_unbalanced, which calls this when non-zero)
	fn on_nonzero_unbalanced(payment_info: EvmPaymentInfo) {
		let result = Currencies::deposit(
			payment_info.currency_id,
			&crate::BifrostTreasuryAccount::get(),
			payment_info.amount,
		);
		debug_assert_eq!(result, Ok(()));
	}
}

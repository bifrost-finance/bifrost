// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{types::Deposits, AssetIdOf, BalanceOf, *};
use frame_support::{
    require_transactional,
    traits::tokens::{
        fungibles::{Inspect, Transfer},
        DepositConsequence, WithdrawConsequence,
    },
};

impl<T: Config> Inspect<T::AccountId> for Pallet<T> {
    type AssetId = AssetIdOf<T>;
    type Balance = BalanceOf<T>;

    /// The total amount of issuance in the system.
    fn total_issuance(ptoken_id: Self::AssetId) -> Self::Balance {
        if let Ok(underlying_id) = Self::underlying_id(ptoken_id) {
            Self::total_supply(underlying_id)
        } else {
            Balance::default()
        }
    }

    /// The minimum balance any single account may have.
    fn minimum_balance(_ptoken_id: Self::AssetId) -> Self::Balance {
        Zero::zero()
    }

    /// Get the ptoken balance of `who`.
    fn balance(ptoken_id: Self::AssetId, who: &T::AccountId) -> Self::Balance {
        if let Ok(underlying_id) = Self::underlying_id(ptoken_id) {
            Self::account_deposits(underlying_id, who).voucher_balance
        } else {
            Balance::default()
        }
    }

    /// Get the maximum amount that `who` can withdraw/transfer successfully.
    /// For ptoken, We don't care if keep_alive is enabled
    fn reducible_balance(
        ptoken_id: Self::AssetId,
        who: &T::AccountId,
        _keep_alive: bool,
    ) -> Self::Balance {
        Self::reducible_asset(ptoken_id, who).unwrap_or_default()
    }

    /// Returns `true` if the balance of `who` may be increased by `amount`.
    fn can_deposit(
        ptoken_id: Self::AssetId,
        who: &T::AccountId,
        amount: Self::Balance,
        _mint: bool,
    ) -> DepositConsequence {
        let underlying_id = match Self::underlying_id(ptoken_id) {
            Ok(asset_id) => asset_id,
            Err(_) => return DepositConsequence::UnknownAsset,
        };

        if let Err(res) =
            Self::ensure_active_market(underlying_id).map_err(|_| DepositConsequence::UnknownAsset)
        {
            return res;
        }

        if Self::total_supply(underlying_id)
            .checked_add(amount)
            .is_none()
        {
            return DepositConsequence::Overflow;
        }

        if Self::balance(ptoken_id, who) + amount < Self::minimum_balance(ptoken_id) {
            return DepositConsequence::BelowMinimum;
        }

        DepositConsequence::Success
    }

    /// Returns `Failed` if the balance of `who` may not be decreased by `amount`, otherwise
    /// the consequence.
    fn can_withdraw(
        ptoken_id: Self::AssetId,
        who: &T::AccountId,
        amount: Self::Balance,
    ) -> WithdrawConsequence<Self::Balance> {
        let underlying_id = match Self::underlying_id(ptoken_id) {
            Ok(asset_id) => asset_id,
            Err(_) => return WithdrawConsequence::UnknownAsset,
        };

        if let Err(res) =
            Self::ensure_active_market(underlying_id).map_err(|_| WithdrawConsequence::UnknownAsset)
        {
            return res;
        }

        let sub_result = Self::balance(ptoken_id, who).checked_sub(amount);
        if sub_result.is_none() {
            return WithdrawConsequence::NoFunds;
        }

        let rest = sub_result.expect("Cannot be none; qed");
        if rest < Self::minimum_balance(ptoken_id) {
            return WithdrawConsequence::ReducedToZero(rest);
        }

        WithdrawConsequence::Success
    }

    fn asset_exists(ptoken_id: Self::AssetId) -> bool {
        Self::underlying_id(ptoken_id).is_ok()
    }
}

impl<T: Config> Transfer<T::AccountId> for Pallet<T> {
    /// Returns `Err` if the reducible ptoken of `who` is insufficient
    ///
    /// For ptoken, We don't care if keep_alive is enabled
    #[transactional]
    fn transfer(
        ptoken_id: Self::AssetId,
        source: &T::AccountId,
        dest: &T::AccountId,
        amount: Self::Balance,
        _keep_alive: bool,
    ) -> Result<BalanceOf<T>, DispatchError> {
        ensure!(
            amount <= Self::reducible_balance(ptoken_id, source, false),
            Error::<T>::InsufficientCollateral
        );

        Self::do_transfer_ptokens(ptoken_id, source, dest, amount)?;
        Ok(amount)
    }
}

impl<T: Config> Pallet<T> {
    #[require_transactional]
    fn do_transfer_ptokens(
        ptoken_id: AssetIdOf<T>,
        source: &T::AccountId,
        dest: &T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        // update supply index before modify supply balance.
        Self::update_reward_supply_index(ptoken_id)?;
        Self::distribute_supplier_reward(ptoken_id, source)?;
        Self::distribute_supplier_reward(ptoken_id, dest)?;

        let underlying_id = Self::underlying_id(ptoken_id)?;
        AccountDeposits::<T>::try_mutate_exists(
            underlying_id,
            source,
            |deposits| -> DispatchResult {
                let mut d = deposits.unwrap_or_default();
                d.voucher_balance = d
                    .voucher_balance
                    .checked_sub(amount)
                    .ok_or(ArithmeticError::Underflow)?;
                if d.voucher_balance.is_zero() {
                    // remove deposits storage if zero balance
                    *deposits = None;
                } else {
                    *deposits = Some(d);
                }
                Ok(())
            },
        )?;

        AccountDeposits::<T>::try_mutate(underlying_id, dest, |deposits| -> DispatchResult {
            deposits.voucher_balance = deposits
                .voucher_balance
                .checked_add(amount)
                .ok_or(ArithmeticError::Overflow)?;
            Ok(())
        })?;

        Ok(())
    }

    fn reducible_asset(
        ptoken_id: AssetIdOf<T>,
        who: &T::AccountId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let underlying_id = Self::underlying_id(ptoken_id)?;
        let Deposits {
            is_collateral,
            voucher_balance,
        } = Self::account_deposits(underlying_id, who);

        if !is_collateral {
            return Ok(voucher_balance);
        }

        let market = Self::ensure_active_market(underlying_id)?;
        let collateral_value = Self::collateral_asset_value(who, underlying_id)?;

        // liquidity of all assets
        let (liquidity, _, _, _) = Self::get_account_liquidity(who)?;

        if liquidity >= collateral_value {
            return Ok(voucher_balance);
        }

        // Formula
        // reducible_underlying_amount = liquidity / collateral_factor / price
        let price = Self::get_price(underlying_id)?;

        let reducible_supply_value = liquidity
            .checked_div(&market.collateral_factor.into())
            .ok_or(ArithmeticError::Overflow)?;

        let reducible_underlying_amount = reducible_supply_value
            .checked_div(&price)
            .ok_or(ArithmeticError::Underflow)?
            .into_inner();

        let exchange_rate = Self::exchange_rate(underlying_id);
        let amount = Self::calc_collateral_amount(reducible_underlying_amount, exchange_rate)?;
        Ok(amount)
    }
}

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

use sp_io::hashing::blake2_256;
use sp_runtime::{traits::Zero, DispatchResult};

use crate::*;

impl<T: Config> Pallet<T> {
    pub(crate) fn reward_account_id() -> Result<T::AccountId, DispatchError> {
        let account_id: T::AccountId = T::PalletId::get().into_account_truncating();
        let entropy = (b"loans/farming", &[account_id]).using_encoded(blake2_256);
        Ok(T::AccountId::decode(&mut &entropy[..]).map_err(|_| Error::<T>::CodecError)?)
    }

    fn reward_scale() -> u128 {
        10_u128.pow(12)
    }

    fn calculate_reward_delta_index(
        delta_block: T::BlockNumber,
        reward_speed: BalanceOf<T>,
        total_share: BalanceOf<T>,
    ) -> Result<u128, sp_runtime::DispatchError> {
        if total_share.is_zero() {
            return Ok(0);
        }
        let delta_block: BalanceOf<T> = delta_block.saturated_into();
        let delta_index = delta_block
            .get_big_uint()
            .checked_mul(&reward_speed.get_big_uint())
            .and_then(|r| r.checked_mul(&Self::reward_scale().get_big_uint()))
            .and_then(|r| r.checked_div(&total_share.get_big_uint()))
            .and_then(|r| r.to_u128())
            .ok_or(ArithmeticError::Overflow)?;
        Ok(delta_index)
    }

    fn calculate_reward_delta(
        share: BalanceOf<T>,
        reward_delta_index: u128,
    ) -> Result<u128, sp_runtime::DispatchError> {
        let reward_delta = share
            .get_big_uint()
            .checked_mul(&reward_delta_index.get_big_uint())
            .and_then(|r| r.checked_div(&Self::reward_scale().get_big_uint()))
            .and_then(|r| r.to_u128())
            .ok_or(ArithmeticError::Overflow)?;
        Ok(reward_delta)
    }

    pub(crate) fn update_reward_supply_index(asset_id: AssetIdOf<T>) -> DispatchResult {
        let current_block_number = <frame_system::Pallet<T>>::block_number();
        RewardSupplyState::<T>::try_mutate(asset_id, |supply_state| -> DispatchResult {
            let delta_block = current_block_number.saturating_sub(supply_state.block);
            if delta_block.is_zero() {
                return Ok(());
            }
            let supply_speed = RewardSupplySpeed::<T>::get(asset_id);
            if !supply_speed.is_zero() {
                let total_supply = TotalSupply::<T>::get(asset_id);
                let delta_index =
                    Self::calculate_reward_delta_index(delta_block, supply_speed, total_supply)?;
                supply_state.index = supply_state
                    .index
                    .checked_add(delta_index)
                    .ok_or(ArithmeticError::Overflow)?;
            }
            supply_state.block = current_block_number;

            Ok(())
        })
    }

    pub(crate) fn update_reward_borrow_index(asset_id: AssetIdOf<T>) -> DispatchResult {
        let current_block_number = <frame_system::Pallet<T>>::block_number();
        RewardBorrowState::<T>::try_mutate(asset_id, |borrow_state| -> DispatchResult {
            let delta_block = current_block_number.saturating_sub(borrow_state.block);
            if delta_block.is_zero() {
                return Ok(());
            }
            let borrow_speed = RewardBorrowSpeed::<T>::get(asset_id);
            if !borrow_speed.is_zero() {
                let current_borrow_amount = TotalBorrows::<T>::get(asset_id);
                let current_borrow_index = BorrowIndex::<T>::get(asset_id);
                let base_borrow_amount = current_borrow_index
                    .reciprocal()
                    .and_then(|r| r.checked_mul_int(current_borrow_amount))
                    .ok_or(ArithmeticError::Overflow)?;
                let delta_index = Self::calculate_reward_delta_index(
                    delta_block,
                    borrow_speed,
                    base_borrow_amount,
                )?;
                borrow_state.index = borrow_state
                    .index
                    .checked_add(delta_index)
                    .ok_or(ArithmeticError::Overflow)?;
            }
            borrow_state.block = current_block_number;

            Ok(())
        })
    }

    pub(crate) fn distribute_supplier_reward(
        asset_id: AssetIdOf<T>,
        supplier: &T::AccountId,
    ) -> DispatchResult {
        RewardSupplierIndex::<T>::try_mutate(
            asset_id,
            supplier,
            |supplier_index| -> DispatchResult {
                let supply_state = RewardSupplyState::<T>::get(asset_id);
                let delta_index = supply_state
                    .index
                    .checked_sub(*supplier_index)
                    .ok_or(ArithmeticError::Underflow)?;
                *supplier_index = supply_state.index;

                RewardAccrued::<T>::try_mutate(supplier, |total_reward| -> DispatchResult {
                    let supplier_account = AccountDeposits::<T>::get(asset_id, supplier);
                    let supplier_amount = supplier_account.voucher_balance;
                    let reward_delta = Self::calculate_reward_delta(supplier_amount, delta_index)?;
                    *total_reward = total_reward
                        .checked_add(reward_delta)
                        .ok_or(ArithmeticError::Overflow)?;
                    Self::deposit_event(Event::<T>::DistributedSupplierReward(
                        asset_id,
                        supplier.clone(),
                        reward_delta,
                        supply_state.index,
                    ));

                    Ok(())
                })
            },
        )
    }

    pub(crate) fn distribute_borrower_reward(
        asset_id: AssetIdOf<T>,
        borrower: &T::AccountId,
    ) -> DispatchResult {
        RewardBorrowerIndex::<T>::try_mutate(
            asset_id,
            borrower,
            |borrower_index| -> DispatchResult {
                let borrow_state = RewardBorrowState::<T>::get(asset_id);
                let delta_index = borrow_state
                    .index
                    .checked_sub(*borrower_index)
                    .ok_or(ArithmeticError::Underflow)?;
                *borrower_index = borrow_state.index;

                RewardAccrued::<T>::try_mutate(borrower, |total_reward| -> DispatchResult {
                    let current_borrow_amount = Self::current_borrow_balance(borrower, asset_id)?;
                    let current_borrow_index = BorrowIndex::<T>::get(asset_id);
                    let base_borrow_amount = current_borrow_index
                        .reciprocal()
                        .and_then(|r| r.checked_mul_int(current_borrow_amount))
                        .ok_or(ArithmeticError::Overflow)?;
                    let reward_delta =
                        Self::calculate_reward_delta(base_borrow_amount, delta_index)?;
                    *total_reward = total_reward
                        .checked_add(reward_delta)
                        .ok_or(ArithmeticError::Overflow)?;
                    Self::deposit_event(Event::<T>::DistributedBorrowerReward(
                        asset_id,
                        borrower.clone(),
                        reward_delta,
                        borrow_state.index,
                    ));

                    Ok(())
                })
            },
        )
    }

    pub(crate) fn collect_market_reward(
        asset_id: AssetIdOf<T>,
        user: &T::AccountId,
    ) -> DispatchResult {
        Self::update_reward_supply_index(asset_id)?;
        Self::distribute_supplier_reward(asset_id, user)?;

        Self::update_reward_borrow_index(asset_id)?;
        Self::distribute_borrower_reward(asset_id, user)?;

        Ok(())
    }

    pub(crate) fn pay_reward(user: &T::AccountId) -> DispatchResult {
        let pool_account = Self::reward_account_id()?;
        let reward_asset = T::RewardAssetId::get();
        let total_reward = RewardAccrued::<T>::get(user);
        if total_reward > 0 {
            T::Assets::transfer(reward_asset, &pool_account, user, total_reward, true)?;
            RewardAccrued::<T>::remove(user);
        }
        Self::deposit_event(Event::<T>::RewardPaid(user.clone(), total_reward));
        Ok(())
    }
}

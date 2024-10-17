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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use crate::{
	AccountIdOf, BalanceOf, Config, CurrencyIdOf, Error, Event, Fees, HookIterationLimit,
	MinTimeUnit, MinimumMint, MinimumRedeem, MintWithLockBlocks, OnRedeemSuccess, OngoingTimeUnit,
	Pallet, RedeemTo, TimeUnitUnlockLedger, TokenPool, TokenUnlockLedger, TokenUnlockNextId,
	UnlockDuration, UnlockId, UnlockingTotal, UserUnlockLedger, VtokenIncentiveCoef,
	VtokenLockLedger, WeightInfo,
};
use bb_bnc::traits::BbBNCInterface;
use bifrost_primitives::{
	currency::BNC, AstarChainId, CurrencyId, CurrencyIdExt, HydrationChainId, InterlayChainId,
	MantaChainId, RedeemType, SlpxOperator, TimeUnit, VTokenMintRedeemProvider,
	VTokenSupplyProvider, VtokenMintingInterface, VtokenMintingOperator, FIL,
};
use frame_support::{
	pallet_prelude::{DispatchResultWithPostInfo, *},
	sp_runtime::{
		traits::{AccountIdConversion, CheckedAdd, CheckedSub, UniqueSaturatedInto, Zero},
		DispatchError, FixedU128, Permill, SaturatedConversion,
	},
	traits::LockIdentifier,
	transactional, BoundedVec,
};
use frame_system::pallet_prelude::*;
use orml_traits::{MultiCurrency, MultiLockableCurrency, XcmTransfer};
use sp_core::U256;
use sp_runtime::{helpers_128bit::multiply_by_rational_with_rounding, Rounding};
use sp_std::{vec, vec::Vec};
use xcm::{prelude::*, v4::Location};

// incentive lock id for vtoken minted by user
const INCENTIVE_LOCK_ID: LockIdentifier = *b"vmincntv";

#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
	Set,
	Add,
	Sub,
}

impl<T: Config> Pallet<T> {
	/// Update the token pool amount.
	/// Parameters:
	/// - `currency_id`: The currency id.
	/// - `currency_amount`: The currency amount.
	/// - `operation`: The operation type. Set, Add, Sub.
	pub fn update_token_pool(
		currency_id: &CurrencyId,
		currency_amount: &BalanceOf<T>,
		operation: Operation,
	) -> DispatchResult {
		TokenPool::<T>::mutate(currency_id, |token_pool_amount| -> DispatchResult {
			match operation {
				Operation::Set => *token_pool_amount = *currency_amount,
				Operation::Add =>
					*token_pool_amount = token_pool_amount
						.checked_add(currency_amount)
						.ok_or(Error::<T>::CalculationOverflow)?,
				Operation::Sub =>
					*token_pool_amount = token_pool_amount
						.checked_sub(currency_amount)
						.ok_or(Error::<T>::CalculationOverflow)?,
			}
			Ok(())
		})
	}

	/// Update the unlocking total amount.
	/// Parameters:
	/// - `currency_id`: The currency id.
	/// - `currency_amount`: The currency amount.
	/// - `operation`: The operation type. Set, Add, Sub.
	pub fn update_unlocking_total(
		currency_id: &CurrencyId,
		currency_amount: &BalanceOf<T>,
		operation: Operation,
	) -> DispatchResult {
		UnlockingTotal::<T>::mutate(currency_id, |unlocking_total_amount| -> DispatchResult {
			match operation {
				Operation::Set => *unlocking_total_amount = *currency_amount,
				Operation::Add =>
					*unlocking_total_amount = unlocking_total_amount
						.checked_add(currency_amount)
						.ok_or(Error::<T>::CalculationOverflow)?,
				Operation::Sub =>
					*unlocking_total_amount = unlocking_total_amount
						.checked_sub(currency_amount)
						.ok_or(Error::<T>::CalculationOverflow)?,
			}
			Ok(())
		})
	}

	/// Update the token unlock ledger.
	/// Parameters:
	/// - `currency_id`: The currency id.
	/// - `currency_amount`: The redeem currency amount.
	/// - `unlock_id`: The unlock id.
	/// - `lock_to_time_unit`: The lock to time unit.
	/// - `redeem_type`: The redeem type.
	/// Returns:
	/// - `bool`: Whether the record is removed.
	pub fn update_token_unlock_ledger(
		redeemer: &AccountIdOf<T>,
		currency_id: &CurrencyId,
		currency_amount: &BalanceOf<T>,
		unlock_id: &UnlockId,
		lock_to_time_unit: &TimeUnit,
		redeem_type: Option<RedeemType<AccountIdOf<T>>>,
		operation: Operation,
	) -> Result<bool, Error<T>> {
		TokenUnlockLedger::<T>::mutate_exists(currency_id, unlock_id, |value| match operation {
			Operation::Set | Operation::Add => {
				let redeem_type = redeem_type.ok_or(Error::<T>::TimeUnitUnlockLedgerNotFound)?;
				*value = Some((
					redeemer.clone(),
					*currency_amount,
					lock_to_time_unit.clone(),
					redeem_type,
				));
				Ok(false)
			},
			Operation::Sub => {
				let (_, total_locked_amount, _, _) =
					value.as_mut().ok_or(Error::<T>::TimeUnitUnlockLedgerNotFound)?;

				if currency_amount >= total_locked_amount {
					*value = None;
					Ok(true)
				} else {
					*total_locked_amount = total_locked_amount
						.checked_sub(currency_amount)
						.ok_or(Error::<T>::CalculationOverflow)?;
					Ok(false)
				}
			},
		})
	}

	/// Update the time unit unlock ledger.
	/// Parameters:
	/// - `time_unit`: The time unit.
	/// - `currency_id`: The currency id.
	/// - `currency_amount`: The redeem currency amount.
	/// - `unlock_id`: The unlock id.
	/// - `operation`: The operation type. Set, Add, Sub.
	/// - `is_remove_record`: Whether to remove the record.
	pub fn update_time_unit_unlock_ledger(
		time_unit: &TimeUnit,
		currency_id: &CurrencyId,
		currency_amount: &BalanceOf<T>,
		unlock_id: &UnlockId,
		operation: Operation,
		is_remove_record: bool,
	) -> DispatchResult {
		TimeUnitUnlockLedger::<T>::mutate_exists(time_unit, currency_id, |unlocking_ledger| {
			match operation {
				Operation::Set | Operation::Add => match unlocking_ledger {
					Some((total_locked, ledger_list, _token_id)) => {
						ledger_list.try_push(*unlock_id).map_err(|_| Error::<T>::TooManyRedeems)?;

						*total_locked = total_locked
							.checked_add(&currency_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
					},
					None =>
						*unlocking_ledger = Some((
							*currency_amount,
							BoundedVec::try_from(vec![*unlock_id])
								.map_err(|_| Error::<T>::TooManyRedeems)?,
							*currency_id,
						)),
				},
				Operation::Sub => {
					let (total_locked_amount, ledger_list, _) = unlocking_ledger
						.as_mut()
						.ok_or(Error::<T>::TimeUnitUnlockLedgerNotFound)?;

					if currency_amount >= total_locked_amount {
						*unlocking_ledger = None;
					} else {
						*total_locked_amount = total_locked_amount
							.checked_sub(currency_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						if is_remove_record {
							ledger_list.retain(|x| x != unlock_id);
						}
					}
				},
			}
			Ok(())
		})
	}

	/// Update the user unlock ledger.
	/// Parameters:
	/// - `account`: The account id.
	/// - `currency_id`: The currency id.
	/// - `currency_amount`: The redeem currency amount.
	/// - `unlock_id`: The unlock id.
	/// - `operation`: The operation type. Set, Add, Sub.
	/// - `is_remove_record`: Whether to remove the record.
	pub fn update_user_unlock_ledger(
		account: &AccountIdOf<T>,
		currency_id: &CurrencyId,
		currency_amount: &BalanceOf<T>,
		unlock_id: &UnlockId,
		operation: Operation,
		is_remove_record: bool,
	) -> Result<(), Error<T>> {
		UserUnlockLedger::<T>::mutate_exists(account, currency_id, |user_unlock_ledger| {
			match operation {
				Operation::Set | Operation::Add => match user_unlock_ledger {
					Some((total_locked, ledger_list)) => {
						ledger_list.try_push(*unlock_id).map_err(|_| Error::<T>::TooManyRedeems)?;

						*total_locked = total_locked
							.checked_add(&currency_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
					},
					None => {
						*user_unlock_ledger = Some((
							*currency_amount,
							BoundedVec::try_from(vec![*unlock_id])
								.map_err(|_| Error::<T>::TooManyRedeems)?,
						));
					},
				},
				Operation::Sub => {
					let (total_locked_amount, ledger_list) = user_unlock_ledger
						.as_mut()
						.ok_or(Error::<T>::TimeUnitUnlockLedgerNotFound)?;

					if currency_amount >= total_locked_amount {
						*user_unlock_ledger = None;
					} else {
						*total_locked_amount = total_locked_amount
							.checked_sub(currency_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;
						if is_remove_record {
							ledger_list.retain(|x| x != unlock_id);
						}
					}
				},
			}
			Ok(())
		})
	}

	/// Update the token lock ledger.
	/// Parameters:
	/// - `account`: The account id.
	/// - `currency_id`: The currency id.
	/// - `currency_amount`: The redeem currency amount.
	/// - `unlock_id`: The unlock id.
	/// - `lock_to_time_unit`: The lock to time unit.
	/// - `redeem_type`: The redeem type.
	/// - `operation`: The operation type. Set, Add, Sub.
	pub fn update_unlock_ledger(
		account: &AccountIdOf<T>,
		currency_id: &CurrencyId,
		currency_amount: &BalanceOf<T>,
		unlock_id: &UnlockId,
		lock_to_time_unit: &TimeUnit,
		redeem_type: Option<RedeemType<AccountIdOf<T>>>,
		operation: Operation,
	) -> Result<bool, DispatchError> {
		let is_remove_record = Self::update_token_unlock_ledger(
			account,
			currency_id,
			currency_amount,
			unlock_id,
			lock_to_time_unit,
			redeem_type,
			operation,
		)?;
		Self::update_user_unlock_ledger(
			account,
			currency_id,
			currency_amount,
			unlock_id,
			operation,
			is_remove_record,
		)?;
		Self::update_time_unit_unlock_ledger(
			lock_to_time_unit,
			currency_id,
			currency_amount,
			unlock_id,
			operation,
			is_remove_record,
		)?;
		Self::update_unlocking_total(&currency_id, &currency_amount, operation)?;
		Ok(is_remove_record)
	}

	/// Mint without transfer.
	/// Parameters:
	/// - `minter`: The minter account id.
	/// - `v_currency_id`: The v_currency id.
	/// - `currency_id`: The currency id.
	/// - `currency_amount`: The currency amount.
	/// Returns:
	/// - `(BalanceOf<T>, BalanceOf<T>, BalanceOf<T>)`: The currency amount, v_currency amount, mint
	///   fee.
	pub fn mint_without_transfer(
		minter: &AccountIdOf<T>,
		v_currency_id: CurrencyId,
		currency_id: CurrencyId,
		currency_amount: BalanceOf<T>,
	) -> Result<(BalanceOf<T>, BalanceOf<T>, BalanceOf<T>), DispatchError> {
		let (mint_rate, _) = Fees::<T>::get();
		let mint_fee = mint_rate.mul_floor(currency_amount);
		// Charging fees
		T::MultiCurrency::transfer(currency_id, minter, &T::FeeAccount::get(), mint_fee)?;

		let currency_amount =
			currency_amount.checked_sub(&mint_fee).ok_or(Error::<T>::CalculationOverflow)?;
		let v_currency_amount = Self::get_v_currency_amount_by_currency_amount(
			currency_id,
			v_currency_id,
			currency_amount,
		)?;

		// Issue the corresponding v_currency to the user's account.
		T::MultiCurrency::deposit(v_currency_id, minter, v_currency_amount)?;
		// Increase the token pool amount.
		Self::update_token_pool(&currency_id, &currency_amount, Operation::Add)?;

		Ok((currency_amount, v_currency_amount, mint_fee))
	}

	/// Process redeem.
	/// Parameters:
	/// - `redeem_currency_id`: The redeem currency id.
	/// - `redeemer`: The redeemer account id.
	/// - `unlock_id`: The unlock id.
	/// - `redeem_currency_amount`: The redeem currency amount.
	/// - `entrance_account_balance`: The entrance account balance.
	/// - `time_unit`: The time unit.
	/// - `redeem_type`: The redeem type.
	fn process_redeem(
		redeem_currency_id: CurrencyId,
		redeemer: AccountIdOf<T>,
		unlock_id: &UnlockId,
		redeem_currency_amount: BalanceOf<T>,
		entrance_account_balance: BalanceOf<T>,
		time_unit: TimeUnit,
		redeem_type: RedeemType<AccountIdOf<T>>,
	) -> DispatchResult {
		let (redeem_currency_amount, redeem_to) = Self::transfer_to_by_redeem_type(
			redeemer.clone(),
			redeem_currency_id,
			redeem_currency_amount,
			entrance_account_balance,
			redeem_type,
		)?;

		Self::update_unlock_ledger(
			&redeemer,
			&redeem_currency_id,
			&redeem_currency_amount,
			unlock_id,
			&time_unit,
			None,
			Operation::Sub,
		)?;

		T::OnRedeemSuccess::on_redeem_success(
			redeem_currency_id,
			redeemer.clone(),
			redeem_currency_amount,
		);

		Self::deposit_event(Event::RedeemSuccess {
			redeemer,
			unlock_id: *unlock_id,
			currency_id: redeem_currency_id,
			to: redeem_to,
			currency_amount: redeem_currency_amount,
		});
		Ok(())
	}

	/// Transfer to by redeem type.
	/// Parameters:
	/// - `redeemer`: The redeemer account id.
	/// - `redeem_currency_id`: The redeem currency id.
	/// - `redeem_currency_amount`: The redeem currency amount.
	/// - `entrance_account_balance`: The entrance account balance.
	/// - `redeem_type`: The redeem type.
	/// Returns:
	/// - `(BalanceOf<T>, RedeemTo<T::AccountId>)`: The redeem currency amount, redeem to.
	pub fn transfer_to_by_redeem_type(
		redeemer: T::AccountId,
		redeem_currency_id: CurrencyId,
		mut redeem_currency_amount: BalanceOf<T>,
		entrance_account_balance: BalanceOf<T>,
		redeem_type: RedeemType<T::AccountId>,
	) -> Result<(BalanceOf<T>, RedeemTo<T::AccountId>), DispatchError> {
		let entrance_account = T::EntranceAccount::get().into_account_truncating();
		if entrance_account_balance >= redeem_currency_amount {
			if let RedeemType::Native = redeem_type {
				let ed = T::MultiCurrency::minimum_balance(redeem_currency_id);
				if redeem_currency_amount >= ed {
					T::MultiCurrency::transfer(
						redeem_currency_id,
						&entrance_account,
						&redeemer,
						redeem_currency_amount,
					)?;
				}
				return Ok((redeem_currency_amount, RedeemTo::Native(redeemer)));
			}
			let (dest, redeem_to) = match redeem_type {
				RedeemType::Astar(receiver) => (
					Location::new(
						1,
						[
							Parachain(AstarChainId::get()),
							AccountId32 {
								network: None,
								id: receiver.encode().try_into().unwrap(),
							},
						],
					),
					RedeemTo::Astar(receiver),
				),
				RedeemType::Hydradx(receiver) => (
					Location::new(
						1,
						[
							Parachain(HydrationChainId::get()),
							AccountId32 {
								network: None,
								id: receiver.encode().try_into().unwrap(),
							},
						],
					),
					RedeemTo::Hydradx(receiver),
				),
				RedeemType::Interlay(receiver) => (
					Location::new(
						1,
						[
							Parachain(InterlayChainId::get()),
							AccountId32 {
								network: None,
								id: receiver.encode().try_into().unwrap(),
							},
						],
					),
					RedeemTo::Interlay(receiver),
				),
				RedeemType::Manta(receiver) => (
					Location::new(
						1,
						[
							Parachain(MantaChainId::get()),
							AccountId32 {
								network: None,
								id: receiver.encode().try_into().unwrap(),
							},
						],
					),
					RedeemTo::Manta(receiver),
				),
				RedeemType::Moonbeam(receiver) => (
					Location::new(
						1,
						[
							Parachain(T::MoonbeamChainId::get()),
							AccountKey20 { network: None, key: receiver.to_fixed_bytes() },
						],
					),
					RedeemTo::Moonbeam(receiver),
				),
				RedeemType::Native => {
					unreachable!()
				},
			};
			if redeem_currency_id == FIL {
				let assets = vec![
					(redeem_currency_id, redeem_currency_amount),
					(BNC, T::BifrostSlpx::get_moonbeam_transfer_to_fee()),
				];

				T::XcmTransfer::transfer_multicurrencies(
					entrance_account.clone(),
					assets,
					1,
					dest,
					Unlimited,
				)?;
			} else {
				T::XcmTransfer::transfer(
					entrance_account.clone(),
					redeem_currency_id,
					redeem_currency_amount,
					dest,
					Unlimited,
				)?;
			};
			Ok((redeem_currency_amount, redeem_to))
		} else {
			redeem_currency_amount = entrance_account_balance;
			let ed = T::MultiCurrency::minimum_balance(redeem_currency_id);
			if redeem_currency_amount >= ed {
				T::MultiCurrency::transfer(
					redeem_currency_id,
					&entrance_account,
					&redeemer,
					redeem_currency_amount,
				)?;
			}
			Ok((redeem_currency_amount, RedeemTo::Native(redeemer)))
		}
	}

	#[transactional]
	pub fn handle_ledger_by_currency(currency: CurrencyId) -> DispatchResult {
		let time_unit = MinTimeUnit::<T>::get(currency);
		if let Some((_total_locked, ledger_list, currency_id)) =
			TimeUnitUnlockLedger::<T>::get(&time_unit, currency)
		{
			for unlock_id in ledger_list.iter().take(HookIterationLimit::<T>::get() as usize) {
				if let Some((account, unlock_amount, time_unit, redeem_type)) =
					TokenUnlockLedger::<T>::get(currency_id, unlock_id)
				{
					let entrance_account_balance = T::MultiCurrency::free_balance(
						currency_id,
						&T::EntranceAccount::get().into_account_truncating(),
					);
					if entrance_account_balance == BalanceOf::<T>::zero() {
						break;
					}

					Self::process_redeem(
						currency_id,
						account,
						unlock_id,
						unlock_amount,
						entrance_account_balance,
						time_unit,
						redeem_type,
					)?;
				}
			}
		} else {
			MinTimeUnit::<T>::mutate(currency, |time_unit| -> Result<(), Error<T>> {
				let unlock_duration =
					UnlockDuration::<T>::get(currency).ok_or(Error::<T>::UnlockDurationNotFound)?;
				let ongoing_time =
					OngoingTimeUnit::<T>::get(currency).ok_or(Error::<T>::OngoingTimeUnitNotSet)?;
				let result_time_unit =
					ongoing_time.add(unlock_duration).ok_or(Error::<T>::CalculationOverflow)?;
				if result_time_unit.gt(time_unit) {
					*time_unit = time_unit.clone().add_one();
				}
				Ok(())
			})?;
		};

		Ok(())
	}

	pub fn do_mint(
		minter: AccountIdOf<T>,
		currency_id: CurrencyIdOf<T>,
		currency_amount: BalanceOf<T>,
		remark: BoundedVec<u8, ConstU32<32>>,
		channel_id: Option<u32>,
	) -> Result<BalanceOf<T>, DispatchError> {
		ensure!(
			currency_amount >= MinimumMint::<T>::get(currency_id),
			Error::<T>::BelowMinimumMint
		);
		let v_currency_id = currency_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;

		let (currency_amount_excluding_fee, v_currency_amount, mint_fee) =
			Self::mint_without_transfer(&minter, v_currency_id, currency_id, currency_amount)?;

		// Transfer the user's token to EntranceAccount.
		T::MultiCurrency::transfer(
			currency_id,
			&minter,
			&T::EntranceAccount::get().into_account_truncating(),
			currency_amount_excluding_fee,
		)?;

		// record the minting information for ChannelCommission module
		T::ChannelCommission::record_mint_amount(channel_id, v_currency_id, v_currency_amount)?;

		Self::deposit_event(Event::Minted {
			minter,
			currency_id,
			currency_amount,
			v_currency_amount,
			mint_fee,
			remark,
			channel_id,
		});
		Ok(v_currency_amount.into())
	}

	pub fn do_redeem(
		redeemer: AccountIdOf<T>,
		v_currency_id: CurrencyIdOf<T>,
		v_currency_amount: BalanceOf<T>,
		redeem_type: RedeemType<AccountIdOf<T>>,
	) -> DispatchResultWithPostInfo {
		let currency_id = v_currency_id.to_token().map_err(|_| Error::<T>::NotSupportTokenType)?;
		ensure!(
			v_currency_amount >= MinimumRedeem::<T>::get(v_currency_id),
			Error::<T>::BelowMinimumRedeem
		);

		// Charging fees
		let (_, redeem_rate) = Fees::<T>::get();
		let redeem_fee = redeem_rate.mul_floor(v_currency_amount);
		T::MultiCurrency::transfer(
			v_currency_id,
			&redeemer,
			&T::RedeemFeeAccount::get(),
			redeem_fee,
		)?;

		// Calculate the currency amount by v_currency_amount
		let v_currency_amount = v_currency_amount
			.checked_sub(&redeem_fee)
			.ok_or(Error::<T>::CalculationOverflow)?;
		let currency_amount = Self::get_currency_amount_by_v_currency_amount(
			currency_id,
			v_currency_id,
			v_currency_amount,
		)?;

		// Withdraw the token from redeemer
		T::MultiCurrency::withdraw(v_currency_id, &redeemer, v_currency_amount)?;

		// Calculate the time to be locked
		let ongoing_time_unit =
			OngoingTimeUnit::<T>::get(currency_id).ok_or(Error::<T>::OngoingTimeUnitNotSet)?;
		let unlock_duration =
			UnlockDuration::<T>::get(currency_id).ok_or(Error::<T>::UnlockDurationNotFound)?;
		let lock_to_time_unit = ongoing_time_unit
			.add(unlock_duration)
			.ok_or(Error::<T>::UnlockDurationNotFound)?;

		// Decrease the token pool amount
		Self::update_token_pool(&currency_id, &currency_amount, Operation::Sub)?;

		TokenUnlockNextId::<T>::mutate(&currency_id, |next_id| -> DispatchResultWithPostInfo {
			Self::update_unlock_ledger(
				&redeemer,
				&currency_id,
				&currency_amount,
				&next_id,
				&lock_to_time_unit,
				Some(redeem_type),
				Operation::Add,
			)?;

			Self::deposit_event(Event::Redeemed {
				redeemer: redeemer.clone(),
				currency_id,
				v_currency_amount,
				currency_amount,
				redeem_fee,
				unlock_id: *next_id,
			});

			// Increase the next unlock id
			*next_id = next_id.checked_add(1).ok_or(Error::<T>::CalculationOverflow)?;

			T::ChannelCommission::record_redeem_amount(v_currency_id, v_currency_amount)?;
			let extra_weight = T::OnRedeemSuccess::on_redeemed(
				redeemer,
				currency_id,
				currency_amount,
				v_currency_amount,
				redeem_fee,
			);
			Ok(Some(T::WeightInfo::redeem() + extra_weight).into())
		})
	}

	pub fn incentive_pool_account() -> AccountIdOf<T> {
		T::IncentivePoolAccount::get().into_account_truncating()
	}

	// to lock user vtoken for incentive minting
	pub fn lock_vtoken_for_incentive_minting(
		minter: AccountIdOf<T>,
		v_currency_id: CurrencyIdOf<T>,
		v_currency_amount: BalanceOf<T>,
	) -> Result<(), Error<T>> {
		// first, lock the vtoken
		// second, record the lock in ledger

		// check whether the minter has enough vtoken
		T::MultiCurrency::ensure_can_withdraw(v_currency_id, &minter, v_currency_amount)
			.map_err(|_| Error::<T>::NotEnoughBalance)?;

		// new amount that should be locked
		let mut new_lock_total = v_currency_amount;

		// check the previous locked amount under the same v_currency_id from ledger
		// and revise ledger to set the new_amount to be previous_amount + v_currency_amount
		VtokenLockLedger::<T>::mutate_exists(
			&minter,
			&v_currency_id,
			|v_token_lock_ledger| -> Result<(), Error<T>> {
				// get the vtoken lock duration from VtokenIncentiveCoef
				let lock_duration = MintWithLockBlocks::<T>::get(v_currency_id)
					.ok_or(Error::<T>::IncentiveLockBlocksNotSet)?;
				let current_block = frame_system::Pallet::<T>::block_number();
				let due_block = current_block
					.checked_add(&lock_duration)
					.ok_or(Error::<T>::CalculationOverflow)?;

				match v_token_lock_ledger {
					Some((total_locked, lock_records)) => {
						// check the total locked amount
						new_lock_total = total_locked
							.checked_add(&v_currency_amount)
							.ok_or(Error::<T>::CalculationOverflow)?;

						*total_locked = new_lock_total;

						// push new item to the boundedvec of the ledger
						lock_records
							.try_push((v_currency_amount, due_block))
							.map_err(|_| Error::<T>::TooManyLocks)?;
					},
					None =>
						*v_token_lock_ledger = Some((
							v_currency_amount,
							BoundedVec::try_from(vec![(v_currency_amount, due_block)])
								.map_err(|_| Error::<T>::TooManyLocks)?,
						)),
				}

				// extend the locked amount to be new_lock_total
				T::MultiCurrency::set_lock(
					INCENTIVE_LOCK_ID,
					v_currency_id,
					&minter,
					new_lock_total,
				)
				.map_err(|_| Error::<T>::NotEnoughBalance)
			},
		)
	}

	pub fn calculate_incentive_vtoken_amount(
		minter: &AccountIdOf<T>,
		v_currency_id: CurrencyIdOf<T>,
		v_currency_amount: BalanceOf<T>,
	) -> Result<BalanceOf<T>, Error<T>> {
		// get the vtoken pool balance
		let vtoken_pool_balance =
			T::MultiCurrency::free_balance(v_currency_id, &Self::incentive_pool_account());
		ensure!(vtoken_pool_balance > BalanceOf::<T>::zero(), Error::<T>::NotEnoughBalance);

		// get current block number
		let current_block_number: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number();
		// get the veBNC total amount
		let vebnc_total_issuance = T::BbBNC::total_supply(current_block_number)
			.map_err(|_| Error::<T>::VeBNCCheckingError)?;
		ensure!(vebnc_total_issuance > BalanceOf::<T>::zero(), Error::<T>::BalanceZero);

		// get the veBNC balance of the minter
		let minter_vebnc_balance =
			T::BbBNC::balance_of(minter, None).map_err(|_| Error::<T>::VeBNCCheckingError)?;
		ensure!(minter_vebnc_balance > BalanceOf::<T>::zero(), Error::<T>::NotEnoughBalance);

		// get the percentage of the veBNC balance of the minter to the total veBNC amount and
		// get the square root of the percentage
		let percentage = Permill::from_rational(minter_vebnc_balance, vebnc_total_issuance);
		let sqrt_percentage =
			FixedU128::from_inner(percentage * 1_000_000_000_000_000_000u128).sqrt();
		let percentage = Permill::from_rational(
			sqrt_percentage.into_inner(),
			1_000_000_000_000_000_000u128.into(),
		);
		// get the total issuance of the vtoken
		let v_currency_total_issuance = T::MultiCurrency::total_issuance(v_currency_id);

		// get the incentive coef for the vtoken
		let incentive_coef = VtokenIncentiveCoef::<T>::get(v_currency_id)
			.ok_or(Error::<T>::IncentiveCoefNotFound)?;

		// calculate the incentive amount, but mind the overflow
		// incentive_amount = vtoken_pool_balance * incentive_coef * v_currency_amount *
		// sqrt_percentage / v_currency_total_issuance
		let incentive_amount =
			U256::from(percentage.mul_ceil(vtoken_pool_balance).saturated_into::<u128>())
				.checked_mul(U256::from(incentive_coef))
				.and_then(|x| x.checked_mul(U256::from(v_currency_amount.saturated_into::<u128>())))
				// .and_then(|x| x.checked_mul(percentage))
				.and_then(|x| {
					x.checked_div(U256::from(v_currency_total_issuance.saturated_into::<u128>()))
				})
				// first turn into u128，then use unique_saturated_into BalanceOf<T>
				.map(|x| x.saturated_into::<u128>())
				.map(|x| x.unique_saturated_into())
				.ok_or(Error::<T>::CalculationOverflow)?;

		Ok(incentive_amount)
	}
}

impl<T: Config> VtokenMintingOperator<CurrencyId, BalanceOf<T>, AccountIdOf<T>, TimeUnit>
	for Pallet<T>
{
	fn get_token_pool(currency_id: CurrencyId) -> BalanceOf<T> {
		TokenPool::<T>::get(currency_id)
	}

	fn increase_token_pool(
		currency_id: CurrencyId,
		currency_amount: BalanceOf<T>,
	) -> DispatchResult {
		Self::update_token_pool(&currency_id, &currency_amount, Operation::Add)
	}

	fn decrease_token_pool(
		currency_id: CurrencyId,
		currency_amount: BalanceOf<T>,
	) -> DispatchResult {
		Self::update_token_pool(&currency_id, &currency_amount, Operation::Sub)
	}

	fn update_ongoing_time_unit(currency_id: CurrencyId, time_unit: TimeUnit) -> DispatchResult {
		OngoingTimeUnit::<T>::mutate(currency_id, |time_unit_old| -> Result<(), Error<T>> {
			*time_unit_old = Some(time_unit);
			Ok(())
		})?;

		Ok(())
	}

	fn get_ongoing_time_unit(currency_id: CurrencyId) -> Option<TimeUnit> {
		OngoingTimeUnit::<T>::get(currency_id)
	}

	fn get_unlock_records(
		currency_id: CurrencyId,
		time_unit: TimeUnit,
	) -> Option<(BalanceOf<T>, Vec<u32>)> {
		if let Some((balance, list, _)) = TimeUnitUnlockLedger::<T>::get(&time_unit, currency_id) {
			Some((balance, list.into_inner()))
		} else {
			None
		}
	}

	fn get_entrance_and_exit_accounts() -> (AccountIdOf<T>, AccountIdOf<T>) {
		(
			T::EntranceAccount::get().into_account_truncating(),
			T::ExitAccount::get().into_account_truncating(),
		)
	}

	fn get_token_unlock_ledger(
		currency_id: CurrencyId,
		index: u32,
	) -> Option<(AccountIdOf<T>, BalanceOf<T>, TimeUnit, RedeemType<AccountIdOf<T>>)> {
		TokenUnlockLedger::<T>::get(currency_id, index)
	}

	fn get_moonbeam_parachain_id() -> u32 {
		T::MoonbeamChainId::get()
	}
}

impl<T: Config> VtokenMintingInterface<AccountIdOf<T>, CurrencyIdOf<T>, BalanceOf<T>>
	for Pallet<T>
{
	fn mint(
		exchanger: AccountIdOf<T>,
		currency_id: CurrencyIdOf<T>,
		currency_amount: BalanceOf<T>,
		remark: BoundedVec<u8, ConstU32<32>>,
		channel_id: Option<u32>,
	) -> Result<BalanceOf<T>, DispatchError> {
		Self::do_mint(exchanger, currency_id, currency_amount, remark, channel_id)
	}

	fn redeem(
		exchanger: AccountIdOf<T>,
		v_currency_id: CurrencyIdOf<T>,
		v_currency_amount: BalanceOf<T>,
	) -> DispatchResultWithPostInfo {
		Self::do_redeem(exchanger, v_currency_id, v_currency_amount, RedeemType::Native)
	}

	fn slpx_redeem(
		exchanger: AccountIdOf<T>,
		v_currency_id: CurrencyIdOf<T>,
		v_currency_amount: BalanceOf<T>,
		redeem_type: RedeemType<AccountIdOf<T>>,
	) -> DispatchResultWithPostInfo {
		Self::do_redeem(exchanger, v_currency_id, v_currency_amount, redeem_type)
	}

	fn get_v_currency_amount_by_currency_amount(
		currency_id: CurrencyIdOf<T>,
		v_currency_id: CurrencyIdOf<T>,
		currency_amount: BalanceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let token_pool_amount = TokenPool::<T>::get(currency_id);
		let v_currency_total_issuance = T::MultiCurrency::total_issuance(v_currency_id);

		if BalanceOf::<T>::zero().eq(&token_pool_amount) {
			Ok(currency_amount)
		} else {
			Ok(multiply_by_rational_with_rounding(
				currency_amount.saturated_into::<u128>(),
				v_currency_total_issuance.saturated_into::<u128>(),
				token_pool_amount.saturated_into::<u128>(),
				Rounding::Down,
			)
			.ok_or(Error::<T>::CalculationOverflow)?
			.unique_saturated_into())
		}
	}

	/// Get the v_currency amount by currency amount.
	/// Parameters:
	/// - `currency_id`: The currency id.
	/// - `v_currency_id`: The v_currency id.
	/// - `currency_amount`: The currency amount.
	/// Returns:
	/// - `Result`: The v_currency amount.
	fn get_currency_amount_by_v_currency_amount(
		currency_id: CurrencyIdOf<T>,
		v_currency_id: CurrencyIdOf<T>,
		v_currency_amount: BalanceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let token_pool_amount = TokenPool::<T>::get(currency_id);
		let v_currency_total_issuance = T::MultiCurrency::total_issuance(v_currency_id);

		if BalanceOf::<T>::zero().eq(&v_currency_total_issuance) {
			Ok(v_currency_amount)
		} else {
			Ok(multiply_by_rational_with_rounding(
				v_currency_amount.saturated_into::<u128>(),
				token_pool_amount.saturated_into::<u128>(),
				v_currency_total_issuance.saturated_into::<u128>(),
				Rounding::Down,
			)
			.ok_or(Error::<T>::CalculationOverflow)?
			.unique_saturated_into())
		}
	}

	fn get_minimums_redeem(v_currency_id: CurrencyIdOf<T>) -> BalanceOf<T> {
		MinimumRedeem::<T>::get(v_currency_id)
	}

	fn get_token_pool(currency_id: CurrencyId) -> BalanceOf<T> {
		TokenPool::<T>::get(currency_id)
	}

	fn get_moonbeam_parachain_id() -> u32 {
		T::MoonbeamChainId::get()
	}
}

impl<T: Config> VTokenSupplyProvider<CurrencyIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn get_vtoken_supply(vtoken: CurrencyIdOf<T>) -> Option<BalanceOf<T>> {
		if CurrencyId::is_vtoken(&vtoken) {
			Some(T::MultiCurrency::total_issuance(vtoken))
		} else {
			None
		}
	}

	fn get_token_supply(token: CurrencyIdOf<T>) -> Option<BalanceOf<T>> {
		if CurrencyId::is_token(&token) | CurrencyId::is_native(&token) {
			Some(TokenPool::<T>::get(token))
		} else {
			None
		}
	}
}

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
use crate::{
	agents::{
		PhalaCall, PhalaUtilityCall, StakePoolv2Call, VaultCall, WrappedBalancesCall, XtransferCall,
	},
	pallet::{Error, Event},
	primitives::{
		Ledger, PhalaLedger, QueryId, SubstrateLedgerUpdateEntry, SubstrateLedgerUpdateOperation,
	},
	traits::{QueryResponseManager, StakingAgent},
	AccountIdOf, BalanceOf, Config, CurrencyId, DelegatorLedgerXcmUpdateQueue, DelegatorLedgers,
	DelegatorsMultilocation2Index, Hash, LedgerUpdateEntry, MinimumsAndMaximums, Pallet, TimeUnit,
	Validators, ValidatorsByDelegatorUpdateEntry,
};
use bifrost_primitives::{TokenSymbol, VtokenMintingOperator, XcmOperationType};
use core::marker::PhantomData;
use frame_support::{ensure, traits::Get};
use frame_system::pallet_prelude::BlockNumberFor;
use parity_scale_codec::Encode;
use sp_core::U256;
use sp_runtime::{
	traits::{CheckedAdd, CheckedSub, Convert, UniqueSaturatedFrom, UniqueSaturatedInto, Zero},
	DispatchResult, SaturatedConversion,
};
use sp_std::prelude::*;
use xcm::{
	opaque::v3::{Junction::GeneralIndex, MultiLocation},
	v3::prelude::*,
};

/// StakingAgent implementation for Phala
pub struct PhalaAgent<T>(PhantomData<T>);

impl<T> PhalaAgent<T> {
	pub fn new() -> Self {
		PhalaAgent(PhantomData::<T>)
	}
}

impl<T: Config>
	StakingAgent<
		BalanceOf<T>,
		AccountIdOf<T>,
		LedgerUpdateEntry<BalanceOf<T>>,
		ValidatorsByDelegatorUpdateEntry,
		Error<T>,
	> for PhalaAgent<T>
{
	fn initialize_delegator(
		&self,
		currency_id: CurrencyId,
		_delegator_location_op: Option<Box<MultiLocation>>,
	) -> Result<MultiLocation, Error<T>> {
		let new_delegator_id = Pallet::<T>::inner_initialize_delegator(currency_id)?;

		// Generate multi-location by id.
		let delegator_multilocation = T::AccountConverter::convert((new_delegator_id, currency_id));
		ensure!(delegator_multilocation != MultiLocation::default(), Error::<T>::FailToConvert);

		// Add the new delegator into storage
		Pallet::<T>::inner_add_delegator(new_delegator_id, &delegator_multilocation, currency_id)
			.map_err(|_| Error::<T>::FailToAddDelegator)?;

		Ok(delegator_multilocation)
	}

	/// In Phala context, it corresponds to `contribute` function.
	fn bond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		share_price: &Option<MultiLocation>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it has already delegated a validator.
		let (pool_id, is_vault) =
			if let Some(Ledger::Phala(ledger)) = DelegatorLedgers::<T>::get(currency_id, *who) {
				let pool_id = ledger.bonded_pool_id.ok_or(Error::<T>::NotDelegateValidator)?;
				let is_vault = ledger.bonded_is_vault.ok_or(Error::<T>::NotDelegateValidator)?;
				(pool_id, is_vault)
			} else {
				Err(Error::<T>::DelegatorNotExist)?
			};

		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.delegator_bonded_minimum, Error::<T>::LowerThanMinimum);

		// Ensure the bond doesn't exceeds delegator_active_staking_maximum
		ensure!(
			amount <= mins_maxs.delegator_active_staking_maximum,
			Error::<T>::ExceedActiveMaximum
		);

		// Construct xcm message.
		let wrap_call = PhalaCall::PhalaWrappedBalances(WrappedBalancesCall::<T>::Wrap(amount));

		let calls = {
			if is_vault {
				let contribute_call =
					PhalaCall::PhalaVault(VaultCall::<T>::Contribute(pool_id, amount));
				let calls = vec![Box::new(wrap_call), Box::new(contribute_call)];
				let batched_calls = PhalaCall::Utility(Box::new(
					PhalaUtilityCall::<PhalaCall<T>>::BatchAll(Box::new(calls)),
				));
				batched_calls.encode()
			} else {
				let contribute_call = PhalaCall::PhalaStakePoolv2(
					StakePoolv2Call::<T>::Contribute(pool_id, amount, None),
				);
				let calls = vec![Box::new(wrap_call), Box::new(contribute_call)];
				let batched_calls = PhalaCall::Utility(Box::new(
					PhalaUtilityCall::<PhalaCall<T>>::BatchAll(Box::new(calls)),
				));
				batched_calls.encode()
			}
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Bond,
				calls,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Calculate how many shares we can get by the amount at current price
		let shares = if let Some(MultiLocation {
			parents: _,
			interior: X2(GeneralIndex(total_value), GeneralIndex(total_shares)),
		}) = share_price
		{
			Self::calculate_shares(total_value, total_shares, amount)
		} else {
			Err(Error::<T>::SharePriceNotValid)
		}?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			SubstrateLedgerUpdateOperation::Bond,
			BalanceOf::<T>::unique_saturated_from(shares),
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Bond extra amount to a delegator. In Phala context, it is the same as bond.
	fn bond_extra(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		share_price: &Option<MultiLocation>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		Self::bond(self, who, amount, share_price, currency_id, weight_and_fee)
	}

	/// Decrease bonding amount to a delegator. In Phala context, it corresponds to `withdraw`
	/// function. Noted that the param for `withdraw` is `shares` instead of `amount`. So we need
	/// to calculate the shares by the input `share_price` and `amount`.
	fn unbond(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		share_price: &Option<MultiLocation>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it has already delegated a validator.
		let (pool_id, active_shares, unlocking_shares, is_vault) =
			if let Some(Ledger::Phala(ledger)) = DelegatorLedgers::<T>::get(currency_id, *who) {
				let pool_id = ledger.bonded_pool_id.ok_or(Error::<T>::NotDelegateValidator)?;
				let active_shares = ledger.active_shares;
				let unlocking_shares = ledger.unlocking_shares;
				let is_vault = ledger.bonded_is_vault.ok_or(Error::<T>::NotDelegateValidator)?;
				Ok((pool_id, active_shares, unlocking_shares, is_vault))
			} else {
				Err(Error::<T>::DelegatorNotExist)
			}?;

		// Ensure this delegator is not in the process of unbonding.
		ensure!(unlocking_shares.is_zero(), Error::<T>::AlreadyRequested);

		// Ensure the amount is not zero
		ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

		// Calculate how many shares we can get by the amount at current price
		let shares = if let Some(MultiLocation {
			parents: _,
			interior: X2(GeneralIndex(total_value), GeneralIndex(total_shares)),
		}) = share_price
		{
			Self::calculate_shares(total_value, total_shares, amount)
		} else {
			Err(Error::<T>::SharePriceNotValid)
		}?;

		// Check if shares exceeds the minimum requirement > 1000(existential value for shares).
		ensure!(
			shares > BalanceOf::<T>::unique_saturated_from(1000u32),
			Error::<T>::LowerThanMinimum
		);

		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.unbond_minimum, Error::<T>::LowerThanMinimum);

		// Check if the remaining active shares is enough for withdrawing.
		active_shares.checked_sub(&shares).ok_or(Error::<T>::NotEnoughToUnbond)?;

		let call = {
			if is_vault {
				// Construct xcm message.
				let withdraw_call =
					PhalaCall::PhalaVault(VaultCall::<T>::Withdraw(pool_id, shares));
				withdraw_call.encode()
			} else {
				let withdraw_call = PhalaCall::PhalaStakePoolv2(StakePoolv2Call::<T>::Withdraw(
					pool_id, shares, None,
				));
				withdraw_call.encode()
			}
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Unbond,
				call,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		Self::insert_delegator_ledger_update_entry(
			who,
			SubstrateLedgerUpdateOperation::Unlock,
			shares,
			query_id,
			timeout,
			currency_id,
		)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// Unbonding all amount of a delegator. Differentiate from regular unbonding.
	fn unbond_all(
		&self,
		_who: &MultiLocation,
		_currency_id: CurrencyId,
		_weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// In Phala context, it is the same as bond.
	fn rebond(
		&self,
		who: &MultiLocation,
		amount: Option<BalanceOf<T>>,
		share_price: &Option<MultiLocation>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		let amount = amount.ok_or(Error::<T>::InvalidAmount)?;
		ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

		Self::bond(self, who, amount, share_price, currency_id, weight_and_fee)
	}

	/// Delegate to some validators. In Phala context, the passed in Multilocation
	/// should contain validator bonded pool id and NFT collection id. Only deal
	/// with the first item in the vec.
	fn delegate(
		&self,
		who: &MultiLocation,
		targets: &Vec<MultiLocation>,
		currency_id: CurrencyId,
		_weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it is in the delegator set.
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(currency_id, *who),
			Error::<T>::DelegatorNotExist
		);

		// Check if targets vec is empty.
		let vec_len = targets.len() as u32;
		ensure!(vec_len > 0, Error::<T>::VectorEmpty);

		// Get the first item of the vec
		let candidate = &targets[0];

		// if parents is 0, it is vault. Otherwise, it is stake pool.
		if let &MultiLocation {
			parents: vault_or_stake_pool,
			interior: X2(GeneralIndex(pool_id), GeneralIndex(collection_id)),
		} = candidate
		{
			// Ensure the candidate is in the validator whitelist.
			let validators_set =
				Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;

			ensure!(validators_set.contains(candidate), Error::<T>::ValidatorNotExist);

			// if the delegator is new, create a ledger for it
			if !DelegatorLedgers::<T>::contains_key(currency_id, &who.clone()) {
				// Create a new delegator ledger\
				let ledger = PhalaLedger::<BalanceOf<T>> {
					account: *who,
					active_shares: Zero::zero(),
					unlocking_shares: Zero::zero(),
					unlocking_time_unit: None,
					bonded_pool_id: None,
					bonded_pool_collection_id: None,
					bonded_is_vault: None,
				};
				let phala_ledger = Ledger::<BalanceOf<T>>::Phala(ledger);

				DelegatorLedgers::<T>::insert(currency_id, *who, phala_ledger);
			}

			DelegatorLedgers::<T>::mutate_exists(
				currency_id,
				*who,
				|old_ledger_opt| -> Result<(), Error<T>> {
					if let Some(Ledger::Phala(ref mut ledger)) = old_ledger_opt {
						ensure!(ledger.active_shares == Zero::zero(), Error::<T>::AlreadyBonded);
						ensure!(ledger.unlocking_shares == Zero::zero(), Error::<T>::AlreadyBonded);

						// delegate the validator
						ledger.bonded_pool_id = Some(u64::unique_saturated_from(pool_id));
						ledger.bonded_pool_collection_id =
							Some(u32::unique_saturated_from(collection_id));

						let is_vault = vault_or_stake_pool == 0;
						ledger.bonded_is_vault = Some(is_vault);
					} else {
						Err(Error::<T>::Unexpected)?;
					}
					Ok(())
				},
			)?;
		} else {
			Err(Error::<T>::ValidatorError)?;
		}

		// Emit event
		Pallet::<T>::deposit_event(Event::Delegated {
			currency_id,
			delegator_id: *who,
			targets: Some(targets.clone()),
			query_id: Zero::zero(),
			query_id_hash: Hash::<T>::default(),
		});

		Ok(Zero::zero())
	}

	/// Remove delegation relationship with some validators. Just change the storage, no need to
	/// call Phala runtime.
	fn undelegate(
		&self,
		who: &MultiLocation,
		_targets: &Vec<MultiLocation>,
		currency_id: CurrencyId,
		_weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it has already delegated a validator.
		DelegatorLedgers::<T>::mutate(
			currency_id,
			*who,
			|old_ledger_opt| -> Result<(), Error<T>> {
				if let Some(Ledger::Phala(ref mut ledger)) = old_ledger_opt {
					// Ensure both active_shares and unlocking_shares are zero.
					ensure!(ledger.active_shares == Zero::zero(), Error::<T>::ValidatorStillInUse);
					ensure!(
						ledger.unlocking_shares == Zero::zero(),
						Error::<T>::ValidatorStillInUse
					);

					// undelegate the validator
					ledger.bonded_pool_id = None;
					ledger.bonded_pool_collection_id = None;
					ledger.bonded_is_vault = None;

					// Emit event
					Pallet::<T>::deposit_event(Event::Undelegated {
						currency_id,
						delegator_id: *who,
						targets: vec![],
						query_id: Zero::zero(),
						query_id_hash: Hash::<T>::default(),
					});

					Ok(())
				} else {
					Err(Error::<T>::DelegatorNotExist)
				}
			},
		)?;

		Ok(Zero::zero())
	}

	/// Re-delegate existing delegation to a new validator set. In Phala context, it's the same as
	/// delegate.
	fn redelegate(
		&self,
		who: &MultiLocation,
		targets: &Option<Vec<MultiLocation>>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		let targets = targets.as_ref().ok_or(Error::<T>::ValidatorNotProvided)?;
		Self::delegate(self, who, &targets, currency_id, weight_and_fee)
	}

	/// Corresponds to the `check_and_maybe_force_withdraw` funtion of PhalaVault pallet.
	/// Usually we don't need to call it, someone else will pay the rewards. But in case,
	/// we can call it to force withdraw the rewards.
	fn payout(
		&self,
		who: &MultiLocation,
		_validator: &MultiLocation,
		_when: &Option<TimeUnit>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if it has already delegated a validator.
		let (pool_id, is_vault) =
			if let Some(Ledger::Phala(ledger)) = DelegatorLedgers::<T>::get(currency_id, *who) {
				let pool_id = ledger.bonded_pool_id.ok_or(Error::<T>::NotDelegateValidator)?;
				let is_vault = ledger.bonded_is_vault.ok_or(Error::<T>::NotDelegateValidator)?;
				(pool_id, is_vault)
			} else {
				Err(Error::<T>::DelegatorNotExist)?
			};

		// Construct xcm message.
		let call = if is_vault {
			let check_and_maybe_force_withdraw_call =
				PhalaCall::PhalaVault(VaultCall::<T>::CheckAndMaybeForceWithdraw(pool_id));
			check_and_maybe_force_withdraw_call.encode()
		} else {
			let check_and_maybe_force_withdraw_call = PhalaCall::PhalaStakePoolv2(
				StakePoolv2Call::<T>::CheckAndMaybeForceWithdraw(pool_id),
			);
			check_and_maybe_force_withdraw_call.encode()
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, _timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::Payout,
				call,
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	/// This is for revising ledger unlocking shares. Since Phala might return the withdrawal amount
	/// by several times, we need to update the ledger to reflect the changes.
	fn liquidize(
		&self,
		who: &MultiLocation,
		_when: &Option<TimeUnit>,
		_validator: &Option<MultiLocation>,
		currency_id: CurrencyId,
		amount: Option<BalanceOf<T>>,
		_weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if amount is provided. This amount will replace the unlocking_shares in ledger.
		let updated_amount = amount.ok_or(Error::<T>::AmountNotProvided)?;

		// update delegator ledger
		DelegatorLedgers::<T>::mutate_exists(
			currency_id,
			who,
			|old_ledger| -> Result<(), Error<T>> {
				if let Some(Ledger::Phala(ref mut old_phala_ledger)) = old_ledger {
					ensure!(
						old_phala_ledger.bonded_pool_id.is_some(),
						Error::<T>::DelegatorNotBonded
					);
					ensure!(
						old_phala_ledger.unlocking_shares > updated_amount,
						Error::<T>::InvalidAmount
					);

					// Update unlocking_shares to amount.
					old_phala_ledger.unlocking_shares = updated_amount;

					if updated_amount.is_zero() {
						old_phala_ledger.unlocking_time_unit = None;
					}

					Ok(())
				} else {
					Err(Error::<T>::Unexpected)?
				}
			},
		)?;

		Ok(Zero::zero())
	}

	/// Not supported in Phala.
	fn chill(
		&self,
		_who: &MultiLocation,
		_currency_id: CurrencyId,
		_weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	/// Make token transferred back to Bifrost chain account.
	fn transfer_back(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<(), Error<T>> {
		// Ensure amount is greater than zero.
		ensure!(!amount.is_zero(), Error::<T>::AmountZero);

		let dest_account_32 = Pallet::<T>::multilocation_to_account_32(to)?;
		let dest = Pallet::<T>::account_32_to_parachain_location(
			dest_account_32,
			T::ParachainId::get().into(),
		)?;

		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		let dest_location =
			xcm::v3::Location::try_from(dest_location).map_err(|_| Error::<T>::FailToConvert)?;
		// Prepare parameter assets.
		let asset = MultiAsset {
			fun: Fungible(amount.unique_saturated_into()),
			id: Concrete(dest_location),
		};

		// Construct xcm message.
		let call: PhalaCall<T> =
			PhalaCall::Xtransfer(XtransferCall::Transfer(Box::new(asset), Box::new(dest), None));

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let fee = Pallet::<T>::construct_xcm_and_send_as_subaccount_without_query_id(
			XcmOperationType::TransferBack,
			call.encode(),
			from,
			currency_id,
			weight_and_fee,
		)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		Ok(())
	}

	/// Make token from Bifrost chain account to the staking chain account.
	/// Receiving account must be one of the currency_id delegators.
	fn transfer_to(
		&self,
		from: &MultiLocation,
		to: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		// Make sure receiving account is one of the currency_id delegators.
		ensure!(
			DelegatorsMultilocation2Index::<T>::contains_key(currency_id, to),
			Error::<T>::DelegatorNotExist
		);

		// Make sure from account is the entrance account of vtoken-minting module.
		let from_account_id = Pallet::<T>::multilocation_to_account(from)?;
		let (entrance_account, _) = T::VtokenMinting::get_entrance_and_exit_accounts();
		ensure!(from_account_id == entrance_account, Error::<T>::InvalidAccount);

		// transfer supplementary fee from treasury to the "from" account. Return the added up
		// amount
		let amount = Pallet::<T>::get_transfer_to_added_amount_and_supplement(
			from_account_id,
			amount,
			currency_id,
		)?;

		Pallet::<T>::do_transfer_to(from, to, amount, currency_id)?;

		Ok(())
	}

	// Convert token to another token.
	// if we convert from currency_id to some other currency, then if_from_currency should be true.
	// if we convert from some other currency to currency_id, then if_from_currency should be false.
	fn convert_asset(
		&self,
		who: &MultiLocation,
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
		if_from_currency: bool,
		weight_and_fee: Option<(Weight, BalanceOf<T>)>,
	) -> Result<QueryId, Error<T>> {
		// Check if delegator exists.
		ensure!(
			DelegatorLedgers::<T>::contains_key(currency_id, *who),
			Error::<T>::DelegatorNotExist
		);

		// Ensure amount is greater than zero.
		ensure!(!amount.is_zero(), Error::<T>::AmountZero);

		// Construct xcm message.
		let call = if if_from_currency {
			PhalaCall::PhalaWrappedBalances(WrappedBalancesCall::<T>::Wrap(amount))
		} else {
			PhalaCall::PhalaWrappedBalances(WrappedBalancesCall::<T>::Unwrap(amount))
		};

		// Wrap the xcm message as it is sent from a subaccount of the parachain account, and
		// send it out.
		let (query_id, _timeout, fee, xcm_message) =
			Pallet::<T>::construct_xcm_as_subaccount_with_query_id(
				XcmOperationType::ConvertAsset,
				call.encode(),
				who,
				currency_id,
				weight_and_fee,
			)?;

		// withdraw this xcm fee from treasury. If treasury doesn't have this money, stop the
		// process.
		Pallet::<T>::burn_fee_from_source_account(fee, currency_id)?;

		// Send out the xcm message.
		let dest_location = Pallet::<T>::convert_currency_to_dest_location(currency_id)?;
		xcm::v4::send_xcm::<T::XcmRouter>(dest_location, xcm_message)
			.map_err(|_e| Error::<T>::XcmFailure)?;

		Ok(query_id)
	}

	fn tune_vtoken_exchange_rate(
		&self,
		who: &Option<MultiLocation>,
		token_amount: BalanceOf<T>,
		_vtoken_amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		let who = who.as_ref().ok_or(Error::<T>::DelegatorNotExist)?;

		// Ensure delegator has bonded to a validator.
		if let Some(Ledger::Phala(ledger)) = DelegatorLedgers::<T>::get(currency_id, *who) {
			ensure!(ledger.bonded_pool_id.is_some(), Error::<T>::DelegatorNotBonded);
		} else {
			Err(Error::<T>::DelegatorNotExist)?;
		}

		Pallet::<T>::tune_vtoken_exchange_rate_without_update_ledger(
			who,
			token_amount,
			currency_id,
		)?;

		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(&self, who: &MultiLocation, currency_id: CurrencyId) -> DispatchResult {
		// Get the delegator ledger
		let ledger =
			DelegatorLedgers::<T>::get(currency_id, who).ok_or(Error::<T>::DelegatorNotBonded)?;

		if let Ledger::Phala(phala_ledger) = ledger {
			// Check if ledger bonding and unlocking amount is zero. If not, return error.
			ensure!(phala_ledger.active_shares.is_zero(), Error::<T>::AmountNotZero);
			ensure!(phala_ledger.unlocking_shares.is_zero(), Error::<T>::AmountNotZero);
		} else {
			Err(Error::<T>::Unexpected)?;
		}

		Pallet::<T>::inner_remove_delegator(who, currency_id)
	}

	/// Charge hosting fee.
	fn charge_hosting_fee(
		&self,
		amount: BalanceOf<T>,
		_from: &MultiLocation,
		to: &MultiLocation,
		currency_id: CurrencyId,
	) -> DispatchResult {
		let vtoken = CurrencyId::VToken(TokenSymbol::PHA);

		let charge_amount =
			Pallet::<T>::inner_calculate_vtoken_hosting_fee(amount, vtoken, currency_id)?;

		Pallet::<T>::inner_charge_hosting_fee(charge_amount, to, vtoken)
	}

	fn check_delegator_ledger_query_response(
		&self,
		query_id: QueryId,
		entry: LedgerUpdateEntry<BalanceOf<T>>,
		manual_mode: bool,
		currency_id: CurrencyId,
	) -> Result<bool, Error<T>> {
		// If this is manual mode, it is always updatable.
		let should_update = if manual_mode {
			true
		} else {
			T::SubstrateResponseManager::get_query_response_record(query_id)
		};

		// Update corresponding storages.
		if should_update {
			Self::update_ledger_query_response_storage(query_id, entry.clone(), currency_id)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorLedgerQueryResponseConfirmed {
				query_id,
				entry,
			});
		}

		Ok(should_update)
	}

	fn check_validators_by_delegator_query_response(
		&self,
		_query_id: QueryId,
		_entry: ValidatorsByDelegatorUpdateEntry,
		_manual_mode: bool,
	) -> Result<bool, Error<T>> {
		Err(Error::<T>::Unsupported)
	}

	fn fail_delegator_ledger_query_response(&self, query_id: QueryId) -> Result<(), Error<T>> {
		// delete pallet_xcm query
		T::SubstrateResponseManager::remove_query_record(query_id);

		// delete update entry
		DelegatorLedgerXcmUpdateQueue::<T>::remove(query_id);

		// Deposit event.
		Pallet::<T>::deposit_event(Event::DelegatorLedgerQueryResponseFailed { query_id });

		Ok(())
	}

	fn fail_validators_by_delegator_query_response(
		&self,
		_query_id: QueryId,
	) -> Result<(), Error<T>> {
		Err(Error::<T>::Unsupported)
	}
}

/// Internal functions.
impl<T: Config> PhalaAgent<T> {
	fn insert_delegator_ledger_update_entry(
		who: &MultiLocation,
		update_operation: SubstrateLedgerUpdateOperation,
		shares: BalanceOf<T>,
		query_id: QueryId,
		timeout: BlockNumberFor<T>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		use crate::primitives::SubstrateLedgerUpdateOperation::Unlock;
		// Insert a delegator ledger update record into DelegatorLedgerXcmUpdateQueue<T>.
		let unlock_time = match &update_operation {
			Unlock => Pallet::<T>::get_unlocking_time_unit_from_current(false, currency_id)?,
			_ => None,
		};

		let entry = LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
			currency_id,
			delegator_id: *who,
			update_operation,
			amount: shares,
			unlock_time,
		});
		DelegatorLedgerXcmUpdateQueue::<T>::insert(query_id, (entry, timeout));

		Ok(())
	}

	fn update_ledger_query_response_storage(
		query_id: QueryId,
		query_entry: LedgerUpdateEntry<BalanceOf<T>>,
		currency_id: CurrencyId,
	) -> Result<(), Error<T>> {
		use crate::primitives::SubstrateLedgerUpdateOperation::{Bond, Unlock};
		// update DelegatorLedgers<T> storage
		if let LedgerUpdateEntry::Substrate(SubstrateLedgerUpdateEntry {
			currency_id: _,
			delegator_id,
			update_operation,
			amount,
			unlock_time,
		}) = query_entry
		{
			DelegatorLedgers::<T>::mutate(
				currency_id,
				delegator_id,
				|old_ledger| -> Result<(), Error<T>> {
					if let Some(Ledger::Phala(ref mut old_pha_ledger)) = old_ledger {
						match update_operation {
							Bond => {
								// If this is a bonding operation, increase active_shares.
								old_pha_ledger.active_shares = old_pha_ledger
									.active_shares
									.checked_add(&amount)
									.ok_or(Error::<T>::OverFlow)?;
							},
							// If this is a bonding operation, increase unlocking_shares.
							Unlock => {
								// we only allow one unlocking operation at a time.
								ensure!(
									old_pha_ledger.unlocking_shares.is_zero(),
									Error::<T>::AlreadyRequested
								);

								old_pha_ledger.active_shares = old_pha_ledger
									.active_shares
									.checked_sub(&amount)
									.ok_or(Error::<T>::UnderFlow)?;

								old_pha_ledger.unlocking_shares = amount;

								let unlock_time_unit =
									unlock_time.ok_or(Error::<T>::TimeUnitNotExist)?;

								old_pha_ledger.unlocking_time_unit = Some(unlock_time_unit);
							},
							_ => return Err(Error::<T>::Unexpected),
						}
						Ok(())
					} else {
						Err(Error::<T>::Unexpected)
					}
				},
			)?;
		} else {
			Err(Error::<T>::Unexpected)?;
		}

		// Delete the DelegatorLedgerXcmUpdateQueue<T> query
		DelegatorLedgerXcmUpdateQueue::<T>::remove(query_id);

		// Delete the query in pallet_xcm.
		T::SubstrateResponseManager::remove_query_record(query_id);

		Ok(())
	}

	fn calculate_shares(
		total_value: &u128,
		total_shares: &u128,
		amount: BalanceOf<T>,
	) -> Result<BalanceOf<T>, Error<T>> {
		ensure!(total_shares > &0u128, Error::<T>::DividedByZero);
		let shares: u128 = U256::from((*total_shares).saturated_into::<u128>())
			.saturating_mul(amount.saturated_into::<u128>().into())
			.checked_div((*total_value).saturated_into::<u128>().into())
			.map(|x| u128::try_from(x))
			.ok_or(Error::<T>::OverFlow)?
			.map_err(|_| Error::<T>::OverFlow)?;

		Ok(BalanceOf::<T>::unique_saturated_from(shares))
	}
}

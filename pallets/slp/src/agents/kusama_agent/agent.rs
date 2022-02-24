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

use core::marker::PhantomData;

use codec::{Decode, Encode};
pub use cumulus_primitives_core::ParaId;
use frame_support::{ensure, traits::Get, weights::Weight};
use node_primitives::CurrencyId;
use sp_runtime::{
	traits::{Convert, UniqueSaturatedInto},
	DispatchResult,
};
use sp_std::prelude::*;
use xcm::{
	latest::prelude::*,
	opaque::latest::{Junction::AccountId32, Junctions::X1, MultiLocation},
};

use crate::{
	agents::{KusamaCall, StakingCall, UtilityCall},
	pallet::Error,
	primitives::{SubstrateLedger, XcmOperation},
	traits::{DelegatorManager, StakingAgent, XcmBuilder},
	BalanceOf, Config, DelegatorLedgers, DelegatorNextIndex, DelegatorsIndex2Multilocation,
	DelegatorsMultilocation2Index, Event, MinimumsAndMaximums, Pallet, ValidatorManager,
	XcmDestWeightAndFee,
};

/// StakingAgent implementation for Kusama
pub struct KusamaAgent<T, AccountConverter, ParachainId, XcmSender>(
	PhantomData<(T, AccountConverter, ParachainId, XcmSender)>,
);

impl<T, AccountConverter, ParachainId, XcmSender> StakingAgent<MultiLocation, MultiLocation>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
{
	type CurrencyId = CurrencyId;
	type Balance = BalanceOf<T>;

	fn initialize_delegator(currency_id: Self::CurrencyId) -> Option<MultiLocation> {
		let new_delegator_id = DelegatorNextIndex::<T>::get(currency_id);
		let rs = DelegatorNextIndex::<T>::mutate(currency_id, |id| -> DispatchResult {
			let option_new_id = id.checked_add(1).ok_or(Error::<T>::OverFlow)?;
			*id = option_new_id;
			Ok(())
		});

		if let Ok(_) = rs {
			// Generate multi-location by id.
			let delegator_multilocation = AccountConverter::convert(new_delegator_id);

			// Add the new delegator into storage
			let _ = Self::add_delegator(currency_id, new_delegator_id, &delegator_multilocation);

			Pallet::<T>::deposit_event(Event::DelegatorInitialized(
				currency_id,
				delegator_multilocation.clone(),
			));
			Some(delegator_multilocation)
		} else {
			None
		}
	}

	/// First time bonding some amount to a delegator.
	fn bond(
		currency_id: Self::CurrencyId,
		who: MultiLocation,
		amount: Self::Balance,
	) -> DispatchResult {
		// Check if it is bonded already.
		let ledger = DelegatorLedgers::<T>::get(currency_id, who.clone());
		if let Some(_) = ledger {
			Err(Error::<T>::AlreadyBonded)?
		} else {
			// Check if the amount exceeds the minimum requirement.
			let mins_maxs =
				MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
			ensure!(amount >= mins_maxs.delegator_bonded_minimum, Error::<T>::LowerThanMinimum);

			// Get the delegator sub-account index.
			let sub_account_index =
				DelegatorsMultilocation2Index::<T>::get(currency_id, who.clone())
					.ok_or(Error::<T>::DelegatorNotExist)?;

			// Get the delegator account id in Kusama network
			let delegator_account_32 = match who.clone() {
				MultiLocation {
					parents: 1,
					interior: X1(AccountId32 { network: NetworkId, id: account_id }),
				} => account_id,
				_ => Err(Error::<T>::DelegatorNotExist)?,
			};
			let delegator_account = T::AccountId::decode(&mut &delegator_account_32[..])
				.expect("32 bytes can always construct an AccountId32");

			// Construct xcm message and send it out.
			let call = KusamaCall::Staking(StakingCall::Bond(
				delegator_account.clone(),
				amount,
				delegator_account,
			));
			let call_as_subaccount =
				KusamaCall::Utility(Box::new(UtilityCall::AsDerivative(sub_account_index, call)));

			let (weight, fee) = XcmDestWeightAndFee::<T>::get(currency_id, XcmOperation::Bond);

			let xcm_message = Self::construct_xcm_message(call_as_subaccount, fee, weight);
			XcmSender::send_xcm(Parent, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorBonded(currency_id, who, amount));

			Ok(())
		}
	}

	/// Bond extra amount to a delegator.
	fn bond_extra(
		currency_id: Self::CurrencyId,
		who: MultiLocation,
		amount: Self::Balance,
	) -> DispatchResult {
		// Check if it is bonded already.
		let ledger = DelegatorLedgers::<T>::get(currency_id, who.clone())
			.ok_or(Error::<T>::DelegatorNotBonded)?;

		// Check if the amount exceeds the minimum requirement.
		let mins_maxs = MinimumsAndMaximums::<T>::get(currency_id).ok_or(Error::<T>::NotExist)?;
		ensure!(amount >= mins_maxs.bond_extra_minimum, Error::<T>::LowerThanMinimum);

		// Get the delegator sub-account index.
		let sub_account_index = DelegatorsMultilocation2Index::<T>::get(currency_id, who.clone())
			.ok_or(Error::<T>::DelegatorNotExist)?;

		// Construct xcm message and send it out.
		let call = KusamaCall::Staking(StakingCall::BondExtra(amount));
		let call_as_subaccount =
			KusamaCall::Utility(Box::new(UtilityCall::AsDerivative(sub_account_index, call)));

		let (weight, fee) = XcmDestWeightAndFee::<T>::get(currency_id, XcmOperation::BondExtra);

		let xcm_message = Self::construct_xcm_message(call_as_subaccount, fee, weight);
		XcmSender::send_xcm(Parent, xcm_message).map_err(|_e| Error::<T>::XcmFailure)?;

		// Deposit event.
		Pallet::<T>::deposit_event(Event::DelegatorBondExtra(currency_id, who, amount));

		Ok(())
	}

	/// Decrease bonding amount to a delegator.
	fn unbond(
		currency_id: Self::CurrencyId,
		who: MultiLocation,
		amount: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Cancel some unbonding amount.
	fn rebond(
		currency_id: Self::CurrencyId,
		who: MultiLocation,
		amount: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Delegate to some validators.
	fn delegate(
		currency_id: Self::CurrencyId,
		who: MultiLocation,
		targets: Vec<MultiLocation>,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Remove delegation relationship with some validators.
	fn undelegate(
		currency_id: Self::CurrencyId,
		who: MultiLocation,
		targets: Vec<MultiLocation>,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Re-delegate existing delegation to a new validator set.
	fn redelegate(
		currency_id: Self::CurrencyId,
		who: MultiLocation,
		targets: Vec<MultiLocation>,
	) -> DispatchResult {
		unimplemented!()
	}

	/// Initiate payout for a certain delegator.
	fn payout(currency_id: Self::CurrencyId, who: MultiLocation) -> Self::Balance {
		unimplemented!()
	}

	/// Withdraw the due payout into free balance.
	fn liquidize(currency_id: Self::CurrencyId, who: MultiLocation) -> Self::Balance {
		unimplemented!()
	}

	/// Increase/decrease the token amount for the storage "token_pool" in the VtokenMining
	/// module. If the increase variable is true, then we increase token_pool by token_amount.
	/// If it is false, then we decrease token_pool by token_amount.
	fn increase_token_pool(
		currency_id: Self::CurrencyId,
		token_amount: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}
	///
	fn decrease_token_pool(
		currency_id: Self::CurrencyId,
		token_amount: Self::Balance,
	) -> DispatchResult {
		unimplemented!()
	}
}

/// DelegatorManager implementation for Kusama
impl<T, AccountConverter, ParachainId, XcmSender>
	DelegatorManager<MultiLocation, SubstrateLedger<MultiLocation, BalanceOf<T>>>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
{
	type CurrencyId = CurrencyId;

	/// Add a new serving delegator for a particular currency.
	fn add_delegator(
		currency_id: Self::CurrencyId,
		index: u16,
		who: &MultiLocation,
	) -> DispatchResult {
		DelegatorsIndex2Multilocation::<T>::insert(currency_id, index, who);
		DelegatorsMultilocation2Index::<T>::insert(currency_id, who, index);
		Ok(())
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_delegator(currency_id: Self::CurrencyId, who: &MultiLocation) -> DispatchResult {
		unimplemented!()
	}
}

/// ValidatorManager implementation for Kusama
impl<T, AccountConverter, ParachainId, XcmSender> ValidatorManager<MultiLocation>
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
{
	type CurrencyId = CurrencyId;
	/// Add a new serving delegator for a particular currency.
	fn add_validator(currency_id: Self::CurrencyId, who: &MultiLocation) -> DispatchResult {
		unimplemented!()
	}

	/// Remove an existing serving delegator for a particular currency.
	fn remove_validator(currency_id: Self::CurrencyId, who: &MultiLocation) -> DispatchResult {
		unimplemented!()
	}
}

/// Trait XcmBuilder implementation for Kusama
impl<T, AccountConverter, ParachainId, XcmSender> XcmBuilder
	for KusamaAgent<T, AccountConverter, ParachainId, XcmSender>
where
	T: Config,
	AccountConverter: Convert<u16, MultiLocation>,
	ParachainId: Get<ParaId>,
	XcmSender: SendXcm,
{
	type Balance = BalanceOf<T>;
	type ChainCallType = KusamaCall<T>;

	fn construct_xcm_message(
		call: Self::ChainCallType,
		extra_fee: Self::Balance,
		weight: Weight,
	) -> Xcm<()> {
		let asset = MultiAsset {
			id: Concrete(MultiLocation::here()),
			fun: Fungibility::Fungible(extra_fee.unique_saturated_into()),
		};
		Xcm(vec![
			WithdrawAsset(asset.clone().into()),
			BuyExecution { fees: asset, weight_limit: Unlimited },
			Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: weight,
				call: call.encode().into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: All.into(),
				max_assets: u32::max_value(),
				beneficiary: MultiLocation {
					parents: 0,
					interior: X1(Parachain(ParachainId::get().into())),
				},
			},
		])
	}
}

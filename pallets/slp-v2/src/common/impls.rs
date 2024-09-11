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
	common::types::{
		Delegator, DelegatorIndex, StakingProtocol, AS_DERIVATIVE_CALL_INDEX,
		LIMITED_RESERVE_TRANSFER_ASSETS_CALL_INDEX,
	},
	Config, ConfigurationByStakingProtocol, DelegatorByStakingProtocolAndDelegatorIndex,
	DelegatorIndexByStakingProtocolAndDelegator, Error, Event, LedgerByStakingProtocolAndDelegator,
	NextDelegatorIndexByStakingProtocol, Pallet, ValidatorsByStakingProtocolAndDelegator,
};
use bifrost_primitives::{Balance, CurrencyId, VtokenMintingOperator};
use frame_support::{
	dispatch::{DispatchResultWithPostInfo, GetDispatchInfo, RawOrigin},
	ensure,
	traits::{EnsureOrigin, Get},
};
use frame_system::pallet_prelude::OriginFor;
use orml_traits::{MultiCurrency, XcmTransfer};
use parity_scale_codec::{Decode, Encode};
use sp_core::blake2_256;
use sp_runtime::{
	helpers_128bit::multiply_by_rational_with_rounding, traits::TrailingZeroInput, DispatchError,
	Rounding, Saturating,
};
use sp_std::{vec, vec::Vec};
use xcm::{
	latest::{OriginKind, QueryId, QueryResponseInfo, WeightLimit, WildAsset},
	prelude::{AccountId32, Fungible, Here, ReportTransactStatus},
	v4::{opaque::Xcm, Asset, AssetFilter, AssetId, Assets, Location, SendXcm},
	DoubleEncoded, VersionedAssets, VersionedLocation,
};

impl<T: Config> Pallet<T> {
	pub fn do_add_delegator(
		staking_protocol: StakingProtocol,
		delegator: Option<Delegator<T::AccountId>>,
	) -> DispatchResultWithPostInfo {
		let mut delegator_index = 0;
		NextDelegatorIndexByStakingProtocol::<T>::mutate(
			staking_protocol,
			|index| -> DispatchResultWithPostInfo {
				delegator_index = *index;
				*index = index.checked_add(1).ok_or(Error::<T>::DelegatorIndexOverflow)?;
				let delegator =
					delegator.unwrap_or(staking_protocol.get_delegator::<T>(delegator_index)?);
				ensure!(
					!DelegatorByStakingProtocolAndDelegatorIndex::<T>::contains_key(
						staking_protocol,
						delegator_index
					),
					Error::<T>::DelegatorAlreadyExists
				);
				ensure!(
					!DelegatorIndexByStakingProtocolAndDelegator::<T>::contains_key(
						staking_protocol,
						delegator.clone()
					),
					Error::<T>::DelegatorIndexAlreadyExists
				);
				DelegatorByStakingProtocolAndDelegatorIndex::<T>::insert(
					staking_protocol,
					delegator_index,
					delegator.clone(),
				);
				DelegatorIndexByStakingProtocolAndDelegator::<T>::insert(
					staking_protocol,
					delegator.clone(),
					delegator_index,
				);
				LedgerByStakingProtocolAndDelegator::<T>::insert(
					staking_protocol,
					delegator.clone(),
					staking_protocol.get_default_ledger(),
				);
				Self::deposit_event(Event::AddDelegator {
					staking_protocol,
					delegator_index,
					delegator,
				});
				Ok(().into())
			},
		)
	}

	pub fn do_remove_delegator(
		staking_protocol: StakingProtocol,
		delegator: Delegator<T::AccountId>,
	) -> DispatchResultWithPostInfo {
		let delegator_index =
			DelegatorIndexByStakingProtocolAndDelegator::<T>::take(&staking_protocol, &delegator)
				.ok_or(Error::<T>::DelegatorIndexNotFound)?;
		DelegatorByStakingProtocolAndDelegatorIndex::<T>::remove(
			&staking_protocol,
			delegator_index,
		);
		ValidatorsByStakingProtocolAndDelegator::<T>::remove(&staking_protocol, &delegator);
		LedgerByStakingProtocolAndDelegator::<T>::remove(&staking_protocol, &delegator);
		Self::deposit_event(Event::RemoveDelegator {
			staking_protocol,
			delegator_index,
			delegator,
		});
		Ok(().into())
	}

	pub fn do_transfer_to(
		staking_protocol: StakingProtocol,
		delegator: Delegator<T::AccountId>,
	) -> DispatchResultWithPostInfo {
		Self::ensure_delegator_exist(&staking_protocol, &delegator)?;
		let currency_id = staking_protocol.info().currency_id;
		let dest_beneficiary_location = staking_protocol
			.get_dest_beneficiary_location::<T>(delegator.clone())
			.ok_or(Error::<T>::UnsupportedStakingProtocol)?;
		let (entrance_account, _) = T::VtokenMinting::get_entrance_and_exit_accounts();
		let entrance_account_free_balance =
			T::MultiCurrency::free_balance(currency_id, &entrance_account);
		T::XcmTransfer::transfer(
			entrance_account.clone(),
			currency_id,
			entrance_account_free_balance,
			dest_beneficiary_location,
			WeightLimit::Unlimited,
		)
		.map_err(|_| Error::<T>::DerivativeAccountIdFailed)?;
		Self::deposit_event(Event::TransferTo {
			staking_protocol,
			from: entrance_account,
			to: delegator,
			amount: entrance_account_free_balance,
		});
		Ok(().into())
	}

	pub fn do_transfer_back(
		staking_protocol: StakingProtocol,
		delegator: Delegator<T::AccountId>,
		amount: Balance,
	) -> DispatchResultWithPostInfo {
		let delegator_index = Self::ensure_delegator_exist(&staking_protocol, &delegator)?;
		let (entrance_account, _) = T::VtokenMinting::get_entrance_and_exit_accounts();
		let transfer_back_call_data =
			Self::wrap_polkadot_xcm_limited_reserve_transfer_assets_call_data(
				&staking_protocol,
				amount,
				entrance_account.clone(),
			)?;
		let utility_as_derivative_call_data = Self::wrap_utility_as_derivative_call_data(
			&staking_protocol,
			delegator_index,
			transfer_back_call_data,
		);
		let xcm_message =
			Self::wrap_xcm_message(&staking_protocol, utility_as_derivative_call_data)?;
		Self::send_xcm_message(staking_protocol, xcm_message)?;
		Self::deposit_event(Event::TransferBack {
			staking_protocol,
			from: delegator,
			to: entrance_account,
			amount,
		});
		Ok(().into())
	}

	/// Implemented by Utility pallet to get derived account id
	pub fn derivative_account_id(
		account_id: T::AccountId,
		delegator_index: DelegatorIndex,
	) -> Result<T::AccountId, Error<T>> {
		let entropy = (b"modlpy/utilisuba", account_id, delegator_index).using_encoded(blake2_256);
		let account_id = Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
			.map_err(|_| Error::<T>::DerivativeAccountIdFailed)?;
		Ok(account_id)
	}

	/// Wrapping any runtime call with as_derivative.
	pub fn wrap_utility_as_derivative_call_data(
		staking_protocol: &StakingProtocol,
		delegator_index: DelegatorIndex,
		call: Vec<u8>,
	) -> Vec<u8> {
		let utility_pallet_index = staking_protocol.info().utility_pallet_index;
		let mut call_data = utility_pallet_index.encode();
		call_data.extend(AS_DERIVATIVE_CALL_INDEX.encode());
		// derivative index
		call_data.extend(delegator_index.encode());
		// runtime call
		call_data.extend(call);
		call_data
	}

	/// Wrapping limited_reserve_transfer_assets
	pub fn wrap_polkadot_xcm_limited_reserve_transfer_assets_call_data(
		staking_protocol: &StakingProtocol,
		amount: Balance,
		to: T::AccountId,
	) -> Result<Vec<u8>, Error<T>> {
		let xcm_pallet_index = staking_protocol.info().xcm_pallet_index;
		let bifrost_dest_location = staking_protocol.info().bifrost_dest_location;
		let account_id =
			to.encode().try_into().map_err(|_| Error::<T>::DerivativeAccountIdFailed)?;
		let beneficiary = Location::new(0, AccountId32 { network: None, id: account_id });
		let fee_asset_item = 0u32;
		let weight_limit = WeightLimit::Unlimited;

		let mut calldata = xcm_pallet_index.encode();
		calldata.extend(LIMITED_RESERVE_TRANSFER_ASSETS_CALL_INDEX.encode());
		// bifrost_dest_location
		calldata.extend(VersionedLocation::V4(bifrost_dest_location).encode());
		// beneficiary
		calldata.extend(VersionedLocation::V4(beneficiary).encode());
		// native asset + amount
		calldata.extend(
			VersionedAssets::V4(Assets::from(vec![Asset {
				id: AssetId(Location::here()),
				fun: Fungible(amount),
			}]))
			.encode(),
		);
		// fee_asset_item
		calldata.extend(fee_asset_item.encode());
		// weight_limit
		calldata.extend(weight_limit.encode());
		Ok(calldata)
	}

	/// Wrapping xcm message
	/// withdraw_asset + buy_execution + transact + refund_surplus + deposit_asset
	pub fn wrap_xcm_message(
		staking_protocol: &StakingProtocol,
		call: Vec<u8>,
	) -> Result<Xcm, Error<T>> {
		let configuration = ConfigurationByStakingProtocol::<T>::get(staking_protocol)
			.ok_or(Error::<T>::ConfigurationNotFound)?;
		let fee_location = staking_protocol.info().remote_fee_location;
		let refund_beneficiary = staking_protocol.info().remote_refund_beneficiary;
		let asset =
			Asset { id: AssetId(fee_location), fun: Fungible(configuration.xcm_task_fee.fee) };
		let assets: Assets = Assets::from(asset.clone());
		let require_weight_at_most = configuration.xcm_task_fee.weight;
		let call: DoubleEncoded<()> = call.into();
		let asset_filter: AssetFilter = AssetFilter::Wild(WildAsset::All);
		Ok(Xcm::builder()
			.withdraw_asset(assets)
			.buy_execution(asset, WeightLimit::Unlimited)
			.transact(OriginKind::SovereignAccount, require_weight_at_most, call)
			.refund_surplus()
			.deposit_asset(asset_filter, refund_beneficiary)
			.build())
	}

	/// Wrapping xcm messages with notify
	/// withdraw_asset + buy_execution + transact + report_transact_status + refund_surplus +
	/// deposit_asset
	pub fn wrap_xcm_message_with_notify(
		staking_protocol: &StakingProtocol,
		call: Vec<u8>,
		notify_call: <T as Config>::RuntimeCall,
		mut_query_id: &mut Option<QueryId>,
	) -> Result<Xcm, Error<T>> {
		let notify_call_weight = notify_call.get_dispatch_info().weight;
		let now = frame_system::Pallet::<T>::block_number();
		let timeout = now.saturating_add(T::QueryTimeout::get());
		let responder = staking_protocol.info().remote_dest_location;
		let query_id =
			pallet_xcm::Pallet::<T>::new_notify_query(responder, notify_call, timeout, Here);
		*mut_query_id = Some(query_id);
		let destination = staking_protocol.info().bifrost_dest_location;
		let report_transact_status = ReportTransactStatus(QueryResponseInfo {
			destination,
			query_id,
			max_weight: notify_call_weight,
		});
		let mut xcm_message = Self::wrap_xcm_message(&staking_protocol, call)?;
		xcm_message.0.insert(3, report_transact_status);
		Ok(xcm_message)
	}

	pub fn send_xcm_message(
		staking_protocol: StakingProtocol,
		xcm_message: Xcm,
	) -> Result<(), Error<T>> {
		let dest_location = staking_protocol.info().remote_dest_location;
		let (ticket, _price) =
			T::XcmSender::validate(&mut Some(dest_location), &mut Some(xcm_message))
				.map_err(|_| Error::<T>::ValidatingFailed)?;
		T::XcmSender::deliver(ticket).map_err(|_| Error::<T>::DeliveringFailed)?;
		Ok(())
	}

	pub fn calculate_vtoken_amount_by_token_amount(
		vtoken_currency_id: CurrencyId,
		currency_id: CurrencyId,
		token_amount: Balance,
	) -> Result<Balance, Error<T>> {
		let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_currency_id);
		let token_pool_amount = T::VtokenMinting::get_token_pool(currency_id);
		// vtoken_amount / vtoken_total_issuance = token_amount / token_pool_amount
		// vtoken_amount = token_amount * vtoken_total_issuance / token_pool_amount
		let vtoken_amount = multiply_by_rational_with_rounding(
			token_amount,
			vtoken_total_issuance,
			token_pool_amount,
			Rounding::Down,
		)
		.ok_or(Error::<T>::CalculateProtocolFeeFailed)?;
		Ok(vtoken_amount)
	}

	pub fn ensure_governance_or_xcm_response(
		origin: OriginFor<T>,
	) -> Result<Location, DispatchError> {
		let responder = T::ResponseOrigin::ensure_origin(origin.clone())
			.or_else(|_| T::ControlOrigin::ensure_origin(origin).map(|_| Here.into()))?;
		Ok(responder)
	}

	pub fn ensure_governance_or_operator(
		origin: OriginFor<T>,
		staking_protocol: StakingProtocol,
	) -> Result<(), Error<T>> {
		match origin.clone().into() {
			Ok(RawOrigin::Signed(signer)) => {
				match ConfigurationByStakingProtocol::<T>::get(staking_protocol) {
					Some(c) => {
						ensure!(c.operator == signer, Error::<T>::NotAuthorized);
						Ok(())
					},
					None => Err(Error::<T>::NotAuthorized),
				}
			},
			_ => {
				T::ControlOrigin::ensure_origin(origin).map_err(|_| Error::<T>::NotAuthorized)?;
				Ok(())
			},
		}
	}

	pub fn ensure_delegator_exist(
		staking_protocol: &StakingProtocol,
		delegator: &Delegator<T::AccountId>,
	) -> Result<DelegatorIndex, Error<T>> {
		let delegator_index =
			DelegatorIndexByStakingProtocolAndDelegator::<T>::get(staking_protocol, delegator)
				.ok_or(Error::<T>::DelegatorIndexNotFound)?;
		ensure!(
			DelegatorByStakingProtocolAndDelegatorIndex::<T>::contains_key(
				staking_protocol,
				delegator_index
			),
			Error::<T>::DelegatorNotFound
		);
		Ok(delegator_index)
	}
}

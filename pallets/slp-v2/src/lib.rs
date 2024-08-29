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

#![cfg_attr(not(feature = "std"), no_std)]

use astar_dapp_staking::types::DappStaking;
use bifrost_primitives::{
	Balance, CurrencyId, CurrencyIdConversion, TimeUnit, VtokenMintingOperator,
};
use common::types::{Delegator, DelegatorIndex};
use frame_support::{
	dispatch::{DispatchResultWithPostInfo, GetDispatchInfo},
	pallet_prelude::*,
	PalletId,
};
use frame_system::pallet_prelude::*;
use orml_traits::{MultiCurrency, XcmTransfer};
use polkadot_parachain_primitives::primitives::Id as ParaId;
use sp_runtime::{traits::AccountIdConversion, Permill};
use xcm::v4::{Location, SendXcm};

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod astar_dapp_staking;
mod common;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::common::types::{
		Ledger, PendingStatus, StakingProtocol, Validator, XcmFee, XcmTask, XcmTaskWithParams,
	};
	use sp_runtime::traits::BlockNumberProvider;
	use xcm::latest::{MaybeErrorCode, QueryId, Response};

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_xcm::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type RuntimeOrigin: IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>;
		type RuntimeCall: IsType<<Self as pallet_xcm::Config>::RuntimeCall>
			+ From<Call<Self>>
			+ GetDispatchInfo;
		type ResponseOrigin: EnsureOrigin<
			<Self as frame_system::Config>::RuntimeOrigin,
			Success = Location,
		>;
		type WeightInfo: crate::weights::WeightInfo;
		type MultiCurrency: MultiCurrency<
			Self::AccountId,
			Balance = Balance,
			CurrencyId = CurrencyId,
		>;
		/// The only origin that can modify pallet params
		type ControlOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// Xcm sender.
		type XcmSender: SendXcm;
		/// XTokens transfer interface
		type XcmTransfer: XcmTransfer<Self::AccountId, Balance, CurrencyId>;
		/// The interface to call VtokenMinting module functions.
		type VtokenMinting: VtokenMintingOperator<CurrencyId, Balance, Self::AccountId, TimeUnit>;
		/// The currency id conversion.
		type CurrencyIdConversion: CurrencyIdConversion<CurrencyId>;
		/// The current block number provider.
		type RelaychainBlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;
		/// The query timeout.
		#[pallet::constant]
		type QueryTimeout: Get<BlockNumberFor<Self>>;
		/// Commission master Pallet Id to get the commission master account
		#[pallet::constant]
		type CommissionPalletId: Get<PalletId>;
		/// Bifrost parachain id.
		#[pallet::constant]
		type ParachainId: Get<ParaId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// StakingProtocol + DelegatorIndex => Delegator
	#[pallet::storage]
	pub type DelegatorByStakingProtocolAndDelegatorIndex<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StakingProtocol,
		Blake2_128Concat,
		DelegatorIndex,
		Delegator<T::AccountId>,
		OptionQuery,
	>;

	/// StakingProtocol + Delegator => DelegatorIndex
	#[pallet::storage]
	pub type DelegatorIndexByStakingProtocolAndDelegator<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StakingProtocol,
		Blake2_128Concat,
		Delegator<T::AccountId>,
		DelegatorIndex,
		OptionQuery,
	>;

	/// StakingProtocol + DelegatorIndex => Delegator
	#[pallet::storage]
	pub type LedgerByStakingProtocolAndDelegator<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StakingProtocol,
		Blake2_128Concat,
		Delegator<T::AccountId>,
		Ledger,
		OptionQuery,
	>;

	/// Validators for different staking protocols.
	#[pallet::storage]
	pub type ValidatorsByStakingProtocolAndDelegator<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StakingProtocol,
		Blake2_128Concat,
		Delegator<T::AccountId>,
		BoundedVec<Validator<T::AccountId>, ConstU32<1000>>,
		OptionQuery,
	>;

	/// Validators for different staking protocols.
	#[pallet::storage]
	pub type ValidatorsByStakingProtocol<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		StakingProtocol,
		BoundedVec<Validator<T::AccountId>, ConstU32<1000>>,
		ValueQuery,
	>;

	/// Next index of different staking protocols.
	#[pallet::storage]
	pub type NextDelegatorIndexByStakingProtocol<T: Config> =
		StorageMap<_, Blake2_128Concat, StakingProtocol, DelegatorIndex, ValueQuery>;

	/// XCM fee for different XCM tasks.
	#[pallet::storage]
	pub type XcmFeeByXcmTask<T: Config> =
		StorageMap<_, Blake2_128Concat, XcmTask, XcmFee, OptionQuery>;

	/// Pending status for different query id.
	#[pallet::storage]
	pub type PendingStatusByQueryId<T: Config> =
		StorageMap<_, Blake2_128Concat, QueryId, PendingStatus<T::AccountId>, OptionQuery>;

	/// Update ongoing time unit interval for different staking protocols.
	#[pallet::storage]
	pub type UpdateOngoingTimeUintIntervalByStakingProtocol<T> =
		StorageMap<_, Blake2_128Concat, StakingProtocol, BlockNumberFor<T>, ValueQuery>;

	/// Last update ongoing time unit block number for different staking protocols.
	#[pallet::storage]
	pub type LastUpdateOngoingTimeUnitBlockNumber<T> =
		StorageMap<_, Blake2_128Concat, StakingProtocol, BlockNumberFor<T>, ValueQuery>;

	/// Update token exchange rate limit for different staking protocols.
	#[pallet::storage]
	pub type UpdateTokenExchangeRateLimitByStakingProtocol<T> =
		StorageMap<_, Blake2_128Concat, StakingProtocol, (BlockNumberFor<T>, Permill), ValueQuery>;

	/// Last update token exchange rate block number for different staking protocols.
	#[pallet::storage]
	pub type LastUpdateTokenExchangeRateBlockNumber<T> =
		StorageMap<_, Blake2_128Concat, StakingProtocol, BlockNumberFor<T>, ValueQuery>;

	/// Protocol fee rate for different staking protocols.
	#[pallet::storage]
	pub type ProtocolFeeRateByStakingProtocol<T> =
		StorageMap<_, Blake2_128Concat, StakingProtocol, Permill, ValueQuery>;

	/// Operator for different staking protocols.
	#[pallet::storage]
	pub type OperatorByStakingProtocol<T: Config> =
		StorageMap<_, Blake2_128Concat, StakingProtocol, T::AccountId, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		AddDelegator {
			staking_protocol: StakingProtocol,
			delegator_index: DelegatorIndex,
			delegator: Delegator<T::AccountId>,
		},
		RemoveDelegator {
			staking_protocol: StakingProtocol,
			delegator_index: DelegatorIndex,
			delegator: Delegator<T::AccountId>,
		},
		AddValidator {
			staking_protocol: StakingProtocol,
			validator: Validator<T::AccountId>,
		},
		RemoveValidator {
			staking_protocol: StakingProtocol,
			validator: Validator<T::AccountId>,
		},
		SetXcmFee {
			xcm_task: XcmTask,
			xcm_fee: XcmFee,
		},
		SetProtocolFeeRate {
			staking_protocol: StakingProtocol,
			fee_rate: Permill,
		},
		SetUpdateOngoingTimeUnitInterval {
			staking_protocol: StakingProtocol,
			update_interval: BlockNumberFor<T>,
		},
		SetUpdateTokenExchangeRateLimit {
			staking_protocol: StakingProtocol,
			update_interval: BlockNumberFor<T>,
			max_update_permill: Permill,
		},
		SetLedger {
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
			ledger: Ledger,
		},
		SetOperator {
			staking_protocol: StakingProtocol,
			operator: T::AccountId,
		},
		SendXcmTask {
			query_id: Option<QueryId>,
			delegator: Delegator<T::AccountId>,
			xcm_task_with_params: XcmTaskWithParams<T::AccountId>,
			pending_status: Option<PendingStatus<T::AccountId>>,
			dest_location: Location,
		},
		NotifyResponseReceived {
			responder: Location,
			pending_status: PendingStatus<T::AccountId>,
		},
		TimeUnitUpdated {
			staking_protocol: StakingProtocol,
			time_unit: TimeUnit,
		},
		TokenExchangeRateUpdated {
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
			protocol_fee_currency_id: CurrencyId,
			protocol_fee: Balance,
			amount: Balance,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Delegator index has exceeded the maximum allowed value of 65535.
		DelegatorIndexOverflow,
		/// The staking protocol used by the delegator is not supported.
		UnsupportedStakingProtocolForDelegator,
		/// The staking protocol is not supported.
		UnsupportedStakingProtocol,
		/// The delegator index was not found.
		DelegatorIndexNotFound,
		/// The delegator was not found.
		DelegatorNotFound,
		/// The ledger was not found.
		LedgerNotFound,
		/// The validator was not found.
		ValidatorNotFound,
		/// The delegator already exists.
		DelegatorAlreadyExists,
		/// The delegator index already exists.
		DelegatorIndexAlreadyExists,
		/// The validator already exists.
		ValidatorAlreadyExists,
		/// The maximum number of validators has been reached.
		ValidatorsTooMuch,
		/// Failed to derive the derivative account ID.
		DerivativeAccountIdFailed,
		/// Missing XCM fee value.
		MissingXcmFee,
		/// Missing pending status.
		MissingPendingStatus,
		/// Missing query ID.
		MissingQueryId,
		/// Error during validation.
		ErrorValidating,
		/// Error during delivery.
		ErrorDelivering,
		/// The specified time unit does not exist.
		TimeUnitNotExist,
		/// The specified time unit is too short.
		UpdateOngoingTimeUnitIntervalTooShort,
		/// The specified token exchange rate is too short.
		UpdateTokenExchangeRateIntervalTooShort,
		/// The specified token exchange rate amount is too large.
		UpdateTokenExchangeRateAmountTooLarge,
		/// Invalid parameter.
		InvalidParameter,
		/// calculate protocol fee failed.
		CalculateProtocolFeeFailed,
		/// Not authorized.
		NotAuthorized,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a delegator to the staking protocol.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn add_delegator(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Option<Delegator<T::AccountId>>,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::do_add_delegator(staking_protocol, delegator)
		}

		/// Remove a delegator from the staking protocol.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn remove_delegator(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::do_remove_delegator(staking_protocol, delegator)
		}

		/// Add a validator to the staking protocol.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn add_validator(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			validator: Validator<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			ValidatorsByStakingProtocol::<T>::mutate(
				staking_protocol,
				|validators| -> DispatchResultWithPostInfo {
					ensure!(!validators.contains(&validator), Error::<T>::ValidatorAlreadyExists);
					validators
						.try_push(validator.clone())
						.map_err(|_| Error::<T>::ValidatorsTooMuch)?;
					Self::deposit_event(Event::<T>::AddValidator { staking_protocol, validator });
					Ok(().into())
				},
			)
		}

		/// Remove a validator from the staking protocol.
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn remove_validator(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			validator: Validator<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			ValidatorsByStakingProtocol::<T>::mutate(
				staking_protocol,
				|validators| -> DispatchResultWithPostInfo {
					ensure!(validators.contains(&validator), Error::<T>::ValidatorNotFound);
					validators.retain(|v| *v != validator);
					Self::deposit_event(Event::<T>::RemoveValidator {
						staking_protocol,
						validator,
					});
					Ok(().into())
				},
			)
		}

		/// Set the XCM fee for a specific XCM task.
		#[pallet::call_index(4)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn set_xcm_task_fee(
			origin: OriginFor<T>,
			xcm_task: XcmTask,
			xcm_fee: XcmFee,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			XcmFeeByXcmTask::<T>::mutate(
				xcm_task,
				|storage_xcm_fee| -> DispatchResultWithPostInfo {
					ensure!(Some(xcm_fee).ne(storage_xcm_fee), Error::<T>::InvalidParameter);
					*storage_xcm_fee = Some(xcm_fee);
					Self::deposit_event(Event::SetXcmFee { xcm_task, xcm_fee });
					Ok(().into())
				},
			)
		}

		/// Set the protocol fee rate for a specific staking protocol.
		#[pallet::call_index(5)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn set_protocol_fee_rate(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			fee_rate: Permill,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			ProtocolFeeRateByStakingProtocol::<T>::mutate(
				staking_protocol,
				|storage_fee_rate| -> DispatchResultWithPostInfo {
					ensure!(*storage_fee_rate != fee_rate, Error::<T>::InvalidParameter);
					*storage_fee_rate = fee_rate;
					Self::deposit_event(Event::SetProtocolFeeRate { staking_protocol, fee_rate });
					Ok(().into())
				},
			)
		}

		/// Set the update ongoing time unit interval for a specific staking protocol.
		#[pallet::call_index(6)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn set_update_ongoing_time_unit_interval(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			update_interval: BlockNumberFor<T>,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			UpdateOngoingTimeUintIntervalByStakingProtocol::<T>::mutate(
				staking_protocol,
				|storage_update_interval| -> DispatchResultWithPostInfo {
					ensure!(
						update_interval.ne(storage_update_interval),
						Error::<T>::InvalidParameter
					);
					*storage_update_interval = update_interval;
					Self::deposit_event(Event::SetUpdateOngoingTimeUnitInterval {
						staking_protocol,
						update_interval,
					});
					Ok(().into())
				},
			)
		}

		/// Set the update token exchange rate limit for a specific staking protocol.
		#[pallet::call_index(7)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn set_update_token_exchange_rate_limit(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			update_interval: BlockNumberFor<T>,
			max_update_permill: Permill,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			UpdateTokenExchangeRateLimitByStakingProtocol::<T>::mutate(staking_protocol, |(storage_update_interval, storage_max_update_permill)| -> DispatchResultWithPostInfo {
				ensure!(update_interval.ne(storage_update_interval) && max_update_permill.ne(storage_max_update_permill), Error::<T>::InvalidParameter);
				*storage_update_interval = update_interval;
				*storage_max_update_permill = max_update_permill;
				Self::deposit_event(Event::SetUpdateTokenExchangeRateLimit { staking_protocol, update_interval, max_update_permill });
				Ok(().into())
			})
		}

		/// Set the update token exchange rate limit for a specific staking protocol.
		#[pallet::call_index(8)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn set_ledger(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
			ledger: Ledger,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			let delegator_index = DelegatorIndexByStakingProtocolAndDelegator::<T>::get(
				staking_protocol,
				delegator.clone(),
			)
			.ok_or(Error::<T>::DelegatorIndexNotFound)?;
			ensure!(
				DelegatorByStakingProtocolAndDelegatorIndex::<T>::contains_key(
					staking_protocol,
					delegator_index
				),
				Error::<T>::DelegatorNotFound
			);
			LedgerByStakingProtocolAndDelegator::<T>::mutate(
				staking_protocol,
				delegator.clone(),
				|storage_ledger| -> DispatchResultWithPostInfo {
					ensure!(Some(ledger.clone()).ne(storage_ledger), Error::<T>::InvalidParameter);
					*storage_ledger = Some(ledger.clone());
					Self::deposit_event(Event::SetLedger { staking_protocol, delegator, ledger });
					Ok(().into())
				},
			)
		}

		/// Set the operator for a specific staking protocol.
		#[pallet::call_index(9)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn set_operator(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			operator: T::AccountId,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			OperatorByStakingProtocol::<T>::mutate(
				staking_protocol,
				|storage_operator| -> DispatchResultWithPostInfo {
					ensure!(
						Some(operator.clone()).ne(storage_operator),
						Error::<T>::InvalidParameter
					);
					*storage_operator = Some(operator.clone());
					Self::deposit_event(Event::SetOperator { staking_protocol, operator });
					Ok(().into())
				},
			)
		}

		/// Transfer the staking token to remote chain.
		#[pallet::call_index(10)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn transfer_to(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_governance_or_operator(origin, staking_protocol)?;
			Self::do_transfer_to(staking_protocol, delegator)
		}

		/// Transfer the staking token back from remote chain.
		#[pallet::call_index(11)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn transfer_back(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
			amount: Balance,
		) -> DispatchResultWithPostInfo {
			Self::ensure_governance_or_operator(origin, staking_protocol)?;
			Self::do_transfer_back(staking_protocol, delegator, amount)
		}

		/// Update the ongoing time unit for a specific staking protocol.
		#[pallet::call_index(12)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn update_ongoing_time_unit(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
		) -> DispatchResultWithPostInfo {
			Self::ensure_governance_or_operator(origin, staking_protocol)?;
			let current_block_number = T::RelaychainBlockNumberProvider::current_block_number();
			let update_interval =
				UpdateOngoingTimeUintIntervalByStakingProtocol::<T>::get(staking_protocol);
			let last_update_block_number =
				LastUpdateOngoingTimeUnitBlockNumber::<T>::get(staking_protocol);
			ensure!(
				current_block_number > last_update_block_number + update_interval,
				Error::<T>::UpdateOngoingTimeUnitIntervalTooShort
			);

			let currency_id = staking_protocol.get_currency_id();

			let time_unit = match T::VtokenMinting::get_ongoing_time_unit(currency_id) {
				Some(time_unit) => time_unit.add_one(),
				None => staking_protocol.get_default_time_unit(),
			};
			T::VtokenMinting::update_ongoing_time_unit(currency_id, time_unit.clone())?;
			LastUpdateOngoingTimeUnitBlockNumber::<T>::insert(
				staking_protocol,
				current_block_number,
			);
			Self::deposit_event(Event::<T>::TimeUnitUpdated { staking_protocol, time_unit });
			Ok(().into())
		}

		/// Update the token exchange rate for a specific staking protocol.
		#[pallet::call_index(13)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn update_token_exchange_rate(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
			amount: Balance,
		) -> DispatchResultWithPostInfo {
			Self::ensure_governance_or_operator(origin, staking_protocol)?;
			let currency_id = staking_protocol.get_currency_id();

			// Check the update token exchange rate limit.
			let (update_interval, max_update_permill) =
				UpdateTokenExchangeRateLimitByStakingProtocol::<T>::get(staking_protocol);
			let current_block_number = T::RelaychainBlockNumberProvider::current_block_number();
			let last_update_block_number =
				LastUpdateTokenExchangeRateBlockNumber::<T>::get(staking_protocol);
			ensure!(
				current_block_number > last_update_block_number + update_interval,
				Error::<T>::UpdateTokenExchangeRateIntervalTooShort
			);
			let pool_token_amount = T::VtokenMinting::get_token_pool(currency_id);
			let max_amount = max_update_permill.mul_floor(pool_token_amount);
			ensure!(
				amount <= max_amount || max_amount == 0,
				Error::<T>::UpdateTokenExchangeRateAmountTooLarge
			);

			// Charge the protocol fee.
			let protocol_fee_rate = ProtocolFeeRateByStakingProtocol::<T>::get(staking_protocol);
			let mut protocol_fee = protocol_fee_rate.mul_floor(amount);
			let protocol_fee_currency_id = T::CurrencyIdConversion::convert_to_vtoken(currency_id)
				.map_err(|_| Error::<T>::DerivativeAccountIdFailed)?;
			if protocol_fee != 0 {
				protocol_fee = Self::calculate_vtoken_amount_by_token_amount(
					protocol_fee_currency_id,
					currency_id,
					protocol_fee,
				)?;
				let protocol_fee_receiver = T::CommissionPalletId::get().into_account_truncating();
				T::MultiCurrency::deposit(
					protocol_fee_currency_id,
					&protocol_fee_receiver,
					protocol_fee,
				)?;
			}

			// Update the token exchange rate.
			T::VtokenMinting::increase_token_pool(currency_id, amount)
				.map_err(|_| Error::<T>::DerivativeAccountIdFailed)?;
			LedgerByStakingProtocolAndDelegator::<T>::mutate(
				staking_protocol,
				delegator.clone(),
				|ledger| match ledger {
					Some(Ledger::AstarDappStaking(astar_dapp_staking_ledger)) => {
						astar_dapp_staking_ledger.add_lock_amount(amount);
						Ok(())
					},
					_ => Err(Error::<T>::LedgerNotFound),
				},
			)?;

			Self::deposit_event(Event::<T>::TokenExchangeRateUpdated {
				staking_protocol,
				delegator,
				protocol_fee_currency_id,
				protocol_fee,
				amount,
			});
			Ok(().into())
		}

		#[pallet::call_index(14)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn astar_dapp_staking(
			origin: OriginFor<T>,
			delegator: Delegator<T::AccountId>,
			task: DappStaking<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_governance_or_operator(origin, StakingProtocol::AstarDappStaking)?;
			Self::do_dapp_staking(delegator, task)
		}

		#[pallet::call_index(15)]
		#[pallet::weight(Weight::from_parts(10_000, 0) + T::DbWeight::get().writes(1))]
		pub fn notify_astar_dapp_staking(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResultWithPostInfo {
			let responder = Self::ensure_governance_or_xcm_response(origin)?;
			let pending_status = PendingStatusByQueryId::<T>::get(query_id)
				.ok_or(Error::<T>::MissingPendingStatus)?;
			if Response::DispatchResult(MaybeErrorCode::Success) == response {
				Self::do_notify_astar_dapp_staking(responder, pending_status)?;
			};
			Ok(().into())
		}
	}
}

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

#[cfg(feature = "polkadot")]
use astar_dapp_staking::types::DappStaking;
use bifrost_primitives::{
	Balance, BlockNumber, CurrencyId, CurrencyIdConversion, TimeUnit, VtokenMintingOperator,
};
use common::types::{Delegator, DelegatorIndex, ProtocolConfiguration};
use frame_support::{
	dispatch::{DispatchResultWithPostInfo, GetDispatchInfo},
	pallet_prelude::*,
	PalletId,
};
use frame_system::pallet_prelude::*;
use orml_traits::{MultiCurrency, XcmTransfer};
use polkadot_parachain_primitives::primitives::Id as ParaId;
use sp_runtime::traits::AccountIdConversion;
pub use weights::WeightInfo;
use xcm::v4::{Location, SendXcm};

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(feature = "polkadot")]
mod astar_dapp_staking;
mod common;
#[cfg(test)]
mod tests;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::common::types::{Ledger, PendingStatus, StakingProtocol, Validator, XcmTask};
	use sp_runtime::{traits::BlockNumberProvider, Permill};
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
		type WeightInfo: weights::WeightInfo;
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
		type RelaychainBlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumber>;
		/// The query timeout.
		#[pallet::constant]
		type QueryTimeout: Get<BlockNumberFor<Self>>;
		/// Commission master Pallet Id to get the commission master account
		#[pallet::constant]
		type CommissionPalletId: Get<PalletId>;
		/// Bifrost parachain id.
		#[pallet::constant]
		type ParachainId: Get<ParaId>;
		/// Maximum validators
		#[pallet::constant]
		type MaxValidators: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configuration for different staking protocols.
	#[pallet::storage]
	pub type ConfigurationByStakingProtocol<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		StakingProtocol,
		ProtocolConfiguration<T::AccountId>,
		OptionQuery,
	>;

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
		BoundedVec<Validator<T::AccountId>, T::MaxValidators>,
		ValueQuery,
	>;

	/// Next index of different staking protocols.
	#[pallet::storage]
	pub type NextDelegatorIndexByStakingProtocol<T: Config> =
		StorageMap<_, Blake2_128Concat, StakingProtocol, DelegatorIndex, ValueQuery>;

	/// Pending status for different query id.
	#[pallet::storage]
	pub type PendingStatusByQueryId<T: Config> =
		StorageMap<_, Blake2_128Concat, QueryId, PendingStatus<T::AccountId>, OptionQuery>;

	/// Last update ongoing time unit block number for different staking protocols.
	#[pallet::storage]
	pub type LastUpdateOngoingTimeUnitBlockNumber<T> =
		StorageMap<_, Blake2_128Concat, StakingProtocol, BlockNumber, ValueQuery>;

	/// Last update token exchange rate block number for different staking protocols.
	#[pallet::storage]
	pub type LastUpdateTokenExchangeRateBlockNumber<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		StakingProtocol,
		Blake2_128Concat,
		Delegator<T::AccountId>,
		BlockNumber,
		ValueQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Add a delegator to the staking protocol.
		AddDelegator {
			/// Slp supports staking protocols.
			staking_protocol: StakingProtocol,
			/// Delegator index.
			delegator_index: DelegatorIndex,
			/// Delegator account.
			delegator: Delegator<T::AccountId>,
		},
		/// Remove a delegator from the staking protocol.
		RemoveDelegator {
			/// Slp supports staking protocols.
			staking_protocol: StakingProtocol,
			/// Delegator index.
			delegator_index: DelegatorIndex,
			/// Delegator account.
			delegator: Delegator<T::AccountId>,
		},
		/// Add a validator to the staking protocol.
		AddValidator {
			/// Slp supports staking protocols.
			staking_protocol: StakingProtocol,
			/// Delegator account.
			delegator: Delegator<T::AccountId>,
			/// Validator account.
			validator: Validator<T::AccountId>,
		},
		/// Remove a validator from the staking protocol.
		RemoveValidator {
			/// Slp supports staking protocols.
			staking_protocol: StakingProtocol,
			/// Delegator account.
			delegator: Delegator<T::AccountId>,
			/// Validator account.
			validator: Validator<T::AccountId>,
		},
		/// Set configuration for a specific staking protocol.
		SetConfiguration {
			/// Slp supports staking protocols.
			staking_protocol: StakingProtocol,
			/// The staking protocol configuration.
			configuration: ProtocolConfiguration<T::AccountId>,
		},
		/// Set ledger for a specific delegator.
		SetLedger {
			/// Slp supports staking protocols.
			staking_protocol: StakingProtocol,
			/// Delegator account.
			delegator: Delegator<T::AccountId>,
			/// Ledger.
			ledger: Ledger,
		},
		/// Send xcm task.
		SendXcmTask {
			/// Xcm Message Query id.
			query_id: Option<QueryId>,
			/// Delegator account.
			delegator: Delegator<T::AccountId>,
			/// Xcm task.
			task: XcmTask<T::AccountId>,
			/// Pending confirmation status.
			pending_status: Option<PendingStatus<T::AccountId>>,
			/// Destination.
			dest_location: Location,
		},
		/// Xcm task response received.
		NotifyResponseReceived {
			/// Xcm responder.
			responder: Location,
			/// Pending confirmation status.
			pending_status: PendingStatus<T::AccountId>,
		},
		/// Time unit updated.
		TimeUnitUpdated {
			/// Slp supports staking protocols.
			staking_protocol: StakingProtocol,
			/// Time unit.
			time_unit: TimeUnit,
		},
		/// Token exchange rate updated.
		TokenExchangeRateUpdated {
			/// Slp supports staking protocols.
			staking_protocol: StakingProtocol,
			/// Delegator account.
			delegator: Delegator<T::AccountId>,
			/// The type of token that the fee is charged to
			protocol_fee_currency_id: CurrencyId,
			/// The amount of the fee charged to the protocol
			protocol_fee: Balance,
			/// Amount of exchange rates updated
			amount: Balance,
		},
		/// Transfer the staking token to remote chain.
		TransferTo {
			/// Slp supports staking protocols.
			staking_protocol: StakingProtocol,
			/// Bifrost Account
			from: T::AccountId,
			/// Delegator account.
			to: Delegator<T::AccountId>,
			/// Amount
			amount: Balance,
		},
		/// Transfer the staking token back from remote chain.
		TransferBack {
			/// Slp supports staking protocols.
			staking_protocol: StakingProtocol,
			/// Delegator account.
			from: Delegator<T::AccountId>,
			/// Bifrost Account.
			to: T::AccountId,
			/// Amount
			amount: Balance,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Delegator index has exceeded the maximum allowed value of 65535.
		DelegatorIndexOverflow,
		/// The maximum number of validators has been reached.
		ValidatorsOverflow,
		/// UnlockRecordOverflow
		UnlockRecordOverflow,
		/// The staking protocol is not supported.
		UnsupportedStakingProtocol,
		/// The delegator index was not found.
		DelegatorIndexNotFound,
		/// The Configuration was not found.
		ConfigurationNotFound,
		/// The delegator was not found.
		DelegatorNotFound,
		/// The ledger was not found.
		LedgerNotFound,
		/// The validator was not found.
		ValidatorNotFound,
		/// Missing XCM fee value.
		XcmFeeNotFound,
		/// Missing pending status.
		PendingStatusNotFound,
		/// The specified time unit does not exist.
		TimeUnitNotFound,
		/// The delegator already exists.
		DelegatorAlreadyExists,
		/// The delegator index already exists.
		DelegatorIndexAlreadyExists,
		/// The validator already exists.
		ValidatorAlreadyExists,
		/// Failed to derive the derivative account ID.
		DerivativeAccountIdFailed,
		/// Error during validation.
		ValidatingFailed,
		/// Error during delivery.
		DeliveringFailed,
		/// calculate protocol fee failed.
		CalculateProtocolFeeFailed,
		/// IncreaseTokenPoolFailed
		IncreaseTokenPoolFailed,
		/// The update interval is too short.
		UpdateIntervalTooShort,
		/// The specified token exchange rate amount is too large.
		UpdateTokenExchangeRateAmountTooLarge,
		/// Invalid parameter.
		InvalidParameter,
		/// Not authorized.
		NotAuthorized,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the XCM fee for a specific XCM task.
		///
		/// Can only be called by governance
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `configuration`: The staking protocol configuration.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::set_protocol_configuration())]
		pub fn set_protocol_configuration(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			configuration: ProtocolConfiguration<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			ConfigurationByStakingProtocol::<T>::mutate(
				staking_protocol,
				|storage_configuration| -> DispatchResultWithPostInfo {
					ensure!(
						Some(configuration.clone()).ne(storage_configuration),
						Error::<T>::InvalidParameter
					);
					*storage_configuration = Some(configuration.clone());
					Self::deposit_event(Event::SetConfiguration {
						staking_protocol,
						configuration,
					});
					Ok(().into())
				},
			)
		}

		/// Add a delegator to the staking protocol.
		///
		/// Can only be called by governance
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `delegator`: If delegator is None, the delegator will be derived from sovereign
		///   account.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::add_delegator())]
		pub fn add_delegator(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Option<Delegator<T::AccountId>>,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::do_add_delegator(staking_protocol, delegator)
		}

		/// Remove a delegator from the staking protocol.
		///
		/// Can only be called by governance
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `delegator`: Delegator that need to be removed.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_delegator())]
		pub fn remove_delegator(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::do_remove_delegator(staking_protocol, delegator)
		}

		/// Add a validator to the staking protocol.
		///
		/// Can only be called by governance
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `delegator`: Select the delegator which is existed.
		/// - `validator`: Validator that need to be added.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::add_validator())]
		pub fn add_validator(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
			validator: Validator<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_delegator_exist(&staking_protocol, &delegator)?;
			ValidatorsByStakingProtocolAndDelegator::<T>::mutate(
				staking_protocol,
				delegator.clone(),
				|validators| -> DispatchResultWithPostInfo {
					ensure!(!validators.contains(&validator), Error::<T>::ValidatorAlreadyExists);
					validators
						.try_push(validator.clone())
						.map_err(|_| Error::<T>::ValidatorsOverflow)?;
					Self::deposit_event(Event::<T>::AddValidator {
						staking_protocol,
						delegator,
						validator,
					});
					Ok(().into())
				},
			)
		}

		/// Remove a validator from the staking protocol.
		///
		/// Can only be called by governance
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `delegator`: Select the delegator which is existed.
		/// - `validator`: Validator that need to be removed.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_validator())]
		pub fn remove_validator(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
			validator: Validator<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_delegator_exist(&staking_protocol, &delegator)?;
			ValidatorsByStakingProtocolAndDelegator::<T>::mutate(
				staking_protocol,
				delegator.clone(),
				|validators| -> DispatchResultWithPostInfo {
					ensure!(validators.contains(&validator), Error::<T>::ValidatorNotFound);
					validators.retain(|v| *v != validator);
					Self::deposit_event(Event::<T>::RemoveValidator {
						staking_protocol,
						delegator,
						validator,
					});
					Ok(().into())
				},
			)
		}

		/// Set the update token exchange rate limit for a specific staking protocol.
		///
		/// Can only be called by governance.
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `delegator`: Select the delegator which is existed.
		/// - `ledger`: Ledger that need to be set.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::set_ledger())]
		pub fn set_ledger(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
			ledger: Ledger,
		) -> DispatchResultWithPostInfo {
			T::ControlOrigin::ensure_origin(origin)?;
			Self::ensure_delegator_exist(&staking_protocol, &delegator)?;
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

		/// Transfer the staking token to remote chain.
		/// Transfer the free balance of the Entrance Account to the selected delegator.
		///
		/// Can be called by governance or staking protocol operator.
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `delegator`: Select the delegator which is existed.
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::transfer_to())]
		pub fn transfer_to(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_governance_or_operator(origin, staking_protocol)?;
			Self::do_transfer_to(staking_protocol, delegator)
		}

		/// Transfer the staking token back from remote chain.
		/// Transfer the amount of tokens from the selected delegator back to the entrance account.
		///
		/// Can be called by governance or staking protocol operator.
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `delegator`: Select the delegator which is existed.
		/// - `amount`: The amount of tokens to transfer back.
		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::transfer_back())]
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
		/// Update frequency controlled by update_time_unit_interval.
		/// Less than update_time_unit_interval will report an error.
		///
		/// Can be called by governance or staking protocol operator.
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `time_uint_option`: If time_uint is None, the ongoing time unit will be increased by
		///   one. Otherwise, the ongoing time unit will be updated to the specified time unit.
		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::update_ongoing_time_unit())]
		pub fn update_ongoing_time_unit(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			time_uint_option: Option<TimeUnit>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_governance_or_operator(origin, staking_protocol)?;
			let current_block_number = T::RelaychainBlockNumberProvider::current_block_number();
			let update_interval = match ConfigurationByStakingProtocol::<T>::get(staking_protocol) {
				Some(configuration) => configuration.update_time_unit_interval,
				None => 0,
			};
			let last_update_block_number =
				LastUpdateOngoingTimeUnitBlockNumber::<T>::get(staking_protocol);
			ensure!(
				current_block_number >= last_update_block_number + update_interval,
				Error::<T>::UpdateIntervalTooShort
			);

			let currency_id = staking_protocol.info().currency_id;

			let time_unit = match time_uint_option {
				Some(time_unit) => time_unit,
				None => {
					let current_time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
						.ok_or(Error::<T>::TimeUnitNotFound)?;
					current_time_unit.add_one()
				},
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
		/// Update frequency controlled by update_exchange_rate_interval.
		/// Amount max update for token pool * max_update_token_exchange_rate.
		///
		/// Can be called by governance or staking protocol operator.
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `delegator`: Select the delegator which is existed.
		/// - `amount`: The amount of tokens to update the token exchange rate.
		#[pallet::call_index(9)]
		#[pallet::weight(<T as Config>::WeightInfo::update_token_exchange_rate())]
		pub fn update_token_exchange_rate(
			origin: OriginFor<T>,
			staking_protocol: StakingProtocol,
			delegator: Delegator<T::AccountId>,
			amount: Balance,
		) -> DispatchResultWithPostInfo {
			Self::ensure_governance_or_operator(origin, staking_protocol)?;
			let currency_id = staking_protocol.info().currency_id;

			// Check the update token exchange rate limit.
			let (update_interval, max_update_permill, protocol_fee_rate) =
				match ConfigurationByStakingProtocol::<T>::get(staking_protocol) {
					Some(configuration) => (
						configuration.update_exchange_rate_interval,
						configuration.max_update_token_exchange_rate,
						configuration.protocol_fee_rate,
					),
					None => (0, Permill::zero(), Permill::zero()),
				};
			let current_block_number = T::RelaychainBlockNumberProvider::current_block_number();
			let last_update_block_number = LastUpdateTokenExchangeRateBlockNumber::<T>::get(
				staking_protocol,
				delegator.clone(),
			);
			ensure!(
				current_block_number >= last_update_block_number + update_interval,
				Error::<T>::UpdateIntervalTooShort
			);
			let pool_token_amount = T::VtokenMinting::get_token_pool(currency_id);
			let max_amount = max_update_permill.mul_floor(pool_token_amount);
			ensure!(
				amount <= max_amount || max_amount == 0,
				Error::<T>::UpdateTokenExchangeRateAmountTooLarge
			);

			// Charge the protocol fee.
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
				.map_err(|_| Error::<T>::IncreaseTokenPoolFailed)?;
			LedgerByStakingProtocolAndDelegator::<T>::mutate(
				staking_protocol,
				delegator.clone(),
				|ledger| match ledger {
					#[cfg(feature = "polkadot")]
					Some(Ledger::AstarDappStaking(astar_dapp_staking_ledger)) => {
						astar_dapp_staking_ledger.add_lock_amount(amount);
						Ok(())
					},
					_ => Err(Error::<T>::LedgerNotFound),
				},
			)?;

			LastUpdateTokenExchangeRateBlockNumber::<T>::insert(
				staking_protocol,
				delegator.clone(),
				current_block_number,
			);
			Self::deposit_event(Event::<T>::TokenExchangeRateUpdated {
				staking_protocol,
				delegator,
				protocol_fee_currency_id,
				protocol_fee,
				amount,
			});
			Ok(().into())
		}

		/// Manipulate a delegator to perform Dapp staking related operations.
		///
		/// Can be called by governance or staking protocol operator.
		///
		/// Parameters
		/// - `staking_protocol`: Slp supports staking protocols.
		/// - `delegator`: Select the delegator which is existed.
		/// - `task`: The Dapp staking task.
		#[cfg(feature = "polkadot")]
		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::astar_dapp_staking())]
		pub fn astar_dapp_staking(
			origin: OriginFor<T>,
			delegator: Delegator<T::AccountId>,
			task: DappStaking<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			Self::ensure_governance_or_operator(origin, StakingProtocol::AstarDappStaking)?;
			Self::do_dapp_staking(delegator, task)
		}

		/// Processing Xcm message execution results.
		///
		/// Can be called by governance or xcm origin.
		#[cfg(feature = "polkadot")]
		#[pallet::call_index(11)]
		#[pallet::weight(<T as Config>::WeightInfo::notify_astar_dapp_staking())]
		pub fn notify_astar_dapp_staking(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResultWithPostInfo {
			let responder = Self::ensure_governance_or_xcm_response(origin)?;
			let pending_status = PendingStatusByQueryId::<T>::get(query_id)
				.ok_or(Error::<T>::PendingStatusNotFound)?;
			if Response::DispatchResult(MaybeErrorCode::Success) == response {
				Self::do_notify_astar_dapp_staking(responder, pending_status)?;
			} else {
				PendingStatusByQueryId::<T>::remove(query_id);
			}
			Ok(().into())
		}
	}
}

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

#![cfg_attr(not(feature = "std"), no_std)]

pub use agents::KusamaAgent;
use cumulus_primitives_core::ParaId;
use frame_support::{dispatch::result::Result, pallet_prelude::*, transactional, weights::Weight};
use frame_system::{
	ensure_signed,
	pallet_prelude::{BlockNumberFor, OriginFor},
};
use node_primitives::{CurrencyId, CurrencyIdExt, TimeUnit, VtokenMintingOperator};
use orml_traits::MultiCurrency;
pub use primitives::Ledger;
use sha3::{Digest, Keccak256};
use sp_arithmetic::traits::Zero;
use sp_core::H256;
use sp_runtime::traits::{CheckedSub, Convert};
use sp_std::{boxed::Box, vec, vec::Vec};
pub use weights::WeightInfo;
use xcm::{
	latest::*,
	opaque::latest::{Junction::Parachain, Junctions::X2, NetworkId::Any},
};

pub use crate::{
	primitives::{
		LedgerUpdateEntry, MinimumsMaximums, SubstrateLedger, ValidatorsByDelegatorUpdateEntry,
		XcmOperation, KSM,
	},
	traits::{
		DelegatorManager, QueryResponseChecker, QueryResponseManager, StakingAgent,
		StakingFeeManager, ValidatorManager,
	},
	Junction::AccountId32,
	Junctions::X1,
};

mod agents;
mod mock;
pub mod primitives;
mod tests;
pub mod traits;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pallet::*;

type XcmQueryId = [u8; 32];
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;
type StakingAgentBoxType<T> =
	Box<dyn StakingAgent<MultiLocation, MultiLocation, BalanceOf<T>, TimeUnit, AccountIdOf<T>>>;
type DelegatorManagerBoxType<T> =
	Box<dyn DelegatorManager<MultiLocation, SubstrateLedger<MultiLocation, BalanceOf<T>>>>;
type ValidatorManagerBoxType = Box<dyn ValidatorManager<MultiLocation>>;
type StakingFeeManagerBoxType<T> = Box<dyn StakingFeeManager<MultiLocation, BalanceOf<T>>>;
type QueryResponseCheckerBoxType<T> = Box<
	dyn QueryResponseChecker<
		XcmQueryId,
		LedgerUpdateEntry<BalanceOf<T>, MultiLocation>,
		ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation>,
	>,
>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Currency operations handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
		/// The only origin that can modify pallet params
		type ControlOrigin: EnsureOrigin<Self::Origin>;

		/// Set default weight.
		type WeightInfo: WeightInfo;

		/// The interface to call VtokenMinting module functions.
		type VtokenMinting: VtokenMintingOperator<
			CurrencyId,
			BalanceOf<Self>,
			AccountIdOf<Self>,
			TimeUnit,
		>;

		/// Substrate account converter, which can convert a u16 number into a sub-account with
		/// MultiLocation format.
		type AccountConverter: Convert<u16, MultiLocation>;

		/// Parachain Id which is gotten from the runtime.
		type ParachainId: Get<ParaId>;

		/// Routes the XCM message outbound.
		type XcmSender: SendXcm;

		/// XCM executor.
		type XcmExecutor: ExecuteXcm<Self::Call>;

		/// Substrate response manager.
		type SubstrateResponseManager: QueryResponseManager<
			XcmQueryId,
			MultiLocation,
			BlockNumberFor<Self>,
		>;

		/// The maximum number of entries to be confirmed in a block for each update queue in the
		/// on_initialize queue.
		#[pallet::constant]
		type MaxTypeEntryPerBlock: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		OperateOriginNotSet,
		NotAuthorized,
		NotSupportedCurrencyId,
		FailToInitializeDelegator,
		FailToBond,
		OverFlow,
		UnderFlow,
		NotExist,
		LowerThanMinimum,
		GreaterThanMaximum,
		AlreadyBonded,
		AccountNotExist,
		DelegatorNotExist,
		XcmFailure,
		DelegatorNotBonded,
		ExceedActiveMaximum,
		ProblematicLedger,
		NotEnoughToUnbond,
		ExceedUnlockingRecords,
		RebondExceedUnlockingAmount,
		DecodingError,
		EncodingError,
		VectorEmpty,
		ValidatorSetNotExist,
		ValidatorNotExist,
		InvalidTimeUnit,
		AmountZero,
		AmountNotZero,
		AlreadyExist,
		ValidatorStillInUse,
		TimeUnitNotExist,
		FeeSourceNotExist,
		BalanceLow,
		WeightAndFeeNotExists,
		OperateOriginNotExists,
		MinimumsAndMaximumsNotExist,
		XcmExecutionFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		DelegatorInitialized {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
		},
		DelegatorBonded {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			bonded_amount: BalanceOf<T>,
		},
		DelegatorBondExtra {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			extra_bonded_amount: BalanceOf<T>,
		},
		DelegatorUnbond {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			unbond_amount: BalanceOf<T>,
		},
		DelegatorUnbondAll {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
		},
		DelegatorRebond {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			rebond_amount: BalanceOf<T>,
		},
		Delegated {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			targets: Vec<MultiLocation>,
		},
		Undelegated {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			targets: Vec<MultiLocation>,
		},
		Payout {
			currency_id: CurrencyId,
			validator: MultiLocation,
			time_unit: Option<TimeUnit>,
		},
		Liquidize {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			time_unit: Option<TimeUnit>,
		},
		Chill {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
		},
		TransferBack {
			currency_id: CurrencyId,
			from: MultiLocation,
			to: AccountIdOf<T>,
			amount: BalanceOf<T>,
		},
		TransferTo {
			currency_id: CurrencyId,
			from: AccountIdOf<T>,
			to: MultiLocation,
			amount: BalanceOf<T>,
		},
		DelegatorAdded {
			currency_id: CurrencyId,
			index: u16,
			delegator_id: MultiLocation,
		},
		DelegatorRemoved {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
		},
		ValidatorsAdded {
			currency_id: CurrencyId,
			validator_id: MultiLocation,
		},
		ValidatorsRemoved {
			currency_id: CurrencyId,
			validator_id: MultiLocation,
		},
		Refund {
			currency_id: CurrencyId,
			time_unit: TimeUnit,
			index: u32,
			amount: BalanceOf<T>,
		},
		FundMoveFromExitToEntrance {
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
		},
		TimeUnitUpdated {
			currency_id: CurrencyId,
			old: TimeUnit,
			new: TimeUnit,
		},
		PoolTokenIncreased {
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
		},
		PoolTokenDecreased {
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
		},
		FeeSupplemented {
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
			from: MultiLocation,
			to: MultiLocation,
		},
		TokenToAddIncreased {
			currency_id: CurrencyId,
			value: BalanceOf<T>,
		},
		TokenToAddDecreased {
			currency_id: CurrencyId,
			value: BalanceOf<T>,
		},
		TokenToDeductIncreased {
			currency_id: CurrencyId,
			value: BalanceOf<T>,
		},
		TokenToDeductDecreased {
			currency_id: CurrencyId,
			value: BalanceOf<T>,
		},
		ValidatorsByDelegatorSet {
			currency_id: CurrencyId,
			validators_list: Vec<(MultiLocation, H256)>,
		},
		XcmDestWeightAndFeeSet {
			currency_id: CurrencyId,
			operation: XcmOperation,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		},
		OperateOriginSet {
			currency_id: CurrencyId,
			operator: Option<AccountIdOf<T>>,
		},
		FeeSourceSet {
			currency_id: CurrencyId,
			who_and_fee: Option<(MultiLocation, BalanceOf<T>)>,
		},
		DelegatorLedgerSet {
			currency_id: CurrencyId,
			delegator: MultiLocation,
			ledger: Option<Ledger<MultiLocation, BalanceOf<T>>>,
		},
	}

	/// The dest weight limit and fee for execution XCM msg sended out. Must be
	/// sufficient, otherwise the execution of XCM msg on the dest chain will fail.
	///
	/// XcmDestWeightAndFee: DoubleMap: CurrencyId, XcmOperation => (Weight, Balance)
	#[pallet::storage]
	#[pallet::getter(fn xcm_dest_weight_and_fee)]
	pub type XcmDestWeightAndFee<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		XcmOperation,
		(Weight, BalanceOf<T>),
		OptionQuery,
	>;

	/// One operate origin(can be a multisig account) for a currency. An operating origins are
	/// normal account in Bifrost chain.
	#[pallet::storage]
	#[pallet::getter(fn get_operate_origin)]
	pub type OperateOrigins<T> = StorageMap<_, Blake2_128Concat, CurrencyId, AccountIdOf<T>>;

	/// Origins and Amounts for the staking operating account fee supplement. An operating account
	/// is identified in MultiLocation format.
	#[pallet::storage]
	#[pallet::getter(fn get_fee_source)]
	pub type FeeSources<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (MultiLocation, BalanceOf<T>)>;

	/// Delegators in service. A delegator is identified in MultiLocation format.
	/// Currency Id + Sub-account index => MultiLocation
	#[pallet::storage]
	#[pallet::getter(fn get_delegator_multilocation_by_index)]
	pub type DelegatorsIndex2Multilocation<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		u16,
		MultiLocation,
		OptionQuery,
	>;

	/// Delegators in service. Currency Id + MultiLocation => Sub-account index
	#[pallet::storage]
	#[pallet::getter(fn get_delegator_index_by_multilocation)]
	pub type DelegatorsMultilocation2Index<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		u16,
		OptionQuery,
	>;

	/// Next index of different currency delegators.
	#[pallet::storage]
	#[pallet::getter(fn get_delegator_next_index)]
	pub type DelegatorNextIndex<T> = StorageMap<_, Blake2_128Concat, CurrencyId, u16, ValueQuery>;

	/// Validator in service. A validator is identified in MultiLocation format.
	#[pallet::storage]
	#[pallet::getter(fn get_validators)]
	pub type Validators<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, Vec<(MultiLocation, H256)>>;

	/// Validators for each delegator. CurrencyId + Delegator => Vec<Validator>
	#[pallet::storage]
	#[pallet::getter(fn get_validators_by_delegator)]
	pub type ValidatorsByDelegator<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		Vec<(MultiLocation, H256)>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_validators_by_delegator_update_entry)]
	pub type ValidatorsByDelegatorXcmUpdateQueue<T> = StorageMap<
		_,
		Blake2_128Concat,
		XcmQueryId,
		ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation>,
	>;

	/// Delegator ledgers. A delegator is identified in MultiLocation format.
	#[pallet::storage]
	#[pallet::getter(fn get_delegator_ledger)]
	pub type DelegatorLedgers<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		Ledger<MultiLocation, BalanceOf<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_delegator_ledger_update_entry)]
	pub type DelegatorLedgerXcmUpdateQueue<T> =
		StorageMap<_, Blake2_128Concat, XcmQueryId, LedgerUpdateEntry<BalanceOf<T>, MultiLocation>>;

	/// Minimum and Maximum constraints for different chains.
	#[pallet::storage]
	#[pallet::getter(fn get_minimums_maximums)]
	pub type MinimumsAndMaximums<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, MinimumsMaximums<BalanceOf<T>>>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			// For queries in update entry queues, search responses in pallet_xcm Queries storage.
			let _ = Self::process_query_entry_records();

			1_000_000_000
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// *****************************/
		/// ****** Outer Calls ******/
		/// *****************************/
		///
		/// Delegator initialization work. Generate a new delegator and return its ID.
		#[pallet::weight(T::WeightInfo::initialize_delegator())]
		pub fn initialize_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let delegator_id = staking_agent
				.initialize_delegator()
				.ok_or(Error::<T>::FailToInitializeDelegator)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorInitialized { currency_id, delegator_id });
			Ok(())
		}

		/// First time bonding some amount to a delegator.
		#[pallet::weight(T::WeightInfo::bond())]
		pub fn bond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.bond(who.clone(), amount)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorBonded {
				currency_id,
				delegator_id: who,
				bonded_amount: amount,
			});
			Ok(())
		}

		/// Bond extra amount to a delegator.
		#[pallet::weight(T::WeightInfo::bond_extra())]
		pub fn bond_extra(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.bond_extra(who.clone(), amount)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorBondExtra {
				currency_id,
				delegator_id: who,
				extra_bonded_amount: amount,
			});
			Ok(())
		}

		/// Decrease some amount to a delegator. Leave no less than the minimum delegator
		/// requirement.
		#[pallet::weight(T::WeightInfo::unbond())]
		pub fn unbond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.unbond(who.clone(), amount)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorUnbond {
				currency_id,
				delegator_id: who,
				unbond_amount: amount,
			});
			Ok(())
		}

		/// Unbond all the active amount of a delegator.
		#[pallet::weight(T::WeightInfo::unbond_all())]
		pub fn unbond_all(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.unbond_all(who.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorUnbondAll {
				currency_id,
				delegator_id: who,
			});
			Ok(())
		}

		/// Rebond some unlocking amount to a delegator.
		#[pallet::weight(T::WeightInfo::rebond())]
		pub fn rebond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.rebond(who.clone(), amount)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorRebond {
				currency_id,
				delegator_id: who,
				rebond_amount: amount,
			});
			Ok(())
		}

		/// Delegate to some validator set.
		#[pallet::weight(T::WeightInfo::delegate())]
		pub fn delegate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			targets: Vec<MultiLocation>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.delegate(who.clone(), targets.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Delegated {
				currency_id,
				delegator_id: who,
				targets,
			});
			Ok(())
		}

		/// Re-delegate existing delegation to a new validator set.
		#[pallet::weight(T::WeightInfo::undelegate())]
		pub fn undelegate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			targets: Vec<MultiLocation>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.undelegate(who.clone(), targets.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Undelegated {
				currency_id,
				delegator_id: who,
				targets,
			});
			Ok(())
		}

		/// Re-delegate existing delegation to a new validator set.
		#[pallet::weight(T::WeightInfo::redelegate())]
		pub fn redelegate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			targets: Vec<MultiLocation>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.redelegate(who.clone(), targets.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Delegated {
				currency_id,
				delegator_id: who,
				targets,
			});
			Ok(())
		}

		/// Initiate payout for a certain delegator.
		#[pallet::weight(T::WeightInfo::payout())]
		pub fn payout(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			validator: MultiLocation,
			when: Option<TimeUnit>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.payout(who, validator.clone(), when.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Payout { currency_id, validator, time_unit: when });
			Ok(())
		}

		/// Withdraw the due payout into free balance.
		#[pallet::weight(T::WeightInfo::liquidize())]
		pub fn liquidize(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			when: Option<TimeUnit>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.liquidize(who.clone(), when.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Liquidize {
				currency_id,
				delegator_id: who,
				time_unit: when,
			});
			Ok(())
		}

		/// Initiate payout for a certain delegator.
		#[pallet::weight(T::WeightInfo::chill())]
		pub fn chill(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.chill(who.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Chill { currency_id, delegator_id: who });
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::transfer_back())]
		pub fn transfer_back(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			from: MultiLocation,
			to: AccountIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.transfer_back(from.clone(), to.clone(), amount)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TransferBack { currency_id, from, to, amount });

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::transfer_to())]
		pub fn transfer_to(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			from: AccountIdOf<T>,
			to: MultiLocation,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.transfer_to(from.clone(), to.clone(), amount)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TransferTo { currency_id, from, to, amount });

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::increase_token_pool())]
		pub fn increase_token_pool(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			// Ensure the amount is valid.
			ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

			T::VtokenMinting::increase_token_pool(currency_id, amount)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::PoolTokenIncreased { currency_id, amount });
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::decrease_token_pool())]
		pub fn decrease_token_pool(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			// Ensure the amount is valid.
			ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

			T::VtokenMinting::decrease_token_pool(currency_id, amount)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::PoolTokenDecreased { currency_id, amount });
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::update_ongoing_time_unit())]
		pub fn update_ongoing_time_unit(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			time_unit: TimeUnit,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let old = T::VtokenMinting::get_ongoing_time_unit(currency_id).unwrap_or_default();
			T::VtokenMinting::update_ongoing_time_unit(currency_id, time_unit.clone())?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TimeUnitUpdated { currency_id, old, new: time_unit });

			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::refund_currency_due_unbond())]
		pub fn refund_currency_due_unbond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			// Get entrance_account and exit_account, as well as their currency balances.
			let (entrance_account, exit_account) =
				T::VtokenMinting::get_entrance_and_exit_accounts();
			let mut exit_account_balance =
				T::MultiCurrency::free_balance(currency_id, &exit_account);
			let ed = T::MultiCurrency::minimum_balance(currency_id);

			// Get the currency due unlocking records
			let time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
				.ok_or(Error::<T>::TimeUnitNotExist)?;
			let rs = T::VtokenMinting::get_unlock_records(currency_id, time_unit.clone());

			// Refund due unlocking records one by one.
			if let Some((_locked_amount, idx_vec)) = rs {
				for idx in idx_vec.iter() {
					let checked_remain =
						exit_account_balance.checked_sub(&ed).ok_or(Error::<T>::UnderFlow)?;

					// get idx record amount
					let idx_record_amount_op =
						T::VtokenMinting::get_token_unlock_ledger(currency_id, *idx);

					if let Some((user_account, idx_record_amount, _unlock_era)) =
						idx_record_amount_op
					{
						let mut deduct_amount = idx_record_amount;
						if checked_remain < idx_record_amount {
							deduct_amount = checked_remain;
						}
						// Transfer some amount from the exit_account to the user's account
						T::MultiCurrency::transfer(
							KSM,
							&exit_account,
							&user_account,
							deduct_amount,
						)?;
						// Delete the corresponding unlocking record storage.
						T::VtokenMinting::deduct_unlock_amount(currency_id, *idx, deduct_amount)?;

						// Deposit event.
						Pallet::<T>::deposit_event(Event::Refund {
							currency_id,
							time_unit: time_unit.clone(),
							index: *idx,
							amount: deduct_amount,
						});

						exit_account_balance = exit_account_balance
							.checked_sub(&deduct_amount)
							.ok_or(Error::<T>::UnderFlow)?;
						if exit_account_balance <= ed {
							break;
						}
					}
				}
			}

			// Automatically move the rest amount in exit account to entrance account.
			let new_exit_account_balance =
				T::MultiCurrency::free_balance(currency_id, &exit_account);

			T::MultiCurrency::transfer(
				currency_id,
				&exit_account,
				&entrance_account,
				new_exit_account_balance,
			)?;

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::increase_token_to_add())]
		/// Increase token_to_add storage by value in VtokenMinting module.
		pub fn increase_token_to_add(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			value: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			// Ensure the value is valid.
			ensure!(value > Zero::zero(), Error::<T>::AmountZero);

			T::VtokenMinting::increase_token_to_add(currency_id, value)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TokenToAddIncreased { currency_id, value });
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::decrease_token_to_add())]
		/// Decrease token_to_add storage by value in VtokenMinting module.
		pub fn decrease_token_to_add(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			value: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			// Ensure the value is valid.
			ensure!(value > Zero::zero(), Error::<T>::AmountZero);

			T::VtokenMinting::decrease_token_to_add(currency_id, value)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TokenToAddDecreased { currency_id, value });
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::increase_token_to_deduct())]
		/// Increase token_to_deduct storage by value in VtokenMinting module.
		pub fn increase_token_to_deduct(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			value: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			// Ensure the value is valid.
			ensure!(value > Zero::zero(), Error::<T>::AmountZero);

			T::VtokenMinting::increase_token_to_deduct(currency_id, value)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TokenToDeductIncreased { currency_id, value });
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::decrease_token_to_deduct())]
		/// Decrease token_to_deduct storage by value in VtokenMinting module.
		pub fn decrease_token_to_deduct(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			value: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			// Ensure the value is valid.
			ensure!(value > Zero::zero(), Error::<T>::AmountZero);

			T::VtokenMinting::decrease_token_to_deduct(currency_id, value)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TokenToDeductDecreased { currency_id, value });
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::supplement_fee_reserve())]
		pub fn supplement_fee_reserve(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			dest: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			// Get the  fee source account and reserve amount from the FeeSources<T> storage.
			let (source_location, reserved_fee) =
				FeeSources::<T>::get(currency_id).ok_or(Error::<T>::FeeSourceNotExist)?;

			// If currency is BNC, transfer directly.
			// Otherwise, call supplement_fee_reserve of StakingFeeManager trait.
			if currency_id.is_native() {
				let source_account = Self::native_multilocation_to_account(&source_location)?;
				let dest_account = Self::native_multilocation_to_account(&dest)?;
				T::MultiCurrency::transfer(
					currency_id,
					&source_account,
					&dest_account,
					reserved_fee,
				)?;
			} else {
				let fee_manager_agent = Self::get_currency_staking_fee_manager(currency_id)?;
				fee_manager_agent.supplement_fee_reserve(
					reserved_fee,
					source_location.clone(),
					dest.clone(),
				)?;
			}

			// Deposit event.
			Pallet::<T>::deposit_event(Event::FeeSupplemented {
				currency_id,
				amount: reserved_fee,
				from: source_location,
				to: dest,
			});

			Ok(())
		}

		/// *****************************/
		/// ****** Storage Setters ******/
		/// *****************************/
		///
		/// Update storage XcmDestWeightAndFee<T>.
		#[pallet::weight(T::WeightInfo::set_xcm_dest_weight_and_fee())]
		pub fn set_xcm_dest_weight_and_fee(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			operation: XcmOperation,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			// If param weight_and_fee is a none, it will delete the storage. Otherwise, revise the
			// storage to the new value if exists, or insert a new record if not exists before.
			XcmDestWeightAndFee::<T>::mutate_exists(currency_id, operation.clone(), |wt_n_f| {
				*wt_n_f = weight_and_fee.clone();
			});

			// Deposit event.
			Pallet::<T>::deposit_event(Event::XcmDestWeightAndFeeSet {
				currency_id,
				operation,
				weight_and_fee,
			});

			Ok(())
		}

		/// Update storage OperateOrigins<T>.
		#[pallet::weight(T::WeightInfo::set_operate_origin())]
		pub fn set_operate_origin(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Option<AccountIdOf<T>>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			OperateOrigins::<T>::mutate_exists(currency_id, |operator| {
				*operator = who.clone();
			});

			// Deposit event.
			Pallet::<T>::deposit_event(Event::OperateOriginSet { currency_id, operator: who });

			Ok(())
		}

		/// Update storage FeeSources<T>.
		#[pallet::weight(T::WeightInfo::set_fee_source())]
		pub fn set_fee_source(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who_and_fee: Option<(MultiLocation, BalanceOf<T>)>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			FeeSources::<T>::mutate_exists(currency_id, |w_n_f| {
				*w_n_f = who_and_fee.clone();
			});

			// Deposit event.
			Pallet::<T>::deposit_event(Event::FeeSourceSet { currency_id, who_and_fee });

			Ok(())
		}

		/// Update storage DelegatorsIndex2Multilocation<T> 和 DelegatorsMultilocation2Index<T>.
		#[pallet::weight(T::WeightInfo::add_delegator())]
		pub fn add_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			index: u16,
			who: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let delegator_manager = Self::get_currency_delegator_manager(currency_id)?;
			delegator_manager.add_delegator(index, &who)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorAdded {
				currency_id,
				index,
				delegator_id: who,
			});
			Ok(())
		}

		/// Update storage DelegatorsIndex2Multilocation<T> 和 DelegatorsMultilocation2Index<T>.
		#[pallet::weight(T::WeightInfo::remove_delegator())]
		pub fn remove_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let delegator_manager = Self::get_currency_delegator_manager(currency_id)?;
			delegator_manager.remove_delegator(&who)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorRemoved { currency_id, delegator_id: who });
			Ok(())
		}

		/// Update storage Validators<T>.
		#[pallet::weight(T::WeightInfo::add_validator())]
		pub fn add_validator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let validator_manager = Self::get_currency_validator_manager(currency_id)?;
			validator_manager.add_validator(&who)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsAdded { currency_id, validator_id: who });
			Ok(())
		}

		/// Update storage Validators<T>.
		#[pallet::weight(T::WeightInfo::remove_validator())]
		pub fn remove_validator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			let authorized = Self::ensure_authorized(origin, currency_id);
			ensure!(authorized, Error::<T>::NotAuthorized);

			let validator_manager = Self::get_currency_validator_manager(currency_id)?;
			validator_manager.remove_validator(&who)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsRemoved { currency_id, validator_id: who });
			Ok(())
		}

		/// Update storage ValidatorsByDelegator<T>.
		#[pallet::weight(T::WeightInfo::set_validators_by_delegator())]
		pub fn set_validators_by_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			validators: Vec<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			// Check the length of validators
			let minimums_and_maximums = MinimumsAndMaximums::<T>::get(currency_id)
				.ok_or(Error::<T>::MinimumsAndMaximumsNotExist)?;
			ensure!(
				validators.len() as u32 <= minimums_and_maximums.validators_back_maximum,
				Error::<T>::GreaterThanMaximum
			);

			// check delegator
			// Check if it is bonded already.
			let _ledger = DelegatorLedgers::<T>::get(KSM, who.clone())
				.ok_or(Error::<T>::DelegatorNotBonded)?;

			let validators_list =
				Self::sort_validators_and_remove_duplicates(currency_id, &validators)?;

			// Update ValidatorsByDelegator storage
			ValidatorsByDelegator::<T>::insert(currency_id, who.clone(), validators_list.clone());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsByDelegatorSet {
				currency_id,
				validators_list,
			});

			Ok(())
		}

		/// Update storage DelegatorLedgers<T>.
		#[pallet::weight(T::WeightInfo::set_delegator_ledger())]
		pub fn set_delegator_ledger(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			ledger: Option<Ledger<MultiLocation, BalanceOf<T>>>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
			// Check the new ledger must has at lease minimum active amount.
			if let Some(ref ldgr) = ledger {
				if let Ledger::Substrate(lg) = ldgr {
					ensure!(
						lg.active >= mins_maxs.delegator_bonded_minimum,
						Error::<T>::LowerThanMinimum
					);
				}
			}

			// Update the ledger.
			DelegatorLedgers::<T>::mutate_exists(currency_id, who.clone(), |old_ledger| {
				*old_ledger = ledger.clone();
			});

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorLedgerSet {
				currency_id,
				delegator: who,
				ledger,
			});

			Ok(())
		}

		/// Update storage MinimumsAndMaximums<T>.
		#[pallet::weight(T::WeightInfo::set_minimums_and_maximums())]
		pub fn set_minimums_and_maximums(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			constraints: Option<MinimumsMaximums<BalanceOf<T>>>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			MinimumsAndMaximums::<T>::mutate_exists(currency_id, |minimums_maximums| {
				*minimums_maximums = constraints;
			});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Ensure privileged origin
		fn ensure_authorized(origin: OriginFor<T>, currency_id: CurrencyId) -> bool {
			let operator = ensure_signed(origin.clone()).ok();
			let privileged = OperateOrigins::<T>::get(currency_id);

			// It is from the privileged group.
			let cond0 = operator.is_some();
			let cond1 = operator == privileged;

			// It is from ControlOrigin.
			let cond2 = T::ControlOrigin::ensure_origin(origin).is_ok();

			(cond0 & cond1) || cond2
		}

		/// Convert native multiLocation to account.
		fn native_multilocation_to_account(
			who: &MultiLocation,
		) -> Result<AccountIdOf<T>, Error<T>> {
			// Get the delegator account id in Kusama network
			let account_32 = match who {
				MultiLocation {
					parents: 0,
					interior: X1(AccountId32 { network: _network_id, id: account_id }),
				} => account_id,
				_ => Err(Error::<T>::AccountNotExist)?,
			};

			let account = T::AccountId::decode(&mut &account_32[..])
				.map_err(|_| Error::<T>::DecodingError)?;

			Ok(account)
		}

		fn get_currency_staking_agent(
			currency_id: CurrencyId,
		) -> Result<StakingAgentBoxType<T>, Error<T>> {
			match currency_id {
				KSM => Ok(Box::new(KusamaAgent::<
					T,
					T::AccountConverter,
					T::ParachainId,
					T::XcmSender,
					T::SubstrateResponseManager,
				>::new())),
				_ => Err(Error::<T>::NotSupportedCurrencyId),
			}
		}

		fn get_currency_delegator_manager(
			currency_id: CurrencyId,
		) -> Result<DelegatorManagerBoxType<T>, Error<T>> {
			match currency_id {
				KSM => Ok(Box::new(KusamaAgent::<
					T,
					T::AccountConverter,
					T::ParachainId,
					T::XcmSender,
					T::SubstrateResponseManager,
				>::new())),
				_ => Err(Error::<T>::NotSupportedCurrencyId),
			}
		}

		fn get_currency_validator_manager(
			currency_id: CurrencyId,
		) -> Result<ValidatorManagerBoxType, Error<T>> {
			match currency_id {
				KSM => Ok(Box::new(KusamaAgent::<
					T,
					T::AccountConverter,
					T::ParachainId,
					T::XcmSender,
					T::SubstrateResponseManager,
				>::new())),
				_ => Err(Error::<T>::NotSupportedCurrencyId),
			}
		}

		fn get_currency_staking_fee_manager(
			currency_id: CurrencyId,
		) -> Result<StakingFeeManagerBoxType<T>, Error<T>> {
			match currency_id {
				KSM => Ok(Box::new(KusamaAgent::<
					T,
					T::AccountConverter,
					T::ParachainId,
					T::XcmSender,
					T::SubstrateResponseManager,
				>::new())),
				_ => Err(Error::<T>::NotSupportedCurrencyId),
			}
		}

		fn get_currency_query_response_checker(
			currency_id: CurrencyId,
		) -> Result<QueryResponseCheckerBoxType<T>, Error<T>> {
			match currency_id {
				KSM => Ok(Box::new(KusamaAgent::<
					T,
					T::AccountConverter,
					T::ParachainId,
					T::XcmSender,
					T::SubstrateResponseManager,
				>::new())),
				_ => Err(Error::<T>::NotSupportedCurrencyId),
			}
		}

		pub fn sort_validators_and_remove_duplicates(
			currency_id: CurrencyId,
			validators: &Vec<MultiLocation>,
		) -> Result<Vec<(MultiLocation, H256)>, Error<T>> {
			let validators_set =
				Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;
			let mut validators_list: Vec<(MultiLocation, H256)> = vec![];
			for validator in validators.iter() {
				// Check if the validator is in the validator whitelist
				let multi_hash = Self::get_hash(&validator);
				ensure!(
					validators_set.contains(&(validator.clone(), multi_hash)),
					Error::<T>::ValidatorNotExist
				);

				// sort the validators and remove duplicates
				let rs = validators_list.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash);

				if let Err(index) = rs {
					validators_list.insert(index, (validator.clone(), multi_hash));
				}
			}

			Ok(validators_list)
		}

		pub fn get_hash(who: &MultiLocation) -> H256 {
			let encoded = who.encode();
			H256::from_slice(Keccak256::digest(&encoded).as_slice())
		}

		pub fn multilocation_to_account(who: &MultiLocation) -> Result<AccountIdOf<T>, Error<T>> {
			// Get the delegator account id in Kusama network
			let account_32 = Self::multilocation_to_account_32(who)?;
			let account = T::AccountId::decode(&mut &account_32[..])
				.map_err(|_| Error::<T>::DecodingError)?;
			Ok(account)
		}

		pub fn multilocation_to_account_32(who: &MultiLocation) -> Result<[u8; 32], Error<T>> {
			// Get the delegator account id in Kusama network
			let account_32 = match who {
				MultiLocation {
					parents: _,
					interior: X1(AccountId32 { network: _network_id, id: account_id }),
				} => account_id,
				_ => Err(Error::<T>::AccountNotExist)?,
			};
			Ok(*account_32)
		}

		pub fn account_id_to_account_32(account_id: AccountIdOf<T>) -> Result<[u8; 32], Error<T>> {
			let account_32 = T::AccountId::encode(&account_id)
				.try_into()
				.map_err(|_| Error::<T>::EncodingError)?;

			Ok(account_32)
		}

		pub fn account_32_to_local_location(
			account_32: [u8; 32],
		) -> Result<MultiLocation, Error<T>> {
			let local_location = MultiLocation {
				parents: 0,
				interior: X1(AccountId32 { network: Any, id: account_32 }),
			};

			Ok(local_location)
		}

		pub fn account_32_to_parent_location(
			account_32: [u8; 32],
		) -> Result<MultiLocation, Error<T>> {
			let parent_location = MultiLocation {
				parents: 1,
				interior: X1(AccountId32 { network: Any, id: account_32 }),
			};

			Ok(parent_location)
		}

		pub fn account_32_to_parachain_location(
			account_32: [u8; 32],
			chain_id: u32,
		) -> Result<MultiLocation, Error<T>> {
			let parachain_location = MultiLocation {
				parents: 1,
				interior: X2(Parachain(chain_id), AccountId32 { network: Any, id: account_32 }),
			};

			Ok(parachain_location)
		}

		/// **************************************/
		/// ****** XCM confirming Functions ******/
		/// **************************************/
		pub fn process_query_entry_records() -> DispatchResult {
			let mut counter = 0u32;

			// Deal with DelegatorLedgerXcmUpdateQueue storage
			for (query_id, query_entry) in DelegatorLedgerXcmUpdateQueue::<T>::iter() {
				ensure!(counter <= T::MaxTypeEntryPerBlock::get(), Error::<T>::GreaterThanMaximum);

				let query_response_agent = match query_entry.clone() {
					LedgerUpdateEntry::Substrate(entry) =>
						Self::get_currency_query_response_checker(entry.currency_id),
					_ => Err(Error::<T>::NotSupportedCurrencyId),
				}?;

				query_response_agent
					.check_delegator_ledger_query_response(query_id, query_entry)?;
				counter = counter.saturating_add(1);
			}

			// Deal with ValidatorsByDelegator storage
			for (query_id, query_entry) in ValidatorsByDelegatorXcmUpdateQueue::<T>::iter() {
				ensure!(counter <= T::MaxTypeEntryPerBlock::get(), Error::<T>::GreaterThanMaximum);

				let query_response_agent = match query_entry.clone() {
					ValidatorsByDelegatorUpdateEntry::Substrate(entry) =>
						Self::get_currency_query_response_checker(entry.currency_id),
					_ => Err(Error::<T>::NotSupportedCurrencyId),
				}?;

				query_response_agent
					.check_validators_by_delegator_query_response(query_id, query_entry)?;
				counter = counter.saturating_add(1);
			}

			Ok(())
		}

		// pub fn confirm_delegator_ledger_update_entry(
		// 	query_id: XcmQueryId,
		// ) -> Result<MultiLocation, Error<T>> {
		// 	// Update corresponding storage if exist.
		// }
	}
}

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
use cumulus_primitives_core::{relay_chain::HashT, ParaId};
use frame_support::{pallet_prelude::*, transactional, weights::Weight};
use frame_system::{
	pallet_prelude::{BlockNumberFor, OriginFor},
	RawOrigin,
};
use node_primitives::{CurrencyId, CurrencyIdExt, TimeUnit, VtokenMintingOperator};
use orml_traits::MultiCurrency;
pub use primitives::Ledger;
use sp_arithmetic::{per_things::Permill, traits::Zero};
use sp_runtime::traits::{CheckedSub, Convert};
use sp_std::{boxed::Box, vec, vec::Vec};
pub use weights::WeightInfo;
use xcm::{
	latest::{ExecuteXcm, Junction, Junctions, MultiLocation, SendXcm, Xcm},
	opaque::latest::{
		Junction::{AccountKey20, Parachain},
		Junctions::X2,
		NetworkId::Any,
	},
};

use crate::agents::MoonriverAgent;
pub use crate::{
	primitives::{
		Delays, LedgerUpdateEntry, MinimumsMaximums, SubstrateLedger,
		ValidatorsByDelegatorUpdateEntry, XcmOperation, KSM, MOVR,
	},
	traits::{QueryResponseManager, StakingAgent},
	Junction::AccountId32,
	Junctions::X1,
};

use sp_core::H160;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::TrailingZeroInput;

mod agents;
mod mock;
pub mod primitives;
mod tests;
pub mod traits;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pallet::*;

pub type Result<T, E> = core::result::Result<T, E>;

pub type QueryId = u64;
pub const TIMEOUT_BLOCKS: u32 = 1000;
pub const BASE_WEIGHT: Weight = 1000;
type Hash<T> = <T as frame_system::Config>::Hash;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;
type StakingAgentBoxType<T> = Box<
	dyn StakingAgent<
		MultiLocation,
		MultiLocation,
		BalanceOf<T>,
		TimeUnit,
		AccountIdOf<T>,
		MultiLocation,
		QueryId,
		LedgerUpdateEntry<BalanceOf<T>, MultiLocation, MultiLocation>,
		ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation, Hash<T>>,
		pallet::Error<T>,
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
		type ControlOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

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
		type AccountConverter: Convert<(u16, CurrencyId), MultiLocation>;

		/// Parachain Id which is gotten from the runtime.
		type ParachainId: Get<ParaId>;

		/// Routes the XCM message outbound.
		type XcmRouter: SendXcm;

		/// XCM executor.
		type XcmExecutor: ExecuteXcm<Self::Call>;

		/// Substrate response manager.
		type SubstrateResponseManager: QueryResponseManager<
			QueryId,
			MultiLocation,
			BlockNumberFor<Self>,
		>;

		//【For xcm v3】
		// /// This chain's Universal Location. Enabled only for xcm v3 version.
		// type UniversalLocation: Get<InteriorMultiLocation>;

		/// The maximum number of entries to be confirmed in a block for update queue in the
		/// on_initialize queue.
		#[pallet::constant]
		type MaxTypeEntryPerBlock: Get<u32>;

		#[pallet::constant]
		type MaxRefundPerBlock: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		OperateOriginNotSet,
		NotAuthorized,
		NotSupportedCurrencyId,
		FailToAddDelegator,
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
		QueryNotExist,
		DelaysNotExist,
		Unexpected,
		UnlockingRecordNotExist,
		QueryResponseRemoveError,
		ValidatorsByDelegatorResponseCheckError,
		LedgerResponseCheckError,
		InvalidHostingFee,
		InvalidAccount,
		IncreaseTokenPoolError,
		TuneExchangeRateLimitNotSet,
		DelegatorLatestTuneRecordNotExist,
		InvalidTransferSource,
		ValidatorNotProvided,
		Unsupported,
		ValidatorNotBonded,
		AlreadyRequested,
		RequestNotExist,
		AlreadyLeaving,
		DelegatorNotLeaving,
		RequestNotDue,
		LeavingNotDue,
		DelegatorSetNotExist,
		DelegatorLeaving,
		DelegatorAlreadyLeaving,
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
			#[codec(compact)]
			bonded_amount: BalanceOf<T>,
			#[codec(compact)]
			query_id: QueryId,
			query_id_hash: Hash<T>,
			validator: Option<MultiLocation>,
		},
		DelegatorBondExtra {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			#[codec(compact)]
			extra_bonded_amount: BalanceOf<T>,
			#[codec(compact)]
			query_id: QueryId,
			query_id_hash: Hash<T>,
			validator: Option<MultiLocation>,
		},
		DelegatorUnbond {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			#[codec(compact)]
			unbond_amount: BalanceOf<T>,
			#[codec(compact)]
			query_id: QueryId,
			query_id_hash: Hash<T>,
			validator: Option<MultiLocation>,
		},
		DelegatorUnbondAll {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			#[codec(compact)]
			query_id: QueryId,
			query_id_hash: Hash<T>,
		},
		DelegatorRebond {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			#[codec(compact)]
			rebond_amount: BalanceOf<T>,
			#[codec(compact)]
			query_id: QueryId,
			query_id_hash: Hash<T>,
			validator: Option<MultiLocation>,
		},
		Delegated {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			targets: Vec<MultiLocation>,
			#[codec(compact)]
			query_id: QueryId,
			query_id_hash: Hash<T>,
		},
		Undelegated {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			targets: Vec<MultiLocation>,
			#[codec(compact)]
			query_id: QueryId,
			query_id_hash: Hash<T>,
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
			#[codec(compact)]
			query_id: QueryId,
			query_id_hash: Hash<T>,
		},
		Chill {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			#[codec(compact)]
			query_id: QueryId,
			query_id_hash: Hash<T>,
		},
		TransferBack {
			currency_id: CurrencyId,
			from: MultiLocation,
			to: MultiLocation,
			#[codec(compact)]
			amount: BalanceOf<T>,
		},
		TransferTo {
			currency_id: CurrencyId,
			from: MultiLocation,
			to: MultiLocation,
			#[codec(compact)]
			amount: BalanceOf<T>,
		},
		DelegatorAdded {
			currency_id: CurrencyId,
			#[codec(compact)]
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
			#[codec(compact)]
			index: u32,
			#[codec(compact)]
			amount: BalanceOf<T>,
		},
		FundMoveFromExitToEntrance {
			currency_id: CurrencyId,
			#[codec(compact)]
			amount: BalanceOf<T>,
		},
		TimeUnitUpdated {
			currency_id: CurrencyId,
			old: TimeUnit,
			new: TimeUnit,
		},
		PoolTokenIncreased {
			currency_id: CurrencyId,
			#[codec(compact)]
			amount: BalanceOf<T>,
		},
		HostingFeeCharged {
			currency_id: CurrencyId,
			#[codec(compact)]
			amount: BalanceOf<T>,
		},
		PoolTokenDecreased {
			currency_id: CurrencyId,
			#[codec(compact)]
			amount: BalanceOf<T>,
		},
		FeeSupplemented {
			currency_id: CurrencyId,
			#[codec(compact)]
			amount: BalanceOf<T>,
			from: MultiLocation,
			to: MultiLocation,
		},
		ValidatorsByDelegatorSet {
			currency_id: CurrencyId,
			validators_list: Vec<(MultiLocation, Hash<T>)>,
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
			ledger: Option<Ledger<MultiLocation, BalanceOf<T>, MultiLocation>>,
		},
		DelegatorLedgerQueryResponseConfirmed {
			#[codec(compact)]
			query_id: QueryId,
			entry: LedgerUpdateEntry<BalanceOf<T>, MultiLocation, MultiLocation>,
		},
		DelegatorLedgerQueryResponseFailSuccessfully {
			#[codec(compact)]
			query_id: QueryId,
		},
		ValidatorsByDelegatorQueryResponseConfirmed {
			#[codec(compact)]
			query_id: QueryId,
			entry: ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation, Hash<T>>,
		},
		ValidatorsByDelegatorQueryResponseFailSuccessfully {
			#[codec(compact)]
			query_id: QueryId,
		},
		MinimumsMaximumsSet {
			currency_id: CurrencyId,
			minimums_and_maximums: Option<MinimumsMaximums<BalanceOf<T>>>,
		},
		CurrencyDelaysSet {
			currency_id: CurrencyId,
			delays: Option<Delays>,
		},
		HostingFeesSet {
			currency_id: CurrencyId,
			fees: Option<(Permill, MultiLocation)>,
		},
		CurrencyTuneExchangeRateLimitSet {
			currency_id: CurrencyId,
			tune_exchange_rate_limit: Option<(u32, Permill)>,
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

	/// Hosting fee percentage and beneficiary account for different chains
	#[pallet::storage]
	#[pallet::getter(fn get_hosting_fee)]
	pub type HostingFees<T> = StorageMap<_, Blake2_128Concat, CurrencyId, (Permill, MultiLocation)>;

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
		StorageMap<_, Blake2_128Concat, CurrencyId, Vec<(MultiLocation, Hash<T>)>>;

	/// Validators for each delegator. CurrencyId + Delegator => Vec<Validator>
	#[pallet::storage]
	#[pallet::getter(fn get_validators_by_delegator)]
	pub type ValidatorsByDelegator<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		Vec<(MultiLocation, Hash<T>)>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_validators_by_delegator_update_entry)]
	pub type ValidatorsByDelegatorXcmUpdateQueue<T> = StorageMap<
		_,
		Blake2_128Concat,
		QueryId,
		(
			ValidatorsByDelegatorUpdateEntry<MultiLocation, MultiLocation, Hash<T>>,
			BlockNumberFor<T>,
		),
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
		Ledger<MultiLocation, BalanceOf<T>, MultiLocation>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_delegator_ledger_update_entry)]
	pub type DelegatorLedgerXcmUpdateQueue<T> = StorageMap<
		_,
		Blake2_128Concat,
		QueryId,
		(LedgerUpdateEntry<BalanceOf<T>, MultiLocation, MultiLocation>, BlockNumberFor<T>),
	>;

	/// Minimum and Maximum constraints for different chains.
	#[pallet::storage]
	#[pallet::getter(fn get_minimums_maximums)]
	pub type MinimumsAndMaximums<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, MinimumsMaximums<BalanceOf<T>>>;

	/// TimeUnit delay params for different chains.
	#[pallet::storage]
	#[pallet::getter(fn get_currency_delays)]
	pub type CurrencyDelays<T> = StorageMap<_, Blake2_128Concat, CurrencyId, Delays>;

	/// A delegator's tuning record of exchange rate for the current time unit.
	/// Currency Id + Delegator Id => (latest tuned TimeUnit, number of tuning times)
	#[pallet::storage]
	#[pallet::getter(fn get_delegator_latest_tune_record)]
	pub type DelegatorLatestTuneRecord<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		(TimeUnit, u32),
		OptionQuery,
	>;

	/// For each currencyId: how many times that a delegator can tune the exchange rate for a single
	/// time unit, and how much at most each time a delegator can tune the exchange rate
	#[pallet::storage]
	#[pallet::getter(fn get_currency_tune_exchange_rate_limit)]
	pub type CurrencyTuneExchangeRateLimit<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (u32, Permill)>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			// For queries in update entry queues, search responses in pallet_xcm Queries storage.
			let counter = Self::process_query_entry_records().unwrap_or(0);

			// Calculate weight
			BASE_WEIGHT.saturating_mul(counter.into())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// *****************************/
		/// ****** Outer Calls ******/
		/// *****************************/
		///
		/// Delegator initialization work. Generate a new delegator and return its ID.
		#[transactional]
		#[pallet::weight(T::WeightInfo::initialize_delegator())]
		pub fn initialize_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let delegator_id = staking_agent.initialize_delegator()?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorInitialized { currency_id, delegator_id });
			Ok(())
		}

		/// First time bonding some amount to a delegator.
		#[transactional]
		#[pallet::weight(T::WeightInfo::bond())]
		pub fn bond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			#[pallet::compact] amount: BalanceOf<T>,
			validator: Option<MultiLocation>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.bond(&who, amount, &validator)?;
			let query_id_hash = T::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorBonded {
				currency_id,
				delegator_id: who,
				bonded_amount: amount,
				query_id,
				query_id_hash,
				validator,
			});
			Ok(())
		}

		/// Bond extra amount to a delegator.
		#[transactional]
		#[pallet::weight(T::WeightInfo::bond_extra())]
		pub fn bond_extra(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			validator: Option<MultiLocation>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.bond_extra(&who, amount, &validator)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorBondExtra {
				currency_id,
				delegator_id: who,
				extra_bonded_amount: amount,
				query_id,
				query_id_hash,
				validator,
			});
			Ok(())
		}

		/// Decrease some amount to a delegator. Leave no less than the minimum delegator
		/// requirement.
		#[transactional]
		#[pallet::weight(T::WeightInfo::unbond())]
		pub fn unbond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			validator: Option<MultiLocation>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.unbond(&who, amount, &validator)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorUnbond {
				currency_id,
				delegator_id: who,
				unbond_amount: amount,
				query_id,
				query_id_hash,
				validator,
			});
			Ok(())
		}

		/// Unbond all the active amount of a delegator.
		#[transactional]
		#[pallet::weight(T::WeightInfo::unbond_all())]
		pub fn unbond_all(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.unbond_all(&who)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorUnbondAll {
				currency_id,
				delegator_id: who,
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		/// Rebond some unlocking amount to a delegator.
		#[transactional]
		#[pallet::weight(T::WeightInfo::rebond())]
		pub fn rebond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			validator: Option<MultiLocation>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.rebond(&who, amount, &validator)?;
			let query_id_hash = T::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorRebond {
				currency_id,
				delegator_id: who,
				rebond_amount: amount,
				query_id,
				query_id_hash,
				validator,
			});
			Ok(())
		}

		/// Delegate to some validator set.
		#[transactional]
		#[pallet::weight(T::WeightInfo::delegate())]
		pub fn delegate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			targets: Vec<MultiLocation>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.delegate(&who, &targets)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Delegated {
				currency_id,
				delegator_id: who,
				targets,
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		/// Re-delegate existing delegation to a new validator set.
		#[transactional]
		#[pallet::weight(T::WeightInfo::undelegate())]
		pub fn undelegate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			targets: Vec<MultiLocation>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.undelegate(&who, &targets)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Undelegated {
				currency_id,
				delegator_id: who,
				targets,
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		/// Re-delegate existing delegation to a new validator set.
		#[transactional]
		#[pallet::weight(T::WeightInfo::redelegate())]
		pub fn redelegate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			targets: Vec<MultiLocation>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.redelegate(&who, &targets)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Delegated {
				currency_id,
				delegator_id: who,
				targets,
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		/// Initiate payout for a certain delegator.
		#[transactional]
		#[pallet::weight(T::WeightInfo::payout())]
		pub fn payout(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			validator: Box<MultiLocation>,
			when: Option<TimeUnit>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.payout(&who, &validator, &when)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Payout {
				currency_id,
				validator: *validator,
				time_unit: when,
			});
			Ok(())
		}

		/// Withdraw the due payout into free balance.
		#[transactional]
		#[pallet::weight(T::WeightInfo::liquidize())]
		pub fn liquidize(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
			when: Option<TimeUnit>,
			validator: Option<MultiLocation>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.liquidize(&who, &when, &validator)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Liquidize {
				currency_id,
				delegator_id: who,
				time_unit: when,
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		/// Initiate payout for a certain delegator.
		#[transactional]
		#[pallet::weight(T::WeightInfo::chill())]
		pub fn chill(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.chill(&who)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Chill {
				currency_id,
				delegator_id: who,
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::transfer_back())]
		pub fn transfer_back(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			from: Box<MultiLocation>,
			to: Box<MultiLocation>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.transfer_back(&from, &to, amount)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TransferBack {
				currency_id,
				from: *from,
				to: *to,
				amount,
			});

			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::transfer_to())]
		pub fn transfer_to(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			from: Box<MultiLocation>,
			to: Box<MultiLocation>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.transfer_to(&from, &to, amount)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TransferTo {
				currency_id,
				from: *from,
				to: *to,
				amount,
			});

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::increase_token_pool())]
		pub fn increase_token_pool(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

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
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

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
			Self::ensure_authorized(origin, currency_id)?;

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
			Self::ensure_authorized(origin, currency_id)?;

			// Get entrance_account and exit_account, as well as their currency balances.
			let (entrance_account, exit_account) =
				T::VtokenMinting::get_entrance_and_exit_accounts();
			let mut exit_account_balance =
				T::MultiCurrency::free_balance(currency_id, &exit_account);

			if exit_account_balance.is_zero() {
				return Ok(());
			}

			// Get the currency due unlocking records
			let time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
				.ok_or(Error::<T>::TimeUnitNotExist)?;
			let rs = T::VtokenMinting::get_unlock_records(currency_id, time_unit.clone());

			// Refund due unlocking records one by one.
			if let Some((_locked_amount, idx_vec)) = rs {
				let mut counter = 0;

				for idx in idx_vec.iter() {
					if counter >= T::MaxRefundPerBlock::get() {
						break;
					}
					// get idx record amount
					let idx_record_amount_op =
						T::VtokenMinting::get_token_unlock_ledger(currency_id, *idx);

					if let Some((user_account, idx_record_amount, _unlock_era)) =
						idx_record_amount_op
					{
						let mut deduct_amount = idx_record_amount;
						if exit_account_balance < idx_record_amount {
							deduct_amount = exit_account_balance;
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

						counter = counter.saturating_add(1);

						exit_account_balance = exit_account_balance
							.checked_sub(&deduct_amount)
							.ok_or(Error::<T>::UnderFlow)?;
						if exit_account_balance == Zero::zero() {
							break;
						}
					}
				}
			} else {
				// Automatically move the rest amount in exit account to entrance account.
				T::MultiCurrency::transfer(
					currency_id,
					&exit_account,
					&entrance_account,
					exit_account_balance,
				)?;
			}

			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::supplement_fee_reserve())]
		pub fn supplement_fee_reserve(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			dest: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

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
				let staking_agent = Self::get_currency_staking_agent(currency_id)?;
				staking_agent.supplement_fee_reserve(reserved_fee, &source_location, &dest)?;
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

		#[transactional]
		#[pallet::weight(T::WeightInfo::charge_host_fee_and_tune_vtoken_exchange_rate())]
		/// Charge staking host fee, tune vtoken/token exchange rate, and update delegator ledger
		/// for single delegator.
		pub fn charge_host_fee_and_tune_vtoken_exchange_rate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] value: BalanceOf<T>,
			who: MultiLocation,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			// Ensure the value is valid.
			ensure!(value > Zero::zero(), Error::<T>::AmountZero);

			// Ensure the value is valid.
			let (limit_num, max_permill) = Self::get_currency_tune_exchange_rate_limit(currency_id)
				.ok_or(Error::<T>::TuneExchangeRateLimitNotSet)?;
			// Get pool token value
			let pool_token = T::VtokenMinting::get_token_pool(currency_id);
			// Calculate max increase allowed.
			let max_to_increase = max_permill.mul_floor(pool_token);
			ensure!(value <= max_to_increase, Error::<T>::GreaterThanMaximum);

			// Ensure this tune is within limit.
			// Get current TimeUnit.
			let current_time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
				.ok_or(Error::<T>::TimeUnitNotExist)?;
			// If this is the first time.
			if !DelegatorLatestTuneRecord::<T>::contains_key(currency_id, &who) {
				// ensure who is a valid delegator
				ensure!(
					DelegatorsMultilocation2Index::<T>::contains_key(currency_id, &who),
					Error::<T>::DelegatorNotExist
				);
				// Insert an empty record into DelegatorLatestTuneRecord storage.
				DelegatorLatestTuneRecord::<T>::insert(
					currency_id,
					who.clone(),
					(current_time_unit.clone(), 0),
				);
			}

			// Get DelegatorLatestTuneRecord for the currencyId.
			let (latest_time_unit, tune_num) =
				Self::get_delegator_latest_tune_record(currency_id, &who)
					.ok_or(Error::<T>::DelegatorLatestTuneRecordNotExist)?;

			// See if exceeds tuning limit.
			// If it has been tuned in the current time unit, ensure this tuning is within limit.
			if latest_time_unit == current_time_unit {
				ensure!(tune_num < limit_num, Error::<T>::GreaterThanMaximum);
			}

			let new_tune_num = tune_num.checked_add(1).ok_or(Error::<T>::OverFlow)?;

			// Get charged fee value
			let (fee_permill, beneficiary) =
				Self::get_hosting_fee(currency_id).ok_or(Error::<T>::InvalidHostingFee)?;
			let fee_to_charge = fee_permill.mul_floor(value);

			// Should first charge fee, and then tune exchange rate. Otherwise, the rate will be
			// wrong.
			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.charge_hosting_fee(
				fee_to_charge,
				// Dummy value for 【from】account
				&beneficiary,
				&beneficiary,
			)?;

			// Tune the new exchange rate.
			staking_agent.tune_vtoken_exchange_rate(
				&who,
				value,
				// Dummy value for vtoken amount
				Zero::zero(),
			)?;

			// Update the DelegatorLatestTuneRecord<T> storage.
			DelegatorLatestTuneRecord::<T>::insert(
				currency_id,
				who.clone(),
				(current_time_unit, new_tune_num),
			);

			// Deposit event.
			Pallet::<T>::deposit_event(Event::HostingFeeCharged {
				currency_id,
				amount: fee_to_charge,
			});
			Pallet::<T>::deposit_event(Event::PoolTokenIncreased { currency_id, amount: value });
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
			XcmDestWeightAndFee::<T>::mutate_exists(currency_id, &operation, |wt_n_f| {
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
		#[transactional]
		#[pallet::weight(T::WeightInfo::add_delegator())]
		pub fn add_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] index: u16,
			who: MultiLocation,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.add_delegator(index, &who)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorAdded {
				currency_id,
				index,
				delegator_id: who,
			});
			Ok(())
		}

		/// Update storage DelegatorsIndex2Multilocation<T> 和 DelegatorsMultilocation2Index<T>.
		#[transactional]
		#[pallet::weight(T::WeightInfo::remove_delegator())]
		pub fn remove_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.remove_delegator(&who)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorRemoved { currency_id, delegator_id: who });
			Ok(())
		}

		/// Update storage Validators<T>.
		#[transactional]
		#[pallet::weight(T::WeightInfo::add_validator())]
		pub fn add_validator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.add_validator(&who)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsAdded { currency_id, validator_id: who });
			Ok(())
		}

		/// Update storage Validators<T>.
		#[transactional]
		#[pallet::weight(T::WeightInfo::remove_validator())]
		pub fn remove_validator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: MultiLocation,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.remove_validator(&who)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsRemoved { currency_id, validator_id: who });
			Ok(())
		}

		/// Update storage ValidatorsByDelegator<T>.
		#[transactional]
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
			ensure!(DelegatorLedgers::<T>::contains_key(KSM, &who), Error::<T>::DelegatorNotBonded);

			let validators_list =
				Self::sort_validators_and_remove_duplicates(currency_id, &validators)?;

			// Update ValidatorsByDelegator storage
			ValidatorsByDelegator::<T>::insert(currency_id, who, validators_list.clone());

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
			who: Box<MultiLocation>,
			ledger: Box<Option<Ledger<MultiLocation, BalanceOf<T>, MultiLocation>>>,
		) -> DispatchResult {
			// Check the validity of origin
			Self::ensure_authorized(origin, currency_id)?;

			let mins_maxs = MinimumsAndMaximums::<T>::get(KSM).ok_or(Error::<T>::NotExist)?;
			// Check the new ledger must has at lease minimum active amount.
			if let Some(ref ldgr) = *ledger {
				if let Ledger::Substrate(lg) = ldgr {
					ensure!(
						lg.active >= mins_maxs.delegator_bonded_minimum,
						Error::<T>::LowerThanMinimum
					);
				}
			}

			// Update the ledger.
			DelegatorLedgers::<T>::mutate_exists(currency_id, &*who, |old_ledger| {
				*old_ledger = *ledger.clone();
			});

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorLedgerSet {
				currency_id,
				delegator: *who,
				ledger: *ledger,
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
				*minimums_maximums = constraints.clone();
			});

			// Deposit event.
			Pallet::<T>::deposit_event(Event::MinimumsMaximumsSet {
				currency_id,
				minimums_and_maximums: constraints,
			});

			Ok(())
		}

		/// Update storage Delays<T>.
		#[pallet::weight(T::WeightInfo::set_currency_delays())]
		pub fn set_currency_delays(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			maybe_delays: Option<Delays>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			CurrencyDelays::<T>::mutate_exists(currency_id, |delays| {
				*delays = maybe_delays.clone();
			});

			// Deposit event.
			Pallet::<T>::deposit_event(Event::CurrencyDelaysSet {
				currency_id,
				delays: maybe_delays,
			});

			Ok(())
		}

		/// Set HostingFees storage.
		#[pallet::weight(T::WeightInfo::set_hosting_fees())]
		pub fn set_hosting_fees(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			maybe_fee_set: Option<(Permill, MultiLocation)>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			HostingFees::<T>::mutate_exists(currency_id, |fee_set| {
				*fee_set = maybe_fee_set.clone();
			});

			Pallet::<T>::deposit_event(Event::HostingFeesSet { currency_id, fees: maybe_fee_set });

			Ok(())
		}

		/// Set  CurrencyTuneExchangeRateLimit<T> storage.
		#[pallet::weight(T::WeightInfo::set_currency_tune_exchange_rate_limit())]
		pub fn set_currency_tune_exchange_rate_limit(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			maybe_tune_exchange_rate_limit: Option<(u32, Permill)>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			CurrencyTuneExchangeRateLimit::<T>::mutate_exists(currency_id, |exchange_rate_limit| {
				*exchange_rate_limit = maybe_tune_exchange_rate_limit.clone();
			});

			Pallet::<T>::deposit_event(Event::CurrencyTuneExchangeRateLimitSet {
				currency_id,
				tune_exchange_rate_limit: maybe_tune_exchange_rate_limit,
			});

			Ok(())
		}

		/// ********************************************************************
		/// *************Outer Confirming Xcm queries functions ****************
		/// ********************************************************************
		#[transactional]
		#[pallet::weight(T::WeightInfo::confirm_delegator_ledger_query_response())]
		pub fn confirm_delegator_ledger_query_response(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] query_id: QueryId,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;
			Self::get_ledger_update_agent_then_process(query_id, true)?;
			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::fail_delegator_ledger_query_response())]
		pub fn fail_delegator_ledger_query_response(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] query_id: QueryId,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			Self::do_fail_delegator_ledger_query_response(query_id)?;
			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::confirm_validators_by_delegator_query_response())]
		pub fn confirm_validators_by_delegator_query_response(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] query_id: QueryId,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;
			Self::get_validators_by_delegator_update_agent_then_process(query_id, true)?;

			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::fail_validators_by_delegator_query_response())]
		pub fn fail_validators_by_delegator_query_response(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] query_id: QueryId,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			Self::do_fail_validators_by_delegator_query_response(query_id)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Ensure privileged origin
		fn ensure_authorized(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> Result<(), Error<T>> {
			match origin.clone().into() {
				Ok(RawOrigin::Signed(ref signer))
					if Some(signer) == <OperateOrigins<T>>::get(currency_id).as_ref() =>
					Ok(()),
				Ok(_) if T::ControlOrigin::ensure_origin(origin).is_ok() => Ok(()),
				_ => Err(Error::<T>::NotAuthorized),
			}
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
				KSM => Ok(Box::new(KusamaAgent::<T>::new())),
				MOVR => Ok(Box::new(MoonriverAgent::<T>::new())),
				_ => Err(Error::<T>::NotSupportedCurrencyId),
			}
		}

		pub fn sort_validators_and_remove_duplicates(
			currency_id: CurrencyId,
			validators: &Vec<MultiLocation>,
		) -> Result<Vec<(MultiLocation, Hash<T>)>, Error<T>> {
			let validators_set =
				Validators::<T>::get(currency_id).ok_or(Error::<T>::ValidatorSetNotExist)?;
			let mut validators_list: Vec<(MultiLocation, Hash<T>)> = vec![];
			for validator in validators.iter() {
				// Check if the validator is in the validator whitelist
				let multi_hash = <T as frame_system::Config>::Hashing::hash(&validator.encode());
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

		pub fn multilocation_to_local_multilocation(
			location: &MultiLocation,
		) -> Result<MultiLocation, Error<T>> {
			let local_location = MultiLocation { parents: 0, interior: location.interior.clone() };

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

		pub fn multilocation_to_account_20(who: &MultiLocation) -> Result<[u8; 20], Error<T>> {
			// Get the delegator account id in Kusama network
			let account_20 = match who {
				MultiLocation {
					parents: _,
					interior:
						X2(Parachain(_), AccountKey20 { network: _network_id, key: account_id }),
				} => account_id,
				_ => Err(Error::<T>::AccountNotExist)?,
			};
			Ok(*account_20)
		}

		pub fn multilocation_to_h160_account(who: &MultiLocation) -> Result<H160, Error<T>> {
			// Get the delegator account id in Kusama network
			let account_20 = Self::multilocation_to_account_20(who)?;
			let account_h160 =
				H160::decode(&mut &account_20[..]).map_err(|_| Error::<T>::DecodingError)?;
			Ok(account_h160)
		}

		/// **************************************/
		/// ****** XCM confirming Functions ******/
		/// **************************************/
		// #[transactional]
		pub fn process_query_entry_records() -> Result<u32, Error<T>> {
			let mut counter = 0u32;

			// Deal with DelegatorLedgerXcmUpdateQueue storage
			for query_id in DelegatorLedgerXcmUpdateQueue::<T>::iter_keys() {
				if counter >= T::MaxTypeEntryPerBlock::get() {
					break;
				}

				let updated = Self::get_ledger_update_agent_then_process(query_id, false)?;
				if updated {
					counter = counter.saturating_add(1);
				}
			}

			// Deal with ValidatorsByDelegator storage
			for query_id in ValidatorsByDelegatorXcmUpdateQueue::<T>::iter_keys() {
				if counter >= T::MaxTypeEntryPerBlock::get() {
					break;
				}
				let updated =
					Self::get_validators_by_delegator_update_agent_then_process(query_id, false)?;

				if updated {
					counter = counter.saturating_add(1);
				}
			}

			Ok(counter)
		}

		pub fn get_ledger_update_agent_then_process(
			query_id: QueryId,
			manual_mode: bool,
		) -> Result<bool, Error<T>> {
			// See if the query exists. If it exists, call corresponding chain storage update
			// function.
			let (entry, timeout) = Self::get_delegator_ledger_update_entry(query_id)
				.ok_or(Error::<T>::QueryNotExist)?;

			let now = frame_system::Pallet::<T>::block_number();
			let mut updated = true;
			if now <= timeout {
				let currency_id = match entry.clone() {
					LedgerUpdateEntry::Substrate(substrate_entry) =>
						Some(substrate_entry.currency_id),
					_ => None,
				}
				.ok_or(Error::<T>::NotSupportedCurrencyId)?;

				let staking_agent = Self::get_currency_staking_agent(currency_id)?;
				updated = staking_agent.check_delegator_ledger_query_response(
					query_id,
					entry,
					manual_mode,
				)?;
			} else {
				Self::do_fail_delegator_ledger_query_response(query_id)?;
			}

			Ok(updated)
		}

		pub fn get_validators_by_delegator_update_agent_then_process(
			query_id: QueryId,
			manual_mode: bool,
		) -> Result<bool, Error<T>> {
			// See if the query exists. If it exists, call corresponding chain storage update
			// function.
			let (entry, timeout) = Self::get_validators_by_delegator_update_entry(query_id)
				.ok_or(Error::<T>::QueryNotExist)?;

			let now = frame_system::Pallet::<T>::block_number();
			let mut updated = true;
			if now <= timeout {
				let currency_id = match entry.clone() {
					ValidatorsByDelegatorUpdateEntry::Substrate(substrate_entry) =>
						Some(substrate_entry.currency_id),
					_ => None,
				}
				.ok_or(Error::<T>::NotSupportedCurrencyId)?;

				let staking_agent = Self::get_currency_staking_agent(currency_id)?;
				updated = staking_agent.check_validators_by_delegator_query_response(
					query_id,
					entry,
					manual_mode,
				)?;
			} else {
				Self::do_fail_validators_by_delegator_query_response(query_id)?;
			}
			Ok(updated)
		}

		fn do_fail_delegator_ledger_query_response(query_id: QueryId) -> Result<(), Error<T>> {
			// See if the query exists. If it exists, call corresponding chain storage update
			// function.
			let (entry, _) = Self::get_delegator_ledger_update_entry(query_id)
				.ok_or(Error::<T>::QueryNotExist)?;
			let currency_id = match entry {
				LedgerUpdateEntry::Substrate(substrate_entry) => Some(substrate_entry.currency_id),
				_ => None,
			}
			.ok_or(Error::<T>::NotSupportedCurrencyId)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.fail_delegator_ledger_query_response(query_id)?;

			Ok(())
		}

		fn do_fail_validators_by_delegator_query_response(
			query_id: QueryId,
		) -> Result<(), Error<T>> {
			// See if the query exists. If it exists, call corresponding chain storage update
			// function.
			let (entry, _) = Self::get_validators_by_delegator_update_entry(query_id)
				.ok_or(Error::<T>::QueryNotExist)?;
			let currency_id = match entry {
				ValidatorsByDelegatorUpdateEntry::Substrate(substrate_entry) =>
					Some(substrate_entry.currency_id),
				_ => None,
			}
			.ok_or(Error::<T>::NotSupportedCurrencyId)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.fail_validators_by_delegator_query_response(query_id)?;

			Ok(())
		}

		pub fn derivative_account_id_20(who: [u8; 20], index: u16) -> H160 {
			let entropy = (b"modlpy/utilisuba", who, index).using_encoded(blake2_256);
			let sub_id: [u8; 20] = Decode::decode(&mut TrailingZeroInput::new(entropy.as_ref()))
				.expect("infinite length input; no invalid inputs for type; qed");

			H160::from_slice(sub_id.as_slice())
		}
	}
}

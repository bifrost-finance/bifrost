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
#![recursion_limit = "256"]

extern crate core;

use crate::{agents::PolkadotAgent, Junction::GeneralIndex, Junctions::X2};
pub use crate::{
	primitives::{
		Delays, LedgerUpdateEntry, MinimumsMaximums, QueryId, SubstrateLedger,
		ValidatorsByDelegatorUpdateEntry,
	},
	traits::{OnRefund, QueryResponseManager, StakingAgent},
	Junction::AccountId32,
	Junctions::X1,
};
use bifrost_asset_registry::AssetMetadata;
use bifrost_parachain_staking::ParachainStakingInterface;
use bifrost_primitives::{
	currency::{BNC, KSM, MANTA, MOVR, PHA},
	traits::XcmDestWeightAndFeeHandler,
	CurrencyId, CurrencyIdExt, CurrencyIdMapping, DerivativeAccountHandler, DerivativeIndex,
	SlpHostingFeeProvider, SlpOperator, TimeUnit, VtokenMintingOperator, XcmOperationType, ASTR,
	DOT, FIL, GLMR,
};
use bifrost_stable_pool::traits::StablePoolHandler;
use cumulus_primitives_core::{relay_chain::HashT, ParaId};
use frame_support::{pallet_prelude::*, traits::Contains, weights::Weight};
use frame_system::{
	ensure_signed,
	pallet_prelude::{BlockNumberFor, OriginFor},
	RawOrigin,
};
use orml_traits::MultiCurrency;
pub use primitives::Ledger;
use sp_arithmetic::{per_things::Permill, traits::Zero};
use sp_core::{bounded::BoundedVec, H160};
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{CheckedAdd, CheckedSub, Convert, TrailingZeroInput, UniqueSaturatedFrom};
use sp_std::{boxed::Box, vec, vec::Vec};
pub use weights::WeightInfo;
use xcm::{
	prelude::*,
	v3::{Junction, Junctions, MultiLocation, Xcm},
};

mod agents;
pub mod migrations;
mod mocks;
pub mod primitives;
mod tests;
pub mod traits;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pallet::*;

pub type Result<T, E> = core::result::Result<T, E>;
type Hash<T> = <T as frame_system::Config>::Hash;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;
type StakingAgentBoxType<T> = Box<
	dyn StakingAgent<
		BalanceOf<T>,
		AccountIdOf<T>,
		LedgerUpdateEntry<BalanceOf<T>>,
		ValidatorsByDelegatorUpdateEntry,
		pallet::Error<T>,
	>,
>;
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;
const SIX_MONTHS: u32 = 5 * 60 * 24 * 180;
const ITERATE_LENGTH: usize = 100;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::agents::{AstarAgent, FilecoinAgent, ParachainStakingAgent, PhalaAgent};
	use bifrost_primitives::{RedeemType, SlpxOperator};
	use frame_support::dispatch::GetDispatchInfo;
	use orml_traits::XcmTransfer;
	use pallet_xcm::ensure_response;
	use xcm::v3::{MaybeErrorCode, Response};

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_xcm::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type RuntimeOrigin: IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>;

		type RuntimeCall: Parameter + From<Call<Self>> + GetDispatchInfo;

		/// Currency operations handler
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
		/// The only origin that can modify pallet params
		type ControlOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		/// Set default weight.
		type WeightInfo: WeightInfo;

		/// The interface to call VtokenMinting module functions.
		type VtokenMinting: VtokenMintingOperator<
			CurrencyId,
			BalanceOf<Self>,
			AccountIdOf<Self>,
			TimeUnit,
		>;

		type BifrostSlpx: SlpxOperator<BalanceOf<Self>>;

		/// xtokens xcm transfer interface
		type XcmTransfer: XcmTransfer<AccountIdOf<Self>, BalanceOf<Self>, CurrencyIdOf<Self>>;

		/// Substrate account converter, which can convert a u16 number into a sub-account with
		/// MultiLocation format.
		type AccountConverter: Convert<(u16, CurrencyId), MultiLocation>;

		/// Parachain Id which is gotten from the runtime.
		type ParachainId: Get<ParaId>;

		/// Substrate response manager.
		type SubstrateResponseManager: QueryResponseManager<
			QueryId,
			MultiLocation,
			BlockNumberFor<Self>,
			<Self as pallet::Config>::RuntimeCall,
		>;

		/// Handler to notify the runtime when refund.
		/// If you don't need it, you can specify the type `()`.
		type OnRefund: OnRefund<AccountIdOf<Self>, CurrencyId, BalanceOf<Self>>;

		type XcmWeightAndFeeHandler: XcmDestWeightAndFeeHandler<CurrencyId, BalanceOf<Self>>;

		#[pallet::constant]
		type MaxTypeEntryPerBlock: Get<u32>;

		#[pallet::constant]
		type MaxRefundPerBlock: Get<u32>;

		#[pallet::constant]
		type MaxLengthLimit: Get<u32>;

		type ParachainStaking: ParachainStakingInterface<AccountIdOf<Self>, BalanceOf<Self>>;

		type ChannelCommission: SlpHostingFeeProvider<
			CurrencyId,
			BalanceOf<Self>,
			AccountIdOf<Self>,
		>;

		type StablePoolHandler: StablePoolHandler<
			Balance = BalanceOf<Self>,
			AccountId = AccountIdOf<Self>,
			CurrencyId = CurrencyId,
		>;

		// asset registry to get asset metadata
		type AssetIdMaps: CurrencyIdMapping<
			CurrencyId,
			MultiLocation,
			AssetMetadata<BalanceOf<Self>>,
		>;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		OperateOriginNotSet,
		NotAuthorized,
		NotSupportedCurrencyId,
		FailToAddDelegator,
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
		WeightAndFeeNotExists,
		MinimumsAndMaximumsNotExist,
		QueryNotExist,
		DelaysNotExist,
		Unexpected,
		QueryResponseRemoveError,
		InvalidHostingFee,
		InvalidAccount,
		IncreaseTokenPoolError,
		TuneExchangeRateLimitNotSet,
		CurrencyLatestTuneRecordNotExist,
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
		ValidatorError,
		AmountNone,
		InvalidDelays,
		OngoingTimeUnitUpdateIntervalNotExist,
		LastTimeUpdatedOngoingTimeUnitNotExist,
		TooFrequent,
		DestAccountNotValid,
		WhiteListNotExist,
		DelegatorAlreadyTuned,
		FeeTooHigh,
		NotEnoughBalance,
		VectorTooLong,
		MultiCurrencyError,
		NotDelegateValidator,
		DividedByZero,
		SharePriceNotValid,
		InvalidAmount,
		ValidatorMultilocationNotvalid,
		AmountNotProvided,
		FailToConvert,
		ExceedMaxLengthLimit,
		/// Transfer to failed
		TransferToError,
		StablePoolNotFound,
		StablePoolTokenIndexNotFound,
		ExceedLimit,
		InvalidPageNumber,
		NoMoreValidatorBoostListForCurrency,
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
			rebond_amount: Option<BalanceOf<T>>,
			#[codec(compact)]
			query_id: QueryId,
			query_id_hash: Hash<T>,
			validator: Option<MultiLocation>,
		},
		Delegated {
			currency_id: CurrencyId,
			delegator_id: MultiLocation,
			targets: Option<Vec<MultiLocation>>,
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
			amount: Option<BalanceOf<T>>,
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
		ConvertAsset {
			currency_id: CurrencyId,
			who: MultiLocation,
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
			old: Option<TimeUnit>,
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
			validators_list: Vec<MultiLocation>,
			delegator_id: MultiLocation,
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
			ledger: Option<Ledger<BalanceOf<T>>>,
		},
		DelegatorLedgerQueryResponseConfirmed {
			#[codec(compact)]
			query_id: QueryId,
			entry: LedgerUpdateEntry<BalanceOf<T>>,
		},
		DelegatorLedgerQueryResponseFailed {
			#[codec(compact)]
			query_id: QueryId,
		},
		ValidatorsByDelegatorQueryResponseConfirmed {
			#[codec(compact)]
			query_id: QueryId,
			entry: ValidatorsByDelegatorUpdateEntry,
		},
		ValidatorsByDelegatorQueryResponseFailed {
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
		OngoingTimeUnitUpdateIntervalSet {
			currency_id: CurrencyId,
			interval: Option<BlockNumberFor<T>>,
		},
		SupplementFeeAccountWhitelistAdded {
			currency_id: CurrencyId,
			who: MultiLocation,
		},
		SupplementFeeAccountWhitelistRemoved {
			currency_id: CurrencyId,
			who: MultiLocation,
		},
		ValidatorsReset {
			currency_id: CurrencyId,
			validator_list: Vec<MultiLocation>,
		},

		ValidatorBoostListSet {
			currency_id: CurrencyId,
			validator_boost_list: Vec<(MultiLocation, BlockNumberFor<T>)>,
		},

		ValidatorBoostListAdded {
			currency_id: CurrencyId,
			who: MultiLocation,
			due_block_number: BlockNumberFor<T>,
		},

		RemovedFromBoostList {
			currency_id: CurrencyId,
			who: MultiLocation,
		},
		OutdatedValidatorBoostListCleaned {
			currency_id: CurrencyId,
			page: u8,
			// already removed num
			remove_num: u32,
			// still to iterate num
			num_left: u32,
		},
		BurnFeeFailed {
			currency_id: CurrencyId,
			amount: BalanceOf<T>,
		},
	}

	/// The current storage version, we set to 3 our new version(after migrate stroage from vec t
	/// boundedVec).
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(3);

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

	/// (VWL) Validator in service. A validator is identified in MultiLocation format.
	#[pallet::storage]
	#[pallet::getter(fn get_validators)]
	pub type Validators<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BoundedVec<MultiLocation, T::MaxLengthLimit>>;

	/// (VBL) Validator Boost List -> (validator multilocation, due block number)
	#[pallet::storage]
	#[pallet::getter(fn get_validator_boost_list)]
	pub type ValidatorBoostList<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		BoundedVec<(MultiLocation, BlockNumberFor<T>), T::MaxLengthLimit>,
	>;

	/// Validators for each delegator. CurrencyId + Delegator => Vec<Validator>
	#[pallet::storage]
	#[pallet::getter(fn get_validators_by_delegator)]
	pub type ValidatorsByDelegator<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		BoundedVec<MultiLocation, T::MaxLengthLimit>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_validators_by_delegator_update_entry)]
	pub type ValidatorsByDelegatorXcmUpdateQueue<T> = StorageMap<
		_,
		Blake2_128Concat,
		QueryId,
		(ValidatorsByDelegatorUpdateEntry, BlockNumberFor<T>),
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
		Ledger<BalanceOf<T>>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_delegator_ledger_update_entry)]
	pub type DelegatorLedgerXcmUpdateQueue<T> = StorageMap<
		_,
		Blake2_128Concat,
		QueryId,
		(LedgerUpdateEntry<BalanceOf<T>>, BlockNumberFor<T>),
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
	/// Currency Id + Delegator Id => latest tuned TimeUnit
	#[pallet::storage]
	#[pallet::getter(fn get_delegator_latest_tune_record)]
	pub type DelegatorLatestTuneRecord<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyId,
		Blake2_128Concat,
		MultiLocation,
		TimeUnit,
		OptionQuery,
	>;

	/// Currency's tuning record of exchange rate for the current time unit.
	/// Currency Id => (latest tuned TimeUnit, number of tuning times)
	#[pallet::storage]
	#[pallet::getter(fn get_currency_latest_tune_record)]
	pub type CurrencyLatestTuneRecord<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (TimeUnit, u32), OptionQuery>;

	/// For each currencyId: how many times that a Currency's all delegators can tune the exchange
	/// rate for a single time unit, and how much at most each time can tune the
	/// exchange rate
	#[pallet::storage]
	#[pallet::getter(fn get_currency_tune_exchange_rate_limit)]
	pub type CurrencyTuneExchangeRateLimit<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, (u32, Permill)>;

	/// reflect if all delegations are on a decrease/revoke status. If yes, then new user redeeming
	/// is unaccepted.
	#[pallet::storage]
	#[pallet::getter(fn get_all_delegations_occupied_status)]
	pub type DelegationsOccupied<T> = StorageMap<_, Blake2_128Concat, CurrencyId, bool>;

	#[pallet::storage]
	#[pallet::getter(fn get_last_time_updated_ongoing_time_unit)]
	pub type LastTimeUpdatedOngoingTimeUnit<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BlockNumberFor<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_ongoing_time_unit_update_interval)]
	pub type OngoingTimeUnitUpdateInterval<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, BlockNumberFor<T>>;

	#[pallet::storage]
	#[pallet::getter(fn get_supplement_fee_account_wihtelist)]
	pub type SupplementFeeAccountWhitelist<T> =
		StorageMap<_, Blake2_128Concat, CurrencyId, Vec<(MultiLocation, Hash<T>)>>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// *****************************
		/// ****** Outer Calls ******
		/// *****************************
		///
		/// Delegator initialization work. Generate a new delegator and return its ID.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::initialize_delegator())]
		pub fn initialize_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			delegator_location: Option<Box<MultiLocation>>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let delegator_id =
				staking_agent.initialize_delegator(currency_id, delegator_location)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorInitialized { currency_id, delegator_id });
			Ok(())
		}

		/// First time bonding some amount to a delegator.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::bond())]
		pub fn bond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			#[pallet::compact] amount: BalanceOf<T>,
			validator: Option<MultiLocation>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id =
				staking_agent.bond(&who, amount, &validator, currency_id, weight_and_fee)?;
			let query_id_hash = T::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorBonded {
				currency_id,
				delegator_id: *who,
				bonded_amount: amount,
				query_id,
				query_id_hash,
				validator,
			});
			Ok(())
		}

		/// Bond extra amount to a delegator.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::bond_extra())]
		pub fn bond_extra(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			validator: Option<MultiLocation>,
			#[pallet::compact] amount: BalanceOf<T>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id =
				staking_agent.bond_extra(&who, amount, &validator, currency_id, weight_and_fee)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorBondExtra {
				currency_id,
				delegator_id: *who,
				extra_bonded_amount: amount,
				query_id,
				query_id_hash,
				validator,
			});
			Ok(())
		}

		/// Decrease some amount to a delegator. Leave no less than the minimum delegator
		/// requirement.
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::unbond())]
		pub fn unbond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			validator: Option<MultiLocation>,
			#[pallet::compact] amount: BalanceOf<T>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id =
				staking_agent.unbond(&who, amount, &validator, currency_id, weight_and_fee)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorUnbond {
				currency_id,
				delegator_id: *who,
				unbond_amount: amount,
				query_id,
				query_id_hash,
				validator,
			});
			Ok(())
		}

		/// Unbond all the active amount of a delegator.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::unbond_all())]
		pub fn unbond_all(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.unbond_all(&who, currency_id, weight_and_fee)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorUnbondAll {
				currency_id,
				delegator_id: *who,
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		/// Rebond some unlocking amount to a delegator.
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::rebond())]
		pub fn rebond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			validator: Option<MultiLocation>,
			amount: Option<BalanceOf<T>>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id =
				staking_agent.rebond(&who, amount, &validator, currency_id, weight_and_fee)?;
			let query_id_hash = T::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorRebond {
				currency_id,
				delegator_id: *who,
				rebond_amount: amount,
				query_id,
				query_id_hash,
				validator,
			});
			Ok(())
		}

		/// Delegate to some validator set.
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::delegate())]
		pub fn delegate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			targets: Vec<MultiLocation>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.delegate(&who, &targets, currency_id, weight_and_fee)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Delegated {
				currency_id,
				delegator_id: *who,
				targets: Some(targets),
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		/// Re-delegate existing delegation to a new validator set.
		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::undelegate())]
		pub fn undelegate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			targets: Vec<MultiLocation>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.undelegate(&who, &targets, currency_id, weight_and_fee)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Undelegated {
				currency_id,
				delegator_id: *who,
				targets,
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		/// Re-delegate existing delegation to a new validator set.
		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::redelegate())]
		pub fn redelegate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			targets: Option<Vec<MultiLocation>>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.redelegate(&who, &targets, currency_id, weight_and_fee)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Delegated {
				currency_id,
				delegator_id: *who,
				targets,
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		/// Initiate payout for a certain delegator.
		#[pallet::call_index(9)]
		#[pallet::weight(<T as Config>::WeightInfo::payout())]
		pub fn payout(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			validator: Box<MultiLocation>,
			when: Option<TimeUnit>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.payout(&who, &validator, &when, currency_id, weight_and_fee)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Payout {
				currency_id,
				validator: *validator,
				time_unit: when,
			});
			Ok(())
		}

		/// Withdraw the due payout into free balance.
		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::liquidize())]
		pub fn liquidize(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			when: Option<TimeUnit>,
			validator: Option<MultiLocation>,
			amount: Option<BalanceOf<T>>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.liquidize(
				&who,
				&when,
				&validator,
				currency_id,
				amount,
				weight_and_fee,
			)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Liquidize {
				currency_id,
				delegator_id: *who,
				time_unit: when,
				query_id,
				query_id_hash,
				amount,
			});
			Ok(())
		}

		/// Initiate payout for a certain delegator.
		#[pallet::call_index(11)]
		#[pallet::weight(<T as Config>::WeightInfo::chill())]
		pub fn chill(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			let query_id = staking_agent.chill(&who, currency_id, weight_and_fee)?;
			let query_id_hash = <T as frame_system::Config>::Hashing::hash(&query_id.encode());

			// Deposit event.
			Pallet::<T>::deposit_event(Event::Chill {
				currency_id,
				delegator_id: *who,
				query_id,
				query_id_hash,
			});
			Ok(())
		}

		#[pallet::call_index(12)]
		#[pallet::weight(<T as Config>::WeightInfo::transfer_back())]
		pub fn transfer_back(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			from: Box<MultiLocation>,
			to: Box<MultiLocation>,
			#[pallet::compact] amount: BalanceOf<T>,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.transfer_back(&from, &to, amount, currency_id, weight_and_fee)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TransferBack {
				currency_id,
				from: *from,
				to: *to,
				amount,
			});

			Ok(())
		}

		#[pallet::call_index(13)]
		#[pallet::weight(<T as Config>::WeightInfo::transfer_to())]
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
			staking_agent.transfer_to(&from, &to, amount, currency_id)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TransferTo {
				currency_id,
				from: *from,
				to: *to,
				amount,
			});

			Ok(())
		}

		// Convert token to another token.
		// if we convert from currency_id to some other currency, then if_from_currency should be
		// true. if we convert from some other currency to currency_id, then if_from_currency should
		// be false.
		#[pallet::call_index(14)]
		#[pallet::weight(<T as Config>::WeightInfo::convert_asset())]
		pub fn convert_asset(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			#[pallet::compact] amount: BalanceOf<T>,
			if_from_currency: bool,
			weight_and_fee: Option<(Weight, BalanceOf<T>)>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.convert_asset(
				&who,
				amount,
				currency_id,
				if_from_currency,
				weight_and_fee,
			)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ConvertAsset { currency_id, who: *who, amount });

			Ok(())
		}

		#[pallet::call_index(15)]
		#[pallet::weight(<T as Config>::WeightInfo::increase_token_pool())]
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

		#[pallet::call_index(16)]
		#[pallet::weight(<T as Config>::WeightInfo::decrease_token_pool())]
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

		#[pallet::call_index(17)]
		#[pallet::weight(<T as Config>::WeightInfo::update_ongoing_time_unit())]
		pub fn update_ongoing_time_unit(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			time_unit: TimeUnit,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			// check current block is beyond the interval of ongoing timeunit updating.
			let interval = OngoingTimeUnitUpdateInterval::<T>::get(currency_id)
				.ok_or(Error::<T>::OngoingTimeUnitUpdateIntervalNotExist)?;

			let last_update_block = LastTimeUpdatedOngoingTimeUnit::<T>::get(currency_id)
				.ok_or(Error::<T>::LastTimeUpdatedOngoingTimeUnitNotExist)?;
			let current_block = frame_system::Pallet::<T>::block_number();
			let blocks_between =
				current_block.checked_sub(&last_update_block).ok_or(Error::<T>::UnderFlow)?;

			ensure!(blocks_between >= interval, Error::<T>::TooFrequent);

			let old_op = T::VtokenMinting::get_ongoing_time_unit(currency_id);

			if let Some(old) = old_op.clone() {
				// enusre old TimeUnit < new TimeUnit
				ensure!(old < time_unit, Error::<T>::InvalidTimeUnit);
			}

			T::VtokenMinting::update_ongoing_time_unit(currency_id, time_unit.clone())?;

			// update LastTimeUpdatedOngoingTimeUnit storage
			LastTimeUpdatedOngoingTimeUnit::<T>::insert(currency_id, current_block);

			// Deposit event.
			Pallet::<T>::deposit_event(Event::TimeUnitUpdated {
				currency_id,
				old: old_op,
				new: time_unit,
			});

			Ok(())
		}

		#[pallet::call_index(18)]
		#[pallet::weight(<T as Config>::WeightInfo::refund_currency_due_unbond())]
		pub fn refund_currency_due_unbond(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
		) -> DispatchResultWithPostInfo {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			// Get entrance_account and exit_account, as well as their currency balances.
			let (entrance_account, exit_account) =
				T::VtokenMinting::get_entrance_and_exit_accounts();
			let mut exit_account_balance =
				T::MultiCurrency::free_balance(currency_id, &exit_account);

			if exit_account_balance.is_zero() {
				return Ok(().into());
			}

			// Get the currency due unlocking records
			let time_unit = T::VtokenMinting::get_ongoing_time_unit(currency_id)
				.ok_or(Error::<T>::TimeUnitNotExist)?;
			let rs = T::VtokenMinting::get_unlock_records(currency_id, time_unit.clone());

			let mut extra_weight = 0 as u64;

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

					if let Some((user_account, idx_record_amount, _unlock_era, redeem_type)) =
						idx_record_amount_op
					{
						let mut deduct_amount = idx_record_amount;
						if exit_account_balance < idx_record_amount {
							match redeem_type {
								RedeemType::Native => {},
								RedeemType::Astar(_) |
								RedeemType::Moonbeam(_) |
								RedeemType::Hydradx(_) |
								RedeemType::Manta(_) |
								RedeemType::Interlay(_) => break,
							};
							deduct_amount = exit_account_balance;
						};
						match redeem_type {
							RedeemType::Native => {
								// Transfer some amount from the exit_account to the user's account
								T::MultiCurrency::transfer(
									currency_id,
									&exit_account,
									&user_account,
									deduct_amount,
								)?;
							},
							RedeemType::Astar(receiver) => {
								let dest = MultiLocation {
									parents: 1,
									interior: X2(
										Parachain(T::VtokenMinting::get_astar_parachain_id()),
										AccountId32 {
											network: None,
											id: receiver.encode().try_into().unwrap(),
										},
									),
								};
								T::XcmTransfer::transfer(
									user_account.clone(),
									currency_id,
									deduct_amount,
									dest,
									Unlimited,
								)?;
							},
							RedeemType::Hydradx(receiver) => {
								let dest = MultiLocation {
									parents: 1,
									interior: X2(
										Parachain(T::VtokenMinting::get_hydradx_parachain_id()),
										AccountId32 {
											network: None,
											id: receiver.encode().try_into().unwrap(),
										},
									),
								};
								T::XcmTransfer::transfer(
									user_account.clone(),
									currency_id,
									deduct_amount,
									dest,
									Unlimited,
								)?;
							},
							RedeemType::Interlay(receiver) => {
								let dest = MultiLocation {
									parents: 1,
									interior: X2(
										Parachain(T::VtokenMinting::get_interlay_parachain_id()),
										AccountId32 {
											network: None,
											id: receiver.encode().try_into().unwrap(),
										},
									),
								};
								T::XcmTransfer::transfer(
									user_account.clone(),
									currency_id,
									deduct_amount,
									dest,
									Unlimited,
								)?;
							},
							RedeemType::Manta(receiver) => {
								let dest = MultiLocation {
									parents: 1,
									interior: X2(
										Parachain(T::VtokenMinting::get_manta_parachain_id()),
										AccountId32 {
											network: None,
											id: receiver.encode().try_into().unwrap(),
										},
									),
								};
								T::XcmTransfer::transfer(
									user_account.clone(),
									currency_id,
									deduct_amount,
									dest,
									Unlimited,
								)?;
							},
							RedeemType::Moonbeam(receiver) => {
								let dest = MultiLocation {
									parents: 1,
									interior: X2(
										Parachain(T::VtokenMinting::get_moonbeam_parachain_id()),
										AccountKey20 {
											network: None,
											key: receiver.to_fixed_bytes(),
										},
									),
								};
								if currency_id == FIL {
									let assets = vec![
										(currency_id, deduct_amount),
										(BNC, T::BifrostSlpx::get_moonbeam_transfer_to_fee()),
									];

									T::XcmTransfer::transfer_multicurrencies(
										user_account.clone(),
										assets,
										1,
										dest,
										Unlimited,
									)?;
								} else {
									T::XcmTransfer::transfer(
										user_account.clone(),
										currency_id,
										deduct_amount,
										dest,
										Unlimited,
									)?;
								}
							},
						};
						// Delete the corresponding unlocking record storage.
						T::VtokenMinting::deduct_unlock_amount(currency_id, *idx, deduct_amount)?;

						extra_weight =
							T::OnRefund::on_refund(currency_id, user_account, deduct_amount);

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

			if extra_weight != 0 {
				Ok(Some(
					<T as Config>::WeightInfo::refund_currency_due_unbond() +
						Weight::from_parts(extra_weight, 0),
				)
				.into())
			} else {
				Ok(().into())
			}
		}

		#[pallet::call_index(19)]
		#[pallet::weight(<T as Config>::WeightInfo::supplement_fee_reserve())]
		pub fn supplement_fee_reserve(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			dest: Box<MultiLocation>,
		) -> DispatchResult {
			// Ensure origin
			Self::ensure_authorized(origin, currency_id)?;

			// Ensure dest is one of delegators accounts, or operators account, or in
			// SupplementFeeAccountWhitelist.
			let mut valid_account = false;

			if DelegatorsMultilocation2Index::<T>::contains_key(currency_id, dest.clone()) {
				valid_account = true;
			}

			if !valid_account {
				let dest_account_id = Self::multilocation_to_account(&dest)?;
				let operate_account_op = OperateOrigins::<T>::get(currency_id);

				if let Some(operate_account) = operate_account_op {
					if dest_account_id == operate_account {
						valid_account = true;
					}
				}
			}

			if !valid_account {
				let white_list_op = SupplementFeeAccountWhitelist::<T>::get(currency_id);

				if let Some(white_list) = white_list_op {
					let multi_hash = T::Hashing::hash(&dest.encode());
					white_list
						.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash)
						.map_err(|_| Error::<T>::DestAccountNotValid)?;

					valid_account = true;
				}
			}

			ensure!(valid_account, Error::<T>::DestAccountNotValid);

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
				staking_agent.supplement_fee_reserve(
					reserved_fee,
					&source_location,
					&dest,
					currency_id,
				)?;
			}

			// Deposit event.
			Pallet::<T>::deposit_event(Event::FeeSupplemented {
				currency_id,
				amount: reserved_fee,
				from: source_location,
				to: *dest,
			});

			Ok(())
		}

		#[pallet::call_index(20)]
		#[pallet::weight(<T as Config>::WeightInfo::charge_host_fee_and_tune_vtoken_exchange_rate())]
		/// Charge staking host fee, tune vtoken/token exchange rate, and update delegator ledger
		/// for single delegator.
		pub fn charge_host_fee_and_tune_vtoken_exchange_rate(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] value: BalanceOf<T>,
			who: Option<MultiLocation>,
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
			if !CurrencyLatestTuneRecord::<T>::contains_key(currency_id) {
				// Insert an empty record into CurrencyLatestTuneRecord storage.
				CurrencyLatestTuneRecord::<T>::insert(currency_id, (current_time_unit.clone(), 0));
			}

			// Get CurrencyLatestTuneRecord for the currencyId.
			let (latest_time_unit, tune_num) =
				Self::get_currency_latest_tune_record(currency_id)
					.ok_or(Error::<T>::CurrencyLatestTuneRecordNotExist)?;

			// See if exceeds tuning limit.
			// If it has been tuned in the current time unit, ensure this tuning is within limit.
			let mut new_tune_num = Zero::zero();
			if latest_time_unit == current_time_unit {
				ensure!(tune_num < limit_num, Error::<T>::GreaterThanMaximum);
				new_tune_num = tune_num;
			}

			new_tune_num = new_tune_num.checked_add(1).ok_or(Error::<T>::OverFlow)?;

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
				currency_id,
			)?;

			// Tune the new exchange rate.
			staking_agent.tune_vtoken_exchange_rate(
				&who,
				value,
				// Dummy value for vtoken amount
				Zero::zero(),
				currency_id,
			)?;

			// Update the CurrencyLatestTuneRecord<T> storage.
			CurrencyLatestTuneRecord::<T>::insert(currency_id, (current_time_unit, new_tune_num));

			T::ChannelCommission::record_hosting_fee(currency_id, fee_to_charge)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::HostingFeeCharged {
				currency_id,
				amount: fee_to_charge,
			});
			Pallet::<T>::deposit_event(Event::PoolTokenIncreased { currency_id, amount: value });
			Ok(())
		}

		/// *****************************
		/// ****** Storage Setters ******
		/// *****************************

		/// Update storage OperateOrigins<T>.
		#[pallet::call_index(22)]
		#[pallet::weight(<T as Config>::WeightInfo::set_operate_origin())]
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
		#[pallet::call_index(23)]
		#[pallet::weight(<T as Config>::WeightInfo::set_fee_source())]
		pub fn set_fee_source(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who_and_fee: Option<(MultiLocation, BalanceOf<T>)>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			FeeSources::<T>::mutate_exists(currency_id, |w_n_f| {
				*w_n_f = who_and_fee;
			});

			// Deposit event.
			Pallet::<T>::deposit_event(Event::FeeSourceSet { currency_id, who_and_fee });

			Ok(())
		}

		/// Update storage DelegatorsIndex2Multilocation<T> 和 DelegatorsMultilocation2Index<T>.
		#[pallet::call_index(24)]
		#[pallet::weight(<T as Config>::WeightInfo::add_delegator())]
		pub fn add_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			#[pallet::compact] index: u16,
			who: Box<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			Pallet::<T>::inner_add_delegator(index, &who, currency_id)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorAdded {
				currency_id,
				index,
				delegator_id: *who,
			});
			Ok(())
		}

		/// Update storage DelegatorsIndex2Multilocation<T> 和 DelegatorsMultilocation2Index<T>.
		#[pallet::call_index(25)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_delegator())]
		pub fn remove_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let staking_agent = Self::get_currency_staking_agent(currency_id)?;
			staking_agent.remove_delegator(&who, currency_id)?;

			// Deposit event.
			Pallet::<T>::deposit_event(Event::DelegatorRemoved { currency_id, delegator_id: *who });
			Ok(())
		}

		/// Update storage Validators<T>.
		#[pallet::call_index(26)]
		#[pallet::weight(<T as Config>::WeightInfo::add_validator())]
		pub fn add_validator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			if currency_id == PHA {
				if let &MultiLocation {
					parents: vault_or_stake_pool,
					interior: X2(GeneralIndex(_pool_id), GeneralIndex(_collection_id)),
				} = who.as_ref()
				{
					ensure!(
						vault_or_stake_pool == 0 || vault_or_stake_pool == 1,
						Error::<T>::ValidatorMultilocationNotvalid
					);
					Pallet::<T>::inner_add_validator(&who, currency_id)?;
				} else {
					Err(Error::<T>::ValidatorMultilocationNotvalid)?;
				}
			} else {
				Pallet::<T>::inner_add_validator(&who, currency_id)?;
			}

			Ok(())
		}

		/// Update storage Validators<T>.
		#[pallet::call_index(27)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_validator())]
		pub fn remove_validator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			Pallet::<T>::inner_remove_validator(&who, currency_id)?;

			Ok(())
		}

		/// Update storage ValidatorsByDelegator<T>.
		#[pallet::call_index(28)]
		#[pallet::weight(<T as Config>::WeightInfo::set_validators_by_delegator())]
		pub fn set_validators_by_delegator(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
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

			// ensure the length of validators does not exceed MaxLengthLimit
			ensure!(
				validators.len() <= T::MaxLengthLimit::get() as usize,
				Error::<T>::ExceedMaxLengthLimit
			);

			// check delegator
			// Check if it is bonded already.
			ensure!(
				DelegatorLedgers::<T>::contains_key(currency_id, who.clone()),
				Error::<T>::DelegatorNotBonded
			);

			let validators_list = Self::remove_validators_duplicates(currency_id, &validators)?;

			let bounded_validators = BoundedVec::try_from(validators_list.clone())
				.map_err(|_| Error::<T>::FailToConvert)?;

			// Update ValidatorsByDelegator storage
			ValidatorsByDelegator::<T>::insert(
				currency_id,
				who.clone(),
				bounded_validators.clone(),
			);

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsByDelegatorSet {
				currency_id,
				validators_list,
				delegator_id: *who,
			});

			Ok(())
		}

		/// Update storage DelegatorLedgers<T>.
		#[pallet::call_index(29)]
		#[pallet::weight(<T as Config>::WeightInfo::set_delegator_ledger())]
		pub fn set_delegator_ledger(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
			ledger: Box<Option<Ledger<BalanceOf<T>>>>,
		) -> DispatchResult {
			// Check the validity of origin
			Self::ensure_authorized(origin, currency_id)?;

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
		#[pallet::call_index(30)]
		#[pallet::weight(<T as Config>::WeightInfo::set_minimums_and_maximums())]
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
		#[pallet::call_index(31)]
		#[pallet::weight(<T as Config>::WeightInfo::set_currency_delays())]
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
		#[pallet::call_index(32)]
		#[pallet::weight(<T as Config>::WeightInfo::set_hosting_fees())]
		pub fn set_hosting_fees(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			maybe_fee_set: Option<(Permill, MultiLocation)>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			HostingFees::<T>::mutate_exists(currency_id, |fee_set| {
				*fee_set = maybe_fee_set;
			});

			Pallet::<T>::deposit_event(Event::HostingFeesSet { currency_id, fees: maybe_fee_set });

			Ok(())
		}

		/// Set  CurrencyTuneExchangeRateLimit<T> storage.
		#[pallet::call_index(33)]
		#[pallet::weight(<T as Config>::WeightInfo::set_currency_tune_exchange_rate_limit())]
		pub fn set_currency_tune_exchange_rate_limit(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			maybe_tune_exchange_rate_limit: Option<(u32, Permill)>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			CurrencyTuneExchangeRateLimit::<T>::mutate_exists(currency_id, |exchange_rate_limit| {
				*exchange_rate_limit = maybe_tune_exchange_rate_limit;
			});

			Pallet::<T>::deposit_event(Event::CurrencyTuneExchangeRateLimitSet {
				currency_id,
				tune_exchange_rate_limit: maybe_tune_exchange_rate_limit,
			});

			Ok(())
		}

		/// Set  OngoingTimeUnitUpdateInterval<T> storage.
		#[pallet::call_index(34)]
		#[pallet::weight(<T as Config>::WeightInfo::set_ongoing_time_unit_update_interval())]
		pub fn set_ongoing_time_unit_update_interval(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			maybe_interval: Option<BlockNumberFor<T>>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			if maybe_interval.is_none() {
				LastTimeUpdatedOngoingTimeUnit::<T>::remove(currency_id);
			} else {
				// if this is the first time to set interval, add an item to
				// LastTimeUpdatedOngoingTimeUnit
				if !OngoingTimeUnitUpdateInterval::<T>::contains_key(currency_id) {
					let zero_block = BlockNumberFor::<T>::from(0u32);
					LastTimeUpdatedOngoingTimeUnit::<T>::insert(currency_id, zero_block);
				}
			}

			OngoingTimeUnitUpdateInterval::<T>::mutate_exists(currency_id, |interval_op| {
				*interval_op = maybe_interval;
			});

			Pallet::<T>::deposit_event(Event::OngoingTimeUnitUpdateIntervalSet {
				currency_id,
				interval: maybe_interval,
			});

			Ok(())
		}

		// Add an account to SupplementFeeAccountWhitelist
		#[pallet::call_index(35)]
		#[pallet::weight(<T as Config>::WeightInfo::add_supplement_fee_account_to_whitelist())]
		pub fn add_supplement_fee_account_to_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let multi_hash = T::Hashing::hash(&who.encode());
			if !SupplementFeeAccountWhitelist::<T>::contains_key(&currency_id) {
				SupplementFeeAccountWhitelist::<T>::insert(
					currency_id,
					vec![(who.clone(), multi_hash)],
				);
			} else {
				SupplementFeeAccountWhitelist::<T>::mutate_exists(
					currency_id,
					|whitelist_op| -> Result<(), Error<T>> {
						if let Some(whitelist) = whitelist_op {
							let rs =
								whitelist.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash);
							if let Err(idx) = rs {
								whitelist.insert(idx, (*who.clone(), multi_hash));
							} else {
								Err(Error::<T>::AlreadyExist)?;
							}
						} else {
							Err(Error::<T>::Unexpected)?;
						}

						Ok(())
					},
				)?;
			}

			Pallet::<T>::deposit_event(Event::SupplementFeeAccountWhitelistAdded {
				currency_id,
				who: *who,
			});

			Ok(())
		}

		// Add an account to SupplementFeeAccountWhitelist
		#[pallet::call_index(36)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_supplement_fee_account_from_whitelist())]
		pub fn remove_supplement_fee_account_from_whitelist(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let multi_hash = T::Hashing::hash(&who.encode());
			if !SupplementFeeAccountWhitelist::<T>::contains_key(&currency_id) {
				Err(Error::<T>::WhiteListNotExist)?;
			} else {
				SupplementFeeAccountWhitelist::<T>::mutate_exists(
					currency_id,
					|whitelist_op| -> Result<(), Error<T>> {
						if let Some(whitelist) = whitelist_op {
							let rs =
								whitelist.binary_search_by_key(&multi_hash, |(_multi, hash)| *hash);
							if let Ok(idx) = rs {
								whitelist.remove(idx);
							} else {
								Err(Error::<T>::AccountNotExist)?;
							}
						} else {
							Err(Error::<T>::Unexpected)?;
						}

						Ok(())
					},
				)?;
			}

			Pallet::<T>::deposit_event(Event::SupplementFeeAccountWhitelistRemoved {
				currency_id,
				who: *who,
			});

			Ok(())
		}

		/// ********************************************************************
		/// *************Outer Confirming Xcm queries functions ****************
		/// ********************************************************************
		#[pallet::call_index(37)]
		#[pallet::weight(<T as Config>::WeightInfo::confirm_delegator_ledger_query_response())]
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

		#[pallet::call_index(38)]
		#[pallet::weight(<T as Config>::WeightInfo::fail_delegator_ledger_query_response())]
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

		#[pallet::call_index(39)]
		#[pallet::weight(<T as Config>::WeightInfo::confirm_validators_by_delegator_query_response())]
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

		#[pallet::call_index(40)]
		#[pallet::weight(<T as Config>::WeightInfo::fail_validators_by_delegator_query_response())]
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

		#[pallet::call_index(41)]
		#[pallet::weight(<T as Config>::WeightInfo::confirm_delegator_ledger_query_response())]
		pub fn confirm_delegator_ledger(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			// Ensure origin
			ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;
			if let Response::DispatchResult(MaybeErrorCode::Success) = response {
				Self::get_ledger_update_agent_then_process(query_id, true)?;
			} else {
				Self::do_fail_delegator_ledger_query_response(query_id)?;
			}
			Ok(())
		}

		#[pallet::call_index(42)]
		#[pallet::weight(<T as Config>::WeightInfo::confirm_validators_by_delegator_query_response())]
		pub fn confirm_validators_by_delegator(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			// Ensure origin
			ensure_response(<T as Config>::RuntimeOrigin::from(origin))?;
			if let Response::DispatchResult(MaybeErrorCode::Success) = response {
				Self::get_validators_by_delegator_update_agent_then_process(query_id, true)?;
			} else {
				Self::do_fail_validators_by_delegator_query_response(query_id)?;
			}
			Ok(())
		}

		/// Reset the whole storage Validators<T>.
		#[pallet::call_index(43)]
		#[pallet::weight(<T as Config>::WeightInfo::reset_validators())]
		pub fn reset_validators(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			validator_list: Vec<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let validator_set =
				Pallet::<T>::check_length_and_deduplicate(currency_id, validator_list)?;

			let bounded_validators =
				BoundedVec::<MultiLocation, T::MaxLengthLimit>::try_from(validator_set.clone())
					.map_err(|_| Error::<T>::FailToConvert)?;

			// Change corresponding storage.
			Validators::<T>::insert(currency_id, bounded_validators);

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsReset {
				currency_id,
				validator_list: validator_set,
			});
			Ok(())
		}

		/// Reset the whole storage Validator_boost_list<T>.
		#[pallet::call_index(44)]
		#[pallet::weight(<T as Config>::WeightInfo::set_validator_boost_list())]
		pub fn set_validator_boost_list(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			validator_list: Vec<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			let validator_set =
				Pallet::<T>::check_length_and_deduplicate(currency_id, validator_list)?;

			// get current block number
			let current_block_number = <frame_system::Pallet<T>>::block_number();
			// get the due block number
			let due_block_number = current_block_number
				.checked_add(&BlockNumberFor::<T>::from(SIX_MONTHS))
				.ok_or(Error::<T>::OverFlow)?;

			let mut validator_boost_list: Vec<(MultiLocation, BlockNumberFor<T>)> = vec![];

			for validator in validator_set.iter() {
				validator_boost_list.push((*validator, due_block_number));
			}

			let bounded_validator_boost_list = BoundedVec::<
				(MultiLocation, BlockNumberFor<T>),
				T::MaxLengthLimit,
			>::try_from(validator_boost_list.clone())
			.map_err(|_| Error::<T>::FailToConvert)?;

			// Change corresponding storage.
			ValidatorBoostList::<T>::insert(currency_id, bounded_validator_boost_list);

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorBoostListSet {
				currency_id,
				validator_boost_list: validator_boost_list.clone(),
			});

			// Add the boost list to the validator set
			let mut validator_vec;
			if let Some(validator_set) = Self::get_validators(currency_id) {
				validator_vec = validator_set.to_vec();
			} else {
				validator_vec = vec![];
			}

			for (validator, _) in validator_boost_list.iter() {
				if !validator_vec.contains(validator) {
					validator_vec.push(*validator);
				}
			}

			ensure!(
				validator_vec.len() <= T::MaxLengthLimit::get() as usize,
				Error::<T>::ExceedMaxLengthLimit
			);

			let bounded_validator_set: BoundedVec<MultiLocation, T::MaxLengthLimit> =
				BoundedVec::try_from(validator_vec.clone())
					.map_err(|_| Error::<T>::FailToConvert)?;

			Validators::<T>::insert(currency_id, bounded_validator_set);

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsReset {
				currency_id,
				validator_list: validator_vec,
			});

			Ok(())
		}

		#[pallet::call_index(45)]
		#[pallet::weight(<T as Config>::WeightInfo::add_to_validator_boost_list())]
		pub fn add_to_validator_boost_list(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			// get current block number
			let current_block_number = <frame_system::Pallet<T>>::block_number();

			// get the due block number if the validator is not in the validator boost list
			let mut due_block_number = current_block_number
				.checked_add(&BlockNumberFor::<T>::from(SIX_MONTHS))
				.ok_or(Error::<T>::OverFlow)?;

			let validator_boost_list_op = ValidatorBoostList::<T>::get(currency_id);

			let mut validator_boost_vec;
			if let Some(validator_boost_list) = validator_boost_list_op {
				// if the validator is in the validator boost list, change the due block
				// number
				validator_boost_vec = validator_boost_list.to_vec();
				if let Some(index) =
					validator_boost_vec.iter().position(|(validator, _)| validator == who.as_ref())
				{
					let original_due_block = validator_boost_vec[index].1;
					// get the due block number
					due_block_number = original_due_block
						.checked_add(&BlockNumberFor::<T>::from(SIX_MONTHS))
						.ok_or(Error::<T>::OverFlow)?;

					validator_boost_vec[index].1 = due_block_number;
				} else {
					validator_boost_vec.push((*who, due_block_number));
				}
			} else {
				validator_boost_vec = vec![(*who, due_block_number)];
			}

			// ensure the length of the validator boost list is less than the maximum
			ensure!(
				validator_boost_vec.len() <= T::MaxLengthLimit::get() as usize,
				Error::<T>::ExceedMaxLengthLimit
			);

			let bounded_list =
				BoundedVec::<(MultiLocation, BlockNumberFor<T>), T::MaxLengthLimit>::try_from(
					validator_boost_vec,
				)
				.map_err(|_| Error::<T>::FailToConvert)?;

			// Change corresponding storage.
			ValidatorBoostList::<T>::insert(currency_id, bounded_list);

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorBoostListAdded {
				currency_id,
				who: *who,
				due_block_number,
			});

			let validator_set_op = Self::get_validators(currency_id);

			let mut validator_vec;
			// Add the newly added validator to the validator set
			if let Some(validator_set) = validator_set_op {
				validator_vec = validator_set.to_vec();
				if !validator_vec.contains(who.as_ref()) {
					validator_vec.push(*who);
				}
			} else {
				validator_vec = vec![*who];
			}

			// ensure the length of the validator set is less than the maximum
			ensure!(
				validator_vec.len() <= T::MaxLengthLimit::get() as usize,
				Error::<T>::ExceedMaxLengthLimit
			);

			let bouded_list =
				BoundedVec::<MultiLocation, T::MaxLengthLimit>::try_from(validator_vec.clone())
					.map_err(|_| Error::<T>::FailToConvert)?;

			// Change corresponding storage.
			Validators::<T>::insert(currency_id, bouded_list);

			// Deposit event.
			Pallet::<T>::deposit_event(Event::ValidatorsReset {
				currency_id,
				validator_list: validator_vec,
			});

			Ok(())
		}

		/// Update storage Validator_boost_list<T>.
		#[pallet::call_index(46)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_from_validator_boot_list())]
		pub fn remove_from_validator_boot_list(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			who: Box<MultiLocation>,
		) -> DispatchResult {
			// Check the validity of origin
			T::ControlOrigin::ensure_origin(origin)?;

			// check if the validator is in the validator boost list
			ValidatorBoostList::<T>::mutate(currency_id, |validator_boost_list_op| {
				if let Some(ref mut validator_boost_list) = validator_boost_list_op {
					// if the validator is in the validator boost list, remove it
					if let Some(index) = validator_boost_list
						.iter()
						.position(|(validator, _)| validator == who.as_ref())
					{
						validator_boost_list.remove(index);

						// if the validator boost list is empty, remove it
						if validator_boost_list.is_empty() {
							*validator_boost_list_op = None;
						}

						// Deposit event.
						Pallet::<T>::deposit_event(Event::RemovedFromBoostList {
							currency_id,
							who: *who,
						});
					}
				}
			});

			Ok(())
		}

		#[pallet::call_index(47)]
		#[pallet::weight(<T as Config>::WeightInfo::convert_treasury_vtoken())]
		pub fn convert_treasury_vtoken(
			origin: OriginFor<T>,
			vtoken: CurrencyId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			ensure!(amount > Zero::zero(), Error::<T>::AmountZero);

			let token = vtoken.to_token().map_err(|_| Error::<T>::NotSupportedCurrencyId)?;
			let (pool_id, _, _) = T::StablePoolHandler::get_pool_id(&vtoken, &token)
				.ok_or(Error::<T>::StablePoolNotFound)?;

			let vtoken_index = T::StablePoolHandler::get_pool_token_index(pool_id, vtoken)
				.ok_or(Error::<T>::StablePoolTokenIndexNotFound)?;
			let token_index = T::StablePoolHandler::get_pool_token_index(pool_id, token)
				.ok_or(Error::<T>::StablePoolTokenIndexNotFound)?;

			// get the vtoken balance of the treasury account
			let source_vtoken_balance =
				T::MultiCurrency::free_balance(vtoken, &T::TreasuryAccount::get());

			// max_amount is 1% of the vtoken balance of the treasury account
			let percentage = Permill::from_percent(1);
			let max_amount = percentage.mul_floor(source_vtoken_balance);

			ensure!(
				amount <= BalanceOf::<T>::unique_saturated_from(max_amount),
				Error::<T>::ExceedLimit
			);

			// swap vtoken from treasury account for token
			let treasury = T::TreasuryAccount::get();
			T::StablePoolHandler::swap(
				&treasury,
				pool_id,
				vtoken_index,
				token_index,
				amount,
				Zero::zero(),
			)
		}

		#[pallet::call_index(48)]
		#[pallet::weight(<T as Config>::WeightInfo::clean_outdated_validator_boost_list())]
		pub fn clean_outdated_validator_boost_list(
			origin: OriginFor<T>,
			token: CurrencyId,
			// start from 1
			page: u8,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let page = page as usize;
			ensure!(page > 0, Error::<T>::InvalidPageNumber);

			let validator_boost_list_len = ValidatorBoostList::<T>::decode_len(token)
				.ok_or(Error::<T>::NoMoreValidatorBoostListForCurrency)?;

			let previous_count = (page - 1) * ITERATE_LENGTH;
			ensure!(
				validator_boost_list_len > previous_count,
				Error::<T>::NoMoreValidatorBoostListForCurrency
			);

			// calculate next page number left
			let num_left = if validator_boost_list_len > (previous_count + ITERATE_LENGTH) {
				validator_boost_list_len - previous_count - ITERATE_LENGTH
			} else {
				0
			};

			let current_block_number = <frame_system::Pallet<T>>::block_number();
			let mut remove_num = 0;
			// for each validator in the validator boost list, if the due block number is less than
			// or equal to the current block number, remove it
			ValidatorBoostList::<T>::mutate(token, |validator_boost_list_op| {
				if let Some(ref mut validator_boost_list) = validator_boost_list_op {
					let mut remove_index = vec![];
					for (index, (_validator, due_block_number)) in validator_boost_list
						.iter()
						.skip(previous_count)
						.take(ITERATE_LENGTH)
						.enumerate()
					{
						if *due_block_number <= current_block_number {
							remove_index.push(index + previous_count);
						}
					}

					// remove from the end to the start
					for index in remove_index.iter().rev() {
						validator_boost_list.remove(*index);
						remove_num += 1;
					}
				}
			});

			// Deposit event.
			Pallet::<T>::deposit_event(Event::OutdatedValidatorBoostListCleaned {
				currency_id: token,
				page: page as u8,
				remove_num,
				num_left: num_left as u32,
			});

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
				_ => {
					T::ControlOrigin::ensure_origin(origin)
						.map_err(|_| Error::<T>::NotAuthorized)?;
					Ok(())
				},
			}
		}

		pub(crate) fn get_currency_staking_agent(
			currency_id: CurrencyId,
		) -> Result<StakingAgentBoxType<T>, Error<T>> {
			match currency_id {
				KSM | DOT => Ok(Box::new(PolkadotAgent::<T>::new())),
				BNC | MOVR | GLMR | MANTA => Ok(Box::new(ParachainStakingAgent::<T>::new())),
				FIL => Ok(Box::new(FilecoinAgent::<T>::new())),
				PHA => Ok(Box::new(PhalaAgent::<T>::new())),
				ASTR => Ok(Box::new(AstarAgent::<T>::new())),
				_ => Err(Error::<T>::NotSupportedCurrencyId),
			}
		}

		pub fn confirm_delegator_ledger_call() -> <T as Config>::RuntimeCall {
			let call =
				Call::<T>::confirm_delegator_ledger { query_id: 0, response: Default::default() };
			<T as Config>::RuntimeCall::from(call)
		}

		pub fn confirm_validators_by_delegator_call() -> <T as Config>::RuntimeCall {
			let call = Call::<T>::confirm_validators_by_delegator {
				query_id: 0,
				response: Default::default(),
			};
			<T as Config>::RuntimeCall::from(call)
		}
	}

	// Functions to be called by other pallets.
	impl<T: Config> SlpOperator<CurrencyId> for Pallet<T> {
		fn all_delegation_requests_occupied(currency_id: CurrencyId) -> bool {
			DelegationsOccupied::<T>::get(currency_id).unwrap_or_default()
		}
	}
}

pub struct DerivativeAccountProvider<T, F>(PhantomData<(T, F)>);

impl<T: Config, F: Contains<CurrencyIdOf<T>>>
	DerivativeAccountHandler<CurrencyIdOf<T>, BalanceOf<T>> for DerivativeAccountProvider<T, F>
{
	fn check_derivative_index_exists(
		token: CurrencyIdOf<T>,
		derivative_index: DerivativeIndex,
	) -> bool {
		Pallet::<T>::get_delegator_multilocation_by_index(token, derivative_index).is_some()
	}

	fn get_multilocation(
		token: CurrencyIdOf<T>,
		derivative_index: DerivativeIndex,
	) -> Option<MultiLocation> {
		Pallet::<T>::get_delegator_multilocation_by_index(token, derivative_index)
	}

	fn get_stake_info(
		token: CurrencyIdOf<T>,
		derivative_index: DerivativeIndex,
	) -> Option<(BalanceOf<T>, BalanceOf<T>)> {
		Self::get_multilocation(token, derivative_index).and_then(|location| {
			Pallet::<T>::get_delegator_ledger(token, location).and_then(|ledger| match ledger {
				Ledger::Substrate(l) if F::contains(&token) => Some((l.total, l.active)),
				_ => None,
			})
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn init_minimums_and_maximums(currency_id: CurrencyIdOf<T>) {
		MinimumsAndMaximums::<T>::insert(
			currency_id,
			MinimumsMaximums {
				delegator_bonded_minimum: 0u32.into(),
				bond_extra_minimum: 0u32.into(),
				unbond_minimum: 0u32.into(),
				rebond_minimum: 0u32.into(),
				unbond_record_maximum: 0u32,
				validators_back_maximum: 0u32,
				delegator_active_staking_maximum: 0u32.into(),
				validators_reward_maximum: 0u32,
				delegation_amount_minimum: 0u32.into(),
				delegators_maximum: u16::MAX,
				validators_maximum: 0u16,
			},
		);
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn new_delegator_ledger(currency_id: CurrencyIdOf<T>, who: MultiLocation) {
		DelegatorLedgers::<T>::insert(
			currency_id,
			&who,
			Ledger::Substrate(SubstrateLedger {
				account: Parent.into(),
				total: u32::MAX.into(),
				active: u32::MAX.into(),
				unlocking: vec![],
			}),
		);
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn add_delegator(currency_id: CurrencyIdOf<T>, index: DerivativeIndex, who: MultiLocation) {
		Pallet::<T>::inner_add_delegator(index, &who, currency_id).unwrap();
	}
}

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

//! # LendMarket pallet
//!
//! ## Overview
//!
//! LendMarket pallet implement the lending protocol by using a pool-based strategy
//! that aggregates each user's supplied assets. The interest rate is dynamically
//! determined by the supply and demand.

#![cfg_attr(not(feature = "std"), no_std)]

use core::cmp::max;

pub use crate::rate_model::*;
use bifrost_primitives::{
	Balance, CurrencyId, Liquidity, OraclePriceProvider, Price, Rate, Ratio, Shortfall, Timestamp,
};
use frame_support::{
	pallet_prelude::*,
	require_transactional,
	traits::{
		fungibles::{Inspect, Mutate},
		tokens::{Fortitude, Preservation},
		UnixTime,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use num_traits::cast::ToPrimitive;
pub use pallet::*;
use pallet_traits::{
	ConvertToBigUint, LendMarket as LendMarketTrait, LendMarketMarketDataProvider,
	LendMarketPositionDataProvider, MarketInfo, MarketStatus,
};
use sp_core::bounded::BoundedVec;
use sp_runtime::{
	traits::{
		AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One,
		SaturatedConversion, Saturating, StaticLookup, Zero,
	},
	ArithmeticError, FixedPointNumber, FixedU128,
};
use sp_std::{result::Result, vec::Vec};

use log;
use sp_io::hashing::blake2_256;
pub use types::{BorrowSnapshot, Deposits, EarnedSnapshot, Market, MarketState, RewardMarketState};
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod migrations;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

mod farming;
mod interest;
mod lend_token;
mod rate_model;
mod types;

pub mod weights;

pub const MAX_EXCHANGE_RATE: u128 = 1_000_000_000_000_000_000; // 1
pub const MIN_EXCHANGE_RATE: u128 = 20_000_000_000_000_000; // 0.02

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type AssetIdOf<T> =
	<<T as Config>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::AssetId;
pub type BalanceOf<T> =
	<<T as Config>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The oracle price feeder
		type OraclePriceProvider: OraclePriceProvider;

		/// The loan's module id, keep all collaterals of CDPs.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The origin which can add/reduce reserves.
		type ReserveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The origin which can update rate model, liquidate incentive and
		/// add/reduce reserves. Root can always do this.
		type UpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// Unix time
		type UnixTime: UnixTime;

		/// Assets for deposit/withdraw collateral assets to/from lend-market module
		type Assets: Inspect<Self::AccountId, AssetId = CurrencyId, Balance = Balance>
			+ Mutate<Self::AccountId, AssetId = CurrencyId, Balance = Balance>;

		/// Reward asset id.
		#[pallet::constant]
		type RewardAssetId: Get<AssetIdOf<Self>>;

		#[pallet::constant]
		type LiquidationFreeAssetId: Get<AssetIdOf<Self>>;

		#[pallet::constant]
		type MaxLengthLimit: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient liquidity to borrow more or disable collateral
		InsufficientLiquidity,
		/// Insufficient deposit to redeem
		InsufficientDeposit,
		/// Repay amount greater than allowed
		TooMuchRepay,
		/// Asset already enabled/disabled collateral
		DuplicateOperation,
		/// No deposit asset
		NoDeposit,
		/// Repay amount more than collateral amount
		InsufficientCollateral,
		/// Liquidator is same as borrower
		LiquidatorIsBorrower,
		/// Deposits are not used as a collateral
		DepositsAreNotCollateral,
		/// Insufficient shortfall to repay
		InsufficientShortfall,
		/// Insufficient reserves
		InsufficientReserves,
		/// Invalid rate model params
		InvalidRateModelParam,
		/// Market not activated
		MarketNotActivated,
		/// Oracle price not ready
		PriceOracleNotReady,
		/// Oracle price is zero
		PriceIsZero,
		/// Invalid asset id
		InvalidCurrencyId,
		/// Invalid lend token id
		InvalidLendTokenId,
		/// Market does not exist
		MarketDoesNotExist,
		/// Market already exists
		MarketAlreadyExists,
		/// New markets must have a pending state
		NewMarketMustHavePendingState,
		/// Upper bound of supplying is exceeded
		SupplyCapacityExceeded,
		/// Upper bound of borrowing is exceeded
		BorrowCapacityExceeded,
		/// Insufficient cash in the pool
		InsufficientCash,
		/// The factor should be greater than 0% and less than 100%
		InvalidFactor,
		/// The supply cap cannot be zero
		InvalidSupplyCap,
		/// The exchange rate should be greater than 0.02 and less than 1
		InvalidExchangeRate,
		/// Amount cannot be zero
		InvalidAmount,
		/// Payer cannot be signer
		PayerIsSigner,
		/// Codec error
		CodecError,
		/// Collateral is reserved and cannot be liquidated
		CollateralReserved,
		/// Market bond does not exist
		MarketBondDoesNotExist,
		/// Error converting Vec to BoundedVec.
		ConversionError,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Enable collateral for certain asset
		/// [sender, asset_id]
		CollateralAssetAdded(T::AccountId, AssetIdOf<T>),
		/// Disable collateral for certain asset
		/// [sender, asset_id]
		CollateralAssetRemoved(T::AccountId, AssetIdOf<T>),
		/// Event emitted when assets are deposited
		/// [sender, asset_id, amount]
		Deposited(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
		/// Event emitted when assets are redeemed
		/// [sender, asset_id, amount]
		Redeemed(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
		/// Event emitted when cash is borrowed
		/// [sender, asset_id, amount]
		Borrowed(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
		/// Event emitted when a borrow is repaid
		/// [sender, asset_id, amount]
		RepaidBorrow(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
		/// Event emitted when a borrow is liquidated
		/// [liquidator, borrower, liquidation_asset_id, collateral_asset_id, repay_amount,
		/// collateral_amount]
		LiquidatedBorrow(
			T::AccountId,
			T::AccountId,
			AssetIdOf<T>,
			AssetIdOf<T>,
			BalanceOf<T>,
			BalanceOf<T>,
		),
		/// Event emitted when the reserves are reduced
		/// [admin, asset_id, reduced_amount, total_reserves]
		ReservesReduced(T::AccountId, AssetIdOf<T>, BalanceOf<T>, BalanceOf<T>),
		/// Event emitted when the reserves are added
		/// [admin, asset_id, added_amount, total_reserves]
		ReservesAdded(T::AccountId, AssetIdOf<T>, BalanceOf<T>, BalanceOf<T>),
		/// New market is set
		/// [new_interest_rate_model]
		NewMarket(AssetIdOf<T>, Market<BalanceOf<T>>),
		/// Event emitted when a market is activated
		/// [admin, asset_id]
		ActivatedMarket(AssetIdOf<T>),
		/// New market parameters is updated
		/// [admin, asset_id]
		UpdatedMarket(AssetIdOf<T>, Market<BalanceOf<T>>),
		/// Reward added
		RewardAdded(T::AccountId, BalanceOf<T>),
		/// Reward withdrawed
		RewardWithdrawn(T::AccountId, BalanceOf<T>),
		/// Event emitted when market reward speed updated.
		MarketRewardSpeedUpdated(AssetIdOf<T>, BalanceOf<T>, BalanceOf<T>),
		/// Deposited when Reward is distributed to a supplier
		DistributedSupplierReward(AssetIdOf<T>, T::AccountId, BalanceOf<T>, BalanceOf<T>),
		/// Deposited when Reward is distributed to a borrower
		DistributedBorrowerReward(AssetIdOf<T>, T::AccountId, BalanceOf<T>, BalanceOf<T>),
		/// Reward Paid for user
		RewardPaid(T::AccountId, BalanceOf<T>),
		/// Event emitted when the incentive reserves are redeemed and transfer to receiver's
		/// account [receive_account_id, asset_id, reduced_amount]
		IncentiveReservesReduced(T::AccountId, AssetIdOf<T>, BalanceOf<T>),
		/// Liquidation free collaterals has been updated
		LiquidationFreeCollateralsUpdated(Vec<AssetIdOf<T>>),
		MarketBonded {
			asset_id: AssetIdOf<T>,
			market_bond: Vec<AssetIdOf<T>>,
		},
	}

	/// The timestamp of the last calculation of accrued interest
	#[pallet::storage]
	pub type LastAccruedInterestTime<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Timestamp, ValueQuery>;

	/// Liquidation free collateral.
	#[pallet::storage]
	pub type LiquidationFreeCollaterals<T: Config> =
		StorageValue<_, BoundedVec<AssetIdOf<T>, T::MaxLengthLimit>, ValueQuery>;

	/// Total number of collateral tokens in circulation
	/// CollateralType -> Balance
	#[pallet::storage]
	pub type TotalSupply<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// Total amount of outstanding borrows of the underlying in this market
	/// CurrencyId -> Balance
	#[pallet::storage]
	pub type TotalBorrows<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// Total amount of reserves of the underlying held in this market
	/// CurrencyId -> Balance
	#[pallet::storage]
	pub type TotalReserves<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// Mapping of account addresses to outstanding borrow balances
	/// CurrencyId -> Owner -> BorrowSnapshot
	#[pallet::storage]
	pub type AccountBorrows<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetIdOf<T>,
		Blake2_128Concat,
		T::AccountId,
		BorrowSnapshot<BalanceOf<T>>,
		ValueQuery,
	>;

	/// Mapping of account addresses to deposit details
	/// CollateralType -> Owner -> Deposits
	#[pallet::storage]
	pub type AccountDeposits<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetIdOf<T>,
		Blake2_128Concat,
		T::AccountId,
		Deposits<BalanceOf<T>>,
		ValueQuery,
	>;

	/// Mapping of account addresses to total deposit interest accrual
	/// CurrencyId -> Owner -> EarnedSnapshot
	#[pallet::storage]
	pub type AccountEarned<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetIdOf<T>,
		Blake2_128Concat,
		T::AccountId,
		EarnedSnapshot<BalanceOf<T>>,
		ValueQuery,
	>;

	/// Accumulator of the total earned interest rate since the opening of the market
	/// CurrencyId -> u128
	#[pallet::storage]
	pub type BorrowIndex<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Rate, ValueQuery>;

	/// The exchange rate from the underlying to the internal collateral
	#[pallet::storage]
	pub type ExchangeRate<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Rate, ValueQuery>;

	/// Mapping of borrow rate to currency type
	#[pallet::storage]
	pub type BorrowRate<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Rate, ValueQuery>;

	/// Mapping of supply rate to currency type
	#[pallet::storage]
	pub type SupplyRate<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Rate, ValueQuery>;

	/// Borrow utilization ratio
	#[pallet::storage]
	pub type UtilizationRatio<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Ratio, ValueQuery>;

	/// Mapping of asset id to its market
	#[pallet::storage]
	pub type Markets<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, Market<BalanceOf<T>>>;

	/// Mapping of lend token id to asset id
	/// `lend token id`: voucher token id
	/// `asset id`: underlying token id
	#[pallet::storage]
	pub type UnderlyingAssetId<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, AssetIdOf<T>>;

	/// Mapping of token id to supply reward speed
	#[pallet::storage]
	pub type RewardSupplySpeed<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// Mapping of token id to borrow reward speed
	#[pallet::storage]
	pub type RewardBorrowSpeed<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// The Reward market supply state for each market
	#[pallet::storage]
	pub type RewardSupplyState<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AssetIdOf<T>,
		RewardMarketState<BlockNumberFor<T>, BalanceOf<T>>,
		ValueQuery,
	>;

	/// The Reward market borrow state for each market
	#[pallet::storage]
	pub type RewardBorrowState<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AssetIdOf<T>,
		RewardMarketState<BlockNumberFor<T>, BalanceOf<T>>,
		ValueQuery,
	>;

	///  The Reward index for each market for each supplier as of the last time they accrued Reward
	#[pallet::storage]
	pub type RewardSupplierIndex<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetIdOf<T>,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T>,
		ValueQuery,
	>;

	///  The Reward index for each market for each borrower as of the last time they accrued Reward
	#[pallet::storage]
	pub type RewardBorrowerIndex<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetIdOf<T>,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T>,
		ValueQuery,
	>;

	/// The reward accrued but not yet transferred to each user.
	#[pallet::storage]
	#[pallet::storage_prefix = "RewardAccured"]
	pub type RewardAccrued<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	pub type MarketBond<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, BoundedVec<AssetIdOf<T>, T::MaxLengthLimit>>;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Stores a new market and its related currency. Returns `Err` if a currency
		/// is not attached to an existent market.
		///
		/// All provided market states must be `Pending`, otherwise an error will be returned.
		///
		/// If a currency is already attached to a market, then the market will be replaced
		/// by the new provided value.
		///
		/// The lend token id and asset id are bound, the lend token id of new provided market
		/// cannot be duplicated with the existing one, otherwise it will return
		/// `InvalidLendTokenId`.
		///
		/// - `asset_id`: Market related currency
		/// - `market`: The market that is going to be stored
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::add_market())]
		#[transactional]
		pub fn add_market(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			market: Market<BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(!Markets::<T>::contains_key(asset_id), Error::<T>::MarketAlreadyExists);
			ensure!(
				market.state == MarketState::Pending,
				Error::<T>::NewMarketMustHavePendingState
			);
			ensure!(market.rate_model.check_model(), Error::<T>::InvalidRateModelParam);
			ensure!(
				market.collateral_factor >= Ratio::zero() &&
					market.collateral_factor < Ratio::one(),
				Error::<T>::InvalidFactor,
			);
			ensure!(
				market.liquidation_threshold < Ratio::one() &&
					market.liquidation_threshold >= market.collateral_factor,
				Error::<T>::InvalidFactor
			);
			ensure!(
				market.reserve_factor > Ratio::zero() && market.reserve_factor < Ratio::one(),
				Error::<T>::InvalidFactor,
			);
			ensure!(
				market.liquidate_incentive_reserved_factor > Ratio::zero() &&
					market.liquidate_incentive_reserved_factor < Ratio::one(),
				Error::<T>::InvalidFactor,
			);
			ensure!(market.supply_cap > Zero::zero(), Error::<T>::InvalidSupplyCap,);

			// Ensures a given `lend_token_id` not exists on the `Market` and `UnderlyingAssetId`.
			Self::ensure_lend_token(market.lend_token_id)?;
			// Update storage of `Market` and `UnderlyingAssetId`
			Markets::<T>::insert(asset_id, market.clone());
			UnderlyingAssetId::<T>::insert(market.lend_token_id, asset_id);

			// Init the ExchangeRate and BorrowIndex for asset
			ExchangeRate::<T>::insert(asset_id, Rate::from_inner(MIN_EXCHANGE_RATE));
			BorrowIndex::<T>::insert(asset_id, Rate::one());

			Self::deposit_event(Event::<T>::NewMarket(asset_id, market));
			Ok(().into())
		}

		/// Activates a market. Returns `Err` if the market currency does not exist.
		///
		/// If the market is already activated, does nothing.
		///
		/// - `asset_id`: Market related currency
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::activate_market())]
		#[transactional]
		pub fn activate_market(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			Self::mutate_market(asset_id, |stored_market| {
				if let MarketState::Active = stored_market.state {
					return stored_market.clone();
				}
				stored_market.state = MarketState::Active;
				stored_market.clone()
			})?;
			Self::deposit_event(Event::<T>::ActivatedMarket(asset_id));
			Ok(().into())
		}

		/// Updates the rate model of a stored market. Returns `Err` if the market
		/// currency does not exist or the rate model is invalid.
		///
		/// - `asset_id`: Market related currency
		/// - `rate_model`: The new rate model to be updated
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::update_rate_model())]
		#[transactional]
		pub fn update_rate_model(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			rate_model: InterestRateModel,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(rate_model.check_model(), Error::<T>::InvalidRateModelParam);
			let market = Self::mutate_market(asset_id, |stored_market| {
				stored_market.rate_model = rate_model;
				stored_market.clone()
			})?;
			Self::deposit_event(Event::<T>::UpdatedMarket(asset_id, market));

			Ok(().into())
		}

		/// Updates a stored market. Returns `Err` if the market currency does not exist.
		///
		/// - `asset_id`: market related currency
		/// - `collateral_factor`: the collateral utilization ratio
		/// - `reserve_factor`: fraction of interest currently set aside for reserves
		/// - `close_factor`: maximum liquidation ratio at one time
		/// - `liquidate_incentive`: liquidation incentive ratio
		/// - `cap`: market capacity
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::update_market())]
		#[transactional]
		pub fn update_market(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			collateral_factor: Option<Ratio>,
			liquidation_threshold: Option<Ratio>,
			reserve_factor: Option<Ratio>,
			close_factor: Option<Ratio>,
			liquidate_incentive_reserved_factor: Option<Ratio>,
			liquidate_incentive: Option<Rate>,
			supply_cap: Option<BalanceOf<T>>,
			borrow_cap: Option<BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;

			let market = Self::market(asset_id)?;

			let collateral_factor = collateral_factor.unwrap_or(market.collateral_factor);
			let liquidation_threshold =
				liquidation_threshold.unwrap_or(market.liquidation_threshold);
			let reserve_factor = reserve_factor.unwrap_or(market.reserve_factor);
			let close_factor = close_factor.unwrap_or(market.close_factor);
			let liquidate_incentive_reserved_factor = liquidate_incentive_reserved_factor
				.unwrap_or(market.liquidate_incentive_reserved_factor);
			let liquidate_incentive = liquidate_incentive.unwrap_or(market.liquidate_incentive);
			let supply_cap = supply_cap.unwrap_or(market.supply_cap);
			let borrow_cap = borrow_cap.unwrap_or(market.borrow_cap);

			ensure!(
				collateral_factor >= Ratio::zero() && collateral_factor < Ratio::one(),
				Error::<T>::InvalidFactor
			);
			ensure!(
				liquidation_threshold >= collateral_factor && liquidation_threshold < Ratio::one(),
				Error::<T>::InvalidFactor
			);
			ensure!(
				reserve_factor > Ratio::zero() && reserve_factor < Ratio::one(),
				Error::<T>::InvalidFactor
			);
			ensure!(supply_cap > Zero::zero(), Error::<T>::InvalidSupplyCap);

			let market = Self::mutate_market(asset_id, |stored_market| {
				*stored_market = Market {
					state: stored_market.state,
					lend_token_id: stored_market.lend_token_id,
					rate_model: stored_market.rate_model,
					collateral_factor,
					liquidation_threshold,
					reserve_factor,
					close_factor,
					liquidate_incentive,
					liquidate_incentive_reserved_factor,
					supply_cap,
					borrow_cap,
				};
				stored_market.clone()
			})?;
			Self::deposit_event(Event::<T>::UpdatedMarket(asset_id, market));

			Ok(().into())
		}

		/// Force updates a stored market. Returns `Err` if the market currency
		/// does not exist.
		///
		/// - `asset_id`: market related currency
		/// - `market`: the new market parameters
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::force_update_market())]
		#[transactional]
		pub fn force_update_market(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			market: Market<BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(market.rate_model.check_model(), Error::<T>::InvalidRateModelParam);
			if UnderlyingAssetId::<T>::contains_key(market.lend_token_id) {
				ensure!(
					Self::underlying_id(market.lend_token_id)? == asset_id,
					Error::<T>::InvalidLendTokenId
				);
			}
			UnderlyingAssetId::<T>::insert(market.lend_token_id, asset_id);
			let updated_market = Self::mutate_market(asset_id, |stored_market| {
				*stored_market = market;
				stored_market.clone()
			})?;

			Self::deposit_event(Event::<T>::UpdatedMarket(asset_id, updated_market));
			Ok(().into())
		}

		/// Add reward for the pallet account.
		///
		/// - `amount`: Reward amount added
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::add_reward())]
		#[transactional]
		pub fn add_reward(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(!amount.is_zero(), Error::<T>::InvalidAmount);

			let reward_asset = T::RewardAssetId::get();
			let pool_account = Self::reward_account_id()?;

			T::Assets::transfer(reward_asset, &who, &pool_account, amount, Preservation::Preserve)?;

			Self::deposit_event(Event::<T>::RewardAdded(who, amount));

			Ok(().into())
		}

		/// Withdraw reward token from pallet account.
		///
		/// The origin must conform to `UpdateOrigin`.
		///
		/// - `target_account`: account receive reward token.
		/// - `amount`: Withdraw amount
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::withdraw_missing_reward())]
		#[transactional]
		pub fn withdraw_missing_reward(
			origin: OriginFor<T>,
			target_account: <T::Lookup as StaticLookup>::Source,
			amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ensure!(!amount.is_zero(), Error::<T>::InvalidAmount);

			let reward_asset = T::RewardAssetId::get();
			let pool_account = Self::reward_account_id()?;
			let target_account = T::Lookup::lookup(target_account)?;

			T::Assets::transfer(
				reward_asset,
				&pool_account,
				&target_account,
				amount,
				Preservation::Preserve,
			)?;
			Self::deposit_event(Event::<T>::RewardWithdrawn(target_account, amount));

			Ok(().into())
		}

		/// Updates reward speed for the specified market
		///
		/// The origin must conform to `UpdateOrigin`.
		///
		/// - `asset_id`: Market related currency
		/// - `reward_per_block`: reward amount per block.
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::update_market_reward_speed())]
		#[transactional]
		pub fn update_market_reward_speed(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			supply_reward_per_block: Option<BalanceOf<T>>,
			borrow_reward_per_block: Option<BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			Self::ensure_active_market(asset_id)?;

			let current_supply_speed = RewardSupplySpeed::<T>::get(asset_id);
			let current_borrow_speed = RewardBorrowSpeed::<T>::get(asset_id);

			let supply_reward_per_block = supply_reward_per_block.unwrap_or(current_supply_speed);
			let borrow_reward_per_block = borrow_reward_per_block.unwrap_or(current_borrow_speed);

			if supply_reward_per_block != current_supply_speed {
				Self::update_reward_supply_index(asset_id)?;
				RewardSupplySpeed::<T>::try_mutate(asset_id, |current_speed| -> DispatchResult {
					*current_speed = supply_reward_per_block;
					Ok(())
				})?;
			}

			if borrow_reward_per_block != current_borrow_speed {
				Self::update_reward_borrow_index(asset_id)?;
				RewardBorrowSpeed::<T>::try_mutate(asset_id, |current_speed| -> DispatchResult {
					*current_speed = borrow_reward_per_block;
					Ok(())
				})?;
			}

			Self::deposit_event(Event::<T>::MarketRewardSpeedUpdated(
				asset_id,
				supply_reward_per_block,
				borrow_reward_per_block,
			));
			Ok(().into())
		}

		/// Claim reward from all market.
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::claim_reward())]
		#[transactional]
		pub fn claim_reward(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			for asset_id in Markets::<T>::iter_keys() {
				Self::collect_market_reward(asset_id, &who)?;
			}

			Self::pay_reward(&who)?;

			Ok(().into())
		}

		/// Claim reward from the specified market.
		///
		/// - `asset_id`: Market related currency
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::claim_reward_for_market())]
		#[transactional]
		pub fn claim_reward_for_market(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			Self::collect_market_reward(asset_id, &who)?;

			Self::pay_reward(&who)?;

			Ok(().into())
		}

		/// Sender supplies assets into the market and receives internal supplies in exchange.
		///
		/// - `asset_id`: the asset to be deposited.
		/// - `mint_amount`: the amount to be deposited.
		#[pallet::call_index(10)]
		#[pallet::weight(T::WeightInfo::mint())]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			#[pallet::compact] mint_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_mint(&who, asset_id, mint_amount)?;

			Ok(().into())
		}

		/// Sender redeems some of internal supplies in exchange for the underlying asset.
		///
		/// - `asset_id`: the asset to be redeemed.
		/// - `redeem_amount`: the amount to be redeemed.
		#[pallet::call_index(11)]
		#[pallet::weight(T::WeightInfo::redeem())]
		#[transactional]
		pub fn redeem(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			#[pallet::compact] redeem_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(!redeem_amount.is_zero(), Error::<T>::InvalidAmount);
			Self::do_redeem(&who, asset_id, redeem_amount)?;

			Ok(().into())
		}

		/// Sender redeems all of internal supplies in exchange for the underlying asset.
		///
		/// - `asset_id`: the asset to be redeemed.
		#[pallet::call_index(12)]
		#[pallet::weight(T::WeightInfo::redeem_all())]
		#[transactional]
		pub fn redeem_all(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let _ = Self::do_redeem_all(&who, asset_id)?;

			Ok(().into())
		}

		/// Sender borrows assets from the protocol to their own address.
		///
		/// - `asset_id`: the asset to be borrowed.
		/// - `borrow_amount`: the amount to be borrowed.
		#[pallet::call_index(13)]
		#[pallet::weight(T::WeightInfo::borrow())]
		#[transactional]
		pub fn borrow(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			#[pallet::compact] borrow_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			Self::do_borrow(&who, asset_id, borrow_amount)?;

			Ok(().into())
		}

		/// Sender repays some of their debts.
		///
		/// - `asset_id`: the asset to be repaid.
		/// - `repay_amount`: the amount to be repaid.
		#[pallet::call_index(14)]
		#[pallet::weight(T::WeightInfo::repay_borrow())]
		#[transactional]
		pub fn repay_borrow(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			#[pallet::compact] repay_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			Self::do_repay_borrow(&who, asset_id, repay_amount)?;

			Ok(().into())
		}

		/// Sender repays all of their debts.
		///
		/// - `asset_id`: the asset to be repaid.
		#[pallet::call_index(15)]
		#[pallet::weight(T::WeightInfo::repay_borrow_all())]
		#[transactional]
		pub fn repay_borrow_all(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::ensure_active_market(asset_id)?;
			Self::accrue_interest(asset_id)?;
			let account_borrows = Self::current_borrow_balance(&who, asset_id)?;
			Self::do_repay_borrow(&who, asset_id, account_borrows)?;

			Ok(().into())
		}

		/// Set the collateral asset.
		///
		/// - `asset_id`: the asset to be set.
		/// - `enable`: turn on/off the collateral option.
		#[pallet::call_index(16)]
		#[pallet::weight(T::WeightInfo::collateral_asset())]
		#[transactional]
		pub fn collateral_asset(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			enable: bool,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::ensure_active_market(asset_id)?;
			ensure!(AccountDeposits::<T>::contains_key(asset_id, &who), Error::<T>::NoDeposit);
			let deposits = AccountDeposits::<T>::get(asset_id, &who);
			if deposits.is_collateral == enable {
				return Err(Error::<T>::DuplicateOperation.into());
			}

			Self::do_collateral_asset(&who, asset_id, enable)?;

			Ok(().into())
		}

		/// The sender liquidates the borrower's collateral.
		///
		/// - `borrower`: the borrower to be liquidated.
		/// - `liquidation_asset_id`: the assert to be liquidated.
		/// - `repay_amount`: the amount to be repaid borrow.
		/// - `collateral_asset_id`: The collateral to seize from the borrower.
		#[pallet::call_index(17)]
		#[pallet::weight(T::WeightInfo::liquidate_borrow())]
		#[transactional]
		pub fn liquidate_borrow(
			origin: OriginFor<T>,
			borrower: T::AccountId,
			liquidation_asset_id: AssetIdOf<T>,
			#[pallet::compact] repay_amount: BalanceOf<T>,
			collateral_asset_id: AssetIdOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(
				!LiquidationFreeCollaterals::<T>::get().contains(&collateral_asset_id),
				Error::<T>::CollateralReserved
			);
			Self::accrue_interest(liquidation_asset_id)?;
			Self::accrue_interest(collateral_asset_id)?;
			Self::do_liquidate_borrow(
				who,
				borrower,
				liquidation_asset_id,
				repay_amount,
				collateral_asset_id,
			)?;
			Ok(().into())
		}

		/// Add reserves by transferring from payer.
		///
		/// May only be called from `T::ReserveOrigin`.
		///
		/// - `payer`: the payer account.
		/// - `asset_id`: the assets to be added.
		/// - `add_amount`: the amount to be added.
		#[pallet::call_index(18)]
		#[pallet::weight(T::WeightInfo::add_reserves())]
		#[transactional]
		pub fn add_reserves(
			origin: OriginFor<T>,
			payer: <T::Lookup as StaticLookup>::Source,
			asset_id: AssetIdOf<T>,
			#[pallet::compact] add_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			T::ReserveOrigin::ensure_origin(origin)?;
			let payer = T::Lookup::lookup(payer)?;
			Self::ensure_active_market(asset_id)?;

			T::Assets::transfer(
				asset_id,
				&payer,
				&Self::account_id(),
				add_amount,
				Preservation::Expendable,
			)?;
			let total_reserves = TotalReserves::<T>::get(asset_id);
			let total_reserves_new =
				total_reserves.checked_add(add_amount).ok_or(ArithmeticError::Overflow)?;
			TotalReserves::<T>::insert(asset_id, total_reserves_new);

			Self::deposit_event(Event::<T>::ReservesAdded(
				payer,
				asset_id,
				add_amount,
				total_reserves_new,
			));

			Ok(().into())
		}

		/// Reduces reserves by transferring to receiver.
		///
		/// May only be called from `T::ReserveOrigin`.
		///
		/// - `receiver`: the receiver account.
		/// - `asset_id`: the assets to be reduced.
		/// - `reduce_amount`: the amount to be reduced.
		#[pallet::call_index(19)]
		#[pallet::weight(T::WeightInfo::reduce_reserves())]
		#[transactional]
		pub fn reduce_reserves(
			origin: OriginFor<T>,
			receiver: <T::Lookup as StaticLookup>::Source,
			asset_id: AssetIdOf<T>,
			#[pallet::compact] reduce_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			T::ReserveOrigin::ensure_origin(origin)?;
			let receiver = T::Lookup::lookup(receiver)?;
			Self::ensure_active_market(asset_id)?;

			let total_reserves = TotalReserves::<T>::get(asset_id);
			if reduce_amount > total_reserves {
				return Err(Error::<T>::InsufficientReserves.into());
			}
			let total_reserves_new =
				total_reserves.checked_sub(reduce_amount).ok_or(ArithmeticError::Underflow)?;
			TotalReserves::<T>::insert(asset_id, total_reserves_new);
			T::Assets::transfer(
				asset_id,
				&Self::account_id(),
				&receiver,
				reduce_amount,
				Preservation::Expendable,
			)?;

			Self::deposit_event(Event::<T>::ReservesReduced(
				receiver,
				asset_id,
				reduce_amount,
				total_reserves_new,
			));

			Ok(().into())
		}

		/// Sender redeems some of internal supplies in exchange for the underlying asset.
		///
		/// - `asset_id`: the asset to be redeemed.
		/// - `redeem_amount`: the amount to be redeemed.
		#[pallet::call_index(20)]
		#[pallet::weight(T::WeightInfo::redeem()+T::WeightInfo::reduce_reserves())]
		#[transactional]
		pub fn reduce_incentive_reserves(
			origin: OriginFor<T>,
			receiver: <T::Lookup as StaticLookup>::Source,
			asset_id: AssetIdOf<T>,
			#[pallet::compact] redeem_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			T::ReserveOrigin::ensure_origin(origin)?;
			ensure!(!redeem_amount.is_zero(), Error::<T>::InvalidAmount);
			let receiver = T::Lookup::lookup(receiver)?;
			let from = Self::incentive_reward_account_id()?;
			Self::ensure_active_market(asset_id)?;
			let exchange_rate = Self::exchange_rate_stored(asset_id)?;
			let voucher_amount = Self::calc_collateral_amount(redeem_amount, exchange_rate)?;
			let redeem_amount = Self::do_redeem_voucher(&from, asset_id, voucher_amount)?;
			T::Assets::transfer(
				asset_id,
				&from,
				&receiver,
				redeem_amount,
				Preservation::Expendable,
			)?;
			Self::deposit_event(Event::<T>::IncentiveReservesReduced(
				receiver,
				asset_id,
				redeem_amount,
			));
			Ok(().into())
		}

		/// Update liquidation free collateral.
		///
		/// The `assets` won't be counted when do general
		#[pallet::call_index(21)]
		#[pallet::weight(T::WeightInfo::update_liquidation_free_collateral())]
		#[transactional]
		pub fn update_liquidation_free_collateral(
			origin: OriginFor<T>,
			collaterals: Vec<AssetIdOf<T>>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			LiquidationFreeCollaterals::<T>::try_mutate(
				|liquidation_free_collaterals| -> DispatchResultWithPostInfo {
					// Attempt to convert `collaterals` into a `BoundedVec` and handle potential
					// conversion error
					*liquidation_free_collaterals = BoundedVec::try_from(collaterals.clone())
						.map_err(|_| Error::<T>::ConversionError)?;
					Ok(().into())
				},
			)?;

			Self::deposit_event(Event::<T>::LiquidationFreeCollateralsUpdated(collaterals));
			Ok(().into())
		}

		#[pallet::call_index(22)]
		#[pallet::weight(T::WeightInfo::add_market())]
		#[transactional]
		pub fn add_market_bond(
			origin: OriginFor<T>,
			asset_id: AssetIdOf<T>,
			market_bond: Vec<AssetIdOf<T>>,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			MarketBond::<T>::insert(
				asset_id,
				BoundedVec::try_from(market_bond.clone())
					.map_err(|_| Error::<T>::ConversionError)?,
			);

			Self::deposit_event(Event::<T>::MarketBonded { asset_id, market_bond });
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	fn get_lf_borrowed_value(account: &T::AccountId) -> Result<FixedU128, DispatchError> {
		let lf_borrowed_amount =
			Self::current_borrow_balance(account, T::LiquidationFreeAssetId::get())?;
		Self::get_asset_value(T::LiquidationFreeAssetId::get(), lf_borrowed_amount)
	}

	fn get_lf_base_position(account: &T::AccountId) -> Result<FixedU128, DispatchError> {
		let mut total_asset_value: FixedU128 = FixedU128::zero();
		for (asset_id, _market) in Self::active_markets()
			.filter(|(asset_id, _)| LiquidationFreeCollaterals::<T>::get().contains(asset_id))
		{
			total_asset_value = total_asset_value
				.checked_add(&Self::collateral_asset_value(account, asset_id)?)
				.ok_or(ArithmeticError::Overflow)?;
		}
		Ok(total_asset_value)
	}

	fn get_lf_liquidation_base_position(
		account: &T::AccountId,
	) -> Result<FixedU128, DispatchError> {
		let mut total_asset_value: FixedU128 = FixedU128::zero();
		for (asset_id, _market) in Self::active_markets()
			.filter(|(asset_id, _)| LiquidationFreeCollaterals::<T>::get().contains(asset_id))
		{
			total_asset_value = total_asset_value
				.checked_add(&Self::liquidation_threshold_asset_value(account, asset_id)?)
				.ok_or(ArithmeticError::Overflow)?;
		}
		Ok(total_asset_value)
	}

	pub fn get_account_liquidity(
		account: &T::AccountId,
	) -> Result<(Liquidity, Shortfall, Liquidity, Shortfall), DispatchError> {
		let total_borrow_value = Self::total_borrowed_value(account)?;
		let total_collateral_value = Self::total_collateral_value(account)?;
		let lf_borrowed_value = Self::get_lf_borrowed_value(account)?;
		let lf_base_position = Self::get_lf_base_position(account)?;

		log::trace!(
			target: "lend-market::get_account_liquidity",
			"account: {:?}, total_borrow_value: {:?}, total_collateral_value: {:?}, lf_borrowed_value: {:?}, lf_base_position: {:?}",
			account,
			total_borrow_value.into_inner(),
			total_collateral_value.into_inner(),
			lf_borrowed_value.into_inner(),
			lf_base_position.into_inner(),
		);
		match (total_collateral_value > total_borrow_value, lf_base_position > lf_borrowed_value) {
			(true, true) => Ok((
				total_collateral_value - total_borrow_value,
				FixedU128::zero(),
				lf_base_position - lf_borrowed_value,
				FixedU128::zero(),
			)),
			(true, false) => Ok((
				total_collateral_value - total_borrow_value,
				FixedU128::zero(),
				FixedU128::zero(),
				lf_borrowed_value - lf_base_position,
			)),
			(false, true) => Ok((
				FixedU128::zero(),
				total_borrow_value - total_collateral_value,
				lf_base_position - lf_borrowed_value,
				FixedU128::zero(),
			)),
			(false, false) => Ok((
				FixedU128::zero(),
				total_borrow_value - total_collateral_value,
				FixedU128::zero(),
				lf_borrowed_value - lf_base_position,
			)),
		}
	}

	pub fn get_account_liquidation_threshold_liquidity(
		account: &T::AccountId,
	) -> Result<(Liquidity, Shortfall, Liquidity, Shortfall), DispatchError> {
		let total_borrow_value = Self::total_borrowed_value(account)?;
		let total_collateral_value = Self::total_liquidation_threshold_value(account)?;

		let lf_borrowed_value = Self::get_lf_borrowed_value(account)?;
		let lf_base_position = Self::get_lf_liquidation_base_position(account)?;

		log::trace!(
			target: "lend-market::get_account_liquidation_threshold_liquidity",
			"account: {:?}, total_borrow_value: {:?}, total_collateral_value: {:?}, lf_borrowed_value: {:?}, lf_base_position: {:?}",
			account,
			total_borrow_value.into_inner(),
			total_collateral_value.into_inner(),
			lf_borrowed_value.into_inner(),
			lf_base_position.into_inner(),
		);

		match (total_collateral_value > total_borrow_value, lf_base_position > lf_borrowed_value) {
			(true, true) => Ok((
				total_collateral_value - total_borrow_value,
				FixedU128::zero(),
				lf_base_position - lf_borrowed_value,
				FixedU128::zero(),
			)),
			(true, false) => Ok((
				total_collateral_value - total_borrow_value,
				FixedU128::zero(),
				FixedU128::zero(),
				lf_borrowed_value - lf_base_position,
			)),
			(false, true) => Ok((
				FixedU128::zero(),
				total_borrow_value - total_collateral_value,
				lf_base_position - lf_borrowed_value,
				FixedU128::zero(),
			)),
			(false, false) => Ok((
				FixedU128::zero(),
				total_borrow_value - total_collateral_value,
				FixedU128::zero(),
				lf_borrowed_value - lf_base_position,
			)),
		}
	}

	fn total_borrowed_value(borrower: &T::AccountId) -> Result<FixedU128, DispatchError> {
		let mut total_borrow_value: FixedU128 = FixedU128::zero();
		for (asset_id, _) in Self::active_markets() {
			let currency_borrow_amount = Self::current_borrow_balance(borrower, asset_id)?;
			if currency_borrow_amount.is_zero() {
				continue;
			}
			total_borrow_value = Self::get_asset_value(asset_id, currency_borrow_amount)?
				.checked_add(&total_borrow_value)
				.ok_or(ArithmeticError::Overflow)?;
		}

		Ok(total_borrow_value)
	}

	fn current_collateral_balance(
		supplier: &T::AccountId,
		asset_id: AssetIdOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		if !AccountDeposits::<T>::contains_key(asset_id, supplier) {
			return Ok(BalanceOf::<T>::zero());
		}
		let deposits = AccountDeposits::<T>::get(asset_id, supplier);
		if !deposits.is_collateral {
			return Ok(BalanceOf::<T>::zero());
		}
		if deposits.voucher_balance.is_zero() {
			return Ok(BalanceOf::<T>::zero());
		}
		let exchange_rate = Self::exchange_rate_stored(asset_id)?;
		let underlying_amount =
			Self::calc_underlying_amount(deposits.voucher_balance, exchange_rate)?;
		let market = Self::market(asset_id)?;
		let effects_amount = market.collateral_factor.mul_ceil(underlying_amount);

		Ok(BalanceOf::<T>::saturated_from(effects_amount))
	}

	fn collateral_asset_value(
		supplier: &T::AccountId,
		asset_id: AssetIdOf<T>,
	) -> Result<FixedU128, DispatchError> {
		let effects_amount = Self::current_collateral_balance(supplier, asset_id)?;

		Self::get_asset_value(asset_id, effects_amount)
	}

	fn liquidation_threshold_asset_value(
		borrower: &T::AccountId,
		asset_id: AssetIdOf<T>,
	) -> Result<FixedU128, DispatchError> {
		if !AccountDeposits::<T>::contains_key(asset_id, borrower) {
			return Ok(FixedU128::zero());
		}
		let deposits = AccountDeposits::<T>::get(asset_id, borrower);
		if !deposits.is_collateral {
			return Ok(FixedU128::zero());
		}
		if deposits.voucher_balance.is_zero() {
			return Ok(FixedU128::zero());
		}
		let exchange_rate = Self::exchange_rate_stored(asset_id)?;
		let underlying_amount =
			Self::calc_underlying_amount(deposits.voucher_balance, exchange_rate)?;
		let market = Self::market(asset_id)?;
		let effects_amount = market.liquidation_threshold.mul_ceil(underlying_amount);

		Self::get_asset_value(asset_id, effects_amount)
	}

	fn total_collateral_value(supplier: &T::AccountId) -> Result<FixedU128, DispatchError> {
		let mut total_asset_value: FixedU128 = FixedU128::zero();
		for (asset_id, _market) in Self::active_markets() {
			total_asset_value = total_asset_value
				.checked_add(&Self::collateral_asset_value(supplier, asset_id)?)
				.ok_or(ArithmeticError::Overflow)?;
		}

		Ok(total_asset_value)
	}

	fn total_liquidation_threshold_value(
		borrower: &T::AccountId,
	) -> Result<FixedU128, DispatchError> {
		let mut total_asset_value: FixedU128 = FixedU128::zero();
		for (asset_id, _market) in Self::active_markets() {
			total_asset_value = total_asset_value
				.checked_add(&Self::liquidation_threshold_asset_value(borrower, asset_id)?)
				.ok_or(ArithmeticError::Overflow)?;
		}

		Ok(total_asset_value)
	}

	/// Checks if the redeemer should be allowed to redeem tokens in given market
	fn redeem_allowed(
		asset_id: AssetIdOf<T>,
		redeemer: &T::AccountId,
		voucher_amount: BalanceOf<T>,
	) -> DispatchResult {
		log::trace!(
			target: "lend-market::redeem_allowed",
			"asset_id: {:?}, redeemer: {:?}, voucher_amount: {:?}",
			asset_id,
			redeemer,
			voucher_amount,
		);
		let deposit = AccountDeposits::<T>::get(asset_id, redeemer);
		if deposit.voucher_balance < voucher_amount {
			return Err(Error::<T>::InsufficientDeposit.into());
		}

		let exchange_rate = Self::exchange_rate_stored(asset_id)?;
		let redeem_amount = Self::calc_underlying_amount(voucher_amount, exchange_rate)?;
		Self::ensure_enough_cash(asset_id, redeem_amount)?;

		if !deposit.is_collateral {
			return Ok(());
		}

		let market = Self::market(asset_id)?;
		let effects_amount = market.collateral_factor.mul_ceil(redeem_amount);
		let redeem_effects_value = Self::get_asset_value(asset_id, effects_amount)?;
		log::trace!(
			target: "lend-market::redeem_allowed",
			"redeem_amount: {:?}, redeem_effects_value: {:?}",
			redeem_amount,
			redeem_effects_value.into_inner(),
		);

		Self::ensure_liquidity(
			redeemer,
			redeem_effects_value,
			LiquidationFreeCollaterals::<T>::get().contains(&asset_id),
		)?;

		Ok(())
	}

	#[require_transactional]
	pub fn do_redeem_voucher(
		who: &T::AccountId,
		asset_id: AssetIdOf<T>,
		voucher_amount: BalanceOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		Self::redeem_allowed(asset_id, who, voucher_amount)?;
		Self::update_reward_supply_index(asset_id)?;
		Self::distribute_supplier_reward(asset_id, who)?;

		let exchange_rate = Self::exchange_rate_stored(asset_id)?;
		let redeem_amount = Self::calc_underlying_amount(voucher_amount, exchange_rate)?;

		AccountDeposits::<T>::try_mutate_exists(asset_id, who, |deposits| -> DispatchResult {
			let mut d = deposits.unwrap_or_default();
			d.voucher_balance = d
				.voucher_balance
				.checked_sub(voucher_amount)
				.ok_or(ArithmeticError::Underflow)?;
			if d.voucher_balance.is_zero() {
				// remove deposits storage if zero balance
				*deposits = None;
			} else {
				*deposits = Some(d);
			}
			Ok(())
		})?;
		TotalSupply::<T>::try_mutate(asset_id, |total_balance| -> DispatchResult {
			let new_balance =
				total_balance.checked_sub(voucher_amount).ok_or(ArithmeticError::Underflow)?;
			*total_balance = new_balance;
			Ok(())
		})?;

		T::Assets::transfer(
			asset_id,
			&Self::account_id(),
			who,
			redeem_amount,
			Preservation::Expendable,
		)
		.map_err(|_| Error::<T>::InsufficientCash)?;
		Ok(redeem_amount)
	}

	/// Borrower shouldn't borrow more than his total collateral value
	fn borrow_allowed(
		asset_id: AssetIdOf<T>,
		borrower: &T::AccountId,
		borrow_amount: BalanceOf<T>,
	) -> DispatchResult {
		Self::ensure_under_borrow_cap(asset_id, borrow_amount)?;
		Self::ensure_enough_cash(asset_id, borrow_amount)?;
		let borrow_value = Self::get_asset_value(asset_id, borrow_amount)?;
		Self::ensure_liquidity(
			borrower,
			borrow_value,
			asset_id == T::LiquidationFreeAssetId::get(),
		)?;

		Ok(())
	}

	/// Borrower shouldn't borrow more than his bonded collateral value
	fn borrow_allowed_for_market_bond(
		borrow_asset_id: AssetIdOf<T>,
		borrower: &T::AccountId,
		borrow_amount: BalanceOf<T>,
	) -> DispatchResult {
		Self::ensure_under_borrow_cap(borrow_asset_id, borrow_amount)?;
		Self::ensure_enough_cash(borrow_asset_id, borrow_amount)?;
		let borrow_value = Self::get_asset_value(borrow_asset_id, borrow_amount)?;
		Self::ensure_liquidity_for_market_bond(borrow_asset_id, borrower, borrow_value)?;

		Ok(())
	}

	#[require_transactional]
	fn do_repay_borrow_with_amount(
		borrower: &T::AccountId,
		asset_id: AssetIdOf<T>,
		account_borrows: BalanceOf<T>,
		repay_amount: BalanceOf<T>,
	) -> DispatchResult {
		if account_borrows < repay_amount {
			return Err(Error::<T>::TooMuchRepay.into());
		}
		Self::update_reward_borrow_index(asset_id)?;
		Self::distribute_borrower_reward(asset_id, borrower)?;

		T::Assets::transfer(
			asset_id,
			borrower,
			&Self::account_id(),
			repay_amount,
			Preservation::Expendable,
		)?;
		let account_borrows_new =
			account_borrows.checked_sub(repay_amount).ok_or(ArithmeticError::Underflow)?;
		let total_borrows = TotalBorrows::<T>::get(asset_id);
		// NOTE : total_borrows use a different way to calculate interest
		// so when user repays all borrows, total_borrows can be less than account_borrows
		// which will cause it to fail with `ArithmeticError::Underflow`
		//
		// Change it back to checked_sub will cause Underflow
		let total_borrows_new = total_borrows.saturating_sub(repay_amount);
		AccountBorrows::<T>::insert(
			asset_id,
			borrower,
			BorrowSnapshot {
				principal: account_borrows_new,
				borrow_index: BorrowIndex::<T>::get(asset_id),
			},
		);
		TotalBorrows::<T>::insert(asset_id, total_borrows_new);

		Ok(())
	}

	// Calculates and returns the most recent amount of borrowed balance of `currency_id`
	// for `who`.
	pub fn current_borrow_balance(
		who: &T::AccountId,
		asset_id: AssetIdOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		let snapshot: BorrowSnapshot<BalanceOf<T>> = AccountBorrows::<T>::get(asset_id, who);
		if snapshot.principal.is_zero() || snapshot.borrow_index.is_zero() {
			return Ok(Zero::zero());
		}
		// Calculate new borrow balance using the interest index:
		// recent_borrow_balance = snapshot.principal * borrow_index / snapshot.borrow_index
		let recent_borrow_balance = BorrowIndex::<T>::get(asset_id)
			.checked_div(&snapshot.borrow_index)
			.and_then(|r| r.checked_mul_int(snapshot.principal))
			.ok_or(ArithmeticError::Overflow)?;

		Ok(recent_borrow_balance)
	}

	#[require_transactional]
	fn update_earned_stored(
		who: &T::AccountId,
		asset_id: AssetIdOf<T>,
		exchange_rate: Rate,
	) -> DispatchResult {
		let deposits = AccountDeposits::<T>::get(asset_id, who);
		let account_earned = AccountEarned::<T>::get(asset_id, who);
		let total_earned_prior_new = exchange_rate
			.checked_sub(&account_earned.exchange_rate_prior)
			.and_then(|r| r.checked_mul_int(deposits.voucher_balance))
			.and_then(|r| r.checked_add(account_earned.total_earned_prior))
			.ok_or(ArithmeticError::Overflow)?;

		AccountEarned::<T>::insert(
			asset_id,
			who,
			EarnedSnapshot {
				exchange_rate_prior: exchange_rate,
				total_earned_prior: total_earned_prior_new,
			},
		);

		Ok(())
	}

	/// Checks if the liquidation should be allowed to occur
	fn liquidate_borrow_allowed(
		borrower: &T::AccountId,
		liquidation_asset_id: AssetIdOf<T>,
		repay_amount: BalanceOf<T>,
		market: &Market<BalanceOf<T>>,
	) -> DispatchResult {
		log::trace!(
			target: "lend-market::liquidate_borrow_allowed",
			"borrower: {:?}, liquidation_asset_id {:?}, repay_amount {:?}, market: {:?}",
			borrower,
			liquidation_asset_id,
			repay_amount,
			market
		);
		let (liquidity, shortfall, lf_liquidity, _) =
			Self::get_account_liquidation_threshold_liquidity(borrower)?;

		// C_other >= B_other + B_dot_over
		// C_other >= B_other + max(B_dot - C_lf, 0)
		// C_other + C_lf >= B_other + B_dot - B_dot + C_lf + max(B_dot - C_lf, 0)
		// C_all - B_all >= max(0, C_lf - B_dot)
		// C_all - B_all >= 0 && C_all - B_all >= max(0, C_lf - B_dot)
		// shortfall == 0 && liquidity > lf_liquidity
		if shortfall.is_zero() && liquidity >= lf_liquidity {
			return Err(Error::<T>::InsufficientShortfall.into());
		}

		// The liquidator may not repay more than 50%(close_factor) of the borrower's borrow
		// balance.
		let account_borrows = Self::current_borrow_balance(borrower, liquidation_asset_id)?;
		let account_borrows_value = Self::get_asset_value(liquidation_asset_id, account_borrows)?;
		let repay_value = Self::get_asset_value(liquidation_asset_id, repay_amount)?;
		let effects_borrows_value = if liquidation_asset_id == T::LiquidationFreeAssetId::get() {
			let base_position = Self::get_lf_base_position(borrower)?;
			if account_borrows_value > base_position {
				account_borrows_value - base_position
			} else {
				FixedU128::zero()
			}
		} else {
			account_borrows_value
		};

		if market.close_factor.mul_ceil(effects_borrows_value.into_inner()) <
			repay_value.into_inner()
		{
			return Err(Error::<T>::TooMuchRepay.into());
		}

		Ok(())
	}

	/// Note:
	/// - liquidation_asset_id is borrower's debt asset.
	/// - collateral_asset_id is borrower's collateral asset.
	/// - repay_amount is amount of liquidation_asset_id
	///
	/// The liquidator will repay a certain amount of liquidation_asset_id from own
	/// account for borrower. Then the protocol will reduce borrower's debt
	/// and liquidator will receive collateral_asset_id(as voucher amount) from
	/// borrower.
	#[require_transactional]
	pub fn do_liquidate_borrow(
		liquidator: T::AccountId,
		borrower: T::AccountId,
		liquidation_asset_id: AssetIdOf<T>,
		repay_amount: BalanceOf<T>,
		collateral_asset_id: AssetIdOf<T>,
	) -> DispatchResult {
		Self::ensure_active_market(liquidation_asset_id)?;
		Self::ensure_active_market(collateral_asset_id)?;

		let market = Self::market(liquidation_asset_id)?;

		if borrower == liquidator {
			return Err(Error::<T>::LiquidatorIsBorrower.into());
		}
		Self::liquidate_borrow_allowed(&borrower, liquidation_asset_id, repay_amount, &market)?;

		let deposits = AccountDeposits::<T>::get(collateral_asset_id, &borrower);
		if !deposits.is_collateral {
			return Err(Error::<T>::DepositsAreNotCollateral.into());
		}
		let exchange_rate = Self::exchange_rate_stored(collateral_asset_id)?;
		let borrower_deposit_amount = exchange_rate
			.checked_mul_int(deposits.voucher_balance)
			.ok_or(ArithmeticError::Overflow)?;

		let collateral_value = Self::get_asset_value(collateral_asset_id, borrower_deposit_amount)?;
		// liquidate_value contains the incentive of liquidator and the punishment of the borrower
		let liquidate_value = Self::get_asset_value(liquidation_asset_id, repay_amount)?
			.checked_mul(&market.liquidate_incentive)
			.ok_or(ArithmeticError::Overflow)?;

		if collateral_value < liquidate_value {
			return Err(Error::<T>::InsufficientCollateral.into());
		}

		// Calculate the collateral will get
		//
		// amount: 1 Unit = 10^12 pico
		// price is for 1 pico: 1$ = FixedU128::saturating_from_rational(1, 10^12)
		// if price is N($) and amount is M(Unit):
		// liquidate_value = price * amount = (N / 10^12) * (M * 10^12) = N * M
		// if liquidate_value >= 340282366920938463463.374607431768211455,
		// FixedU128::saturating_from_integer(liquidate_value) will overflow, so we use from_inner
		// instead of saturating_from_integer, and after calculation use into_inner to get final
		// value.
		let collateral_token_price = Self::get_price(collateral_asset_id)?;
		let real_collateral_underlying_amount = liquidate_value
			.checked_div(&collateral_token_price)
			.ok_or(ArithmeticError::Underflow)?
			.into_inner();

		//inside transfer token
		Self::liquidated_transfer(
			&liquidator,
			&borrower,
			liquidation_asset_id,
			collateral_asset_id,
			repay_amount,
			real_collateral_underlying_amount,
			&market,
		)?;

		Ok(())
	}

	#[require_transactional]
	fn liquidated_transfer(
		liquidator: &T::AccountId,
		borrower: &T::AccountId,
		liquidation_asset_id: AssetIdOf<T>,
		collateral_asset_id: AssetIdOf<T>,
		repay_amount: BalanceOf<T>,
		collateral_underlying_amount: BalanceOf<T>,
		market: &Market<BalanceOf<T>>,
	) -> DispatchResult {
		log::trace!(
			target: "lend-market::liquidated_transfer",
			"liquidator: {:?}, borrower: {:?}, liquidation_asset_id: {:?},
				collateral_asset_id: {:?}, repay_amount: {:?}, collateral_underlying_amount: {:?}",
			liquidator,
			borrower,
			liquidation_asset_id,
			collateral_asset_id,
			repay_amount,
			collateral_underlying_amount
		);

		// update borrow index after accrue interest.
		Self::update_reward_borrow_index(liquidation_asset_id)?;
		Self::distribute_borrower_reward(liquidation_asset_id, liquidator)?;

		// 1.liquidator repay borrower's debt,
		// transfer from liquidator to module account
		T::Assets::transfer(
			liquidation_asset_id,
			liquidator,
			&Self::account_id(),
			repay_amount,
			Preservation::Expendable,
		)?;

		// 2.the system reduce borrower's debt
		let account_borrows = Self::current_borrow_balance(borrower, liquidation_asset_id)?;
		let account_borrows_new =
			account_borrows.checked_sub(repay_amount).ok_or(ArithmeticError::Underflow)?;
		let total_borrows = TotalBorrows::<T>::get(liquidation_asset_id);
		let total_borrows_new =
			total_borrows.checked_sub(repay_amount).ok_or(ArithmeticError::Underflow)?;
		AccountBorrows::<T>::insert(
			liquidation_asset_id,
			borrower,
			BorrowSnapshot {
				principal: account_borrows_new,
				borrow_index: BorrowIndex::<T>::get(liquidation_asset_id),
			},
		);
		TotalBorrows::<T>::insert(liquidation_asset_id, total_borrows_new);

		// update supply index before modify supply balance.
		Self::update_reward_supply_index(collateral_asset_id)?;
		Self::distribute_supplier_reward(collateral_asset_id, liquidator)?;
		Self::distribute_supplier_reward(collateral_asset_id, borrower)?;
		Self::distribute_supplier_reward(
			collateral_asset_id,
			&Self::incentive_reward_account_id()?,
		)?;

		// 3.the liquidator will receive voucher token from borrower
		let exchange_rate = Self::exchange_rate_stored(collateral_asset_id)?;
		let collateral_amount =
			Self::calc_collateral_amount(collateral_underlying_amount, exchange_rate)?;
		AccountDeposits::<T>::try_mutate(
			collateral_asset_id,
			borrower,
			|deposits| -> DispatchResult {
				deposits.voucher_balance = deposits
					.voucher_balance
					.checked_sub(collateral_amount)
					.ok_or(ArithmeticError::Underflow)?;
				Ok(())
			},
		)?;
		let incentive_reserved_amount = market.liquidate_incentive_reserved_factor.mul_floor(
			FixedU128::from_inner(collateral_amount)
				.checked_div(&market.liquidate_incentive)
				.map(|r| r.into_inner())
				.ok_or(ArithmeticError::Underflow)?,
		);
		// increase liquidator's voucher_balance
		AccountDeposits::<T>::try_mutate(
			collateral_asset_id,
			liquidator,
			|deposits| -> DispatchResult {
				deposits.voucher_balance = deposits
					.voucher_balance
					.checked_add(collateral_amount - incentive_reserved_amount)
					.ok_or(ArithmeticError::Overflow)?;
				Ok(())
			},
		)?;
		// increase reserve's voucher_balance
		AccountDeposits::<T>::try_mutate(
			collateral_asset_id,
			Self::incentive_reward_account_id()?,
			|deposits| -> DispatchResult {
				deposits.voucher_balance = deposits
					.voucher_balance
					.checked_add(incentive_reserved_amount)
					.ok_or(ArithmeticError::Overflow)?;
				Ok(())
			},
		)?;

		Self::deposit_event(Event::<T>::LiquidatedBorrow(
			liquidator.clone(),
			borrower.clone(),
			liquidation_asset_id,
			collateral_asset_id,
			repay_amount,
			collateral_underlying_amount,
		));

		Ok(())
	}

	// Ensures a given `asset_id` is an active market.
	fn ensure_active_market(asset_id: AssetIdOf<T>) -> Result<Market<BalanceOf<T>>, DispatchError> {
		Self::active_markets()
			.find(|(id, _)| id == &asset_id)
			.map(|(_, market)| market)
			.ok_or_else(|| Error::<T>::MarketNotActivated.into())
	}

	/// Ensure market is enough to supply `amount` asset.
	fn ensure_under_supply_cap(asset_id: AssetIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		let market = Self::market(asset_id)?;
		// Assets holded by market currently.
		let current_cash = T::Assets::balance(asset_id, &Self::account_id());
		let total_cash = current_cash.checked_add(amount).ok_or(ArithmeticError::Overflow)?;
		ensure!(total_cash <= market.supply_cap, Error::<T>::SupplyCapacityExceeded);

		Ok(())
	}

	/// Make sure the borrowing under the borrow cap
	fn ensure_under_borrow_cap(asset_id: AssetIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		let market = Self::market(asset_id)?;
		let total_borrows = TotalBorrows::<T>::get(asset_id);
		let new_total_borrows =
			total_borrows.checked_add(amount).ok_or(ArithmeticError::Overflow)?;
		ensure!(new_total_borrows <= market.borrow_cap, Error::<T>::BorrowCapacityExceeded);

		Ok(())
	}

	/// Make sure there is enough cash available in the pool
	fn ensure_enough_cash(asset_id: AssetIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		let reducible_cash = Self::get_total_cash(asset_id)
			.checked_sub(TotalReserves::<T>::get(asset_id))
			.ok_or(ArithmeticError::Underflow)?;
		if reducible_cash < amount {
			return Err(Error::<T>::InsufficientCash.into());
		}

		Ok(())
	}

	// Ensures a given `lend_token_id` is unique in `Markets` and `UnderlyingAssetId`.
	fn ensure_lend_token(lend_token_id: CurrencyId) -> DispatchResult {
		// The lend token id is unique, cannot be repeated
		ensure!(
			!UnderlyingAssetId::<T>::contains_key(lend_token_id),
			Error::<T>::InvalidLendTokenId
		);

		// The lend token id should not be the same as the id of any asset in markets
		ensure!(!Markets::<T>::contains_key(lend_token_id), Error::<T>::InvalidLendTokenId);

		Ok(())
	}

	// Ensures that `account` have sufficient liquidity to move your assets
	// Returns `Err` If InsufficientLiquidity
	// `account`: account that need a liquidity check
	// `reduce_amount`: values that will have an impact on liquidity
	// `lf_enable`: check in liquidation free mode which means borrowing dot or redeeming assets in
	// `LiquidationFreeCollaterals`.
	fn ensure_liquidity(
		account: &T::AccountId,
		reduce_amount: FixedU128,
		lf_enable: bool,
	) -> DispatchResult {
		let (total_liquidity, _, lf_liquidity, _) = Self::get_account_liquidity(account)?;

		if lf_enable && max(total_liquidity, lf_liquidity) >= reduce_amount {
			return Ok(());
		}

		if !lf_enable && total_liquidity >= lf_liquidity + reduce_amount {
			return Ok(());
		}

		Err(Error::<T>::InsufficientLiquidity.into())
	}

	fn ensure_liquidity_for_market_bond(
		borrow_asset_id: AssetIdOf<T>,
		account: &T::AccountId,
		reduce_amount: FixedU128,
	) -> DispatchResult {
		let collateral_asset_ids = MarketBond::<T>::try_get(borrow_asset_id)
			.map_err(|_err| Error::<T>::MarketBondDoesNotExist)?;

		let currency_borrow_amount = Self::current_borrow_balance(account, borrow_asset_id)?;
		let total_borrow_value = Self::get_asset_value(borrow_asset_id, currency_borrow_amount)?;

		let mut total_collateral_value: FixedU128 = FixedU128::zero();
		for asset_id in collateral_asset_ids {
			total_collateral_value = total_collateral_value
				.checked_add(&Self::collateral_asset_value(account, asset_id)?)
				.ok_or(ArithmeticError::Overflow)?;
		}

		let total_liquidity = total_collateral_value
			.checked_sub(&total_borrow_value)
			.ok_or(ArithmeticError::Underflow)?;

		if total_liquidity >= reduce_amount {
			return Ok(());
		}

		Err(Error::<T>::InsufficientLiquidity.into())
	}

	pub fn calc_underlying_amount(
		voucher_amount: BalanceOf<T>,
		exchange_rate: Rate,
	) -> Result<BalanceOf<T>, DispatchError> {
		Ok(exchange_rate.checked_mul_int(voucher_amount).ok_or(ArithmeticError::Overflow)?)
	}

	pub fn calc_collateral_amount(
		underlying_amount: BalanceOf<T>,
		exchange_rate: Rate,
	) -> Result<BalanceOf<T>, DispatchError> {
		Ok(FixedU128::from_inner(underlying_amount)
			.checked_div(&exchange_rate)
			.map(|r| r.into_inner())
			.ok_or(ArithmeticError::Underflow)?)
	}

	fn get_total_cash(asset_id: AssetIdOf<T>) -> BalanceOf<T> {
		T::Assets::reducible_balance(
			asset_id,
			&Self::account_id(),
			Preservation::Expendable,
			Fortitude::Polite,
		)
	}

	// Returns the uniform format price.
	// Formula: `price = oracle_price * 10.pow(18 - asset_decimal)`
	// This particular price makes it easy to calculate the value ,
	// because we don't have to consider decimal for each asset. ref: get_asset_value
	//
	// Returns `Err` if the oracle price not ready
	pub fn get_price(asset_id: AssetIdOf<T>) -> Result<Price, DispatchError> {
		let (price, _) =
			T::OraclePriceProvider::get_price(&asset_id).ok_or(Error::<T>::PriceOracleNotReady)?;
		if price.is_zero() {
			return Err(Error::<T>::PriceIsZero.into());
		}
		log::trace!(
			target: "lend-market::get_price", "price: {:?}", price.into_inner()
		);

		Ok(price)
	}

	// Returns the value of the asset, in dollars.
	// Formula: `value = oracle_price * balance / 1e18(oracle_price_decimal) / asset_decimal`
	// As the price is a result of `oracle_price * 10.pow(18 - asset_decimal)`,
	// then `value = price * balance / 1e18`.
	// We use FixedU128::from_inner(balance) instead of `balance / 1e18`.
	//
	// Returns `Err` if oracle price not ready or arithmetic error.
	pub fn get_asset_value(
		asset_id: AssetIdOf<T>,
		amount: BalanceOf<T>,
	) -> Result<FixedU128, DispatchError> {
		let value = Self::get_price(asset_id)?
			.checked_mul(&FixedU128::from_inner(amount))
			.ok_or(ArithmeticError::Overflow)?;

		Ok(value)
	}

	// Returns a stored Market.
	//
	// Returns `Err` if market does not exist.
	pub fn market(asset_id: AssetIdOf<T>) -> Result<Market<BalanceOf<T>>, DispatchError> {
		Markets::<T>::try_get(asset_id).map_err(|_err| Error::<T>::MarketDoesNotExist.into())
	}

	// Mutates a stored Market.
	//
	// Returns `Err` if market does not exist.
	pub(crate) fn mutate_market<F>(
		asset_id: AssetIdOf<T>,
		cb: F,
	) -> Result<Market<BalanceOf<T>>, DispatchError>
	where
		F: FnOnce(&mut Market<BalanceOf<T>>) -> Market<BalanceOf<T>>,
	{
		Markets::<T>::try_mutate(asset_id, |opt| -> Result<Market<BalanceOf<T>>, DispatchError> {
			if let Some(market) = opt {
				return Ok(cb(market));
			}
			Err(Error::<T>::MarketDoesNotExist.into())
		})
	}

	// All markets that are `MarketStatus::Active`.
	fn active_markets() -> impl Iterator<Item = (AssetIdOf<T>, Market<BalanceOf<T>>)> {
		Markets::<T>::iter().filter(|(_, market)| market.state == MarketState::Active)
	}

	// Returns a stored asset_id
	//
	// Returns `Err` if asset_id does not exist, it also means that lend_token_id is invalid.
	pub fn underlying_id(lend_token_id: AssetIdOf<T>) -> Result<AssetIdOf<T>, DispatchError> {
		UnderlyingAssetId::<T>::try_get(lend_token_id)
			.map_err(|_err| Error::<T>::InvalidLendTokenId.into())
	}

	// Returns the lend_token_id of the related asset
	//
	// Returns `Err` if market does not exist.
	pub fn lend_token_id(asset_id: AssetIdOf<T>) -> Result<AssetIdOf<T>, DispatchError> {
		if let Ok(market) = Self::market(asset_id) {
			Ok(market.lend_token_id)
		} else {
			Err(Error::<T>::MarketDoesNotExist.into())
		}
	}

	// Returns the incentive reward account
	pub fn incentive_reward_account_id() -> Result<T::AccountId, DispatchError> {
		let account_id: T::AccountId = T::PalletId::get().into_account_truncating();
		let entropy = (b"lend-market/incentive", &[account_id]).using_encoded(blake2_256);
		Ok(T::AccountId::decode(&mut &entropy[..]).map_err(|_| Error::<T>::CodecError)?)
	}

	pub fn do_redeem_all(
		who: &AccountIdOf<T>,
		asset_id: AssetIdOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		Self::ensure_active_market(asset_id)?;
		Self::accrue_interest(asset_id)?;
		let exchange_rate = Self::exchange_rate_stored(asset_id)?;
		Self::update_earned_stored(&who, asset_id, exchange_rate)?;
		let deposits = AccountDeposits::<T>::get(asset_id, &who);
		let redeem_amount = Self::do_redeem_voucher(&who, asset_id, deposits.voucher_balance)?;
		Self::deposit_event(Event::<T>::Redeemed(who.clone(), asset_id, redeem_amount));
		Ok(redeem_amount)
	}
}

impl<T: Config> LendMarketTrait<AssetIdOf<T>, AccountIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn do_mint(
		supplier: &AccountIdOf<T>,
		asset_id: AssetIdOf<T>,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		ensure!(!amount.is_zero(), Error::<T>::InvalidAmount);
		Self::ensure_active_market(asset_id)?;
		Self::ensure_under_supply_cap(asset_id, amount)?;

		Self::accrue_interest(asset_id)?;

		// update supply index before modify supply balance.
		Self::update_reward_supply_index(asset_id)?;
		Self::distribute_supplier_reward(asset_id, supplier)?;

		let exchange_rate = Self::exchange_rate_stored(asset_id)?;
		Self::update_earned_stored(supplier, asset_id, exchange_rate)?;
		let voucher_amount = Self::calc_collateral_amount(amount, exchange_rate)?;
		ensure!(!voucher_amount.is_zero(), Error::<T>::InvalidExchangeRate);

		T::Assets::transfer(
			asset_id,
			supplier,
			&Self::account_id(),
			amount,
			Preservation::Expendable,
		)?;
		AccountDeposits::<T>::try_mutate(asset_id, supplier, |deposits| -> DispatchResult {
			deposits.voucher_balance = deposits
				.voucher_balance
				.checked_add(voucher_amount)
				.ok_or(ArithmeticError::Overflow)?;
			Ok(())
		})?;
		TotalSupply::<T>::try_mutate(asset_id, |total_balance| -> DispatchResult {
			let new_balance =
				total_balance.checked_add(voucher_amount).ok_or(ArithmeticError::Overflow)?;
			*total_balance = new_balance;
			Ok(())
		})?;
		Self::deposit_event(Event::<T>::Deposited(supplier.clone(), asset_id, amount));
		Ok(())
	}

	fn do_borrow(
		borrower: &AccountIdOf<T>,
		asset_id: AssetIdOf<T>,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		Self::ensure_active_market(asset_id)?;

		Self::accrue_interest(asset_id)?;
		Self::borrow_allowed_for_market_bond(asset_id, borrower, amount)?;
		Self::borrow_allowed(asset_id, borrower, amount)?;

		// update borrow index after accrue interest.
		Self::update_reward_borrow_index(asset_id)?;
		Self::distribute_borrower_reward(asset_id, borrower)?;

		let account_borrows = Self::current_borrow_balance(borrower, asset_id)?;
		let account_borrows_new =
			account_borrows.checked_add(amount).ok_or(ArithmeticError::Overflow)?;
		let total_borrows = TotalBorrows::<T>::get(asset_id);
		let total_borrows_new =
			total_borrows.checked_add(amount).ok_or(ArithmeticError::Overflow)?;
		AccountBorrows::<T>::insert(
			asset_id,
			borrower,
			BorrowSnapshot {
				principal: account_borrows_new,
				borrow_index: BorrowIndex::<T>::get(asset_id),
			},
		);
		TotalBorrows::<T>::insert(asset_id, total_borrows_new);
		T::Assets::transfer(
			asset_id,
			&Self::account_id(),
			borrower,
			amount,
			Preservation::Expendable,
		)?;
		Self::deposit_event(Event::<T>::Borrowed(borrower.clone(), asset_id, amount));
		Ok(())
	}

	fn do_collateral_asset(
		supplier: &AccountIdOf<T>,
		asset_id: AssetIdOf<T>,
		enable: bool,
	) -> Result<(), DispatchError> {
		Self::ensure_active_market(asset_id)?;
		ensure!(AccountDeposits::<T>::contains_key(asset_id, supplier), Error::<T>::NoDeposit);
		let mut deposits = AccountDeposits::<T>::get(asset_id, supplier);
		// turn on the collateral button
		if enable {
			deposits.is_collateral = true;
			AccountDeposits::<T>::insert(asset_id, supplier, deposits);
			Self::deposit_event(Event::<T>::CollateralAssetAdded(supplier.clone(), asset_id));
			return Ok(());
		}
		// turn off the collateral button after checking the liquidity
		let total_collateral_value = Self::total_collateral_value(supplier)?;
		let collateral_asset_value = Self::collateral_asset_value(supplier, asset_id)?;
		let total_borrowed_value = Self::total_borrowed_value(supplier)?;
		log::trace!(
			target: "lend-market::collateral_asset",
			"total_collateral_value: {:?}, collateral_asset_value: {:?}, total_borrowed_value: {:?}",
			total_collateral_value.into_inner(),
			collateral_asset_value.into_inner(),
			total_borrowed_value.into_inner(),
		);
		if total_collateral_value <
			total_borrowed_value
				.checked_add(&collateral_asset_value)
				.ok_or(ArithmeticError::Overflow)?
		{
			return Err(Error::<T>::InsufficientLiquidity.into());
		}
		deposits.is_collateral = false;
		AccountDeposits::<T>::insert(asset_id, supplier, deposits);

		Self::deposit_event(Event::<T>::CollateralAssetRemoved(supplier.clone(), asset_id));

		Ok(())
	}

	fn do_repay_borrow(
		borrower: &AccountIdOf<T>,
		asset_id: AssetIdOf<T>,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		Self::ensure_active_market(asset_id)?;
		Self::accrue_interest(asset_id)?;
		let account_borrows = Self::current_borrow_balance(borrower, asset_id)?;
		Self::do_repay_borrow_with_amount(borrower, asset_id, account_borrows, amount)?;
		Self::deposit_event(Event::<T>::RepaidBorrow(borrower.clone(), asset_id, amount));
		Ok(())
	}

	fn do_redeem(
		supplier: &AccountIdOf<T>,
		asset_id: AssetIdOf<T>,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		Self::ensure_active_market(asset_id)?;
		Self::accrue_interest(asset_id)?;
		let exchange_rate = Self::exchange_rate_stored(asset_id)?;
		Self::update_earned_stored(supplier, asset_id, exchange_rate)?;
		let voucher_amount = Self::calc_collateral_amount(amount, exchange_rate)?;
		let redeem_amount = Self::do_redeem_voucher(supplier, asset_id, voucher_amount)?;
		Self::deposit_event(Event::<T>::Redeemed(supplier.clone(), asset_id, redeem_amount));
		Ok(())
	}
}

impl<T: Config> LendMarketMarketDataProvider<AssetIdOf<T>, BalanceOf<T>> for Pallet<T> {
	fn get_market_info(asset_id: AssetIdOf<T>) -> Result<MarketInfo, DispatchError> {
		let market = Self::market(asset_id)?;
		let full_rate =
			Self::get_full_interest_rate(asset_id).ok_or(Error::<T>::InvalidRateModelParam)?;
		Ok(MarketInfo {
			collateral_factor: market.collateral_factor,
			liquidation_threshold: market.liquidation_threshold,
			reserve_factor: market.reserve_factor,
			close_factor: market.close_factor,
			full_rate,
		})
	}

	fn get_market_status(asset_id: AssetIdOf<T>) -> Result<MarketStatus<Balance>, DispatchError> {
		let (
			borrow_rate,
			supply_rate,
			exchange_rate,
			utilization,
			total_borrows,
			total_reserves,
			borrow_index,
		) = Self::get_market_status(asset_id)?;
		Ok(MarketStatus {
			borrow_rate,
			supply_rate,
			exchange_rate,
			utilization,
			total_borrows,
			total_reserves,
			borrow_index,
		})
	}

	fn get_full_interest_rate(asset_id: AssetIdOf<T>) -> Option<Rate> {
		if let Ok(market) = Self::market(asset_id) {
			let rate = match market.rate_model {
				InterestRateModel::Jump(jump) => Some(jump.full_rate),
				_ => None,
			};
			return rate;
		}
		None
	}
}

impl<T: Config> LendMarketPositionDataProvider<AssetIdOf<T>, AccountIdOf<T>, BalanceOf<T>>
	for Pallet<T>
{
	fn get_current_borrow_balance(
		borrower: &AccountIdOf<T>,
		asset_id: AssetIdOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		Self::accrue_interest(asset_id)?;
		Self::current_borrow_balance(borrower, asset_id)
	}

	fn get_current_collateral_balance(
		supplier: &AccountIdOf<T>,
		asset_id: AssetIdOf<T>,
	) -> Result<BalanceOf<T>, DispatchError> {
		Self::current_collateral_balance(supplier, asset_id)
	}
}

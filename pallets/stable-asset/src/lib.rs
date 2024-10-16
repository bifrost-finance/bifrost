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
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod weights;

pub mod migration;

pub use crate::traits::StableAsset;
use frame_support::{dispatch::DispatchResult, ensure, traits::Get, weights::Weight};
use frame_system::pallet_prelude::BlockNumberFor;
use orml_traits::MultiCurrency;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{U256, U512};
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Zero},
	ArithmeticError, DispatchError, SaturatedConversion,
};
use sp_std::prelude::*;

pub type PoolTokenIndex = u32;

pub type StableAssetPoolId = u32;

const NUMBER_OF_ITERATIONS_TO_CONVERGE: i32 = 255; // the number of iterations to sum d and y

#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug, TypeInfo)]
pub struct StableAssetPoolInfo<AssetId, AtLeast64BitUnsigned, Balance, AccountId, BlockNumber> {
	pub pool_id: StableAssetPoolId,
	pub pool_asset: AssetId,
	pub assets: Vec<AssetId>,
	pub precisions: Vec<AtLeast64BitUnsigned>,
	pub mint_fee: AtLeast64BitUnsigned,
	pub swap_fee: AtLeast64BitUnsigned,
	pub redeem_fee: AtLeast64BitUnsigned,
	pub total_supply: Balance,
	pub a: AtLeast64BitUnsigned,
	pub a_block: BlockNumber,
	pub future_a: AtLeast64BitUnsigned,
	pub future_a_block: BlockNumber,
	pub balances: Vec<Balance>,
	pub fee_recipient: AccountId,
	pub account_id: AccountId,
	pub yield_recipient: AccountId,
	pub precision: AtLeast64BitUnsigned,
}

pub trait WeightInfo {
	fn create_pool() -> Weight;
	fn modify_a() -> Weight;
	fn modify_fees() -> Weight;
	fn modify_recipients() -> Weight;
	fn mint(u: u32) -> Weight;
	fn swap(u: u32) -> Weight;
	fn redeem_proportion(u: u32) -> Weight;
	fn redeem_single(u: u32) -> Weight;
	fn redeem_multi(u: u32) -> Weight;
}

pub mod traits {
	use crate::{
		MintResult, PoolTokenIndex, RedeemProportionResult, StableAssetPoolId, StableAssetPoolInfo,
		SwapResult,
	};
	use frame_support::dispatch::DispatchResult;
	use sp_runtime::DispatchError;
	use sp_std::prelude::*;

	pub trait ValidateAssetId<AssetId> {
		fn validate(a: AssetId) -> bool;
	}

	pub trait StableAsset {
		type AssetId;
		type AtLeast64BitUnsigned;
		type Balance;
		type AccountId;
		type BlockNumber;
		type Config: crate::Config;

		fn set_token_rate(
			pool_id: StableAssetPoolId,
			token_rate_info: Vec<(
				Self::AssetId,
				(Self::AtLeast64BitUnsigned, Self::AtLeast64BitUnsigned),
			)>,
		) -> DispatchResult;

		fn get_token_rate(
			pool_id: StableAssetPoolId,
			asset_id: Self::AssetId,
		) -> Option<(Self::AtLeast64BitUnsigned, Self::AtLeast64BitUnsigned)>;

		fn insert_pool(
			pool_id: StableAssetPoolId,
			pool_info: &StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		);

		fn pool_count() -> StableAssetPoolId;

		fn pool(
			id: StableAssetPoolId,
		) -> Option<
			StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		>;

		fn create_pool(
			pool_asset: Self::AssetId,
			assets: Vec<Self::AssetId>,
			precisions: Vec<Self::AtLeast64BitUnsigned>,
			mint_fee: Self::AtLeast64BitUnsigned,
			swap_fee: Self::AtLeast64BitUnsigned,
			redeem_fee: Self::AtLeast64BitUnsigned,
			initial_a: Self::AtLeast64BitUnsigned,
			fee_recipient: Self::AccountId,
			yield_recipient: Self::AccountId,
			precision: Self::AtLeast64BitUnsigned,
		) -> DispatchResult;

		fn mint(
			who: &Self::AccountId,
			pool_id: StableAssetPoolId,
			amounts: Vec<Self::Balance>,
			min_mint_amount: Self::Balance,
		) -> DispatchResult;

		fn swap(
			who: &Self::AccountId,
			pool_id: StableAssetPoolId,
			i: PoolTokenIndex,
			j: PoolTokenIndex,
			dx: Self::Balance,
			min_dy: Self::Balance,
			asset_length: u32,
		) -> sp_std::result::Result<(Self::Balance, Self::Balance), DispatchError>;

		fn redeem_proportion(
			who: &Self::AccountId,
			pool_id: StableAssetPoolId,
			amount: Self::Balance,
			min_redeem_amounts: Vec<Self::Balance>,
		) -> DispatchResult;

		fn redeem_single(
			who: &Self::AccountId,
			pool_id: StableAssetPoolId,
			amount: Self::Balance,
			i: PoolTokenIndex,
			min_redeem_amount: Self::Balance,
			asset_length: u32,
		) -> sp_std::result::Result<(Self::Balance, Self::Balance), DispatchError>;

		fn redeem_multi(
			who: &Self::AccountId,
			pool_id: StableAssetPoolId,
			amounts: Vec<Self::Balance>,
			max_redeem_amount: Self::Balance,
		) -> DispatchResult;

		fn collect_fee(
			pool_id: StableAssetPoolId,
			pool_info: &mut StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		) -> DispatchResult;

		fn update_balance(
			pool_id: StableAssetPoolId,
			pool_info: &mut StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		) -> DispatchResult;

		fn collect_yield(
			pool_id: StableAssetPoolId,
			pool_info: &mut StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		) -> DispatchResult;

		fn modify_a(
			pool_id: StableAssetPoolId,
			a: Self::AtLeast64BitUnsigned,
			future_a_block: Self::BlockNumber,
		) -> DispatchResult;

		fn get_collect_yield_amount(
			pool_info: &StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		) -> Option<
			StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		>;

		fn get_balance_update_amount(
			pool_info: &StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		) -> Option<
			StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
		>;

		fn get_redeem_proportion_amount(
			pool_info: &StableAssetPoolInfo<
				Self::AssetId,
				Self::AtLeast64BitUnsigned,
				Self::Balance,
				Self::AccountId,
				Self::BlockNumber,
			>,
			amount_bal: Self::Balance,
		) -> Option<RedeemProportionResult<Self::Balance>>;

		/// Get the best swap route in all pools
		///  params:
		/// - input_asset: the input asset.
		/// - output_asset: the output asset.
		/// - input_amount: the input amount of input asset.
		fn get_best_route(
			input_asset: Self::AssetId,
			output_asset: Self::AssetId,
			input_amount: Self::Balance,
		) -> Option<(StableAssetPoolId, PoolTokenIndex, PoolTokenIndex, Self::Balance)>;

		/// Get the swap result at exact input amount.
		///  params:
		/// - pool_id: the pool id.
		/// - input_index: the asset index of input asset.
		/// - output_index: the asset index of output asset.
		/// - dx_bal: the input amount.
		fn get_swap_output_amount(
			pool_id: StableAssetPoolId,
			input_index: PoolTokenIndex,
			output_index: PoolTokenIndex,
			dx_bal: Self::Balance,
		) -> Option<SwapResult<Self::Balance>>;

		/// Get the swap result at exact output amount.
		///  params:
		/// - pool_id: the pool id.
		/// - input_index: the asset index of input asset.
		/// - output_index: the asset index of output asset.
		/// - dy_bal: the output amount.
		fn get_swap_input_amount(
			pool_id: StableAssetPoolId,
			input_index: PoolTokenIndex,
			output_index: PoolTokenIndex,
			dy_bal: Self::Balance,
		) -> Option<SwapResult<Self::Balance>>;

		fn get_mint_amount(
			pool_id: StableAssetPoolId,
			amounts_bal: &[Self::Balance],
		) -> Option<MintResult<Self::Config>>;

		fn get_a(
			a0: Self::AtLeast64BitUnsigned,
			t0: Self::BlockNumber,
			a1: Self::AtLeast64BitUnsigned,
			t1: Self::BlockNumber,
		) -> Option<Self::AtLeast64BitUnsigned>;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::{PoolTokenIndex, StableAssetPoolId, StableAssetPoolInfo};
	use crate::{
		traits::{StableAsset, ValidateAssetId},
		WeightInfo,
	};
	use frame_support::{
		dispatch::DispatchResult, pallet_prelude::*, traits::EnsureOrigin, transactional, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use orml_traits::MultiCurrency;
	use parity_scale_codec::Codec;
	use sp_runtime::{
		traits::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, One, Zero},
		FixedPointOperand, Permill,
	};
	use sp_std::prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type AssetId: Parameter + Ord + Copy;
		type Balance: Parameter + Codec + Copy + Ord + From<Self::AtLeast64BitUnsigned> + Zero;
		type Assets: MultiCurrency<
			Self::AccountId,
			CurrencyId = Self::AssetId,
			Balance = Self::Balance,
		>;
		type AtLeast64BitUnsigned: Parameter
			+ CheckedAdd
			+ CheckedSub
			+ CheckedMul
			+ CheckedDiv
			+ Copy
			+ Eq
			+ Ord
			+ From<Self::Balance>
			+ From<u8>
			+ From<u128>
			+ From<BlockNumberFor<Self>>
			+ TryFrom<usize>
			+ Zero
			+ One
			+ FixedPointOperand;
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		#[pallet::constant]
		type FeePrecision: Get<Self::AtLeast64BitUnsigned>;
		#[pallet::constant]
		type SwapExactOverAmount: Get<Self::AtLeast64BitUnsigned>;
		#[pallet::constant]
		type APrecision: Get<Self::AtLeast64BitUnsigned>;
		#[pallet::constant]
		type PoolAssetLimit: Get<u32>;
		type WeightInfo: WeightInfo;
		type EnsurePoolAssetId: ValidateAssetId<Self::AssetId>;

		/// The origin which may create pool or modify pool.
		type ListingOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The last pool id.
	#[pallet::storage]
	pub type PoolCount<T: Config> = StorageValue<_, StableAssetPoolId, ValueQuery>;

	/// The pool info.
	#[pallet::storage]
	pub type Pools<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		StableAssetPoolId,
		StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
	>;

	/// Price anchor used to bind the corresponding pool and currency.
	#[pallet::storage]
	pub type TokenRateCaches<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		StableAssetPoolId,
		Twox64Concat,
		T::AssetId,
		(T::AtLeast64BitUnsigned, T::AtLeast64BitUnsigned),
	>;

	/// Record the maximum percentage that can exceed the token rate.
	#[pallet::storage]
	pub type TokenRateHardcap<T: Config> = StorageMap<_, Twox64Concat, T::AssetId, Permill>;

	#[pallet::event]
	#[pallet::generate_deposit(pub fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new pool is created.
		CreatePool {
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// Amplification coefficient of the pool.
			a: T::AtLeast64BitUnsigned,
			/// The system account of the pool.
			swap_id: T::AccountId,
			/// Pallet id.
			pallet_id: T::AccountId,
		},
		/// Liquidity is added to the pool.
		LiquidityAdded {
			/// The account who added the liquidity.
			minter: T::AccountId,
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// Amplification coefficient of the pool.
			a: T::AtLeast64BitUnsigned,
			/// The input amounts of the assets.
			input_amounts: Vec<T::Balance>,
			/// Expected minimum output amount.
			min_output_amount: T::Balance,
			/// Balances data.
			balances: Vec<T::Balance>,
			/// The total supply of the pool asset.
			total_supply: T::Balance,
			/// fee amount of the pool asset.
			fee_amount: T::Balance,
			/// Actual minimum output amount.
			output_amount: T::Balance,
		},
		/// Token is swapped.
		TokenSwapped {
			/// The account who swapped the token.
			swapper: T::AccountId,
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// Amplification coefficient of the pool.
			a: T::AtLeast64BitUnsigned,
			/// The input asset type.
			input_asset: T::AssetId,
			/// The output asset type.
			output_asset: T::AssetId,
			/// The input amount of the input asset.
			input_amount: T::Balance,
			/// The expected minimum output amount of the output asset.
			min_output_amount: T::Balance,
			/// Balances data.
			balances: Vec<T::Balance>,
			/// The total supply of the pool asset.
			total_supply: T::Balance,
			/// Actual output amount of the output asset.
			output_amount: T::Balance,
		},
		/// Token is redeemed by proportion.
		RedeemedProportion {
			/// The account who redeemed the token.
			redeemer: T::AccountId,
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// Amplification coefficient of the pool.
			a: T::AtLeast64BitUnsigned,
			/// The input amount of the pool asset.
			input_amount: T::Balance,
			/// The expected minimum output amounts of the assets.
			min_output_amounts: Vec<T::Balance>,
			/// Balances data.
			balances: Vec<T::Balance>,
			/// The total supply of the pool asset.
			total_supply: T::Balance,
			/// fee amount of the pool asset.
			fee_amount: T::Balance,
			/// Actual output amounts of the assets.
			output_amounts: Vec<T::Balance>,
		},
		/// Token is redeemed by single asset.
		RedeemedSingle {
			/// The account who redeemed the token.
			redeemer: T::AccountId,
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// Amplification coefficient of the pool.
			a: T::AtLeast64BitUnsigned,
			/// The input asset type.
			input_amount: T::Balance,
			/// The output asset type.
			output_asset: T::AssetId,
			/// The expected minimum output amount of the output asset.
			min_output_amount: T::Balance,
			/// Balances data.
			balances: Vec<T::Balance>,
			/// The total supply of the pool asset.
			total_supply: T::Balance,
			/// fee amount of the pool asset.
			fee_amount: T::Balance,
			/// Actual output amount of the output asset.
			output_amount: T::Balance,
		},
		/// Token is redeemed by multiple assets.
		RedeemedMulti {
			/// The account who redeemed the token.
			redeemer: T::AccountId,
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// Amplification coefficient of the pool.
			a: T::AtLeast64BitUnsigned,
			/// The expected output amounts.
			output_amounts: Vec<T::Balance>,
			/// The maximum input amount of the pool asset to get the output amounts.
			max_input_amount: T::Balance,
			/// Balances data.
			balances: Vec<T::Balance>,
			/// The total supply of the pool asset.
			total_supply: T::Balance,
			/// fee amount of the pool asset.
			fee_amount: T::Balance,
			/// Actual input amount of the pool asset.
			input_amount: T::Balance,
		},
		/// The pool field balances is updated.
		BalanceUpdated {
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// The old balances.
			old_balances: Vec<T::Balance>,
			/// The new balances.
			new_balances: Vec<T::Balance>,
		},
		/// Yield is collected.
		YieldCollected {
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// Amplification coefficient of the pool.
			a: T::AtLeast64BitUnsigned,
			/// The old total supply of the pool asset.
			old_total_supply: T::Balance,
			/// The new total supply of the pool asset.
			new_total_supply: T::Balance,
			/// The account who collected the yield.
			who: T::AccountId,
			/// The amount of the pool asset collected.
			amount: T::Balance,
		},
		/// Fee is collected.
		FeeCollected {
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// Amplification coefficient of the pool.
			a: T::AtLeast64BitUnsigned,
			/// The old balances.
			old_balances: Vec<T::Balance>,
			/// The new balances.
			new_balances: Vec<T::Balance>,
			/// The old total supply of the pool asset.
			old_total_supply: T::Balance,
			/// The new total supply of the pool asset.
			new_total_supply: T::Balance,
			/// The account who has been collected the fee.
			who: T::AccountId,
			/// The fee amount of the pool asset.
			amount: T::Balance,
		},
		/// The pool amplification coefficient is modified.
		AModified {
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// The new amplification coefficient.
			value: T::AtLeast64BitUnsigned,
			/// The block number when the new amplification coefficient will be effective.
			time: BlockNumberFor<T>,
		},
		/// The pool fees are modified.
		FeeModified {
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// The new mint fee.
			mint_fee: T::AtLeast64BitUnsigned,
			/// The new swap fee.
			swap_fee: T::AtLeast64BitUnsigned,
			/// The new redeem fee.
			redeem_fee: T::AtLeast64BitUnsigned,
		},
		/// The pool recipients are modified.
		RecipientModified {
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// The new fee recipient.
			fee_recipient: T::AccountId,
			/// The new yield recipient.
			yield_recipient: T::AccountId,
		},
		/// The token rate is set.
		TokenRateSet {
			/// The pool id.
			pool_id: StableAssetPoolId,
			/// The token rate info[(currency_id, (denominator, numerator))].
			token_rate: Vec<(T::AssetId, (T::AtLeast64BitUnsigned, T::AtLeast64BitUnsigned))>,
		},
		/// The hardcap of the token rate is configured.
		TokenRateHardcapConfigured {
			/// The token type.
			vtoken: T::AssetId,
			/// The hardcap of the token rate.
			hardcap: Permill,
		},
		/// The hardcap of the token rate is removed.
		TokenRateHardcapRemoved {
			/// The token type.
			vtoken: T::AssetId,
		},
		/// The token rate is refreshed.
		TokenRateRefreshFailed {
			/// The pool id.
			pool_id: StableAssetPoolId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The pool is existed, cannot create again.
		InconsistentStorage,
		/// The pool asset is invalid.
		InvalidPoolAsset,
		/// The arguments are mismatch, not match the expected length.
		ArgumentsMismatch,
		/// The arguments are error.
		ArgumentsError,
		/// The pool is not found, cannot modify.
		PoolNotFound,
		/// make mistakes in calculation.
		Math,
		/// The new invariant of the pool is invalid.
		InvalidPoolValue,
		/// The actual output amount is less than the expected minimum output amount when add
		/// liquidity.
		MintUnderMin,
		/// The actual output amount is less than the expected minimum output amount when swap.
		SwapUnderMin,
		/// The actual output amount is less than the expected minimum output amount when redeem.
		RedeemUnderMin,
		/// The actual input amount is more than the expected maximum input amount when redeem
		/// multi.
		RedeemOverMax,
		/// The old token rate is not cleared.
		TokenRateNotCleared,
	}

	/// The add liquidity result.
	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct MintResult<T: Config> {
		/// The amount of the pool asset that the user will get.
		pub mint_amount: T::Balance,
		/// The fee amount of the pool asset that the user will pay.
		pub fee_amount: T::Balance,
		/// The balances data.
		pub balances: Vec<T::Balance>,
		/// The total supply of the pool asset.
		pub total_supply: T::Balance,
	}

	/// The swap result.
	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct SwapResult<Balance> {
		/// the input amount.
		pub dx: Balance,
		/// the output amount.
		pub dy: Balance,
		/// the output amount in balances field.
		pub y: Balance,
		/// the input amount in balances field.
		pub balance_i: Balance,
	}

	/// The redeem proportion result.
	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct RedeemProportionResult<Balance> {
		/// The amounts of the assets that the user will get.
		pub amounts: Vec<Balance>,
		/// Balances data.
		pub balances: Vec<Balance>,
		/// The fee amount of the pool asset that the user will pay.
		pub fee_amount: Balance,
		/// The total supply of the pool asset.
		pub total_supply: Balance,
		/// The amount of the pool asset that the user want to redeem.
		pub redeem_amount: Balance,
	}

	/// The redeem single result.
	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct RedeemSingleResult<T: Config> {
		/// The amount of the token index i that the user will get.
		pub dy: T::Balance,
		/// The fee amount of the pool asset that the user will pay.
		pub fee_amount: T::Balance,
		/// The total supply of the pool asset.
		pub total_supply: T::Balance,
		/// The balances data.
		pub balances: Vec<T::Balance>,
		/// The amount of the pool asset that the user want to redeem.
		pub redeem_amount: T::Balance,
	}

	/// The redeem multi result.
	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct RedeemMultiResult<T: Config> {
		/// The amount of the pool asset that the user should redeemed.
		pub redeem_amount: T::Balance,
		/// The fee amount of the pool asset that the user will pay.
		pub fee_amount: T::Balance,
		/// The balances data.
		pub balances: Vec<T::Balance>,
		/// The total supply of the pool asset.
		pub total_supply: T::Balance,
		/// The amount of the pool asset that the user should redeemed except fee amount.
		pub burn_amount: T::Balance,
	}

	/// The pending fee result.
	#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, Debug)]
	pub struct PendingFeeResult<T: Config> {
		/// The fee amount of the pool asset that the user will pay.
		pub fee_amount: T::Balance,
		/// The balances data.
		pub balances: Vec<T::Balance>,
		/// The total supply of the pool asset.
		pub total_supply: T::Balance,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_pool())]
		#[transactional]
		pub fn create_pool(
			origin: OriginFor<T>,
			pool_asset: T::AssetId,
			assets: Vec<T::AssetId>,
			precisions: Vec<T::AtLeast64BitUnsigned>,
			mint_fee: T::AtLeast64BitUnsigned,
			swap_fee: T::AtLeast64BitUnsigned,
			redeem_fee: T::AtLeast64BitUnsigned,
			initial_a: T::AtLeast64BitUnsigned,
			fee_recipient: T::AccountId,
			yield_recipient: T::AccountId,
			precision: T::AtLeast64BitUnsigned,
		) -> DispatchResult {
			T::ListingOrigin::ensure_origin(origin.clone())?;
			ensure!(T::EnsurePoolAssetId::validate(pool_asset), Error::<T>::InvalidPoolAsset);
			<Self as StableAsset>::create_pool(
				pool_asset,
				assets,
				precisions,
				mint_fee,
				swap_fee,
				redeem_fee,
				initial_a,
				fee_recipient,
				yield_recipient,
				precision,
			)
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::mint(amounts.len() as u32))]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amounts: Vec<T::Balance>,
			min_mint_amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAsset>::mint(&who, pool_id, amounts, min_mint_amount)
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::swap(*asset_length))]
		#[transactional]
		pub fn swap(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			i: PoolTokenIndex,
			j: PoolTokenIndex,
			dx: T::Balance,
			min_dy: T::Balance,
			asset_length: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAsset>::swap(&who, pool_id, i, j, dx, min_dy, asset_length)?;
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::redeem_proportion(min_redeem_amounts.len() as u32))]
		#[transactional]
		pub fn redeem_proportion(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amount: T::Balance,
			min_redeem_amounts: Vec<T::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAsset>::redeem_proportion(&who, pool_id, amount, min_redeem_amounts)
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::redeem_single(*asset_length))]
		#[transactional]
		pub fn redeem_single(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amount: T::Balance,
			i: PoolTokenIndex,
			min_redeem_amount: T::Balance,
			asset_length: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAsset>::redeem_single(
				&who,
				pool_id,
				amount,
				i,
				min_redeem_amount,
				asset_length,
			)?;
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::redeem_multi(amounts.len() as u32))]
		#[transactional]
		pub fn redeem_multi(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amounts: Vec<T::Balance>,
			max_redeem_amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Self as StableAsset>::redeem_multi(&who, pool_id, amounts, max_redeem_amount)
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::modify_a())]
		#[transactional]
		pub fn modify_a(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			future_a_block: BlockNumberFor<T>,
		) -> DispatchResult {
			T::ListingOrigin::ensure_origin(origin)?;
			<Self as StableAsset>::modify_a(pool_id, a, future_a_block)
		}

		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::modify_fees())]
		#[transactional]
		pub fn modify_fees(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			mint_fee: Option<T::AtLeast64BitUnsigned>,
			swap_fee: Option<T::AtLeast64BitUnsigned>,
			redeem_fee: Option<T::AtLeast64BitUnsigned>,
		) -> DispatchResult {
			T::ListingOrigin::ensure_origin(origin)?;
			Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
				let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
				if let Some(fee) = mint_fee {
					pool_info.mint_fee = fee;
				}
				if let Some(fee) = swap_fee {
					pool_info.swap_fee = fee;
				}
				if let Some(fee) = redeem_fee {
					pool_info.redeem_fee = fee;
				}
				Self::deposit_event(Event::FeeModified {
					pool_id,
					mint_fee: pool_info.mint_fee,
					swap_fee: pool_info.swap_fee,
					redeem_fee: pool_info.redeem_fee,
				});
				Ok(())
			})
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::modify_recipients())]
		#[transactional]
		pub fn modify_recipients(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			fee_recipient: Option<T::AccountId>,
			yield_recipient: Option<T::AccountId>,
		) -> DispatchResult {
			T::ListingOrigin::ensure_origin(origin)?;
			Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
				let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
				if let Some(recipient) = fee_recipient {
					pool_info.fee_recipient = recipient;
				}
				if let Some(recipient) = yield_recipient {
					pool_info.yield_recipient = recipient;
				}
				Self::deposit_event(Event::RecipientModified {
					pool_id,
					fee_recipient: pool_info.fee_recipient.clone(),
					yield_recipient: pool_info.yield_recipient.clone(),
				});
				Ok(())
			})
		}
	}
}
impl<T: Config> Pallet<T> {
	pub fn convert_vec_number_to_balance(numbers: Vec<T::AtLeast64BitUnsigned>) -> Vec<T::Balance> {
		numbers.into_iter().map(|x| x.into()).collect()
	}

	pub fn convert_vec_balance_to_number(
		balances: Vec<T::Balance>,
	) -> Vec<T::AtLeast64BitUnsigned> {
		balances.into_iter().map(|x| x.into()).collect()
	}

	pub fn get_a(
		a0: T::AtLeast64BitUnsigned,
		t0: BlockNumberFor<T>,
		a1: T::AtLeast64BitUnsigned,
		t1: BlockNumberFor<T>,
	) -> Option<T::AtLeast64BitUnsigned> {
		let current_block = frame_system::Pallet::<T>::block_number();
		if current_block < t1 {
			let time_diff: u128 = current_block.checked_sub(&t0)?.saturated_into();
			let time_diff: T::AtLeast64BitUnsigned = time_diff.into();
			let time_diff_div: u128 = t1.checked_sub(&t0)?.saturated_into();
			let time_diff_div: T::AtLeast64BitUnsigned = time_diff_div.into();
			if a1 > a0 {
				let diff = a1.checked_sub(&a0)?;
				let amount = u128::try_from(
					U256::from(diff.saturated_into::<u128>())
						.checked_mul(U256::from(time_diff.saturated_into::<u128>()))?
						.checked_div(U256::from(time_diff_div.saturated_into::<u128>()))?,
				)
				.ok()?
				.into();
				Some(a0.checked_add(&amount)?)
			} else {
				let diff = a0.checked_sub(&a1)?;
				let amount = u128::try_from(
					U256::from(diff.saturated_into::<u128>())
						.checked_mul(U256::from(time_diff.saturated_into::<u128>()))?
						.checked_div(U256::from(time_diff_div.saturated_into::<u128>()))?,
				)
				.ok()?
				.into();
				Some(a0.checked_sub(&amount)?)
			}
		} else {
			Some(a1)
		}
	}

	pub fn get_d(
		balances: &[T::AtLeast64BitUnsigned],
		a: T::AtLeast64BitUnsigned,
	) -> Option<T::AtLeast64BitUnsigned> {
		let zero: U512 = U512::from(0u128);
		let one: U512 = U512::from(1u128);
		let mut sum: U512 = U512::from(0u128);
		let mut ann: U512 = U512::from(a.saturated_into::<u128>());
		let balance_size: U512 = U512::from(balances.len());
		let a_precision_u256: U512 = U512::from(T::APrecision::get().saturated_into::<u128>());
		for x in balances.iter() {
			let balance: u128 = (*x).saturated_into::<u128>();
			sum = sum.checked_add(balance.into())?;
			ann = ann.checked_mul(balance_size)?;
		}
		if sum == zero {
			return Some(Zero::zero());
		}

		let mut prev_d: U512;
		let mut d: U512 = sum;
		for _i in 0..NUMBER_OF_ITERATIONS_TO_CONVERGE {
			let mut p_d: U512 = d;
			for x in balances.iter() {
				let balance: u128 = (*x).saturated_into::<u128>();
				let div_op = U512::from(balance).checked_mul(balance_size)?;
				p_d = p_d.checked_mul(d)?.checked_div(div_op)?;
			}
			prev_d = d;
			let t1: U512 = p_d.checked_mul(balance_size)?;
			let t2: U512 = balance_size.checked_add(one)?.checked_mul(p_d)?;
			let t3: U512 = ann
				.checked_sub(a_precision_u256)?
				.checked_mul(d)?
				.checked_div(a_precision_u256)?
				.checked_add(t2)?;
			d = ann
				.checked_mul(sum)?
				.checked_div(a_precision_u256)?
				.checked_add(t1)?
				.checked_mul(d)?
				.checked_div(t3)?;
			if d > prev_d {
				if d - prev_d <= one {
					break;
				}
			} else if prev_d - d <= one {
				break;
			}
		}
		let result: u128 = u128::try_from(d).ok()?;
		Some(result.into())
	}

	pub fn get_y(
		balances: &[T::AtLeast64BitUnsigned],
		token_index: PoolTokenIndex,
		target_d: T::AtLeast64BitUnsigned,
		amplitude: T::AtLeast64BitUnsigned,
	) -> Option<T::AtLeast64BitUnsigned> {
		let one: U512 = U512::from(1u128);
		let two: U512 = U512::from(2u128);
		let mut c: U512 = U512::from(target_d.saturated_into::<u128>());
		let mut sum: U512 = U512::from(0u128);
		let mut ann: U512 = U512::from(amplitude.saturated_into::<u128>());
		let balance_size: U512 = U512::from(balances.len());
		let target_d_u256: U512 = U512::from(target_d.saturated_into::<u128>());
		let a_precision_u256: U512 = U512::from(T::APrecision::get().saturated_into::<u128>());

		for (i, balance_ref) in balances.iter().enumerate() {
			let balance: U512 = U512::from((*balance_ref).saturated_into::<u128>());
			ann = ann.checked_mul(balance_size)?;
			let token_index_usize = token_index as usize;
			if i == token_index_usize {
				continue;
			}
			sum = sum.checked_add(balance)?;
			let div_op: U512 = balance.checked_mul(balance_size)?;
			c = c.checked_mul(target_d_u256)?.checked_div(div_op)?
		}

		c = c
			.checked_mul(target_d_u256)?
			.checked_mul(a_precision_u256)?
			.checked_div(ann.checked_mul(balance_size)?)?;
		let b: U512 =
			sum.checked_add(target_d_u256.checked_mul(a_precision_u256)?.checked_div(ann)?)?;
		let mut prev_y: U512;
		let mut y: U512 = target_d_u256;

		for _i in 0..NUMBER_OF_ITERATIONS_TO_CONVERGE {
			prev_y = y;
			y = y
				.checked_mul(y)?
				.checked_add(c)?
				.checked_div(y.checked_mul(two)?.checked_add(b)?.checked_sub(target_d_u256)?)?;
			if y > prev_y {
				if y - prev_y <= one {
					break;
				}
			} else if prev_y - y <= one {
				break;
			}
		}
		let result: u128 = u128::try_from(y).ok()?;
		Some(result.into())
	}

	pub fn get_mint_amount(
		pool_info: &StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
		amounts_bal: &[T::Balance],
	) -> Result<MintResult<T>, Error<T>> {
		// update pool balances and total supply to avoid stale data
		let pool_info = Self::get_balance_update_amount(pool_info)?;
		let pool_info = Self::get_collect_yield_amount(&pool_info)?;

		if pool_info.balances.len() != amounts_bal.len() {
			return Err(Error::<T>::ArgumentsMismatch);
		}

		let amounts = Self::convert_vec_balance_to_number(amounts_bal.to_vec());
		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let old_d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();

		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		for i in 0..balances.len() {
			if amounts[i] == zero {
				if old_d == zero {
					return Err(Error::<T>::ArgumentsError);
				}
				continue;
			}
			let result: T::AtLeast64BitUnsigned = balances[i]
				.checked_add(
					&amounts[i].checked_mul(&pool_info.precisions[i]).ok_or(Error::<T>::Math)?,
				)
				.ok_or(Error::<T>::Math)?;
			balances[i] = result;
		}
		let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;
		let mut mint_amount: T::AtLeast64BitUnsigned =
			new_d.checked_sub(&old_d).ok_or(Error::<T>::Math)?;
		let mut fee_amount: T::AtLeast64BitUnsigned = zero;
		let mint_fee: T::AtLeast64BitUnsigned = pool_info.mint_fee;

		if pool_info.mint_fee > zero {
			fee_amount = u128::try_from(
				U256::from(mint_amount.saturated_into::<u128>())
					.checked_mul(U256::from(mint_fee.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?
					.checked_div(U256::from(fee_denominator.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?,
			)
			.map_err(|_| Error::<T>::Math)?
			.into();

			mint_amount = mint_amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
		}

		Ok(MintResult {
			mint_amount: mint_amount.into(),
			fee_amount: fee_amount.into(),
			balances: Self::convert_vec_number_to_balance(balances),
			total_supply: new_d.into(),
		})
	}

	pub fn get_swap_amount(
		pool_info: &StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
		input_index: PoolTokenIndex,
		output_index: PoolTokenIndex,
		dx_bal: T::Balance,
	) -> Result<SwapResult<T::Balance>, Error<T>> {
		// update pool balances and total supply to avoid stale data
		let pool_info = Self::get_balance_update_amount(pool_info)?;
		let pool_info = Self::get_collect_yield_amount(&pool_info)?;

		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		let one: T::AtLeast64BitUnsigned = One::one();
		let balance_size: usize = pool_info.balances.len();
		let dx: T::AtLeast64BitUnsigned = dx_bal.into();
		let input_index_usize = input_index as usize;
		let output_index_usize = output_index as usize;
		if input_index == output_index {
			return Err(Error::<T>::ArgumentsError);
		}
		if dx <= zero {
			return Err(Error::<T>::ArgumentsError);
		}
		if input_index_usize >= balance_size {
			return Err(Error::<T>::ArgumentsError);
		}
		if output_index_usize >= balance_size {
			return Err(Error::<T>::ArgumentsError);
		}

		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		balances[input_index_usize] = balances[input_index_usize]
			.checked_add(
				&dx.checked_mul(&pool_info.precisions[input_index_usize])
					.ok_or(Error::<T>::Math)?,
			)
			.ok_or(Error::<T>::Math)?;
		let y: T::AtLeast64BitUnsigned =
			Self::get_y(&balances, output_index, d, a).ok_or(Error::<T>::Math)?;
		let mut dy: T::AtLeast64BitUnsigned = balances[output_index_usize]
			.checked_sub(&y)
			.ok_or(Error::<T>::Math)?
			.checked_sub(&one)
			.ok_or(Error::<T>::Math)?
			.checked_div(&pool_info.precisions[output_index_usize])
			.ok_or(Error::<T>::Math)?;
		if pool_info.swap_fee > zero {
			let fee_amount = u128::try_from(
				U256::from(dy.saturated_into::<u128>())
					.checked_mul(U256::from(pool_info.swap_fee.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?
					.checked_div(U256::from(fee_denominator.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?,
			)
			.map_err(|_| Error::<T>::Math)?
			.into();
			dy = dy.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
		}
		Ok(SwapResult {
			dx: dx_bal,
			dy: dy.into(),
			y: y.into(),
			balance_i: balances[input_index_usize].into(),
		})
	}

	pub fn get_swap_amount_exact(
		pool_info: &StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
		input_index: PoolTokenIndex,
		output_index: PoolTokenIndex,
		dy_bal: T::Balance,
	) -> Option<SwapResult<T::Balance>> {
		// update pool balances and total supply to avoid stale data
		let pool_info = Self::get_balance_update_amount(pool_info).ok()?;
		let pool_info = Self::get_collect_yield_amount(&pool_info).ok()?;

		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		let one: T::AtLeast64BitUnsigned = One::one();
		let balance_size: usize = pool_info.balances.len();
		let mut dy: T::AtLeast64BitUnsigned = dy_bal.into();
		let input_index_usize = input_index as usize;
		let output_index_usize = output_index as usize;
		if input_index == output_index {
			return None;
		}
		if dy <= zero {
			return None;
		}
		if input_index_usize >= balance_size {
			return None;
		}
		if output_index_usize >= balance_size {
			return None;
		}
		let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();
		let swap_exact_over_amount = T::SwapExactOverAmount::get();
		if pool_info.swap_fee > zero {
			let diff = fee_denominator.checked_sub(&pool_info.swap_fee)?;
			dy = u128::try_from(
				U256::from(dy.saturated_into::<u128>())
					.checked_mul(U256::from(fee_denominator.saturated_into::<u128>()))?
					.checked_div(U256::from(diff.saturated_into::<u128>()))?,
			)
			.ok()?
			.into();
		}

		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)?;
		let d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		balances[output_index_usize] = balances[output_index_usize]
			.checked_sub(&dy.checked_mul(&pool_info.precisions[output_index_usize])?)?;
		let y: T::AtLeast64BitUnsigned = Self::get_y(&balances, input_index, d, a)?;
		let dx: T::AtLeast64BitUnsigned = y
			.checked_sub(&balances[input_index_usize])?
			.checked_sub(&one)?
			.checked_div(&pool_info.precisions[input_index_usize])?
			.checked_add(&swap_exact_over_amount)?;

		Some(SwapResult {
			dx: dx.into(),
			dy: dy_bal,
			y: y.into(),
			balance_i: balances[input_index_usize].into(),
		})
	}

	pub fn get_redeem_proportion_amount(
		pool_info: &StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
		amount_bal: T::Balance,
	) -> Result<RedeemProportionResult<T::Balance>, Error<T>> {
		// update pool balances and total supply to avoid stale data
		let pool_info = Self::get_balance_update_amount(pool_info)?;
		let pool_info = Self::get_collect_yield_amount(&pool_info)?;

		let mut amount: T::AtLeast64BitUnsigned = amount_bal.into();
		let zero: T::AtLeast64BitUnsigned = Zero::zero();

		if amount <= zero {
			return Err(Error::<T>::ArgumentsError);
		}

		let d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let mut amounts: Vec<T::AtLeast64BitUnsigned> = Vec::new();
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();

		let mut fee_amount: T::AtLeast64BitUnsigned = zero;
		if pool_info.redeem_fee > zero {
			fee_amount = u128::try_from(
				U256::from(amount.saturated_into::<u128>())
					.checked_mul(U256::from(pool_info.redeem_fee.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?
					.checked_div(U256::from(fee_denominator.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?,
			)
			.map_err(|_| Error::<T>::Math)?
			.into();
			// Redemption fee is charged with pool token before redemption.
			amount = amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
		}

		for i in 0..pool_info.balances.len() {
			let balance_i: T::AtLeast64BitUnsigned = balances[i];
			let diff_i: T::AtLeast64BitUnsigned = u128::try_from(
				U256::from(balance_i.saturated_into::<u128>())
					.checked_mul(U256::from(amount.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?
					.checked_div(U256::from(d.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?,
			)
			.map_err(|_| Error::<T>::Math)?
			.into();
			balances[i] = balance_i.checked_sub(&diff_i).ok_or(Error::<T>::Math)?;
			let amounts_i: T::AtLeast64BitUnsigned =
				diff_i.checked_div(&pool_info.precisions[i]).ok_or(Error::<T>::Math)?;
			amounts.push(amounts_i);
		}
		let total_supply: T::AtLeast64BitUnsigned =
			d.checked_sub(&amount).ok_or(Error::<T>::Math)?;
		Ok(RedeemProportionResult {
			amounts: Self::convert_vec_number_to_balance(amounts),
			balances: Self::convert_vec_number_to_balance(balances),
			fee_amount: fee_amount.into(),
			total_supply: total_supply.into(),
			redeem_amount: amount.into(),
		})
	}

	pub fn get_redeem_single_amount(
		pool_info: &StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
		amount_bal: T::Balance,
		i: PoolTokenIndex,
	) -> Result<RedeemSingleResult<T>, Error<T>> {
		// update pool balances and total supply to avoid stale data
		let pool_info = Self::get_balance_update_amount(pool_info)?;
		let pool_info = Self::get_collect_yield_amount(&pool_info)?;

		let mut amount: T::AtLeast64BitUnsigned = amount_bal.into();
		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		let one: T::AtLeast64BitUnsigned = One::one();
		let i_usize = i as usize;
		if amount <= zero {
			return Err(Error::<T>::ArgumentsError);
		}
		if i_usize >= pool_info.balances.len() {
			return Err(Error::<T>::ArgumentsError);
		}
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();
		let mut fee_amount: T::AtLeast64BitUnsigned = zero;

		if pool_info.redeem_fee > zero {
			fee_amount = u128::try_from(
				U256::from(amount.saturated_into::<u128>())
					.checked_mul(U256::from(pool_info.redeem_fee.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?
					.checked_div(U256::from(fee_denominator.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?,
			)
			.map_err(|_| Error::<T>::Math)?
			.into();
			// Redemption fee is charged with pool token before redemption.
			amount = amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
		}

		// The pool token amount becomes D - _amount
		let y: T::AtLeast64BitUnsigned =
			Self::get_y(&balances, i, d.checked_sub(&amount).ok_or(Error::<T>::Math)?, a)
				.ok_or(Error::<T>::Math)?;
		// dy = (balance[i] - y - 1) / precisions[i] in case there was rounding errors
		let balance_i: T::AtLeast64BitUnsigned = pool_info.balances[i_usize].into();
		let dy: T::AtLeast64BitUnsigned = balance_i
			.checked_sub(&y)
			.ok_or(Error::<T>::Math)?
			.checked_sub(&one)
			.ok_or(Error::<T>::Math)?
			.checked_div(&pool_info.precisions[i_usize])
			.ok_or(Error::<T>::Math)?;
		let total_supply: T::AtLeast64BitUnsigned =
			d.checked_sub(&amount).ok_or(Error::<T>::Math)?;
		balances[i_usize] = y;
		Ok(RedeemSingleResult {
			dy: dy.into(),
			fee_amount: fee_amount.into(),
			total_supply: total_supply.into(),
			balances: Self::convert_vec_number_to_balance(balances),
			redeem_amount: amount.into(),
		})
	}

	pub fn get_redeem_multi_amount(
		pool_info: &StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
		amounts: &[T::Balance],
	) -> Result<RedeemMultiResult<T>, Error<T>> {
		// update pool balances and total supply to avoid stale data
		let pool_info = Self::get_balance_update_amount(pool_info)?;
		let pool_info = Self::get_collect_yield_amount(&pool_info)?;

		if amounts.len() != pool_info.balances.len() {
			return Err(Error::<T>::ArgumentsError);
		}
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let old_d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		let zero: T::AtLeast64BitUnsigned = Zero::zero();
		for i in 0..balances.len() {
			let amounts_i: T::AtLeast64BitUnsigned = amounts[i].into();
			if amounts_i == zero {
				continue;
			}
			let balance_i: T::AtLeast64BitUnsigned = balances[i];
			// balance = balance + amount * precision
			let sub_amount: T::AtLeast64BitUnsigned =
				amounts_i.checked_mul(&pool_info.precisions[i]).ok_or(Error::<T>::Math)?;
			balances[i] = balance_i.checked_sub(&sub_amount).ok_or(Error::<T>::Math)?;
		}
		let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;
		let mut redeem_amount: T::AtLeast64BitUnsigned =
			old_d.checked_sub(&new_d).ok_or(Error::<T>::Math)?;
		let mut fee_amount: T::AtLeast64BitUnsigned = zero;
		if pool_info.redeem_fee > zero {
			let fee_denominator: T::AtLeast64BitUnsigned = T::FeePrecision::get();
			let div_amount: T::AtLeast64BitUnsigned =
				fee_denominator.checked_sub(&pool_info.redeem_fee).ok_or(Error::<T>::Math)?;
			redeem_amount = u128::try_from(
				U256::from(redeem_amount.saturated_into::<u128>())
					.checked_mul(U256::from(fee_denominator.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?
					.checked_div(U256::from(div_amount.saturated_into::<u128>()))
					.ok_or(Error::<T>::Math)?,
			)
			.map_err(|_| Error::<T>::Math)?
			.into();
			let sub_amount: T::AtLeast64BitUnsigned =
				old_d.checked_sub(&new_d).ok_or(Error::<T>::Math)?;
			fee_amount = redeem_amount.checked_sub(&sub_amount).ok_or(Error::<T>::Math)?;
		}
		let burn_amount: T::AtLeast64BitUnsigned =
			redeem_amount.checked_sub(&fee_amount).ok_or(Error::<T>::Math)?;
		let total_supply: T::AtLeast64BitUnsigned =
			old_d.checked_sub(&burn_amount).ok_or(Error::<T>::Math)?;
		Ok(RedeemMultiResult {
			redeem_amount: redeem_amount.into(),
			fee_amount: fee_amount.into(),
			balances: Self::convert_vec_number_to_balance(balances),
			total_supply: total_supply.into(),
			burn_amount: burn_amount.into(),
		})
	}

	pub fn get_pending_fee_amount(
		pool_info: &StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
	) -> Result<PendingFeeResult<T>, Error<T>> {
		let mut balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let old_d: T::AtLeast64BitUnsigned = pool_info.total_supply.into();
		for (i, balance) in balances.iter_mut().enumerate() {
			let mut balance_of: T::AtLeast64BitUnsigned =
				T::Assets::free_balance(pool_info.assets[i], &pool_info.account_id).into();
			if let Some((denominator, numerator)) =
				Self::get_token_rate(pool_info.pool_id, pool_info.assets[i])
			{
				balance_of = u128::try_from(
					U256::from(balance_of.saturated_into::<u128>())
						.checked_mul(U256::from(numerator.saturated_into::<u128>()))
						.ok_or(Error::<T>::Math)?
						.checked_div(U256::from(denominator.saturated_into::<u128>()))
						.ok_or(Error::<T>::Math)?,
				)
				.map_err(|_| Error::<T>::Math)?
				.into();
			}
			*balance = balance_of.checked_mul(&pool_info.precisions[i]).ok_or(Error::<T>::Math)?;
		}
		let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;
		let old_d_div_100: T::AtLeast64BitUnsigned =
			old_d.checked_div(&100u128.into()).ok_or(Error::<T>::Math)?;
		let old_d_margin: T::AtLeast64BitUnsigned =
			old_d.checked_sub(&old_d_div_100).ok_or(Error::<T>::Math)?;

		if new_d > old_d {
			let fee_amount: T::AtLeast64BitUnsigned =
				new_d.checked_sub(&old_d).ok_or(Error::<T>::Math)?;
			Ok(PendingFeeResult {
				fee_amount: fee_amount.into(),
				balances: Self::convert_vec_number_to_balance(balances),
				total_supply: new_d.into(),
			})
		} else if new_d >= old_d_margin {
			// this is due to rounding issues for token balance conversion
			Ok(PendingFeeResult {
				fee_amount: Zero::zero(),
				balances: Self::convert_vec_number_to_balance(balances),
				total_supply: new_d.into(),
			})
		} else {
			Err(Error::<T>::Math)
		}
	}

	pub fn get_collect_yield_amount(
		pool_info: &StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
	) -> Result<
		StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
		Error<T>,
	> {
		let a: T::AtLeast64BitUnsigned = Self::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		let balances: Vec<T::AtLeast64BitUnsigned> =
			Self::convert_vec_balance_to_number(pool_info.balances.clone());
		let new_d: T::AtLeast64BitUnsigned = Self::get_d(&balances, a).ok_or(Error::<T>::Math)?;
		let mut cloned_stable_asset_info = pool_info.clone();
		cloned_stable_asset_info.total_supply = new_d.into();
		Ok(cloned_stable_asset_info)
	}

	pub fn get_balance_update_amount(
		pool_info: &StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
	) -> Result<
		StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
		Error<T>,
	> {
		let mut updated_balances = pool_info.balances.clone();
		for (i, balance) in updated_balances.iter_mut().enumerate() {
			let mut balance_of: T::AtLeast64BitUnsigned =
				T::Assets::free_balance(pool_info.assets[i], &pool_info.account_id).into();
			if let Some((denominator, numerator)) =
				Self::get_token_rate(pool_info.pool_id, pool_info.assets[i])
			{
				balance_of = u128::try_from(
					U256::from(balance_of.saturated_into::<u128>())
						.checked_mul(U256::from(numerator.saturated_into::<u128>()))
						.ok_or(Error::<T>::Math)?
						.checked_div(U256::from(denominator.saturated_into::<u128>()))
						.ok_or(Error::<T>::Math)?,
				)
				.map_err(|_| Error::<T>::Math)?
				.into();
			}
			*balance =
				balance_of.checked_mul(&pool_info.precisions[i]).ok_or(Error::<T>::Math)?.into();
		}
		let mut cloned_stable_asset_info = pool_info.clone();
		cloned_stable_asset_info.balances = updated_balances;
		Ok(cloned_stable_asset_info)
	}
}

impl<T: Config> StableAsset for Pallet<T> {
	type AssetId = T::AssetId;
	type AtLeast64BitUnsigned = T::AtLeast64BitUnsigned;
	type Balance = T::Balance;
	type AccountId = T::AccountId;
	type BlockNumber = BlockNumberFor<T>;
	type Config = T;

	fn set_token_rate(
		pool_id: StableAssetPoolId,
		token_rate_info: Vec<(
			Self::AssetId,
			(Self::AtLeast64BitUnsigned, Self::AtLeast64BitUnsigned),
		)>,
	) -> DispatchResult {
		ensure!(pool_id < PoolCount::<T>::get(), Error::<T>::ArgumentsError);
		if token_rate_info.last().is_none() {
			let res = TokenRateCaches::<T>::clear_prefix(pool_id, u32::max_value(), None);
			ensure!(res.maybe_cursor.is_none(), Error::<T>::TokenRateNotCleared);
		} else {
			let mut token_rate_info = token_rate_info.into_iter();
			let mut token_rate = token_rate_info.next();
			while let Some((asset_id, is_token_rate)) = token_rate {
				TokenRateCaches::<T>::insert(pool_id, asset_id, is_token_rate);
				token_rate = token_rate_info.next();
			}
		}
		Self::deposit_event(Event::TokenRateSet {
			pool_id,
			token_rate: TokenRateCaches::<T>::iter_prefix(pool_id)
				.collect::<Vec<(T::AssetId, (T::AtLeast64BitUnsigned, T::AtLeast64BitUnsigned))>>(),
		});
		Ok(())
	}

	fn get_token_rate(
		pool_id: StableAssetPoolId,
		asset_id: Self::AssetId,
	) -> Option<(Self::AtLeast64BitUnsigned, Self::AtLeast64BitUnsigned)> {
		TokenRateCaches::<T>::get(pool_id, asset_id)
	}

	fn insert_pool(
		pool_id: StableAssetPoolId,
		pool_info: &StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	) {
		Pools::<T>::insert(pool_id, pool_info)
	}

	fn pool_count() -> StableAssetPoolId {
		PoolCount::<T>::get()
	}

	fn pool(
		id: StableAssetPoolId,
	) -> Option<
		StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	> {
		Pools::<T>::get(id)
	}

	/// Update the balance with underlying rebasing token balances
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `pool_info` - a mutable representation of the current pool state

	fn update_balance(
		pool_id: StableAssetPoolId,
		pool_info: &mut StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	) -> DispatchResult {
		let old_balances = pool_info.balances.clone();
		let new_balances_pool_info = Self::get_balance_update_amount(pool_info)?;
		pool_info.balances = new_balances_pool_info.balances;
		Self::deposit_event(Event::BalanceUpdated {
			pool_id,
			old_balances,
			new_balances: pool_info.balances.clone(),
		});
		Ok(())
	}

	/// Collect the yield from the underlying rebasing token balances
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `pool_info` - a mutable representation of the current pool state

	fn collect_yield(
		pool_id: StableAssetPoolId,
		pool_info: &mut StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	) -> DispatchResult {
		let old_total_supply = pool_info.total_supply;
		let old_d: T::AtLeast64BitUnsigned = old_total_supply.into();
		Self::update_balance(pool_id, pool_info)?;

		let updated_total_supply_pool_info = Self::get_collect_yield_amount(pool_info)?;
		let new_d: T::AtLeast64BitUnsigned = updated_total_supply_pool_info.total_supply.into();

		ensure!(new_d >= old_d, Error::<T>::InvalidPoolValue);
		if new_d > old_d {
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			let yield_amount: T::AtLeast64BitUnsigned = new_d - old_d;
			T::Assets::deposit(
				pool_info.pool_asset,
				&pool_info.yield_recipient,
				yield_amount.into(),
			)?;
			pool_info.total_supply = new_d.into();
			Self::deposit_event(Event::YieldCollected {
				pool_id,
				a,
				old_total_supply,
				new_total_supply: pool_info.total_supply,
				who: pool_info.yield_recipient.clone(),
				amount: yield_amount.into(),
			});
		}
		Ok(())
	}

	/// Collect the fees from user interactions
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `pool_info` - a mutable representation of the current pool state

	fn collect_fee(
		pool_id: StableAssetPoolId,
		pool_info: &mut StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	) -> DispatchResult {
		let old_balances = pool_info.balances.clone();
		let old_total_supply = pool_info.total_supply;
		let PendingFeeResult { fee_amount, balances, total_supply } =
			Self::get_pending_fee_amount(pool_info)?;
		let zero: T::Balance = Zero::zero();
		pool_info.total_supply = total_supply;
		pool_info.balances = balances;
		if fee_amount > zero {
			let fee_recipient = pool_info.fee_recipient.clone();
			T::Assets::deposit(pool_info.pool_asset, &fee_recipient, fee_amount)?;
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			Self::deposit_event(Event::FeeCollected {
				pool_id,
				a,
				old_balances,
				new_balances: pool_info.balances.clone(),
				old_total_supply,
				new_total_supply: total_supply,
				who: fee_recipient,
				amount: fee_amount,
			});
		}
		Ok(())
	}

	/// Create a new pool
	///
	/// # Arguments
	///
	/// * `pool_asset` - the asset ID of the pool token
	/// * `assets` - underlying assets of the pool
	/// * `precisions` - 10**precision / 10**underlying_pool_token_precision
	/// * `mint_fee` - mint fee percent
	/// * `swap_fee` - swap fee percent
	/// * `redeem_fee` - redeem fee percent
	/// * `initial_a` - the A value of the pool
	/// * `fee_recipient` - account ID for fees from user interactions
	/// * `yield_recipient` - account ID for yield from rebasing tokens
	/// * `precision` - the pool token precision

	fn create_pool(
		pool_asset: Self::AssetId,
		assets: Vec<Self::AssetId>,
		precisions: Vec<Self::AtLeast64BitUnsigned>,
		mint_fee: Self::AtLeast64BitUnsigned,
		swap_fee: Self::AtLeast64BitUnsigned,
		redeem_fee: Self::AtLeast64BitUnsigned,
		initial_a: Self::AtLeast64BitUnsigned,
		fee_recipient: Self::AccountId,
		yield_recipient: Self::AccountId,
		precision: Self::AtLeast64BitUnsigned,
	) -> DispatchResult {
		ensure!(assets.len() > 1, Error::<T>::ArgumentsError);
		let pool_asset_limit = T::PoolAssetLimit::get() as usize;
		ensure!(assets.len() <= pool_asset_limit, Error::<T>::ArgumentsError);
		ensure!(assets.len() == precisions.len(), Error::<T>::ArgumentsMismatch);
		PoolCount::<T>::try_mutate(|pool_count| -> DispatchResult {
			let pool_id = *pool_count;
			let swap_id: T::AccountId = T::PalletId::get().into_sub_account_truncating(pool_id);
			Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
				ensure!(maybe_pool_info.is_none(), Error::<T>::InconsistentStorage);

				let balances = sp_std::vec![Zero::zero(); assets.len()];
				frame_system::Pallet::<T>::inc_providers(&swap_id);
				let current_block = frame_system::Pallet::<T>::block_number();
				*maybe_pool_info = Some(StableAssetPoolInfo {
					pool_id,
					pool_asset,
					assets,
					precisions,
					mint_fee,
					swap_fee,
					redeem_fee,
					total_supply: Zero::zero(),
					a: initial_a,
					a_block: current_block,
					future_a: initial_a,
					future_a_block: current_block,
					balances,
					fee_recipient,
					account_id: swap_id.clone(),
					yield_recipient,
					precision,
				});

				Ok(())
			})?;

			*pool_count = pool_id.checked_add(1).ok_or(ArithmeticError::Overflow)?;

			Self::deposit_event(Event::CreatePool {
				pool_id,
				swap_id,
				a: initial_a,
				pallet_id: T::PalletId::get().into_account_truncating(),
			});
			Ok(())
		})
	}

	/// Mint the pool token
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `amounts` - the amount of tokens to be put in the pool
	/// * `min_mint_amount` - the amount of minimum pool token received

	fn mint(
		who: &Self::AccountId,
		pool_id: StableAssetPoolId,
		amounts: Vec<Self::Balance>,
		min_mint_amount: Self::Balance,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			Self::collect_yield(pool_id, pool_info)?;
			let MintResult { mint_amount, fee_amount, balances, total_supply } =
				Self::get_mint_amount(pool_info, &amounts)?;
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			ensure!(mint_amount >= min_mint_amount, Error::<T>::MintUnderMin);
			for (i, amount) in amounts.iter().enumerate() {
				if *amount == Zero::zero() {
					continue;
				}
				T::Assets::transfer(pool_info.assets[i], who, &pool_info.account_id, *amount)?;
			}
			let zero: T::Balance = Zero::zero();
			if fee_amount > zero {
				T::Assets::deposit(pool_info.pool_asset, &pool_info.fee_recipient, fee_amount)?;
			}
			T::Assets::deposit(pool_info.pool_asset, who, mint_amount)?;
			pool_info.total_supply = total_supply;
			pool_info.balances = balances;
			Self::collect_fee(pool_id, pool_info)?;
			Self::deposit_event(Event::LiquidityAdded {
				minter: who.clone(),
				pool_id,
				a,
				input_amounts: amounts,
				min_output_amount: min_mint_amount,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				fee_amount,
				output_amount: mint_amount,
			});
			Ok(())
		})
	}

	/// Swap tokens
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `i` - the array index of the input token in StableAssetPoolInfo.assets
	/// * `j` - the array index of the output token in StableAssetPoolInfo.assets
	/// * `dx` - the amount of input token
	/// * `min_dy` - the minimum amount of output token received
	/// * `asset_length` - the length of array in StableAssetPoolInfo.assets

	fn swap(
		who: &Self::AccountId,
		pool_id: StableAssetPoolId,
		i: PoolTokenIndex,
		j: PoolTokenIndex,
		dx: Self::Balance,
		min_dy: Self::Balance,
		asset_length: u32,
	) -> sp_std::result::Result<(Self::Balance, Self::Balance), DispatchError> {
		Pools::<T>::try_mutate_exists(
			pool_id,
			|maybe_pool_info| -> sp_std::result::Result<(Self::Balance, Self::Balance), DispatchError> {
				let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
				let asset_length_usize = asset_length as usize;
				ensure!(asset_length_usize == pool_info.assets.len(), Error::<T>::ArgumentsError);
				Self::collect_yield(pool_id, pool_info)?;
				let SwapResult {
					dx: _,
					dy,
					y,
					balance_i,
				} = Self::get_swap_amount(pool_info, i, j, dx)?;
				ensure!(dy >= min_dy, Error::<T>::SwapUnderMin);
				let mut balances = pool_info.balances.clone();
				let i_usize = i as usize;
				let j_usize = j as usize;
				balances[i_usize] = balance_i;
				balances[j_usize] = y;
				T::Assets::transfer(pool_info.assets[i_usize], who, &pool_info.account_id, dx)?;
				T::Assets::transfer(pool_info.assets[j_usize], &pool_info.account_id, who, dy)?;
				let asset_i = pool_info.assets[i_usize];
				let asset_j = pool_info.assets[j_usize];

				// Since the actual output amount is round down, collect fee should update the pool balances and
				// total supply
				Self::collect_fee(pool_id, pool_info)?;
				let a: T::AtLeast64BitUnsigned = Self::get_a(
					pool_info.a,
					pool_info.a_block,
					pool_info.future_a,
					pool_info.future_a_block,
				)
				.ok_or(Error::<T>::Math)?;
				Self::deposit_event(Event::TokenSwapped {
					swapper: who.clone(),
					pool_id,
					a,
					input_asset: asset_i,
					output_asset: asset_j,
					input_amount: dx,
					min_output_amount: min_dy,
					balances: pool_info.balances.clone(),
					total_supply: pool_info.total_supply,
					output_amount: dy,
				});
				Ok((dx, dy))
			},
		)
	}

	/// Redeem the token proportionally
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `amount` - the amount of token to be redeemed
	/// * `min_redeem_amounts` - the minimum amounts of redeemed token received

	fn redeem_proportion(
		who: &Self::AccountId,
		pool_id: StableAssetPoolId,
		amount: Self::Balance,
		min_redeem_amounts: Vec<Self::Balance>,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			Self::collect_yield(pool_id, pool_info)?;
			ensure!(
				min_redeem_amounts.len() == pool_info.assets.len(),
				Error::<T>::ArgumentsMismatch
			);
			let RedeemProportionResult {
				amounts,
				balances,
				fee_amount,
				total_supply,
				redeem_amount,
			} = Self::get_redeem_proportion_amount(pool_info, amount)?;
			let zero: T::Balance = Zero::zero();
			for i in 0..amounts.len() {
				ensure!(amounts[i] >= min_redeem_amounts[i], Error::<T>::RedeemUnderMin);
				T::Assets::transfer(pool_info.assets[i], &pool_info.account_id, who, amounts[i])?;
			}
			if fee_amount > zero {
				T::Assets::transfer(
					pool_info.pool_asset,
					who,
					&pool_info.fee_recipient,
					fee_amount,
				)?;
			}
			T::Assets::withdraw(pool_info.pool_asset, who, redeem_amount)?;

			pool_info.total_supply = total_supply;
			pool_info.balances = balances;
			// Since the output amounts are round down, collect fee updates pool balances and total
			// supply.
			Self::collect_fee(pool_id, pool_info)?;
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			Self::deposit_event(Event::RedeemedProportion {
				redeemer: who.clone(),
				pool_id,
				a,
				input_amount: amount,
				min_output_amounts: min_redeem_amounts,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				fee_amount,
				output_amounts: amounts,
			});
			Ok(())
		})
	}

	/// Redeem the token into a single token
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `amount` - the amount of token to be redeemed
	/// * `i` - the array index of the input token in StableAssetPoolInfo.assets
	/// * `min_redeem_amount` - the minimum amount of redeemed token received
	/// * `asset_length` - the length of array in StableAssetPoolInfo.assets

	fn redeem_single(
		who: &Self::AccountId,
		pool_id: StableAssetPoolId,
		amount: Self::Balance,
		i: PoolTokenIndex,
		min_redeem_amount: Self::Balance,
		asset_length: u32,
	) -> sp_std::result::Result<(Self::Balance, Self::Balance), DispatchError> {
		Pools::<T>::try_mutate_exists(
			pool_id,
			|maybe_pool_info| -> sp_std::result::Result<(Self::Balance, Self::Balance), DispatchError> {
				let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
				Self::collect_yield(pool_id, pool_info)?;
				let RedeemSingleResult {
					dy,
					fee_amount,
					total_supply,
					balances,
					redeem_amount,
				} = Self::get_redeem_single_amount(pool_info, amount, i)?;
				let i_usize = i as usize;
				let pool_size = pool_info.assets.len();
				let asset_length_usize = asset_length as usize;
				ensure!(asset_length_usize == pool_size, Error::<T>::ArgumentsError);
				ensure!(dy >= min_redeem_amount, Error::<T>::RedeemUnderMin);
				if fee_amount > Zero::zero() {
					T::Assets::transfer(pool_info.pool_asset, who, &pool_info.fee_recipient, fee_amount)?;
				}
				T::Assets::transfer(pool_info.assets[i_usize], &pool_info.account_id, who, dy)?;
				T::Assets::withdraw(pool_info.pool_asset, who, redeem_amount)?;
				let mut amounts: Vec<T::Balance> = Vec::new();
				for idx in 0..pool_size {
					if idx == i_usize {
						amounts.push(dy);
					} else {
						amounts.push(Zero::zero());
					}
				}

				pool_info.total_supply = total_supply;
				pool_info.balances = balances;
				// Since the output amounts are round down, collect fee updates pool balances and total supply.
				Self::collect_fee(pool_id, pool_info)?;
				let a: T::AtLeast64BitUnsigned = Self::get_a(
					pool_info.a,
					pool_info.a_block,
					pool_info.future_a,
					pool_info.future_a_block,
				)
				.ok_or(Error::<T>::Math)?;
				Self::deposit_event(Event::RedeemedSingle {
					redeemer: who.clone(),
					pool_id,
					a,
					input_amount: amount,
					output_asset: pool_info.assets[i as usize],
					min_output_amount: min_redeem_amount,
					balances: pool_info.balances.clone(),
					total_supply: pool_info.total_supply,
					fee_amount,
					output_amount: dy,
				});
				Ok((amount, dy))
			},
		)
	}

	/// Redeem the token into desired underlying tokens
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `amounts` - the amounts of underlying token to be received
	/// * `max_redeem_amount` - the maximum amount of pool token to be redeemed

	fn redeem_multi(
		who: &Self::AccountId,
		pool_id: StableAssetPoolId,
		amounts: Vec<Self::Balance>,
		max_redeem_amount: Self::Balance,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			Self::collect_yield(pool_id, pool_info)?;
			let RedeemMultiResult {
				redeem_amount,
				fee_amount,
				balances,
				total_supply,
				burn_amount,
			} = Self::get_redeem_multi_amount(pool_info, &amounts)?;
			let zero: T::Balance = Zero::zero();
			ensure!(redeem_amount <= max_redeem_amount, Error::<T>::RedeemOverMax);
			if fee_amount > zero {
				T::Assets::transfer(
					pool_info.pool_asset,
					who,
					&pool_info.fee_recipient,
					fee_amount,
				)?;
			}
			for (idx, amount) in amounts.iter().enumerate() {
				if *amount > zero {
					T::Assets::transfer(
						pool_info.assets[idx],
						&pool_info.account_id,
						who,
						amounts[idx],
					)?;
				}
			}
			T::Assets::withdraw(pool_info.pool_asset, who, burn_amount)?;

			pool_info.total_supply = total_supply;
			pool_info.balances = balances;
			Self::collect_fee(pool_id, pool_info)?;
			let a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			Self::deposit_event(Event::RedeemedMulti {
				redeemer: who.clone(),
				pool_id,
				a,
				output_amounts: amounts,
				max_input_amount: max_redeem_amount,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				fee_amount,
				input_amount: redeem_amount,
			});
			Ok(())
		})
	}

	/// Modify A value
	///
	/// # Arguments
	///
	/// * `pool_id` - the ID of the pool
	/// * `a` - the new A value
	/// * `future_a_block` - the block number for the above A to take effect

	fn modify_a(
		pool_id: StableAssetPoolId,
		a: Self::AtLeast64BitUnsigned,
		future_a_block: BlockNumberFor<T>,
	) -> DispatchResult {
		Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
			let pool_info = maybe_pool_info.as_mut().ok_or(Error::<T>::PoolNotFound)?;
			ensure!(future_a_block > pool_info.a_block, Error::<T>::ArgumentsError);
			let current_block = frame_system::Pallet::<T>::block_number();
			let initial_a: T::AtLeast64BitUnsigned = Self::get_a(
				pool_info.a,
				pool_info.a_block,
				pool_info.future_a,
				pool_info.future_a_block,
			)
			.ok_or(Error::<T>::Math)?;
			pool_info.a = initial_a;
			pool_info.a_block = current_block;
			pool_info.future_a = a;
			pool_info.future_a_block = future_a_block;
			Self::deposit_event(Event::AModified { pool_id, value: a, time: future_a_block });
			Ok(())
		})
	}

	fn get_collect_yield_amount(
		pool_info: &StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
	) -> Option<
		StableAssetPoolInfo<
			T::AssetId,
			T::AtLeast64BitUnsigned,
			T::Balance,
			T::AccountId,
			BlockNumberFor<T>,
		>,
	> {
		Self::get_collect_yield_amount(pool_info).ok()
	}

	fn get_balance_update_amount(
		pool_info: &StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	) -> Option<
		StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
	> {
		Self::get_balance_update_amount(pool_info).ok()
	}

	fn get_redeem_proportion_amount(
		pool_info: &StableAssetPoolInfo<
			Self::AssetId,
			Self::AtLeast64BitUnsigned,
			Self::Balance,
			Self::AccountId,
			Self::BlockNumber,
		>,
		amount_bal: Self::Balance,
	) -> Option<RedeemProportionResult<T::Balance>> {
		Self::get_redeem_proportion_amount(pool_info, amount_bal).ok()
	}

	fn get_best_route(
		input_asset: Self::AssetId,
		output_asset: Self::AssetId,
		input_amount: Self::Balance,
	) -> Option<(StableAssetPoolId, PoolTokenIndex, PoolTokenIndex, Self::Balance)> {
		let mut maybe_best: Option<(
			StableAssetPoolId,
			PoolTokenIndex,
			PoolTokenIndex,
			Self::Balance,
		)> = None;

		// iterater all pool
		for (pool_id, pool_info) in Pools::<T>::iter() {
			let maybe_input_index = pool_info
				.assets
				.iter()
				.position(|&a| a == input_asset)
				.map(|usize_index| usize_index as PoolTokenIndex);
			let maybe_output_index = pool_info
				.assets
				.iter()
				.position(|&a| a == output_asset)
				.map(|usize_index| usize_index as PoolTokenIndex);

			if let (Some(input_index), Some(output_index)) = (maybe_input_index, maybe_output_index)
			{
				// calculate swap amount
				if let Ok(swap_result) =
					Self::get_swap_amount(&pool_info, input_index, output_index, input_amount)
				{
					let mut balance_of: T::AtLeast64BitUnsigned =
						T::Assets::free_balance(output_asset, &pool_info.account_id).into();
					if let Some((denominator, numerator)) =
						Self::get_token_rate(pool_info.pool_id, output_asset)
					{
						balance_of = u128::try_from(
							U256::from(balance_of.saturated_into::<u128>())
								.checked_mul(U256::from(numerator.saturated_into::<u128>()))?
								.checked_div(U256::from(denominator.saturated_into::<u128>()))?,
						)
						.ok()?
						.into();
					}
					// make sure pool can affort the output amount
					if swap_result.dy <= balance_of.into() {
						if let Some((_, _, _, output_amount)) = maybe_best {
							// this pool is better, replace maybe_best
							if output_amount < swap_result.dy {
								maybe_best =
									Some((pool_id, input_index, output_index, swap_result.dy))
							}
						} else {
							maybe_best = Some((pool_id, input_index, output_index, swap_result.dy))
						}
					}
				}
			}
		}

		maybe_best
	}

	fn get_swap_output_amount(
		pool_id: StableAssetPoolId,
		input_index: PoolTokenIndex,
		output_index: PoolTokenIndex,
		dx_bal: Self::Balance,
	) -> Option<SwapResult<Self::Balance>> {
		let pool_info_opt = Self::pool(pool_id);
		match pool_info_opt {
			Some(pool_info) =>
				Self::get_swap_amount(&pool_info, input_index, output_index, dx_bal).ok(),
			None => None,
		}
	}

	fn get_swap_input_amount(
		pool_id: StableAssetPoolId,
		input_index: PoolTokenIndex,
		output_index: PoolTokenIndex,
		dy_bal: Self::Balance,
	) -> Option<SwapResult<Self::Balance>> {
		let pool_info_opt = Self::pool(pool_id);
		match pool_info_opt {
			Some(pool_info) =>
				Self::get_swap_amount_exact(&pool_info, input_index, output_index, dy_bal),
			None => None,
		}
	}

	fn get_mint_amount(
		pool_id: StableAssetPoolId,
		amounts_bal: &[Self::Balance],
	) -> Option<MintResult<T>> {
		let pool_info_opt = Self::pool(pool_id);
		match pool_info_opt {
			Some(pool_info) => Self::get_mint_amount(&pool_info, amounts_bal).ok(),
			None => None,
		}
	}

	fn get_a(
		a0: T::AtLeast64BitUnsigned,
		t0: BlockNumberFor<T>,
		a1: T::AtLeast64BitUnsigned,
		t1: BlockNumberFor<T>,
	) -> Option<T::AtLeast64BitUnsigned> {
		Self::get_a(a0, t0, a1, t1)
	}
}

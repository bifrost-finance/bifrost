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
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;
pub mod traits;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::Zero,
	traits::{
		fungibles::Mutate,
		tokens::{fungibles, Fortitude, Precision, Preservation},
	},
	transactional,
};
use frame_system::pallet_prelude::*;
use node_primitives::{
	CurrencyId, CurrencyIdConversion, CurrencyIdExt, TimeUnit, VtokenMintingOperator,
};
pub use nutsfinance_stable_asset::{
	MintResult, PoolTokenIndex, Pools, RedeemMultiResult, RedeemProportionResult,
	RedeemSingleResult, StableAsset, StableAssetPoolId, SwapResult,
};
use sp_core::U256;
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type AssetIdOf<T> = <T as Config>::CurrencyId;

#[allow(type_alias_bounds)]
pub type AtLeast64BitUnsignedOf<T> = <T as nutsfinance_stable_asset::Config>::AtLeast64BitUnsigned;
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config:
		frame_system::Config + nutsfinance_stable_asset::Config<AssetId = AssetIdOf<Self>>
	{
		type WeightInfo: WeightInfo;

		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type MultiCurrency: fungibles::Inspect<
				AccountIdOf<Self>,
				AssetId = AssetIdOf<Self>,
				Balance = Self::Balance,
			> + fungibles::Mutate<AccountIdOf<Self>, AssetId = AssetIdOf<Self>, Balance = Self::Balance>;

		type CurrencyId: Parameter
			+ Ord
			+ Copy
			+ CurrencyIdExt
			+ From<CurrencyId>
			+ Into<CurrencyId>;

		type StableAsset: nutsfinance_stable_asset::StableAsset<
			AssetId = AssetIdOf<Self>,
			Balance = Self::Balance,
			AccountId = AccountIdOf<Self>,
			AtLeast64BitUnsigned = Self::AtLeast64BitUnsigned,
			Config = Self,
			BlockNumber = Self::BlockNumber,
		>;

		type VtokenMinting: VtokenMintingOperator<
			AssetIdOf<Self>,
			Self::Balance,
			AccountIdOf<Self>,
			TimeUnit,
		>;

		type CurrencyIdConversion: CurrencyIdConversion<AssetIdOf<Self>>;
	}

	#[pallet::error]
	pub enum Error<T> {
		SwapUnderMin,
		MintUnderMin,
		CantMint,
		RedeemOverMax,
		TokenRateNotSet,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(10)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::edit_token_rate())]
		pub fn edit_token_rate(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			token_rate_info: Vec<(
				AssetIdOf<T>,
				(AtLeast64BitUnsignedOf<T>, AtLeast64BitUnsignedOf<T>),
			)>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			nutsfinance_stable_asset::Pallet::<T>::set_token_rate(pool_id, token_rate_info)
		}

		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_pool())]
		#[transactional]
		pub fn create_pool(
			origin: OriginFor<T>,
			pool_asset: AssetIdOf<T>,
			assets: Vec<AssetIdOf<T>>,
			precisions: Vec<AtLeast64BitUnsignedOf<T>>,
			mint_fee: AtLeast64BitUnsignedOf<T>,
			swap_fee: AtLeast64BitUnsignedOf<T>,
			redeem_fee: AtLeast64BitUnsignedOf<T>,
			initial_a: AtLeast64BitUnsignedOf<T>,
			fee_recipient: AccountIdOf<T>,
			yield_recipient: AccountIdOf<T>,
			precision: AtLeast64BitUnsignedOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			T::StableAsset::create_pool(
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::add_liquidity())]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amounts: Vec<T::Balance>,
			min_mint_amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::mint_inner(&who, pool_id, amounts, min_mint_amount)
		}

		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::swap())]
		pub fn swap(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			i: PoolTokenIndex,
			j: PoolTokenIndex,
			dx: T::Balance,
			min_dy: T::Balance,
			// asset_length: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::on_swap(&who, pool_id, i, j, dx, min_dy)
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::redeem_proportion())]
		pub fn redeem_proportion(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amount: T::Balance,
			min_redeem_amounts: Vec<T::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::redeem_proportion_inner(&who, pool_id, amount, min_redeem_amounts)
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::redeem_single())]
		pub fn redeem_single(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amount: T::Balance,
			i: PoolTokenIndex,
			min_redeem_amount: T::Balance,
			asset_length: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::redeem_single_inner(&who, pool_id, amount, i, min_redeem_amount, asset_length)?;
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::redeem_multi())]
		pub fn redeem_multi(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amounts: Vec<T::Balance>,
			max_redeem_amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::redeem_multi_inner(&who, pool_id, amounts, max_redeem_amount)
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::modify_a())]
		pub fn modify_a(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			a: T::AtLeast64BitUnsigned,
			future_a_block: T::BlockNumber,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			T::StableAsset::modify_a(pool_id, a, future_a_block)
		}

		#[pallet::call_index(7)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::modify_fees())]
		pub fn modify_fees(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			mint_fee: Option<T::AtLeast64BitUnsigned>,
			swap_fee: Option<T::AtLeast64BitUnsigned>,
			redeem_fee: Option<T::AtLeast64BitUnsigned>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
				let pool_info = maybe_pool_info
					.as_mut()
					.ok_or(nutsfinance_stable_asset::Error::<T>::PoolNotFound)?;
				if let Some(fee) = mint_fee {
					pool_info.mint_fee = fee;
				}
				if let Some(fee) = swap_fee {
					pool_info.swap_fee = fee;
				}
				if let Some(fee) = redeem_fee {
					pool_info.redeem_fee = fee;
				}
				nutsfinance_stable_asset::Pallet::<T>::deposit_event(
					nutsfinance_stable_asset::Event::<T>::FeeModified {
						pool_id,
						mint_fee: pool_info.mint_fee,
						swap_fee: pool_info.swap_fee,
						redeem_fee: pool_info.redeem_fee,
					},
				);
				Ok(())
			})
		}

		#[pallet::call_index(8)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::modify_recipients())]
		pub fn modify_recipients(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			fee_recipient: Option<T::AccountId>,
			yield_recipient: Option<T::AccountId>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			Pools::<T>::try_mutate_exists(pool_id, |maybe_pool_info| -> DispatchResult {
				let pool_info = maybe_pool_info
					.as_mut()
					.ok_or(nutsfinance_stable_asset::Error::<T>::PoolNotFound)?;
				if let Some(recipient) = fee_recipient {
					pool_info.fee_recipient = recipient;
				}
				if let Some(recipient) = yield_recipient {
					pool_info.yield_recipient = recipient;
				}
				nutsfinance_stable_asset::Pallet::<T>::deposit_event(
					nutsfinance_stable_asset::Event::<T>::RecipientModified {
						pool_id,
						fee_recipient: pool_info.fee_recipient.clone(),
						yield_recipient: pool_info.yield_recipient.clone(),
					},
				);
				Ok(())
			})
		}
	}
}

impl<T: Config> Pallet<T> {
	#[transactional]
	fn mint_inner(
		who: &AccountIdOf<T>,
		pool_id: StableAssetPoolId,
		mut amounts: Vec<T::Balance>,
		min_mint_amount: T::Balance,
	) -> DispatchResult {
		let mut pool_info = T::StableAsset::pool(pool_id)
			.ok_or(nutsfinance_stable_asset::Error::<T>::PoolNotFound)?;
		let amounts_old = amounts.clone();
		for (i, amount) in amounts.iter_mut().enumerate() {
			*amount = Self::upscale(
				*amount,
				pool_id,
				*pool_info
					.assets
					.get(i as usize)
					.ok_or(nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch)?,
			)?;
		}
		T::StableAsset::collect_yield(pool_id, &mut pool_info)?;
		let MintResult { mint_amount, fee_amount, balances, total_supply } =
			nutsfinance_stable_asset::Pallet::<T>::get_mint_amount(&pool_info, &amounts)?;
		let a = T::StableAsset::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(nutsfinance_stable_asset::Error::<T>::Math)?;
		ensure!(mint_amount >= min_mint_amount, Error::<T>::MintUnderMin);
		for (i, amount) in amounts.iter().enumerate() {
			if *amount == Zero::zero() {
				continue;
			}
			ensure!(
				amounts_old[i] >=
					Self::downscale(
						*amount,
						pool_id,
						*pool_info
							.assets
							.get(i as usize)
							.ok_or(nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch)?,
					)?,
				Error::<T>::CantMint
			);
			<T as nutsfinance_stable_asset::Config>::Assets::transfer(
				pool_info.assets[i],
				who,
				&pool_info.account_id,
				amounts_old[i],
				Preservation::Expendable,
			)?;
		}
		if fee_amount > Zero::zero() {
			<T as nutsfinance_stable_asset::Config>::Assets::mint_into(
				pool_info.pool_asset,
				&pool_info.fee_recipient,
				fee_amount,
			)?;
		}
		<T as nutsfinance_stable_asset::Config>::Assets::mint_into(
			pool_info.pool_asset,
			who,
			mint_amount.into(),
		)?;
		pool_info.total_supply = total_supply;
		pool_info.balances = balances;
		T::StableAsset::collect_fee(pool_id, &mut pool_info)?;
		T::StableAsset::insert_pool(pool_id, &pool_info);
		nutsfinance_stable_asset::Pallet::<T>::deposit_event(
			nutsfinance_stable_asset::Event::<T>::Minted {
				minter: who.clone(),
				pool_id,
				a,
				input_amounts: amounts,
				min_output_amount: min_mint_amount,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				fee_amount,
				output_amount: mint_amount,
			},
		);
		Ok(())
	}

	#[transactional]
	fn redeem_proportion_inner(
		who: &AccountIdOf<T>,
		pool_id: StableAssetPoolId,
		amount: T::Balance,
		min_redeem_amounts: Vec<T::Balance>,
	) -> DispatchResult {
		let mut pool_info = T::StableAsset::pool(pool_id)
			.ok_or(nutsfinance_stable_asset::Error::<T>::PoolNotFound)?;
		T::StableAsset::collect_yield(pool_id, &mut pool_info)?;
		ensure!(
			min_redeem_amounts.len() == pool_info.assets.len(),
			nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch
		);
		let RedeemProportionResult {
			mut amounts,
			balances,
			fee_amount,
			total_supply,
			redeem_amount,
		} = nutsfinance_stable_asset::Pallet::<T>::get_redeem_proportion_amount(&pool_info, amount)?;

		for (i, amount) in amounts.iter_mut().enumerate() {
			*amount = Self::downscale(
				*amount,
				pool_id,
				*pool_info
					.assets
					.get(i as usize)
					.ok_or(nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch)?,
			)?;
		}

		let zero = Zero::zero();
		for i in 0..amounts.len() {
			ensure!(
				amounts[i] >=
					*min_redeem_amounts
						.get(i as usize)
						.ok_or(nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch)?,
				nutsfinance_stable_asset::Error::<T>::RedeemUnderMin
			);
			<T as nutsfinance_stable_asset::Config>::Assets::transfer(
				pool_info.assets[i],
				&pool_info.account_id,
				who,
				amounts[i],
				Preservation::Expendable,
			)?;
		}
		if fee_amount > zero {
			<T as nutsfinance_stable_asset::Config>::Assets::transfer(
				pool_info.pool_asset,
				who,
				&pool_info.fee_recipient,
				fee_amount,
				Preservation::Expendable,
			)?;
		}
		<T as nutsfinance_stable_asset::Config>::Assets::burn_from(
			pool_info.pool_asset,
			who,
			redeem_amount,
			Precision::Exact,
			Fortitude::Polite,
		)?;

		pool_info.total_supply = total_supply;
		pool_info.balances = balances;
		// Since the output amounts are round down, collect fee updates pool balances and total
		// supply.
		T::StableAsset::collect_fee(pool_id, &mut pool_info)?;
		T::StableAsset::insert_pool(pool_id, &pool_info);
		let a = T::StableAsset::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(nutsfinance_stable_asset::Error::<T>::Math)?;
		nutsfinance_stable_asset::Pallet::<T>::deposit_event(
			nutsfinance_stable_asset::Event::<T>::RedeemedProportion {
				redeemer: who.clone(),
				pool_id,
				a,
				input_amount: amount,
				min_output_amounts: min_redeem_amounts,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				fee_amount,
				output_amounts: amounts,
			},
		);
		Ok(())
	}

	#[transactional]
	fn redeem_multi_inner(
		who: &AccountIdOf<T>,
		pool_id: StableAssetPoolId,
		amounts: Vec<T::Balance>,
		max_redeem_amount: T::Balance,
	) -> DispatchResult {
		let mut pool_info = T::StableAsset::pool(pool_id)
			.ok_or(nutsfinance_stable_asset::Error::<T>::PoolNotFound)?;
		T::StableAsset::collect_yield(pool_id, &mut pool_info)?;
		let mut new_amounts = amounts.clone();
		for (i, amount) in new_amounts.iter_mut().enumerate() {
			*amount = Self::upscale(
				*amount,
				pool_id,
				*pool_info
					.assets
					.get(i as usize)
					.ok_or(nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch)?,
			)?;
		}
		let RedeemMultiResult { redeem_amount, fee_amount, balances, total_supply, burn_amount } =
			nutsfinance_stable_asset::Pallet::<T>::get_redeem_multi_amount(
				&mut pool_info,
				&new_amounts,
			)?;
		let zero: T::Balance = Zero::zero();
		ensure!(redeem_amount <= max_redeem_amount, Error::<T>::RedeemOverMax);
		if fee_amount > zero {
			<T as nutsfinance_stable_asset::Config>::Assets::transfer(
				pool_info.pool_asset,
				who,
				&pool_info.fee_recipient,
				fee_amount,
				Preservation::Expendable,
			)?;
		}
		for (idx, amount) in amounts.iter().enumerate() {
			if *amount > zero {
				<T as nutsfinance_stable_asset::Config>::Assets::transfer(
					pool_info.assets[idx],
					&pool_info.account_id,
					who,
					*amount,
					Preservation::Expendable,
				)?;
			}
		}
		<T as nutsfinance_stable_asset::Config>::Assets::burn_from(
			pool_info.pool_asset,
			who,
			burn_amount,
			Precision::Exact,
			Fortitude::Polite,
		)?;

		pool_info.total_supply = total_supply;
		pool_info.balances = balances;
		T::StableAsset::collect_fee(pool_id, &mut pool_info)?;
		T::StableAsset::insert_pool(pool_id, &pool_info);
		let a = T::StableAsset::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(nutsfinance_stable_asset::Error::<T>::Math)?;
		nutsfinance_stable_asset::Pallet::<T>::deposit_event(
			nutsfinance_stable_asset::Event::<T>::RedeemedMulti {
				redeemer: who.clone(),
				pool_id,
				a,
				output_amounts: amounts,
				max_input_amount: max_redeem_amount,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				fee_amount,
				input_amount: redeem_amount,
			},
		);
		Ok(())
	}

	#[transactional]
	fn redeem_single_inner(
		who: &AccountIdOf<T>,
		pool_id: StableAssetPoolId,
		amount: T::Balance, // LP
		i: PoolTokenIndex,
		min_redeem_amount: T::Balance,
		asset_length: u32,
	) -> Result<(T::Balance, T::Balance), DispatchError> {
		let mut pool_info = T::StableAsset::pool(pool_id)
			.ok_or(nutsfinance_stable_asset::Error::<T>::PoolNotFound)?;

		T::StableAsset::collect_yield(pool_id, &mut pool_info)?;
		let RedeemSingleResult { mut dy, fee_amount, total_supply, balances, redeem_amount } =
			nutsfinance_stable_asset::Pallet::<T>::get_redeem_single_amount(
				&mut pool_info,
				amount,
				i,
			)?;
		dy = Self::downscale(
			dy,
			pool_id,
			*pool_info
				.assets
				.get(i as usize)
				.ok_or(nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch)?,
		)?;
		let i_usize = i as usize;
		let pool_size = pool_info.assets.len();
		let asset_length_usize = asset_length as usize;
		ensure!(
			asset_length_usize == pool_size,
			nutsfinance_stable_asset::Error::<T>::ArgumentsError
		);
		ensure!(dy >= min_redeem_amount, nutsfinance_stable_asset::Error::<T>::RedeemUnderMin);
		if fee_amount > Zero::zero() {
			<T as nutsfinance_stable_asset::Config>::Assets::transfer(
				pool_info.pool_asset,
				who,
				&pool_info.fee_recipient,
				fee_amount,
				Preservation::Expendable,
			)?;
		}
		<T as nutsfinance_stable_asset::Config>::Assets::transfer(
			pool_info.assets[i_usize],
			&pool_info.account_id,
			who,
			dy,
			Preservation::Expendable,
		)?;
		<T as nutsfinance_stable_asset::Config>::Assets::burn_from(
			pool_info.pool_asset,
			who,
			redeem_amount,
			Precision::Exact,
			Fortitude::Polite,
		)?;
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
		// Since the output amounts are round down, collect fee updates pool balances and total
		// supply.
		T::StableAsset::collect_fee(pool_id, &mut pool_info)?;
		T::StableAsset::insert_pool(pool_id, &pool_info);
		let a: T::AtLeast64BitUnsigned = T::StableAsset::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(nutsfinance_stable_asset::Error::<T>::Math)?;
		nutsfinance_stable_asset::Pallet::<T>::deposit_event(
			nutsfinance_stable_asset::Event::<T>::RedeemedSingle {
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
			},
		);
		Ok((amount, dy))
	}

	#[transactional]
	fn on_swap(
		who: &AccountIdOf<T>,
		pool_id: StableAssetPoolId,
		currency_id_in: PoolTokenIndex,
		currency_id_out: PoolTokenIndex,
		amount: T::Balance,
		min_dy: T::Balance,
	) -> DispatchResult {
		let mut pool_info = T::StableAsset::pool(pool_id)
			.ok_or(nutsfinance_stable_asset::Error::<T>::PoolNotFound)?;
		T::StableAsset::collect_yield(pool_id, &mut pool_info)?;
		let dx = Self::upscale(
			amount,
			pool_id,
			*pool_info
				.assets
				.get(currency_id_in as usize)
				.ok_or(nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch)?,
		)?;
		let SwapResult { dx: _, dy, y, balance_i } =
			nutsfinance_stable_asset::Pallet::<T>::get_swap_amount(
				&pool_info,
				currency_id_in,
				currency_id_out,
				dx,
			)?;

		let downscale_out = Self::downscale(
			dy,
			pool_id,
			*pool_info
				.assets
				.get(currency_id_out as usize)
				.ok_or(nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch)?,
		)?;
		ensure!(downscale_out >= min_dy, Error::<T>::SwapUnderMin);

		let mut balances = pool_info.balances.clone();
		let i_usize = currency_id_in as usize;
		let j_usize = currency_id_out as usize;
		balances[i_usize] = balance_i;
		balances[j_usize] = y;
		<T as nutsfinance_stable_asset::Config>::Assets::transfer(
			pool_info.assets[i_usize],
			who,
			&pool_info.account_id,
			amount,
			Preservation::Expendable,
		)?;
		<T as nutsfinance_stable_asset::Config>::Assets::transfer(
			pool_info.assets[j_usize],
			&pool_info.account_id,
			who,
			downscale_out,
			Preservation::Expendable,
		)?;
		let asset_i = pool_info.assets[i_usize];
		let asset_j = pool_info.assets[j_usize];
		T::StableAsset::collect_fee(pool_id, &mut pool_info)?;
		T::StableAsset::insert_pool(pool_id, &pool_info);
		let a = T::StableAsset::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(nutsfinance_stable_asset::Error::<T>::Math)?;
		nutsfinance_stable_asset::Pallet::<T>::deposit_event(
			nutsfinance_stable_asset::Event::<T>::TokenSwapped {
				swapper: who.clone(),
				pool_id,
				a,
				input_asset: asset_i,
				output_asset: asset_j,
				input_amount: amount,
				min_output_amount: min_dy,
				balances: pool_info.balances.clone(),
				total_supply: pool_info.total_supply,
				output_amount: downscale_out,
			},
		);
		Ok(())
	}

	pub fn upscale(
		amount: T::Balance,
		pool_id: StableAssetPoolId,
		currency_id: AssetIdOf<T>,
	) -> Result<T::Balance, DispatchError> {
		if let Some((demoninator, numerator)) =
			nutsfinance_stable_asset::Pallet::<T>::get_token_rate(pool_id, currency_id)
		{
			return Ok(Self::calculate_scaling(
				amount.into(),
				numerator.into(),
				demoninator.into(),
			));
		}
		return Err(Error::<T>::TokenRateNotSet.into());
	}
	pub fn downscale(
		amount: T::Balance,
		pool_id: StableAssetPoolId,
		currency_id: AssetIdOf<T>,
	) -> Result<T::Balance, DispatchError> {
		if let Some((numerator, demoninator)) =
			nutsfinance_stable_asset::Pallet::<T>::get_token_rate(pool_id, currency_id)
		{
			return Ok(Self::calculate_scaling(
				amount.into(),
				numerator.into(),
				demoninator.into(),
			));
		}
		return Err(Error::<T>::TokenRateNotSet.into());
	}

	fn calculate_scaling(
		amount: AtLeast64BitUnsignedOf<T>,
		numerator: AtLeast64BitUnsignedOf<T>,
		denominator: AtLeast64BitUnsignedOf<T>,
	) -> T::Balance {
		let amount: u128 = amount.saturated_into::<u128>();
		let denominator: u128 = denominator.saturated_into::<u128>();
		let numerator: u128 = numerator.saturated_into::<u128>();
		let can_get_vtoken = U256::from(amount)
			.checked_mul(U256::from(numerator))
			.and_then(|n| n.checked_div(U256::from(denominator)))
			.and_then(|n| TryInto::<u128>::try_into(n).ok())
			.unwrap_or_else(Zero::zero);

		let charge_amount: AtLeast64BitUnsignedOf<T> = can_get_vtoken.into();
		charge_amount.into()
	}

	pub fn get_swap_output(
		pool_id: StableAssetPoolId,
		currency_id_in: PoolTokenIndex,
		currency_id_out: PoolTokenIndex,
		amount: T::Balance,
	) -> Result<T::Balance, DispatchError> {
		let mut pool_info = T::StableAsset::pool(pool_id)
			.ok_or(nutsfinance_stable_asset::Error::<T>::PoolNotFound)?;
		T::StableAsset::collect_yield(pool_id, &mut pool_info)?;
		let dx = Self::upscale(
			amount,
			pool_id,
			*pool_info
				.assets
				.get(currency_id_in as usize)
				.ok_or(nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch)?,
		)?;
		let SwapResult { dx: _, dy, .. } = nutsfinance_stable_asset::Pallet::<T>::get_swap_amount(
			&pool_info,
			currency_id_in,
			currency_id_out,
			dx,
		)?;
		let downscale_out = Self::downscale(
			dy,
			pool_id,
			*pool_info
				.assets
				.get(currency_id_out as usize)
				.ok_or(nutsfinance_stable_asset::Error::<T>::ArgumentsMismatch)?,
		)?;

		Ok(downscale_out)
	}
}

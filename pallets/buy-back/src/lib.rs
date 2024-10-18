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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;

use bb_bnc::{BbBNCInterface, BB_BNC_SYSTEM_POOL_ID};
use bifrost_primitives::{currency::BNC, CurrencyId, CurrencyIdRegister, TryConvertFrom};
use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, One, Zero},
		Permill, SaturatedConversion, Saturating,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_std::{vec, vec::Vec};
pub use weights::WeightInfo;
use zenlink_protocol::{AssetId, ExportZenlink};

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type WeightInfo: WeightInfo;

		type DexOperator: ExportZenlink<Self::AccountId, AssetId>;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type BuyBackAccount: Get<PalletId>;

		#[pallet::constant]
		type LiquidityAccount: Get<PalletId>;

		type ParachainId: Get<ParaId>;

		type CurrencyIdRegister: CurrencyIdRegister<CurrencyId>;

		type BbBNC: BbBNCInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
			BlockNumberFor<Self>,
		>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A successful call of the `Charge` extrinsic will create this event.
		Charged { who: AccountIdOf<T>, currency_id: CurrencyIdOf<T>, value: BalanceOf<T> },
		/// A successful call of the `SetVtoken` extrinsic will create this event.
		ConfigSet { currency_id: CurrencyIdOf<T>, info: Info<BalanceOf<T>, BlockNumberFor<T>> },
		/// A successful call of the `RemoveVtoken` extrinsic will create this event.
		Removed { currency_id: CurrencyIdOf<T> },
		/// A failed call of the `BuyBack` extrinsic will create this event.
		BuyBackFailed { currency_id: CurrencyIdOf<T>, block_number: BlockNumberFor<T> },
		/// A successful call of the `BuyBack` extrinsic will create this event.
		BuyBackSuccess { currency_id: CurrencyIdOf<T>, block_number: BlockNumberFor<T> },
		/// A failed call of the `AddLiquidity` extrinsic will create this event.
		AddLiquidityFailed { currency_id: CurrencyIdOf<T>, block_number: BlockNumberFor<T> },
		/// A successful call of the `AddLiquidity` extrinsic will create this event.
		AddLiquiditySuccess { currency_id: CurrencyIdOf<T>, block_number: BlockNumberFor<T> },
		/// A failed call of the `SetSwapOutMin` extrinsic will create this event.
		SetSwapOutMinFailed { currency_id: CurrencyIdOf<T>, block_number: BlockNumberFor<T> },
		/// A successful call of the `SetSwapOutMin` extrinsic will create this event.
		SetSwapOutMinSuccess { currency_id: CurrencyIdOf<T>, block_number: BlockNumberFor<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Insufficient balance.
		NotEnoughBalance,
		/// Currency does not exist.
		CurrencyIdNotExists,
		/// Currency is not supported.
		CurrencyIdError,
		/// Duration can't be zero.
		ZeroDuration,
		/// Field min_swap_value can't be zero.
		ZeroMinSwapValue,
	}

	#[pallet::storage]
	pub type Infos<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, Info<BalanceOf<T>, BlockNumberFor<T>>>;

	#[pallet::storage]
	pub type SwapOutMin<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u128>;

	#[pallet::storage]
	pub type AddLiquiditySwapOutMin<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u128>;

	/// Information on buybacks and add liquidity
	#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	pub struct Info<BalanceOf, BlockNumberFor> {
		/// The minimum value of the token to be swapped.
		min_swap_value: BalanceOf,
		/// Whether to automatically add liquidity and buy back.
		if_auto: bool,
		/// The proportion of the token to be added to the liquidity pool.
		proportion: Permill,
		/// The duration of the buyback.
		buyback_duration: BlockNumberFor,
		/// The last time the buyback was executed.
		last_buyback: BlockNumberFor,
		/// The end block of the last buyback cycle.
		last_buyback_cycle: BlockNumberFor,
		/// The duration of adding liquidity.
		add_liquidity_duration: BlockNumberFor,
		/// The last time liquidity was added.
		last_add_liquidity: BlockNumberFor,
		/// The destruction ratio of BNC.
		destruction_ratio: Option<Permill>,
		/// The bias of the token value to be swapped.
		bias: Permill,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let buyback_address = T::BuyBackAccount::get().into_account_truncating();
			let liquidity_address = T::LiquidityAccount::get().into_account_truncating();
			for (currency_id, mut info) in Infos::<T>::iter() {
				if !info.if_auto {
					continue;
				}

				match info.last_add_liquidity + info.add_liquidity_duration {
					target_block if target_block.saturating_sub(One::one()) == n => {
						if let Some(e) = Self::set_add_liquidity_swap_out_min(
							&liquidity_address,
							currency_id,
							&info,
						)
						.err()
						{
							log::error!(
								target: "buy-back::set_add_liquidity_swap_out_min",
								"Received invalid justification for {:?}",
								e,
							);
							Self::deposit_event(Event::SetSwapOutMinFailed {
								currency_id,
								block_number: n,
							});
						} else {
							Self::deposit_event(Event::SetSwapOutMinSuccess {
								currency_id,
								block_number: n,
							});
						}
					},
					target_block if target_block == n => {
						if let Some(swap_out_min) = AddLiquiditySwapOutMin::<T>::get(currency_id) {
							if let Some(e) = Self::add_liquidity(
								&liquidity_address,
								currency_id,
								&info,
								swap_out_min,
							)
							.err()
							{
								log::error!(
									target: "buy-back::add_liquidity",
									"Received invalid justification for {:?}",
									e,
								);
								Self::deposit_event(Event::AddLiquidityFailed {
									currency_id,
									block_number: n,
								});
							} else {
								Self::deposit_event(Event::AddLiquiditySuccess {
									currency_id,
									block_number: n,
								});
							}
							info.last_add_liquidity =
								info.last_add_liquidity + info.add_liquidity_duration;
							Infos::<T>::insert(currency_id, info.clone());
							AddLiquiditySwapOutMin::<T>::remove(currency_id);
						}
					},
					_ => (),
				}

				// If the previous period has not ended, continue with the next currency.
				if info.last_buyback_cycle >= n {
					continue;
				}
				match Self::get_target_block(info.last_buyback, info.buyback_duration) {
					target_block
						if target_block ==
							n.saturating_sub(info.last_buyback_cycle)
								.saturated_into::<u32>() =>
					{
						if let Some(e) = Self::set_swap_out_min(currency_id, &info).err() {
							log::error!(
								target: "buy-back::set_swap_out_min",
								"Received invalid justification for {:?}",
								e,
							);
							Self::deposit_event(Event::SetSwapOutMinFailed {
								currency_id,
								block_number: n,
							});
						} else {
							Self::deposit_event(Event::SetSwapOutMinSuccess {
								currency_id,
								block_number: n,
							});
						}
					},
					target_block
						if target_block ==
							n.saturating_sub(info.last_buyback_cycle)
								.saturated_into::<u32>()
								.saturating_sub(One::one()) =>
					{
						if let Some(swap_out_min) = SwapOutMin::<T>::get(currency_id) {
							if let Some(e) =
								Self::buy_back(&buyback_address, currency_id, &info, swap_out_min)
									.err()
							{
								log::error!(
									target: "buy-back::buy_back",
									"Received invalid justification for {:?}",
									e,
								);
								Self::deposit_event(Event::BuyBackFailed {
									currency_id,
									block_number: n,
								});
							} else {
								Self::deposit_event(Event::BuyBackSuccess {
									currency_id,
									block_number: n,
								});
							}
							info.last_buyback_cycle =
								info.last_buyback_cycle.saturating_add(info.buyback_duration);
							info.last_buyback = n;
							Infos::<T>::insert(currency_id, info);
							SwapOutMin::<T>::remove(currency_id);
						}
					},
					_ => (),
				}
			}
			T::WeightInfo::on_idle()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Configuration for setting up buybacks and adding liquidity.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_vtoken())]
		pub fn set_vtoken(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			min_swap_value: BalanceOf<T>,
			proportion: Permill,
			buyback_duration: BlockNumberFor<T>,
			add_liquidity_duration: BlockNumberFor<T>,
			if_auto: bool,
			destruction_ratio: Option<Permill>,
			bias: Permill,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Self::check_currency_id(currency_id)?;
			ensure!(min_swap_value > Zero::zero(), Error::<T>::ZeroMinSwapValue);
			ensure!(buyback_duration > Zero::zero(), Error::<T>::ZeroDuration);
			ensure!(add_liquidity_duration > Zero::zero(), Error::<T>::ZeroDuration);

			let now = frame_system::Pallet::<T>::block_number();

			let info = Info {
				min_swap_value,
				if_auto,
				proportion,
				buyback_duration,
				last_buyback: now,
				last_buyback_cycle: now,
				add_liquidity_duration,
				last_add_liquidity: now,
				destruction_ratio,
				bias,
			};
			Infos::<T>::insert(currency_id, info.clone());

			Self::deposit_event(Event::ConfigSet { currency_id, info });

			Ok(())
		}

		/// Charge the buyback account.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::charge())]
		pub fn charge(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			Self::check_currency_id(currency_id)?;
			T::MultiCurrency::transfer(
				currency_id,
				&exchanger,
				&T::BuyBackAccount::get().into_account_truncating(),
				value,
			)?;

			Self::deposit_event(Event::Charged { who: exchanger, currency_id, value });

			Ok(())
		}

		/// Remove the configuration of the buyback.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::remove_vtoken())]
		pub fn remove_vtoken(origin: OriginFor<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			ensure!(Infos::<T>::contains_key(currency_id), Error::<T>::CurrencyIdNotExists);
			Infos::<T>::remove(currency_id);

			Self::deposit_event(Event::Removed { currency_id });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		#[transactional]
		pub fn buy_back(
			buyback_address: &AccountIdOf<T>,
			currency_id: CurrencyId,
			info: &Info<BalanceOf<T>, BlockNumberFor<T>>,
			swap_out_min: u128,
		) -> DispatchResult {
			let balance = T::MultiCurrency::free_balance(currency_id, &buyback_address);
			ensure!(balance >= info.min_swap_value, Error::<T>::NotEnoughBalance);
			let path = Self::get_path(currency_id)?;
			let amount_out_min = swap_out_min.saturating_sub(info.bias * swap_out_min);

			T::DexOperator::inner_swap_exact_assets_for_assets(
				buyback_address,
				info.min_swap_value.saturated_into(),
				amount_out_min,
				&path,
				&buyback_address,
			)?;

			if let Some(ratio) = info.destruction_ratio {
				let bnc_balance_before_burn = T::MultiCurrency::free_balance(BNC, &buyback_address);
				let destruction_amount = ratio * bnc_balance_before_burn;
				T::MultiCurrency::withdraw(BNC, &buyback_address, destruction_amount)?;
			}
			let bnc_balance = T::MultiCurrency::free_balance(BNC, &buyback_address);
			T::BbBNC::notify_reward(
				BB_BNC_SYSTEM_POOL_ID,
				&Some(buyback_address.clone()),
				vec![(BNC, bnc_balance)],
			)
		}

		#[transactional]
		fn add_liquidity(
			liquidity_address: &AccountIdOf<T>,
			currency_id: CurrencyId,
			info: &Info<BalanceOf<T>, BlockNumberFor<T>>,
			swap_out_min: u128,
		) -> DispatchResult {
			let path = Self::get_path(currency_id)?;
			let balance = T::MultiCurrency::free_balance(currency_id, &liquidity_address);
			let token_balance = info.proportion * balance;
			ensure!(token_balance > Zero::zero(), Error::<T>::NotEnoughBalance);
			let amount_out_min = swap_out_min.saturating_sub(info.bias * swap_out_min);

			T::DexOperator::inner_swap_exact_assets_for_assets(
				liquidity_address,
				token_balance.saturated_into(),
				amount_out_min,
				&path,
				&liquidity_address,
			)?;
			let remaining_balance = T::MultiCurrency::free_balance(currency_id, &liquidity_address);
			let bnc_balance = T::MultiCurrency::free_balance(BNC, &liquidity_address);

			let amount_0_min = 0;
			let amount_1_min = 0;
			T::DexOperator::inner_add_liquidity(
				liquidity_address,
				path[0],
				path[path.len() - 1],
				remaining_balance.saturated_into(),
				bnc_balance.saturated_into(),
				amount_0_min,
				amount_1_min,
			)
		}

		pub fn check_currency_id(currency_id: CurrencyId) -> Result<(), DispatchError> {
			match currency_id {
				CurrencyId::VToken(token_symbol) =>
					if !T::CurrencyIdRegister::check_vtoken_registered(token_symbol) {
						return Err(Error::<T>::CurrencyIdNotExists.into());
					},
				CurrencyId::VToken2(token_id) => {
					if !T::CurrencyIdRegister::check_vtoken2_registered(token_id) {
						return Err(Error::<T>::CurrencyIdNotExists.into());
					}
				},
				_ => return Err(Error::<T>::CurrencyIdError.into()),
			};
			Ok(())
		}

		pub fn get_target_block(n: BlockNumberFor<T>, duration: BlockNumberFor<T>) -> u32 {
			let block_hash = frame_system::Pallet::<T>::block_hash(n);
			let hash_bytes = block_hash.as_ref();
			let hash_value =
				u32::from_le_bytes([hash_bytes[0], hash_bytes[1], hash_bytes[2], hash_bytes[3]]);
			let target_block =
				hash_value % (duration.saturating_sub(One::one()).saturated_into::<u32>());

			target_block + 1
		}

		pub fn get_path(currency_id: CurrencyId) -> Result<Vec<AssetId>, DispatchError> {
			let asset_id: AssetId =
				AssetId::try_convert_from(currency_id, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let bnc_asset_id: AssetId =
				AssetId::try_convert_from(BNC, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			Ok(vec![asset_id, bnc_asset_id])
		}

		pub fn set_swap_out_min(
			currency_id: CurrencyId,
			info: &Info<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			let path = Self::get_path(currency_id)?;
			let amounts = T::DexOperator::get_amount_out_by_path(
				info.min_swap_value.saturated_into(),
				&path,
			)?;
			SwapOutMin::<T>::insert(currency_id, amounts[amounts.len() - 1]);
			Ok(())
		}

		pub fn set_add_liquidity_swap_out_min(
			liquidity_address: &AccountIdOf<T>,
			currency_id: CurrencyId,
			info: &Info<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			let path = Self::get_path(currency_id)?;
			let balance = T::MultiCurrency::free_balance(currency_id, &liquidity_address);
			let token_balance = info.proportion * balance;
			ensure!(token_balance > Zero::zero(), Error::<T>::NotEnoughBalance);
			let amounts =
				T::DexOperator::get_amount_out_by_path(token_balance.saturated_into(), &path)?;
			AddLiquiditySwapOutMin::<T>::insert(currency_id, amounts[amounts.len() - 1]);
			Ok(())
		}
	}
}

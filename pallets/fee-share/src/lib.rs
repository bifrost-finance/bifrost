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

use bifrost_primitives::{CurrencyId, DistributionId, Price};
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{
			AccountIdConversion, CheckedAdd, CheckedMul, SaturatedConversion, Saturating, Zero,
		},
		ArithmeticError, FixedU128, Perbill,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
pub use pallet::*;
use pallet_traits::PriceFeeder;
use sp_std::{cmp::Ordering, collections::btree_map::BTreeMap, vec::Vec};
pub use weights::WeightInfo;

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

		#[pallet::constant]
		type FeeSharePalletId: Get<PalletId>;

		/// The oracle price feeder
		type PriceFeeder: PriceFeeder;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created {
			distribution_id: DistributionId,
			info: Info<AccountIdOf<T>>,
		},
		Edited {
			distribution_id: DistributionId,
			info: Info<AccountIdOf<T>>,
		},
		EraLengthSet {
			era_length: BlockNumberFor<T>,
			next_era: BlockNumberFor<T>,
		},
		Executed {
			distribution_id: DistributionId,
		},
		Deleted {
			distribution_id: DistributionId,
		},
		ExecuteFailed {
			distribution_id: DistributionId,
			info: Info<AccountIdOf<T>>,
			next_era: BlockNumberFor<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		NotSupportProportion,
		CalculationOverflow,
		ExistentialDeposit,
		DistributionNotExist,
		/// Price oracle not ready
		PriceOracleNotReady,
		/// Price is zero
		PriceIsZero,
	}

	#[pallet::storage]
	pub type DistributionInfos<T: Config> =
		StorageMap<_, Twox64Concat, DistributionId, Info<AccountIdOf<T>>>;

	#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	pub struct Info<AccountIdOf> {
		pub receiving_address: AccountIdOf,
		pub token_type: Vec<CurrencyId>,
		pub tokens_proportion: BTreeMap<AccountIdOf, Perbill>,
		pub if_auto: bool,
	}

	#[pallet::storage]
	pub type DollarStandardInfos<T: Config> = StorageMap<
		_,
		Twox64Concat,
		DistributionId,
		DollarStandardInfo<BlockNumberFor<T>, AccountIdOf<T>>,
	>;

	#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	pub struct DollarStandardInfo<BlockNumberFor, AccountIdOf> {
		pub target_value: u128,
		pub cumulative: u128,
		pub target_address: AccountIdOf,
		pub target_block: BlockNumberFor,
		pub duration: BlockNumberFor,
	}

	#[pallet::storage]
	pub type DistributionNextId<T: Config> = StorageValue<_, DistributionId, ValueQuery>;

	#[pallet::storage]
	pub type AutoEra<T: Config> =
		StorageValue<_, (BlockNumberFor<T>, BlockNumberFor<T>), ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(bn: BlockNumberFor<T>, _remaining_weight: Weight) -> Weight {
			DollarStandardInfos::<T>::iter().for_each(|(distribution_id, mut info)| {
				if bn == info.target_block {
					info.target_block = info.target_block.saturating_add(info.duration);
					info.cumulative = Zero::zero();
					DollarStandardInfos::<T>::insert(distribution_id, info);
				}
			});
			let (era_length, next_era) = AutoEra::<T>::get();
			if bn.eq(&next_era) {
				for (distribution_id, info) in DistributionInfos::<T>::iter() {
					if info.if_auto {
						if let Some(e) =
							Self::execute_distribute_inner(distribution_id, &info).err()
						{
							Self::deposit_event(Event::ExecuteFailed {
								distribution_id,
								info,
								next_era,
							});

							log::error!(
								target: "runtime::fee-share",
								"Received invalid justification for {:?}",
								e,
							);
						}
					}
				}
				let next_era = next_era.saturating_add(era_length);
				AutoEra::<T>::put((era_length, next_era));
			}
			Weight::zero()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_distribution())]
		pub fn create_distribution(
			origin: OriginFor<T>,
			token_type: Vec<CurrencyId>,
			tokens_proportion: Vec<(AccountIdOf<T>, Perbill)>,
			if_auto: bool,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut total_proportion = Perbill::from_percent(0);
			let tokens_proportion_map: BTreeMap<AccountIdOf<T>, Perbill> = tokens_proportion
				.into_iter()
				.map(|(k, v)| {
					total_proportion = total_proportion.saturating_add(v);
					(k, v)
				})
				.collect();
			ensure!(total_proportion.is_one(), Error::<T>::NotSupportProportion);

			let distribution_id = DistributionNextId::<T>::get();
			let receiving_address =
				T::FeeSharePalletId::get().into_sub_account_truncating(distribution_id);
			let info = Info {
				receiving_address,
				token_type,
				tokens_proportion: tokens_proportion_map,
				if_auto,
			};
			DistributionInfos::<T>::insert(distribution_id, info.clone());
			DistributionNextId::<T>::mutate(|id| -> DispatchResult {
				*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::Created { distribution_id, info });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::edit_distribution())]
		pub fn edit_distribution(
			origin: OriginFor<T>,
			distribution_id: DistributionId,
			token_type: Option<Vec<CurrencyId>>,
			tokens_proportion: Option<Vec<(AccountIdOf<T>, Perbill)>>,
			if_auto: Option<bool>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut info = DistributionInfos::<T>::get(distribution_id)
				.ok_or(Error::<T>::DistributionNotExist)?;
			if let Some(tokens_proportion) = tokens_proportion {
				let mut total_proportion = Perbill::from_percent(0);
				let tokens_proportion_map: BTreeMap<AccountIdOf<T>, Perbill> = tokens_proportion
					.into_iter()
					.map(|(k, v)| {
						total_proportion = total_proportion.saturating_add(v);
						(k, v)
					})
					.collect();
				ensure!(total_proportion.is_one(), Error::<T>::NotSupportProportion);
				info.tokens_proportion = tokens_proportion_map;
			}

			if let Some(token_type) = token_type {
				info.token_type = token_type;
			}

			if let Some(if_auto) = if_auto {
				info.if_auto = if_auto;
			}
			DistributionInfos::<T>::insert(distribution_id, info.clone());

			Self::deposit_event(Event::Edited { distribution_id, info });
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::set_era_length())]
		pub fn set_era_length(
			origin: OriginFor<T>,
			era_length: BlockNumberFor<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let current_block = frame_system::Pallet::<T>::block_number();
			let next_era =
				current_block.checked_add(&era_length).ok_or(ArithmeticError::Overflow)?;
			AutoEra::<T>::put((era_length, next_era));

			Self::deposit_event(Event::EraLengthSet { era_length, next_era });
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::execute_distribute())]
		pub fn execute_distribute(
			origin: OriginFor<T>,
			distribution_id: DistributionId,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if let Some(info) = DistributionInfos::<T>::get(distribution_id) {
				Self::execute_distribute_inner(distribution_id, &info)?;
			}

			Self::deposit_event(Event::Executed { distribution_id });
			Ok(())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::delete_distribution())]
		pub fn delete_distribution(
			origin: OriginFor<T>,
			distribution_id: DistributionId,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if let Some(info) = DistributionInfos::<T>::get(distribution_id) {
				Self::execute_distribute_inner(distribution_id, &info)?;
				DistributionInfos::<T>::remove(distribution_id);
			}

			Self::deposit_event(Event::Deleted { distribution_id });
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::usd_cumulation())]
		pub fn usd_cumulation(
			origin: OriginFor<T>,
			distribution_id: DistributionId,
			target_value: u128,
			duration: BlockNumberFor<T>,
			target_address: AccountIdOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			ensure!(
				DistributionInfos::<T>::contains_key(distribution_id),
				Error::<T>::DistributionNotExist
			);

			let now = frame_system::Pallet::<T>::block_number();
			let info = DollarStandardInfo {
				target_value,
				cumulative: Zero::zero(),
				target_address,
				target_block: now + duration,
				duration,
			};
			DollarStandardInfos::<T>::insert(distribution_id, info);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn execute_distribute_inner(
			distribution_id: DistributionId,
			infos: &Info<AccountIdOf<T>>,
		) -> DispatchResult {
			let mut usd_value: FixedU128 = Zero::zero();
			infos.token_type.iter().try_for_each(|&currency_id| -> DispatchResult {
				let amount = T::MultiCurrency::free_balance(currency_id, &infos.receiving_address);
				let value = Self::get_asset_value(currency_id, amount)?;
				usd_value = usd_value.checked_add(&value).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;
			if let Some(mut usd_infos) = DollarStandardInfos::<T>::get(distribution_id) {
				match usd_infos.cumulative.cmp(&usd_infos.target_value) {
					Ordering::Equal | Ordering::Greater => (),
					Ordering::Less => {
						usd_infos.cumulative = usd_infos
							.cumulative
							.checked_add(usd_value.into_inner())
							.ok_or(ArithmeticError::Overflow)?;
						DollarStandardInfos::<T>::insert(distribution_id, &usd_infos);
						return Self::transfer_all(infos, usd_infos.target_address);
					},
				}
			}

			infos.token_type.iter().try_for_each(|&currency_id| -> DispatchResult {
				let ed = T::MultiCurrency::minimum_balance(currency_id);
				let amount = T::MultiCurrency::free_balance(currency_id, &infos.receiving_address);
				infos.tokens_proportion.iter().try_for_each(
					|(account_to_send, &proportion)| -> DispatchResult {
						let withdraw_amount = proportion.mul_floor(amount);
						if withdraw_amount < ed {
							let receiver_balance =
								T::MultiCurrency::total_balance(currency_id, &account_to_send);

							let receiver_balance_after = receiver_balance
								.checked_add(&withdraw_amount)
								.ok_or(ArithmeticError::Overflow)?;
							if receiver_balance_after < ed {
								Err(Error::<T>::ExistentialDeposit)?;
							}
						}
						T::MultiCurrency::transfer(
							currency_id,
							&infos.receiving_address,
							&account_to_send,
							withdraw_amount,
						)
					},
				)
			})
		}

		pub fn get_price(currency_id: CurrencyIdOf<T>) -> Result<Price, DispatchError> {
			let (price, _) =
				T::PriceFeeder::get_price(&currency_id).ok_or(Error::<T>::PriceOracleNotReady)?;
			if price.is_zero() {
				return Err(Error::<T>::PriceIsZero.into());
			}
			log::trace!(
				target: "fee-share::get_price", "price: {:?}", price.into_inner()
			);

			Ok(price)
		}

		pub fn get_asset_value(
			currency_id: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> Result<FixedU128, DispatchError> {
			let value = Self::get_price(currency_id)?
				.checked_mul(&FixedU128::from_inner(amount.saturated_into()))
				.ok_or(ArithmeticError::Overflow)?;

			Ok(value)
		}

		fn transfer_all(
			infos: &Info<AccountIdOf<T>>,
			target_address: AccountIdOf<T>,
		) -> DispatchResult {
			infos.token_type.iter().try_for_each(|&currency_id| -> DispatchResult {
				let amount = T::MultiCurrency::free_balance(currency_id, &infos.receiving_address);
				T::MultiCurrency::transfer(
					currency_id,
					&infos.receiving_address,
					&target_address,
					amount,
				)
			})
		}
	}
}

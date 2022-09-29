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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, CheckedAdd, Saturating},
		ArithmeticError, Perbill,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, DistributionId};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};
pub use weights::WeightInfo;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type ControlOrigin: EnsureOrigin<Self::Origin>;

		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type FeeSharePalletId: Get<PalletId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created {
			info: Info<AccountIdOf<T>>,
		},
		Edited {
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
	}

	#[pallet::storage]
	#[pallet::getter(fn distribution_infos)]
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
	#[pallet::getter(fn distribution_next_id)]
	pub type DistributionNextId<T: Config> = StorageValue<_, DistributionId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn auto_era)]
	pub type AutoEra<T: Config> =
		StorageValue<_, (BlockNumberFor<T>, BlockNumberFor<T>), ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(bn: BlockNumberFor<T>, _remaining_weight: Weight) -> Weight {
			let (era_length, next_era) = Self::auto_era();
			if bn.eq(&next_era) {
				for (distribution_id, info) in DistributionInfos::<T>::iter() {
					if info.if_auto {
						if let Some(e) = Self::execute_distribute_inner(&info).err() {
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
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
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

			let distribution_id = Self::distribution_next_id();
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

			Self::deposit_event(Event::Created { info });
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::edit_distribution())]
		pub fn edit_distribution(
			origin: OriginFor<T>,
			distribution_id: DistributionId,
			token_type: Option<Vec<CurrencyId>>,
			tokens_proportion: Option<Vec<(AccountIdOf<T>, Perbill)>>,
			if_auto: Option<bool>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let mut info = Self::distribution_infos(distribution_id)
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

			Self::deposit_event(Event::Edited { info });
			Ok(())
		}

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

		#[pallet::weight(T::WeightInfo::execute_distribute())]
		pub fn execute_distribute(
			origin: OriginFor<T>,
			distribution_id: DistributionId,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if let Some(info) = Self::distribution_infos(distribution_id) {
				Self::execute_distribute_inner(&info)?;
			}

			Self::deposit_event(Event::Executed { distribution_id });
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::delete_distribution())]
		pub fn delete_distribution(
			origin: OriginFor<T>,
			distribution_id: DistributionId,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			if let Some(info) = Self::distribution_infos(distribution_id) {
				Self::execute_distribute_inner(&info)?;
				DistributionInfos::<T>::remove(distribution_id);
			}

			Self::deposit_event(Event::Deleted { distribution_id });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn execute_distribute_inner(infos: &Info<AccountIdOf<T>>) -> DispatchResult {
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
	}
}

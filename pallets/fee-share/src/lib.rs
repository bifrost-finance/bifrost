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

use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, CheckedAdd, Saturating},
		ArithmeticError, Perbill, SaturatedConversion,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, CurrencyIdConversion, TryConvertFrom, VtokenMintingInterface};
use orml_traits::MultiCurrency;
pub use pallet::*;
// use sp_core::U256;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};
// use sp_std::{borrow::ToOwned, vec};
pub use weights::WeightInfo;
use zenlink_protocol::ExportZenlink;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

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

		type DexOperator: ExportZenlink<Self::AccountId>;

		type CurrencyIdConversion: CurrencyIdConversion<CurrencyId>;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type RelayChainToken: Get<CurrencyId>;

		#[pallet::constant]
		type FeeSharePalletId: Get<PalletId>;

		type ParachainId: Get<ParaId>;

		/// The interface to call VtokenMinting module functions.
		type VtokenMintingInterface: VtokenMintingInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created { info: Info<AccountIdOf<T>> },
		ConfigSet { currency_id: CurrencyIdOf<T>, info: Info<AccountIdOf<T>> },
		Closed { currency_id: CurrencyIdOf<T> },
		Paid { currency_id: CurrencyIdOf<T>, value: BalanceOf<T> },
		RedeemFailed { vcurrency_id: CurrencyIdOf<T>, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		NotSupportProportion,
		CalculationOverflow,
		ExistentialDeposit,
	}

	#[pallet::storage]
	#[pallet::getter(fn distribution_infos)]
	pub type DistributionInfos<T: Config> =
		StorageMap<_, Twox64Concat, DistributionId, Info<AccountIdOf<T>>>;

	#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	pub struct Info<AccountIdOf> {
		pub receiving_address: AccountIdOf,
		pub token_type: CurrencyId,
		pub tokens_proportion: BTreeMap<AccountIdOf, Perbill>,
		pub if_auto: bool,
	}

	pub type DistributionId = u32;

	#[pallet::storage]
	#[pallet::getter(fn distribution_next_id)]
	pub type DistributionNextId<T: Config> = StorageValue<_, DistributionId, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(_bn: BlockNumberFor<T>, _remaining_weight: Weight) -> Weight {
			// let fee_share = T::FeeSharePalletId::get().into_account_truncating();
			for (currency_id, info) in DistributionInfos::<T>::iter() {
				if let Some(e) = Self::execute_distribute(currency_id, &info).err() {
					log::error!(
						target: "runtime::fee-share",
						"Received invalid justification for {:?}",
						e,
					);
				}
			}
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::create_distribution())]
		pub fn create_distribution(
			origin: OriginFor<T>,
			token_type: CurrencyId,
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
	}

	impl<T: Config> Pallet<T> {
		fn execute_distribute(a: u32, infos: &Info<AccountIdOf<T>>) -> DispatchResult {
			let currency_id = infos.token_type;
			let ed = T::MultiCurrency::minimum_balance(currency_id);
			let amount = T::MultiCurrency::free_balance(currency_id, &infos.receiving_address);
			// if let Some(infos) = Self::distribution_infos(distribution_id) {
			infos.tokens_proportion.iter().try_for_each(
				|(account_to_send, &proportion)| -> DispatchResult {
					let withdraw_amount = proportion * amount;
					// let ed = T::MultiCurrency::minimum_balance(currency_id);

					if withdraw_amount < ed {
						let receiver_balance =
							T::MultiCurrency::total_balance(currency_id, &account_to_send);

						let receiver_balance_after = receiver_balance
							.checked_add(&withdraw_amount)
							.ok_or(ArithmeticError::Overflow)?;
						if receiver_balance_after < ed {
							// account_to_send = T::TreasuryAccount::get();
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
			)?;
			// };
			Ok(())
		}
	}
}

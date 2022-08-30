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
#![allow(deprecated)] // TODO: clear transaction

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
		traits::{AccountIdConversion, CheckedSub},
		SaturatedConversion,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, CurrencyIdConversion, TokenSymbol};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_arithmetic::{per_things::Permill, traits::Zero};
pub use weights::WeightInfo;
use zenlink_protocol::{AssetBalance, AssetId, ExportZenlink};

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
	#[pallet::generate_store(pub(super) trait Store)]
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
		type SystemMakerPalletId: Get<PalletId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Charged { who: AccountIdOf<T>, currency_id: CurrencyIdOf<T>, value: BalanceOf<T> },
		ConfigSet { currency_id: CurrencyIdOf<T>, annualization: Permill },
		Closed { currency_id: CurrencyIdOf<T> },
		Paid { currency_id: CurrencyIdOf<T>, value: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		NotSupportTokenType,
		CalculationOverflow,
	}

	#[pallet::storage]
	#[pallet::getter(fn info)]
	pub type Info<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, Permill, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(bn: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let system_maker = T::SystemMakerPalletId::get().into_account_truncating();
			for (currency_id, annualization) in Info::<T>::iter() {
				Self::handle_by_currency_id(system_maker, currency_id, annualization)
					.map_err(|e| {
						log::error!(
							target: "runtime::system-maker",
							"Received invalid justification for {:?}",
							e,
						);
						e
					})
					.ok();
			}
			// log::debug!(
			//     target: "runtime",
			//     "block #{:?} with weight='{:?}'",
			//     bn,
			//     remaining_weight,
			// );
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[transactional]
		#[pallet::weight(T::WeightInfo::set_config())]
		pub fn set_config(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			annualization: Permill,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Info::<T>::mutate(currency_id, |old_annualization| {
				*old_annualization = Some(annualization.clone());
			});

			Self::deposit_event(Event::ConfigSet { currency_id, annualization });

			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::charge())]
		pub fn charge(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			let exchanger = ensure_signed(origin)?;

			T::MultiCurrency::transfer(
				currency_id,
				&exchanger,
				&T::SystemMakerPalletId::get().into_account_truncating(),
				value,
			)?;

			Self::deposit_event(Event::Charged { who: exchanger, currency_id, value });

			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::close())]
		pub fn close(origin: OriginFor<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Info::<T>::remove(currency_id);

			Self::deposit_event(Event::Closed { currency_id });

			Ok(())
		}

		#[transactional]
		#[pallet::weight(T::WeightInfo::payout())]
		pub fn payout(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			T::MultiCurrency::transfer(
				currency_id,
				&T::SystemMakerPalletId::get().into_account_truncating(),
				&T::TreasuryAccount::get(),
				value,
			)?;

			Self::deposit_event(Event::Paid { currency_id, value });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn handle_by_currency_id(
			system_maker: AccountIdOf<T>,
			currency_id: CurrencyId,
			annualization: Permill,
		) -> DispatchResult {
			let relay_currency_id = T::RelayChainToken::get();
			let relay_vtoken_id = T::CurrencyIdConversion::convert_to_vtoken(relay_currency_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let relay_asset_id: AssetId = AssetId::try_from(relay_currency_id)
				.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let relay_vtoken_asset_id: AssetId = AssetId::try_from(relay_vtoken_id)
				.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let path = vec![relay_asset_id, relay_vtoken_asset_id];

			let balance = T::MultiCurrency::free_balance(currency_id, &system_maker);
			T::DexOperator::inner_swap_exact_assets_for_assets(
				&system_maker,
				balance.saturated_into(),
				annualization.saturating_reciprocal_mul(balance).saturated_into(),
				&path,
				&system_maker,
			)
			.err();
			Ok(())
		}
	}
}

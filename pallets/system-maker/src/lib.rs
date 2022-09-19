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
	sp_runtime::{traits::AccountIdConversion, ArithmeticError, SaturatedConversion},
	PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, CurrencyIdConversion, TryConvertFrom, VtokenMintingInterface};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_core::U256;
use sp_std::{borrow::ToOwned, vec};
pub use weights::WeightInfo;
use zenlink_protocol::{AssetId, ExportZenlink};

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
		type SystemMakerPalletId: Get<PalletId>;

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
		Charged { who: AccountIdOf<T>, currency_id: CurrencyIdOf<T>, value: BalanceOf<T> },
		ConfigSet { currency_id: CurrencyIdOf<T>, info: Info<BalanceOf<T>> },
		Closed { currency_id: CurrencyIdOf<T> },
		Paid { currency_id: CurrencyIdOf<T>, value: BalanceOf<T> },
		RedeemFailed { vcurrency_id: CurrencyIdOf<T>, amount: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		NotSupportTokenType,
		CalculationOverflow,
	}

	#[pallet::storage]
	#[pallet::getter(fn infos)]
	pub type Infos<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, Info<BalanceOf<T>>>;

	#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	pub struct Info<BalanceOf> {
		pub vcurrency_id: CurrencyId,
		pub annualization: u32,
		pub granularity: BalanceOf,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(_bn: BlockNumberFor<T>, _remaining_weight: Weight) -> Weight {
			let system_maker = T::SystemMakerPalletId::get().into_account_truncating();
			for (currency_id, info) in Infos::<T>::iter() {
				if let Some(e) = Self::swap_by_currency_id(&system_maker, currency_id, &info).err()
				{
					log::error!(
						target: "runtime::system-maker",
						"Received invalid justification for {:?}",
						e,
					);
				}
				Self::handle_redeem_by_currency_id(&system_maker, &info);
			}
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::set_config())]
		pub fn set_config(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			info: Info<BalanceOf<T>>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			let vcurrency_id = T::CurrencyIdConversion::convert_to_vtoken(currency_id)
				.map_err(|_| Error::<T>::NotSupportTokenType)?;
			ensure!(vcurrency_id == info.vcurrency_id, Error::<T>::NotSupportTokenType);
			Infos::<T>::mutate(currency_id, |old_info| {
				*old_info = Some(info.clone());
			});

			Self::deposit_event(Event::ConfigSet { currency_id, info });

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::charge())]
		pub fn charge(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
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

		#[pallet::weight(T::WeightInfo::close())]
		pub fn close(origin: OriginFor<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Infos::<T>::remove(currency_id);

			Self::deposit_event(Event::Closed { currency_id });

			Ok(())
		}

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
		fn swap_by_currency_id(
			system_maker: &AccountIdOf<T>,
			currency_id: CurrencyId,
			info: &Info<BalanceOf<T>>,
		) -> DispatchResult {
			let asset_id: AssetId =
				AssetId::try_convert_from(currency_id, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let vcurrency_asset_id: AssetId =
				AssetId::try_convert_from(info.vcurrency_id, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let path = vec![asset_id, vcurrency_asset_id];
			let balance = T::MultiCurrency::free_balance(currency_id, &system_maker);
			ensure!(balance > info.granularity, Error::<T>::NotEnoughBalance);

			let denominator = U256::from(
				(1_000_000u32.saturating_add(info.annualization)).saturated_into::<u128>(),
			);
			let amount_out_min: u128 = U256::from(info.granularity.saturated_into::<u128>())
				.saturating_mul(U256::from(1_000_000u32))
				.checked_div(denominator)
				.ok_or(ArithmeticError::Overflow)?
				.as_u128();

			T::DexOperator::inner_swap_exact_assets_for_assets(
				system_maker,
				info.granularity.saturated_into(),
				amount_out_min,
				&path,
				&system_maker,
			)
		}

		fn handle_redeem_by_currency_id(system_maker: &AccountIdOf<T>, info: &Info<BalanceOf<T>>) {
			let redeem_amount = T::MultiCurrency::free_balance(info.vcurrency_id, system_maker);

			if let Some(e) = T::VtokenMintingInterface::redeem(
				system_maker.to_owned(),
				info.vcurrency_id,
				redeem_amount,
			)
			.err()
			{
				Self::deposit_event(Event::RedeemFailed {
					vcurrency_id: info.vcurrency_id,
					amount: redeem_amount,
				});
				log::error!(
					target: "runtime::system-maker",
					"Received invalid justification for {:?}",
					e,
				);
			}
		}
	}
}

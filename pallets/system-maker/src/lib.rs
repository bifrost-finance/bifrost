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

use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, CheckedSub},
		SaturatedConversion,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{
	CurrencyId, CurrencyIdConversion, TokenSymbol, TryConvertFrom, VtokenMintingInterface,
};
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_arithmetic::{
	per_things::Permill,
	traits::{UniqueSaturatedInto, Zero},
};
use sp_core::U256;
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
		pub annualization: u32,
		pub granularity: BalanceOf,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(bn: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let system_maker = T::SystemMakerPalletId::get().into_account_truncating();
			for (currency_id, info) in Infos::<T>::iter() {
				let vcurrency_id = T::CurrencyIdConversion::convert_to_vtoken(currency_id)
					.ok()
					.unwrap_or_default();
				// .map_err(|_| Error::<T>::NotSupportTokenType)?;
				// Self::handle_by_currency_id(&system_maker, currency_id, info).err().ok_or(|e| {
				// 	log::error!(
				// 		target: "runtime::system-maker",
				// 		"Received invalid justification for {:?}",
				// 		e,
				// 	);
				// });
				if let Some(e) =
					Self::handle_by_currency_id(&system_maker, currency_id, vcurrency_id, info)
						.err()
				{
					log::error!(
						target: "runtime::system-maker",
						"Received invalid justification for {:?}",
						e,
					);
				}

				if let Some(e) =
					Self::handle_redeem_by_currency_id(&system_maker, vcurrency_id).err()
				{
					// Self::deposit_event(Event::RedeemFailed {
					// 	token: token_id,
					// 	amount: vredeem_amount,
					// 	farming_staking_amount: token_info.farming_staking_amount,
					// 	system_stakable_amount: token_info.system_stakable_amount,
					// 	system_shadow_amount: token_info.system_shadow_amount,
					// 	pending_redeem_amount: token_info.pending_redeem_amount,
					// });
					log::error!(
						target: "runtime::system-maker",
						"Received invalid justification for {:?}",
						e,
					);
				}

				// Self::handle_redeem_by_currency_id(&system_maker, currency_id)
				// 	.map_err(|e| {
				// 		log::error!(
				// 			target: "runtime::system-maker",
				// 			"Received invalid justification for {:?}",
				// 			e,
				// 		);
				// 		e
				// 	})
				// 	.ok();
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
			info: Info<BalanceOf<T>>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Infos::<T>::mutate(currency_id, |old_info| {
				*old_info = Some(info.clone());
			});

			Self::deposit_event(Event::ConfigSet { currency_id, info });

			Ok(())
		}

		#[transactional]
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

		#[transactional]
		#[pallet::weight(T::WeightInfo::close())]
		pub fn close(origin: OriginFor<T>, currency_id: CurrencyIdOf<T>) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Infos::<T>::remove(currency_id);

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

		#[transactional]
		#[pallet::weight(T::WeightInfo::payout())]
		pub fn handle_redeem_by_currency_id2(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			// info: Info<BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			let exchanger = ensure_signed(origin)?;
			Self::handle_redeem_by_currency_id(&exchanger, currency_id)
			// Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		#[transactional]
		fn handle_by_currency_id(
			system_maker: &AccountIdOf<T>,
			currency_id: CurrencyId,
			vcurrency_id: CurrencyId,
			info: Info<BalanceOf<T>>, // annualization: Permill,
		) -> DispatchResult {
			// T::MultiCurrency::transfer(
			// 	currency_id,
			// 	&T::SystemMakerPalletId::get().into_account_truncating(),
			// 	&T::TreasuryAccount::get(),
			// 	info.granularity,
			// )?;
			// let vcurrency_id = T::CurrencyIdConversion::convert_to_vtoken(currency_id)
			// 	.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let asset_id: AssetId =
				AssetId::try_convert_from(currency_id, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let vcurrency_asset_id: AssetId =
				AssetId::try_convert_from(vcurrency_id, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let path = vec![asset_id, vcurrency_asset_id];
			let balance = T::MultiCurrency::free_balance(currency_id, &system_maker);

			let denominator = U256::from(
				(1_000_000u32.saturating_add(info.annualization)).saturated_into::<u128>(),
			);
			let amount_out_min: u128 = U256::from(info.granularity.saturated_into::<u128>())
				.saturating_mul(U256::from(1_000_000u32))
				.checked_div(denominator)
				.unwrap_or_default()
				.as_u128();

			T::DexOperator::inner_swap_exact_assets_for_assets(
				system_maker,
				info.granularity.saturated_into(),
				amount_out_min,
				&path,
				&system_maker,
			)
		}

		#[transactional]
		fn handle_redeem_by_currency_id(
			system_maker: &AccountIdOf<T>,
			vcurrency_id: CurrencyId,
			// info: Info<BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			// let vcurrency_id = T::CurrencyIdConversion::convert_to_vtoken(currency_id)
			// 	.map_err(|_| Error::<T>::NotSupportTokenType)?;
			let redeem_amount = T::MultiCurrency::free_balance(vcurrency_id, system_maker);

			T::VtokenMintingInterface::redeem(system_maker.to_owned(), vcurrency_id, redeem_amount)
		}
	}
}

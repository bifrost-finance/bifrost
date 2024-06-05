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

use bifrost_primitives::{
	currency::BNC, CurrencyId, CurrencyIdConversion, CurrencyIdRegister, TryConvertFrom,
};
use bifrost_ve_minting::VeMintingInterface;
use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, Zero},
		Permill, SaturatedConversion,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
pub use pallet::*;
use sp_std::vec;
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

		type CurrencyIdConversion: CurrencyIdConversion<CurrencyId>;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type RelayChainToken: Get<CurrencyId>;

		#[pallet::constant]
		type BuyBackAccount: Get<PalletId>;

		#[pallet::constant]
		type LiquidityAccount: Get<PalletId>;

		type ParachainId: Get<ParaId>;

		type CurrencyIdRegister: CurrencyIdRegister<CurrencyId>;

		type VeMinting: VeMintingInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
			BlockNumberFor<Self>,
		>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Charged { who: AccountIdOf<T>, asset_id: CurrencyIdOf<T>, value: BalanceOf<T> },
		ConfigSet { asset_id: CurrencyIdOf<T>, info: Info<BalanceOf<T>, BlockNumberFor<T>> },
		Closed { asset_id: CurrencyIdOf<T> },
		Paid { asset_id: CurrencyIdOf<T>, value: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		ArgumentsError,
		NotEnoughBalance,
	}

	#[pallet::storage]
	#[pallet::getter(fn configs)]
	pub type Infos<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, Info<BalanceOf<T>, BlockNumberFor<T>>>;

	#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	pub struct Info<BalanceOf, BlockNumberFor> {
		value: BalanceOf,
		if_auto: bool,
		proportion: Permill,
		buyback_duration: BlockNumberFor,
		last_buyback: BlockNumberFor,
		add_liquidity_duration: BlockNumberFor,
		last_add_liquidity: BlockNumberFor,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(n: BlockNumberFor<T>, _remaining_weight: Weight) -> Weight {
			let buyback_address = T::BuyBackAccount::get().into_account_truncating();
			let liquidity_address = T::LiquidityAccount::get().into_account_truncating();
			for (asset_id, mut info) in Infos::<T>::iter() {
				if !info.if_auto {
					continue;
				}

				if info.last_add_liquidity == BlockNumberFor::<T>::from(0u32) ||
					info.last_add_liquidity + info.add_liquidity_duration == n
				{
					if let Some(e) = Self::add_liquidity(&liquidity_address, asset_id, &info).err()
					{
						log::error!(
							target: "runtime::add_liquidity",
							"Received invalid justification for {:?}",
							e,
						);
					}
					info.last_add_liquidity = n;
					Infos::<T>::insert(asset_id, info.clone());
				}
				if info.last_buyback == BlockNumberFor::<T>::from(0u32) ||
					info.last_buyback + info.buyback_duration == n
				{
					if let Some(e) = Self::buy_back(&buyback_address, asset_id).err() {
						log::error!(
							target: "runtime::buyback",
							"Received invalid justification for {:?}",
							e,
						);
					}
					info.last_buyback = n;
					Infos::<T>::insert(asset_id, info);
				}
			}
			T::WeightInfo::on_idle()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_vtoken())]
		pub fn set_vtoken(
			origin: OriginFor<T>,
			asset_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
			proportion: Permill,
			buyback_duration: BlockNumberFor<T>,
			add_liquidity_duration: BlockNumberFor<T>,
			if_auto: bool,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			match asset_id {
				CurrencyId::Token(token_symbol) =>
					if !T::CurrencyIdRegister::check_vtoken_registered(token_symbol) {
						return Err(Error::<T>::ArgumentsError.into());
					},
				CurrencyId::Token2(token_id) => {
					if !T::CurrencyIdRegister::check_vtoken2_registered(token_id) {
						return Err(Error::<T>::ArgumentsError.into());
					}
				},
				_ => (),
			};

			let info = Info {
				value,
				if_auto,
				proportion,
				buyback_duration,
				last_buyback: BlockNumberFor::<T>::from(0u32),
				add_liquidity_duration,
				last_add_liquidity: BlockNumberFor::<T>::from(0u32),
			};
			Infos::<T>::insert(asset_id, info.clone());

			Self::deposit_event(Event::ConfigSet { asset_id, info });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::charge())]
		pub fn charge(
			origin: OriginFor<T>,
			asset_id: CurrencyIdOf<T>,
			value: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			T::MultiCurrency::transfer(
				asset_id,
				&exchanger,
				&T::BuyBackAccount::get().into_account_truncating(),
				value,
			)?;

			Self::deposit_event(Event::Charged { who: exchanger, asset_id, value });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::remove_vtoken())]
		pub fn remove_vtoken(origin: OriginFor<T>, asset_id: CurrencyIdOf<T>) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Infos::<T>::remove(asset_id);

			Self::deposit_event(Event::Closed { asset_id });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		#[transactional]
		pub fn buy_back(
			buyback_address: &AccountIdOf<T>,
			currency_id: CurrencyId,
		) -> DispatchResult {
			let asset_id: AssetId =
				AssetId::try_convert_from(currency_id, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let bnc_asset_id: AssetId =
				AssetId::try_convert_from(BNC, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let path = vec![asset_id, bnc_asset_id];
			let balance = T::MultiCurrency::free_balance(currency_id, &buyback_address);
			ensure!(balance > Zero::zero(), Error::<T>::NotEnoughBalance);

			T::DexOperator::inner_swap_exact_assets_for_assets(
				buyback_address,
				balance.saturated_into(),
				0,
				&path,
				&buyback_address,
			)?;

			let bnc_balance = T::MultiCurrency::free_balance(BNC, &buyback_address);
			T::VeMinting::notify_reward(0, &Some(buyback_address.clone()), vec![(BNC, bnc_balance)])
		}

		#[transactional]
		fn add_liquidity(
			buyback_address: &AccountIdOf<T>,
			currency_id: CurrencyId,
			info: &Info<BalanceOf<T>, BlockNumberFor<T>>,
		) -> DispatchResult {
			let asset_id: AssetId =
				AssetId::try_convert_from(currency_id, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let bnc_asset_id: AssetId =
				AssetId::try_convert_from(BNC, T::ParachainId::get().into())
					.map_err(|_| DispatchError::Other("Conversion Error."))?;
			let path = vec![asset_id, bnc_asset_id];
			let balance = T::MultiCurrency::free_balance(currency_id, &buyback_address);
			let token_balance = info.proportion * balance;
			ensure!(token_balance > Zero::zero(), Error::<T>::NotEnoughBalance);

			T::DexOperator::inner_swap_exact_assets_for_assets(
				buyback_address,
				token_balance.saturated_into(),
				0,
				&path,
				&buyback_address,
			)?;
			let remaining_balance = T::MultiCurrency::free_balance(currency_id, &buyback_address);
			let bnc_balance = T::MultiCurrency::free_balance(BNC, &buyback_address);

			T::DexOperator::inner_add_liquidity(
				buyback_address,
				asset_id,
				bnc_asset_id,
				remaining_balance.saturated_into(),
				bnc_balance.saturated_into(),
				0,
				0,
			)
		}
	}
}

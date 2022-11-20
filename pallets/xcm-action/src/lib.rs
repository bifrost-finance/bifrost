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
use cumulus_primitives_core::ParaId;
use frame_support::{
	dispatch::DispatchResultWithPostInfo, sp_runtime::SaturatedConversion, traits::Get, PalletId,
};
use node_primitives::{CurrencyId, TryConvertFrom, VtokenMintingInterface};
use orml_traits::{arithmetic::Zero, MultiCurrency, XcmTransfer};
pub use pallet::*;
use scale_info::prelude::vec;
use xcm::{latest::prelude::*, v1::MultiLocation};
use xcm_interface::traits::parachains;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use zenlink_protocol::{AssetId, ExportZenlink};

	#[allow(type_alias_bounds)]
	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

	#[allow(type_alias_bounds)]
	pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::CurrencyId;

	#[allow(type_alias_bounds)]
	pub type BalanceOf<T> =
		<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type DexOperator: ExportZenlink<Self::AccountId>;

		/// The interface to call VtokenMinting module functions.
		type VtokenMintingInterface: VtokenMintingInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;

		/// xtokens xcm transfer interface
		type XcmTransfer: XcmTransfer<AccountIdOf<Self>, BalanceOf<Self>, CurrencyIdOf<Self>>;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		XcmMinted {
			receiver: [u8; 20],
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		},
		XcmRedeemed {
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		},
		XcmSwapped {
			receiver: [u8; 20],
			amount_in_max: BalanceOf<T>,
			amount_out: BalanceOf<T>,
			in_currency_id: CurrencyIdOf<T>,
			out_currency_id: CurrencyIdOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Token not found in vtoken minting
		TokenNotFoundInVtokenMinting,
		/// Token not found in zenlink
		TokenNotFoundInZenlink,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// vtoken-minting mint
		#[pallet::weight(<T as Config>::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			receiver: [u8; 20],
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?; // use the derivative acount from OriginConverter
			T::VtokenMintingInterface::mint(who.clone(), token_id, token_amount)?;
			let vtoken_id = T::VtokenMintingInterface::vtoken_id(token_id)
				.ok_or(Error::<T>::TokenNotFoundInVtokenMinting)?;
			// success
			let vtoken_balance = T::MultiCurrency::free_balance(vtoken_id, &who);
			if vtoken_balance != BalanceOf::<T>::zero() {
				T::XcmTransfer::transfer(
					who,
					vtoken_id,
					vtoken_balance,
					MultiLocation {
						parents: 1,
						interior: X2(
							Parachain(parachains::moonriver::ID),
							Junction::AccountKey20 { network: Any, key: receiver },
						),
					},
					4_000_000_000,
				)
				.ok();
			}

			Self::deposit_event(Event::XcmMinted { receiver, token_id, token_amount });

			Ok(().into())
		}

		/// vtoken-minting redeem
		#[pallet::weight(<T as Config>::WeightInfo::redeem())]
		pub fn redeem(
			origin: OriginFor<T>,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?; // use the derivative acount from OriginConverter
			T::VtokenMintingInterface::redeem(who.clone(), vtoken_id, vtoken_amount)?;

			Self::deposit_event(Event::XcmRedeemed { vtoken_id, vtoken_amount });

			Ok(().into())
		}

		/// zenlink inner_swap_assets_for_exact_assets
		#[pallet::weight(<T as Config>::WeightInfo::swap())]
		pub fn swap(
			origin: OriginFor<T>,
			receiver: [u8; 20],
			amount_in_max: BalanceOf<T>,
			amount_out: BalanceOf<T>,
			in_currency_id: CurrencyIdOf<T>,
			out_currency_id: CurrencyIdOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;

			let in_asset_id: AssetId =
				AssetId::try_convert_from(in_currency_id, T::ParachainId::get().into())
					.map_err(|_| Error::<T>::TokenNotFoundInZenlink)?;

			let out_asset_id: AssetId =
				AssetId::try_convert_from(out_currency_id, T::ParachainId::get().into())
					.map_err(|_| Error::<T>::TokenNotFoundInZenlink)?;

			let path = vec![in_asset_id, out_asset_id];
			T::DexOperator::inner_swap_assets_for_exact_assets(
				&who,
				amount_out.saturated_into(),
				amount_in_max.saturated_into(),
				&path,
				&who,
			)?;

			let out_balance = T::MultiCurrency::free_balance(out_currency_id, &who);
			if out_balance != BalanceOf::<T>::zero() {
				T::XcmTransfer::transfer(
					who,
					out_currency_id,
					out_balance,
					MultiLocation {
						parents: 1,
						interior: X2(
							Parachain(parachains::moonriver::ID),
							Junction::AccountKey20 { network: Any, key: receiver },
						),
					},
					4_000_000_000,
				)
				.ok();
			}

			Self::deposit_event(Event::XcmSwapped {
				receiver,
				amount_in_max,
				amount_out,
				in_currency_id,
				out_currency_id,
			});

			Ok(().into())
		}
	}
}

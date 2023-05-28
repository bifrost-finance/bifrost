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
use bifrost_asset_registry::AssetMetadata;
use codec::{Decode, Encode, MaxEncodedLen};
use cumulus_pallet_xcm::Origin as CumulusOrigin;
use cumulus_primitives_core::ParaId;
use frame_support::{
	dispatch::{DispatchResult, DispatchResultWithPostInfo},
	ensure,
	sp_runtime::SaturatedConversion,
	traits::Get,
	PalletId, RuntimeDebug,
};
use frame_system::{ensure_signed, pallet_prelude::OriginFor, Config as SystemConfig};
use node_primitives::{
	CurrencyId, CurrencyIdMapping, TokenSymbol, TryConvertFrom, VtokenMintingInterface,
};
use orml_traits::{MultiCurrency, XcmTransfer};
pub use pallet::*;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::{Hasher, H160};
use sp_runtime::traits::{BlakeTwo256, UniqueSaturatedFrom};
use sp_std::vec;
use xcm::{latest::prelude::*, v3::MultiLocation};
use zenlink_protocol::AssetBalance;

pub mod weights;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;
pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub const ASTAR_PARA_ID: u32 = 2006;
pub const MOONBEAM_PARA_ID: u32 = 2004;
pub const MOONRIVER_PARA_ID: u32 = 2023;

#[derive(
	Encode,
	Decode,
	MaxEncodedLen,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum TargetChain {
	Astar,
	Shiden,
	Moonbeam,
	Moonriver,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::{ValueQuery, *};
	use node_primitives::RedeemType;
	use zenlink_protocol::{AssetId, ExportZenlink};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type RuntimeOrigin: From<<Self as SystemConfig>::RuntimeOrigin>
			+ Into<Result<CumulusOrigin, <Self as Config>::RuntimeOrigin>>;
		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type DexOperator: ExportZenlink<Self::AccountId, AssetId>;

		/// The interface to call VtokenMinting module functions.
		type VtokenMintingInterface: VtokenMintingInterface<
			AccountIdOf<Self>,
			CurrencyIdOf<Self>,
			BalanceOf<Self>,
		>;

		/// xtokens xcm transfer interface
		type XcmTransfer: XcmTransfer<AccountIdOf<Self>, BalanceOf<Self>, CurrencyIdOf<Self>>;

		/// Convert MultiLocation to `T::CurrencyId`.
		type CurrencyIdConvert: CurrencyIdMapping<
			CurrencyId,
			MultiLocation,
			AssetMetadata<BalanceOf<Self>>,
		>;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type ParachainId: Get<ParaId>;

		#[pallet::constant]
		type NativeCurrencyId: Get<CurrencyIdOf<Self>>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		AddWhitelistAccountId {
			target_chain: TargetChain,
			evm_contract_account_id: AccountIdOf<T>,
		},
		RemoveWhitelistAccountId {
			target_chain: TargetChain,
			evm_contract_account_id: AccountIdOf<T>,
		},
		XcmMint {
			evm_caller: H160,
			currency_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			target_chain: TargetChain,
		},
		XcmMintFailed {
			evm_caller: H160,
			currency_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
			target_chain: TargetChain,
		},
		XcmSwap {
			evm_caller: H160,
			currency_id_in: CurrencyIdOf<T>,
			currency_id_out: CurrencyIdOf<T>,
			target_chain: TargetChain,
		},
		XcmSwapFailed {
			evm_caller: H160,
			currency_id_in: CurrencyIdOf<T>,
			currency_id_out: CurrencyIdOf<T>,
			target_chain: TargetChain,
		},
		XcmRedeem {
			evm_caller: H160,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
			target_chain: TargetChain,
		},
		XcmRedeemFailed {
			evm_caller: H160,
			vtoken_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
			target_chain: TargetChain,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Token not found in vtoken minting
		TokenNotFoundInVtokenMinting,
		/// Token not found in zenlink
		TokenNotFoundInZenlink,
		/// Accountid decode error
		DecodingError,
		/// Multilocation to Curency id convert error
		CurrencyIdConvert,
		BalanceBeforeAndAfterMintIsEqual,
		NotSetActionInfo,
		ChainNotSupported,
		VTokenMintError,
		AccountIdAlreadyInWhitelist,
		AccountIdNotInWhitelist,
		ExceededWhitelistMaxNumber,
	}

	#[pallet::storage]
	#[pallet::getter(fn whitelist_account_ids)]
	pub type WhitelistAccountId<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		TargetChain,
		BoundedVec<AccountIdOf<T>, ConstU32<10>>,
		ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// vtoken mint and transfer to target chain
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::mint())]
		pub fn mint(
			origin: OriginFor<T>,
			evm_caller: H160,
			currency_id: CurrencyIdOf<T>,
			target_chain: TargetChain,
		) -> DispatchResultWithPostInfo {
			let evm_contract_account_id = ensure_signed(origin)?;
			let evm_caller_account_id = Self::h160_to_account_id(evm_caller);
			Self::ensure_singer_on_whitelist(&evm_contract_account_id, target_chain)?;

			if target_chain == TargetChain::Moonbeam || target_chain == TargetChain::Moonriver {
				T::MultiCurrency::transfer(
					CurrencyId::Native(TokenSymbol::BNC),
					&evm_contract_account_id,
					&evm_caller_account_id,
					BalanceOf::<T>::unique_saturated_from(1_000_000_000_000u128),
				)?;
			}

			let token_amount = T::MultiCurrency::free_balance(currency_id, &evm_caller_account_id);
			match T::VtokenMintingInterface::mint(
				evm_caller_account_id.clone(),
				currency_id,
				token_amount,
			) {
				Ok(_) => {
					// success
					let vtoken_id = T::VtokenMintingInterface::vtoken_id(currency_id)
						.ok_or(Error::<T>::TokenNotFoundInVtokenMinting)?;
					let vtoken_amount =
						T::MultiCurrency::free_balance(vtoken_id, &evm_caller_account_id);

					match target_chain {
						TargetChain::Astar => Self::transfer_assets_to_astr(
							evm_caller_account_id.clone(),
							vtoken_id,
							vtoken_amount,
							evm_caller_account_id.clone(),
						)?,
						TargetChain::Moonbeam => Self::transfer_multiassets_to_moonbeam(
							evm_caller_account_id.clone(),
							vtoken_id,
							vtoken_amount,
							evm_caller,
							MOONBEAM_PARA_ID,
						)?,
						TargetChain::Moonriver => Self::transfer_multiassets_to_moonbeam(
							evm_caller_account_id.clone(),
							vtoken_id,
							vtoken_amount,
							evm_caller,
							MOONRIVER_PARA_ID,
						)?,
						_ => ensure!(false, Error::<T>::ChainNotSupported),
					};

					Self::deposit_event(Event::XcmMint {
						evm_caller,
						currency_id,
						token_amount,
						target_chain,
					});
				},
				Err(_) => {
					match target_chain {
						TargetChain::Astar => Self::transfer_assets_to_astr(
							evm_caller_account_id.clone(),
							currency_id,
							token_amount,
							evm_caller_account_id.clone(),
						)?,
						TargetChain::Moonbeam => Self::transfer_multiassets_to_moonbeam(
							evm_caller_account_id.clone(),
							currency_id,
							token_amount,
							evm_caller,
							MOONBEAM_PARA_ID,
						)?,
						TargetChain::Moonriver => Self::transfer_multiassets_to_moonbeam(
							evm_caller_account_id.clone(),
							currency_id,
							token_amount,
							evm_caller,
							MOONRIVER_PARA_ID,
						)?,
						_ => ensure!(false, Error::<T>::ChainNotSupported),
					}
					Self::deposit_event(Event::XcmMintFailed {
						evm_caller,
						currency_id,
						token_amount,
						target_chain,
					});
				},
			};
			Ok(().into())
		}

		/// zenlink inner_swap_assets_for_exact_assets
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::swap())]
		pub fn swap(
			origin: OriginFor<T>,
			evm_caller: H160,
			currency_id_in: CurrencyIdOf<T>,
			currency_id_out: CurrencyIdOf<T>,
			currency_id_out_min: AssetBalance,
			target_chain: TargetChain,
		) -> DispatchResultWithPostInfo {
			let evm_contract_account_id = ensure_signed(origin)?;
			let evm_caller_account_id = Self::h160_to_account_id(evm_caller);
			Self::ensure_singer_on_whitelist(&evm_contract_account_id, target_chain)?;

			if target_chain == TargetChain::Moonbeam || target_chain == TargetChain::Moonriver {
				T::MultiCurrency::transfer(
					CurrencyId::Native(TokenSymbol::BNC),
					&evm_contract_account_id,
					&evm_caller_account_id,
					BalanceOf::<T>::unique_saturated_from(1_000_000_000_000u128),
				)?;
			}

			let in_asset_id: AssetId =
				AssetId::try_convert_from(currency_id_in, T::ParachainId::get().into())
					.map_err(|_| Error::<T>::TokenNotFoundInZenlink)?;
			let out_asset_id: AssetId =
				AssetId::try_convert_from(currency_id_out, T::ParachainId::get().into())
					.map_err(|_| Error::<T>::TokenNotFoundInZenlink)?;

			let currency_id_in_amount =
				T::MultiCurrency::free_balance(currency_id_in, &evm_caller_account_id);
			let path = vec![in_asset_id, out_asset_id];
			match T::DexOperator::inner_swap_exact_assets_for_assets(
				&evm_caller_account_id,
				currency_id_in_amount.saturated_into(),
				currency_id_out_min,
				&path,
				&evm_caller_account_id,
			) {
				Ok(_) => {
					let currency_id_out_amount =
						T::MultiCurrency::free_balance(currency_id_out, &evm_caller_account_id);

					match target_chain {
						TargetChain::Astar => Self::transfer_assets_to_astr(
							evm_caller_account_id.clone(),
							currency_id_out,
							currency_id_out_amount,
							evm_caller_account_id.clone(),
						)?,
						TargetChain::Moonbeam => Self::transfer_multiassets_to_moonbeam(
							evm_caller_account_id.clone(),
							currency_id_out,
							currency_id_out_amount,
							evm_caller,
							MOONBEAM_PARA_ID,
						)?,
						TargetChain::Moonriver => Self::transfer_multiassets_to_moonbeam(
							evm_caller_account_id.clone(),
							currency_id_out,
							currency_id_out_amount,
							evm_caller,
							MOONRIVER_PARA_ID,
						)?,
						_ => ensure!(false, Error::<T>::ChainNotSupported),
					}

					Self::deposit_event(Event::XcmSwap {
						evm_caller,
						currency_id_in,
						currency_id_out,
						target_chain,
					});
				},
				Err(_) => match target_chain {
					TargetChain::Astar => Self::transfer_assets_to_astr(
						evm_caller_account_id.clone(),
						currency_id_in,
						currency_id_in_amount,
						evm_caller_account_id.clone(),
					)?,
					TargetChain::Moonbeam => Self::transfer_multiassets_to_moonbeam(
						evm_caller_account_id.clone(),
						currency_id_in,
						currency_id_in_amount,
						evm_caller,
						MOONBEAM_PARA_ID,
					)?,
					TargetChain::Moonriver => Self::transfer_multiassets_to_moonbeam(
						evm_caller_account_id.clone(),
						currency_id_in,
						currency_id_in_amount,
						evm_caller,
						MOONRIVER_PARA_ID,
					)?,
					_ => ensure!(false, Error::<T>::ChainNotSupported),
				},
			}
			Ok(().into())
		}
		/// Redeem
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::swap())]
		pub fn redeem(
			origin: OriginFor<T>,
			evm_caller: H160,
			vtoken_id: CurrencyIdOf<T>,
			target_chain: TargetChain,
		) -> DispatchResultWithPostInfo {
			let evm_contract_account_id = ensure_signed(origin)?;
			let evm_caller_account_id = Self::h160_to_account_id(evm_caller);
			Self::ensure_singer_on_whitelist(&evm_contract_account_id, target_chain)?;

			if target_chain == TargetChain::Moonbeam || target_chain == TargetChain::Moonriver {
				T::MultiCurrency::transfer(
					CurrencyId::Native(TokenSymbol::BNC),
					&evm_contract_account_id,
					&evm_caller_account_id,
					BalanceOf::<T>::unique_saturated_from(1_000_000_000_000u128),
				)?;
			}

			let vtoken_amount = T::MultiCurrency::free_balance(vtoken_id, &evm_caller_account_id);

			let redeem_type = match target_chain {
				TargetChain::Astar => RedeemType::Astar,
				TargetChain::Moonbeam => RedeemType::Moonbeam(evm_caller),
				_ => {
					ensure!(false, Error::<T>::ChainNotSupported);
					Default::default()
				},
			};

			match T::VtokenMintingInterface::xcm_action_redeem(
				evm_caller_account_id.clone(),
				vtoken_id,
				vtoken_amount,
				redeem_type,
			) {
				Ok(_) => Self::deposit_event(Event::XcmRedeem {
					evm_caller,
					vtoken_id,
					vtoken_amount,
					target_chain,
				}),
				Err(_) => {
					match target_chain {
						TargetChain::Astar => Self::transfer_assets_to_astr(
							evm_caller_account_id.clone(),
							vtoken_id,
							vtoken_amount,
							evm_caller_account_id.clone(),
						)?,
						TargetChain::Moonbeam => Self::transfer_multiassets_to_moonbeam(
							evm_caller_account_id.clone(),
							vtoken_id,
							vtoken_amount,
							evm_caller,
							MOONBEAM_PARA_ID,
						)?,
						TargetChain::Moonriver => Self::transfer_multiassets_to_moonbeam(
							evm_caller_account_id.clone(),
							vtoken_id,
							vtoken_amount,
							evm_caller,
							MOONRIVER_PARA_ID,
						)?,
						_ => ensure!(false, Error::<T>::ChainNotSupported),
					}
					Self::deposit_event(Event::XcmRedeemFailed {
						evm_caller,
						vtoken_id,
						vtoken_amount,
						target_chain,
					});
				},
			};
			Ok(().into())
		}
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::mint())]
		pub fn add_whitelist(
			origin: OriginFor<T>,
			target_chain: TargetChain,
			evm_contract_account_id: T::AccountId,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			let mut whitelist_account_ids = WhitelistAccountId::<T>::get(&target_chain);

			ensure!(
				!whitelist_account_ids.contains(&evm_contract_account_id),
				Error::<T>::AccountIdAlreadyInWhitelist
			);
			whitelist_account_ids
				.try_push(evm_contract_account_id.clone())
				.map_err(|_| Error::<T>::ExceededWhitelistMaxNumber)?;
			WhitelistAccountId::<T>::insert(target_chain, whitelist_account_ids);
			Self::deposit_event(Event::AddWhitelistAccountId {
				target_chain,
				evm_contract_account_id,
			});
			Ok(().into())
		}
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::mint())]
		pub fn remove_whitelist(
			origin: OriginFor<T>,
			target_chain: TargetChain,
			evm_contract_account_id: T::AccountId,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;

			let mut whitelist_account_ids = WhitelistAccountId::<T>::get(&target_chain);

			ensure!(
				whitelist_account_ids.contains(&evm_contract_account_id),
				Error::<T>::AccountIdNotInWhitelist
			);
			whitelist_account_ids.retain(|x| *x != evm_contract_account_id);
			WhitelistAccountId::<T>::insert(target_chain, whitelist_account_ids);
			Self::deposit_event(Event::RemoveWhitelistAccountId {
				target_chain,
				evm_contract_account_id,
			});
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn ensure_singer_on_whitelist(
		evm_contract_account_id: &T::AccountId,
		target_chain: TargetChain,
	) -> DispatchResult {
		let whitelist_account_ids = WhitelistAccountId::<T>::get(&target_chain);
		ensure!(
			whitelist_account_ids.contains(evm_contract_account_id),
			Error::<T>::AccountIdNotInWhitelist
		);
		Ok(())
	}

	fn transfer_assets_to_astr(
		caller: T::AccountId,
		currency_id: CurrencyIdOf<T>,
		amount: BalanceOf<T>,
		receiver: T::AccountId,
	) -> DispatchResult {
		let dest = MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(ASTAR_PARA_ID),
				AccountId32 { network: None, id: receiver.encode().try_into().unwrap() },
			),
		};

		T::XcmTransfer::transfer(caller, currency_id, amount, dest, Unlimited)?;
		Ok(())
	}

	fn transfer_multiassets_to_moonbeam(
		caller: T::AccountId,
		currency_id: CurrencyIdOf<T>,
		amount: BalanceOf<T>,
		receiver: H160,
		para_id: u32,
	) -> DispatchResult {
		let dest = MultiLocation {
			parents: 1,
			interior: X2(
				Parachain(para_id),
				AccountKey20 { network: None, key: receiver.to_fixed_bytes() },
			),
		};

		let fee = CurrencyId::Native(TokenSymbol::BNC);
		let fee_amount = BalanceOf::<T>::unique_saturated_from(1_000_000_000_000u128);

		let assets = vec![(currency_id, amount), (fee, fee_amount)];

		T::XcmTransfer::transfer_multicurrencies(caller, assets, 1, dest, Unlimited)?;
		Ok(())
	}

	fn h160_to_account_id(address: H160) -> T::AccountId {
		let mut data = [0u8; 24];
		data[0..4].copy_from_slice(b"evm:");
		data[4..24].copy_from_slice(&address[..]);
		let hash = BlakeTwo256::hash(&data);

		let account_id_32 = sp_runtime::AccountId32::from(Into::<[u8; 32]>::into(hash));
		T::AccountId::decode(&mut account_id_32.as_ref()).expect("Fail to decode address")
	}
}

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

//! # Asset Registry Module
//!
//! Local and foreign assets management. The foreign assets can be updated without runtime upgrade.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::*,
	traits::{Currency, EnsureOrigin},
	weights::constants::WEIGHT_PER_SECOND,
	RuntimeDebug,
};
use frame_system::pallet_prelude::*;
use primitives::{
	AssetIds, CurrencyId,
	CurrencyId::{Native, Token, Token2},
	CurrencyIdConversion, CurrencyIdMapping, CurrencyIdRegister, ForeignAssetId, LeasePeriod,
	ParaId, TokenId, TokenInfo, TokenSymbol,
};
use scale_info::TypeInfo;
use sp_runtime::{traits::One, ArithmeticError, FixedPointNumber, FixedU128};
use sp_std::{boxed::Box, vec::Vec};
// NOTE:v1::MultiLocation is used in storages, we would need to do migration if upgrade the
// MultiLocation in the future.
use xcm::{
	opaque::latest::{prelude::XcmError, AssetId, Fungibility::Fungible, MultiAsset},
	v1::MultiLocation,
	VersionedMultiLocation,
};
use xcm_builder::TakeRevenue;
use xcm_executor::{traits::WeightTrader, Assets};

mod mock;
mod tests;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub use pallet::*;
pub use weights::WeightInfo;

/// Type alias for currency balance.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Currency type for withdraw and balance storage.
		type Currency: Currency<Self::AccountId>;

		/// Required origin for registering asset.
		type RegisterOrigin: EnsureOrigin<Self::Origin>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[derive(Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, TypeInfo)]
	pub struct AssetMetadata<Balance> {
		pub name: Vec<u8>,
		pub symbol: Vec<u8>,
		pub decimals: u8,
		pub minimal_balance: Balance,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The given location could not be used (e.g. because it cannot be expressed in the
		/// desired version of XCM).
		BadLocation,
		/// MultiLocation existed
		MultiLocationExisted,
		/// AssetId not exists
		AssetIdNotExists,
		/// AssetId exists
		AssetIdExisted,
		/// CurrencyId not exists
		CurrencyIdNotExists,
		/// CurrencyId exists
		CurrencyIdExisted,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The foreign asset registered.
		ForeignAssetRegistered {
			asset_id: ForeignAssetId,
			asset_address: MultiLocation,
			metadata: AssetMetadata<BalanceOf<T>>,
		},
		/// The foreign asset updated.
		ForeignAssetUpdated {
			asset_id: ForeignAssetId,
			asset_address: MultiLocation,
			metadata: AssetMetadata<BalanceOf<T>>,
		},
		/// The asset registered.
		AssetRegistered { asset_id: AssetIds, metadata: AssetMetadata<BalanceOf<T>> },
		/// The asset updated.
		AssetUpdated { asset_id: AssetIds, metadata: AssetMetadata<BalanceOf<T>> },
		/// The CurrencyId registered.
		CurrencyIdRegistered { currency_id: CurrencyId, metadata: AssetMetadata<BalanceOf<T>> },
	}

	/// Next available Foreign AssetId ID.
	///
	/// NextForeignAssetId: ForeignAssetId
	#[pallet::storage]
	#[pallet::getter(fn next_foreign_asset_id)]
	pub type NextForeignAssetId<T: Config> = StorageValue<_, ForeignAssetId, ValueQuery>;

	/// Next available TokenId ID.
	///
	/// NextTokenId: TokenId
	#[pallet::storage]
	#[pallet::getter(fn next_token_id)]
	pub type NextTokenId<T: Config> = StorageValue<_, TokenId, ValueQuery>;

	/// The storages for MultiLocations.
	///
	/// CurrencyIdToLocations: map CurrencyId => Option<MultiLocation>
	#[pallet::storage]
	#[pallet::getter(fn currency_id_to_locations)]
	pub type CurrencyIdToLocations<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, MultiLocation, OptionQuery>;

	/// The storages for CurrencyIds.
	///
	/// LocationToCurrencyIds: map MultiLocation => Option<CurrencyId>
	#[pallet::storage]
	#[pallet::getter(fn location_to_currency_ids)]
	pub type LocationToCurrencyIds<T: Config> =
		StorageMap<_, Twox64Concat, MultiLocation, CurrencyId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn currency_id_to_weight)]
	pub type CurrencyIdToWeights<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, u128, OptionQuery>;

	/// The storages for AssetMetadatas.
	///
	/// AssetMetadatas: map AssetIds => Option<AssetMetadata>
	#[pallet::storage]
	#[pallet::getter(fn asset_metadatas)]
	pub type AssetMetadatas<T: Config> =
		StorageMap<_, Twox64Concat, AssetIds, AssetMetadata<BalanceOf<T>>, OptionQuery>;

	/// The storages for AssetMetadata.
	///
	/// CurrencyMetadatas: map CurrencyId => Option<AssetMetadata>
	#[pallet::storage]
	#[pallet::getter(fn currency_metadatas)]
	pub type CurrencyMetadatas<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, AssetMetadata<BalanceOf<T>>, OptionQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub currency: Vec<(CurrencyId, BalanceOf<T>)>,
		pub vcurrency: Vec<CurrencyId>,
		pub vsbond: Vec<(CurrencyId, u32, u32, u32)>,
		pub phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				currency: Default::default(),
				vcurrency: Default::default(),
				vsbond: Default::default(),
				phantom: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (currency_id, metadata) in
				self.currency.iter().map(|(currency_id, minimal_balance)| {
					(
						currency_id,
						AssetMetadata {
							name: currency_id
								.name()
								.map(|s| s.as_bytes().to_vec())
								.unwrap_or_default(),
							symbol: currency_id
								.symbol()
								.map(|s| s.as_bytes().to_vec())
								.unwrap_or_default(),
							decimals: currency_id.decimals().unwrap_or_default(),
							minimal_balance: *minimal_balance,
						},
					)
				}) {
				Pallet::<T>::do_register_metadata(*currency_id, &metadata).expect("Token register");
			}

			for (currency, para_id, first_slot, last_slot) in self.vsbond.iter() {
				match currency {
					Token(symbol) | Native(symbol) => {
						AssetIdMaps::<T>::register_vsbond_metadata(
							*symbol,
							*para_id,
							*first_slot,
							*last_slot,
						)
						.expect("VSBond register");
					},
					Token2(token_id) => {
						AssetIdMaps::<T>::register_vsbond2_metadata(
							*token_id,
							*para_id,
							*first_slot,
							*last_slot,
						)
						.expect("VToken register");
					},
					_ => (),
				}
			}

			for &currency in self.vcurrency.iter() {
				match currency {
					CurrencyId::VToken(symbol) => {
						AssetIdMaps::<T>::register_vtoken_metadata(symbol)
							.expect("VToken register");
					},
					CurrencyId::VToken2(token_id) => {
						AssetIdMaps::<T>::register_vtoken2_metadata(token_id)
							.expect("VToken register");
					},
					CurrencyId::VSToken(symbol) => {
						AssetIdMaps::<T>::register_vstoken_metadata(symbol)
							.expect("VSToken register");
					},
					CurrencyId::VSToken2(token_id) => {
						AssetIdMaps::<T>::register_vstoken2_metadata(token_id)
							.expect("VSToken register");
					},
					_ => (),
				}
			}
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::register_foreign_asset())]
		pub fn register_foreign_asset(
			origin: OriginFor<T>,
			location: Box<VersionedMultiLocation>,
			metadata: Box<AssetMetadata<BalanceOf<T>>>,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			let location: MultiLocation =
				(*location).try_into().map_err(|()| Error::<T>::BadLocation)?;
			let foreign_asset_id = Self::do_register_foreign_asset(&location, &metadata)?;

			Self::deposit_event(Event::<T>::ForeignAssetRegistered {
				asset_id: foreign_asset_id,
				asset_address: location,
				metadata: *metadata,
			});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::update_foreign_asset())]
		pub fn update_foreign_asset(
			origin: OriginFor<T>,
			foreign_asset_id: ForeignAssetId,
			location: Box<VersionedMultiLocation>,
			metadata: Box<AssetMetadata<BalanceOf<T>>>,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			let location: MultiLocation =
				(*location).try_into().map_err(|()| Error::<T>::BadLocation)?;
			Self::do_update_foreign_asset(foreign_asset_id, &location, &metadata)?;

			Self::deposit_event(Event::<T>::ForeignAssetUpdated {
				asset_id: foreign_asset_id,
				asset_address: location,
				metadata: *metadata,
			});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::register_native_asset())]
		pub fn register_native_asset(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			location: Box<VersionedMultiLocation>,
			metadata: Box<AssetMetadata<BalanceOf<T>>>,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			let location: MultiLocation =
				(*location).try_into().map_err(|()| Error::<T>::BadLocation)?;
			Self::do_register_native_asset(currency_id, &location, &metadata)?;

			Self::deposit_event(Event::<T>::AssetRegistered {
				asset_id: AssetIds::NativeAssetId(currency_id),
				metadata: *metadata,
			});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::update_native_asset())]
		pub fn update_native_asset(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			location: Box<VersionedMultiLocation>,
			metadata: Box<AssetMetadata<BalanceOf<T>>>,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			let location: MultiLocation =
				(*location).try_into().map_err(|()| Error::<T>::BadLocation)?;
			Self::do_update_native_asset(currency_id, &location, &metadata)?;

			Self::deposit_event(Event::<T>::AssetUpdated {
				asset_id: AssetIds::NativeAssetId(currency_id),
				metadata: *metadata,
			});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::register_token_metadata())]
		pub fn register_token_metadata(
			origin: OriginFor<T>,
			metadata: Box<AssetMetadata<BalanceOf<T>>>,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			let token_id = Self::get_next_token_id()?;
			let currency_id = CurrencyId::Token2(token_id);
			Self::do_register_metadata(currency_id, &metadata)?;

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::register_vtoken_metadata())]
		pub fn register_vtoken_metadata(origin: OriginFor<T>, token_id: TokenId) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			if let Some(token_metadata) = CurrencyMetadatas::<T>::get(CurrencyId::Token2(token_id))
			{
				let vtoken_metadata = Self::convert_to_vtoken_metadata(token_metadata);
				Self::do_register_metadata(CurrencyId::VToken2(token_id), &vtoken_metadata)?;

				return Ok(());
			} else {
				return Err(Error::<T>::CurrencyIdNotExists)?;
			}
		}

		#[pallet::weight(T::WeightInfo::register_vstoken_metadata())]
		pub fn register_vstoken_metadata(
			origin: OriginFor<T>,
			token_id: TokenId,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			if let Some(token_metadata) = CurrencyMetadatas::<T>::get(CurrencyId::Token2(token_id))
			{
				let vstoken_metadata = Self::convert_to_vstoken_metadata(token_metadata);
				Self::do_register_metadata(CurrencyId::VSToken2(token_id), &vstoken_metadata)?;

				return Ok(());
			} else {
				return Err(Error::<T>::CurrencyIdNotExists)?;
			}
		}

		#[pallet::weight(T::WeightInfo::register_vsbond_metadata())]
		pub fn register_vsbond_metadata(
			origin: OriginFor<T>,
			token_id: TokenId,
			para_id: ParaId,
			first_slot: LeasePeriod,
			last_slot: LeasePeriod,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			if let Some(token_metadata) = CurrencyMetadatas::<T>::get(CurrencyId::Token2(token_id))
			{
				let vsbond_metadata = Self::convert_to_vsbond_metadata(
					token_metadata,
					para_id,
					first_slot,
					last_slot,
				);
				Self::do_register_metadata(
					CurrencyId::VSBond2(token_id, para_id, first_slot, last_slot),
					&vsbond_metadata,
				)?;

				return Ok(());
			} else {
				return Err(Error::<T>::CurrencyIdNotExists)?;
			}
		}

		#[pallet::weight(T::WeightInfo::register_multilocation())]
		pub fn register_multilocation(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			location: Box<VersionedMultiLocation>,
			weight: u128,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			let location: MultiLocation =
				(*location).try_into().map_err(|()| Error::<T>::BadLocation)?;
			Self::do_register_multilocation(currency_id, &location)?;
			Self::do_register_weight(currency_id, weight)?;

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn get_next_foreign_asset_id() -> Result<ForeignAssetId, DispatchError> {
		NextForeignAssetId::<T>::try_mutate(|current| -> Result<ForeignAssetId, DispatchError> {
			let id = *current;
			*current = current.checked_add(One::one()).ok_or(ArithmeticError::Overflow)?;
			Ok(id)
		})
	}

	pub fn get_next_token_id() -> Result<TokenId, DispatchError> {
		NextTokenId::<T>::try_mutate(|current| -> Result<TokenId, DispatchError> {
			let id = *current;
			*current = current.checked_add(One::one()).ok_or(ArithmeticError::Overflow)?;
			Ok(id)
		})
	}

	fn do_register_foreign_asset(
		location: &MultiLocation,
		metadata: &AssetMetadata<BalanceOf<T>>,
	) -> Result<ForeignAssetId, DispatchError> {
		let foreign_asset_id = Self::get_next_foreign_asset_id()?;
		LocationToCurrencyIds::<T>::try_mutate(location, |maybe_currency_ids| -> DispatchResult {
			ensure!(maybe_currency_ids.is_none(), Error::<T>::MultiLocationExisted);
			*maybe_currency_ids = Some(CurrencyId::ForeignAsset(foreign_asset_id));

			CurrencyIdToLocations::<T>::try_mutate(
				CurrencyId::ForeignAsset(foreign_asset_id),
				|maybe_location| -> DispatchResult {
					ensure!(maybe_location.is_none(), Error::<T>::MultiLocationExisted);
					*maybe_location = Some(location.clone());

					AssetMetadatas::<T>::try_mutate(
						AssetIds::ForeignAssetId(foreign_asset_id),
						|maybe_asset_metadatas| -> DispatchResult {
							ensure!(maybe_asset_metadatas.is_none(), Error::<T>::AssetIdExisted);

							*maybe_asset_metadatas = Some(metadata.clone());
							Ok(())
						},
					)
				},
			)
		})?;

		Ok(foreign_asset_id)
	}

	fn do_update_foreign_asset(
		foreign_asset_id: ForeignAssetId,
		location: &MultiLocation,
		metadata: &AssetMetadata<BalanceOf<T>>,
	) -> DispatchResult {
		CurrencyIdToLocations::<T>::try_mutate(
			CurrencyId::ForeignAsset(foreign_asset_id),
			|maybe_multi_locations| -> DispatchResult {
				let old_multi_locations =
					maybe_multi_locations.as_mut().ok_or(Error::<T>::AssetIdNotExists)?;

				AssetMetadatas::<T>::try_mutate(
					AssetIds::ForeignAssetId(foreign_asset_id),
					|maybe_asset_metadatas| -> DispatchResult {
						ensure!(maybe_asset_metadatas.is_some(), Error::<T>::AssetIdNotExists);

						// modify location
						if location != old_multi_locations {
							LocationToCurrencyIds::<T>::remove(old_multi_locations.clone());
							LocationToCurrencyIds::<T>::try_mutate(
								location,
								|maybe_currency_ids| -> DispatchResult {
									ensure!(
										maybe_currency_ids.is_none(),
										Error::<T>::MultiLocationExisted
									);
									*maybe_currency_ids =
										Some(CurrencyId::ForeignAsset(foreign_asset_id));
									Ok(())
								},
							)?;
						}
						*maybe_asset_metadatas = Some(metadata.clone());
						*old_multi_locations = location.clone();
						Ok(())
					},
				)
			},
		)
	}

	pub fn do_register_native_asset(
		currency_id: CurrencyId,
		location: &MultiLocation,
		metadata: &AssetMetadata<BalanceOf<T>>,
	) -> DispatchResult {
		ensure!(LocationToCurrencyIds::<T>::get(location).is_none(), Error::<T>::AssetIdExisted);
		ensure!(
			CurrencyIdToLocations::<T>::get(currency_id).is_none(),
			Error::<T>::MultiLocationExisted
		);
		ensure!(
			AssetMetadatas::<T>::get(AssetIds::NativeAssetId(currency_id)).is_none(),
			Error::<T>::AssetIdExisted
		);

		LocationToCurrencyIds::<T>::insert(location, currency_id);
		CurrencyIdToLocations::<T>::insert(currency_id, location);
		AssetMetadatas::<T>::insert(AssetIds::NativeAssetId(currency_id), metadata);

		Ok(())
	}

	pub fn convert_to_vtoken_metadata(
		token_metadata: AssetMetadata<BalanceOf<T>>,
	) -> AssetMetadata<BalanceOf<T>> {
		let mut name = "Voucher ".as_bytes().to_vec();
		name.extend_from_slice(&token_metadata.symbol);
		let mut symbol = "v".as_bytes().to_vec();
		symbol.extend_from_slice(&token_metadata.symbol);
		AssetMetadata { name, symbol, ..token_metadata }
	}

	pub fn convert_to_vstoken_metadata(
		token_metadata: AssetMetadata<BalanceOf<T>>,
	) -> AssetMetadata<BalanceOf<T>> {
		let mut name = "Voucher Slot ".as_bytes().to_vec();
		name.extend_from_slice(&token_metadata.symbol);
		let mut symbol = "vs".as_bytes().to_vec();
		symbol.extend_from_slice(&token_metadata.symbol);
		AssetMetadata { name, symbol, ..token_metadata }
	}

	pub fn convert_to_vsbond_metadata(
		token_metadata: AssetMetadata<BalanceOf<T>>,
		para_id: ParaId,
		first_slot: LeasePeriod,
		last_slot: LeasePeriod,
	) -> AssetMetadata<BalanceOf<T>> {
		let name = scale_info::prelude::format!(
			"vsBOND-{}-{}-{}-{}",
			core::str::from_utf8(&token_metadata.symbol).unwrap_or(""),
			para_id,
			first_slot,
			last_slot
		)
		.as_bytes()
		.to_vec();
		AssetMetadata { name: name.clone(), symbol: name, ..token_metadata }
	}

	pub fn do_register_metadata(
		currency_id: CurrencyId,
		metadata: &AssetMetadata<BalanceOf<T>>,
	) -> DispatchResult {
		ensure!(CurrencyMetadatas::<T>::get(currency_id).is_none(), Error::<T>::CurrencyIdExisted);

		CurrencyMetadatas::<T>::insert(currency_id, metadata.clone());

		Pallet::<T>::deposit_event(Event::<T>::CurrencyIdRegistered {
			currency_id,
			metadata: metadata.clone(),
		});

		Ok(())
	}

	pub fn do_register_multilocation(
		currency_id: CurrencyId,
		location: &MultiLocation,
	) -> DispatchResult {
		ensure!(
			CurrencyMetadatas::<T>::get(currency_id).is_some(),
			Error::<T>::CurrencyIdNotExists
		);
		ensure!(LocationToCurrencyIds::<T>::get(location).is_none(), Error::<T>::CurrencyIdExisted);
		ensure!(
			CurrencyIdToLocations::<T>::get(currency_id).is_none(),
			Error::<T>::MultiLocationExisted
		);

		LocationToCurrencyIds::<T>::insert(location, currency_id);
		CurrencyIdToLocations::<T>::insert(currency_id, location);

		Ok(())
	}

	pub fn do_register_weight(currency_id: CurrencyId, weight: u128) -> DispatchResult {
		ensure!(
			CurrencyMetadatas::<T>::get(currency_id).is_some(),
			Error::<T>::CurrencyIdNotExists
		);

		CurrencyIdToWeights::<T>::insert(currency_id, weight);

		Ok(())
	}

	fn do_update_native_asset(
		currency_id: CurrencyId,
		location: &MultiLocation,
		metadata: &AssetMetadata<BalanceOf<T>>,
	) -> DispatchResult {
		ensure!(LocationToCurrencyIds::<T>::get(location).is_some(), Error::<T>::AssetIdNotExists);
		ensure!(
			CurrencyIdToLocations::<T>::get(currency_id).is_some(),
			Error::<T>::MultiLocationExisted
		);
		ensure!(
			AssetMetadatas::<T>::get(AssetIds::NativeAssetId(currency_id)).is_some(),
			Error::<T>::AssetIdNotExists
		);

		LocationToCurrencyIds::<T>::insert(location, currency_id);
		CurrencyIdToLocations::<T>::insert(currency_id, location);
		AssetMetadatas::<T>::insert(AssetIds::NativeAssetId(currency_id), metadata);

		Ok(())
	}
}

pub struct AssetIdMaps<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> CurrencyIdMapping<CurrencyId, MultiLocation, AssetMetadata<BalanceOf<T>>>
	for AssetIdMaps<T>
{
	fn get_asset_metadata(asset_ids: AssetIds) -> Option<AssetMetadata<BalanceOf<T>>> {
		Pallet::<T>::asset_metadatas(asset_ids)
	}

	fn get_currency_metadata(currency_id: CurrencyId) -> Option<AssetMetadata<BalanceOf<T>>> {
		Pallet::<T>::currency_metadatas(currency_id)
	}

	fn get_multi_location(currency_id: CurrencyId) -> Option<MultiLocation> {
		Pallet::<T>::currency_id_to_locations(currency_id)
	}

	fn get_currency_id(multi_location: MultiLocation) -> Option<CurrencyId> {
		Pallet::<T>::location_to_currency_ids(multi_location)
	}
}

impl<T: Config> CurrencyIdConversion<CurrencyId> for AssetIdMaps<T> {
	fn convert_to_token(currency_id: CurrencyId) -> Result<CurrencyId, ()> {
		match currency_id {
			CurrencyId::VSBond(TokenSymbol::BNC, 2001, 13, 20) =>
				Ok(CurrencyId::Token(TokenSymbol::KSM)),
			CurrencyId::VToken(TokenSymbol::BNC) => Ok(CurrencyId::Native(TokenSymbol::BNC)),
			CurrencyId::VToken(token_symbol) |
			CurrencyId::VSToken(token_symbol) |
			CurrencyId::VSBond(token_symbol, ..) => Ok(CurrencyId::Token(token_symbol)),
			CurrencyId::VToken2(token_id) |
			CurrencyId::VSToken2(token_id) |
			CurrencyId::VSBond2(token_id, ..) => Ok(CurrencyId::Token2(token_id)),
			_ => Err(()),
		}
	}

	fn convert_to_vtoken(currency_id: CurrencyId) -> Result<CurrencyId, ()> {
		match currency_id {
			CurrencyId::Token(token_symbol) | CurrencyId::Native(token_symbol) =>
				Ok(CurrencyId::VToken(token_symbol)),
			CurrencyId::Token2(token_id) => Ok(CurrencyId::VToken2(token_id)),
			_ => Err(()),
		}
	}

	fn convert_to_vstoken(currency_id: CurrencyId) -> Result<CurrencyId, ()> {
		match currency_id {
			CurrencyId::Token(token_symbol) => Ok(CurrencyId::VSToken(token_symbol)),
			CurrencyId::Token2(token_id) => Ok(CurrencyId::VSToken2(token_id)),
			_ => Err(()),
		}
	}

	fn convert_to_vsbond(
		currency_id: CurrencyId,
		index: ParaId,
		first_slot: LeasePeriod,
		last_slot: LeasePeriod,
	) -> Result<CurrencyId, ()> {
		match currency_id {
			CurrencyId::Token(token_symbol) => {
				let mut vs_bond = CurrencyId::VSBond(token_symbol, index, first_slot, last_slot);
				if vs_bond == CurrencyId::VSBond(TokenSymbol::KSM, 2001, 13, 20) {
					// fix vsBOND::BNC
					vs_bond = CurrencyId::VSBond(TokenSymbol::BNC, 2001, 13, 20);
				}
				Ok(vs_bond)
			},
			CurrencyId::Token2(token_id) =>
				Ok(CurrencyId::VSBond2(token_id, index, first_slot, last_slot)),
			_ => Err(()),
		}
	}
}

impl<T: Config> CurrencyIdRegister<CurrencyId> for AssetIdMaps<T> {
	fn check_token_registered(token_symbol: TokenSymbol) -> bool {
		CurrencyMetadatas::<T>::get(CurrencyId::Token(token_symbol)).is_some()
	}

	fn check_vtoken_registered(token_symbol: TokenSymbol) -> bool {
		CurrencyMetadatas::<T>::get(CurrencyId::VToken(token_symbol)).is_some()
	}

	fn check_vstoken_registered(token_symbol: TokenSymbol) -> bool {
		CurrencyMetadatas::<T>::get(CurrencyId::VSToken(token_symbol)).is_some()
	}

	fn check_vsbond_registered(
		token_symbol: TokenSymbol,
		para_id: ParaId,
		first_slot: LeasePeriod,
		last_slot: LeasePeriod,
	) -> bool {
		CurrencyMetadatas::<T>::get(CurrencyId::VSBond(
			token_symbol,
			para_id,
			first_slot,
			last_slot,
		))
		.is_some()
	}

	fn register_vtoken_metadata(token_symbol: TokenSymbol) -> sp_runtime::DispatchResult {
		if let Some(token_metadata) = CurrencyMetadatas::<T>::get(CurrencyId::Token(token_symbol)) {
			let vtoken_metadata = Pallet::<T>::convert_to_vtoken_metadata(token_metadata);
			Pallet::<T>::do_register_metadata(CurrencyId::VToken(token_symbol), &vtoken_metadata)?;
			return Ok(());
		} else if let Some(token_metadata) =
			CurrencyMetadatas::<T>::get(CurrencyId::Native(token_symbol))
		{
			let vtoken_metadata = Pallet::<T>::convert_to_vtoken_metadata(token_metadata);
			Pallet::<T>::do_register_metadata(CurrencyId::VToken(token_symbol), &vtoken_metadata)?;
			return Ok(());
		} else {
			return Err(Error::<T>::CurrencyIdNotExists.into());
		}
	}

	fn register_vstoken_metadata(token_symbol: TokenSymbol) -> sp_runtime::DispatchResult {
		if let Some(token_metadata) = CurrencyMetadatas::<T>::get(CurrencyId::Token(token_symbol)) {
			let vstoken_metadata = Pallet::<T>::convert_to_vstoken_metadata(token_metadata);
			Pallet::<T>::do_register_metadata(
				CurrencyId::VSToken(token_symbol),
				&vstoken_metadata,
			)?;

			return Ok(());
		} else {
			return Err(Error::<T>::CurrencyIdNotExists.into());
		}
	}

	fn register_vsbond_metadata(
		token_symbol: TokenSymbol,
		para_id: ParaId,
		first_slot: LeasePeriod,
		last_slot: LeasePeriod,
	) -> sp_runtime::DispatchResult {
		let option_token_metadata =
			if CurrencyMetadatas::<T>::contains_key(CurrencyId::Token(token_symbol)) {
				CurrencyMetadatas::<T>::get(CurrencyId::Token(token_symbol))
			} else if token_symbol == TokenSymbol::BNC &&
				CurrencyMetadatas::<T>::contains_key(CurrencyId::Native(token_symbol))
			{
				CurrencyMetadatas::<T>::get(CurrencyId::Native(token_symbol))
			} else {
				None
			};

		if let Some(token_metadata) = option_token_metadata {
			let vsbond_metadata = Pallet::<T>::convert_to_vsbond_metadata(
				token_metadata,
				para_id,
				first_slot,
				last_slot,
			);
			Pallet::<T>::do_register_metadata(
				CurrencyId::VSBond(token_symbol, para_id, first_slot, last_slot),
				&vsbond_metadata,
			)?;

			return Ok(());
		} else {
			return Err(Error::<T>::CurrencyIdNotExists.into());
		}
	}

	fn check_token2_registered(token_id: TokenId) -> bool {
		CurrencyMetadatas::<T>::get(CurrencyId::Token2(token_id)).is_some()
	}

	fn check_vtoken2_registered(token_id: TokenId) -> bool {
		CurrencyMetadatas::<T>::get(CurrencyId::VToken2(token_id)).is_some()
	}

	fn check_vstoken2_registered(token_id: TokenId) -> bool {
		CurrencyMetadatas::<T>::get(CurrencyId::VSToken2(token_id)).is_some()
	}

	fn check_vsbond2_registered(
		token_id: TokenId,
		para_id: ParaId,
		first_slot: LeasePeriod,
		last_slot: LeasePeriod,
	) -> bool {
		CurrencyMetadatas::<T>::get(CurrencyId::VSBond2(token_id, para_id, first_slot, last_slot))
			.is_some()
	}

	fn register_vtoken2_metadata(token_id: TokenId) -> DispatchResult {
		if let Some(token_metadata) = CurrencyMetadatas::<T>::get(CurrencyId::Token2(token_id)) {
			let vtoken_metadata = Pallet::<T>::convert_to_vtoken_metadata(token_metadata);
			Pallet::<T>::do_register_metadata(CurrencyId::VToken2(token_id), &vtoken_metadata)?;

			return Ok(());
		} else {
			return Err(Error::<T>::CurrencyIdNotExists.into());
		}
	}

	fn register_vstoken2_metadata(token_id: TokenId) -> DispatchResult {
		if let Some(token_metadata) = CurrencyMetadatas::<T>::get(CurrencyId::Token2(token_id)) {
			let vstoken_metadata = Pallet::<T>::convert_to_vstoken_metadata(token_metadata);
			Pallet::<T>::do_register_metadata(CurrencyId::VSToken2(token_id), &vstoken_metadata)?;

			return Ok(());
		} else {
			return Err(Error::<T>::CurrencyIdNotExists.into());
		}
	}

	fn register_vsbond2_metadata(
		token_id: TokenId,
		para_id: ParaId,
		first_slot: LeasePeriod,
		last_slot: LeasePeriod,
	) -> DispatchResult {
		if let Some(token_metadata) = CurrencyMetadatas::<T>::get(CurrencyId::Token2(token_id)) {
			let vsbond_metadata = Pallet::<T>::convert_to_vsbond_metadata(
				token_metadata,
				para_id,
				first_slot,
				last_slot,
			);
			Pallet::<T>::do_register_metadata(
				CurrencyId::VSBond2(token_id, para_id, first_slot, last_slot),
				&vsbond_metadata,
			)?;

			return Ok(());
		} else {
			return Err(Error::<T>::CurrencyIdNotExists.into());
		}
	}
}

/// Simple fee calculator that requires payment in a single fungible at a fixed rate.
///
/// The constant `FixedRate` type parameter should be the concrete fungible ID and the amount of it
/// required for one second of weight.
pub struct FixedRateOfAsset<T, FixedRate: Get<u128>, R: TakeRevenue> {
	weight: Weight,
	amount: u128,
	ed_ratio: FixedU128,
	multi_location: Option<MultiLocation>,
	_marker: PhantomData<(T, FixedRate, R)>,
}

impl<T: Config, FixedRate: Get<u128>, R: TakeRevenue> WeightTrader
	for FixedRateOfAsset<T, FixedRate, R>
where
	BalanceOf<T>: Into<u128>,
{
	fn new() -> Self {
		Self {
			weight: 0,
			amount: 0,
			ed_ratio: Default::default(),
			multi_location: None,
			_marker: PhantomData,
		}
	}

	fn buy_weight(&mut self, weight: Weight, payment: Assets) -> Result<Assets, XcmError> {
		log::trace!(target: "asset-registry::weight", "buy_weight weight: {:?}, payment: {:?}", weight, payment);

		// only support first fungible assets now.
		let asset_id = payment
			.fungible
			.iter()
			.next()
			.map_or(Err(XcmError::TooExpensive), |v| Ok(v.0))?;

		if let AssetId::Concrete(ref multi_location) = asset_id {
			log::debug!(target: "asset-registry::weight", "buy_weight multi_location: {:?}", multi_location);

			if let Some(currency_id) = Pallet::<T>::location_to_currency_ids(multi_location.clone())
			{
				if let Some(currency_metadatas) = Pallet::<T>::currency_metadatas(currency_id) {
					// The integration tests can ensure the ed is non-zero.
					let ed_ratio = FixedU128::saturating_from_rational(
						currency_metadatas.minimal_balance.into(),
						T::Currency::minimum_balance().into(),
					);
					// The WEIGHT_PER_SECOND is non-zero.
					let weight_ratio = FixedU128::saturating_from_rational(
						weight as u128,
						WEIGHT_PER_SECOND as u128,
					);
					let amount = ed_ratio
						.saturating_mul_int(weight_ratio.saturating_mul_int(FixedRate::get()));

					let required = MultiAsset { id: asset_id.clone(), fun: Fungible(amount) };

					log::trace!(
						target: "asset-registry::weight", "buy_weight payment: {:?}, required: {:?}, fixed_rate: {:?}, ed_ratio: {:?}, weight_ratio: {:?}",
						payment, required, FixedRate::get(), ed_ratio, weight_ratio
					);
					let unused = payment
						.clone()
						.checked_sub(required)
						.map_err(|_| XcmError::TooExpensive)?;
					self.weight = self.weight.saturating_add(weight);
					self.amount = self.amount.saturating_add(amount);
					self.ed_ratio = ed_ratio;
					self.multi_location = Some(multi_location.clone());
					return Ok(unused);
				}
			}
		}

		log::trace!(target: "asset-registry::weight", "no concrete fungible asset");
		Err(XcmError::TooExpensive)
	}

	fn refund_weight(&mut self, weight: Weight) -> Option<MultiAsset> {
		log::trace!(
			target: "asset-registry::weight", "refund_weight weight: {:?}, weight: {:?}, amount: {:?}, ed_ratio: {:?}, multi_location: {:?}",
			weight, self.weight, self.amount, self.ed_ratio, self.multi_location
		);
		let weight = weight.min(self.weight);
		let weight_ratio =
			FixedU128::saturating_from_rational(weight as u128, WEIGHT_PER_SECOND as u128);
		let amount = self
			.ed_ratio
			.saturating_mul_int(weight_ratio.saturating_mul_int(FixedRate::get()));

		self.weight = self.weight.saturating_sub(weight);
		self.amount = self.amount.saturating_sub(amount);

		log::trace!(target: "asset-registry::weight", "refund_weight amount: {:?}", amount);
		if amount > 0 && self.multi_location.is_some() {
			Some(
				(self.multi_location.as_ref().expect("checked is non-empty; qed").clone(), amount)
					.into(),
			)
		} else {
			None
		}
	}
}

impl<T, FixedRate: Get<u128>, R: TakeRevenue> Drop for FixedRateOfAsset<T, FixedRate, R> {
	fn drop(&mut self) {
		log::trace!(target: "asset-registry::weight", "take revenue, weight: {:?}, amount: {:?}, multi_location: {:?}", self.weight, self.amount, self.multi_location);
		if self.amount > 0 && self.multi_location.is_some() {
			R::take_revenue(
				(
					self.multi_location.as_ref().expect("checked is non-empty; qed").clone(),
					self.amount,
				)
					.into(),
			);
		}
	}
}

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

//! # Asset Registry Module
//!
//! Local and foreign assets management. The foreign assets can be updated without runtime upgrade.

#![cfg_attr(not(feature = "std"), no_std)]

pub use bifrost_primitives::{
	AssetIds, CurrencyId,
	CurrencyId::{Native, Token, Token2},
	CurrencyIdConversion, CurrencyIdMapping, CurrencyIdRegister, ForeignAssetId, LeasePeriod,
	ParaId, PoolId, TokenId, TokenInfo, TokenSymbol,
};
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::*,
	traits::{Currency, EnsureOrigin},
	weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight},
};
use frame_system::pallet_prelude::*;
use scale_info::{prelude::string::String, TypeInfo};
use sp_runtime::{
	traits::{One, UniqueSaturatedFrom},
	ArithmeticError, FixedPointNumber, FixedU128, RuntimeDebug,
};
use sp_std::{boxed::Box, vec::Vec};
use xcm::{
	opaque::lts::XcmContext,
	v3::MultiLocation,
	v4::{prelude::*, Asset, Location},
	VersionedLocation,
};
use xcm_builder::TakeRevenue;
use xcm_executor::{traits::WeightTrader, AssetsInHolding};

pub mod migrations;
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

#[derive(Clone, Eq, PartialEq, RuntimeDebug, Encode, Decode, TypeInfo)]
pub struct AssetMetadata<Balance> {
	pub name: Vec<u8>,
	pub symbol: Vec<u8>,
	pub decimals: u8,
	pub minimal_balance: Balance,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Currency type for withdraw and balance storage.
		type Currency: Currency<Self::AccountId>;

		/// Required origin for registering asset.
		type RegisterOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The given location could not be used (e.g. because it cannot be expressed in the
		/// desired version of XCM).
		BadLocation,
		/// Location existed
		LocationExisted,
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
		/// The asset registered.
		AssetRegistered { asset_id: AssetIds, metadata: AssetMetadata<BalanceOf<T>> },
		/// The asset updated.
		AssetUpdated { asset_id: AssetIds, metadata: AssetMetadata<BalanceOf<T>> },
		/// The CurrencyId registered.
		CurrencyIdRegistered { currency_id: CurrencyId, metadata: AssetMetadata<BalanceOf<T>> },
		/// Location Force set.
		LocationSet { currency_id: CurrencyId, location: Location, weight: Weight },
		/// The CurrencyId updated.
		CurrencyIdUpdated { currency_id: CurrencyId, metadata: AssetMetadata<BalanceOf<T>> },
	}

	/// Next available Foreign AssetId ID.
	///
	/// NextForeignAssetId: ForeignAssetId
	#[pallet::storage]
	pub type NextForeignAssetId<T: Config> = StorageValue<_, ForeignAssetId, ValueQuery>;

	/// Next available TokenId ID.
	///
	/// NextTokenId: TokenId
	#[pallet::storage]
	pub type NextTokenId<T: Config> = StorageValue<_, TokenId, ValueQuery>;

	/// The storages for Locations.
	///
	/// CurrencyIdToLocations: map CurrencyId => Option<Location>
	#[pallet::storage]
	pub type CurrencyIdToLocations<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, Location, OptionQuery>;

	/// The storages for CurrencyIds.
	///
	/// LocationToCurrencyIds: map Location => Option<CurrencyId>
	#[pallet::storage]
	pub type LocationToCurrencyIds<T: Config> =
		StorageMap<_, Twox64Concat, Location, CurrencyId, OptionQuery>;

	#[pallet::storage]
	pub type CurrencyIdToWeights<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, Weight, OptionQuery>;

	/// The storages for AssetMetadatas.
	///
	/// AssetMetadatas: map AssetIds => Option<AssetMetadata>
	#[pallet::storage]
	pub type AssetMetadatas<T: Config> =
		StorageMap<_, Twox64Concat, AssetIds, AssetMetadata<BalanceOf<T>>, OptionQuery>;

	/// The storages for AssetMetadata.
	///
	/// CurrencyMetadatas: map CurrencyId => Option<AssetMetadata>
	#[pallet::storage]
	pub type CurrencyMetadatas<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, AssetMetadata<BalanceOf<T>>, OptionQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub currency: Vec<(CurrencyId, BalanceOf<T>, Option<(String, String, u8)>)>,
		pub vcurrency: Vec<CurrencyId>,
		pub vsbond: Vec<(CurrencyId, u32, u32, u32)>,
		pub phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for (currency_id, metadata) in
				self.currency.iter().map(|(currency_id, minimal_balance, metadata)| {
					(
						currency_id,
						match &metadata {
							None => AssetMetadata {
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
							Some(metadata) => AssetMetadata {
								name: metadata.0.as_bytes().to_vec(),
								symbol: metadata.1.as_bytes().to_vec(),
								decimals: metadata.2,
								minimal_balance: *minimal_balance,
							},
						},
					)
				}) {
				if let CurrencyId::Token2(_token_id) = *currency_id {
					Pallet::<T>::get_next_token_id().expect("Token register");
				}
				Pallet::<T>::do_register_metadata(*currency_id, &metadata).expect("Token register");
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
					_ => (),
				}
			}
		}
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::without_storage_info]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::register_token_metadata())]
		pub fn register_token_metadata(
			origin: OriginFor<T>,
			metadata: Box<AssetMetadata<BalanceOf<T>>>,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			let token_id = Self::get_next_token_id()?;
			let currency_id = Token2(token_id);
			Self::do_register_metadata(currency_id, &metadata)?;

			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::register_vtoken_metadata())]
		pub fn register_vtoken_metadata(origin: OriginFor<T>, token_id: TokenId) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			if let Some(token_metadata) = CurrencyMetadatas::<T>::get(Token2(token_id)) {
				let vtoken_metadata = Self::convert_to_vtoken_metadata(token_metadata);
				Self::do_register_metadata(CurrencyId::VToken2(token_id), &vtoken_metadata)?;

				return Ok(());
			} else {
				return Err(Error::<T>::CurrencyIdNotExists)?;
			}
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::register_location())]
		pub fn register_location(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			location: Box<VersionedLocation>,
			weight: Weight,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			let location: Location =
				(*location).try_into().map_err(|()| Error::<T>::BadLocation)?;
			Self::do_register_location(currency_id, &location)?;
			Self::do_register_weight(currency_id, weight)?;

			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::force_set_location())]
		pub fn force_set_location(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			location: Box<VersionedLocation>,
			weight: Weight,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			let location: Location =
				(*location).try_into().map_err(|()| Error::<T>::BadLocation)?;

			let v4_location =
				Location::try_from(location.clone()).map_err(|_| Error::<T>::BadLocation)?;

			ensure!(
				CurrencyMetadatas::<T>::get(currency_id).is_some(),
				Error::<T>::CurrencyIdNotExists
			);

			LocationToCurrencyIds::<T>::insert(v4_location.clone(), currency_id);
			CurrencyIdToLocations::<T>::insert(currency_id, v4_location);
			CurrencyIdToWeights::<T>::insert(currency_id, weight);

			Pallet::<T>::deposit_event(Event::<T>::LocationSet { currency_id, location, weight });

			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::update_currency_metadata())]
		pub fn update_currency_metadata(
			origin: OriginFor<T>,
			currency_id: CurrencyId,
			asset_name: Option<Vec<u8>>,
			asset_symbol: Option<Vec<u8>>,
			asset_decimals: Option<u8>,
			asset_minimal_balance: Option<BalanceOf<T>>,
		) -> DispatchResult {
			T::RegisterOrigin::ensure_origin(origin)?;

			// Check if the currency metadata exists
			let mut metadata =
				CurrencyMetadatas::<T>::get(currency_id).ok_or(Error::<T>::CurrencyIdNotExists)?;

			// Update the metadata fields based on the provided options
			if let Some(name) = asset_name {
				metadata.name = name;
			}
			if let Some(symbol) = asset_symbol {
				metadata.symbol = symbol;
			}
			if let Some(decimals) = asset_decimals {
				metadata.decimals = decimals;
			}
			if let Some(minimal_balance) = asset_minimal_balance {
				metadata.minimal_balance = minimal_balance;
			}

			// Store the updated metadata
			CurrencyMetadatas::<T>::insert(currency_id, metadata.clone());
			Self::deposit_event(Event::<T>::CurrencyIdUpdated { currency_id, metadata });

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn get_next_token_id() -> Result<TokenId, DispatchError> {
		NextTokenId::<T>::try_mutate(|current| -> Result<TokenId, DispatchError> {
			let id = *current;
			*current = current.checked_add(One::one()).ok_or(ArithmeticError::Overflow)?;
			Ok(id)
		})
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

	pub fn do_register_location(currency_id: CurrencyId, location: &Location) -> DispatchResult {
		let v4_location =
			Location::try_from(location.clone()).map_err(|_| Error::<T>::BadLocation)?;

		ensure!(
			CurrencyMetadatas::<T>::get(currency_id).is_some(),
			Error::<T>::CurrencyIdNotExists
		);
		ensure!(
			LocationToCurrencyIds::<T>::get(v4_location.clone()).is_none(),
			Error::<T>::CurrencyIdExisted
		);
		ensure!(
			CurrencyIdToLocations::<T>::get(currency_id).is_none(),
			Error::<T>::LocationExisted
		);

		LocationToCurrencyIds::<T>::insert(v4_location.clone(), currency_id);
		CurrencyIdToLocations::<T>::insert(currency_id, v4_location);

		Ok(())
	}

	pub fn do_register_weight(currency_id: CurrencyId, weight: Weight) -> DispatchResult {
		ensure!(
			CurrencyMetadatas::<T>::get(currency_id).is_some(),
			Error::<T>::CurrencyIdNotExists
		);

		CurrencyIdToWeights::<T>::insert(currency_id, weight);

		Ok(())
	}
}

pub struct AssetIdMaps<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> CurrencyIdMapping<CurrencyId, MultiLocation, AssetMetadata<BalanceOf<T>>>
	for AssetIdMaps<T>
{
	fn get_asset_metadata(asset_ids: AssetIds) -> Option<AssetMetadata<BalanceOf<T>>> {
		AssetMetadatas::<T>::get(asset_ids)
	}

	fn get_currency_metadata(currency_id: CurrencyId) -> Option<AssetMetadata<BalanceOf<T>>> {
		CurrencyMetadatas::<T>::get(currency_id)
	}

	fn get_all_currency() -> Vec<CurrencyId> {
		CurrencyMetadatas::<T>::iter_keys().collect()
	}

	fn get_location(currency_id: CurrencyId) -> Option<Location> {
		CurrencyIdToLocations::<T>::get(currency_id).map(|location| location.try_into().ok())?
	}

	fn get_currency_id(multi_location: Location) -> Option<CurrencyId> {
		let v4_location = Location::try_from(multi_location).ok()?;
		LocationToCurrencyIds::<T>::get(v4_location)
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

	fn check_token2_registered(token_id: TokenId) -> bool {
		CurrencyMetadatas::<T>::get(CurrencyId::Token2(token_id)).is_some()
	}

	fn check_vtoken2_registered(token_id: TokenId) -> bool {
		CurrencyMetadatas::<T>::get(CurrencyId::VToken2(token_id)).is_some()
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

	fn register_blp_metadata(pool_id: PoolId, decimals: u8) -> DispatchResult {
		let name = scale_info::prelude::format!("Bifrost Stable Pool Token {}", pool_id)
			.as_bytes()
			.to_vec();
		let symbol = scale_info::prelude::format!("BLP{}", pool_id).as_bytes().to_vec();
		Pallet::<T>::do_register_metadata(
			CurrencyId::BLP(pool_id),
			&AssetMetadata {
				name,
				symbol,
				decimals,
				minimal_balance: BalanceOf::<T>::unique_saturated_from(1_000_000u128),
			},
		)
	}
}

/// Simple fee calculator that requires payment in a single fungible at a fixed rate.
///
/// The constant `FixedRate` type parameter should be the concrete fungible ID and the amount of it
/// required for one second of weight.
pub struct FixedRateOfAsset<T, FixedRate: Get<u128>, R: TakeRevenue> {
	weight: u64,
	amount: u128,
	ed_ratio: FixedU128,
	location: Option<Location>,
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
			location: None,
			_marker: PhantomData,
		}
	}

	fn buy_weight(
		&mut self,
		weight: Weight,
		payment: AssetsInHolding,
		_context: &XcmContext,
	) -> Result<AssetsInHolding, XcmError> {
		log::trace!(target: "asset-registry::weight", "buy_weight weight: {:?}, payment: {:?}", weight, payment);

		// only support first fungible assets now.
		let asset_id = payment
			.fungible
			.iter()
			.next()
			.map_or(Err(XcmError::TooExpensive), |v| Ok(v.0))?;

		let AssetId(ref location) = asset_id.clone();
		log::debug!(target: "asset-registry::weight", "buy_weight location: {:?}", location);

		let v4_location =
			Location::try_from(location.clone()).map_err(|_| XcmError::InvalidLocation)?;

		if let Some(currency_id) = LocationToCurrencyIds::<T>::get(v4_location) {
			if let Some(currency_metadatas) = CurrencyMetadatas::<T>::get(currency_id) {
				// The integration tests can ensure the ed is non-zero.
				let ed_ratio = FixedU128::saturating_from_rational(
					currency_metadatas.minimal_balance.into(),
					T::Currency::minimum_balance().into(),
				);
				// The WEIGHT_REF_TIME_PER_SECOND is non-zero.
				let weight_ratio = FixedU128::saturating_from_rational(
					weight.ref_time(),
					WEIGHT_REF_TIME_PER_SECOND,
				);
				let amount =
					ed_ratio.saturating_mul_int(weight_ratio.saturating_mul_int(FixedRate::get()));

				let required = Asset { id: asset_id.clone(), fun: Fungible(amount) };

				log::trace!(
					target: "asset-registry::weight", "buy_weight payment: {:?}, required: {:?}, fixed_rate: {:?}, ed_ratio: {:?}, weight_ratio: {:?}",
					payment, required, FixedRate::get(), ed_ratio, weight_ratio
				);
				let unused =
					payment.clone().checked_sub(required).map_err(|_| XcmError::TooExpensive)?;
				self.weight = self.weight.saturating_add(weight.ref_time());
				self.amount = self.amount.saturating_add(amount);
				self.ed_ratio = ed_ratio;
				self.location = Some(location.clone());
				return Ok(unused);
			}
		};

		log::trace!(target: "asset-registry::weight", "no concrete fungible asset");
		Err(XcmError::TooExpensive)
	}

	fn refund_weight(&mut self, weight: Weight, _context: &XcmContext) -> Option<Asset> {
		log::trace!(
			target: "asset-registry::weight", "refund_weight weight: {:?}, weight: {:?}, amount: {:?}, ed_ratio: {:?}, location: {:?}",
			weight, self.weight, self.amount, self.ed_ratio, self.location
		);
		let weight = weight.min(Weight::from_parts(self.weight, 0));
		let weight_ratio =
			FixedU128::saturating_from_rational(weight.ref_time(), WEIGHT_REF_TIME_PER_SECOND);
		let amount = self
			.ed_ratio
			.saturating_mul_int(weight_ratio.saturating_mul_int(FixedRate::get()));

		self.weight = self.weight.saturating_sub(weight.ref_time());
		self.amount = self.amount.saturating_sub(amount);

		log::trace!(target: "asset-registry::weight", "refund_weight amount: {:?}", amount);
		if amount > 0 && self.location.is_some() {
			Some(Asset {
				fun: Fungible(amount),
				id: AssetId(
					self.location.clone().expect("checked is non-empty; qed").try_into().unwrap(),
				),
			})
		} else {
			None
		}
	}
}

impl<T, FixedRate: Get<u128>, R: TakeRevenue> Drop for FixedRateOfAsset<T, FixedRate, R> {
	fn drop(&mut self) {
		log::trace!(target: "asset-registry::weight", "take revenue, weight: {:?}, amount: {:?}, location: {:?}", self.weight, self.amount, self.location);
		if self.amount > 0 && self.location.is_some() {
			R::take_revenue(Asset {
				fun: Fungible(self.amount),
				id: AssetId(
					self.location.clone().expect("checked is non-empty; qed").try_into().unwrap(),
				),
			});
		}
	}
}

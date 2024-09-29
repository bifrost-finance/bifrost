// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Prices pallet
//!
//! ## Overview
//!
//! This pallet provides the price from Oracle Module by implementing the
//! `OraclePriceProvider` trait. In case of emergency, the price can be set directly
//! by Oracle Collective.

#![cfg_attr(not(feature = "std"), no_std)]

use bifrost_asset_registry::AssetMetadata;
use bifrost_primitives::{
	Balance, CurrencyId, CurrencyIdMapping, OraclePriceProvider, Price, PriceDetail,
	TimeStampedPrice, TokenInfo,
};
use frame_support::{dispatch::DispatchClass, pallet_prelude::*, transactional};
use frame_system::pallet_prelude::*;
use log;
use orml_oracle::{DataFeeder, DataProvider, DataProviderExtended};
pub use pallet::*;
use pallet_traits::*;
use sp_runtime::{traits::CheckedDiv, FixedU128};
use sp_std::vec::Vec;
use xcm::v3::MultiLocation;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	use frame_support::traits::fungibles::{Inspect, Mutate};
	use weights::WeightInfo;

	pub(crate) type BalanceOf<T> =
		<<T as Config>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The data source, such as Oracle.
		type Source: DataProvider<CurrencyId, TimeStampedPrice>
			+ DataProviderExtended<CurrencyId, TimeStampedPrice>
			+ DataFeeder<CurrencyId, TimeStampedPrice, Self::AccountId>;

		/// The origin which may set prices feed to system.
		type FeederOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The origin which can update prices link.
		type UpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Currency type for deposit/withdraw assets to/from amm route
		/// module
		type Assets: Inspect<Self::AccountId, AssetId = CurrencyId, Balance = Balance>
			+ Mutate<Self::AccountId, AssetId = CurrencyId, Balance = Balance>;

		/// Relay currency
		#[pallet::constant]
		type RelayCurrency: Get<CurrencyId>;

		/// Convert Location to `T::CurrencyId`.
		type CurrencyIdConvert: CurrencyIdMapping<
			CurrencyId,
			MultiLocation,
			AssetMetadata<BalanceOf<Self>>,
		>;

		/// Weight information
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Set emergency price. \[asset_id, price_detail\]
		SetPrice(CurrencyId, Price),
		/// Reset emergency price. \[asset_id\]
		ResetPrice(CurrencyId),
	}

	/// Mapping from currency id to it's emergency price
	#[pallet::storage]
	pub type EmergencyPrice<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, Price, OptionQuery>;

	/// Mapping from foreign vault token to our's vault token
	#[pallet::storage]
	pub type ForeignToNativeAsset<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyId, CurrencyId, OptionQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub emergency_price: Vec<(CurrencyId, Price)>,
		pub foreign_to_native_asset: Vec<(CurrencyId, CurrencyId)>,
		pub phantom: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for (asset_id, price) in self.emergency_price.iter() {
				EmergencyPrice::<T>::insert(asset_id, price);
			}
			for (foreign_asset_id, native) in self.foreign_to_native_asset.iter() {
				ForeignToNativeAsset::<T>::insert(foreign_asset_id, native);
			}
		}
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set emergency price
		#[pallet::call_index(0)]
		#[pallet::weight((<T as Config>::WeightInfo::set_price(), DispatchClass::Operational))]
		#[transactional]
		pub fn set_price(
			origin: OriginFor<T>,
			asset_id: CurrencyId,
			price: Price,
		) -> DispatchResultWithPostInfo {
			T::FeederOrigin::ensure_origin(origin)?;
			<Pallet<T> as EmergencyOraclePriceProvider<CurrencyId, Price>>::set_emergency_price(
				asset_id, price,
			);
			Ok(().into())
		}

		/// Reset emergency price
		#[pallet::call_index(1)]
		#[pallet::weight((<T as Config>::WeightInfo::reset_price(), DispatchClass::Operational))]
		#[transactional]
		pub fn reset_price(
			origin: OriginFor<T>,
			asset_id: CurrencyId,
		) -> DispatchResultWithPostInfo {
			T::FeederOrigin::ensure_origin(origin)?;
			<Pallet<T> as EmergencyOraclePriceProvider<CurrencyId, Price>>::reset_emergency_price(
				asset_id,
			);
			Ok(().into())
		}

		/// Set foreign vault token mapping
		#[pallet::call_index(2)]
		#[pallet::weight((<T as Config>::WeightInfo::set_foreign_asset(), DispatchClass::Operational))]
		#[transactional]
		pub fn set_foreign_asset(
			origin: OriginFor<T>,
			foreign_asset_id: CurrencyId,
			asset_id: CurrencyId,
		) -> DispatchResultWithPostInfo {
			T::UpdateOrigin::ensure_origin(origin)?;
			ForeignToNativeAsset::<T>::insert(foreign_asset_id, asset_id);
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	// get emergency price, the timestamp is zero
	fn get_emergency_price(asset_id: &CurrencyId) -> Option<PriceDetail> {
		EmergencyPrice::<T>::get(asset_id).and_then(|p| {
			let mantissa = Self::get_asset_mantissa(asset_id)?;
			log::trace!(
				target: "prices::get_emergency_price",
				"asset_id: {:?}, mantissa: {:?}",
				asset_id,
				mantissa
			);
			p.checked_div(&FixedU128::from_inner(mantissa)).map(|price| (price, 0))
		})
	}

	fn get_storage_price(asset_id: &CurrencyId) -> Option<Price> {
		EmergencyPrice::<T>::get(asset_id)
			.or_else(|| T::Source::get(asset_id).and_then(|price| Some(price.value)))
	}

	fn get_asset_mantissa(asset_id: &CurrencyId) -> Option<u128> {
		10u128.checked_pow(
			asset_id
				.decimals()
				.unwrap_or(
					T::CurrencyIdConvert::get_currency_metadata(*asset_id)
						.map_or(12, |metatata| metatata.decimals.into()),
				)
				.into(),
		)
	}

	fn get_special_asset_price(
		_asset_id: CurrencyId,
		_base_price: TimeStampedPrice,
	) -> Option<TimeStampedPrice> {
		None
	}

	fn normalize_detail_price(price: TimeStampedPrice, mantissa: u128) -> Option<PriceDetail> {
		price
			.value
			.checked_div(&FixedU128::from_inner(mantissa))
			.map(|value| (value, price.timestamp))
	}
}

impl<T: Config> OraclePriceProvider for Pallet<T> {
	/// Returns the uniform format price and timestamp by asset id.
	/// Formula: `price = oracle_price * 10.pow(18 - asset_decimal)`
	/// We use `oracle_price.checked_div(&FixedU128::from_inner(mantissa))` represent that.
	/// This particular price makes it easy to calculate the asset value in other pallets,
	/// because we don't have to consider decimal for each asset.
	///
	/// Timestamp is zero means the price is emergency price
	fn get_price(asset_id: &CurrencyId) -> Option<PriceDetail> {
		// if emergency price exists, return it
		Self::get_emergency_price(asset_id).or_else(|| {
			let mantissa = Self::get_asset_mantissa(asset_id)?;
			T::Source::get(&T::RelayCurrency::get())
				.and_then(|base_price| Self::get_special_asset_price(*asset_id, base_price))
				.or_else(|| T::Source::get(asset_id))
				.and_then(|price| Self::normalize_detail_price(price, mantissa))
		})
	}

	/// Get the amount of currencies according to the input price data.
	/// Parameters:
	/// - `currency_in`: The currency to be converted.
	/// - `amount_in`: The amount of currency to be converted.
	/// - `price_in`: The price of currency_in.
	/// - `currency_out`: The currency to be converted to.
	/// - `price_out`: The price of currency_out.
	/// Returns:
	/// - The amount of currency_out.
	fn get_amount_by_prices(
		currency_in: &CurrencyId,
		amount_in: Balance,
		price_in: Price,
		currency_out: &CurrencyId,
		price_out: Price,
	) -> Option<Balance> {
		let currency_in_mantissa = Self::get_asset_mantissa(currency_in)?;
		let currency_out_mantissa = Self::get_asset_mantissa(currency_out)?;
		let total_value = price_in
			.mul(FixedU128::from_inner(amount_in))
			.div(FixedU128::from_inner(currency_in_mantissa));
		let amount_out =
			total_value.mul(FixedU128::from_inner(currency_out_mantissa)).div(price_out);
		Some(amount_out.into_inner())
	}

	/// Get the amount of currencies according to the oracle price data.
	/// Parameters:
	/// - `currency_in`: The currency to be converted.
	/// - `amount_in`: The amount of currency to be converted.
	/// - `currency_out`: The currency to be converted to.
	/// Returns:
	/// - The amount of currency_out.
	/// - The price of currency_in.
	/// - The price of currency_out.
	fn get_oracle_amount_by_currency_and_amount_in(
		currency_in: &CurrencyId,
		amount_in: Balance,
		currency_out: &CurrencyId,
	) -> Option<(Balance, Price, Price)> {
		let price_in = Self::get_storage_price(currency_in)?;
		if currency_in == currency_out {
			Some((amount_in, price_in, price_in))
		} else {
			let price_out = Self::get_storage_price(currency_out)?;
			Self::get_amount_by_prices(currency_in, amount_in, price_in, currency_out, price_out)
				.map(|amount_out| (amount_out, price_in, price_out))
		}
	}
}

impl<T: Config> EmergencyOraclePriceProvider<CurrencyId, Price> for Pallet<T> {
	/// Set emergency price
	fn set_emergency_price(asset_id: CurrencyId, price: Price) {
		// set price direct
		EmergencyPrice::<T>::insert(asset_id, price);
		<Pallet<T>>::deposit_event(Event::SetPrice(asset_id, price));
	}

	/// Reset emergency price
	fn reset_emergency_price(asset_id: CurrencyId) {
		EmergencyPrice::<T>::remove(asset_id);
		<Pallet<T>>::deposit_event(Event::ResetPrice(asset_id));
	}
}

impl<T: Config> DataProviderExtended<CurrencyId, TimeStampedPrice> for Pallet<T> {
	fn get_no_op(asset_id: &CurrencyId) -> Option<TimeStampedPrice> {
		let _mantissa = Self::get_asset_mantissa(asset_id)?;
		T::Source::get_no_op(&T::RelayCurrency::get())
			.and_then(|base_price| Self::get_special_asset_price(*asset_id, base_price))
			.or_else(|| T::Source::get_no_op(asset_id))
	}

	fn get_all_values() -> Vec<(CurrencyId, Option<TimeStampedPrice>)> {
		T::Source::get_all_values()
	}
}

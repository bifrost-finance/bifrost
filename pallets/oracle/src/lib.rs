//! # Oracle Pallet
//! Based on the [specification](https://spec.interlay.io/spec/oracle.html).

#![deny(warnings)]
#![cfg_attr(test, feature(proc_macro_hygiene))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod default_weights;
pub use default_weights::WeightInfo;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

pub mod types;

#[cfg(test)]
extern crate mocktopus;

#[cfg(test)]
use mocktopus::macros::mockable;

use crate::types::{UnsignedFixedPoint, Version};
use codec::{Decode, Encode, EncodeLike, MaxEncodedLen};
// use currency::Amount;
use frame_support::{
	dispatch::{DispatchError, DispatchResult},
	ensure,
	traits::Get,
	transactional,
	weights::Weight,
	BoundedVec,
};
use frame_system::{ensure_root, ensure_signed};
// pub use orml_traits::MultiCurrency;
use pallet_traits::*;
use scale_info::TypeInfo;
use sp_runtime::{traits::*, FixedPointNumber};
use sp_std::{convert::TryInto, vec::Vec};

pub use pallet::*;
pub use primitives::{Balance, CurrencyId, Price, PriceDetail};
// pub use traits::OnExchangeRateChange;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, Clone, Eq, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub enum OracleKey {
	ExchangeRate(CurrencyId),
	FeeEstimation,
}

pub type NameOf<T> = BoundedVec<u8, <T as pallet::Config>::MaxNameLength>;

#[derive(Encode, Decode, Eq, PartialEq, Clone, Copy, Ord, PartialOrd, TypeInfo, MaxEncodedLen)]
pub struct TimestampedValue<Value, Moment> {
	pub value: Value,
	pub timestamp: Moment,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	/// ## Configuration
	/// The pallet's configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config
	// + security::Config
	// + currency::Config<CurrencyId = CurrencyId>
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Hook for aggregate changes.
		type OnExchangeRateChange: OnExchangeRateChange<CurrencyId>;

		/// Weight information for the extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// The maximum length of an oracle name.
		#[pallet::constant]
		type MaxNameLength: Get<u32>;

		// type MultiCurrency: MultiCurrency<
		// 	Self::AccountId,
		// 	CurrencyId = CurrencyId,
		// 	// Balance = BalanceOf<Self>,
		// 	// Balance = Self::Balance,
		// >;

		// #[pallet::constant]
		// type UnsignedFixedPoint: Get<Price>;

		type UnsignedFixedPoint: FixedPointNumber<Inner = Balance>
			// + TruncateFixedPointToInt
			+ Encode
			+ EncodeLike
			+ Decode
			+ MaybeSerializeDeserialize
			+ TypeInfo
			+ From<Balance>
			+ Into<Price>
			+ MaxEncodedLen;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when exchange rate is set
		FeedValues {
			oracle_id: T::AccountId,
			values: Vec<(OracleKey, T::UnsignedFixedPoint)>,
		},
		AggregateUpdated {
			values: Vec<(OracleKey, Option<T::UnsignedFixedPoint>)>,
		},
		OracleAdded {
			oracle_id: T::AccountId,
			name: NameOf<T>,
		},
		OracleRemoved {
			oracle_id: T::AccountId,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Not authorized to set exchange rate
		InvalidOracleSource,
		/// Exchange rate not specified or has expired
		MissingExchangeRate,
		/// Unable to convert value
		TryIntoIntError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let iterations = Self::begin_block(n);
			<T as Config>::WeightInfo::on_initialize(iterations)
		}
	}

	/// Current medianized value for the given key
	#[pallet::storage]
	pub type Aggregate<T: Config> =
		StorageMap<_, Blake2_128Concat, OracleKey, UnsignedFixedPoint<T>>;

	#[pallet::storage]
	pub type RawValues<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		OracleKey,
		Blake2_128Concat,
		T::AccountId,
		TimestampedValue<UnsignedFixedPoint<T>, T::Moment>,
	>;

	#[pallet::storage]
	/// if a key is present, it means the values have been updated
	pub type RawValuesUpdated<T: Config> = StorageMap<_, Blake2_128Concat, OracleKey, bool>;

	/// Time until which the aggregate is valid
	#[pallet::storage]
	pub type ValidUntil<T: Config> = StorageMap<_, Blake2_128Concat, OracleKey, T::Moment>;

	/// Maximum delay (milliseconds) for a reported value to be used
	#[pallet::storage]
	#[pallet::getter(fn max_delay)]
	pub type MaxDelay<T: Config> = StorageValue<_, T::Moment, ValueQuery>;

	// Oracles allowed to set the exchange rate, maps to the name
	#[pallet::storage]
	#[pallet::getter(fn authorized_oracles)]
	pub type AuthorizedOracles<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, NameOf<T>, ValueQuery>;

	#[pallet::type_value]
	pub(super) fn DefaultForStorageVersion() -> Version {
		Version::V0
	}

	/// Build storage at V1 (requires default 0).
	#[pallet::storage]
	#[pallet::getter(fn storage_version)]
	pub(super) type StorageVersion<T: Config> =
		StorageValue<_, Version, ValueQuery, DefaultForStorageVersion>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub max_delay: u32,
		pub authorized_oracles: Vec<(T::AccountId, NameOf<T>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { max_delay: Default::default(), authorized_oracles: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// T::Moment doesn't implement serialize so we use
			// From<u32> as bound by AtLeast32Bit
			MaxDelay::<T>::put(T::Moment::from(self.max_delay));

			for (ref who, name) in self.authorized_oracles.iter() {
				AuthorizedOracles::<T>::insert(who, name);
			}
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	// The pallet's dispatchable functions.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Feeds data from the oracles, e.g., the exchange rates. This function
		/// is intended to be API-compatible with orml-oracle.
		///
		/// # Arguments
		///
		/// * `values` - a vector of (key, value) pairs to submit
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::feed_values(values.len() as u32))]
		pub fn feed_values(
			origin: OriginFor<T>,
			values: Vec<(OracleKey, T::UnsignedFixedPoint)>,
		) -> DispatchResultWithPostInfo {
			let signer = ensure_signed(origin)?;

			// fail if the signer is not an authorized oracle
			ensure!(Self::is_authorized(&signer), Error::<T>::InvalidOracleSource);

			Self::_feed_values(signer, values);
			Ok(Pays::No.into())
		}

		/// Adds an authorized oracle account (only executable by the Root account)
		///
		/// # Arguments
		/// * `account_id` - the account Id of the oracle
		/// * `name` - a descriptive name for the oracle
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::insert_authorized_oracle())]
		#[transactional]
		pub fn insert_authorized_oracle(
			origin: OriginFor<T>,
			account_id: T::AccountId,
			name: NameOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::insert_oracle(account_id.clone(), name.clone());
			Self::deposit_event(Event::OracleAdded { oracle_id: account_id, name });
			Ok(())
		}

		/// Removes an authorized oracle account (only executable by the Root account)
		///
		/// # Arguments
		/// * `account_id` - the account Id of the oracle
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_authorized_oracle())]
		#[transactional]
		pub fn remove_authorized_oracle(
			origin: OriginFor<T>,
			account_id: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			<AuthorizedOracles<T>>::remove(account_id.clone());
			Self::deposit_event(Event::OracleRemoved { oracle_id: account_id });
			Ok(())
		}
	}
}

#[allow(clippy::forget_copy, clippy::forget_ref)]
#[cfg_attr(test, mockable)]
impl<T: Config> Pallet<T> {
	// public only for testing purposes
	pub fn begin_block(_height: T::BlockNumber) -> u32 {
		// read to a temporary value, because we can't alter the map while we iterate over it
		let raw_values_updated: Vec<_> = RawValuesUpdated::<T>::iter().collect();

		let current_time = Self::get_current_time();

		let mut updated_items = Vec::new();
		for (key, is_updated) in raw_values_updated.iter() {
			if *is_updated || Self::is_outdated(key, current_time) {
				let new_value = Self::update_aggregate(key);
				updated_items.push((key.clone(), new_value));
			}
		}

		if !updated_items.is_empty() {
			Self::deposit_event(Event::<T>::AggregateUpdated { values: updated_items });
		}

		raw_values_updated.len().saturated_into()
	}

	// public only for testing purposes
	pub fn _feed_values(oracle: T::AccountId, values: Vec<(OracleKey, T::UnsignedFixedPoint)>) {
		for (key, value) in values.iter() {
			let timestamped =
				TimestampedValue { timestamp: Self::get_current_time(), value: *value };
			RawValues::<T>::insert(key, &oracle, timestamped);
			RawValuesUpdated::<T>::insert(key, true);
		}

		Self::deposit_event(Event::<T>::FeedValues { oracle_id: oracle, values });
	}

	/// Public getters

	/// Get the exchange rate in planck per satoshi
	pub fn get_price(key: OracleKey) -> Result<UnsignedFixedPoint<T>, DispatchError> {
		Aggregate::<T>::get(key).ok_or(Error::<T>::MissingExchangeRate.into())
	}

	// pub fn wrapped_to_collateral(
	// 	amount: BalanceOf<T>,
	// 	currency_id: CurrencyId,
	// ) -> Result<BalanceOf<T>, DispatchError> {
	// 	let amount = Amount::<T>::new(amount, currency_id);

	// 	let rate = Self::get_price(OracleKey::ExchangeRate(currency_id))?;

	// 	amount.checked_mul(&rate).map(|x| x.amount())
	// }

	// pub fn collateral_to_wrapped(
	// 	amount: BalanceOf<T>,
	// 	currency_id: CurrencyId,
	// ) -> Result<BalanceOf<T>, DispatchError> {
	// 	let rate = Self::get_price(OracleKey::ExchangeRate(currency_id))?;
	// 	if amount.is_zero() {
	// 		return Ok(Zero::zero());
	// 	}
	// 	let amount = Amount::<T>::new(amount, currency_id);

	// 	amount.checked_div(&rate).map(|x| x.amount())
	// }

	fn update_aggregate(key: &OracleKey) -> Option<T::UnsignedFixedPoint> {
		RawValuesUpdated::<T>::insert(key, false);
		let mut raw_values: Vec<_> =
			RawValues::<T>::iter_prefix(key).map(|(_, value)| value).collect();
		let min_timestamp = Self::get_current_time().saturating_sub(Self::get_max_delay());
		raw_values.retain(|value| value.timestamp >= min_timestamp);
		let ret = if raw_values.len() == 0 {
			Aggregate::<T>::remove(key);
			ValidUntil::<T>::remove(key);
			None
		} else {
			let valid_until = raw_values
				.iter()
				.map(|x| x.timestamp)
				.min()
				.map(|timestamp| timestamp + Self::get_max_delay())
				.unwrap_or_default(); // Unwrap will never fail, but if somehow it did, we retry next block

			let value = Self::median(raw_values.iter().map(|x| x.value).collect())?;

			Aggregate::<T>::insert(key, value);
			ValidUntil::<T>::insert(key, valid_until);

			Some(value)
		};

		if let OracleKey::ExchangeRate(currency_id) = key {
			T::OnExchangeRateChange::on_exchange_rate_change(currency_id);
		}

		ret
	}

	fn median(mut raw_values: Vec<UnsignedFixedPoint<T>>) -> Option<UnsignedFixedPoint<T>> {
		let mid_index = raw_values.len().checked_div(2)?;
		raw_values.sort_unstable();
		match raw_values.len() {
			0 => None,
			len if len.checked_rem(2)? == 0 => {
				// even number - get avg of 2 values
				let value_1 = raw_values.get(mid_index.checked_sub(1)?)?;
				let value_2 = raw_values.get(mid_index)?;
				let value = value_1
					.checked_add(&value_2)?
					.checked_div(&UnsignedFixedPoint::<T>::from(2u32.into()))?;
				Some(value)
			},
			_ => Some(*raw_values.get(mid_index)?),
		}
	}

	/// Private getters and setters

	fn is_outdated(key: &OracleKey, current_time: T::Moment) -> bool {
		let valid_until = ValidUntil::<T>::get(key);
		matches!(valid_until, Some(t) if current_time > t)
	}

	fn get_max_delay() -> T::Moment {
		<MaxDelay<T>>::get()
	}

	/// Set the current exchange rate. ONLY FOR TESTING.
	///
	/// # Arguments
	///
	/// * `exchange_rate` - i.e. planck per satoshi
	pub fn _set_exchange_rate(
		currency_id: CurrencyId,
		exchange_rate: UnsignedFixedPoint<T>,
	) -> DispatchResult {
		Aggregate::<T>::insert(&OracleKey::ExchangeRate(currency_id), exchange_rate);
		T::OnExchangeRateChange::on_exchange_rate_change(&currency_id);

		Ok(())
	}

	#[cfg(feature = "testing-utils")]
	pub fn expire_price(currency_id: CurrencyId) {
		Aggregate::<T>::remove(&OracleKey::ExchangeRate(currency_id.clone()));
		T::OnExchangeRateChange::on_exchange_rate_change(&currency_id);
	}

	#[cfg(feature = "testing-utils")]
	pub fn expire_all() {
		for (key, _old_rate) in Aggregate::<T>::drain() {
			if let OracleKey::ExchangeRate(currency_id) = key {
				T::OnExchangeRateChange::on_exchange_rate_change(&currency_id);
			}
		}
	}

	/// Returns the current timestamp
	fn get_current_time() -> T::Moment {
		<pallet_timestamp::Pallet<T>>::get()
	}

	/// Add a new authorized oracle
	fn insert_oracle(oracle: T::AccountId, name: NameOf<T>) {
		<AuthorizedOracles<T>>::insert(oracle, name)
	}

	/// True if oracle is authorized
	fn is_authorized(oracle: &T::AccountId) -> bool {
		<AuthorizedOracles<T>>::contains_key(oracle)
	}
}

impl<T: Config> PriceFeeder for Pallet<T> {
	fn get_price(asset_id: &CurrencyId) -> Option<PriceDetail> {
		Self::get_price(OracleKey::ExchangeRate(*asset_id)).map(|a| (a.into(), 0)).ok()
	}
}

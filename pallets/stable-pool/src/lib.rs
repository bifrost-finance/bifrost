#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use frame_support::{
	pallet_prelude::*,
	traits::{tokens::currency, Currency},
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, TimeUnit, VtokenMintingOperator};
use orml_traits::MultiCurrency;

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
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type WeightInfo: WeightInfo;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type StableAsset: nutsfinance_stable_asset::StableAsset;

		type VtokenMinting: VtokenMintingOperator<
			CurrencyId,
			BalanceOf<Self>,
			AccountIdOf<Self>,
			TimeUnit,
		>;
	}

	#[pallet::storage]
	#[pallet::getter(fn something)]
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::storage]
	#[pallet::getter(fn token_rate_caches)]
	pub type TokenRateCaches<T: Config> = StorageMap<_, Twox64Concat, CurrencyId, BalanceOf<T>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		SomethingStored { something: u32, who: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::do_something())]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			<Something<T>>::put(something);

			Self::deposit_event(Event::SomethingStored { something, who });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn get_token_rates(currency: CurrencyId) -> BalanceOf<T> {
		if let Some(rate) = Self::token_rate_caches(currency) {
			// getTokenRateCache
			rate
		} else {
			// getCurrentRate
			BalanceOf::<T>::default()
		}
		// Ok(())
	}

	pub fn get_scaling_factors(something: u32) -> DispatchResult {
		Ok(())
	}

	pub fn upscale(amount: BalanceOf<T>, currency: CurrencyId) -> BalanceOf<T> {
		if let Some(rate) = Self::token_rate_caches(currency) {
			amount * rate
		} else {
			amount
		}
	}
}

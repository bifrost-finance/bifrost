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
	sp_runtime::traits::{UniqueSaturatedFrom, UniqueSaturatedInto, Zero},
	traits::{tokens::currency, Currency},
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, CurrencyIdConversion, TimeUnit, VtokenMintingOperator};
use nutsfinance_stable_asset::{
	PoolTokenIndex, StableAsset, StableAssetPoolId, StableAssetPoolInfo,
};
use orml_traits::MultiCurrency;
use sp_core::U256;

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

		type StableAsset: nutsfinance_stable_asset::StableAsset<
			AssetId = CurrencyId,
			Balance = BalanceOf<Self>,
		>;

		type VtokenMinting: VtokenMintingOperator<
			CurrencyId,
			BalanceOf<Self>,
			AccountIdOf<Self>,
			TimeUnit,
		>;

		type CurrencyIdConversion: CurrencyIdConversion<CurrencyId>;
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
		NotSupportTokenType,
		PoolNotExist,
		NotNullable,
		CantBeZero,
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

	fn on_swap(
		pool_id: StableAssetPoolId,
		currency_id_in: PoolTokenIndex,
		currency_id_out: PoolTokenIndex,
		amount: BalanceOf<T>,
	) -> DispatchResult {
		let pool_info = T::StableAsset::pool(pool_id).ok_or(Error::<T>::PoolNotExist)?;

		let upscale_out = Self::upscale(
			amount,
			*pool_info.assets.get(currency_id_in as usize).ok_or(Error::<T>::NotNullable)?,
		)?;
		let amount_out = T::StableAsset::get_swap_output_amount(
			pool_id,
			currency_id_in,
			currency_id_out,
			amount,
		);
		log::debug!("amount_out:{:?}", amount_out);

		// amount_out.ok_or(Error::<T>::CantBeZero)?;
		let downscale_out = Self::downscale(
			amount_out.ok_or(Error::<T>::CantBeZero)?.dy, // TODO
			*pool_info.assets.get(currency_id_out as usize).ok_or(Error::<T>::NotNullable)?,
		)?;
		log::debug!("downscale_out:{:?}", downscale_out);
		if downscale_out.is_zero() {
			// TODO
		}
		Ok(())
	}

	pub fn upscale(
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<BalanceOf<T>, DispatchError> {
		log::debug!("upscale currency_id:{:?}", currency_id);
		match currency_id {
			CurrencyId::VToken(_) | CurrencyId::VToken2(_) =>
				Self::upscale_vtoken(amount, currency_id),
			_ => Ok(amount),
		}
	}
	pub fn downscale(
		amount: BalanceOf<T>,
		currency_id: CurrencyId,
	) -> Result<BalanceOf<T>, DispatchError> {
		log::debug!("downscale currency_id:{:?}", currency_id);
		match currency_id {
			CurrencyId::VToken(_) | CurrencyId::VToken2(_) =>
				Self::downscale_vtoken(amount, currency_id),
			// CurrencyId::Token2(_) => Self::downscale_token(amount, currency_id),
			_ => Ok(amount),
		}
	}

	pub fn upscale_vtoken(
		amount: BalanceOf<T>,
		vcurrency_id: CurrencyId,
	) -> Result<BalanceOf<T>, DispatchError> {
		// if let Some(rate) = Self::token_rate_caches(currency) {
		// 	amount * rate
		// } else {
		// 	amount
		// }
		let currency_id = T::CurrencyIdConversion::convert_to_token(vcurrency_id)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;
		let vtoken_issuance = T::MultiCurrency::total_issuance(vcurrency_id);
		let token_pool = T::VtokenMinting::get_token_pool(currency_id);
		log::debug!("vtoken_issuance:{:?}token_pool{:?}", vtoken_issuance, token_pool);

		Ok(Self::calculate_scaling(amount, token_pool, vtoken_issuance))

		// let amount: u128 = amount.unique_saturated_into();
		// let vtoken_issuance: u128 = vtoken_issuance.unique_saturated_into();
		// let token_pool: u128 = token_pool.unique_saturated_into();
		// let can_get_vtoken = U256::from(amount)
		// 	.checked_mul(U256::from(token_pool))
		// 	.and_then(|n| n.checked_div(U256::from(vtoken_issuance)))
		// 	.and_then(|n| TryInto::<u128>::try_into(n).ok())
		// 	.unwrap_or_else(Zero::zero);

		// let charge_amount = BalanceOf::<T>::unique_saturated_from(can_get_vtoken);
		// Ok(charge_amount)
	}

	pub fn downscale_vtoken(
		amount: BalanceOf<T>,
		vcurrency_id: CurrencyId,
	) -> Result<BalanceOf<T>, DispatchError> {
		let currency_id = T::CurrencyIdConversion::convert_to_token(vcurrency_id)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;
		let vtoken_issuance = T::MultiCurrency::total_issuance(vcurrency_id);
		let token_pool = T::VtokenMinting::get_token_pool(currency_id);
		// let amount: u128 = amount.unique_saturated_into();
		log::debug!("downscale_vtoken--vtoken_issuance:{:?}token_pool{:?}", vtoken_issuance, token_pool);
		Ok(Self::calculate_scaling(amount, vtoken_issuance, token_pool))
		// let vtoken_issuance: u128 = vtoken_issuance.unique_saturated_into();
		// let token_pool: u128 = token_pool.unique_saturated_into();
		// let can_get_vtoken = U256::from(amount)
		// 	.checked_mul(U256::from(vtoken_issuance))
		// 	.and_then(|n| n.checked_div(U256::from(token_pool)))
		// 	.and_then(|n| TryInto::<u128>::try_into(n).ok())
		// 	.unwrap_or_else(Zero::zero);

		// let charge_amount = BalanceOf::<T>::unique_saturated_from(can_get_vtoken);
		// Ok(charge_amount)
	}

	fn calculate_scaling(
		amount: BalanceOf<T>,
		denominator: BalanceOf<T>,
		numerator: BalanceOf<T>,
	) -> BalanceOf<T> {
		let amount: u128 = amount.unique_saturated_into();
		let denominator: u128 = denominator.unique_saturated_into();
		let numerator: u128 = numerator.unique_saturated_into();
		let can_get_vtoken = U256::from(amount)
			.checked_mul(U256::from(denominator))
			.and_then(|n| n.checked_div(U256::from(numerator)))
			.and_then(|n| TryInto::<u128>::try_into(n).ok())
			.unwrap_or_else(Zero::zero);

		let charge_amount = BalanceOf::<T>::unique_saturated_from(can_get_vtoken);
		charge_amount
	}
}

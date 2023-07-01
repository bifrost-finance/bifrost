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
	PoolTokenIndex, StableAsset, StableAssetPoolId, StableAssetPoolInfo, SwapResult,
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
			AccountId = AccountIdOf<Self>,
			AtLeast64BitUnsigned = u128,
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
		SomethingStored {
			something: u32,
			who: T::AccountId,
		},
		TokenSwapped {
			swapper: AccountIdOf<T>,
			pool_id: StableAssetPoolId,
			a: u128,
			input_asset: CurrencyId,
			output_asset: CurrencyId,
			input_amount: BalanceOf<T>,
			min_output_amount: BalanceOf<T>,
			balances: Vec<BalanceOf<T>>,
			total_supply: BalanceOf<T>,
			output_amount: BalanceOf<T>,
		},
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
		Math,
		CantScaling,
		SwapUnderMin,
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
	}

	pub fn get_scaling_factors(something: u32) -> DispatchResult {
		Ok(())
	}

	fn mint(
		who: &AccountIdOf<T>,
		pool_id: StableAssetPoolId,
		mut amounts: Vec<BalanceOf<T>>,
		min_mint_amount: BalanceOf<T>,
	) -> DispatchResult {
		let mut pool_info = T::StableAsset::pool(pool_id).ok_or(Error::<T>::PoolNotExist)?;
		for (i, amount) in amounts.iter_mut().enumerate() {
			*amount = Self::upscale(
				*amount,
				*pool_info.assets.get(i as usize).ok_or(Error::<T>::NotNullable)?,
			)?;
		}
		log::debug!("amounts:{:?}", amounts);
		T::StableAsset::mint(who, pool_id, amounts, min_mint_amount)?;
		// T::StableAsset::get_mint_amount(who, pool_id, amounts, min_mint_amount)?;
		Ok(())
	}

	fn on_swap(
		who: &AccountIdOf<T>,
		pool_id: StableAssetPoolId,
		currency_id_in: PoolTokenIndex,
		currency_id_out: PoolTokenIndex,
		amount: BalanceOf<T>,
		min_dy: BalanceOf<T>,
	) -> DispatchResult {
		let mut pool_info = T::StableAsset::pool(pool_id).ok_or(Error::<T>::PoolNotExist)?;
		T::StableAsset::collect_yield(pool_id, &mut pool_info)?;
		let dx = Self::upscale(
			amount,
			*pool_info.assets.get(currency_id_in as usize).ok_or(Error::<T>::NotNullable)?,
		)?;
		// let amount_out
		let SwapResult { dx: _, dy, y, balance_i } =
			T::StableAsset::get_swap_output_amount(pool_id, currency_id_in, currency_id_out, dx)
				.ok_or(Error::<T>::CantBeZero)?;
		log::debug!("amount_out:{:?}", dy);
		let downscale_out = Self::downscale(
			dy, // TODO
			*pool_info.assets.get(currency_id_out as usize).ok_or(Error::<T>::NotNullable)?,
		)?;
		log::debug!("downscale_out:{:?}", downscale_out);
		ensure!(downscale_out >= min_dy, Error::<T>::SwapUnderMin);

		let mut balances = pool_info.balances.clone();
		let i_usize = currency_id_in as usize;
		let j_usize = currency_id_out as usize;
		balances[i_usize] = balance_i;
		balances[j_usize] = y;
		T::MultiCurrency::transfer(pool_info.assets[i_usize], who, &pool_info.account_id, amount)?;
		T::MultiCurrency::transfer(
			pool_info.assets[j_usize],
			&pool_info.account_id,
			who,
			downscale_out,
		)?;
		let asset_i = pool_info.assets[i_usize];
		let asset_j = pool_info.assets[j_usize];
		T::StableAsset::collect_fee(pool_id, &mut pool_info)?;
		T::StableAsset::insert_pool(pool_id, &pool_info);
		let a = T::StableAsset::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		Self::deposit_event(Event::TokenSwapped {
			swapper: who.clone(),
			pool_id,
			a,
			input_asset: asset_i,
			output_asset: asset_j,
			input_amount: amount,
			min_output_amount: min_dy,
			balances: pool_info.balances.clone(),
			total_supply: pool_info.total_supply,
			output_amount: downscale_out,
		});
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
		let currency_id = T::CurrencyIdConversion::convert_to_token(vcurrency_id)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;
		let vtoken_issuance = T::MultiCurrency::total_issuance(vcurrency_id);
		let token_pool = T::VtokenMinting::get_token_pool(currency_id);
		log::debug!("vtoken_issuance:{:?}token_pool{:?}", vtoken_issuance, token_pool);
		ensure!(vtoken_issuance <= token_pool, Error::<T>::CantScaling);
		Ok(Self::calculate_scaling(amount, token_pool, vtoken_issuance))
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
		log::debug!(
			"downscale_vtoken--vtoken_issuance:{:?}token_pool{:?}",
			vtoken_issuance,
			token_pool
		);
		ensure!(vtoken_issuance <= token_pool, Error::<T>::CantScaling);
		Ok(Self::calculate_scaling(amount, vtoken_issuance, token_pool))
	}

	fn calculate_scaling(
		amount: BalanceOf<T>,
		numerator: BalanceOf<T>,
		denominator: BalanceOf<T>,
	) -> BalanceOf<T> {
		let amount: u128 = amount.unique_saturated_into();
		let denominator: u128 = denominator.unique_saturated_into();
		let numerator: u128 = numerator.unique_saturated_into();
		let can_get_vtoken = U256::from(amount)
			.checked_mul(U256::from(numerator))
			.and_then(|n| n.checked_div(U256::from(denominator)))
			.and_then(|n| TryInto::<u128>::try_into(n).ok())
			.unwrap_or_else(Zero::zero);

		let charge_amount = BalanceOf::<T>::unique_saturated_from(can_get_vtoken);
		charge_amount
	}
}

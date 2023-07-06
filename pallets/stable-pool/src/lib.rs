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
	traits::{
		fungibles::{Inspect, Mutate, Transfer},
		tokens::{currency, fungibles},
		Currency,
	},
	transactional,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, CurrencyIdConversion, TimeUnit, VtokenMintingOperator};
use nutsfinance_stable_asset::{
	MintResult, PoolTokenIndex, RedeemProportionResult, StableAsset, StableAssetPoolId,
	StableAssetPoolInfo, SwapResult,
};
use orml_traits::MultiCurrency;
use sp_core::U256;
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

// #[allow(type_alias_bounds)]
// pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
// 	<T as frame_system::Config>::AccountId,
// >>::CurrencyId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + nutsfinance_stable_asset::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type WeightInfo: WeightInfo;

		type ControlOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		type MultiCurrency: fungibles::Inspect<AccountIdOf<Self>, AssetId = CurrencyId, Balance = Self::Balance>
			+ fungibles::Mutate<AccountIdOf<Self>, AssetId = CurrencyId, Balance = Self::Balance>
			+ fungibles::Transfer<AccountIdOf<Self>, AssetId = CurrencyId, Balance = Self::Balance>;
		// + MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type StableAsset: nutsfinance_stable_asset::StableAsset<
			AssetId = CurrencyId,
			Balance = Self::Balance,
			AccountId = AccountIdOf<Self>,
			AtLeast64BitUnsigned = u128,
			Config = Self,
			BlockNumber = Self::BlockNumber,
		>;

		type VtokenMinting: VtokenMintingOperator<
			CurrencyId,
			Self::Balance,
			AccountIdOf<Self>,
			TimeUnit,
		>;

		type CurrencyIdConversion: CurrencyIdConversion<CurrencyId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn something)]
	pub type Something<T> = StorageValue<_, u32>;

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
			input_amount: T::Balance,
			min_output_amount: T::Balance,
			balances: Vec<T::Balance>,
			total_supply: T::Balance,
			output_amount: T::Balance,
		},
		Minted {
			minter: AccountIdOf<T>,
			pool_id: StableAssetPoolId,
			a: u128,
			input_amounts: Vec<T::Balance>,
			min_output_amount: T::Balance,
			balances: Vec<T::Balance>,
			total_supply: T::Balance,
			fee_amount: T::Balance,
			output_amount: T::Balance,
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
		RedeemUnderMin,
		MintUnderMin,
		CantMint,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(10)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::do_something())]
		pub fn set_token_rate(
			origin: OriginFor<T>,
			currency: CurrencyId,
			token_rate: Option<(u128, u128)>,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			T::StableAsset::set_token_rate(currency, token_rate);
			Ok(())
		}

		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::do_something())]
		#[transactional]
		pub fn create_pool(
			origin: OriginFor<T>,
			pool_asset: CurrencyId,
			assets: Vec<CurrencyId>,
			precisions: Vec<u128>,
			mint_fee: u128,
			swap_fee: u128,
			redeem_fee: u128,
			initial_a: u128,
			fee_recipient: T::AccountId,
			yield_recipient: T::AccountId,
			precision: u128,
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			T::StableAsset::create_pool(
				pool_asset,
				assets,
				precisions,
				mint_fee,
				swap_fee,
				redeem_fee,
				initial_a,
				fee_recipient,
				yield_recipient,
				precision,
			)
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::do_something())]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amounts: Vec<T::Balance>,
			min_mint_amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::mint_inner(&who, pool_id, amounts, min_mint_amount)
		}

		#[pallet::call_index(2)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::do_something())]
		#[transactional]
		pub fn swap(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			i: PoolTokenIndex,
			j: PoolTokenIndex,
			dx: T::Balance,
			min_dy: T::Balance,
			// asset_length: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::on_swap(&who, pool_id, i, j, dx, min_dy)
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::do_something())]
		pub fn redeem_proportion(
			origin: OriginFor<T>,
			pool_id: StableAssetPoolId,
			amount: T::Balance,
			min_redeem_amounts: Vec<T::Balance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::redeem_proportion_inner(&who, pool_id, amount, min_redeem_amounts)
		}
	}
}

impl<T: Config> Pallet<T> {
	fn mint_inner(
		who: &AccountIdOf<T>,
		pool_id: StableAssetPoolId,
		mut amounts: Vec<T::Balance>,
		min_mint_amount: T::Balance,
	) -> DispatchResult {
		let mut pool_info = T::StableAsset::pool(pool_id).ok_or(Error::<T>::PoolNotExist)?;
		let amounts_old = amounts.clone();
		for (i, amount) in amounts.iter_mut().enumerate() {
			*amount = Self::upscale(
				*amount,
				*pool_info.assets.get(i as usize).ok_or(Error::<T>::NotNullable)?,
			)?;
		}
		log::debug!("amounts:{:?}", amounts);
		// T::StableAsset::mint(who, pool_id, amounts, min_mint_amount)?;
		T::StableAsset::collect_yield(pool_id, &mut pool_info)?;
		let MintResult { mint_amount, fee_amount, balances, total_supply } =
			T::StableAsset::get_mint_amount(pool_id, &amounts).ok_or(Error::<T>::CantBeZero)?;

		let a = T::StableAsset::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		ensure!(mint_amount >= min_mint_amount, Error::<T>::MintUnderMin);
		for (i, amount) in amounts.iter().enumerate() {
			if *amount == Zero::zero() {
				continue;
			}
			ensure!(
				amounts_old[i] >=
					Self::downscale(
						*amount,
						*pool_info.assets.get(i as usize).ok_or(Error::<T>::NotNullable)?,
					)?,
				Error::<T>::CantMint
			);
			<<T as pallet::Config>::MultiCurrency as fungibles::Transfer<AccountIdOf<T>>>::transfer(
				pool_info.assets[i],
				who,
				&pool_info.account_id,
				amounts_old[i],
				false,
			)?;
		}
		log::debug!("mint___amounts:{:?}{:?}", amounts, total_supply);
		if fee_amount > Zero::zero() {
			<<T as pallet::Config>::MultiCurrency as fungibles::Mutate<AccountIdOf<T>>>::mint_into(
				pool_info.pool_asset,
				&pool_info.fee_recipient,
				fee_amount,
			)?;
		}
		<<T as pallet::Config>::MultiCurrency as fungibles::Mutate<AccountIdOf<T>>>::mint_into(
			pool_info.pool_asset,
			who,
			mint_amount.into(),
		)?;
		pool_info.total_supply = total_supply;
		pool_info.balances = balances;
		T::StableAsset::collect_fee(pool_id, &mut pool_info)?;
		T::StableAsset::insert_pool(pool_id, &pool_info);
		Self::deposit_event(Event::Minted {
			minter: who.clone(),
			pool_id,
			a,
			input_amounts: amounts,
			min_output_amount: min_mint_amount,
			balances: pool_info.balances.clone(),
			total_supply: pool_info.total_supply,
			fee_amount,
			output_amount: mint_amount,
		});
		Ok(())
	}

	#[transactional]
	fn redeem_proportion_inner(
		who: &AccountIdOf<T>,
		pool_id: StableAssetPoolId,
		amount: T::Balance,
		min_redeem_amounts: Vec<T::Balance>,
	) -> DispatchResult {
		let mut pool_info: StableAssetPoolInfo<
			CurrencyId,
			u128,
			<T as nutsfinance_stable_asset::Config>::Balance,
			AccountIdOf<T>,
			T::BlockNumber,
		> = T::StableAsset::pool(pool_id).ok_or(Error::<T>::PoolNotExist)?;
		T::StableAsset::collect_yield(pool_id, &mut pool_info)?;

		let RedeemProportionResult {
			mut amounts,
			balances,
			fee_amount,
			total_supply,
			redeem_amount,
		} = T::StableAsset::get_redeem_proportion_amount(&pool_info, amount)
			.ok_or(Error::<T>::CantBeZero)?;
		log::debug!("redeem_proportion++amounts:{:?}redeem_amount{:?}", amounts, redeem_amount);

		for (i, amount) in amounts.iter_mut().enumerate() {
			*amount = Self::downscale(
				*amount,
				*pool_info.assets.get(i as usize).ok_or(Error::<T>::NotNullable)?,
			)?;
		}
		log::debug!("redeem_proportion==amounts:{:?}", amounts);

		let zero = Zero::zero();
		for i in 0..amounts.len() {
			ensure!(
				amounts[i] >= *min_redeem_amounts.get(i as usize).ok_or(Error::<T>::NotNullable)?,
				Error::<T>::RedeemUnderMin
			);
			<<T as pallet::Config>::MultiCurrency as fungibles::Transfer<AccountIdOf<T>>>::transfer(
				pool_info.assets[i],
				&pool_info.account_id,
				who,
				amounts[i],
				false,
			)?;
		}
		if fee_amount > zero {
			<<T as pallet::Config>::MultiCurrency as fungibles::Transfer<AccountIdOf<T>>>::transfer(
				pool_info.pool_asset,
				who,
				&pool_info.fee_recipient,
				fee_amount,
				false,
			)?;
		}
		<<T as pallet::Config>::MultiCurrency as fungibles::Mutate<AccountIdOf<T>>>::burn_from(
			pool_info.pool_asset,
			who,
			redeem_amount,
		)?;

		pool_info.total_supply = total_supply;
		pool_info.balances = balances;
		// Since the output amounts are round down, collect fee updates pool balances and total
		// supply.
		T::StableAsset::collect_fee(pool_id, &mut pool_info)?;
		T::StableAsset::insert_pool(pool_id, &pool_info);
		let a = T::StableAsset::get_a(
			pool_info.a,
			pool_info.a_block,
			pool_info.future_a,
			pool_info.future_a_block,
		)
		.ok_or(Error::<T>::Math)?;
		Ok(())
	}

	fn on_swap(
		who: &AccountIdOf<T>,
		pool_id: StableAssetPoolId,
		currency_id_in: PoolTokenIndex,
		currency_id_out: PoolTokenIndex,
		amount: T::Balance,
		min_dy: T::Balance,
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
		<<T as pallet::Config>::MultiCurrency as fungibles::Transfer<AccountIdOf<T>>>::transfer(
			pool_info.assets[i_usize],
			who,
			&pool_info.account_id,
			amount,
			false,
		)?;
		<<T as pallet::Config>::MultiCurrency as fungibles::Transfer<AccountIdOf<T>>>::transfer(
			pool_info.assets[j_usize],
			&pool_info.account_id,
			who,
			downscale_out,
			false,
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
		amount: T::Balance,
		currency_id: CurrencyId,
	) -> Result<T::Balance, DispatchError> {
		log::debug!("upscale currency_id:{:?}", currency_id);
		match currency_id {
			CurrencyId::VToken(_) | CurrencyId::VToken2(_) =>
				Self::upscale_vtoken(amount, currency_id),
			_ => Ok(amount),
		}
	}
	pub fn downscale(
		amount: T::Balance,
		currency_id: CurrencyId,
	) -> Result<T::Balance, DispatchError> {
		log::debug!("downscale currency_id:{:?}", currency_id);
		match currency_id {
			CurrencyId::VToken(_) | CurrencyId::VToken2(_) =>
				Self::downscale_vtoken(amount, currency_id),
			// CurrencyId::Token2(_) => Self::downscale_token(amount, currency_id),
			_ => Ok(amount),
		}
	}

	pub fn upscale_vtoken(
		amount: T::Balance,
		vcurrency_id: CurrencyId,
	) -> Result<T::Balance, DispatchError> {
		if let Some((demoninator, numerator)) = T::StableAsset::get_token_rate(vcurrency_id) {
			return Ok(Self::calculate_scaling(
				amount.into(),
				numerator.into(),
				demoninator.into(),
			));
		}

		let currency_id = T::CurrencyIdConversion::convert_to_token(vcurrency_id)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;
		let vtoken_issuance = <<T as pallet::Config>::MultiCurrency as fungibles::Inspect<
			AccountIdOf<T>,
		>>::total_issuance(vcurrency_id);
		let token_pool = T::VtokenMinting::get_token_pool(currency_id);
		log::debug!("vtoken_issuance:{:?}token_pool{:?}", vtoken_issuance, token_pool);
		ensure!(vtoken_issuance <= token_pool, Error::<T>::CantScaling);
		Ok(Self::calculate_scaling(amount.into(), token_pool.into(), vtoken_issuance.into()))
	}

	pub fn downscale_vtoken(
		amount: T::Balance,
		vcurrency_id: CurrencyId,
	) -> Result<T::Balance, DispatchError> {
		if let Some((numerator, demoninator)) = T::StableAsset::get_token_rate(vcurrency_id) {
			return Ok(Self::calculate_scaling(
				amount.into(),
				numerator.into(),
				demoninator.into(),
			));
		}

		let currency_id = T::CurrencyIdConversion::convert_to_token(vcurrency_id)
			.map_err(|_| Error::<T>::NotSupportTokenType)?;
		let vtoken_issuance = <<T as pallet::Config>::MultiCurrency as fungibles::Inspect<
			AccountIdOf<T>,
		>>::total_issuance(vcurrency_id);
		let token_pool = T::VtokenMinting::get_token_pool(currency_id);
		// let amount: u128 = amount.unique_saturated_into();
		log::debug!(
			"downscale_vtoken--vtoken_issuance:{:?}token_pool{:?}",
			vtoken_issuance,
			token_pool
		);
		ensure!(vtoken_issuance <= token_pool, Error::<T>::CantScaling);
		Ok(Self::calculate_scaling(amount.into(), vtoken_issuance.into(), token_pool.into()))
	}

	fn calculate_scaling(
		amount: <T as nutsfinance_stable_asset::Config>::AtLeast64BitUnsigned, // T::Balance,
		numerator: <T as nutsfinance_stable_asset::Config>::AtLeast64BitUnsigned,
		denominator: <T as nutsfinance_stable_asset::Config>::AtLeast64BitUnsigned,
	) -> T::Balance {
		let amount: u128 = amount.saturated_into::<u128>(); //.unique_saturated_into();
		let denominator: u128 = denominator.saturated_into::<u128>();
		let numerator: u128 = numerator.saturated_into::<u128>();
		let can_get_vtoken = U256::from(amount)
			.checked_mul(U256::from(numerator))
			.and_then(|n| n.checked_div(U256::from(denominator)))
			.and_then(|n| TryInto::<u128>::try_into(n).ok())
			.unwrap_or_else(Zero::zero);

		let charge_amount: <T as nutsfinance_stable_asset::Config>::AtLeast64BitUnsigned =
			can_get_vtoken.into();
		charge_amount.into()
	}
}

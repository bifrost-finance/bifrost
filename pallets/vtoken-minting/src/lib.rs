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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, Saturating, Zero},
		SaturatedConversion,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{Balance, CurrencyId};
use orml_traits::MultiCurrency;
pub use pallet::*;

#[allow(type_alias_bounds)]
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

#[allow(type_alias_bounds)]
pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[allow(type_alias_bounds)]
type BalanceOf<T: Config> =
	<<T as Config>::MultiCurrency as MultiCurrency<AccountIdOf<T>>>::Balance;

pub type MintId = u32;

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub enum TimeUnit {
	Era(u32),
	SlashingSpan(u32),
}
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;
		// + MultiReservableCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		/// The only origin that can edit token issuer list
		type ControlOrigin: EnsureOrigin<Self::Origin>;

		/// The amount of mint
		#[pallet::constant]
		type MaximumMintId: Get<u32>;

		#[pallet::constant]
		type EntranceAccount: Get<PalletId>;

		#[pallet::constant]
		type ExitAccount: Get<PalletId>;

		#[pallet::constant]
		type FeeAccount: Get<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		minted {
			token: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		},
		Redeemed {
			token: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		},
		Rebonded {
			token: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		},
		UnlockDurationSet {
			token: CurrencyIdOf<T>,
			era_count: u32,
		},
		MinimumMintSet {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		MinimumRedeemSet {
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		},
		SupportRebondTokenAdded {
			token: CurrencyIdOf<T>,
		},
		SupportRebondTokenRemoved {
			token: CurrencyIdOf<T>,
		},
		/// Several fees has been set.
		FeeSet {
			mint_fee: BalanceOf<T>,
			redeem_fee: BalanceOf<T>,
			hosting_fee: BalanceOf<T>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Number of user unlocking chunks exceed MaxUserUnlockingChunks
		TooManyUserUnlockingChunks,
		/// Number of era unlocking chunks exceed MaxEraUnlockingChunks
		TooManyEraUnlockingChunks,
		/// Invalid token to rebond.
		InvalidRebondToken,
		/// Invalid token.
		InvalidToken,
		/// Token type not support.
		NotSupportTokenType,
		NotEnoughBalanceToUnlock,
		TokenToRebondNotZero,
	}

	#[pallet::storage]
	#[pallet::getter(fn fees)]
	pub type Fees<T: Config> =
		StorageValue<_, (BalanceOf<T>, BalanceOf<T>, BalanceOf<T>), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_pool)]
	pub type TokenPool<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn unlock_duration)]
	pub type UnlockDuration<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ongoing_era)]
	pub type OngoingEra<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit>;

	#[pallet::storage]
	#[pallet::getter(fn minimum_mint)]
	pub type MinimumMint<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn minimum_redeem)]
	pub type MinimumRedeem<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_unlock_next_id)]
	pub type TokenUnlockNextId<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_unlock_ledger)]
	pub(crate) type TokenUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		Blake2_128Concat,
		MintId,
		(T::AccountId, BalanceOf<T>, TimeUnit),
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn user_unlock_ledger)]
	pub(crate) type UserUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountIdOf<T>,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		(BalanceOf<T>, BoundedVec<MintId, T::MaximumMintId>),
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn era_unlock_ledger)]
	pub(crate) type EraUnlockLedger<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TimeUnit,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		(BalanceOf<T>, BoundedVec<MintId, T::MaximumMintId>),
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn max_user_unlocking_chunks)]
	pub type MaxUserUnlockingChunks<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u32>;

	#[pallet::storage]
	#[pallet::getter(fn max_era_unlocking_chuncks)]
	pub type MaxEraUnlockingChunks<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, TimeUnit>;

	#[pallet::storage]
	#[pallet::getter(fn token_to_deduct)]
	pub type TokenToDeduct<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_to_add)]
	pub type TokenToAdd<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_to_rebond)]
	pub type TokenToRebond<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, Option<BalanceOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn minter)]
	pub(crate) type Minter<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// #[pallet::weight(T::WeightInfo::mint())]
		#[pallet::weight(10000)]
		pub fn mint(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;
			let token_pool_amount = Self::token_pool(token_id);
			let vtoken_id = token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);
			let vtoken_amount = token_amount.saturating_mul(vtoken_total_issuance.into()) /
				token_pool_amount.into();
			// Transfer the user's token to EntranceAccount.
			T::MultiCurrency::transfer(
				token_id,
				&exchanger,
				&T::EntranceAccount::get().into_account(),
				token_amount,
			);
			// Issue the corresponding vtoken to the user's account.
			T::MultiCurrency::deposit(vtoken_id, &exchanger, vtoken_amount)?;
			TokenPool::<T>::mutate(token_id, |pool| pool.saturating_add(token_pool_amount));
			TokenToAdd::<T>::mutate(token_id, |pool| pool.saturating_add(token_pool_amount));

			Self::deposit_event(Event::minted { token: token_id, token_amount });
			Ok(())
		}

		#[pallet::weight(10000)]
		pub fn redeem(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			vtoken_amount: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;
			let token_pool_amount = Self::token_pool(token_id);
			let vtoken_id = token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;
			let vtoken_total_issuance = T::MultiCurrency::total_issuance(vtoken_id);

			// let token_pool_amount = Self::user_unlock_ledger(&exchanger, token_id);

			// ensure!(
			// 	counts <= T::MaxUserUnlockingChunks::get(token_id),
			// 	Error::<T>::TooManyUserUnlockingChunks
			// );
			// ensure!(
			// 	counts <= T::MaxEraUnlockingChunks::get(token_id),
			// 	Error::<T>::TooManyEraUnlockingChunks
			// );

			T::MultiCurrency::withdraw(vtoken_id, &exchanger, vtoken_amount)?;
			TokenPool::<T>::mutate(token_id, |pool| pool.saturating_sub(token_pool_amount));
			TokenToDeduct::<T>::mutate(token_id, |pool| pool.saturating_add(token_pool_amount));

			Ok(())
		}

		#[pallet::weight(10000)]
		pub fn rebond(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			let token_amount_to_rebond =
				Self::token_to_rebond(token_id).ok_or(Error::<T>::InvalidRebondToken)?;
			if let Some((user_unlock_amount, ledger_list)) =
				Self::user_unlock_ledger(&exchanger, token_id)
			{
				ensure!(user_unlock_amount >= token_amount, Error::<T>::NotEnoughBalanceToUnlock);
				TokenPool::<T>::mutate(token_id, |pool| pool.saturating_add(token_amount));
				for index in ledger_list.iter() {}
			}
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_unlock_duration(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			era_count: u32,
		) -> DispatchResult {
			ensure_root(origin)?;

			if !UnlockDuration::<T>::contains_key(token) {
				UnlockDuration::<T>::insert(token, era_count);
			} else {
				UnlockDuration::<T>::mutate(token, |old_era_count| {
					*old_era_count = era_count;
				});
			}

			Self::deposit_event(Event::UnlockDurationSet { token, era_count });

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_minimum_mint(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			if !MinimumMint::<T>::contains_key(token) {
				MinimumMint::<T>::insert(token, amount);
			} else {
				MinimumMint::<T>::mutate(token, |old_amount| {
					*old_amount = amount;
				});
			}

			Self::deposit_event(Event::MinimumMintSet { token, amount });

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_minimum_redeem(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			if !MinimumRedeem::<T>::contains_key(token) {
				MinimumRedeem::<T>::insert(token, amount);
			} else {
				MinimumRedeem::<T>::mutate(token, |old_amount| {
					*old_amount = amount;
				});
			}

			Self::deposit_event(Event::MinimumRedeemSet { token, amount });
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn add_support_rebond_token(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			if !TokenToRebond::<T>::contains_key(token) {
				TokenToRebond::<T>::insert(token, Some(BalanceOf::<T>::zero()));
				Self::deposit_event(Event::SupportRebondTokenAdded { token });
			}

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn remove_support_rebond_token(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			if TokenToRebond::<T>::contains_key(token) {
				let token_amount_to_rebond =
					Self::token_to_rebond(token).ok_or(Error::<T>::InvalidRebondToken)?;
				ensure!(
					token_amount_to_rebond == Some(BalanceOf::<T>::zero()),
					Error::<T>::TokenToRebondNotZero
				);

				TokenToRebond::<T>::remove(token);
				Self::deposit_event(Event::SupportRebondTokenRemoved { token });
			}
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_fees(
			origin: OriginFor<T>,
			mint_fee: BalanceOf<T>,
			redeem_fee: BalanceOf<T>,
			hosting_fee: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Fees::<T>::mutate(|fees| *fees = (mint_fee, redeem_fee, hosting_fee));

			Self::deposit_event(Event::FeeSet { mint_fee, redeem_fee, hosting_fee });
			Ok(())
		}
	}
}

/// The interface to call VtokneMinting module functions.
pub trait VtokenMintingOperator<CurrencyId, Balance> {
	/// Increase the token amount for the storage "token_pool" in the VtokenMining module.
	fn increase_token_pool(currency_id: CurrencyId, token_amount: Balance) -> DispatchResult;

	/// Decrease the token amount for the storage "token_pool" in the VtokenMining module.
	fn decrease_token_pool(currency_id: CurrencyId, token_amount: Balance) -> DispatchResult;

	// Update the ongoing era for a CurrencyId.
	fn update_ongoing_era(currency_id: CurrencyId, era: TimeUnit) -> DispatchResult;

	// Get the current era of a CurrencyId.
	fn get_ongoing_era(currency_id: CurrencyId) -> Option<TimeUnit>;
}

impl<T: Config> VtokenMintingOperator<CurrencyId, BalanceOf<T>> for Pallet<T> {
	fn increase_token_pool(currency_id: CurrencyId, token_amount: BalanceOf<T>) -> DispatchResult {
		TokenPool::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.saturating_add(token_amount);
			Ok(())
		})?;

		Ok(())
	}

	fn decrease_token_pool(currency_id: CurrencyId, token_amount: BalanceOf<T>) -> DispatchResult {
		TokenPool::<T>::mutate(currency_id, |pool| -> Result<(), Error<T>> {
			*pool = pool.saturating_sub(token_amount);
			Ok(())
		})?;

		Ok(())
	}

	fn update_ongoing_era(currency_id: CurrencyId, era: TimeUnit) -> DispatchResult {
		OngoingEra::<T>::mutate(currency_id, |time_unit| -> Result<(), Error<T>> {
			*time_unit = Some(era);
			Ok(())
		})?;

		Ok(())
	}

	fn get_ongoing_era(currency_id: CurrencyId) -> Option<TimeUnit> {
		Self::ongoing_era(currency_id)
	}
}

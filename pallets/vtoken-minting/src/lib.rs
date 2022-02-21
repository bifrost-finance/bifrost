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

use frame_support::{pallet_prelude::*, PalletId};
use frame_system::pallet_prelude::*;
use node_primitives::CurrencyId;
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
	Era(u64),
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
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {}

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
	pub type MinimumMint<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn minimum_redeem)]
	pub type MinimumRedeem<T: Config> =
		StorageMap<_, Twox64Concat, CurrencyIdOf<T>, u32, ValueQuery>;

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
	pub type TokenToDeduct<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn token_to_add)]
	pub type TokenToAdd<T: Config> = StorageMap<_, Twox64Concat, CurrencyIdOf<T>, BalanceOf<T>>;

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
		pub fn mint(origin: OriginFor<T>, token_amount: BalanceOf<T>) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(10000)]
		pub fn redeem(origin: OriginFor<T>, vtoken_amount: BalanceOf<T>) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(10000)]
		pub fn rebond(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			token_amount: BalanceOf<T>,
		) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_unlock_duration(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			era_count: u32,
		) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_minimum_mint(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_minimum_redeem(origin: OriginFor<T>, token: CurrencyIdOf<T>) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn add_support_rebond_token(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn remove_support_rebond_token(
			origin: OriginFor<T>,
			token: CurrencyIdOf<T>,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_fees(
			origin: OriginFor<T>,
			mint_fee: BalanceOf<T>,
			redeem_fee: BalanceOf<T>,
			hosting_fee: BalanceOf<T>,
		) -> DispatchResult {
			Ok(())
		}
	}
}

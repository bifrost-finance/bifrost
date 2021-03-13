// Copyright 2019-2021 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use core::marker::PhantomData;
use frame_support::{
	transactional, pallet_prelude::*, traits::{Get, Hooks, IsType}
};
use frame_system::{
	ensure_root, ensure_signed, pallet_prelude::{OriginFor, BlockNumberFor}
};
use node_primitives::{CurrencyIdExt, CurrencyId, VtokenMintExt};
use orml_traits::{
	account::MergeAccount, MultiCurrency, GetByKey,
	MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency
};
use sp_runtime::{traits::{Saturating, Zero}, DispatchResult};

pub use pallet::*;

mod mock;
mod tests;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<<T as frame_system::Config>::AccountId>>::Balance;
type CurrencyIdOf<T> =
	<<T as Config>::MultiCurrency as MultiCurrency<<T as frame_system::Config>::AccountId>>::CurrencyId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub trait WeightInfo {
		fn to_vtoken<T: Config>() -> Weight;
		fn to_token() -> Weight;
	}
	
	impl WeightInfo for () {
		fn to_vtoken<T: Config>() -> Weight { Default::default() }
		fn to_token() -> Weight { Default::default() }
	}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// A handler to manipulate assets module
		type MultiCurrency: MergeAccount<Self::AccountId>
			+ MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<Self::AccountId, CurrencyId = CurrencyId>;
	
		/// Event
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	
		#[pallet::constant]
		type VtokenMintDuration: Get<Self::BlockNumber>;

		/// The ROI of each token by every block.
		type RateOfInterestEachBlock: GetByKey<CurrencyIdOf<Self>, BalanceOf<Self>>;
	
		/// Set default weight
		type WeightInfo: WeightInfo;
	}

	/// Total mint pool
	#[pallet::storage]
	#[pallet::getter(fn mint_pool)]
	pub(crate) type MintPool<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		BalanceOf<T>,
		ValueQuery
	>;

	/// Collect referrer, minter => ([(referrer1, 1000), (referrer2, 2000), ...], total_point)
	/// total_point = 1000 + 2000 + ...
	/// referrer must be unique, so check it unique while a new referrer incoming.
	/// and insert the new channel to the
	#[pallet::storage]
	#[pallet::getter(fn referrer_channels)]
	pub(crate) type ReferrerChannels<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		(Vec<(T::AccountId, BalanceOf<T>)>, BalanceOf<T>),
		ValueQuery
	>;

	/// Referer channels for all users
	#[pallet::storage]
	#[pallet::getter(fn all_referer_channels)]
	pub(crate) type AllReferrerChannels<T: Config> = StorageValue<
		_,
		(BTreeMap<T::AccountId, BalanceOf<T>>, BalanceOf<T>),
		ValueQuery,
		()
	>;

	#[pallet::event]
	#[pallet::metadata(BalanceOf<T> = "Balance", CurrencyIdOf<T> = "CurrencyId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		UpdateRatePerBlockSuccess,
		MintedVToken(T::AccountId, CurrencyIdOf<T>, BalanceOf<T>),
		MintedToken(T::AccountId, CurrencyIdOf<T>, BalanceOf<T>),
		RedeemedPointsSuccess,
		UpdateVtokenPoolSuccess,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account balance must be greater than or equal to the transfer amount.
		BalanceLow,
		/// Balance should be non-zero.
		BalanceZero,
		/// Token type not support
		NotSupportTokenType,
		/// Empty vtoken pool, cause there's no price at all
		EmptyVtokenPool,
		/// The amount of token you want to mint is bigger than the vtoken pool
		NotEnoughVtokenPool,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set price for minting vtoken.
		///
		/// The dispatch origin for this call must be `Root` by the
		/// transactor.
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		#[transactional]
		pub fn set_vtoken_pool(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			#[pallet::compact] new_token_pool: BalanceOf<T>,
			#[pallet::compact] new_vtoken_pool: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			ensure_root(origin.clone())?;

			let (token_id, vtoken_id) = currency_id
				.get_token_pair()
				.ok_or(Error::<T>::NotSupportTokenType)?;

			Self::expand_mint_pool(token_id.into(), new_token_pool)?;
			Self::expand_mint_pool(vtoken_id.into(), new_vtoken_pool)?;

			Self::deposit_event(Event::UpdateVtokenPoolSuccess);

			Ok(().into())
		}

		/// Mint vtoken by token.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		#[transactional]
		pub fn to_vtoken(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			#[pallet::compact] token_amount: BalanceOf<T>
		) -> DispatchResultWithPostInfo {
			let minter = ensure_signed(origin)?;

			ensure!(!token_amount.is_zero(), Error::<T>::BalanceZero);
			ensure!(currency_id.is_vtoken(), Error::<T>::NotSupportTokenType);

			// Get paired tokens.
			let (token_id, _vtoken_id) = currency_id
				.get_token_pair()
				.ok_or(Error::<T>::NotSupportTokenType)?;

			let token_balances = T::MultiCurrency::free_balance(token_id.into(), &minter);
			ensure!(token_balances >= token_amount, Error::<T>::BalanceLow);

			// Total amount of tokens.
			let token_pool = Self::get_mint_pool(token_id.into());
			// Total amount of vtokens.
			let vtoken_pool = Self::get_mint_pool(currency_id);
			ensure!(
				!token_pool.is_zero() && !vtoken_pool.is_zero(),
				Error::<T>::EmptyVtokenPool
			);

			let vtokens_buy = token_amount.saturating_mul(vtoken_pool) / token_pool;

			T::MultiCurrency::withdraw(token_id.into(), &minter, token_amount)?;
			T::MultiCurrency::deposit(currency_id, &minter, vtokens_buy)?;

			// Alter mint pool
			Self::expand_mint_pool(token_id.into(), token_amount)?;
			Self::expand_mint_pool(currency_id, vtokens_buy)?;

			Self::deposit_event(Event::MintedVToken(minter, currency_id, vtokens_buy));

			Ok(().into())
		}

		/// Mint token by vtoken.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		#[transactional]
		pub fn to_token(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			#[pallet::compact] vtoken_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let minter = ensure_signed(origin)?;

			ensure!(!vtoken_amount.is_zero(), Error::<T>::BalanceZero);
			ensure!(currency_id.is_token(), Error::<T>::NotSupportTokenType);

			// Get paired tokens.
			let (_token_id, vtoken_id) = currency_id.get_token_pair().unwrap();

			let vtoken_balances = T::MultiCurrency::free_balance(vtoken_id.into(), &minter);
			ensure!(vtoken_balances >= vtoken_amount, Error::<T>::BalanceLow);

			// Total amount of tokens.
			let token_pool = Self::get_mint_pool(currency_id);
			// Total amount of vtokens.
			let vtoken_pool = Self::get_mint_pool(vtoken_id.into());
			ensure!(
				!token_pool.is_zero() && !vtoken_pool.is_zero(),
				Error::<T>::EmptyVtokenPool
			);

			let tokens_buy = vtoken_amount.saturating_mul(token_pool) / vtoken_pool;
			ensure!(
				vtoken_pool >= tokens_buy && vtoken_pool >= vtoken_amount,
				Error::<T>::NotEnoughVtokenPool
			);

			T::MultiCurrency::withdraw(vtoken_id.into(), &minter, vtoken_amount)?;
			T::MultiCurrency::deposit(currency_id, &minter, tokens_buy)?;

			// Alter mint pool
			Self::reduce_mint_pool(currency_id, tokens_buy)?;
			Self::reduce_mint_pool(vtoken_id.into(), vtoken_amount)?;

			Self::deposit_event(Event::MintedToken(minter, currency_id, tokens_buy));

			Ok(().into())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_block_number: T::BlockNumber) {
			// Mock staking reward for pulling up vtoken price
			for (currency_id, _) in MintPool::<T>::iter() {
				// Only inject tokens into token pool
				if currency_id.is_token() {
					let year_rate = T::RateOfInterestEachBlock::get(&currency_id);
					let _ = Self::expand_mint_pool(currency_id, year_rate);
				}
			}
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub pools: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig { pools: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (currency_id, token_pool) in self.pools.iter() {
				MintPool::<T>::insert(currency_id, token_pool);
			}
		}
	}
}

impl<T: Config> VtokenMintExt for Pallet<T> {
	type CurrencyId = CurrencyIdOf<T>;
	type Balance = BalanceOf<T>;

	/// Get mint pool by currency id
	fn get_mint_pool(currency_id: Self::CurrencyId) -> Self::Balance {
		Self::mint_pool(currency_id)
	}

	/// Expand mint pool
	fn expand_mint_pool(currency_id: Self::CurrencyId, amount: Self::Balance) -> DispatchResult {
		MintPool::<T>::mutate(currency_id, |pool| {
			*pool = pool.saturating_add(amount);
		});

		Ok(())
	}

	/// Reduce mint pool
	fn reduce_mint_pool(currency_id: Self::CurrencyId, amount: Self::Balance) -> DispatchResult {
		MintPool::<T>::mutate(currency_id, |pool| {
			*pool = pool.saturating_sub(amount);
		});

		Ok(())
	}
}
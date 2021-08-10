// This file is part of Bifrost.

// Copyright (C) 2019-2021 Liebi Technologies (UK) Ltd.
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
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use core::marker::PhantomData;

use frame_support::{
	pallet_prelude::*,
	traits::{Hooks, IsType},
	transactional,
};
use frame_system::{
	ensure_root, ensure_signed,
	pallet_prelude::{BlockNumberFor, OriginFor},
};
use node_primitives::{CurrencyId, CurrencyIdExt, MinterRewardExt, VtokenMintExt};
use orml_traits::{
	currency::TransferAll, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency,
	MultiReservableCurrency,
};
pub use pallet::*;
use sp_runtime::{
	traits::{CheckedSub, Saturating, Zero},
	DispatchResult,
};
pub use weights::WeightInfo;

mod mock;
mod tests;
pub mod weights;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// A handler to manipulate assets module.
		type MultiCurrency: TransferAll<Self::AccountId>
			+ MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<Self::AccountId, CurrencyId = CurrencyId>;

		/// Event
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Record mint reward
		type MinterReward: MinterRewardExt<
			Self::AccountId,
			BalanceOf<Self>,
			CurrencyIdOf<Self>,
			Self::BlockNumber,
		>;

		/// Set default weight.
		type WeightInfo: WeightInfo;
	}

	/// Total mint pool
	#[pallet::storage]
	#[pallet::getter(fn mint_pool)]
	pub(crate) type MintPool<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;

	/// Record when and how much balance user want to redeem.
	#[pallet::storage]
	#[pallet::getter(fn redeem_record)]
	pub(crate) type RedeemRecord<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		Vec<(T::BlockNumber, BalanceOf<T>)>,
		ValueQuery,
	>;

	/// List lock period while staking.
	#[pallet::storage]
	#[pallet::getter(fn staking_lock_period)]
	pub(crate) type StakingLockPeriod<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyIdOf<T>, T::BlockNumber, ValueQuery>;

	#[pallet::event]
	#[pallet::metadata(BalanceOf<T> = "Balance", CurrencyIdOf<T> = "CurrencyId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Minted(T::AccountId, CurrencyIdOf<T>, BalanceOf<T>),
		RedeemStarted(T::AccountId, CurrencyIdOf<T>, BalanceOf<T>, T::BlockNumber),
		UpdateVtokenPoolSuccess,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Account balance must be greater than or equal to the transfer amount.
		BalanceLow,
		/// Balance should be non-zero.
		BalanceZero,
		/// Token type not support.
		NotSupportTokenType,
		/// Empty vtoken pool, cause there's no price at all.
		EmptyVtokenPool,
		/// The amount of token you want to mint is bigger than the vtoken pool.
		NotEnoughVtokenPool,
		/// Calculation Overflow
		CalculationOverflow,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set price for minting vtoken.
		///
		/// The dispatch origin for this call must be `Root` by the
		/// transactor.
		#[pallet::weight(T::WeightInfo::set_token_staking_lock_period())]
		#[transactional]
		pub fn set_token_staking_lock_period(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			locking_blocks: T::BlockNumber,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(token_id.is_token(), Error::<T>::NotSupportTokenType);

			if StakingLockPeriod::<T>::contains_key(token_id) {
				StakingLockPeriod::<T>::mutate(token_id, |locking_period| {
					*locking_period = locking_blocks;
				})
			} else {
				StakingLockPeriod::<T>::insert(token_id, locking_blocks);
			}

			Self::deposit_event(Event::UpdateVtokenPoolSuccess);

			Ok(())
		}

		/// Set staking lock period for a token
		#[pallet::weight(T::WeightInfo::set_vtoken_pool())]
		#[transactional]
		pub fn set_vtoken_pool(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			#[pallet::compact] new_token_pool: BalanceOf<T>,
			#[pallet::compact] new_vtoken_pool: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(token_id.is_token(), Error::<T>::NotSupportTokenType);
			let vtoken_id = token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;

			Self::expand_mint_pool(token_id, new_token_pool)?;
			Self::expand_mint_pool(vtoken_id, new_vtoken_pool)?;

			Self::deposit_event(Event::UpdateVtokenPoolSuccess);

			Ok(())
		}

		/// Mint vtoken.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		#[pallet::weight(T::WeightInfo::mint())]
		#[transactional]
		pub fn mint(
			origin: OriginFor<T>,
			vtoken_id: CurrencyIdOf<T>,
			#[pallet::compact] token_amount: BalanceOf<T>,
		) -> DispatchResult {
			let minter = ensure_signed(origin)?;

			ensure!(!token_amount.is_zero(), Error::<T>::BalanceZero);
			ensure!(vtoken_id.is_vtoken(), Error::<T>::NotSupportTokenType);

			let token_id = vtoken_id.to_token().map_err(|_| Error::<T>::NotSupportTokenType)?;

			let token_balances = T::MultiCurrency::free_balance(token_id, &minter);
			ensure!(token_balances >= token_amount, Error::<T>::BalanceLow);

			// Total amount of tokens.
			let token_pool = Self::get_mint_pool(token_id);
			// Total amount of vtokens.
			let vtoken_pool = Self::get_mint_pool(vtoken_id);
			ensure!(!token_pool.is_zero() && !vtoken_pool.is_zero(), Error::<T>::EmptyVtokenPool);

			let vtokens_buy = token_amount.saturating_mul(vtoken_pool) / token_pool;

			T::MultiCurrency::withdraw(token_id, &minter, token_amount)?;
			T::MultiCurrency::deposit(vtoken_id, &minter, vtokens_buy)?;

			// Alter mint pool
			Self::expand_mint_pool(token_id, token_amount)?;
			Self::expand_mint_pool(vtoken_id, vtokens_buy)?;

			let current_block = <frame_system::Pallet<T>>::block_number();

			// reward mint reward
			let _r = T::MinterReward::reward_minted_vtoken(
				&minter,
				vtoken_id,
				vtokens_buy,
				current_block,
			);

			Self::deposit_event(Event::Minted(minter, vtoken_id, vtokens_buy));

			Ok(())
		}

		/// Redeem token.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		#[pallet::weight(T::WeightInfo::redeem())]
		#[transactional]
		pub fn redeem(
			origin: OriginFor<T>,
			token_id: CurrencyIdOf<T>,
			#[pallet::compact] vtoken_amount: BalanceOf<T>,
		) -> DispatchResult {
			let redeemer = ensure_signed(origin)?;

			ensure!(!vtoken_amount.is_zero(), Error::<T>::BalanceZero);
			ensure!(token_id.is_token(), Error::<T>::NotSupportTokenType);

			ensure!(token_id.is_token(), Error::<T>::NotSupportTokenType);
			let vtoken_id = token_id.to_vtoken().map_err(|_| Error::<T>::NotSupportTokenType)?;

			let vtoken_balances = T::MultiCurrency::free_balance(vtoken_id, &redeemer);
			ensure!(vtoken_balances >= vtoken_amount, Error::<T>::BalanceLow);

			// Reach the end of staking period, begin to redeem.
			// Total amount of tokens.
			let token_pool = Self::get_mint_pool(token_id);
			// Total amount of vtokens.
			let vtoken_pool = Self::get_mint_pool(vtoken_id);
			ensure!(!token_pool.is_zero() && !vtoken_pool.is_zero(), Error::<T>::EmptyVtokenPool);

			let tokens_redeem = vtoken_amount.saturating_mul(token_pool) / vtoken_pool;
			ensure!(
				token_pool >= tokens_redeem && vtoken_pool >= vtoken_amount,
				Error::<T>::NotEnoughVtokenPool
			);

			// Alter redeemer's balance
			T::MultiCurrency::withdraw(vtoken_id, &redeemer, vtoken_amount)?;

			// Alter mint pool
			Self::reduce_mint_pool(token_id, tokens_redeem)?;
			Self::reduce_mint_pool(vtoken_id, vtoken_amount)?;

			Self::update_redeem_record(token_id, &redeemer, tokens_redeem);

			let current_block = <frame_system::Pallet<T>>::block_number();
			Self::deposit_event(Event::RedeemStarted(
				redeemer,
				vtoken_id,
				vtoken_amount,
				current_block,
			));

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(block_number: T::BlockNumber) {
			// Check redeem
			let _ = Self::check_redeem_period(block_number);
		}
	}

	impl<T: Config> Pallet<T> {
		fn update_redeem_record(
			currency_id: CurrencyIdOf<T>,
			who: &T::AccountId,
			amount: BalanceOf<T>,
		) {
			let current_block = <frame_system::Pallet<T>>::block_number();

			if RedeemRecord::<T>::contains_key(who, currency_id) {
				RedeemRecord::<T>::mutate(who, currency_id, |record| {
					record.push((current_block, amount));
				})
			} else {
				let mut new_record = Vec::with_capacity(1);
				new_record.push((current_block, amount));
				RedeemRecord::<T>::insert(who, currency_id, new_record);
			}
		}

		fn check_redeem_period(n: T::BlockNumber) -> DispatchResult {
			for (who, currency_id, records) in RedeemRecord::<T>::iter() {
				let redeem_period = StakingLockPeriod::<T>::get(&currency_id);
				let mut exist_redeem_record = Vec::new();
				for (when, amount) in records.iter().cloned() {
					let rs = n.checked_sub(&when).ok_or(Error::<T>::CalculationOverflow)?;
					if rs >= redeem_period {
						T::MultiCurrency::deposit(currency_id, &who, amount)?;
					} else {
						exist_redeem_record.push((when, amount));
					}
				}
				RedeemRecord::<T>::mutate(who, currency_id, |record| {
					*record = exist_redeem_record;
				});
			}

			Ok(())
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub pools: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		pub staking_lock_period: Vec<(CurrencyIdOf<T>, T::BlockNumber)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			Self { pools: vec![], staking_lock_period: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (currency_id, token_pool) in self.pools.iter() {
				MintPool::<T>::insert(currency_id, token_pool);
			}

			for (currency_id, period) in self.staking_lock_period.iter() {
				StakingLockPeriod::<T>::insert(currency_id, period);
			}
		}
	}
}

impl<T: Config> VtokenMintExt for Pallet<T> {
	type Balance = BalanceOf<T>;
	type CurrencyId = CurrencyIdOf<T>;

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

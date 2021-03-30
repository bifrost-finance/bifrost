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
	transactional, pallet_prelude::*, traits::{Get, Hooks, IsType, Randomness}
};
use frame_system::{
	ensure_root, ensure_signed, pallet_prelude::{OriginFor, BlockNumberFor}
};
use node_primitives::{CurrencyIdExt, CurrencyId, DEXOperations, VtokenMintExt, MinterRewardExt};
use orml_traits::{
	account::MergeAccount, MultiCurrency,
	MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency
};
use sp_runtime::{Permill, traits::{Saturating, Zero}, DispatchResult, ModuleId};

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
		/// A handler to manipulate assets module.
		type MultiCurrency: MergeAccount<Self::AccountId>
			+ MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<Self::AccountId, CurrencyId = CurrencyId>;
	
		/// Event
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Identifier for the staking lock.
		#[pallet::constant]
		type ModuleId: Get<ModuleId>;

		/// Get swap price from zenlink module
		type DEXOperations: DEXOperations<Self::AccountId>;

		/// Record mint reward
		type MinterReward: MinterRewardExt<Self::AccountId, BalanceOf<Self>, CurrencyIdOf<Self>, Self::BlockNumber>;
	
		/// Set default weight.
		type WeightInfo: WeightInfo;

		/// Random source for determinated yield
		type RandomnessSource: Randomness<sp_core::H256>;
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

	/// Referer channels for all users.
	#[pallet::storage]
	#[pallet::getter(fn all_referer_channels)]
	pub(crate) type AllReferrerChannels<T: Config> = StorageValue<
		_,
		(BTreeMap<T::AccountId, BalanceOf<T>>, BalanceOf<T>),
		ValueQuery,
		()
	>;

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
		ValueQuery
	>;

	/// List lock period while staking.
	#[pallet::storage]
	#[pallet::getter(fn staking_lock_period)]
	pub(crate) type StakingLockPeriod<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		T::BlockNumber,
		ValueQuery
	>;

	/// The ROI of each token by every block.
	#[pallet::storage]
	#[pallet::getter(fn rate_of_interest_each_block)]
	pub(crate) type RateOfInterestEachBlock<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		BalanceOf<T>,
		ValueQuery
	>;

	/// Yeild rate for each token
	#[pallet::storage]
	#[pallet::getter(fn yield_rate)]
	pub(crate) type YieldRate<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		CurrencyIdOf<T>,
		Permill,
		ValueQuery
	>;

	#[pallet::event]
	#[pallet::metadata(BalanceOf<T> = "Balance", CurrencyIdOf<T> = "CurrencyId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		UpdateRatePerBlockSuccess,
		Minted(T::AccountId, CurrencyIdOf<T>, BalanceOf<T>),
		RedeemStarted(T::AccountId, CurrencyIdOf<T>, BalanceOf<T>),
		RedeemedPointsSuccess,
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
		/// User's token still under staking while he want to redeem.
		UnderStaking,
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

		/// Mint vtoken.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		#[transactional]
		pub fn mint(
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

			let current_block = <frame_system::Module<T>>::block_number();

			// reward mint reward
			let _ = T::MinterReward::reward_minted_vtoken(&minter, currency_id, vtokens_buy, current_block);

			Self::deposit_event(Event::Minted(minter, currency_id, vtokens_buy));

			Ok(().into())
		}

		/// Redeem token.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		#[transactional]
		pub fn redeem(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			#[pallet::compact] vtoken_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let redeemer = ensure_signed(origin)?;

			ensure!(!vtoken_amount.is_zero(), Error::<T>::BalanceZero);
			ensure!(currency_id.is_token(), Error::<T>::NotSupportTokenType);
			
			// Get paired tokens.
			let (_token_id, vtoken_id) = currency_id
				.get_token_pair()
				.ok_or(Error::<T>::NotSupportTokenType)?;
			
			let vtoken_balances = T::MultiCurrency::free_balance(vtoken_id.into(), &redeemer);
			ensure!(vtoken_balances >= vtoken_amount, Error::<T>::BalanceLow);

			Self::update_redeem_record(currency_id, &redeemer, vtoken_amount);

			Self::deposit_event(Event::Minted(redeemer, currency_id, vtoken_amount));

			Ok(().into())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_block_number: T::BlockNumber) {
			// Mock staking reward for pulling up vtoken price
			let random_sum = Self::mock_yield_change();
			let fluctuation = Permill::from_percent(3); // +- 3%
			for (currency_id, _) in MintPool::<T>::iter() {
				// Only inject tokens into token pool
				let year_rate = YieldRate::<T>::get(&currency_id);
				if year_rate.is_zero() {
					continue;
				}

				let bonus = RateOfInterestEachBlock::<T>::get(&currency_id);
				if currency_id.is_token() {
					if year_rate.deconstruct() % random_sum > random_sum / 2u32 {
						// up to 17.8% or 11.2%
						let rate = year_rate.saturating_add(fluctuation) * bonus;
						let _ = Self::expand_mint_pool(currency_id, rate);
					} else {
						// down to 11.8% or 5.2%
						let rate = year_rate.saturating_sub(fluctuation) * bonus;
						let _ = Self::expand_mint_pool(currency_id, rate);
					}
				}
			}

			// Check redeem
			let _ = Self::check_redeem_period(_block_number);
		}
	}

	/// Mock yield change
	impl<T: Config> Pallet<T> {
		fn update_redeem_record(
			currency_id: CurrencyIdOf<T>,
			who: &T::AccountId,
			amount: BalanceOf<T>,
		) {
			let current_block = <frame_system::Module<T>>::block_number();

			if RedeemRecord::<T>::contains_key(who, currency_id) {
				RedeemRecord::<T>::mutate(who, currency_id, |record| {
					record.push((current_block, amount));
				})
			} else {
				let mut new_recrod = Vec::with_capacity(1);
				new_recrod.push((current_block, amount));
				RedeemRecord::<T>::insert(who, currency_id, new_recrod);
			}
		}

		fn check_redeem_period(n: T::BlockNumber) -> DispatchResult {
			for (who, currency_id, records) in RedeemRecord::<T>::iter() {
				let redeem_period = StakingLockPeriod::<T>::get(&currency_id);
				for (when, amount) in records.iter() {
					if n - *when >= redeem_period {
						// Get paired tokens.
						let (_token_id, vtoken_id) = currency_id
							.get_token_pair()
							.ok_or(Error::<T>::NotSupportTokenType)?;

						// Reach the end of staking period, begin to redeem.
						// Total amount of tokens.
						let token_pool = Self::get_mint_pool(currency_id);
						// Total amount of vtokens.
						let vtoken_pool = Self::get_mint_pool(vtoken_id.into());
						ensure!(
							!token_pool.is_zero() && !vtoken_pool.is_zero(),
							Error::<T>::EmptyVtokenPool
						);

						let tokens_redeem = amount.saturating_mul(token_pool) / vtoken_pool;
						ensure!(
							vtoken_pool >= tokens_redeem && vtoken_pool >= *amount,
							Error::<T>::NotEnoughVtokenPool
						);

						T::MultiCurrency::withdraw(vtoken_id.into(), &who, *amount)?;
						T::MultiCurrency::deposit(currency_id, &who, tokens_redeem)?;

						// Alter mint pool
						Self::reduce_mint_pool(currency_id, tokens_redeem)?;
						Self::reduce_mint_pool(vtoken_id.into(), *amount)?;

						T::MultiCurrency::deposit(currency_id, &who, *amount)?;
					}
				}
			}

			Ok(())
		}

		fn mock_yield_change() -> u32 {
			// Use block number as seed
			let current_block = <frame_system::Module<T>>::block_number();
    		let random_result = T::RandomnessSource::random(&current_block.encode());
			let random_sum = random_result.0.iter().fold(0u32, |acc, x| acc + *x as u32);

			random_sum
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub pools: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		pub staking_lock_period: Vec<(CurrencyIdOf<T>, T::BlockNumber)>,
		pub rate_of_interest_each_block: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
		pub yield_rate: Vec<(CurrencyIdOf<T>, Permill)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			Self {
				pools: vec![],
				staking_lock_period: vec![],
				rate_of_interest_each_block: vec![],
				yield_rate: vec![],
			}
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

			for (currency_id, reward_by_block) in self.rate_of_interest_each_block.iter() {
				RateOfInterestEachBlock::<T>::insert(currency_id, reward_by_block);
			}

			for (currency_id, rate) in self.yield_rate.iter() {
				YieldRate::<T>::insert(currency_id, rate);
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

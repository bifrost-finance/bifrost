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

pub mod primitives;
pub mod rewards;
pub mod weights;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{AccountIdConversion, AtLeast32BitUnsigned, CheckedSub},
		ArithmeticError, FixedPointOperand,
	},
	transactional, PalletId,
};
use frame_system::pallet_prelude::*;
use node_primitives::{CurrencyId, TokenSymbol};
use orml_traits::MultiCurrency;
pub use pallet::*;
pub use primitives::{VstokenConversionExchangeFee, VstokenConversionExchangeRate};
pub use rewards::*;
// use sp_arithmetic::per_things::Percent;
use sp_std::{collections::btree_map::BTreeMap, fmt::Debug, vec::Vec};
pub use weights::WeightInfo;

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
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type MultiCurrency: MultiCurrency<AccountIdOf<Self>, CurrencyId = CurrencyId>;

		type ControlOrigin: EnsureOrigin<Self::Origin>;

		#[pallet::constant]
		type TreasuryAccount: Get<Self::AccountId>;

		#[pallet::constant]
		type VsbondAccount: Get<PalletId>;

		/// Set default weight.
		type WeightInfo: WeightInfo;

		/// The share type of pool.
		type Share: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Debug
			+ FixedPointOperand;

		/// The reward balance type.
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Debug
			+ FixedPointOperand;

		type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord;

		/// ModuleID for creating sub account
		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FarmingPoolCreated {},
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		NotSupportTokenType,
		CalculationOverflow,
		PoolDoesNotExist,
		PoolStateError,
	}

	#[pallet::storage]
	#[pallet::getter(fn pool_next_id)]
	pub type PoolNextId<T: Config> = StorageValue<_, PoolId, ValueQuery>;

	/// Record reward pool info.
	///
	/// map PoolId => PoolInfo
	#[pallet::storage]
	#[pallet::getter(fn pool_infos)]
	pub type PoolInfos<T: Config> = StorageMap<
		_,
		Twox64Concat,
		PoolId,
		PoolInfo<T::Share, BalanceOf<T>, CurrencyIdOf<T>, AccountIdOf<T>>,
		ValueQuery,
	>;

	/// Record share amount, reward currency and withdrawn reward amount for
	/// specific `AccountId` under `PoolId`.
	///
	/// double_map (PoolId, AccountId) => (Share, BTreeMap<CurrencyId, Balance>)
	#[pallet::storage]
	#[pallet::getter(fn shares_and_withdrawn_rewards)]
	pub type SharesAndWithdrawnRewards<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		PoolId,
		Twox64Concat,
		T::AccountId,
		(T::Share, BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>),
		ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[transactional]
		#[pallet::weight(10000)]
		pub fn deposit(origin: OriginFor<T>, pid: PoolId, add_amount: T::Share) -> DispatchResult {
			// Check origin
			let exchanger = ensure_signed(origin)?;

			Self::add_share(&exchanger, pid, add_amount);
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn create_farming_pool(
			origin: OriginFor<T>,
			tokens: BTreeMap<CurrencyIdOf<T>, BalanceOf<T>>,
			basic_reward: BTreeMap<CurrencyIdOf<T>, (BalanceOf<T>, BalanceOf<T>)>,
			/* tokens: BoundedVec<(CurrencyIdOf<T>, u32)>,
			 * basic_reward: BoundedVec<(CurrencyIdOf<T>, Balance)>,
			 * gauge_token: Option<CurrencyIdOf<T>>,
			 * charge_account: AccountIdOf<T>,
			 * #[pallet::compact] min_deposit_to_start: Vec<(CurrencyIdOf<T>, BalanceOf<T>)>,
			 * #[pallet::compact] after_block_to_start: BlockNumberFor<T>,
			 * #[pallet::compact] withdraw_limit_time: BlockNumberFor<T>,
			 * #[pallet::compact] claim_limit_time: BlockNumberFor<T>, */
		) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;
			// let mut d = Asset::<T, I>::get(tokens.keys).ok_or(Error::<T, I>::Unknown)?;

			let mut pid = Self::pool_next_id();
			let keeper = T::PalletId::get().into_sub_account(pid);
			let pool_info = PoolInfo::new(keeper, tokens, basic_reward);
			// PoolInfo { tokens, total_shares: Default::default(), rewards: basic_reward };
			PoolInfos::<T>::insert(pid, &pool_info);
			PoolNextId::<T>::mutate(|id| -> DispatchResult {
				*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::FarmingPoolCreated {});
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn charge(origin: OriginFor<T>, pid: PoolId) -> DispatchResult {
			let exchanger = ensure_signed(origin)?;

			let mut pool_info = Self::pool_infos(&pid);
			ensure!(pool_info.state == PoolState::UnCharged, Error::<T>::PoolStateError);
			match pool_info.keeper {
				None => return Err(Error::<T>::PoolStateError.into()),
				Some(keeper) => {
					pool_info.rewards.iter().for_each(
						|(reward_currency, (total_reward, total_withdrawn_reward))| {
							T::MultiCurrency::transfer(
								*reward_currency,
								&exchanger,
								&keeper,
								*total_reward,
							);
						},
					);
					return Ok(());
				},
			}
			pool_info.state = PoolState::Charged;
			PoolInfos::<T>::insert(&pid, &pool_info);
			// PoolInfos::<T>::mutate(&pid, |pool_info_origin| -> DispatchResult {
			// 	pool_info_origin = pool_info;
			// 	Ok(())
			// })?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn force_retire_pool(origin: OriginFor<T>, pid: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn close_pool(origin: OriginFor<T>, pid: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn reset_pool(origin: OriginFor<T>, pid: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn kill_pool(origin: OriginFor<T>, pid: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn edit_pool(origin: OriginFor<T>, pid: u32) -> DispatchResult {
			T::ControlOrigin::ensure_origin(origin)?;

			Ok(())
		}
	}
}

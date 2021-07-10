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

use core::marker::PhantomData;

use fixed::{types::extra::U0, FixedU128};
#[cfg(feature = "std")]
pub use frame_support::traits::GenesisBuild;
use frame_support::{
	pallet_prelude::{
		Blake2_128Concat, DispatchResultWithPostInfo, IsType, StorageDoubleMap, StorageMap,
		StorageValue, ValueQuery,
	},
	traits::{Get, Hooks},
	PalletId as SystemPalletId, Parameter,
};
use frame_system::pallet_prelude::{ensure_signed, BlockNumberFor, OriginFor};
use node_primitives::{CurrencyId, MinterRewardExt, TokenSymbol};
use orml_traits::{
	currency::TransferAll, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency,
	MultiReservableCurrency,
};
pub use pallet::*;
use sp_runtime::traits::{
	AtLeast32Bit, MaybeSerializeDeserialize, Member, SaturatedConversion, Saturating,
	UniqueSaturatedFrom, Zero,
};
use zenlink_protocol::{AssetId, ExportZenlink};

mod mock;
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub type Fixed = FixedU128<U0>;
	pub type IsExtended = bool;
	pub type BalanceOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;
	pub type CurrencyIdOf<T> = <<T as Config>::MultiCurrency as MultiCurrency<
		<T as frame_system::Config>::AccountId,
	>>::CurrencyId;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// A handler to manipulate assets module.
		type MultiCurrency: TransferAll<Self::AccountId>
			+ MultiCurrencyExtended<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiLockableCurrency<Self::AccountId, CurrencyId = CurrencyId>
			+ MultiReservableCurrency<Self::AccountId, CurrencyId = CurrencyId>;

		/// Event
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Two year as a round, 600 * 24 * 365 * 2
		#[pallet::constant]
		type TwoYear: Get<BlockNumberFor<Self>>;

		/// Reward period, normally it's 50 blocks after.
		#[pallet::constant]
		type RewardPeriod: Get<BlockNumberFor<Self>>;

		/// Allow maximum blocks can be extended.
		#[pallet::constant]
		type MaximumExtendedPeriod: Get<BlockNumberFor<Self>>;

		/// Get price from swap module to compare maximumm vtoken minted
		type ZenlinkOperator: ExportZenlink<Self::AccountId>;

		/// Identifier for adjusting weight
		#[pallet::constant]
		type SystemPalletId: Get<SystemPalletId>;

		type ShareWeight: Member
			+ Parameter
			+ AtLeast32Bit
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Into<BalanceOf<Self>>
			+ From<BalanceOf<Self>>;
	}

	/// How much BNC will be issued to minters each block after.
	#[pallet::storage]
	#[pallet::getter(fn reward_by_one_block)]
	pub(crate) type BNCRewardByOneBlock<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Ieally, BNC reward will be issued after each 50 blocks.
	#[pallet::storage]
	#[pallet::getter(fn current_round_start_at)]
	pub type CurrentRoundStartAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	// BNC reward will be issued by weight calculation.
	#[pallet::storage]
	#[pallet::getter(fn weight)]
	pub type Weights<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyIdOf<T>, T::ShareWeight, ValueQuery>;

	// Total vtoken minted while in one Period
	#[pallet::storage]
	#[pallet::getter(fn total_vtoken_minted)]
	pub type TotalVtokenMinted<T: Config> =
		StorageMap<_, Blake2_128Concat, CurrencyIdOf<T>, BalanceOf<T>, ValueQuery>;
	/// Who mints vtoken
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

	/// Record maximum vtoken value is minted and when minted
	#[pallet::storage]
	#[pallet::getter(fn maximum_vtoken_minted)]
	pub(crate) type MaximumVtokenMinted<T: Config> = StorageValue<
		_,
		// (when, amount, currency _id, extended)
		(BlockNumberFor<T>, BalanceOf<T>, CurrencyIdOf<T>, IsExtended),
		ValueQuery,
	>;

	/// Record a user how much bnc s/he reveives.
	#[pallet::storage]
	#[pallet::getter(fn user_bnc_reward)]
	pub(crate) type UserBNCReward<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	/// Record maximum vtoken value is minted and when minted
	#[pallet::storage]
	#[pallet::getter(fn current_round)]
	pub(crate) type CurrentRound<T: Config> = StorageValue<_, u8, ValueQuery>;

	#[pallet::event]
	#[pallet::metadata(BalanceOf<T> = "Balance", CurrencyIdOf<T> = "CurrencyId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);
	/// No call in this pallet.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn mint(
			origin: OriginFor<T>,
			currency_id: CurrencyIdOf<T>,
			#[pallet::compact] vtoken_amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let minter = ensure_signed(origin)?;

			let current_block = <frame_system::Pallet<T>>::block_number();

			Self::reward_minted_vtoken(&minter, currency_id, vtoken_amount, current_block)?;

			Ok(().into())
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// There's no price at all.
		FailToGetSwapPrice,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			// reach two year
			if n % T::TwoYear::get() == Zero::zero() && n > Zero::zero() {
				// Change round index
				CurrentRound::<T>::mutate(|round| {
					*round += 1u8;
				});
				// cut off half reward next round
				BNCRewardByOneBlock::<T>::mutate(|reward| {
					*reward /= BalanceOf::<T>::from(2u32);
				});
			}
			// if extended,
			// check BNC should be issued or not
			// check reaching the period or not
			let started_block_num = CurrentRoundStartAt::<T>::get();
			if n - started_block_num >= T::RewardPeriod::get() && started_block_num > Zero::zero() {
				// mint period is not extended.
				let (last_max_minted_block, _current_max_minted, _last_currency_id, is_extended) =
					MaximumVtokenMinted::<T>::get();
				// not extended
				if !is_extended {
					// issue BNC reward to minters
					let period =
						BalanceOf::<T>::from(T::RewardPeriod::get().saturated_into::<u32>());
					// let period: BalanceOf<T> = T::RewardPeriod::get().unique_saturated_into();
					let toal_reward = period * BNCRewardByOneBlock::<T>::get();
					Self::issue_bnc_reward(toal_reward);

					// after issued reward, need to clean this round data
					let _ = MaximumVtokenMinted::<T>::kill();
					CurrentRoundStartAt::<T>::put(BlockNumberFor::<T>::from(0u32));
					let _ = Minter::<T>::remove_all(None);
					let _ = TotalVtokenMinted::<T>::remove_all(None);
				} else {
					// mint period is extended
					// two senario need to consider
					if n - last_max_minted_block >= T::RewardPeriod::get() {
						let period =
							BalanceOf::<T>::from((n - started_block_num).saturated_into::<u32>());
						let toal_reward = period * BNCRewardByOneBlock::<T>::get();
						Self::issue_bnc_reward(toal_reward);
						// after issued reward, need to clean this round data
						let _ = MaximumVtokenMinted::<T>::kill();
						CurrentRoundStartAt::<T>::put(BlockNumberFor::<T>::from(0u32));
						let _ = Minter::<T>::remove_all(None);
						let _ = TotalVtokenMinted::<T>::remove_all(None);
					}

					let max_extended_period = T::MaximumExtendedPeriod::get();
					// reaching the MaximumExtendedPeriod, must issue BNC reward.
					if n - started_block_num >= max_extended_period {
						let period =
							BalanceOf::<T>::from(max_extended_period.saturated_into::<u32>());
						let toal_reward = period * BNCRewardByOneBlock::<T>::get();
						Self::issue_bnc_reward(toal_reward);
						// after issued reward, need to clean this round data
						let _ = MaximumVtokenMinted::<T>::kill();
						CurrentRoundStartAt::<T>::put(BlockNumberFor::<T>::from(0u32));
						let _ = Minter::<T>::remove_all(None);
						let _ = TotalVtokenMinted::<T>::remove_all(None);
					}
				}
			}
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub wegiths: Vec<(CurrencyIdOf<T>, T::ShareWeight)>,
		pub reward_by_one_block: BalanceOf<T>,
		pub round_index: u8,
	}
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			Self {
				wegiths: Default::default(),
				reward_by_one_block: Default::default(),
				round_index: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (currency_id, weight) in self.wegiths.iter() {
				Weights::<T>::insert(currency_id, weight);
			}

			CurrentRound::<T>::put(self.round_index);
			BNCRewardByOneBlock::<T>::put(self.reward_by_one_block);
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn compare_max_vtoken_minted(
			currency_id: CurrencyId,
			ausd_amount: BalanceOf<T>,
			block_num: BlockNumberFor<T>,
		) -> Result<(), Error<T>> {
			let _current_block = <frame_system::Pallet<T>>::block_number();
			// let ausd_amount = get_ausd_amount_by_zenlink(minted_vtoken)?;

			let (_last_block, current_max_minted, _last_currency_id, is_extended) =
				MaximumVtokenMinted::<T>::get();
			if ausd_amount > current_max_minted {
				MaximumVtokenMinted::<T>::mutate(|max_minted| {
					if block_num.saturating_sub(CurrentRoundStartAt::<T>::get()) >=
						T::RewardPeriod::get()
					{
						if !is_extended {
							max_minted.3 = true;
						}
					}
					max_minted.0 = block_num;
					max_minted.1 = ausd_amount;
					max_minted.2 = currency_id;
				});
			}

			Ok(())
		}

		pub fn issue_bnc_reward(bnc_reward: BalanceOf<T>) {
			let total_weight: BalanceOf<T> = {
				let mut total: T::ShareWeight = Zero::zero();
				for (_, _weight) in Weights::<T>::iter() {
					total = total.saturating_add(_weight);
				}
				total.into()
			};
			for (minter, currency_id, vtoken_amount) in Minter::<T>::iter() {
				let weight = Weights::<T>::get(&currency_id);
				let total_vtoken_mint = TotalVtokenMinted::<T>::get(currency_id); // AUSD
				let reward = bnc_reward.saturating_mul(weight.into().saturating_mul(vtoken_amount)) /
					(total_weight * total_vtoken_mint);
				let _ = T::MultiCurrency::deposit(
					CurrencyId::Native(TokenSymbol::ASG),
					&minter,
					reward,
				);
				// let _ = T::NativeCurrency::deposit(&minter, reward);

				// Record all BNC rewards the user receives.
				if UserBNCReward::<T>::contains_key(&minter) {
					UserBNCReward::<T>::mutate(&minter, |balance| {
						*balance = balance.saturating_add(reward);
					})
				} else {
					UserBNCReward::<T>::insert(&minter, reward);
				}
			}
		}

		pub fn get_ausd_amount_by_zenlink(
			vtoken_amount: BalanceOf<T>,
			currency_id: CurrencyId,
		) -> Result<BalanceOf<T>, Error<T>> {
			let ausd_amount = T::ZenlinkOperator::get_amount_out_by_path(
				vtoken_amount.saturated_into(),
				&[
					AssetId {
						chain_id: 2001 as u32,
						asset_type: 2,
						asset_index: *currency_id as u32,
					},
					AssetId { chain_id: 2001 as u32, asset_type: 2, asset_index: 2 as u32 },
				],
			)
			.map_err(|_| Error::<T>::FailToGetSwapPrice)?
			.last()
			.copied()
			.ok_or(Error::<T>::FailToGetSwapPrice)?;

			Ok(BalanceOf::<T>::unique_saturated_from(ausd_amount))
		}
	}
}

impl<T: Config> MinterRewardExt<T::AccountId, BalanceOf<T>, CurrencyIdOf<T>, BlockNumberFor<T>>
	for Pallet<T>
{
	type Error = Error<T>;

	fn reward_minted_vtoken(
		minter: &T::AccountId,
		currency_id: CurrencyId,
		minted_vtoken: BalanceOf<T>,
		block_num: BlockNumberFor<T>,
	) -> Result<(), Self::Error> {
		let ausd_amount = Self::get_ausd_amount_by_zenlink(minted_vtoken, currency_id)?;

		// Update minter mint how much vtoken
		if TotalVtokenMinted::<T>::contains_key(currency_id) {
			TotalVtokenMinted::<T>::mutate(currency_id, |total| {
				total.saturating_add(ausd_amount.saturated_into());
			});
		} else {
			TotalVtokenMinted::<T>::insert(currency_id, ausd_amount);
		}

		// check it is a new round
		if CurrentRoundStartAt::<T>::get() == Zero::zero() {
			CurrentRoundStartAt::<T>::put(block_num);
		}

		// Update minter mint how much vtoken
		if Minter::<T>::contains_key(minter, &currency_id) {
			Minter::<T>::mutate(minter, &currency_id, |minted| {
				minted.saturating_add(ausd_amount);
			});
		} else {
			Minter::<T>::insert(minter, &currency_id, ausd_amount);
		}

		Self::compare_max_vtoken_minted(currency_id, ausd_amount, block_num)
	}
}

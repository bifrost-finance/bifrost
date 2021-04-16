// Copyright 2019-2020 Liebi Technologies.
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

pub use self::pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use core::marker::PhantomData;
	use fixed::{types::extra::U0, FixedU128};
	use frame_support::{
		Parameter, debug,
		traits::{Get, Hooks, Currency, ReservableCurrency},
		pallet_prelude::{
			Blake2_128Concat, ensure, StorageMap, StorageValue,
			ValueQuery, StorageDoubleMap
		}
	};
	#[cfg(feature = "std")]
	pub use frame_support::traits::GenesisBuild;
	use frame_system::pallet_prelude::BlockNumberFor;
	use node_primitives::{MintTrait, DEXOperations};
	use sp_runtime::traits::{
		AtLeast32Bit, Member, Saturating, Zero, MaybeSerializeDeserialize, UniqueSaturatedInto
	};

	type Fix = FixedU128<U0>;
	type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The currency trait is going to manpulate balances module.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// BNC price half interval amounts
		#[pallet::constant]
		type PriceHalfBlockInterval: Get<u32>;

		/// BNC issue max block number
		#[pallet::constant]
		type MaxIssueBlockInterval: Get<u32>;

		/// Max transaction amounts
		#[pallet::constant]
		type MaxTxAmount: Get<u32>;

		/// BNC pledge base amounts
		#[pallet::constant]
		type PledgeBaseAmount: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);
	
	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(current_block_number: BlockNumberFor<T>) {
			// Get current block generates bnc stimulate
			let (record_block_number, mut current_bnc_price) = BncPrice::<T>::get();
			let zero_balance: BalanceOf<T> = Zero::zero();
			// Check bnc price
			if current_bnc_price.eq(&zero_balance) { return }

			if current_block_number.saturating_sub(record_block_number)
				.eq(&BlockNumberFor::<T>::from(T::PriceHalfBlockInterval::get())) {
				BncPrice::<T>::mutate (|(record_block_number, bnc_price)| {
					*record_block_number = current_block_number;
					*bnc_price /= BalanceOf::<T>::from(2u32);
				});
				current_bnc_price = BncPrice::<T>::get().1;
			}

			// Bnc stimulate
			Self::count_bnc(current_bnc_price);
			// Obtain monitor data
			let ((previous_block_numer, bnc_mint_amount), max_bnc_mint_amount, tx_amount)
				= BncMonitor::<T>::get();
			
			// Check issue condition
			if current_block_number.saturating_sub(previous_block_numer)
				.eq(&BlockNumberFor::<T>::from(T::MaxIssueBlockInterval::get()))
				&& BncSum::<T>::get().ne(&zero_balance) && max_bnc_mint_amount.ne(&zero_balance)
				|| tx_amount.ge(&T::MaxTxAmount::get())
			{
				// issue bnc
				match Self::issue_bnc_by_weight() {
					Ok(_) => return,
					Err(e) => debug::error!("An error happened while issue bnc : {:?}", e)
				}
			}

			// Update  block_number and max_bnc_mint_amount
			if max_bnc_mint_amount.gt(&bnc_mint_amount) {
				BncMonitor::<T>::mutate(|(tup, _, _)|{
					tup.0 = current_block_number;
					tup.1 = max_bnc_mint_amount;
				});
			}
		}
	}
	
	#[pallet::error]
	pub enum Error<T> {
		/// No included referer
		MinterNotExist,
		/// Bnc total amount is zero
		BncAmountNotExist,
		/// Vtoken not Exist
		AssetScoreNotExist,
		/// pledge amount not enough
		PledgeAmountNotEnough,
		/// Bnc issue fail
		DepositBncFailure,
	}

	#[pallet::storage]
	#[pallet::getter(fn bnc_sum)]
	pub(crate) type BncSum<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;
	
	#[pallet::storage]
	#[pallet::getter(fn bnc_price)]
	pub(crate) type BncPrice<T: Config> = StorageValue<_, (BlockNumberFor<T>, BalanceOf<T>), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bnc_monitor)]
	pub(crate) type BncMonitor<T: Config> = StorageValue<
		_,
		((BlockNumberFor<T>, BalanceOf<T>), BalanceOf<T>, u32), 
		ValueQuery
	>;

	#[pallet::storage]
	#[pallet::getter(fn bnc_mint)]
	pub(crate) type BncMint<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T>,
		ValueQuery
	>;

	#[pallet::storage]
	#[pallet::getter(fn vtoken_weight)]
	pub(crate) type VtokenWeightScore<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AssetId,
		(BalanceOf<T>, BalanceOf<T>),
		ValueQuery
	>;

	#[pallet::storage]
	#[pallet::getter(fn vtoken_mint)]
	pub(crate) type VtokenWeightMint<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AssetId,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T>,
		ValueQuery
	>;

	#[pallet::storage]
	#[pallet::getter(fn storage_version)]
	pub(crate) type StorageVersion<T: Config> = StorageValue<
		_,
		node_primitives::StorageVersion, 
		ValueQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub block_num: BlockNumberFor<T>,
		pub bnc_price: BalanceOf<T>,
	}
	
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig { 
				block_num: Zero::zero(),
				bnc_price: Zero::zero(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			BncPrice::<T>::put((self.block_num, self.bnc_price));
		}
	}

	impl<T: Config> MintTrait<T::AccountId, BalanceOf<T>, T::AssetId> for Pallet<T> {
		type Error = Error<T>;
	
		// Statistics bnc
		fn count_bnc(generate_amount: BalanceOf<T>) {
			BncSum::<T>::mutate(|bnc_amount| {
				*bnc_amount = bnc_amount.saturating_add(generate_amount);
			});
		}
	
		// Settlement model mint
		fn mint_bnc(minter: T::AccountId, mint_amount: BalanceOf<T>) -> Result<(), Self::Error> {
			// Judge
			if BncMint::<T>::contains_key(&minter) {
				BncMint::<T>::mutate(minter, |v| {
					*v = v.saturating_add(mint_amount)
				});
			} else {
				BncMint::<T>::insert(minter, mint_amount);
			}
	
			let (_, max_bnc_amount, _) = BncMonitor::<T>::get();
			if mint_amount.gt(&max_bnc_amount) {
				// Update max_bnc_amount
				BncMonitor::<T>::mutate(|(_, max_val, _)| {
					*max_val = mint_amount;
				})
			}
			BncMonitor::<T>::mutate(|(_, _, tx_amount)| {
				*tx_amount = tx_amount.saturating_add(1);
			});
	
			Ok(())
		}
	
		// Settlement model mint
		fn issue_bnc() -> Result<(), Self::Error> {
			// Check Bnc total amount
			let zero_balance: BalanceOf<T> = Zero::zero();
			let zero_block_number:  BlockNumberFor<T>= Zero::zero();
			ensure!(BncSum::<T>::get().ne(&zero_balance), Error::<T>::BncAmountNotExist);
			let bnc_amount = BncSum::<T>::get();
			// Get total point
			let sum: BalanceOf<T> =
				BncMint::<T>::iter().fold(zero_balance, |acc, x| acc.saturating_add(x.1));
			// Check minter point
			ensure!(sum.ne(&zero_balance), Error::<T>::MinterNotExist);
	
			// Traverse dispatch BNC reward
			for (minter, point) in BncMint::<T>::iter() {
				let minter_reward = point.saturating_mul(bnc_amount) / sum;
				if minter_reward.ne(&zero_balance) {
					ensure!(
						T::Currency::deposit_into_existing(&minter, minter_reward).is_ok(),
						Error::<T>::DepositBncFailure
					);
				}
			}
			// Reset BncSum
			BncSum::<T>::put(zero_balance);
			// Clear BncMint
			for _ in BncMint::<T>::drain() {};
			// Clear Monitor data
			BncMonitor::<T>::put(((zero_block_number, zero_balance), zero_balance, 0u32));
	
			Ok(())
		}
	
		// Currency weight model
		fn v_token_score_exists(asset_id: T::AssetId) -> bool {
			VtokenWeightScore::<T>::contains_key(&asset_id)
		}
	
		fn init_v_token_score(asset_id: T::AssetId, score: BalanceOf<T>) {
			let adjust_score: BalanceOf<T> = Zero::zero();
			VtokenWeightScore::<T>::insert(asset_id, (score, adjust_score));
		}
	
		fn mint_bnc_by_weight(minter: T::AccountId, mint_amount: BalanceOf<T>, asset_id: T::AssetId)
			-> Result<(), Self::Error>
		{
			ensure!(Self::v_token_score_exists(asset_id), Error::<T>::AssetScoreNotExist);
			// Judge
			if VtokenWeightMint::<T>::contains_key(&asset_id, &minter) {
				VtokenWeightMint::<T>::mutate(&asset_id, &minter, |v| {
					*v = v.saturating_add(mint_amount);
				});
			} else {
				VtokenWeightMint::<T>::insert(asset_id, minter, mint_amount);
			}
	
			// Obtain max_bnc_amount
			let (_, max_bnc_amount, _) = BncMonitor::<T>::get();
			if mint_amount.gt(&max_bnc_amount) {
				// Update max_bnc_amount
				BncMonitor::<T>::mutate(|(_, max_val, _)| {
					*max_val = mint_amount;
				})
			}
			BncMonitor::<T>::mutate(|(_, _, tx_amount)| {
				*tx_amount = tx_amount.saturating_add(1);
			});
	
			Ok(())
		}
	
		fn issue_bnc_by_weight() -> Result<(), Self::Error> {
			// Check Bnc total amount
			let zero_balance: BalanceOf<T> = Zero::zero();
			ensure!(BncSum::<T>::get().ne(&zero_balance), Error::<T>::BncAmountNotExist);
			let bnc_amount = BncSum::<T>::get();
			let total_score: BalanceOf<T> = VtokenWeightScore::<T>::iter()
				.fold(zero_balance, |acc, x| acc.saturating_add(x.1.0).saturating_add(x.1.1));
	
			// Traverse
			for (asset_id, (base_score, adjust_score)) in VtokenWeightScore::<T>::iter() {
				let v_token_reward = base_score.saturating_add(adjust_score)
					.saturating_mul(bnc_amount) / total_score;
				// Get v_token point
				let v_token_point: BalanceOf<T> = VtokenWeightMint::<T>::iter_prefix(&asset_id)
					.fold(zero_balance, |acc, x| acc.saturating_add(x.1));
				// Check asset point
				if v_token_point.eq(&zero_balance) { continue }
				// Traverse dispatch BNC reward
				for (minter,point) in VtokenWeightMint::<T>::iter_prefix(asset_id) {
					let minter_reward = point.saturating_mul(v_token_reward) / v_token_point;
					if minter_reward.ne(&zero_balance) {
						ensure!(
							T::Currency::deposit_into_existing(&minter, minter_reward).is_ok(),
							Error::<T>::DepositBncFailure
						);
					}
				}
			}
	
			// Reset BncSum
			BncSum::<T>::put(zero_balance);
			// Clear BncMint
			for _ in VtokenWeightMint::<T>::drain() {};
			// Clear Monitor data
			let zero_block_number: BlockNumberFor<T> = Zero::zero();
			BncMonitor::<T>::put(((zero_block_number, zero_balance), zero_balance, 0u32));
	
			Ok(())
		}
	
		fn improve_v_token_weight(asset_id: T::AssetId, pledge_amount: BalanceOf<T>)
			-> Result<(), Self::Error>
		{
			let base_amount = BalanceOf::<T>::from(T::PledgeBaseAmount::get());
			ensure!(pledge_amount.gt(&base_amount), Error::<T>::PledgeAmountNotEnough);
			// Add weight score
			VtokenWeightScore::<T>::mutate(asset_id, |(_, v)| {
				if let Some(x) = Fix::from_num::<u128>(pledge_amount.saturating_sub(base_amount)
					.unique_saturated_into()).checked_int_log2()
				{
					*v = v.saturating_add(BalanceOf::<T>::from(x as u32));
				}
			});
	
			Ok(())
		}
	
		fn withdraw_v_token_pledge(asset_id: T::AssetId, pledge_amount: BalanceOf<T>)
			-> Result<(), Self::Error>
		{
			let base_amount = BalanceOf::<T>::from(T::PledgeBaseAmount::get());
			ensure!(pledge_amount.gt(&base_amount), Error::<T>::PledgeAmountNotEnough);
			// Reduce weight score
			VtokenWeightScore::<T>::mutate(asset_id, |(_, v)| {
				if let Some(x) = Fix::from_num::<u128>(pledge_amount.saturating_sub(base_amount)
					.unique_saturated_into()).checked_int_log2()
				{
					*v = v.saturating_sub(BalanceOf::<T>::from(x as u32));
				}
			});
	
			Ok(())
		}
	}
}

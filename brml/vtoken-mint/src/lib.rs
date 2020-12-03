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

use frame_support::{Parameter, ensure, decl_module, decl_error, decl_storage};
use node_primitives::MintTrait;
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, MaybeSerializeDeserialize};

mod mock;
mod tests;

pub const INTERVAL: u32 = 10_519_200u32;

pub trait Trait: frame_system::Trait {
	/// The units in which we record balances.
	type Balance: Member
		+ Parameter
		+ AtLeast32Bit
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ From<Self::BlockNumber>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Mint {
		/// bnc total stimulate amount
		BncSum: T::Balance;
		/// record block_number and price for caculate bnc_price
		BncPrice get(fn number_price) config(): (T::BlockNumber, T::Balance);
		/// record block_number and price for issue bnc reward
		BncMonitor: ((T::BlockNumber, T::Balance), T::Balance);
		/// bnc mint
		BncMint get(fn bnc_mint): map hasher(blake2_128_concat) T::AccountId => T::Balance;
		/// bnc reward
		BncReward get(fn bnc_reward): map hasher(blake2_128_concat) T::AccountId => T::Balance;
	}

	add_extra_genesis {
		build(|config: &GenesisConfig<T>| {
			BncPrice::<T>::put(config.number_price);
		});
	}

}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		fn on_finalize(current_block_number: T::BlockNumber) {
			// Get current block generates bnc stimulate
			let (record_block_number, mut current_bnc_price) = BncPrice::<T>::get();
			if current_block_number.saturating_sub(record_block_number).eq(&T::BlockNumber::from(INTERVAL)) {
				BncPrice::<T>::mutate (|(record_block_number, bnc_price)| {
					*record_block_number = current_block_number;
					*bnc_price /= T::Balance::from(2u32);
				});
				current_bnc_price = BncPrice::<T>::get().1;
			}
			// BNC stimulate
			Self::count_bnc(current_bnc_price);
			// Obtain monitor data
			let ((previous_block_numer, bnc_mint_amount), max_bnc_mint_amount) = BncMonitor::<T>::get();
			// Check issue condition
			if current_block_number.saturating_sub(previous_block_numer).eq(&T::BlockNumber::from(50u32))
				&& BncSum::<T>::get().ne(&T::Balance::from(0u32))
				&& max_bnc_mint_amount.ne(&T::Balance::from(0u32)) {
				// issue
				if Self::issue_bnc().is_ok() {
					return;
				}
			}
			// Update  block_number and max_bnc_mint_amount
			if max_bnc_mint_amount.gt(&bnc_mint_amount) {
				BncMonitor::<T>::mutate(|(tup, _)|{
					tup.0 = current_block_number;
					tup.1 = max_bnc_mint_amount;
				});
			}
		}

	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// No included referer
		MinterNotExist,
		/// Bnc total amount is zero
		BncAmountNotExist,
	}
}


impl<T: Trait> MintTrait<T::AccountId, T::Balance> for Module<T> {
	type Error = Error<T>;

	fn count_bnc(generate_amount: T::Balance) {
		BncSum::<T>::mutate(|bnc_amount| {
			*bnc_amount = bnc_amount.saturating_add(generate_amount);
		});
	}

	fn mint_bnc(minter: T::AccountId, mint_amount: T::Balance) -> Result<(), Self::Error> {
		// Judge
		if BncMint::<T>::contains_key(&minter) {
			BncMint::<T>::mutate(minter, |v| {
				*v = v.saturating_add(mint_amount)
			});
		} else {
			BncMint::<T>::insert(minter, mint_amount);
		}
		let (_, max_bnc_amount) = BncMonitor::<T>::get();
		if mint_amount.gt(&max_bnc_amount) {
			// Update max_bnc_amount
			BncMonitor::<T>::mutate(|(_, max_val)| {
				*max_val = mint_amount;
			})
		}

		Ok(())
	}

	fn issue_bnc() -> Result<(), Self::Error> {
		// Check Bnc total amount
		ensure!(BncSum::<T>::get().ne(&T::Balance::from(0u32)), Error::<T>::BncAmountNotExist);
		let bnc_amount = BncSum::<T>::get();
		// Get total point
		let sum: T::Balance =
			BncMint::<T>::iter().fold(T::Balance::from(0u32), |acc, x| acc.saturating_add(x.1));
		// Check minter point
		ensure!(sum.ne(&T::Balance::from(0u32)), Error::<T>::MinterNotExist);
		// Traverse dispatch BNC reward
		for (minter, point) in BncMint::<T>::iter() {
			let bnc_reward = point.saturating_mul(bnc_amount) / sum;
			if bnc_reward.ne(&T::Balance::from(0u32)) {
				if BncReward::<T>::contains_key(&minter) {
					BncReward::<T>::mutate(minter, |v| {
						*v = v.saturating_add(bnc_reward)
					});
				} else {
					BncReward::<T>::insert(&minter, bnc_reward);
				}
			}
		}
		// Reset BncSum
		BncSum::<T>::put(T::Balance::from(0u32));
		// Clear BncMint
		for _ in BncMint::<T>::drain() {};
		// Clear Monitor data
		BncMonitor::<T>::kill();

		Ok(())
	}

	fn query_bnc(minter: T::AccountId) -> Result<T::Balance, Self::Error> {
		// Check
		ensure!(BncMint::<T>::contains_key(&minter), Error::<T>::MinterNotExist);

		Ok(BncMint::<T>::get(&minter))
	}

}


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

use fixed::{types::extra::U4, FixedU128};
type Fix = FixedU128<U4>;

use frame_support::{Parameter, ensure, decl_module, decl_error, decl_storage};
use node_primitives::MintTrait;
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, MaybeSerializeDeserialize};
use sp_runtime::traits::UniqueSaturatedInto;

mod mock;
mod tests;

pub const INTERVAL: u32 = 10_519_200u32;

pub trait Config: frame_system::Config {
	/// The arithmetic type of asset identifier.
	type AssetId: Member
	+ Parameter
	+ AtLeast32Bit
	+ Default
	+ Copy
	+ MaybeSerializeDeserialize;
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
	trait Store for Module<T: Config> as Mint {
		/// bnc total stimulate amount
		BncSum: T::Balance;
		/// record block_number and price for caculate bnc_price
		BncPrice get(fn number_price) config(): (T::BlockNumber, T::Balance);
		/// record block_number and price for issue bnc reward
		BncMonitor: ((T::BlockNumber, T::Balance), T::Balance);
		/// bnc reward
		BncReward get(fn bnc_reward): map hasher(blake2_128_concat) T::AccountId => T::Balance;

		/// bnc mint (apply to settlement model)
		BncMint get(fn bnc_mint): map hasher(blake2_128_concat) T::AccountId => T::Balance;

		/// asset weight (apply to currency weight model)
		VtokenWeightScore get(fn vtoken_weight): map hasher(blake2_128_concat) T::AssetId
			=> T::Balance;
		/// bnc mint by weight (apply to currency weight model)
		VtokenWeightMint get(fn vtoken_mint): double_map hasher(blake2_128_concat) T::AssetId,
			hasher(blake2_128_concat) T::AccountId => T::Balance;
	}

	add_extra_genesis {
		build(|config: &GenesisConfig<T>| {
			BncPrice::<T>::put(config.number_price);
		});
	}

}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {

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

			// Bnc stimulate
			Self::count_bnc(current_bnc_price);
			// Obtain monitor data
			let ((previous_block_numer, bnc_mint_amount), max_bnc_mint_amount) = BncMonitor::<T>::get();
			// Check issue condition
			if current_block_number.saturating_sub(previous_block_numer).eq(&T::BlockNumber::from(50u32))
				&& BncSum::<T>::get().ne(&T::Balance::from(0u32))
				&& max_bnc_mint_amount.ne(&T::Balance::from(0u32)) {
				// issue
				if Self::issue_bnc_by_weight().is_ok() {
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
	pub enum Error for Module<T: Config> {
		/// No included referer
		MinterNotExist,
		/// Bnc total amount is zero
		BncAmountNotExist,
		/// Vtoken not Exist
		AssetScoreNotExist,
		/// pledge amount not enough
		PledgeAmountNotEnough
	}
}

impl<T:Config> Module<T> {
	fn minter_bnc_reward(minter: T::AccountId, bnc_reward: T::Balance) {
		if BncReward::<T>::contains_key(&minter) {
			BncReward::<T>::mutate(minter, |v| {
				*v = v.saturating_add(bnc_reward)
			});
		} else {
			BncReward::<T>::insert(minter, bnc_reward);
		}
	}
}

impl<T: Config> MintTrait<T::AccountId, T::Balance, T::AssetId> for Module<T> {
	type Error = Error<T>;

	// Statistics bnc
	fn count_bnc(generate_amount: T::Balance) {
		BncSum::<T>::mutate(|bnc_amount| {
			*bnc_amount = bnc_amount.saturating_add(generate_amount);
		});
	}

	// Settlement model mint
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

	// Settlement model mint
	fn issue_bnc() -> Result<(), Self::Error> {
		// Check Bnc total amount
		let (zero_balance, zero_block_number) =
			(T::Balance::from(0u32), T::BlockNumber::from(0u32));
		ensure!(BncSum::<T>::get().ne(&zero_balance), Error::<T>::BncAmountNotExist);
		let bnc_amount = BncSum::<T>::get();
		// Get total point
		let sum: T::Balance =
			BncMint::<T>::iter().fold(zero_balance, |acc, x| acc.saturating_add(x.1));
		// Check minter point
		ensure!(sum.ne(&zero_balance), Error::<T>::MinterNotExist);

		// Traverse dispatch BNC reward
		for (minter, point) in BncMint::<T>::iter() {
			let bnc_reward = point.saturating_mul(bnc_amount) / sum;
			if bnc_reward.ne(&zero_balance) {
				Self::minter_bnc_reward(minter, bnc_reward);
			}
		}
		// Reset BncSum
		BncSum::<T>::put(zero_balance);
		// Clear BncMint
		for _ in BncMint::<T>::drain() {};
		// Clear Monitor data
		BncMonitor::<T>::put(((zero_block_number, zero_balance), zero_balance));

		Ok(())
	}

	// Currency weight model
	fn v_token_score_exists(asset_id: T::AssetId) -> bool {
		VtokenWeightScore::<T>::contains_key(&asset_id)
	}

	fn init_v_token_score(asset_id: T::AssetId, score: T::Balance) {
		VtokenWeightScore::<T>::insert(asset_id, score);
	}

	fn mint_bnc_by_weight(minter: T::AccountId, mint_amount: T::Balance, asset_id: T::AssetId)
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
		let (_, max_bnc_amount) = BncMonitor::<T>::get();
		if mint_amount.gt(&max_bnc_amount) {
			// Update max_bnc_amount
			BncMonitor::<T>::mutate(|(_, max_val)| {
				*max_val = mint_amount;
			})
		}

		Ok(())
	}

	fn issue_bnc_by_weight() -> Result<(), Self::Error> {
		// Check Bnc total amount
		ensure!(BncSum::<T>::get().ne(&T::Balance::from(0u32)), Error::<T>::BncAmountNotExist);
		let (bnc_amount, zero) = (BncSum::<T>::get(), T::Balance::from(0u32));
		let total_score: T::Balance = VtokenWeightScore::<T>::iter()
			.fold(zero, |acc, x| acc.saturating_add(x.1));

		// Traverse
		for (asset_id, v_token_score) in VtokenWeightScore::<T>::iter() {
			let vtoken_reward = v_token_score.saturating_mul(bnc_amount) / total_score;
			// Get v_token point
			let vtoken_point: T::Balance = VtokenWeightMint::<T>::iter_prefix(&asset_id)
					.fold(zero, |acc, x| acc.saturating_add(x.1));
			// Check asset point
			if vtoken_point.eq(&zero) { continue; }
			// Traverse dispatch BNC reward
			for (minter,point) in VtokenWeightMint::<T>::iter_prefix(asset_id) {
				let minter_reward = point.saturating_mul(vtoken_reward) / vtoken_point;
				if minter_reward.ne(&zero) {
					Self::minter_bnc_reward(minter, minter_reward);
				}
			}
		}

		// Reset BncSum
		BncSum::<T>::put(zero);
		// Clear BncMint
		for _ in VtokenWeightMint::<T>::drain() {};
		// Clear Monitor data
		BncMonitor::<T>::put(((T::BlockNumber::from(0u32), zero), zero));

		Ok(())
	}

	fn adjust_v_token_weight(asset_id: T::AssetId, pledge_amount: T::Balance)
		-> Result<(), Self::Error>
	{
		let base = T::Balance::from(512u32);
		ensure!(pledge_amount.gt(&base), Error::<T>::PledgeAmountNotEnough);
		VtokenWeightScore::<T>::mutate(asset_id, |v| {
			if let Some(x) = Fix::from_num::<u128>(pledge_amount.saturating_sub(base)
				.unique_saturated_into()).checked_int_log2()
			{
				*v = v.saturating_add(T::Balance::from(x as u32));
			}
		});

		Ok(())
	}
}
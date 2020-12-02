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

extern crate alloc;

use alloc::vec::Vec;
use frame_support::{Parameter, ensure, decl_module, decl_error, decl_storage};
use node_primitives::{RewardTrait, AssetTrait};
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, MaybeSerializeDeserialize};
use codec::{Encode, Decode};

mod mock;
mod tests;

pub trait Trait: frame_system::Trait {
	/// The units in which we record balances.
	type Balance: Member
		+ Parameter
		+ AtLeast32Bit
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ From<Self::BlockNumber>;
	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;
	/// Assets
	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance>;
}

#[derive(Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct RewardRecord<AccountId, Balance> {
	pub account_id: AccountId,
	pub record_amount: Balance,
}

pub const CAPACITY: usize = 512;
pub const LEN: usize = 256;

decl_storage! {
	trait Store for Module<T: Trait> as Reward {
		Point get(fn query_point): map hasher(blake2_128_concat) (T::AssetId, T::AccountId) => T::Balance;
		Reward get(fn vtoken_reward): map hasher(blake2_128_concat) T::AssetId
			=> Vec<RewardRecord<T::AccountId, T::Balance>> = Vec::with_capacity(CAPACITY);
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// No included referer
		RefererNotExist,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}

impl<T: Trait> RewardTrait<T::Balance, T::AccountId, T::AssetId> for Module<T> {
	type Error = Error<T>;
	
	fn record_reward(
		v_token_id: T::AssetId,
		convert_amount: T::Balance,
		referer: T::AccountId
	) -> Result<(), Self::Error> {
		// Traverse (if map doesn't contains v_token_id, the system will be initial)
		Reward::<T>::mutate(v_token_id, |vec| {
			let mut flag = true;
			for item in vec.iter_mut() {
				if item.account_id.eq(&referer) {
					// Update the referer's record_amount
					item.record_amount = item.record_amount.saturating_add(convert_amount);
					flag = false;
					break;
				}
			}
			if flag {
				// Create new account
				let new_referer = RewardRecord::<T::AccountId, T::Balance> {
					account_id: referer.clone(),
					record_amount: convert_amount,
				};
				// Append to vec
				vec.push(new_referer);
			}
			// Sort vec
			vec.sort_by(|a, b| b.record_amount.cmp(&a.record_amount));
		});
		
		Point::<T>::mutate((v_token_id, referer), |val| {
			*val = val.saturating_add(convert_amount);
		});
		
		Ok(())
	}
	
	fn dispatch_reward(
		v_token_id: T::AssetId,
		staking_profit: T::Balance
	) -> Result<(), Self::Error> {
		// Obtain vec
		let record_vec = Self::vtoken_reward(v_token_id);
		ensure!(!record_vec.is_empty(), Error::<T>::RefererNotExist);
		// The total statistics
		let sum: T::Balance = {
			if record_vec.len() >= LEN {
				record_vec[..LEN].iter()
					.fold(T::Balance::from(0u32), |acc, x| acc.saturating_add(x.record_amount))
			} else {
				record_vec.iter()
					.fold(T::Balance::from(0u32), |acc, x| acc.saturating_add(x.record_amount))
			}
		};
		// Dispatch reward
		let length = if record_vec.len() < LEN { record_vec.len() } else { LEN };
		for referer in record_vec[0..length].iter() {
			let reward = referer.record_amount.saturating_mul(staking_profit) / sum;
			// Check dispatch reward
			if reward.ne(&T::Balance::from(0u32)) {
				T::AssetTrait::asset_issue(v_token_id, &referer.account_id, reward);
			}
		}
		// Clear vec and point
		Reward::<T>::mutate(v_token_id, |vec| {
			for item in vec.iter() {
				Point::<T>::remove((v_token_id, &item.account_id));
			}
			vec.clear();
		});
		
		Ok(())
	}
}

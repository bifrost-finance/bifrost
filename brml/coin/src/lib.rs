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

use frame_support::{Parameter, ensure, decl_module, decl_error, decl_storage, dispatch::DispatchResult};
use node_primitives::CoinTrait;
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, MaybeSerializeDeserialize};

mod mock;
mod tests;

pub trait Trait: frame_system::Trait {
	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize + From<Self::BlockNumber>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Reward {
		BncAmount: T::Balance;
		BncCoin get(fn bnc_coin): map hasher(blake2_128_concat) T::AccountId => T::Balance;
		BncReward get(fn bnc_reward): map hasher(blake2_128_concat) T::AccountId => T::Balance;
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// No included referer
		CoinerNotExist
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {}
}



use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::{self, spawn};
use std::time::Duration;

fn get_channel<T>() -> (Sender<T>, Receiver<T>) {
	let (sender, receiver) = channel::<T>();
	(sender, receiver)
}

impl<T: Trait> CoinTrait<T::AccountId, T::Balance> for Module<T> {
	type Error = Error<T>;
	
	// monitor
	fn monitor() -> DispatchResult {
		static mut SECOND:u64 = 50 * 6;
		spawn(|| unsafe {
			let mut max_coin_amount = T::Balance::from(0u32);
			let (_, rx) = get_channel::<T::Balance>();
			for receive_amount in rx {
				if receive_amount.ge(&max_coin_amount) {
					max_coin_amount = receive_amount;
					// Reset Timer
					SECOND = 0;
				}
			}
		});
		//
		unsafe{
			thread::sleep(Duration::from_secs(SECOND));
		}
		Module::<T>::issue_bnc()
	}
	
	
	fn calculate_bnc(generate_amount: T::Balance) -> DispatchResult {
		BncAmount::<T>::mutate(|bnc_amount| {
			*bnc_amount = bnc_amount.saturating_add(generate_amount);
		});
		
		Ok(())
	}
	
	fn coin_bnc(coiner: T::AccountId, coin_amount: T::Balance) -> DispatchResult {
		if BncCoin::<T>::contains_key(&coiner) {
			BncCoin::<T>::mutate(coiner, |v| {
				*v = v.saturating_add(coin_amount)
			});
		} else {
			BncCoin::<T>::insert(coiner, coin_amount);
		}
		
		let (sender, _) = get_channel::<T::Balance>();
		spawn(move || {
			if sender.send(coin_amount).is_err(){};
		});
		
		Ok(())
	}
	
	fn issue_bnc() -> DispatchResult {
		ensure!(BncAmount::<T>::exists(), Error::<T>::CoinerNotExist);
		let bnc_amount = BncAmount::<T>::get();
		// Get total integral
		let mut sum = T::Balance::from(0u32);
		for (_, val) in BncCoin::<T>::iter() {
			sum = sum.saturating_add(val);
		}
		// Traverse dispatch BNC reward
		for (coiner, integral) in BncCoin::<T>::iter() {
			let bnc_reward = integral.saturating_mul(bnc_amount) / sum;
			if bnc_reward.ne(&T::Balance::from(0u32)) {
				if BncReward::<T>::contains_key(&coiner) {
					BncReward::<T>::mutate(coiner, |v| {
						*v = v.saturating_add(bnc_reward)
					});
				} else {
					BncReward::<T>::insert(&coiner, bnc_reward);
				}
			}
		}
		// Clear BncAmount and BncCoin
		BncAmount::<T>::kill();
		for _ in BncCoin::<T>::drain() {};
		
		Ok(())
	}
	
	fn query_bnc(coiner: T::AccountId) -> Result<T::Balance, Self::Error> {
		ensure!(BncCoin::<T>::contains_key(&coiner), Error::<T>::CoinerNotExist);
		Ok(BncCoin::<T>::get(&coiner))
	}
}

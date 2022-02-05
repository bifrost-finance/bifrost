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

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_module, decl_storage, ensure, Parameter};
use node_primitives::{CurrencyId, RewardTrait};
use orml_traits::MultiCurrency;
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Saturating};

mod mock;
mod tests;

pub type CurrencyIdOf<T> = <<T as Config>::CurrenciesHandler as MultiCurrency<
	<T as frame_system::Config>::AccountId,
>>::CurrencyId;

pub trait Config: frame_system::Config {
	/// The units in which we record balances.
	type Balance: Member
		+ Parameter
		+ AtLeast32Bit
		+ Default
		+ Copy
		+ MaybeSerializeDeserialize
		+ From<Self::BlockNumber>;
	/// Assets
	type CurrenciesHandler: MultiCurrency<
		Self::AccountId,
		CurrencyId = CurrencyId,
		Balance = Self::Balance,
	>;
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
	trait Store for Module<T: Config> as Reward {
		Point get(fn query_point): map hasher(blake2_128_concat) (CurrencyIdOf<T>, T::AccountId) => T::Balance;
		Reward get(fn vtoken_reward): map hasher(blake2_128_concat) CurrencyIdOf<T>
			=> Vec<RewardRecord<T::AccountId, T::Balance>> = Vec::with_capacity(CAPACITY);
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		/// No included referer
		RefererNotExist,
		/// Deposit Error
		DepositError,
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {}
}

impl<T: Config> RewardTrait<T::Balance, T::AccountId, CurrencyIdOf<T>> for Module<T> {
	type Error = Error<T>;

	fn record_reward(
		v_token_id: CurrencyIdOf<T>,
		convert_amount: T::Balance,
		referer: T::AccountId,
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
		v_token_id: CurrencyIdOf<T>,
		staking_profit: T::Balance,
	) -> Result<(), Self::Error> {
		// Obtain vec
		let record_vec = Self::vtoken_reward(v_token_id);
		ensure!(!record_vec.is_empty(), Error::<T>::RefererNotExist);
		// The total statistics
		let sum: T::Balance = {
			if record_vec.len() >= LEN {
				record_vec[..LEN]
					.iter()
					.fold(T::Balance::from(0u32), |acc, x| acc.saturating_add(x.record_amount))
			} else {
				record_vec
					.iter()
					.fold(T::Balance::from(0u32), |acc, x| acc.saturating_add(x.record_amount))
			}
		};
		// Dispatch reward
		let length = if record_vec.len() < LEN { record_vec.len() } else { LEN };
		for referer in record_vec[0..length].iter() {
			let reward = referer.record_amount.saturating_mul(staking_profit) / sum;
			// Check dispatch reward
			if reward.ne(&T::Balance::from(0u32)) {
				<<T as Config>::CurrenciesHandler as MultiCurrency<
					<T as frame_system::Config>::AccountId,
				>>::deposit(v_token_id, &referer.account_id, reward)
				.map_err(|_| Error::<T>::DepositError)?;
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

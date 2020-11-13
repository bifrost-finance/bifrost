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

#[macro_use]
extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::btree_map::BTreeMap;
use frame_support::traits::Get;
use frame_support::weights::DispatchClass;
use frame_support::{weights::Weight,Parameter, decl_event, decl_error, decl_module, decl_storage, debug, ensure, StorageValue, IterableStorageMap};
use frame_system::{ensure_root,ensure_signed};
use node_primitives::{AssetTrait, ConvertPool, FetchConvertPrice, FetchConvertPool, AssetReward, TokenSymbol, RewardHandler};
use sp_runtime::traits::{AtLeast32Bit, Member, Saturating, Zero, MaybeSerializeDeserialize};
use codec::{Encode, Decode};
mod mock;
mod tests;

pub trait WeightInfo {
    // accumulate reward
    fn sum_reward()->Weight;
    // Dispatch reward
    fn dispatch_reward()->Weight;
}

impl WeightInfo for (){
    fn sum_reward() -> u64 {
        Default::default()
    }
    fn dispatch_reward() -> u64 {
        Default::default()
    }
}

pub trait Trait: frame_system::Trait {
    /// event
    type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
    /// Set default weight
    type WeightInfo: WeightInfo;
    /// Convert rate
    type ConvertPrice: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance> + MaybeSerializeDeserialize;
    /// The units in which we record balances.
    type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize + From<Self::BlockNumber> + Into<Self::ConvertPrice>;
    /// The arithmetic type of asset identifier.
    type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;
    /// The units in which we record costs.
    type Cost: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;
    /// The units in which we record incomes.
    type Income: Member + Parameter + AtLeast32Bit + Default + Copy + MaybeSerializeDeserialize;
    /// Assets
    type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance, Self::Cost, Self::Income>;
}

decl_event! {
	pub enum Event {
		SumRewardSuccess,
		DispatchRewardSuccess,
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Asset id doesn't exist
		TokenNotExist,
		/// Vtoken id is not equal to token id
		NotSupportaUSD,
		/// vtoken is not exist
		NotSupportVToken,
		/// Dispatch is error
		DispatchFailure
	}
}

#[derive(Encode, Decode, Default, Clone)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct RewardRecord<A,B>{
    pub id: A,
    pub amount: B
}

decl_storage! {
	trait Store for Module<T: Trait> as Rebate {
        Reward get(fn vtoken_reward): map hasher(blake2_128_concat) TokenSymbol
        => Vec<RewardRecord<T::AccountId, T::Balance>> = Vec::with_capacity(256);
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;
        #[weight = T::WeightInfo::sum_reward()]
        fn sum_reward(
            origin,
            vtoken_symbol: TokenSymbol,
            #[compact] convert_amount: T::Balance,
            referer: T::AccountId,
        ) {
            // Verify convertor
            ensure_root(origin)?;
            ensure!(vtoken_symbol != TokenSymbol::aUSD, Error::<T>::NotSupportaUSD);
            ensure!(T::AssetTrait::token_exists(vtoken_symbol), Error::<T>::TokenNotExist);
            // Record referer amount
            let bl = Self::caculate_referer_vtoken(vtoken_symbol,convert_amount,referer);
            ensure!(bl, Error::<T>::NotSupportVToken);
            // Trigger event
            Self::deposit_event(Event::SumRewardSuccess);
        }

        #[weight = T::WeightInfo::dispatch_reward()]
        fn dispatch_reward(
            origin,
            vtoken_symbol: TokenSymbol,
            #[compact] staking_amount: T::Balance,
        ){
            // Verify dispatcher
            ensure_root(origin)?;
            ensure!(vtoken_symbol != TokenSymbol::aUSD, Error::<T>::NotSupportaUSD);
            ensure!(T::AssetTrait::token_exists(vtoken_symbol), Error::<T>::TokenNotExist);
            // Dispatch staking
            let bl = Self::payout_profit(vtoken_symbol,staking_amount);
            ensure!(bl, Error::<T>::DispatchFailure);
            // Trigger event
            Self::deposit_event(Event::DispatchRewardSuccess);
        }
	}
}

impl<T: Trait> Module<T> {
    fn caculate_referer_vtoken (
        vtoken_symbol: TokenSymbol,
        convert_amount: T::Balance,
        referer: T::AccountId,
    ) -> bool {
        // Record table
        // let mut record_vec = Reward::<T>::get(vtoken_symbol);
        let record_vec = Self::vtoken_reward(vtoken_symbol);
        if record_vec.is_empty() {
            return false;
        }
        // Traverse
        Reward::<T>::mutate(vtoken_symbol, |vec| {
            let mut flag = true;
            for item in vec.iter_mut() {
                if item.id.eq(&referer) {
                    item.amount += convert_amount;
                    flag = false;
                    break;
                }
            }
            if flag {
                // Create new account
                let new_referer = RewardRecord::<T::AccountId,T::Balance>{
                    id: referer,
                    amount: convert_amount
                };
                // Append to record_vec
                vec.push(new_referer);
            }
            // Sort record_vec
            vec.sort_by(|a, b| a.amount.partial_cmp(&b.amount).unwrap());
        });
        true
    }

    fn payout_profit(
        vtoken_symbol: TokenSymbol,
        staking_amount: T::Balance,
    ) -> bool {
        let record_vec = Reward::<T>::get(vtoken_symbol);
        if record_vec.is_empty() {
            return false;
        }
        let sum: T::Balance = {
            if record_vec.len() > 256 {
                record_vec[..256].iter().fold(0.into(), |acc, x| acc + x.amount)
            } else {
                record_vec.iter().fold(0.into(), |acc, x| acc + x.amount)
            }
        };
        // Dispatch reward
        let mut length = record_vec.len();
        if length > 256 {
            length = 256
        }
        for i in 0..length {
            // TODO
            let _rebate_money = record_vec[i].amount / sum * staking_amount;
        }

        // Clear vec
        Reward::<T>::mutate(vtoken_symbol, |reward| {
            reward.clear();
        });
        true
    }

}

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

use codec::{Encode, Decode};
use core::convert::{From, Into};
use frame_support::traits::Get;
use frame_support::storage::{StorageMap, IterableStorageMap};
use frame_support::{decl_event, decl_error, decl_module, decl_storage, ensure, Parameter};
use frame_system::{self as system, ensure_root, ensure_signed};
use node_primitives::AssetSymbol;
use sp_runtime::RuntimeDebug;
use sp_runtime::traits::{Member, Saturating, AtLeast32Bit, Zero};

#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct AssetConfig<Balance> {
	redeem_duration: u16,
	reward_per_block: Balance,
}

impl<Balance> AssetConfig<Balance> {
	fn new(redeem_duration: u16, reward_per_block: Balance) -> Self {
		AssetConfig {
			redeem_duration,
			reward_per_block,
		}
	}
}

#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct Validator<Balance, BlockNumber> {
	last_block: BlockNumber,
	deposit: Balance,
	need: Balance,
	current: Balance,
	validator_address: Vec<u8>,
}

impl<Balance: Default, BlockNumber: Default> Validator<Balance, BlockNumber> {
	fn new(need: Balance, validator_address: Vec<u8>) -> Self {
		Self {
			need,
			validator_address,
			..Default::default()
		}
	}
}

pub trait Trait: frame_system::Trait {
	/// event
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy;
	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + From<Self::BlockNumber>;
}

decl_event! {
	pub enum Event<T> where <T as Trait>::Balance {
		AssetConfigSet,
		ValidatorRegistered,
		ValidatorDeposited(AssetSymbol, Balance),
		ValidatorWithdrawn(AssetSymbol, Balance),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		ValidatorRegistered,
		ValidatorNotRegistered,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Xvoting {
		AssetConfigs get(fn asset_configs): map hasher(blake2_128_concat) AssetSymbol => AssetConfig<T::Balance>;
		Validators get(fn validators): map hasher(blake2_128_concat) AssetSymbol => Validator<T::Balance, T::BlockNumber>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = 0]
		fn set_asset(
			origin,
			asset_symbol: AssetSymbol,
			redeem_duration: u16,
			reward_per_block: T::Balance,
		) {
			let _ = ensure_root(origin)?;

			let asset_config = AssetConfig::new(redeem_duration, reward_per_block);
			AssetConfigs::<T>::insert(&asset_symbol, asset_config);

			Self::deposit_event(RawEvent::AssetConfigSet);
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn register(
			origin,
			asset_symbol: AssetSymbol,
			need: T::Balance,
			validator_address: Vec<u8>,
		) {
			let origin = ensure_signed(origin)?;

			ensure!(!Validators::<T>::contains_key(&asset_symbol), Error::<T>::ValidatorRegistered);

			let validator  = Validator::new(need, validator_address);
			Validators::<T>::insert(&asset_symbol, validator);

			Self::deposit_event(RawEvent::ValidatorRegistered);
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn deposit(origin, asset_symbol: AssetSymbol, amount: T::Balance) {
			let _ = ensure_signed(origin)?;

			ensure!(Validators::<T>::contains_key(&asset_symbol), Error::<T>::ValidatorNotRegistered);

			// Lock balance from bridge chain
			// T::Validator::lock(origin, amount);

			Validators::<T>::mutate(asset_symbol, |validator| {
				validator.deposit = validator.deposit.saturating_add(amount);
			});

			Self::deposit_event(RawEvent::ValidatorDeposited(asset_symbol, amount));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn withdraw(origin, asset_symbol: AssetSymbol, amount: T::Balance) {
			let _ = ensure_signed(origin)?;

			ensure!(Validators::<T>::contains_key(&asset_symbol), Error::<T>::ValidatorNotRegistered);

			// UnLock balance from bridge chain
			// T::Validator::unlock(origin, amount);

			Validators::<T>::mutate(&asset_symbol, |validator| {
				validator.deposit = validator.deposit.saturating_sub(amount);
			});

			Self::deposit_event(RawEvent::ValidatorWithdrawn(asset_symbol, amount));
		}

		fn on_finalize(now_block: T::BlockNumber) {
			Self::validator_deduct(now_block);
		}
	}
}

impl<T: Trait> Module<T> {
	fn validator_deduct(now_block: T::BlockNumber) {
		for (key, mut val) in Validators::<T>::iter() {
			// calculate validator's deposit balance
			match key {
				AssetSymbol::DOT => {
					unimplemented!();
				},
				AssetSymbol::KSM => {
					unimplemented!();
				},
				AssetSymbol::EOS => {
					let asset_config = AssetConfigs::<T>::get(&key);

					let redeem_duration = asset_config.redeem_duration;
					let reward_per_block = asset_config.reward_per_block;

					// let account = meta.account;
					// let &mut deposit = val.deposit;
					let need = val.need;
					let current = val.current;

					let redeem_fee = reward_per_block.saturating_mul(reward_per_block);
					if redeem_fee >= val.deposit {
						val.deposit = Zero::zero();
					} else {
						let blocks = now_block - val.last_block;
						val.deposit = val.deposit - reward_per_block.saturating_mul(blocks.into());

						// TOD call redeem from bridge-eos
					}
				},
				_ => {
					unreachable!()
				}
			}

			// update validator
			Validators::<T>::insert(&key, val);
		}
	}
}

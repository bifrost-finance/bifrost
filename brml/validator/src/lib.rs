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

mod mock;
mod tests;

use codec::{Encode, Decode};
use core::convert::{From, Into};
use frame_support::traits::Get;
use frame_support::storage::{StorageMap, IterableStorageDoubleMap};
use frame_support::{decl_event, decl_error, decl_module, decl_storage, ensure, Parameter};
use frame_system::{self as system, ensure_root, ensure_signed};
use node_primitives::{AssetSymbol, AssetTrait, TokenType};
use sp_runtime::RuntimeDebug;
use sp_runtime::traits::{Member, Saturating, AtLeast32Bit, Zero};

#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug)]
pub struct AssetConfig<Balance> {
	redeem_duration: u16,
	min_reward_per_block: Balance,
}

impl<Balance> AssetConfig<Balance> {
	fn new(redeem_duration: u16, min_reward_per_block: Balance) -> Self {
		AssetConfig {
			/// The redeem deration in blocks.
			redeem_duration,
			/// The minimium reward for staking of asset per unit per block.
			min_reward_per_block,
		}
	}
}

#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug)]
pub struct Validator<Balance, BlockNumber> {
	last_block: BlockNumber,
	deposit: Balance,
	need: Balance,
	staking: Balance,
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
	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy + From<Self::BlockNumber>;
	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + From<AssetSymbol>;
	/// The units in which we record costs.
	type Cost: Member + Parameter + AtLeast32Bit + Default + Copy;
	/// The units in which we record incomes.
	type Income: Member + Parameter + AtLeast32Bit + Default + Copy;
	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance, Self::Cost, Self::Income>;
}

decl_event! {
	pub enum Event<T> where
		<T as Trait>::Balance,
		<T as frame_system::Trait>::AccountId,
		<T as frame_system::Trait>::BlockNumber,
	{
		/// A new asset has been set.
		AssetConfigSet(AssetSymbol, AssetConfig<Balance>),
		/// A new validator has been registered.
		ValidatorRegistered(AssetSymbol, AccountId, Validator<Balance, BlockNumber>),
		/// The validator changed the amount of staking it's needed.
		ValidatorNeedAmountSet(AssetSymbol, AccountId, Balance),
		/// The validator deposited the amount of reward.
		ValidatorDeposited(AssetSymbol, AccountId, Balance),
		/// The validator withdrawn the amount of reward.
		ValidatorWithdrawn(AssetSymbol, AccountId, Balance),
		/// The amount of asset staked to the account.
		ValidatorStaked(AssetSymbol, AccountId, Balance),
		/// The amount of asset un-staked from the account.
		ValidatorUnStaked(AssetSymbol, AccountId, Balance),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// The validator has been registered.
		ValidatorRegistered,
		/// The validator has not been registered.
		ValidatorNotRegistered,
		/// The validator's free balance is not enough for locking.
		FreeBalanceNotEnough,
		/// The validator's locked balance is not enough for unlocking.
		LockedBalanceNotEnough,
		/// The staking amount is exceeded the validator's needs.
		StakingAmountExceeded,
		/// The staking amount is insufficient for un-staking.
		StakingAmountInsufficient,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Validator {
		AssetConfigs get(fn asset_configs): map hasher(blake2_128_concat) AssetSymbol => AssetConfig<T::Balance>;
		Validators get(fn validators): double_map hasher(blake2_128_concat) AssetSymbol, hasher(blake2_128_concat) T::AccountId
			=> Validator<T::Balance, T::BlockNumber>;
		LockedBalances get(fn locked_balances): map hasher(blake2_128_concat) T::AccountId => T::Balance;
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
			min_reward_per_block: T::Balance,
		) {
			let _ = ensure_root(origin)?;

			let asset_config = AssetConfig::new(redeem_duration, min_reward_per_block);
			AssetConfigs::<T>::insert(&asset_symbol, &asset_config);

			Self::deposit_event(RawEvent::AssetConfigSet(asset_symbol, asset_config));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn staking(
			origin,
			asset_symbol: AssetSymbol,
			target: T::AccountId,
			amount: T::Balance,
		) {
			let _ = ensure_root(origin)?;
			ensure!(
				Validators::<T>::contains_key(&asset_symbol, &target),
				Error::<T>::ValidatorNotRegistered
			);
			let validator = Validators::<T>::get(&asset_symbol, &target);
			ensure!(
				validator.need - validator.staking >= amount,
				Error::<T>::StakingAmountExceeded,
			);

			Validators::<T>::mutate(&asset_symbol, &target, |validator| {
				validator.staking = validator.staking.saturating_add(amount);
			});

			// TODO stake asset by bridge module

			Self::deposit_event(RawEvent::ValidatorStaked(asset_symbol, target, amount));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn unstaking(
			origin,
			asset_symbol: AssetSymbol,
			target: T::AccountId,
			amount: T::Balance,
		) {
			let _ = ensure_root(origin)?;
			ensure!(
				Validators::<T>::contains_key(&asset_symbol, &target),
				Error::<T>::ValidatorNotRegistered
			);
			let validator = Validators::<T>::get(&asset_symbol, &target);
			ensure!(
				validator.staking >= amount,
				Error::<T>::StakingAmountInsufficient,
			);

			Validators::<T>::mutate(&asset_symbol, &target, |validator| {
				validator.staking = validator.staking.saturating_sub(amount);
			});

			// TODO un-stake asset by bridge module

			Self::deposit_event(RawEvent::ValidatorUnStaked(asset_symbol, target, amount));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn register(
			origin,
			asset_symbol: AssetSymbol,
			need: T::Balance,
			validator_address: Vec<u8>,
		) {
			let origin = ensure_signed(origin)?;

			ensure!(
				!Validators::<T>::contains_key(&asset_symbol, &origin),
				Error::<T>::ValidatorRegistered
			);

			let validator  = Validator::new(need, validator_address);
			Validators::<T>::insert(&asset_symbol, &origin, &validator);

			Self::deposit_event(RawEvent::ValidatorRegistered(asset_symbol, origin, validator));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn set_need_amount(origin, asset_symbol: AssetSymbol, amount: T::Balance) {
			let origin = ensure_signed(origin)?;

			ensure!(
				Validators::<T>::contains_key(&asset_symbol, &origin),
				Error::<T>::ValidatorNotRegistered
			);

			Validators::<T>::mutate(&asset_symbol, &origin, |validator| {
				validator.need = validator.need.saturating_add(amount);
			});

			Self::deposit_event(RawEvent::ValidatorNeedAmountSet(asset_symbol, origin, amount));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn deposit(origin, asset_symbol: AssetSymbol, amount: T::Balance) {
			let origin = ensure_signed(origin)?;

			ensure!(
				Validators::<T>::contains_key(&asset_symbol, &origin),
				Error::<T>::ValidatorNotRegistered
			);

			// Lock balance
			Self::asset_lock(origin.clone(), asset_symbol, amount)?;

			Validators::<T>::mutate(&asset_symbol, &origin, |validator| {
				validator.deposit = validator.deposit.saturating_add(amount);
			});

			Self::deposit_event(RawEvent::ValidatorDeposited(asset_symbol, origin, amount));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn withdraw(origin, asset_symbol: AssetSymbol, amount: T::Balance) {
			let origin = ensure_signed(origin)?;

			ensure!(
				Validators::<T>::contains_key(&asset_symbol, &origin),
				Error::<T>::ValidatorNotRegistered
			);

			// UnLock balance
			Self::asset_unlock(origin.clone(), asset_symbol, amount)?;

			Validators::<T>::mutate(&asset_symbol, &origin, |validator| {
				validator.deposit = validator.deposit.saturating_sub(amount);
			});

			Self::deposit_event(RawEvent::ValidatorWithdrawn(asset_symbol, origin, amount));
		}

		fn on_finalize(now_block: T::BlockNumber) {
			Self::validator_deduct(now_block);
		}
	}
}

impl<T: Trait> Module<T> {
	fn asset_lock(
		account_id: T::AccountId,
		asset_symbol: AssetSymbol,
		amount: T::Balance
	) -> Result<(), Error<T>> {
		// check if has enough balance
		let asset_id: T::AssetId = asset_symbol.into();
		let account_asset = T::AssetTrait::get_account_asset(
			&asset_id,
			TokenType::Token,
			&account_id
		);
		ensure!(account_asset.balance >= amount, Error::<T>::FreeBalanceNotEnough);

		// lock asset to this module
		LockedBalances::<T>::mutate(&account_id, |locked_balance| {
			*locked_balance = locked_balance.saturating_add(amount)
		});

		// destroy asset in assets module
		T::AssetTrait::asset_destroy(
			asset_id,
			TokenType::Token,
			account_id,
			amount
		);

		Ok(())
	}

	fn asset_unlock(
		account_id: T::AccountId,
		asset_symbol: AssetSymbol,
		amount: T::Balance
	) -> Result<(), Error<T>> {
		// check if has enough locked_balance
		ensure!(LockedBalances::<T>::contains_key(&account_id), Error::<T>::LockedBalanceNotEnough);
		ensure!(LockedBalances::<T>::get(&account_id) >= amount, Error::<T>::LockedBalanceNotEnough);

		// unlock asset to this module
		LockedBalances::<T>::mutate(&account_id, |locked_balance| {
			*locked_balance = locked_balance.saturating_sub(amount)
		});

		// issue asset in assets module
		let asset_id: T::AssetId = asset_symbol.into();
		T::AssetTrait::asset_issue(
			asset_id,
			TokenType::Token,
			account_id,
			amount
		);

		Ok(())
	}

	fn validator_deduct(now_block: T::BlockNumber) {
		for (asset_symbol, account_id, mut val) in Validators::<T>::iter() {
			// calculate validator's deposit balance
			match asset_symbol {
				AssetSymbol::DOT => {
					unimplemented!();
				},
				AssetSymbol::KSM => {
					unimplemented!();
				},
				AssetSymbol::EOS => {
					let asset_config = AssetConfigs::<T>::get(&asset_symbol);

					let redeem_duration = asset_config.redeem_duration;
					let min_reward_per_block = asset_config.min_reward_per_block;

					let need = val.need;
					let staking = val.staking;

					let max_fee = min_reward_per_block.saturating_mul(redeem_duration.into());
					let mut fee = Zero::zero();
					if max_fee >= val.deposit {
						fee = val.deposit;
						val.deposit = Zero::zero();
					} else {
						let blocks = now_block - val.last_block;
						fee = min_reward_per_block.saturating_mul(blocks.into());
						val.deposit = val.deposit.saturating_sub(fee);
					}

					val.last_block = now_block;
					// TODO call redeem from bridge-eos
				},
				_ => {
					unreachable!()
				}
			}

			// update validator
			Validators::<T>::insert(&asset_symbol, &account_id, val);
		}
	}
}

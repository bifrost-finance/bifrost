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
use frame_support::{decl_event, decl_error, decl_module, decl_storage, ensure, debug, Parameter};
use frame_system::{self as system, ensure_root, ensure_signed};
use node_primitives::{AssetTrait, BridgeAssetTo, RewardHandler, TokenSymbol};
use sp_runtime::RuntimeDebug;
use sp_runtime::traits::{Member, Saturating, AtLeast32Bit, Zero};
use sp_std::prelude::*;

#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug)]
pub struct AssetConfig<BlockNumber, Balance> {
	redeem_duration: BlockNumber,
	min_reward_per_block: Balance,
}

impl<BlockNumber, Balance> AssetConfig<BlockNumber, Balance> {
	fn new(redeem_duration: BlockNumber, min_reward_per_block: Balance) -> Self {
		AssetConfig {
			/// The redeem deration in blocks.
			redeem_duration,
			/// The minimium reward for staking of asset per unit per block.
			min_reward_per_block,
		}
	}
}

#[derive(Encode, Decode, Clone, Default, PartialEq, Eq, RuntimeDebug)]
pub struct ProxyValidatorRegister<Balance, BlockNumber> {
	last_block: BlockNumber,
	deposit: Balance,
	need: Balance,
	staking: Balance,
	reward_per_block: Balance,
	validator_address: Vec<u8>,
}

impl<Balance: Default, BlockNumber: Default> ProxyValidatorRegister<Balance, BlockNumber> {
	fn new(need: Balance, reward_per_block: Balance, validator_address: Vec<u8>) -> Self {
		Self {
			need,
			validator_address,
			reward_per_block,
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
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy + From<TokenSymbol>;
	/// The units in which we record costs.
	type Cost: Member + Parameter + AtLeast32Bit + Default + Copy;
	/// The units in which we record incomes.
	type Income: Member + Parameter + AtLeast32Bit + Default + Copy;
	/// The units in which we record asset precision.
	type Precision: Member + Parameter + AtLeast32Bit + Default + Copy;
	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance, Self::Cost, Self::Income>;
	type BridgeAssetTo: BridgeAssetTo<Self::AccountId, Self::Precision, Self::Balance>;
	type RewardHandler: RewardHandler<Self::Balance>;
}

decl_event! {
	pub enum Event<T> where
		<T as Trait>::Balance,
		<T as frame_system::Trait>::AccountId,
		<T as frame_system::Trait>::BlockNumber,
	{
		/// A new asset config has been set.
		AssetConfigSet(TokenSymbol, AssetConfig<BlockNumber, Balance>),
		/// A new proxy validator has been registered.
		ProxyValidatorRegistered(TokenSymbol, AccountId, ProxyValidatorRegister<Balance, BlockNumber>),
		/// The proxy validator changed the amount of staking it's needed.
		ProxyValidatorNeedAmountSet(TokenSymbol, AccountId, Balance),
		/// The proxy validator deposited the amount of reward.
		ProxyValidatorDeposited(TokenSymbol, AccountId, Balance),
		/// The proxy validator withdrawn the amount of reward.
		ProxyValidatorWithdrawn(TokenSymbol, AccountId, Balance),
		/// The amount of asset staked to the account.
		ProxyValidatorStaked(TokenSymbol, AccountId, Balance),
		/// The amount of asset un-staked from the account.
		ProxyValidatorUnStaked(TokenSymbol, AccountId, Balance),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// The asset config has not been set.
		AssetConfigNotSet,
		/// The proxy validator has been registered.
		ProxyValidatorRegistered,
		/// The proxy validator has not been registered.
		ProxyValidatorNotRegistered,
		/// The proxy validator's free balance is not enough for locking.
		FreeBalanceNotEnough,
		/// The proxy validator's locked balance is not enough for unlocking.
		LockedBalanceNotEnough,
		/// The staking amount is exceeded the validator's needs.
		StakingAmountExceeded,
		/// The staking amount is insufficient for un-staking.
		StakingAmountInsufficient,
		/// An error occurred in stake action of bridge module.
		BridgeStakeError,
		/// An error occurred in unstake action of bridge module.
		BridgeUnstakeError,
		/// An error while calling redeem by bridge-eos
		BridgeEOSRedeemError,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as ProxyValidator {
		/// Asset config data.
		AssetConfigs get(fn asset_configs): map hasher(blake2_128_concat) TokenSymbol => AssetConfig<T::BlockNumber, T::Balance>;
		/// The total amount of asset has been locked for staking.
		AssetLockedBalances get(fn asset_locked_balances): map hasher(blake2_128_concat) TokenSymbol => T::Balance;
		/// The proxy validators registered from cross chain.
		ProxyValidators get(fn validators): double_map hasher(blake2_128_concat) TokenSymbol, hasher(blake2_128_concat) T::AccountId
			=> ProxyValidatorRegister<T::Balance, T::BlockNumber>;
		/// The locked amount of asset of account for staking.
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
			token_symbol: TokenSymbol,
			redeem_duration: T::BlockNumber,
			min_reward_per_block: T::Balance,
		) {
			let _ = ensure_root(origin)?;

			let asset_config = AssetConfig::new(redeem_duration, min_reward_per_block);
			AssetConfigs::<T>::insert(&token_symbol, &asset_config);

			Self::deposit_event(RawEvent::AssetConfigSet(token_symbol, asset_config));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn stake(
			origin,
			token_symbol: TokenSymbol,
			target: T::AccountId,
			amount: T::Balance,
		) {
			let _ = ensure_root(origin)?;
			ensure!(
				ProxyValidators::<T>::contains_key(&token_symbol, &target),
				Error::<T>::ProxyValidatorNotRegistered
			);
			let validator = ProxyValidators::<T>::get(&token_symbol, &target);
			ensure!(
				validator.need - validator.staking >= amount,
				Error::<T>::StakingAmountExceeded,
			);

			ProxyValidators::<T>::mutate(&token_symbol, &target, |validator| {
				validator.staking = validator.staking.saturating_add(amount);
			});

			AssetLockedBalances::<T>::mutate(&token_symbol, |balance| {
				*balance = balance.saturating_add(amount);
			});

			// stake asset by bridge module
			let validator_address = validator.validator_address;
			T::BridgeAssetTo::stake(token_symbol, amount, validator_address)
				.map_err(|_| Error::<T>::BridgeStakeError)?;

			Self::deposit_event(RawEvent::ProxyValidatorStaked(token_symbol, target, amount));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn unstake(
			origin,
			token_symbol: TokenSymbol,
			target: T::AccountId,
			amount: T::Balance,
		) {
			let _ = ensure_root(origin)?;
			ensure!(
				ProxyValidators::<T>::contains_key(&token_symbol, &target),
				Error::<T>::ProxyValidatorNotRegistered
			);
			let validator = ProxyValidators::<T>::get(&token_symbol, &target);
			ensure!(
				validator.staking >= amount,
				Error::<T>::StakingAmountInsufficient,
			);

			ProxyValidators::<T>::mutate(&token_symbol, &target, |validator| {
				validator.staking = validator.staking.saturating_sub(amount);
			});

			AssetLockedBalances::<T>::mutate(&token_symbol, |balance| {
				*balance = balance.saturating_sub(amount);
			});

			// un-stake asset by bridge module
			let validator_address = validator.validator_address;
			T::BridgeAssetTo::unstake(token_symbol, amount, validator_address)
				.map_err(|_| Error::<T>::BridgeUnstakeError)?;

			Self::deposit_event(RawEvent::ProxyValidatorUnStaked(token_symbol, target, amount));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn register(
			origin,
			token_symbol: TokenSymbol,
			need: T::Balance,
			reward_per_block: T::Balance,
			validator_address: Vec<u8>,
		) {
			let origin = ensure_signed(origin)?;

			ensure!(
				AssetConfigs::<T>::contains_key(&token_symbol),
				Error::<T>::AssetConfigNotSet
			);

			ensure!(
				!ProxyValidators::<T>::contains_key(&token_symbol, &origin),
				Error::<T>::ProxyValidatorRegistered
			);

			let validator = ProxyValidatorRegister::new(need, reward_per_block, validator_address);
			ProxyValidators::<T>::insert(&token_symbol, &origin, &validator);

			Self::deposit_event(RawEvent::ProxyValidatorRegistered(token_symbol, origin, validator));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn set_need_amount(origin, token_symbol: TokenSymbol, amount: T::Balance) {
			let origin = ensure_signed(origin)?;

			ensure!(
				ProxyValidators::<T>::contains_key(&token_symbol, &origin),
				Error::<T>::ProxyValidatorNotRegistered
			);

			ProxyValidators::<T>::mutate(&token_symbol, &origin, |validator| {
				validator.need = amount;
			});

			Self::deposit_event(RawEvent::ProxyValidatorNeedAmountSet(token_symbol, origin, amount));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn deposit(origin, token_symbol: TokenSymbol, amount: T::Balance) {
			let origin = ensure_signed(origin)?;

			ensure!(
				ProxyValidators::<T>::contains_key(&token_symbol, &origin),
				Error::<T>::ProxyValidatorNotRegistered
			);

			// Lock balance
			Self::asset_lock(origin.clone(), token_symbol, amount)?;

			ProxyValidators::<T>::mutate(&token_symbol, &origin, |validator| {
				validator.deposit = validator.deposit.saturating_add(amount);
			});

			Self::deposit_event(RawEvent::ProxyValidatorDeposited(token_symbol, origin, amount));
		}

		#[weight = T::DbWeight::get().writes(1)]
		fn withdraw(origin, token_symbol: TokenSymbol, amount: T::Balance) {
			let origin = ensure_signed(origin)?;

			ensure!(
				ProxyValidators::<T>::contains_key(&token_symbol, &origin),
				Error::<T>::ProxyValidatorNotRegistered
			);

			// UnLock balance
			Self::asset_unlock(origin.clone(), token_symbol, amount)?;

			ProxyValidators::<T>::mutate(&token_symbol, &origin, |validator| {
				validator.deposit = validator.deposit.saturating_sub(amount);
			});

			Self::deposit_event(RawEvent::ProxyValidatorWithdrawn(token_symbol, origin, amount));
		}

		fn on_finalize(now_block: T::BlockNumber) {
			match Self::validator_deduct(now_block) {
				Ok(_) => debug::info!("Deducted successfully."),
				Err(e) => debug::error!("An error happened while deduct: {:?}", e),
			}
		}
	}
}

impl<T: Trait> Module<T> {
	fn asset_lock(
		account_id: T::AccountId,
		token_symbol: TokenSymbol,
		amount: T::Balance
	) -> Result<(), Error<T>> {
		// check if has enough balance
		let account_asset = T::AssetTrait::get_account_asset(token_symbol, &account_id);
		ensure!(account_asset.balance >= amount, Error::<T>::FreeBalanceNotEnough);

		// lock asset to this module
		LockedBalances::<T>::mutate(&account_id, |locked_balance| {
			*locked_balance = locked_balance.saturating_add(amount)
		});

		// destroy asset in assets module
		T::AssetTrait::asset_destroy(token_symbol, account_id, amount);

		Ok(())
	}

	fn asset_unlock(
		account_id: T::AccountId,
		token_symbol: TokenSymbol,
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
		T::AssetTrait::asset_issue(token_symbol, account_id, amount);

		Ok(())
	}

	fn validator_deduct(now_block: T::BlockNumber) -> Result<(), Error<T>> {
		for (token_symbol, account_id, mut val) in ProxyValidators::<T>::iter() {
			// calculate proxy validator's deposit balance
			let asset_config = AssetConfigs::<T>::get(&token_symbol);
			let redeem_duration = asset_config.redeem_duration;
			let min_reward_per_block = asset_config.min_reward_per_block;

			let mut reward = Zero::zero();
			let min_fee = val.staking.saturating_mul(
				min_reward_per_block.saturating_mul(redeem_duration.into())
			);
			if min_fee >= val.deposit {
				// call redeem by bridge-eos
				T::BridgeAssetTo::redeem(token_symbol, val.deposit, val.validator_address.clone()).map_err(|_| Error::<T>::BridgeEOSRedeemError)?;
				reward = val.deposit;
				val.deposit = Zero::zero();
			} else {
				let blocks = now_block - val.last_block;
				reward = val.staking.saturating_mul(
					val.reward_per_block.saturating_mul(blocks.into())
				);
				val.deposit = val.deposit.saturating_sub(reward);
			}

			if reward > Zero::zero() {
				T::RewardHandler::send_reward(reward);
			}

			val.last_block = now_block;

			// update proxy validator
			ProxyValidators::<T>::insert(&token_symbol, &account_id, val);
		}

		Ok(())
	}
}

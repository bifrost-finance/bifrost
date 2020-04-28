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

use core::convert::{From, Into};
use frame_support::{decl_event, decl_error, decl_module, decl_storage, ensure, Parameter, weights::SimpleDispatchInfo};
use frame_system::{self as system, ensure_root, ensure_signed};
use node_primitives::{AssetTrait, AssetSymbol, TokenType};
use sp_runtime::traits::{Member, Saturating, AtLeast32Bit, Zero};

pub trait Trait: frame_system::Trait {
	/// fee
	type Fee: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::TokenPool> + Into<Self::VTokenPool>;

	/// pool size
	type TokenPool: Member + Parameter + AtLeast32Bit + Default + Copy + Into<Self::Balance> + From<Self::Balance>;
	type VTokenPool: Member + Parameter + AtLeast32Bit + Default + Copy + From<Self::Balance> + Into<Self::Balance>;
	type InVariantPool: Member + Parameter + AtLeast32Bit + Default + Copy + From<Self::Balance> + Into<Self::Balance>;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + AtLeast32Bit + Default + Copy;

	/// The units in which we record balances.
	type Balance: Member + Parameter + AtLeast32Bit + Default + Copy;

	/// The units in which we record costs.
	type Cost: Member + Parameter + AtLeast32Bit + Default + Copy;

	/// The units in which we record incomes.
	type Income: Member + Parameter + AtLeast32Bit + Default + Copy;

	type AssetTrait: AssetTrait<Self::AssetId, Self::AccountId, Self::Balance, Self::Cost, Self::Income>;

	/// event
	type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event! {
	pub enum Event {
		AddLiquiditySuccess,
		RemoveLiquiditySuccess,
		UpdateFeeSuccess,
		VTokenToTokenSuccess,
		SwapTokenToVTokenSuccess,
		SwapVTokenToTokenSuccess,
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Asset id doesn't exist
		TokenNotExist,
		/// Amount of input should be less than or equal to origin balance
		InvalidBalanceForTransaction,
		/// Fee doesn't be set
		FeeDoesNotSet,
		/// This is an invalid fee
		InvalidFee,
		/// Vtoken id is not equal to token id
		InvalidTokenPair,
		/// Invalid pool size
		InvalidPoolSize,
		/// If token_pool * vtoken_pool != invariant
		InvalidInvariantValue,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Swap {
		/// fee
		Fee: map hasher(blake2_128_concat) T::AssetId => T::Fee;

		/// the value must meet the requirement: InVariantPool = TokenPool * VTokenPool
		InVariant: map hasher(blake2_128_concat) T::AssetId => (T::TokenPool, T::VTokenPool, T::InVariantPool);
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		#[weight = SimpleDispatchInfo::default()]
		fn set_fee(
			origin,
			token_symbol: AssetSymbol,
			fee: T::Fee
		) {
			ensure_root(origin)?;
			ensure!(!fee.is_zero(), Error::<T>::InvalidFee);

			let token_id = T::AssetId::from(token_symbol as u32);

			ensure!(T::AssetTrait::token_exists(token_id), Error::<T>::TokenNotExist);
			ensure!(fee >= 0.into(), Error::<T>::InvalidFee);
			ensure!(fee <= 100.into(), Error::<T>::InvalidFee);

			<Fee<T>>::insert(token_id, fee);

			Self::deposit_event(Event::UpdateFeeSuccess);
		}

		#[weight = SimpleDispatchInfo::default()]
		fn add_liquidity(
			origin,
			provider: T::AccountId,
			#[compact] token_pool: T::Balance,
			token_symbol: AssetSymbol,
			#[compact] vtoken_pool: T::Balance
		) {
			// only root user has the privilidge to add liquidity
			ensure_root(origin)?;
			ensure!(!vtoken_pool.is_zero() && !token_pool.is_zero(), Error::<T>::InvalidPoolSize);

			let token_id = T::AssetId::from(token_symbol as u32);
			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(token_id), Error::<T>::TokenNotExist);

			// check the balance
			let token_balances = T::AssetTrait::get_account_asset(&token_id, TokenType::Token, &provider).balance;
			ensure!(token_balances >= token_pool, Error::<T>::InvalidBalanceForTransaction);

			let vtoken_balances = T::AssetTrait::get_account_asset(&token_id, TokenType::VToken, &provider).balance;
			ensure!(vtoken_balances >= vtoken_pool, Error::<T>::InvalidBalanceForTransaction);

			// destroy balances from both tokens
			T::AssetTrait::asset_redeem(token_id, TokenType::Token, provider.clone(), token_pool);
			T::AssetTrait::asset_redeem(token_id, TokenType::VToken, provider, vtoken_pool);

			let x: T::InVariantPool = token_pool.into();
			let y: T::InVariantPool = vtoken_pool.into();
			let in_variant: T::InVariantPool = x.saturating_mul(y);
			let x: T::TokenPool = token_pool.into();
			let y: T::VTokenPool = vtoken_pool.into();

			<InVariant<T>>::insert(token_id, (x, y, in_variant));

			Self::deposit_event(Event::AddLiquiditySuccess);
		}

		#[weight = SimpleDispatchInfo::default()]
		fn remove_liquidity(
			origin,
			provider: T::AccountId,
			token_symbol: AssetSymbol
		) {
			// only root user has the privilidge to remove liquidity
			ensure_root(origin)?;
			let token_id = T::AssetId::from(token_symbol as u32);

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(token_id), Error::<T>::TokenNotExist);

			let invariant = <InVariant<T>>::get(token_id);
			let current_token_pool: T::Balance = invariant.0.into();
			let current_vtoken_pool: T::Balance = invariant.1.into();
			let invariant: T::Balance = invariant.2.into();

			ensure!(current_token_pool.saturating_mul(current_vtoken_pool) == invariant, Error::<T>::InvalidInvariantValue);

			T::AssetTrait::asset_issue(token_id, TokenType::Token, provider.clone(), current_token_pool);
			T::AssetTrait::asset_issue(token_id, TokenType::VToken, provider, current_vtoken_pool);

			// update pool
			InVariant::<T>::mutate(token_id, |invariant| {
				invariant.0 = Default::default();
				invariant.1 = Default::default();
				invariant.2 = Default::default();
			});

			Self::deposit_event(Event::RemoveLiquiditySuccess);
		}

		#[weight = SimpleDispatchInfo::default()]
		fn swap_vtoken_to_token(
			origin,
			#[compact] vtoken_amount: T::Balance,
			token_symbol: AssetSymbol
		) {
			ensure!(!vtoken_amount.is_zero(), Error::<T>::InvalidBalanceForTransaction);
			let sender = ensure_signed(origin)?;
			let token_id = T::AssetId::from(token_symbol as u32);

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(token_id), Error::<T>::TokenNotExist);

			// check there's enough balances for transaction
			let vtoken_balances = T::AssetTrait::get_account_asset(&token_id, TokenType::VToken, &sender).balance;
			ensure!(vtoken_balances >= vtoken_amount, Error::<T>::InvalidBalanceForTransaction);

			let invariant = <InVariant<T>>::get(token_id);
			let current_token_pool: T::Balance = invariant.0.into();
			let current_vtoken_pool: T::Balance = invariant.1.into();
			let invariant: T::Balance = invariant.2.into();

			ensure!(current_token_pool.saturating_mul(current_vtoken_pool) == invariant, Error::<T>::InvalidInvariantValue);

			// get fee for both tokens
			// let fee = <Fee<T>>::get(&token_id, &vtoken_id).into();
			// let fee_amount = vtoken_amount * fee.into();

			let new_vtoken_pool = current_vtoken_pool + vtoken_amount;
			ensure!(!new_vtoken_pool.is_zero(), Error::<T>::InvalidPoolSize);
			// let new_token_pool = invariant / (new_vtoken_pool - fee_amount.into());
			let new_token_pool = invariant / new_vtoken_pool;
			let tokens_buy = current_token_pool.saturating_sub(new_token_pool);

			// ensure!(new_vtoken_pool * new_token_pool == invariant, "this is an invalid invariant.");

			T::AssetTrait::asset_destroy(token_id, TokenType::VToken, sender.clone(), vtoken_amount);
			T::AssetTrait::asset_issue(token_id, TokenType::Token, sender, tokens_buy);

			// update pool
			InVariant::<T>::mutate(token_id, |invariant| {
				invariant.0 = new_token_pool.into();
				invariant.1 = new_vtoken_pool.into();
				invariant.2 = (new_vtoken_pool.saturating_mul(new_token_pool)).into();
			});

			Self::deposit_event(Event::SwapVTokenToTokenSuccess);
		}

		#[weight = SimpleDispatchInfo::default()]
		fn swap_token_to_vtoken(
			origin,
			#[compact] token_amount: T::Balance,
			token_symbol: AssetSymbol
		) {
			ensure!(!token_amount.is_zero(), Error::<T>::InvalidBalanceForTransaction);
			let sender = ensure_signed(origin)?;
			let token_id = T::AssetId::from(token_symbol as u32);

			// check asset_id exist or not
			ensure!(T::AssetTrait::token_exists(token_id), Error::<T>::TokenNotExist);

			// check there's enough balances for transaction
			let token_balances = T::AssetTrait::get_account_asset(&token_id, TokenType::Token, &sender).balance;
			ensure!(token_balances >= token_amount, Error::<T>::InvalidBalanceForTransaction);

			let invariant = <InVariant<T>>::get(token_id);
			let current_token_pool: T::Balance = invariant.0.into();
			let current_vtoken_pool: T::Balance = invariant.1.into();
			let invariant: T::Balance = invariant.2.into();

			ensure!(current_token_pool.saturating_mul(current_vtoken_pool) == invariant, Error::<T>::InvalidInvariantValue);

			// get fee for both tokens
			// let fee = <Fee<T>>::get(&token_id, &vtoken_id).into();

			// let fee_amount = token_amount * fee.into();
			let new_token_pool = current_token_pool + token_amount;
			ensure!(!new_token_pool.is_zero(), Error::<T>::InvalidPoolSize);
			// let new_vtoken_pool = invariant / (new_token_pool - fee_amount.into());
			let new_vtoken_pool = invariant / new_token_pool;
			let vtokens_buy = current_vtoken_pool.saturating_sub(new_vtoken_pool);

			// ensure!(new_vtoken_pool * new_token_pool == invariant, "this is an invalid invariant.");

			T::AssetTrait::asset_destroy(token_id, TokenType::Token, sender.clone(), token_amount);
			T::AssetTrait::asset_issue(token_id, TokenType::VToken, sender, vtokens_buy);

			// update pool
			InVariant::<T>::mutate(token_id, |invariant| {
				invariant.0 = new_token_pool.into();
				invariant.1 = new_vtoken_pool.into();
				invariant.2 = (new_vtoken_pool.saturating_mul(new_token_pool)).into();
			});

			Self::deposit_event(Event::SwapTokenToVTokenSuccess);
		}
	}
}

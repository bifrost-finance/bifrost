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
use frame_support::{decl_event, decl_module, decl_storage, ensure, Parameter};
use frame_system::{self as system, ensure_root, ensure_signed};
use node_primitives::{AssetRedeem, TokenType};
use sp_runtime::traits::{Member, Saturating, SimpleArithmetic};

pub trait Trait: assets::Trait {
	/// fee
	type Fee: Member + Parameter + SimpleArithmetic + Default + Copy + Into<Self::TokenPool> + Into<Self::VTokenPool>;

	/// pool size
	type TokenPool: Member + Parameter + SimpleArithmetic + Default + Copy + Into<<Self as assets::Trait>::Balance> + From<<Self as assets::Trait>::Balance>;
	type VTokenPool: Member + Parameter + SimpleArithmetic + Default + Copy + From<<Self as assets::Trait>::Balance> + Into<<Self as assets::Trait>::Balance>;
	type InVariantPool: Member + Parameter + SimpleArithmetic + Default + Copy + From<<Self as assets::Trait>::Balance> + Into<<Self as assets::Trait>::Balance>;

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

decl_storage! {
	trait Store for Module<T: Trait> as Swap {
		/// fee
		Fee: double_map hasher(blake2_256) <T as assets::Trait>::AssetId, hasher(twox_128) <T as assets::Trait>::AssetId
			=> T::Fee;

		/// the value must meet the requirement: InVariantPool = TokenPool * VTokenPool
		InVariant: double_map hasher(blake2_256) <T as assets::Trait>::AssetId, hasher(twox_128) <T as assets::Trait>::AssetId
			=> (T::TokenPool, T::VTokenPool, T::InVariantPool);
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		fn set_fee(
			origin,
			token_id: <T as assets::Trait>::AssetId,
			vtoken_id: <T as assets::Trait>::AssetId,
			fee: T::Fee
		) {
			ensure_root(origin)?;

			ensure!(<assets::Tokens<T>>::exists(token_id), "this token id doesn't exist.");
			ensure!(<assets::Tokens<T>>::exists(vtoken_id), "this vtoken id doesn't exist.");

			ensure!(fee >= 0.into(), "fee cannot be less than 0.");
			ensure!(fee <= 100.into(), "fee cannot be bigger than 100.");

			<Fee<T>>::insert(token_id, vtoken_id, fee);

			Self::deposit_event(Event::UpdateFeeSuccess);
		}

		fn add_liquidity(
			origin,
			provider: T::AccountId,
			token_id: <T as assets::Trait>::AssetId,
			token_pool: T::Balance,
			vtoken_id: <T as assets::Trait>::AssetId,
			vtoken_pool: T::Balance
		) {
			// only root user has the privilidge to add liquidity
			ensure_root(origin)?;

			// check asset_id exist or not
			ensure!(<assets::Tokens<T>>::exists(token_id), "this token id doesn't exists.");
			ensure!(<assets::Tokens<T>>::exists(vtoken_id), "this vtoken id doesn't exists.");

			// check the balance
			let token_balances = <assets::Balances<T>>::get((&token_id, TokenType::Token, &provider));
			ensure!(token_balances >= token_pool, "amount should be less than or equal to origin balance");

			let vtoken_balances = <assets::Balances<T>>::get((&vtoken_id, TokenType::VToken, &provider));
			ensure!(vtoken_balances >= vtoken_pool, "amount should be less than or equal to origin balance");

			// destroy balances from both tokens
			assets::Module::<T>::asset_redeem(token_id, TokenType::Token, provider.clone(), token_pool, None);
			assets::Module::<T>::asset_redeem(vtoken_id, TokenType::VToken, provider, vtoken_pool, None);

			let x: T::InVariantPool = token_pool.into();
			let y: T::InVariantPool = vtoken_pool.into();
			let in_variant: T::InVariantPool = x * y;
			let x: T::TokenPool = token_pool.into();
			let y: T::VTokenPool = vtoken_pool.into();

			<InVariant<T>>::insert(token_id, vtoken_id, (x, y, in_variant));

			Self::deposit_event(Event::AddLiquiditySuccess);
		}

		fn remove_liquidity(
			origin,
			provider: T::AccountId,
			token_id: <T as assets::Trait>::AssetId,
			vtoken_id: <T as assets::Trait>::AssetId
		) {
			// only root user has the privilidge to remove liquidity
			ensure_root(origin)?;

			// check asset_id exist or not
			ensure!(<assets::Tokens<T>>::exists(token_id), "this token id doesn't exists.");
			ensure!(<assets::Tokens<T>>::exists(vtoken_id), "this vtoken id doesn't exists.");

			let invariant = <InVariant<T>>::get(&token_id, &vtoken_id);
			let current_token_pool: T::Balance = invariant.0.into();
			let current_vtoken_pool: T::Balance = invariant.1.into();
			let invariant: T::Balance = invariant.2.into();

			ensure!(current_token_pool * current_vtoken_pool == invariant, "this is an invalid invariant.");

			// transfer
			let to_asset = (&token_id, TokenType::Token, &provider);
			<assets::Balances<T>>::mutate(to_asset, |balances| {
				*balances += current_token_pool;
			});

			let to_asset = (&vtoken_id, TokenType::VToken, &provider);
			<assets::Balances<T>>::mutate(to_asset, |balances| {
				*balances += current_vtoken_pool;
			});

			// update pool
			InVariant::<T>::mutate(&token_id, &vtoken_id, |invariant| {
				invariant.0 = Default::default();
				invariant.1 = Default::default();
				invariant.2 = Default::default();
			});

			Self::deposit_event(Event::RemoveLiquiditySuccess);
		}

		fn swap_vtoken_to_token(
			origin,
			vtoken_amount: T::Balance,
			vtoken_id: <T as assets::Trait>::AssetId,
			token_id: <T as assets::Trait>::AssetId
		) {
			let sender = ensure_signed(origin)?;

			// check asset_id exist or not
			ensure!(<assets::Tokens<T>>::exists(token_id), "this token id is doesn't exist.");
			ensure!(<assets::Tokens<T>>::exists(vtoken_id), "this vtoken id is doesn't exist.");

			// check there's enough balances for transaction
			let vtoken_balances = <assets::Balances<T>>::get((&vtoken_id, TokenType::VToken, &sender));
			ensure!(vtoken_balances >= vtoken_amount, "amount should be less than or equal to origin balance");

			let invariant = <InVariant<T>>::get(&token_id, &vtoken_id);
			let current_token_pool: T::Balance = invariant.0.into();
			let current_vtoken_pool: T::Balance = invariant.1.into();
			let invariant: T::Balance = invariant.2.into();

			ensure!(current_token_pool * current_vtoken_pool == invariant, "this is an invalid invariant.");

			// get fee for both tokens
			// let fee = <Fee<T>>::get(&token_id, &vtoken_id).into();
			// let fee_amount = vtoken_amount * fee.into();

			let new_vtoken_pool = current_vtoken_pool + vtoken_amount;
			// let new_token_pool = invariant / (new_vtoken_pool - fee_amount.into());
			let new_token_pool = invariant / new_vtoken_pool;
			let tokens_buy = current_token_pool - new_token_pool;

			ensure!(new_vtoken_pool * new_token_pool == invariant, "this is an invalid invariant.");

			// vtoken transfer
			let to_asset = (&token_id, TokenType::Token, &sender);
			<assets::Balances<T>>::mutate(to_asset, |balances| {
				*balances += tokens_buy;
			});

			// token decrease
			let to_asset = (&vtoken_id, TokenType::VToken, &sender);
			<assets::Balances<T>>::mutate(to_asset, |balances| {
				*balances -= vtoken_amount;
			});

			// update pool
			InVariant::<T>::mutate(&token_id, &vtoken_id, |invariant| {
				invariant.0 = new_token_pool.into();
				invariant.1 = new_vtoken_pool.into();
			});

			Self::deposit_event(Event::SwapVTokenToTokenSuccess);
		}

		fn swap_token_to_vtoken(
			origin,
			token_amount: T::Balance,
			token_id: <T as assets::Trait>::AssetId,
			vtoken_id: <T as assets::Trait>::AssetId
		) {
			let sender = ensure_signed(origin)?;

			// check asset_id exist or not
			ensure!(<assets::Tokens<T>>::exists(token_id), "this token id is doesn't exist.");
			ensure!(<assets::Tokens<T>>::exists(vtoken_id), "this vtoken id is doesn't exist.");

			// check there's enough balances for transaction
			let token_balances = <assets::Balances<T>>::get((&token_id, TokenType::Token, &sender));
			ensure!(token_balances >= token_amount, "amount should be less than or equal to origin balance");

			let invariant = <InVariant<T>>::get(&token_id, &vtoken_id);
			let current_token_pool: T::Balance = invariant.0.into();
			let current_vtoken_pool: T::Balance = invariant.1.into();
			let invariant: T::Balance = invariant.2.into();

			ensure!(current_token_pool * current_vtoken_pool == invariant, "this is an invalid invariant.");

			// get fee for both tokens
			// let fee = <Fee<T>>::get(&token_id, &vtoken_id).into();

			// let fee_amount = token_amount * fee.into();
			let new_token_pool = current_token_pool + token_amount;
			// let new_vtoken_pool = invariant / (new_token_pool - fee_amount.into());
			let new_vtoken_pool = invariant / new_token_pool;
			let vtokens_buy = current_vtoken_pool - new_vtoken_pool;

			ensure!(new_vtoken_pool * new_token_pool == invariant, "this is an invalid invariant.");

			// transfer
			let to_asset = (&vtoken_id, TokenType::VToken, &sender);
			<assets::Balances<T>>::mutate(to_asset, |balances| {
				*balances += vtokens_buy;
			});

			let to_asset = (&token_id, TokenType::Token, &sender);
			<assets::Balances<T>>::mutate(to_asset, |balances| {
				*balances -= token_amount;
			});

			// update pool
			InVariant::<T>::mutate(&token_id, &vtoken_id, |invariant| {
				invariant.0 = new_token_pool.into();
				invariant.1 = new_vtoken_pool.into();
			});

			Self::deposit_event(Event::SwapTokenToVTokenSuccess);
		}
	}
}

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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use core::convert::TryInto;
use frame_support::{Parameter, decl_module, decl_event, decl_storage, ensure, debug};
use sp_runtime::traits::{Member, SimpleArithmetic, One, Zero, StaticLookup};
use sp_std::prelude::*;
use system::{ensure_signed, ensure_root};
use node_primitives::{ClearingHandler, AssetCreate, AssetIssue, AssetRedeem, Token};

mod mock;
mod tests;

/// The module configuration trait.
pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + SimpleArithmetic + Default + Copy + Zero + From<Self::BlockNumber>;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + SimpleArithmetic + Default + Copy;

	/// Clearing handler for assets change
	type ClearingHandler: ClearingHandler<Self::AssetId, Self::AccountId, Self::BlockNumber, Self::Balance>;

	/// Handler for asset redeem
	type AssetRedeem: AssetRedeem<Self::AssetId, Self::AccountId, Self::Balance>;
}

decl_event!(
	pub enum Event<T>
		where <T as system::Trait>::AccountId,
			<T as Trait>::Balance,
			<T as Trait>::AssetId,
	{
		/// Some assets were created.
		Created(AssetId, Token<Balance>),
		/// Some assets were issued.
		Issued(AssetId, AccountId, Balance),
		/// Some assets were transferred.
		Transferred(AssetId, AccountId, AccountId, Balance),
		/// Some assets were destroyed.
		Destroyed(AssetId, AccountId, Balance),
		/// Bind Asset with AccountId
		AccountAssetCreated(AccountId, AssetId),
		/// Bind Asset with AccountId
		AccountAssetDestroy(AccountId, AssetId),
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Assets {
		/// The number of units of assets held by any given asset ans given account.
		pub Balances get(fn balances): map hasher(blake2_256) (T::AssetId, T::AccountId) => T::Balance;
		/// The next asset identifier up for grabs.
		NextAssetId get(fn next_asset_id): T::AssetId;
		/// Details of the token corresponding to an asset id.
		pub Tokens get(fn token_details): map hasher(blake2_256) T::AssetId => Token<T::Balance>;
		/// A collection of asset which an account owned
		pub AccountAssets get(fn account_assets): map hasher(blake2_256) T::AccountId => Vec<T::AssetId>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {

		fn deposit_event() = default;

		/// Create a new class of fungible assets. It will have an
		/// identifier `AssetId` instance: this will be specified in the `Created` event.
		pub fn create(origin, symbol: Vec<u8>, precision: u16) {
			let _origin = ensure_root(origin)?;

			ensure!(symbol.len() > 0, "token symbol must great then 0");
			ensure!(symbol.len() <= 32, "token symbol cannot exceed 32 bytes");
			ensure!(precision <= 16, "token precision cannot exceed 16");

			let (id, token) = Self::asset_create(symbol, precision);

			Self::deposit_event(RawEvent::Created(id, token));
		}

		/// Issue any amount of fungible assets.
		pub fn issue(origin,
			#[compact] id: T::AssetId,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance)
		{
			let _origin = ensure_root(origin)?;

			ensure!(<Tokens<T>>::exists(&id), "asset should be created first");

			let target = T::Lookup::lookup(target)?;
			ensure!(!amount.is_zero(), "issue amount should be non-zero");

			Self::asset_issue(id, target.clone(), amount);

			Self::deposit_event(RawEvent::Issued(id, target, amount));
		}

		/// Move some assets from one holder to another.
		pub fn transfer(origin,
			#[compact] id: T::AssetId,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance)
		{
			let origin = ensure_signed(origin)?;
			let origin_account = (id, origin.clone());
			let origin_balance = <Balances<T>>::get(&origin_account);
			let target = T::Lookup::lookup(target)?;
			ensure!(!amount.is_zero(), "transfer amount should be non-zero");
			ensure!(origin_balance >= amount,
				"origin account balance must be greater than or equal to the transfer amount");

			Self::asset_transfer(id, origin.clone(), target.clone(), amount);

			Self::deposit_event(RawEvent::Transferred(id, origin, target, amount));
		}

		/// Destroy any amount of assets of `id` owned by `origin`.
		pub fn destroy(origin, #[compact] id: T::AssetId, #[compact] amount: T::Balance) {
			let origin = ensure_signed(origin)?;
			let origin_account = (id, origin.clone());

			let balance = <Balances<T>>::get(&origin_account);
			ensure!(amount <= balance , "amount should be less than or equal to origin balance");

			Self::asset_destroy(id, origin.clone(), amount);

			Self::deposit_event(RawEvent::Destroyed(id, origin, amount));
		}

		pub fn redeem(origin, #[compact] id: T::AssetId, #[compact] amount: T::Balance, to_name: Option<Vec<u8>>) {
			let origin = ensure_signed(origin)?;

			let origin_account = (id, origin.clone());

			let balance = <Balances<T>>::get(&origin_account);
			ensure!(amount <= balance , "amount should be less than or equal to origin balance");

			T::AssetRedeem::asset_redeem(id, origin.clone(), amount, to_name);

			Self::asset_destroy(id, origin.clone(), amount);
		}
	}
}

impl<T: Trait> AssetCreate<T::AssetId, T::Balance> for Module<T> {
	fn asset_create(symbol: Vec<u8>, precision: u16) -> (T::AssetId, Token<T::Balance>) {
		Self::asset_create(symbol, precision)
	}
}

impl<T: Trait> AssetIssue<T::AssetId, T::AccountId, T::Balance> for Module<T> {
	fn asset_issue(asset_id: T::AssetId, target: T::AccountId, amount: T::Balance) {
		Self::asset_issue(asset_id, target, amount);
	}
}

impl<T: Trait> AssetRedeem<T::AssetId, T::AccountId, T::Balance> for Module<T> {
	fn asset_redeem(asset_id: T::AssetId, target: T::AccountId, amount: T::Balance, to_name: Option<Vec<u8>>) {
		Self::asset_destroy(asset_id, target, amount);
	}
}

// The main implementation block for the module.
impl<T: Trait> Module<T> {
	fn asset_create(symbol: Vec<u8>, precision: u16) -> (T::AssetId, Token<T::Balance>) {
		let id = Self::next_asset_id();
		<NextAssetId<T>>::mutate(|id| *id += One::one());

		// Initial total supply is zero.
		let total_supply: T::Balance = 0.into();

		// Create token
		let token = Token::new(symbol, precision, total_supply);

		// Insert to storage
		<Tokens<T>>::insert(id, token.clone());

		(id, token)
	}

	fn asset_issue(
		asset_id: T::AssetId,
		target: T::AccountId,
		amount: T::Balance,
	) {
		let now_block: T::BlockNumber = <system::Module<T>>::block_number();

		let target_asset = (asset_id, target.clone());
		let prev_target_balance = <Balances<T>>::get(&target_asset);
		let target_balance = <Balances<T>>::mutate(&target_asset, |balance| {
			*balance += amount;
			*balance
		});
		T::ClearingHandler::asset_clearing(asset_id, target.clone(), now_block, prev_target_balance, target_balance);

		let prev_token = <Tokens<T>>::get(asset_id);
		let total_supply = <Tokens<T>>::mutate(asset_id, |token| {
			token.total_supply += amount;
			token.total_supply
		});
		T::ClearingHandler::token_clearing(asset_id, now_block, prev_token.total_supply, total_supply);
	}

	fn asset_transfer(
		asset_id: T::AssetId,
		from: T::AccountId,
		to: T::AccountId,
		amount: T::Balance,
	) {
		let now_block: T::BlockNumber = <system::Module<T>>::block_number();

		let from_asset = (asset_id, from.clone());
		let prev_from_balance = <Balances<T>>::get(&from_asset);
		let from_balance = <Balances<T>>::mutate(&from_asset, |balance| {
			*balance -= amount;
			*balance
		});
		T::ClearingHandler::asset_clearing(asset_id, from.clone(), now_block, prev_from_balance, from_balance);

		let to_asset = (asset_id, to.clone());
		let prev_to_balance = <Balances<T>>::get(&to_asset);
		let to_balance = <Balances<T>>::mutate(&to_asset, |balance| {
			*balance += amount;
			*balance
		});
		T::ClearingHandler::asset_clearing(asset_id, to.clone(), now_block, prev_to_balance, to_balance);
	}

	fn asset_destroy(
		asset_id: T::AssetId,
		target: T::AccountId,
		amount: T::Balance,
	) {
		let now_block: T::BlockNumber = <system::Module<T>>::block_number();

		let target_asset = (asset_id, target.clone());
		let prev_target_balance = <Balances<T>>::get(&target_asset);
		let target_balance = <Balances<T>>::mutate(&target_asset, |balance| {
			*balance -= amount;
			*balance
		});
		T::ClearingHandler::asset_clearing(asset_id, target.clone(), now_block, prev_target_balance, target_balance);

		let prev_token = <Tokens<T>>::get(&asset_id);
		let total_supply = <Tokens<T>>::mutate(asset_id, |token| {
			token.total_supply -= amount;
			token.total_supply
		});
		T::ClearingHandler::token_clearing(asset_id, now_block, prev_token.total_supply, total_supply);
	}

	pub fn asset_balances(asset_id: T::AssetId, target: T::AccountId) -> u64 {
		debug::info!("asset id: {:?}, account: {:?}", asset_id, target);
		let origin_account = (asset_id, target);
		let balance_u128 = <Balances<T>>::get(origin_account);

		// balance type is u128, but serde cannot serialize u128.
		// So I have to convert to u64, see this link
		// https://github.com/paritytech/substrate/issues/4641
		let balance_u64: u64 = balance_u128.try_into().unwrap_or(usize::max_value()) as u64;

		balance_u64
	}

	pub fn asset_tokens(target: T::AccountId) -> Vec<T::AssetId> {
		let all_tokens = <AccountAssets<T>>::get(target);

		all_tokens
	}
}

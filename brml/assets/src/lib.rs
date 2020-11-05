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
use frame_support::traits::{Get};
use frame_support::{weights::Weight,Parameter, decl_module, decl_event, decl_error, decl_storage, ensure, dispatch::DispatchResult, IterableStorageMap};
use sp_runtime::traits::{Member, AtLeast32Bit, Saturating, One, Zero, StaticLookup, MaybeSerializeDeserialize};
use sp_std::prelude::*;
use frame_system::{self as system, ensure_signed, ensure_root};
use node_primitives::{
	AccountAsset, AssetRedeem, AssetTrait, FetchConvertPrice, Token, TokenPriceHandler, TokenSymbol,
};

mod mock;
mod tests;

pub trait WeightInfo {
	fn create() -> Weight;
	fn issue() -> Weight;
	fn transfer() -> Weight;
	fn destroy() -> Weight;
	fn redeem() -> Weight;
}

lazy_static::lazy_static! {
	/// (token, precision)
	pub static ref TOKEN_LIST: [(Vec<u8>, u16); 9] = {
		let ausd = (b"aUSD".to_vec(), 18);
		let dot = (b"DOT".to_vec(), 12);
		let vdot = (b"vDOT".to_vec(), 12);
		let ksm = (b"KSM".to_vec(), 12);
		let vksm = (b"vKSM".to_vec(), 12);
		let eos = (b"EOS".to_vec(), 4);
		let veos = (b"vEOS".to_vec(), 4);
		let iost = (b"IOST".to_vec(), 8);
		let viost = (b"vIOST".to_vec(), 8);
		[ausd, dot, vdot, ksm, vksm, eos, veos, iost, viost]
	};
}

/// The module configuration trait.
pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + From<Self::Convert> + MaybeSerializeDeserialize;

	/// The units in which we record prices.
	type Price: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + MaybeSerializeDeserialize;

	/// The units in which we record convert rate.
	type Convert: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + MaybeSerializeDeserialize;

	/// The units in which we record costs.
	type Cost: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + From<Self::Balance> + MaybeSerializeDeserialize;

	/// The units in which we record incomes.
	type Income: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + From<Self::Balance> + MaybeSerializeDeserialize;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + Default + AtLeast32Bit + Copy + From<TokenSymbol> + Into<TokenSymbol> + MaybeSerializeDeserialize;

	/// Handler for asset redeem
	type AssetRedeem: AssetRedeem<Self::AssetId, Self::AccountId, Self::Balance>;

	/// Handler for fetch convert rate from convert runtime
	type FetchConvertPrice: FetchConvertPrice<TokenSymbol, Self::Convert>;

	/// Set default weight
	type WeightInfo: WeightInfo;
}

decl_event! {
	pub enum Event<T>
		where <T as system::Trait>::AccountId,
			<T as Trait>::Balance,
			<T as Trait>::AssetId,
	{
		/// Some assets were created.
		Created(AssetId, Token<Balance>),
		/// Some assets were issued.
		Issued(TokenSymbol, AccountId, Balance),
		/// Some assets were transferred.
		Transferred(TokenSymbol, AccountId, AccountId, Balance),
		/// Some assets were destroyed.
		Destroyed(TokenSymbol, AccountId, Balance),
		/// Bind Asset with AccountId
		AccountAssetCreated(AccountId, AssetId),
		/// Bind Asset with AccountId
		AccountAssetDestroy(AccountId, AssetId),
		/// Unlock user asset
		UnlockedAsset(AccountId, TokenSymbol, Balance),
	}
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Cannot create a token existed
		TokenExisted,
		/// Token symbol is too long
		TokenSymbolTooLong,
		/// if Vec<u8> is empty, meaning this symbol is empty string
		EmptyTokenSymbol,
		/// Precision too big or too small
		InvalidPrecision,
		/// Asset id doesn't exist
		TokenNotExist,
		/// Transaction cannot be made if he amount of balances are 0
		ZeroAmountOfBalance,
		/// Amount of input should be less than or equal to origin balance
		InsufficientBalanceForTransaction,
		/// Convert rate doesn't be set
		ConvertRateDoesNotSet,
		/// This is an invalid convert rate
		InvalidConvertRate,
		/// Vtoken id is not equal to token id
		InvalidTokenPair,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Assets {
		/// The number of units of assets held by any given asset ans given account.
		pub AccountAssets get(fn account_assets) config(): map hasher(blake2_128_concat) (TokenSymbol, T::AccountId)
			=> AccountAsset<T::Balance, T::Cost, T::Income>;
		/// The number of units of prices held by any given asset.
		pub Prices get(fn prices) config(): map hasher(blake2_128_concat) TokenSymbol => T::Price;
		/// The next asset identifier up for grabs.
		pub NextAssetId get(fn next_asset_id) config(): T::AssetId;
		/// Details of the token corresponding to an asset id.
		pub Tokens get(fn token_details) config(): map hasher(blake2_128_concat) TokenSymbol => Token<T::Balance>;
		/// A collection of asset which an account owned
		pub AccountAssetIds get(fn account_asset_ids): map hasher(blake2_128_concat) T::AccountId => Vec<TokenSymbol>;
	}
	add_extra_genesis {
		build(|config: &GenesisConfig<T>| {
			// initalize assets for account
			for ((token_symbol, who), asset) in config.account_assets.iter() {
				<AccountAssets<T>>::insert((token_symbol, who), asset);
			}
			// initialze three assets id for these tokens
			<NextAssetId<T>>::put(config.next_asset_id);

			// now, not support iost, so leave 6 here.
			for i in 0..=8 {
				// initialize token
				let current_token = &TOKEN_LIST[i as usize];
				let token = Token::new(current_token.0.clone(), current_token.1, 0.into());

				let token_symbol = TokenSymbol::from(i as u32);
				<Tokens<T>>::insert(token_symbol, token);

				// initialize price
				<Prices<T>>::insert(token_symbol, T::Price::from(0u32));
			}
		});
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Create a new class of fungible assets. It will have an
		/// identifier `AssetId` instance: this will be specified in the `Created` event.
		#[weight = T::WeightInfo::create()]
		pub fn create(origin, symbol: Vec<u8>, precision: u16) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(!symbol.is_empty(), Error::<T>::EmptyTokenSymbol);
			ensure!(symbol.len() <= 32, Error::<T>::TokenSymbolTooLong);
			ensure!(precision <= 18, Error::<T>::InvalidPrecision); // increase to precision 18

			let (id, token) = Self::asset_create(symbol, precision)?;

			Self::deposit_event(RawEvent::Created(id, token));

			Ok(())
		}

		/// Issue any amount of fungible assets.
		#[weight = T::WeightInfo::issue()]
		pub fn issue(
			origin,
			token_symbol: TokenSymbol,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance,
		) {
			ensure_root(origin)?;

			ensure!(<Tokens<T>>::contains_key(token_symbol), Error::<T>::TokenNotExist);

			let target = T::Lookup::lookup(target)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmountOfBalance);

			Self::asset_issue(token_symbol, &target, amount);

			Self::deposit_event(RawEvent::Issued(token_symbol, target, amount));
		}

		/// Move some assets from one holder to another.
		#[weight = T::WeightInfo::transfer()]
		pub fn transfer(
			origin,
			token_symbol: TokenSymbol,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance,
		) {
			let origin = ensure_signed(origin)?;

			let origin_account = (token_symbol, origin.clone());
			let origin_balance = <AccountAssets<T>>::get(&origin_account).balance;
			let target = T::Lookup::lookup(target)?;

			ensure!(!amount.is_zero(), Error::<T>::ZeroAmountOfBalance);
			ensure!(origin_balance >= amount, Error::<T>::InsufficientBalanceForTransaction);

			Self::asset_transfer(token_symbol, origin.clone(), target.clone(), amount);

			Self::deposit_event(RawEvent::Transferred(token_symbol, origin, target, amount));
		}

		/// Destroy any amount of assets of `id` owned by `origin`.
		#[weight = T::WeightInfo::destroy()]
		pub fn destroy(
			origin,
			token_symbol: TokenSymbol,
			#[compact] amount: T::Balance,
		) {
			let origin = ensure_signed(origin)?;

			let origin_account = (token_symbol, origin.clone());

			let balance = <AccountAssets<T>>::get(&origin_account).balance;
			ensure!(amount <= balance , Error::<T>::InsufficientBalanceForTransaction);

			Self::asset_destroy(token_symbol, &origin, amount);

			Self::deposit_event(RawEvent::Destroyed(token_symbol, origin, amount));
		}

		#[weight = T::WeightInfo::redeem()]
		pub fn redeem(
			origin,
			token_symbol: TokenSymbol,
			#[compact] amount: T::Balance,
			to_name: Option<Vec<u8>>,
		) {
			let origin = ensure_signed(origin)?;

			let origin_account = (token_symbol, origin.clone());

			let balance = <AccountAssets<T>>::get(&origin_account).balance;
			ensure!(amount <= balance , Error::<T>::InsufficientBalanceForTransaction);

			T::AssetRedeem::asset_redeem(token_symbol, origin.clone(), amount, to_name);

			Self::asset_destroy(token_symbol, &origin, amount);
		}

		/// Issue any amount of fungible assets.
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn unlock(
			origin,
			token_symbol: TokenSymbol,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance,
		) {
			ensure_root(origin)?;

			ensure!(<Tokens<T>>::contains_key(token_symbol), Error::<T>::TokenNotExist);

			let target = T::Lookup::lookup(target)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmountOfBalance);

			let locked = <AccountAssets<T>>::get((token_symbol, &target)).locked;
			// ensure this locked amount of balance should be less than his lock balance
			ensure!(locked >= amount, Error::<T>::InsufficientBalanceForTransaction);

			let target_asset = (token_symbol, &target);
			<AccountAssets<T>>::mutate(target_asset, |asset| {
				asset.available = asset.available.saturating_add(amount);
				asset.locked -= amount;
			});

			Self::deposit_event(RawEvent::UnlockedAsset(target, token_symbol, amount));
		}
	}
}

impl<T: Trait> AssetTrait<T::AssetId, T::AccountId, T::Balance, T::Cost, T::Income> for Module<T> {
	type Error = Error<T>;
	fn asset_create(symbol: Vec<u8>, precision: u16) -> Result<(T::AssetId, Token<T::Balance>), Self::Error> {
		for (_, token) in <Tokens<T>>::iter() {
			if token.symbol.eq(&symbol) {
				return Err(Error::<T>::TokenExisted);
			}
		}

		let id = Self::next_asset_id();
		<NextAssetId<T>>::mutate(|id| *id += One::one());

		// Initial total supply is zero.
		let total_supply: T::Balance = 0.into();

		// Create token
		let token = Token::new(symbol.clone(), precision, total_supply);
		let token_symbol: TokenSymbol = id.into();

		// Insert to storage
		<Tokens<T>>::insert(token_symbol, token.clone());

		Ok((id, token))
	}

	fn asset_issue(
		token_symbol: TokenSymbol,
		target: &T::AccountId,
		amount: T::Balance,
	) {
		let convert_rate = T::FetchConvertPrice::fetch_convert_price(token_symbol);
		let target_asset = (token_symbol, target.clone());
		<AccountAssets<T>>::mutate(&target_asset, |asset| {
			asset.balance = asset.balance.saturating_add(amount);
			asset.available = asset.available.saturating_add(amount);
			asset.cost = asset.cost.saturating_add(amount.saturating_mul(convert_rate.into()).into());
		});

		// save asset id for this account
		if <AccountAssetIds<T>>::contains_key(&target) {
			<AccountAssetIds<T>>::mutate(&target, |ids| {
				if !ids.contains(&token_symbol) { // do not push a duplicated asset id to list
					ids.push(token_symbol);
				}
			});
		} else {
			<AccountAssetIds<T>>::insert(&target, vec![token_symbol]);
		}

		<Tokens<T>>::mutate(token_symbol, |token| {
			token.total_supply = token.total_supply.saturating_add(amount);
		});
	}

	fn asset_redeem(
		token_symbol: TokenSymbol,
		target: &T::AccountId,
		amount: T::Balance,
	) {
		Self::asset_destroy(token_symbol, &target, amount);
	}

	fn asset_destroy(
		token_symbol: TokenSymbol,
		target: &T::AccountId,
		amount: T::Balance,
	) {
		let convert_rate = T::FetchConvertPrice::fetch_convert_price(token_symbol);
		let target_asset = (token_symbol, target);
		<AccountAssets<T>>::mutate(target_asset, |asset| {
			asset.balance = asset.balance.saturating_sub(amount);
			asset.available = asset.available.saturating_sub(amount);
			asset.income = asset.income.saturating_add(amount.saturating_mul(convert_rate.into()).into());
		});

		<Tokens<T>>::mutate(token_symbol, |token| {
			token.total_supply = token.total_supply.saturating_sub(amount);
		});
	}

	fn asset_id_exists(who: &T::AccountId, symbol: &[u8], precision: u16) -> Option<TokenSymbol> {
		let all_ids = <AccountAssetIds<T>>::get(who);
		for id in all_ids {
			let token = <Tokens<T>>::get(id);
			if token.symbol.as_slice().eq(symbol) && token.precision.eq(&precision) {
				return Some(id);
			}
		}
		None
	}

	fn token_exists(token_symbol: TokenSymbol) -> bool {
		<Tokens<T>>::contains_key(&token_symbol)
	}

	fn get_account_asset(
		token_symbol: TokenSymbol,
		target: &T::AccountId,
	) -> AccountAsset<T::Balance, T::Cost, T::Income> {
		<AccountAssets<T>>::get((token_symbol, &target))
	}

	fn get_token(token_symbol: TokenSymbol) -> Token<T::Balance> {
		<Tokens<T>>::get(&token_symbol)
	}

	fn lock_asset(who: &T::AccountId, token_symbol: TokenSymbol, locked: T::Balance) {
		let target_asset = (token_symbol, who);
		<AccountAssets<T>>::mutate(target_asset, |asset| {
			asset.locked += locked;
			asset.available = asset.balance.saturating_sub(asset.locked);
		});
	}

	fn unlock_asset(who: &T::AccountId, token_symbol: TokenSymbol, locked: T::Balance) {
		let target_asset = (token_symbol, who);
		<AccountAssets<T>>::mutate(target_asset, |asset| {
			asset.balance = asset.balance.saturating_sub(locked);
			asset.locked -= locked;
		});
	}
}

impl<T: Trait> TokenPriceHandler<T::Price> for Module<T> {
	fn set_token_price(symbol: Vec<u8>, price: T::Price) {
		match TOKEN_LIST.iter().position(|s| s.0 == symbol) {
			Some(id) => {
				let token_symbol = TokenSymbol::from(id as u32 + 1); // skip aUSD
				<Prices<T>>::mutate(token_symbol, |p| *p = price);
			},
			_ => {},
		}
	}
}

// The main implementation block for the module.
impl<T: Trait> Module<T> {
	fn asset_transfer(
		token_symbol: TokenSymbol,
		from: T::AccountId,
		to: T::AccountId,
		amount: T::Balance,
	) {
		let from_asset = (token_symbol, from);
		<AccountAssets<T>>::mutate(&from_asset, |asset| {
			asset.balance = asset.balance.saturating_sub(amount);
			asset.available = asset.available.saturating_sub(amount);
		});

		let to_asset = (token_symbol, &to);
		<AccountAssets<T>>::mutate(to_asset, |asset| {
			asset.balance = asset.balance.saturating_add(amount);
			asset.available = asset.available.saturating_add(amount);
		});

		// save asset id for this account
		if <AccountAssetIds<T>>::contains_key(&to) {
			<AccountAssetIds<T>>::mutate(&to, |ids| {
				// do not push a duplicated asset id to list
				if !ids.contains(&token_symbol) { ids.push(token_symbol); }
			});
		} else {
			<AccountAssetIds<T>>::insert(&to, vec![token_symbol]);
		}
	}

	pub fn asset_balances(token_symbol: TokenSymbol, target: T::AccountId) -> u64 {
		let origin_account = (token_symbol, target);
		let balance_u128 = <AccountAssets<T>>::get(origin_account).balance;

		// balance type is u128, but serde cannot serialize u128.
		// So I have to convert to u64, see this link
		// https://github.com/paritytech/substrate/issues/4641
		let balance_u64: u64 = balance_u128.try_into().unwrap_or(usize::max_value()) as u64;

		balance_u64
	}

	pub fn asset_tokens(target: T::AccountId) -> Vec<TokenSymbol> {
		<AccountAssetIds<T>>::get(target)
	}
}

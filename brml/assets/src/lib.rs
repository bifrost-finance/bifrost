// Copyright 2019-2021 Liebi Technologies.
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
use frame_support::{weights::Weight,Parameter, decl_module, decl_event, decl_error, decl_storage, ensure, dispatch::DispatchResult};
use sp_runtime::traits::{Member, AtLeast32Bit, Saturating, One, Zero, StaticLookup, MaybeSerializeDeserialize};
use sp_std::prelude::*;
use frame_system::{self as system, ensure_signed, ensure_root};
use node_primitives::{
	AccountAsset, AssetRedeem, AssetTrait, FetchVtokenMintPrice, Token, TokenPriceHandler, TokenType,
};

mod mock;
mod tests;

pub trait WeightInfo {
	fn create() -> Weight;
	fn create_pair() -> Weight;
	fn issue() -> Weight;
	fn transfer() -> Weight;
	fn destroy() -> Weight;
	fn redeem() -> Weight;
}

impl WeightInfo for () {
	fn create() -> Weight { Default::default() }
	fn create_pair() -> Weight { Default::default() }
	fn issue() -> Weight { Default::default() }
	fn transfer() -> Weight { Default::default() }
	fn destroy() -> Weight { Default::default() }
	fn redeem() -> Weight { Default::default() }
}

/// The module configuration trait.
pub trait Config: system::Config {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + From<Self::VtokenMint> + MaybeSerializeDeserialize;

	/// The units in which we record prices.
	type Price: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + MaybeSerializeDeserialize;

	/// The units in which we record vtoken mint rate.
	type VtokenMint: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + MaybeSerializeDeserialize;

	/// The arithmetic type of asset identifier.
	type AssetId: Member + Parameter + Default + AtLeast32Bit + Copy + MaybeSerializeDeserialize;

	/// Handler for asset redeem
	type AssetRedeem: AssetRedeem<Self::AssetId, Self::AccountId, Self::Balance>;

	/// Handler for fetch vtoken mint rate from vtoken mint runtime
	type FetchVtokenMintPrice: FetchVtokenMintPrice<Self::AssetId, Self::VtokenMint>;

	/// Set default weight
	type WeightInfo: WeightInfo;
}

decl_event! {
	pub enum Event<T>
		where <T as system::Config>::AccountId,
			<T as Config>::Balance,
			<T as Config>::AssetId,
	{
		/// Some assets were created.
		Created(AssetId, Token<AssetId, Balance>),
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
		/// Unlock user asset
		UnlockedAsset(AccountId, AssetId, Balance),
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
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
		/// vtoken mint rate doesn't be set
		VtokenMintRateDoesNotSet,
		/// This is an invalid vtoken mint rate
		InvalidVtokenMintRate,
		/// Vtoken id is not equal to token id
		InvalidTokenPair,
	}
}

decl_storage! {
	trait Store for Module<T: Config> as Assets {
		/// The number of units of assets held by any given asset ans given account.
		pub AccountAssets get(fn account_assets) config(): map hasher(blake2_128_concat) (T::AssetId, T::AccountId)
			=> AccountAsset<T::Balance>;
		/// The number of units of prices held by any given asset.
		pub Prices get(fn prices): map hasher(blake2_128_concat) T::AssetId => T::Price;
		/// The next asset identifier up for grabs.
		pub NextAssetId get(fn next_asset_id): T::AssetId;
		/// Details of the token corresponding to an asset id.
		pub Tokens get(fn token_details) config(): map hasher(blake2_128_concat) T::AssetId => Token<T::AssetId, T::Balance>;
		/// A collection of asset which an account owned
		pub AccountAssetIds get(fn account_asset_ids): map hasher(blake2_128_concat) T::AccountId => Vec<T::AssetId>;
	}
	add_extra_genesis {
		build(|config: &GenesisConfig<T>| {
			// initialize assets for account
			for ((asset_id, who), asset) in config.account_assets.iter() {
				<AccountAssets<T>>::insert((asset_id, who), asset);
			}
			// initialize tokens
			for (asset_id, token) in config.token_details.iter() {
				assert!(*asset_id == <NextAssetId<T>>::get());
				if token.token_type == TokenType::Token {
					<Module<T>>::asset_create_pair(token.symbol.clone(), token.precision).unwrap();
				} else {
					<Module<T>>::asset_create(token.symbol.clone(), token.precision, token.token_type).unwrap();
				}
			}
		});
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Create a new class of fungible assets. It will have an
		/// identifier `AssetId` instance: this will be specified in the `Created` event.
		#[weight = T::WeightInfo::create()]
		pub fn create(origin, symbol: Vec<u8>, precision: u16, token_type: TokenType) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(!symbol.is_empty(), Error::<T>::EmptyTokenSymbol);
			ensure!(symbol.len() <= 32, Error::<T>::TokenSymbolTooLong);
			ensure!(precision <= 18, Error::<T>::InvalidPrecision); // increase to precision 18

			let (id, token) = Self::asset_create(symbol, precision, token_type)?;

			Self::deposit_event(RawEvent::Created(id, token));

			Ok(())
		}

		#[weight = T::WeightInfo::create_pair()]
		pub fn create_pair(origin, symbol: Vec<u8>, precision: u16) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(!symbol.is_empty(), Error::<T>::EmptyTokenSymbol);
			ensure!(symbol.len() <= 32, Error::<T>::TokenSymbolTooLong);
			ensure!(precision <= 18, Error::<T>::InvalidPrecision);

			let (token_id, v_token_id) = Self::asset_create_pair(symbol, precision)?;

			Self::deposit_event(RawEvent::Created(token_id, Self::get_token(token_id)));
			Self::deposit_event(RawEvent::Created(v_token_id, Self::get_token(v_token_id)));

			Ok(())
		}

		/// Issue any amount of fungible assets.
		#[weight = T::WeightInfo::issue()]
		pub fn issue(
			origin,
			asset_id: T::AssetId,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance,
		) {
			ensure_root(origin)?;

			ensure!(<Tokens<T>>::contains_key(asset_id), Error::<T>::TokenNotExist);

			let target = T::Lookup::lookup(target)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmountOfBalance);

			Self::asset_issue(asset_id, &target, amount);

			Self::deposit_event(RawEvent::Issued(asset_id, target, amount));
		}

		/// Move some assets from one holder to another.
		#[weight = T::WeightInfo::transfer()]
		pub fn transfer(
			origin,
			asset_id: T::AssetId,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance,
		) {
			let origin = ensure_signed(origin)?;

			let origin_account = (asset_id, origin.clone());
			let origin_balance = <AccountAssets<T>>::get(&origin_account).balance;
			let target = T::Lookup::lookup(target)?;

			ensure!(!amount.is_zero(), Error::<T>::ZeroAmountOfBalance);
			ensure!(origin_balance >= amount, Error::<T>::InsufficientBalanceForTransaction);

			Self::asset_transfer(asset_id, origin.clone(), target.clone(), amount);

			Self::deposit_event(RawEvent::Transferred(asset_id, origin, target, amount));
		}

		/// Destroy any amount of assets of `id` owned by `origin`.
		#[weight = T::WeightInfo::destroy()]
		pub fn destroy(
			origin,
			asset_id: T::AssetId,
			#[compact] amount: T::Balance,
		) {
			let origin = ensure_signed(origin)?;

			let origin_account = (asset_id, origin.clone());

			let balance = <AccountAssets<T>>::get(&origin_account).balance;
			ensure!(amount <= balance , Error::<T>::InsufficientBalanceForTransaction);

			Self::asset_destroy(asset_id, &origin, amount);

			Self::deposit_event(RawEvent::Destroyed(asset_id, origin, amount));
		}

		#[weight = T::WeightInfo::redeem()]
		pub fn redeem(
			origin,
			asset_id: T::AssetId,
			#[compact] amount: T::Balance,
			to_name: Option<Vec<u8>>,
		) {
			let origin = ensure_signed(origin)?;

			let origin_account = (asset_id, origin.clone());

			let balance = <AccountAssets<T>>::get(&origin_account).balance;
			ensure!(amount <= balance , Error::<T>::InsufficientBalanceForTransaction);

			T::AssetRedeem::asset_redeem(asset_id, origin.clone(), amount, to_name);

			Self::asset_destroy(asset_id, &origin, amount);
		}

		/// Issue any amount of fungible assets.
		#[weight = T::DbWeight::get().reads_writes(1, 1)]
		pub fn unlock(
			origin,
			asset_id: T::AssetId,
			target: <T::Lookup as StaticLookup>::Source,
			#[compact] amount: T::Balance,
		) {
			ensure_root(origin)?;

			ensure!(<Tokens<T>>::contains_key(asset_id), Error::<T>::TokenNotExist);

			let target = T::Lookup::lookup(target)?;
			ensure!(!amount.is_zero(), Error::<T>::ZeroAmountOfBalance);

			let locked = <AccountAssets<T>>::get((asset_id, &target)).locked;
			// ensure this locked amount of balance should be less than his lock balance
			ensure!(locked >= amount, Error::<T>::InsufficientBalanceForTransaction);

			let target_asset = (asset_id, &target);
			<AccountAssets<T>>::mutate(target_asset, |asset| {
				asset.available = asset.available.saturating_add(amount);
				asset.locked -= amount;
			});

			Self::deposit_event(RawEvent::UnlockedAsset(target, asset_id, amount));
		}
	}
}

impl<T: Config> AssetTrait<T::AssetId, T::AccountId, T::Balance> for Module<T> {
	type Error = Error<T>;
	fn asset_create(symbol: Vec<u8>, precision: u16, token_type: TokenType) -> Result<(T::AssetId, Token<T::AssetId, T::Balance>), Self::Error> {
		let id = Self::next_asset_id();
		<NextAssetId<T>>::mutate(|id| *id += One::one());

		// Initial total supply is zero.
		let total_supply: T::Balance = Zero::zero();

		// Create token
		let token = Token::new(symbol.clone(), precision, total_supply, token_type);
		let asset_id: T::AssetId = id.into();

		// Insert to storage
		<Tokens<T>>::insert(asset_id, token.clone());

		Ok((id, token))
	}

	fn asset_create_pair(symbol: Vec<u8>, precision: u16) -> Result<(T::AssetId, T::AssetId), Self::Error> {
		let (token_id, _) = Self::asset_create(symbol.clone(), precision, TokenType::Token)?;
		let (vtoken_id, _) = Self::asset_create(symbol, precision, TokenType::VToken)?;

		<Tokens<T>>::mutate(&token_id, |token| {
			token.pair = Some(vtoken_id);
		});
		<Tokens<T>>::mutate(&vtoken_id, |vtoken| {
			vtoken.pair = Some(token_id);
		});

		Ok((token_id, vtoken_id))
	}

	fn asset_issue(
		asset_id: T::AssetId,
		target: &T::AccountId,
		amount: T::Balance,
	) {
		let vtoken_mint_rate = T::FetchVtokenMintPrice::fetch_vtoken_price(asset_id);
		let target_asset = (asset_id, target.clone());
		<AccountAssets<T>>::mutate(&target_asset, |asset| {
			asset.balance = asset.balance.saturating_add(amount);
			asset.available = asset.available.saturating_add(amount);
			asset.cost = asset.cost.saturating_add(amount.saturating_mul(vtoken_mint_rate.into()).into());
		});

		// save asset id for this account
		if <AccountAssetIds<T>>::contains_key(&target) {
			<AccountAssetIds<T>>::mutate(&target, |ids| {
				if !ids.contains(&asset_id) { // do not push a duplicated asset id to list
					ids.push(asset_id);
				}
			});
		} else {
			<AccountAssetIds<T>>::insert(&target, vec![asset_id]);
		}

		<Tokens<T>>::mutate(asset_id, |token| {
			token.total_supply = token.total_supply.saturating_add(amount);
		});
	}

	fn asset_redeem(
		asset_id: T::AssetId,
		target: &T::AccountId,
		amount: T::Balance,
	) {
		Self::asset_destroy(asset_id, &target, amount);
	}

	fn asset_destroy(
		asset_id: T::AssetId,
		target: &T::AccountId,
		amount: T::Balance,
	) {
		let vtoken_mint_rate = T::FetchVtokenMintPrice::fetch_vtoken_price(asset_id);
		let target_asset = (asset_id, target);
		<AccountAssets<T>>::mutate(target_asset, |asset| {
			asset.balance = asset.balance.saturating_sub(amount);
			asset.available = asset.available.saturating_sub(amount);
			asset.income = asset.income.saturating_add(amount.saturating_mul(vtoken_mint_rate.into()).into());
		});

		<Tokens<T>>::mutate(asset_id, |token| {
			token.total_supply = token.total_supply.saturating_sub(amount);
		});
	}

	fn asset_id_exists(who: &T::AccountId, symbol: &[u8], precision: u16) -> Option<T::AssetId> {
		let all_ids = <AccountAssetIds<T>>::get(who);
		for id in all_ids {
			let token = <Tokens<T>>::get(id);
			if token.symbol.as_slice().eq(symbol) && token.precision.eq(&precision) {
				return Some(id);
			}
		}
		None
	}

	fn token_exists(asset_id: T::AssetId) -> bool {
		<Tokens<T>>::contains_key(&asset_id)
	}

	fn get_account_asset(
		asset_id: T::AssetId,
		target: &T::AccountId,
	) -> AccountAsset<T::Balance> {
		<AccountAssets<T>>::get((asset_id, &target))
	}

	fn get_token(asset_id: T::AssetId) -> Token<T::AssetId, T::Balance> {
		<Tokens<T>>::get(&asset_id)
	}

	fn lock_asset(who: &T::AccountId, asset_id: T::AssetId, locked: T::Balance) {
		let target_asset = (asset_id, who);
		<AccountAssets<T>>::mutate(target_asset, |asset| {
			asset.locked += locked;
			asset.available = asset.balance.saturating_sub(asset.locked);
		});
	}

	fn unlock_asset(who: &T::AccountId, asset_id: T::AssetId, locked: T::Balance) {
		let target_asset = (asset_id, who);
		<AccountAssets<T>>::mutate(target_asset, |asset| {
			asset.balance = asset.balance.saturating_sub(locked);
			asset.locked -= locked;
		});
	}

	fn is_token(asset_id: T::AssetId) -> bool {
		<Tokens<T>>::get(asset_id).token_type == TokenType::Token
	}

	fn is_v_token(asset_id: T::AssetId) -> bool {
		<Tokens<T>>::get(asset_id).token_type == TokenType::VToken
	}

	fn get_pair(asset_id: T::AssetId) -> Option<T::AssetId> {
		<Tokens<T>>::get(asset_id).pair
	}
}

impl<T: Config> TokenPriceHandler<T::AssetId, T::Price> for Module<T> {
	fn set_token_price(asset_id: T::AssetId, price: T::Price) {
		<Prices<T>>::mutate(asset_id, |p| *p = price);
	}
}

// The main implementation block for the module.
impl<T: Config> Module<T> {
	fn asset_transfer(
		asset_id: T::AssetId,
		from: T::AccountId,
		to: T::AccountId,
		amount: T::Balance,
	) {
		let from_asset = (asset_id, from);
		<AccountAssets<T>>::mutate(&from_asset, |asset| {
			asset.balance = asset.balance.saturating_sub(amount);
			asset.available = asset.available.saturating_sub(amount);
		});

		let to_asset = (asset_id, &to);
		<AccountAssets<T>>::mutate(to_asset, |asset| {
			asset.balance = asset.balance.saturating_add(amount);
			asset.available = asset.available.saturating_add(amount);
		});

		// save asset id for this account
		if <AccountAssetIds<T>>::contains_key(&to) {
			<AccountAssetIds<T>>::mutate(&to, |ids| {
				// do not push a duplicated asset id to list
				if !ids.contains(&asset_id) { ids.push(asset_id); }
			});
		} else {
			<AccountAssetIds<T>>::insert(&to, vec![asset_id]);
		}
	}

	pub fn asset_balances(asset_id: T::AssetId, target: T::AccountId) -> u64 {
		let origin_account = (asset_id, target);
		let balance_u128 = <AccountAssets<T>>::get(origin_account).balance;

		// balance type is u128, but serde cannot serialize u128.
		// So I have to convert to u64, see this link
		// https://github.com/paritytech/substrate/issues/4641
		let balance_u64: u64 = balance_u128.try_into().unwrap_or(usize::max_value()) as u64;

		balance_u64
	}

	pub fn asset_tokens(target: T::AccountId) -> Vec<T::AssetId> {
		<AccountAssetIds<T>>::get(target)
	}
}

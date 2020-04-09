#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{Parameter, decl_module, decl_event, decl_error, decl_storage, ensure};
use sp_runtime::traits::{Member, AtLeast32Bit, Saturating, Zero, StaticLookup};
use sp_std::prelude::*;
use system::{ensure_signed};
use node_primitives::{
	AccountAsset, TokenPair, TokenType,
};


pub trait Trait: system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record balances.
	type Balance: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + From<Self::Exchange>;

	type Exchange: Member + Parameter + Default + AtLeast32Bit + Copy + Zero;
	/// The units in which we record costs.
	type Cost: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + From<Self::Balance>;
	/// The units in which we record incomes.
	type Income: Member + Parameter + Default + AtLeast32Bit + Copy + Zero + From<Self::Balance>;
	/// The units in which we record prices.
	type Price: Member + Parameter + Default + AtLeast32Bit + Copy + Zero;

	type AssetId: Member + Parameter + Default + AtLeast32Bit + Copy;
}

decl_error! {
	pub enum Error for Module<T: Trait> {
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
		InvalidBalanceForTransaction,
		/// Exchange rate doesn't be set
		ExchangeRateDoesNotSet,
		/// This is an invalid exchange rate
		InvalidExchangeRate,
		/// Vtoken id is not equal to token id
		InvalidTokenPair,
	}
}

decl_event! {
	pub enum Event<T>
		where <T as system::Trait>::AccountId,
			<T as Trait>::Balance,
			<T as Trait>::AssetId,
	{
		/// Some assets were transferred.
		Transferred(AssetId, TokenType, AccountId, AccountId, Balance),
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Sudo {
		pub AccountAssets get(fn account_assets): map hasher(blake2_128_concat) (T::AssetId, TokenType, T::AccountId)
			=> AccountAsset<T::Balance, T::Cost, T::Income>;

		Init get(initialized): bool;

		pub Tokens get(fn token_details): map hasher(blake2_128_concat) T::AssetId => TokenPair<T::Balance>;

		pub AccountAssetIds get(fn account_asset_ids): map hasher(blake2_128_concat) T::AccountId => Vec<T::AssetId>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

        pub fn transfer_sudo(origin,
              #[compact] id: T::AssetId,
              token_type: TokenType,
              target: <T::Lookup as StaticLookup>::Source,
              #[compact] amount: T::Balance,
         ) {
              let origin = ensure_signed(origin)?;
              let origin_account = (id, token_type, origin.clone());
              let origin_balance = <AccountAssets<T>>::get(&origin_account).balance;
              let target = T::Lookup::lookup(target)?;

              ensure!(!amount.is_zero(), Error::<T>::ZeroAmountOfBalance);
              ensure!(origin_balance >= amount, Error::<T>::InvalidBalanceForTransaction);

              Self::sodu_transfer(id, token_type, origin.clone(), target.clone(), amount);

            Self::deposit_event(RawEvent::Transferred(id, token_type, origin, target, amount));
         }
	}
}

impl<T: Trait> Module<T> {
	fn sodu_transfer(
		asset_id: T::AssetId,
		token_type: TokenType,
		from: T::AccountId,
		to: T::AccountId,
		amount: T::Balance,
	) {
		let from_asset = (asset_id, token_type, from);
		<AccountAssets<T>>::mutate(&from_asset, |asset| {
			asset.balance = asset.balance.saturating_sub(amount);
		});

		let to_asset = (asset_id, token_type, &to);
		<AccountAssets<T>>::mutate(to_asset, |asset| {
			asset.balance = asset.balance.saturating_add(amount);
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
}
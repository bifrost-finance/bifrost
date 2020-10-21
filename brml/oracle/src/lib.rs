#![cfg_attr(not(feature = "std"), no_std)]

use chainlink::{Event, create_request_event_from_parameters};
use frame_support::traits::{Get};
use frame_support::{weights::Weight,Parameter, decl_module, decl_storage, dispatch::DispatchResult};
use node_primitives::TokenPriceHandler;
use frame_system::{self as system, ensure_signed};
use sp_std::prelude::*;
use sp_runtime::traits::{Member, AtLeast32Bit, Zero, SaturatedConversion};

pub trait WeightInfo{
	fn send_request() -> Weight;
	fn callback() -> Weight;
	fn send_request_price_eos() -> Weight;
	fn send_request_price_iost() -> Weight;
	fn callback_price_eos() -> Weight;
	fn callback_price_iost() -> Weight;
}


pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;

	/// The units in which we record prices.
	type Price: Member + Parameter + AtLeast32Bit + Default + Copy + Zero;

	/// Handler for fetch token price
	type TokenPriceHandler: TokenPriceHandler<Self::Price>;

	/// Set default weight
	type WeightInfo : WeightInfo;
}

decl_storage! {
	trait Store for Module<T: Trait> as Oracle {
		pub Result: u128;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		// #[weight = T::DbWeight::get().reads_writes(1, 1)]
		#[weight = T::WeightInfo::send_request()]
		pub fn send_request(origin) -> DispatchResult {
			// TODO Investigate if Enum can be safely used to refer to a callback
			// let name: &str = stringify!(Call::<T>::callback); 'Example :: callback'
			// For now , simply rely on a string to identify the callback
			let who : <T as system::Trait>::AccountId = ensure_signed(origin)?;
			Self::deposit_event(create_request_event_from_parameters::<T, (&[u8], &[u8], &[u8], &[u8], &[u8], &[u8])>(1, 0, who, 0, (b"get", b"https://min-api.cryptocompare.com/data/pricemultifull?fsyms=ETH&tsyms=USD", b"path", b"RAW.ETH.USD.PRICE", b"times", b"100000000"), "Oracle.callback".into()));
			Ok(())
		}

		// #[weight = T::DbWeight::get().reads_writes(1, 1)]
		#[weight = T::WeightInfo::callback()]
		pub fn callback(origin, result: u128) -> DispatchResult {
			let _who : <T as system::Trait>::AccountId = ensure_signed(origin)?;
			<Result>::put(result);
			Ok(())
		}

		// #[weight = T::DbWeight::get().reads_writes(1, 1)]
		#[weight = T::WeightInfo::send_request_price_eos()]
		pub fn send_request_price_eos(origin) -> DispatchResult {
			let who : <T as system::Trait>::AccountId = ensure_signed(origin)?;
			Self::deposit_event(create_request_event_from_parameters::<T, (&[u8], &[u8], &[u8], &[u8], &[u8], &[u8])>(1, 0, who.clone(), 0, (b"get", b"https://api.huobi.pro/market/detail/merged?symbol=eosusdt", b"path", b"tick.close", b"times", b"100000000"), "Oracle.callback_price_eos".into()));
			Ok(())
		}

		// #[weight = T::DbWeight::get().reads_writes(1, 1)]
		#[weight = T::WeightInfo::send_request_price_iost()]
		pub fn send_request_price_iost(origin) -> DispatchResult {
			let who : <T as system::Trait>::AccountId = ensure_signed(origin)?;
			Self::deposit_event(create_request_event_from_parameters::<T, (&[u8], &[u8], &[u8], &[u8], &[u8], &[u8])>(1, 0, who.clone(), 0, (b"get", b"https://api.huobi.pro/market/detail/merged?symbol=iostusdt", b"path", b"tick.close", b"times", b"100000000"), "Oracle.callback_price_iost".into()));
			Ok(())
		}

		// #[weight = T::DbWeight::get().reads_writes(1, 1)]
		#[weight = T::WeightInfo::callback_price_eos()]
		pub fn callback_price_eos(origin, result: u128) -> DispatchResult {
			let _who : <T as system::Trait>::AccountId = ensure_signed(origin)?;
			T::TokenPriceHandler::set_token_price(b"EOS".to_vec(), result.saturated_into());
			Ok(())
		}

		// #[weight = T::DbWeight::get().reads_writes(1, 1)]
		#[weight = T::WeightInfo::callback_price_iost()]
		pub fn callback_price_iost(origin, result: u128) -> DispatchResult {
			let _who : <T as system::Trait>::AccountId = ensure_signed(origin)?;
			T::TokenPriceHandler::set_token_price(b"IOST".to_vec(), result.saturated_into());
			Ok(())
		}
	}
}

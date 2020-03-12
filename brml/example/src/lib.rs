#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_module, decl_storage, dispatch::DispatchResult};
use system::ensure_signed;
use chainlink::{Event, create_request_event_from_parameters};
use sp_std::prelude::*;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as ExampleStorage {
        pub Result: u128;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        pub fn send_request(origin) -> DispatchResult {
            // TODO Investigate if Enum can be safely used to refer to a callback
            // let name: &str = stringify!(Call::<T>::callback); 'Example :: callback'
            // For now , simply rely on a string to identify the callback
            let who : <T as system::Trait>::AccountId = ensure_signed(origin)?;
            Self::deposit_event(create_request_event_from_parameters::<T, (&[u8], &[u8], &[u8], &[u8], &[u8], &[u8])>(1, 0, who, 0, (b"get", b"https://min-api.cryptocompare.com/data/pricemultifull?fsyms=ETH&tsyms=USD", b"path", b"RAW.ETH.USD.PRICE", b"times", b"100000000"), "Example.callback".into()));
            Ok(())
        }

        pub fn callback(origin, result: u128) -> DispatchResult {
            let _who : <T as system::Trait>::AccountId = ensure_signed(origin)?;
            <Result>::put(result);
            Ok(())
        }
    }
}

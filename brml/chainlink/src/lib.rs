//! A pallet to interact with Chainlink nodes

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::{decl_event, decl_module, decl_storage};
use sp_std::prelude::Vec;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

pub type SpecIndex = u32;
pub type RequestIdentifier = u64;
pub type DataVersion = u64;

pub fn create_request_event_from_parameters<T: system::Trait, U: Encode>(
    spec_index: SpecIndex,
    request_id: RequestIdentifier,
    requester: T::AccountId,
    data_version: DataVersion,
    parameters: U,
    callback: Vec<u8>,
) -> Event<T> {
    create_request_event::<T>(
        spec_index,
        request_id,
        requester,
        data_version,
        parameters.encode(),
        callback,
    )
}

pub fn create_request_event<T: system::Trait>(
    spec_index: SpecIndex,
    request_id: RequestIdentifier,
    requester: T::AccountId,
    data_version: DataVersion,
    data: Vec<u8>,
    callback: Vec<u8>,
) -> Event<T> {
    RawEvent::OracleRequest(
        spec_index,
        request_id,
        requester,
        data_version,
        data,
        callback,
    )
}

decl_storage! {
    trait Store for Module<T: Trait> as ChainlinkStorage {
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
    {
        OracleRequest(
            SpecIndex,
            RequestIdentifier,
            AccountId,
            DataVersion,
            Vec<u8>,
            Vec<u8>,
        ),
    }
);

// #[cfg(test)]
// mod tests {
// 	use super::*;
//
// 	use sp_core::H256;
// 	use frame_support::{impl_outer_origin, assert_ok, parameter_types, weights::Weight};
// 	use sp_runtime::{
// 		traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
// 	};
//
// 	impl_outer_origin! {
// 		pub enum Origin for Test {}
// 	}
//
// 	#[derive(Clone, Eq, PartialEq)]
// 	pub struct Test;
// 	parameter_types! {
// 		pub const BlockHashCount: u64 = 250;
// 		pub const MaximumBlockWeight: Weight = 1024;
// 		pub const MaximumBlockLength: u32 = 2 * 1024;
// 		pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
// 	}
// 	impl system::Trait for Test {
// 		type Origin = Origin;
// 		type Call = ();
// 		type Index = u64;
// 		type BlockNumber = u64;
// 		type Hash = H256;
// 		type Hashing = BlakeTwo256;
// 		type AccountId = u64;
// 		type Lookup = IdentityLookup<Self::AccountId>;
// 		type Header = Header;
// 		type Event = ();
// 		type BlockHashCount = BlockHashCount;
// 		type MaximumBlockWeight = MaximumBlockWeight;
// 		type MaximumBlockLength = MaximumBlockLength;
// 		type AvailableBlockRatio = AvailableBlockRatio;
// 		type Version = ();
// 		type ModuleToIndex = ();
// 	}
// 	impl Trait for Test {
// 		type Event = ();
// 	}
// 	type Chainlink = Module<Test>;
//
// 	fn new_test_ext() -> sp_io::TestExternalities {
// 		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
// 	}
//
// 	#[derive(Debug, PartialEq, Encode, Decode)]
// 	struct Params {
// 		a: u32
// 	}
//
// 	#[test]
// 	fn it_works_for_default_value() {
// 		new_test_ext().execute_with(|| {
// 			assert_ok!(Chainlink::send_request(Origin::signed(1), 42));
// 			assert_eq!(Chainlink::something(), Some(42));
//
// 			let b = Params { a: 5 };
// 			let mut data: &[u8] = &Params::encode(&b);
// 			//let mut data: &[u8] = &<(u32, &str)>::encode(&(1, "sdqf"));
// 			//assert_eq!(data, vec![]);
// 			assert_eq!(b, Params::decode(&mut data).unwrap());
// 			//let mut da: &[u8] = b"\x0f";
// 			//let res:&[u8]  = &<(u32, &str)>::decode(&mut data).unwrap();
// 			//assert_eq!(<()>::decode(&mut data).unwrap(), &(1, "sdqf"));
// 		});
// 	}
// }

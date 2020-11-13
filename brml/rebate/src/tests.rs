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

//! Tests for the module.
#![cfg(test)]

use crate::*;
use crate::mock::*;
use sp_core::crypto::{Ss58Codec, AccountId32, PublicError, Ss58AddressFormat};
use sp_core::bytes::from_hex;
use hex;


#[test]
fn ss58Codec_should_be_ok() {
	new_test_ext().execute_with(|| {
		//0. 从 Ss58 Address 转换为默认格式地址 Ss58AddressFormat
		let s = "16V52HXV2YpqBWa4jJAuQYABPqPuyYSETE1LpmuJvKiASobP";
		let s = "hD2aEmuLJ5C7rzrAVT3fwkjGkFgJeDMasL8Wsm7hTH68Ya6";
		let s = "5HYmsxGRAmZMjyZYmf7uGPL2YDQGHEt6NjGrfUuxNEgeGN2P";
		let s = "16V52HXV2YpqBWa4jJAuQYABPqPuyYSETE1LpmuJvKiASobP";

		//let x = Ss58Codec::from_string(s);
		/*match x{
			// Console:" 5HYmsxGRAmZMjyZYmf7uGPL2YDQGHEt6NjGrfUuxNEgeGN2P "
			Ok::<AccountId32,_>(y) => println!("============= {} ============",y),
			Err(e) => println!("============= {:?} ============",e),
		}*/

		//1. 从任一 Ss58 Address 获取 Public Key 及默认格式化地址 Ss58AddressFormat
		let s = "hD2aEmuLJ5C7rzrAVT3fwkjGkFgJeDMasL8Wsm7hTH68Ya6";
		//let y = Ss58Codec::from_ss58check_with_version(s);
		// match y{
		// 	// AccountId32 : " 0xf295940fa750df68a686fcf4abd4111c8a9c5a5a5a83c4c8639c451a94a7adfd "
		// 	// Ss58Address : " 5HYmsxGRAmZMjyZYmf7uGPL2YDQGHEt6NjGrfUuxNEgeGN2P "
		// 	Ok::<(AccountId32,Ss58AddressFormat),_>(y) => println!("============= {:?} ============",y),
		// 	Err(e) => println!("============= {:?} ============",e),
		// }

		//
		let s = "f295940fa750df68a686fcf4abd4111c8a9c5a5a5a83c4c8639c451a94a7adfd";


		//let b = AccountId32::from([0 as u8;32]);
		//let b:AccountId32;

		let mut bytes = [0u8; 32];
		assert_eq!(hex::decode_to_slice(s, &mut bytes as &mut [u8]), Ok(()));
		let b = AccountId32::from(bytes);

		let z = Ss58Codec::to_ss58check_with_version(&b,Ss58AddressFormat::BifrostAccount);
		println!("============= {} ============",z);

		let x = Ss58Codec::to_ss58check_with_version(&b,Ss58AddressFormat::AcalaAccount);
		println!("============= {} ============",x);
	});
}
